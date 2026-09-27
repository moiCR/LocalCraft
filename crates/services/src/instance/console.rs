use std::{collections::VecDeque, sync::Arc, time::Duration};

use tokio::{
    io::{AsyncRead, AsyncReadExt},
    sync::{mpsc, watch},
    time,
};

pub use super::ansi::ConsoleText;
use super::{
    CONSOLE_CAPACITY,
    ansi::{AnsiParser, MAX_LINE_BYTES},
};

pub const CONSOLE_FLUSH_INTERVAL: Duration = Duration::from_millis(33);
const READ_BATCH_LINES: usize = 32;

#[derive(Clone, Debug)]
pub struct ConsoleLine {
    pub sequence: u64,
    pub timestamp: String,
    pub content: Arc<ConsoleText>,
}

pub fn timestamp_now() -> String {
    chrono::Local::now().format("%H:%M:%S").to_string()
}

#[derive(Default, Debug)]
pub struct ConsoleSnapshot {
    pub lines: Vec<ConsoleLine>,
}

pub(super) fn channel() -> watch::Sender<Arc<ConsoleSnapshot>> {
    watch::channel(Arc::default()).0
}

pub(super) async fn collect(
    mut incoming: mpsc::Receiver<Vec<Arc<ConsoleText>>>,
    published: watch::Sender<Arc<ConsoleSnapshot>>,
) {
    let initial = published.borrow().clone();
    let mut ring: VecDeque<_> = initial.lines.iter().cloned().collect();
    drop(initial);
    let mut next_sequence = ring
        .back()
        .map_or(0, |line| line.sequence.saturating_add(1));
    let mut dirty = false;
    let mut tick = time::interval_at(
        time::Instant::now() + CONSOLE_FLUSH_INTERVAL,
        CONSOLE_FLUSH_INTERVAL,
    );
    tick.set_missed_tick_behavior(time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            batch = incoming.recv() => match batch {
                Some(batch) => {
                    for content in batch {
                        if ring.len() == CONSOLE_CAPACITY { ring.pop_front(); }
                        ring.push_back(ConsoleLine {
                            sequence: next_sequence,
                            timestamp: timestamp_now(),
                            content,
                        });
                        next_sequence = next_sequence.saturating_add(1);
                        dirty = true;
                    }
                }
                None => break,
            },
            _ = tick.tick(), if dirty => {
                publish(&ring, &published);
                dirty = false;
            }
        }
    }
    if dirty {
        publish(&ring, &published);
    }
}

fn publish(ring: &VecDeque<ConsoleLine>, published: &watch::Sender<Arc<ConsoleSnapshot>>) {
    // A slow UI needs the latest bounded history, not a backlog of obsolete frames.
    published.send_replace(Arc::new(ConsoleSnapshot {
        lines: ring.iter().cloned().collect(),
    }));
}

pub(super) async fn read_output(
    mut stream: impl AsyncRead + Unpin,
    output: mpsc::Sender<Vec<Arc<ConsoleText>>>,
) {
    let mut buffer = [0_u8; 4096];
    let mut line = Vec::new();
    let mut parser = AnsiParser::default();
    let mut truncated = false;
    loop {
        match stream.read(&mut buffer).await {
            Ok(0) => break,
            Ok(count) => {
                let mut batch = Vec::new();
                for byte in buffer.iter().take(count) {
                    if *byte == b'\n' {
                        batch.push(
                            parser.parse(String::from_utf8_lossy(&line).trim_end_matches('\r')),
                        );
                        line.clear();
                        if truncated {
                            // A discarded tail may contain the reset sequence for the next line.
                            parser.reset();
                            truncated = false;
                        }
                        if batch.len() == READ_BATCH_LINES
                            && output.send(std::mem::take(&mut batch)).await.is_err()
                        {
                            return;
                        }
                    } else if line.len() < MAX_LINE_BYTES {
                        line.push(*byte);
                    } else {
                        truncated = true;
                    }
                }
                if !batch.is_empty() && output.send(batch).await.is_err() {
                    return;
                }
            }
            Err(error) => {
                let _ = output
                    .send(vec![ConsoleText::plain(format!(
                        "Could not read server output: {error}"
                    ))])
                    .await;
                break;
            }
        }
    }
    if !line.is_empty() {
        let _ = output
            .send(vec![parser.parse(&String::from_utf8_lossy(&line))])
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn slow_consumer_receives_latest_bounded_snapshot_with_shared_text() -> anyhow::Result<()>
    {
        let published = channel();
        let mut ui = published.subscribe();
        let (sender, receiver) = mpsc::channel(2);
        let task = tokio::spawn(collect(receiver, published.clone()));
        let content = ConsoleText::plain("burst".into());
        for _ in 0..100 {
            sender.send(vec![content.clone(); READ_BATCH_LINES]).await?;
        }
        drop(sender);
        task.await?;
        ui.changed().await?;
        let snapshot = ui.borrow_and_update().clone();
        assert_eq!(snapshot.lines.len(), CONSOLE_CAPACITY);
        assert_eq!(snapshot.lines.last().map(|line| line.sequence), Some(3199));
        assert_eq!(snapshot.lines.first().map(|line| line.sequence), Some(1200));
        assert!(
            snapshot
                .lines
                .iter()
                .all(|line| Arc::ptr_eq(&line.content, &content))
        );
        assert!(!ui.has_changed()?);
        Ok(())
    }

    #[tokio::test]
    async fn ansi_sequences_split_across_reads_are_parsed_once() -> anyhow::Result<()> {
        let (mut writer, reader) = tokio::io::duplex(8);
        let (sender, mut receiver) = mpsc::channel(8);
        let task = tokio::spawn(read_output(reader, sender));
        writer.write_all(b"\x1b[38;2;255;").await?;
        writer
            .write_all("85;85mé猫\x1b[0m\nlast".as_bytes())
            .await?;
        drop(writer);
        task.await?;
        let mut lines = Vec::new();
        while let Some(batch) = receiver.recv().await {
            lines.extend(batch);
        }
        assert_eq!(lines.first().map(|line| line.text.as_ref()), Some("é猫"));
        assert_eq!(lines.first().map(|line| line.highlights.len()), Some(1));
        assert_eq!(lines.last().map(|line| line.text.as_ref()), Some("last"));
        Ok(())
    }

    #[tokio::test]
    async fn output_caps_long_lines_and_preserves_tail() {
        let mut bytes = vec![b'x'; 10_000];
        bytes.extend_from_slice(b"\nlast");
        let (sender, mut receiver) = mpsc::channel(4);
        read_output(bytes.as_slice(), sender).await;
        let first = receiver.recv().await;
        assert_eq!(
            first
                .as_ref()
                .and_then(|batch| batch.first())
                .map(|line| line.text.len()),
            Some(MAX_LINE_BYTES)
        );
        let last = receiver.recv().await;
        assert_eq!(
            last.as_ref()
                .and_then(|batch| batch.last())
                .map(|line| line.text.as_ref()),
            Some("last")
        );
    }
}
