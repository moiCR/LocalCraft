use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use tokio::{
    io::AsyncWriteExt,
    process::{Child, ChildStderr, ChildStdin, ChildStdout},
    sync::{broadcast, mpsc, oneshot, watch},
    time,
};

use super::{
    ServerEvent,
    console::{self, ConsoleSnapshot},
};

pub(super) enum Request {
    Command(String, oneshot::Sender<Result<()>>),
    Stop(Duration),
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn run(
    mut child: Child,
    mut stdin: ChildStdin,
    stdout: ChildStdout,
    stderr: ChildStderr,
    mut requests: mpsc::Receiver<Request>,
    done: watch::Sender<bool>,
    console: watch::Sender<Arc<ConsoleSnapshot>>,
    events: broadcast::Sender<ServerEvent>,
) {
    let (lines, incoming) = mpsc::channel(64);
    let mut readers = tokio::task::JoinSet::new();
    readers.spawn(console::read_output(stdout, lines.clone()));
    readers.spawn(console::read_output(stderr, lines));
    // Pipe draining must continue even while a server stalls a stdin write.
    let console_task = tokio::spawn(console::collect(incoming, console));
    let mut tick = time::interval(Duration::from_millis(75));
    let mut shutdown = None;
    let mut terminated = false;
    let outcome = loop {
        tokio::select! {
            status = child.wait() => break status,
            request = requests.recv(), if shutdown.is_none() => {
                match request {
                    Some(Request::Command(command, reply)) => {
                        if reply.is_closed() {
                            continue;
                        }
                        let result = write_command(&mut stdin, &command).await;
                        let _ = reply.send(result);
                    }
                    Some(Request::Stop(timeout)) => {
                        let _ = write_command(&mut stdin, "stop").await;
                        shutdown = Some(time::Instant::now() + timeout);
                    }
                    None => {
                        let _ = write_command(&mut stdin, "stop").await;
                        shutdown = Some(time::Instant::now() + Duration::from_secs(30));
                    }
                }
            }
            _ = tick.tick(), if shutdown.is_some() => {
                if shutdown.is_some_and(|deadline| time::Instant::now() >= deadline) {
                    if !terminated {
                        terminate(&mut child);
                        terminated = true;
                        shutdown = Some(time::Instant::now() + Duration::from_secs(5));
                    } else if let Err(error) = child.start_kill() {
                        let _ = events.send(ServerEvent::Error(format!("Could not kill server: {error}")));
                    }
                }
            }
        }
    };
    // Descendants can inherit pipes; bound draining so they cannot hold shutdown open.
    if time::timeout(Duration::from_secs(1), async {
        while readers.join_next().await.is_some() {}
    })
    .await
    .is_err()
    {
        readers.abort_all();
        while readers.join_next().await.is_some() {}
    }
    if let Err(error) = console_task.await {
        let _ = events.send(ServerEvent::Error(format!(
            "Console reader failed: {error}"
        )));
    }
    let _ = done.send(true);
    match outcome {
        Ok(status) => {
            let _ = events.send(ServerEvent::Exited(status.code()));
        }
        Err(error) => {
            let _ = events.send(ServerEvent::Error(format!(
                "Could not wait for server: {error}"
            )));
        }
    }
}

async fn write_command(stdin: &mut ChildStdin, command: &str) -> Result<()> {
    time::timeout(Duration::from_secs(2), async {
        stdin.write_all(command.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await
    })
    .await
    .context("Server stdin timed out")?
    .context("Could not write server command")
}

fn terminate(child: &mut Child) {
    #[cfg(unix)]
    if let Some(pid) = child.id().and_then(|pid| i32::try_from(pid).ok()) {
        // The child has not been reaped, so its PID cannot be reused here.
        unsafe {
            libc::kill(pid, libc::SIGTERM);
        }
    }
    #[cfg(not(unix))]
    let _ = child.start_kill();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[tokio::test]
    async fn commands_shutdown_and_final_output() -> Result<()> {
        let mut child = tokio::process::Command::new("/bin/sh")
            .args(["-c", "while IFS= read -r line; do printf '%s\\n' \"$line\"; [ \"$line\" = stop ] && exit 0; done"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;
        let stdin = child.stdin.take().context("Missing stdin")?;
        let stdout = child.stdout.take().context("Missing stdout")?;
        let stderr = child.stderr.take().context("Missing stderr")?;
        let (sender, requests) = mpsc::channel(4);
        let (done, mut finished) = watch::channel(false);
        let console = console::channel();
        let (events, mut receiver) = broadcast::channel(8);
        let task = tokio::spawn(run(
            child,
            stdin,
            stdout,
            stderr,
            requests,
            done,
            console.clone(),
            events,
        ));
        let (reply, response) = oneshot::channel();
        sender.send(Request::Command("list".into(), reply)).await?;
        response.await??;
        sender.send(Request::Stop(Duration::from_secs(1))).await?;
        time::timeout(Duration::from_secs(4), finished.wait_for(|value| *value)).await??;
        task.await?;
        let snapshot = console.borrow().clone();
        assert!(
            snapshot
                .lines
                .iter()
                .any(|line| line.content.text == "list")
        );
        assert!(
            snapshot
                .lines
                .iter()
                .any(|line| line.content.text == "stop")
        );
        let mut exited = false;
        while let Ok(event) = receiver.try_recv() {
            if matches!(event, ServerEvent::Exited(Some(0))) {
                exited = true;
            }
        }
        assert!(exited);
        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn blocked_stdin_does_not_block_log_delivery() -> Result<()> {
        let mut child = tokio::process::Command::new("/bin/sh")
            .args([
                "-c",
                "printf 'ready\\n'; sleep 0.1; printf 'progress\\n'; sleep 1",
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;
        let stdin = child.stdin.take().context("Missing stdin")?;
        let stdout = child.stdout.take().context("Missing stdout")?;
        let stderr = child.stderr.take().context("Missing stderr")?;
        let (sender, requests) = mpsc::channel(4);
        let (done, _) = watch::channel(false);
        let console = console::channel();
        let mut snapshots = console.subscribe();
        let (events, _) = broadcast::channel(4);
        let task = tokio::spawn(run(
            child, stdin, stdout, stderr, requests, done, console, events,
        ));
        let (reply, mut response) = oneshot::channel();
        sender
            .send(Request::Command("x".repeat(1_000_000), reply))
            .await?;
        time::timeout(Duration::from_millis(600), async {
            loop {
                snapshots.changed().await?;
                if snapshots
                    .borrow_and_update()
                    .lines
                    .iter()
                    .any(|line| line.content.text == "progress")
                {
                    break;
                }
            }
            Ok::<_, anyhow::Error>(())
        })
        .await??;
        assert!(matches!(
            response.try_recv(),
            Err(oneshot::error::TryRecvError::Empty)
        ));
        time::timeout(Duration::from_secs(4), task).await??;
        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn commands_dispatch_during_a_large_ansi_log_burst() -> Result<()> {
        let mut child = tokio::process::Command::new("/bin/sh")
            .args(["-c", "i=0; while [ \"$i\" -lt 10000 ]; do printf '\\033[38;2;255;85;85mline %s\\033[0m\\n' \"$i\"; i=$((i + 1)); done; while IFS= read -r line; do printf '[received] %s\\n' \"$line\"; [ \"$line\" = stop ] && exit 0; done"])
            .stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped())
            .kill_on_drop(true).spawn()?;
        let stdin = child.stdin.take().context("Missing stdin")?;
        let stdout = child.stdout.take().context("Missing stdout")?;
        let stderr = child.stderr.take().context("Missing stderr")?;
        let (sender, requests) = mpsc::channel(4);
        let (done, _finished) = watch::channel(false);
        let console = console::channel();
        let mut snapshots = console.subscribe();
        let (events, _) = broadcast::channel(4);
        let task = tokio::spawn(run(
            child, stdin, stdout, stderr, requests, done, console, events,
        ));
        let (reply, response) = oneshot::channel();
        let started = time::Instant::now();
        sender.send(Request::Command("list".into(), reply)).await?;
        time::timeout(Duration::from_millis(500), response).await???;
        let dispatch = started.elapsed();
        time::timeout(Duration::from_secs(3), async {
            loop {
                snapshots.changed().await?;
                if snapshots
                    .borrow_and_update()
                    .lines
                    .iter()
                    .any(|line| line.content.text == "[received] list")
                {
                    break;
                }
            }
            Ok::<_, anyhow::Error>(())
        })
        .await??;
        assert!(snapshots.borrow().lines.len() <= super::super::CONSOLE_CAPACITY);
        assert!(
            snapshots
                .borrow()
                .lines
                .iter()
                .all(|line| !line.content.text.contains('\u{1b}'))
        );
        eprintln!(
            "10,000 ANSI lines: stdin acknowledgement={dispatch:?}; command output={:?}",
            started.elapsed()
        );
        sender.send(Request::Stop(Duration::from_secs(1))).await?;
        time::timeout(Duration::from_secs(3), task).await??;
        Ok(())
    }
}
