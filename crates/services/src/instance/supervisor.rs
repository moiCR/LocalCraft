use std::{collections::VecDeque, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    process::{Child, ChildStderr, ChildStdin, ChildStdout},
    sync::{Mutex, broadcast, mpsc, oneshot, watch},
    time,
};

use super::{CONSOLE_CAPACITY, ServerEvent};

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
    console: Arc<Mutex<VecDeque<String>>>,
    events: broadcast::Sender<ServerEvent>,
) {
    let (lines, mut incoming) = mpsc::channel(256);
    let stdout_task = tokio::spawn(read_output(stdout, lines.clone()));
    let stderr_task = tokio::spawn(read_output(stderr, lines));
    let mut batch = Vec::new();
    let mut tick = time::interval(Duration::from_millis(75));
    let mut shutdown = None;
    let mut terminated = false;
    let outcome = loop {
        tokio::select! {
            status = child.wait() => break status,
            request = requests.recv(), if shutdown.is_none() => {
                match request {
                    Some(Request::Command(command, reply)) => {
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
            Some(line) = incoming.recv() => {
                if batch.len() < CONSOLE_CAPACITY { batch.push(line); }
            }
            _ = tick.tick() => {
                flush(&mut batch, &console, &events).await;
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
    let drain_deadline = time::sleep(Duration::from_secs(1));
    tokio::pin!(drain_deadline);
    loop {
        tokio::select! {
            line = incoming.recv() => match line {
                Some(line) => {
                    if batch.len() == CONSOLE_CAPACITY { flush(&mut batch, &console, &events).await; }
                    batch.push(line);
                }
                None => break,
            },
            _ = &mut drain_deadline => break,
        }
    }
    stdout_task.abort();
    stderr_task.abort();
    let _ = stdout_task.await;
    let _ = stderr_task.await;
    flush(&mut batch, &console, &events).await;
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
    let _ = done.send(true);
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

async fn flush(
    batch: &mut Vec<String>,
    console: &Mutex<VecDeque<String>>,
    events: &broadcast::Sender<ServerEvent>,
) {
    if batch.is_empty() {
        return;
    }
    let lines = std::mem::take(batch);
    {
        let mut console = console.lock().await;
        for line in &lines {
            if console.len() == CONSOLE_CAPACITY {
                console.pop_front();
            }
            console.push_back(line.clone());
        }
    }
    let _ = events.send(ServerEvent::Console(lines));
}

async fn read_output(mut stream: impl AsyncRead + Unpin, output: mpsc::Sender<String>) {
    let mut buffer = [0_u8; 4096];
    let mut line = Vec::new();
    loop {
        match stream.read(&mut buffer).await {
            Ok(0) => break,
            Ok(count) => {
                for byte in buffer.iter().take(count) {
                    if *byte == b'\n' {
                        if output
                            .send(
                                String::from_utf8_lossy(&line)
                                    .trim_end_matches('\r')
                                    .to_owned(),
                            )
                            .await
                            .is_err()
                        {
                            return;
                        }
                        line.clear();
                    } else if line.len() < 8192 {
                        line.push(*byte);
                    }
                }
            }
            Err(error) => {
                let _ = output
                    .send(format!("Could not read server output: {error}"))
                    .await;
                break;
            }
        }
    }
    if !line.is_empty() {
        let _ = output
            .send(String::from_utf8_lossy(&line).into_owned())
            .await;
    }
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
        let console = Arc::new(Mutex::new(VecDeque::new()));
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
        let lines = console.lock().await;
        assert!(lines.iter().any(|line| line == "list"));
        assert!(lines.iter().any(|line| line == "stop"));
        let mut exited = false;
        while let Ok(event) = receiver.try_recv() {
            if matches!(event, ServerEvent::Exited(Some(0))) {
                exited = true;
            }
        }
        assert!(exited);
        Ok(())
    }

    #[tokio::test]
    async fn console_is_bounded() {
        let console = Mutex::new(VecDeque::new());
        let (events, _) = broadcast::channel(2);
        let mut lines = (0..CONSOLE_CAPACITY + 10).map(|n| n.to_string()).collect();
        flush(&mut lines, &console, &events).await;
        let console = console.lock().await;
        assert_eq!(console.len(), CONSOLE_CAPACITY);
        assert_eq!(console.front().map(String::as_str), Some("10"));
    }

    #[tokio::test]
    async fn output_caps_long_lines_and_preserves_tail() {
        let mut bytes = vec![b'x'; 10_000];
        bytes.extend_from_slice(b"\nlast");
        let (sender, mut receiver) = mpsc::channel(4);
        read_output(bytes.as_slice(), sender).await;
        assert_eq!(receiver.recv().await.map(|line| line.len()), Some(8192));
        assert_eq!(receiver.recv().await.as_deref(), Some("last"));
    }
}
