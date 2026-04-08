use crate::software::software_manager::SoftwareManager;
use directories::ProjectDirs;
use serde::Serialize;
use std::process::Stdio;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_opener::OpenerExt;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

#[derive(Serialize, Clone, Debug)]
pub struct TunnelStatus {
    pub active: bool,
    pub address: Option<String>,
    pub claim_url: Option<String>,
}

pub struct TunnelManager {
    pub child: Arc<Mutex<Option<Child>>>,
    pub address: Arc<Mutex<Option<String>>>,
    pub claim_url: Arc<Mutex<Option<String>>>,
}

impl TunnelManager {
    pub fn new() -> Self {
        Self {
            child: Arc::new(Mutex::new(None)),
            address: Arc::new(Mutex::new(None)),
            claim_url: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn is_active(&self) -> bool {
        let mut guard = self.child.lock().await;
        if let Some(child) = guard.as_mut() {
            match child.try_wait() {
                Ok(None) => true,
                _ => {
                    *guard = None;
                    false
                }
            }
        } else {
            false
        }
    }

    pub async fn get_status(&self) -> TunnelStatus {
        TunnelStatus {
            active: self.is_active().await,
            address: self.address.lock().await.clone(),
            claim_url: self.claim_url.lock().await.clone(),
        }
    }
}

fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\x1b' => {
                match chars.peek().copied() {
                    Some('[') => {
                        // CSI sequence: ESC [ ... <final byte 0x40–0x7E>
                        chars.next();
                        while let Some(&ch) = chars.peek() {
                            chars.next();
                            if ch >= '\x40' && ch <= '\x7e' {
                                break;
                            }
                        }
                    }
                    Some(']') => {
                        // OSC sequence: ESC ] ... BEL or ESC \
                        chars.next();
                        loop {
                            match chars.next() {
                                Some('\x07') => break,
                                Some('\x1b') => {
                                    if chars.peek().copied() == Some('\\') {
                                        chars.next();
                                    }
                                    break;
                                }
                                None => break,
                                _ => {}
                            }
                        }
                    }
                    _ => {
                        chars.next(); // skip next char for other ESC sequences
                    }
                }
            }
            '\r' => {} // skip carriage returns
            '\x08' => {
                result.pop(); // backspace
            }
            other => result.push(other),
        }
    }

    result
}

fn extract_playit_data(text: &str) -> (Option<String>, Option<String>) {
    let mut claim_url = None;
    let mut address = None;

    // Find claim URL — stop at whitespace OR any common surrounding punctuation
    if let Some(start) = text.find("https://playit.gg/claim/") {
        let url: String = text[start..]
            .chars()
            .take_while(|&c| {
                !c.is_whitespace()
                    && c != ']'
                    && c != ')'
                    && c != '"'
                    && c != '\''
                    && c != '}'
                    && c != '>'
            })
            .collect();
        // Only store if it has content after the prefix
        if url.len() > "https://playit.gg/claim/".len() {
            claim_url = Some(url);
        }
    }

    // Strategy 1: scan word-by-word for known tunnel domains.
    // Covers both free plan (*.joinmc.link) and paid plan (*.playit.gg / *.gl.playit.gg).
    // Takes the LAST match so the freshest address in the rolling buffer wins.
    // NOTE: ':' is intentionally excluded from trim_matches so the port (":25565") is kept.
    for word in text.split_whitespace() {
        let clean = word.trim_matches(|c: char| "()[],'\"{}|><!".contains(c));
        if !clean.is_empty() && !clean.starts_with("http") {
            if clean.contains(".playit.gg")
                || clean.contains(".joinmc.link")
                || clean.contains(".gl.playit.gg")
            {
                address = Some(clean.to_string());
            }
        }
    }

    // Strategy 2: fallback — grab the token right after '=>' (local:port => public:port).
    // This handles any future playit domain without needing to hard-code it.
    if address.is_none() {
        if let Some(pos) = text.rfind("=>") {
            let after = text[pos + 2..].trim_start();
            let candidate: String = after.chars().take_while(|&c| !c.is_whitespace()).collect();
            let candidate = candidate.trim_matches(|c: char| "()[],'\"{}|><!".contains(c));
            // Accept only if it looks like a real public address (host:port, not a local one)
            if candidate.contains(':')
                && !candidate.is_empty()
                && !candidate.starts_with("0.0.0.0")
                && !candidate.starts_with("127.")
                && !candidate.starts_with("[::]")
            {
                address = Some(candidate.to_string());
            }
        }
    }

    (claim_url, address)
}

#[tauri::command]
pub async fn start_tunnel(
    handle: AppHandle,
    state: State<'_, TunnelManager>,
) -> Result<TunnelStatus, String> {
    if state.is_active().await {
        return Ok(state.get_status().await);
    }

    let proj_dirs = ProjectDirs::from("com", "localcraft", "LocalCraft")
        .ok_or("Failed to determine app directory")?;
    let tools_dir = proj_dirs.data_dir().join("tools");
    let playit_path = tools_dir.join("playit.exe");

    if !playit_path.exists() {
        SoftwareManager::download_playit(&handle).await?;
    }

    let mut command = Command::new(&playit_path);
    command
        .current_dir(&tools_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .kill_on_drop(true);

    #[cfg(target_os = "windows")]
    {
        command.creation_flags(0x08000000);
    }

    let mut child = command
        .spawn()
        .map_err(|e| format!("Failed to spawn playit process: {}", e))?;

    let mut stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let mut stderr = child.stderr.take().ok_or("Failed to capture stderr")?;

    {
        let mut child_guard = state.child.lock().await;
        *child_guard = Some(child);
    }

    let address_state = state.address.clone();
    let claim_url_state = state.claim_url.clone();
    let handle_clone = handle.clone();

    {
        *address_state.lock().await = None;
        *claim_url_state.lock().await = None;
    }

    let addr_s = address_state.clone();
    let url_s = claim_url_state.clone();
    let h1 = handle_clone.clone();

    tokio::spawn(async move {
        let mut buffer = [0; 1024];
        let mut output_acc = String::new();

        loop {
            match stdout.read(&mut buffer).await {
                Ok(0) => break,
                Ok(n) => {
                    let text = String::from_utf8_lossy(&buffer[..n]);
                    output_acc.push_str(&text);

                    let clean = strip_ansi(&output_acc);
                    let (found_url, found_addr) = extract_playit_data(&clean);

                    let mut updated = false;

                    if let Some(url) = found_url {
                        let mut url_guard = url_s.lock().await;
                        if url_guard.is_none() {
                            // First detection: auto-open the browser
                            let _ = h1.opener().open_url(&url, None::<&str>);
                            *url_guard = Some(url);
                            updated = true;
                        } else if url_guard.as_deref() != Some(&url) {
                            *url_guard = Some(url);
                            updated = true;
                        }
                    }

                    if let Some(addr) = found_addr {
                        let mut addr_guard = addr_s.lock().await;
                        if addr_guard.as_deref() != Some(&addr) {
                            *addr_guard = Some(addr);
                            updated = true;
                        }
                    }

                    if updated {
                        let _ = h1.emit(
                            "tunnel-status",
                            TunnelStatus {
                                active: true,
                                address: addr_s.lock().await.clone(),
                                claim_url: url_s.lock().await.clone(),
                            },
                        );
                    }

                    if output_acc.len() > 4096 {
                        output_acc = output_acc[2048..].to_string();
                    }
                }
                Err(_) => break,
            }
        }

        let _ = h1.emit(
            "tunnel-status",
            TunnelStatus {
                active: false,
                address: None,
                claim_url: None,
            },
        );
    });

    let addr_s2 = address_state.clone();
    let url_s2 = claim_url_state.clone();
    let h2 = handle_clone.clone();

    tokio::spawn(async move {
        let mut buffer = [0; 1024];
        let mut output_acc = String::new();

        loop {
            match stderr.read(&mut buffer).await {
                Ok(0) => break,
                Ok(n) => {
                    let text = String::from_utf8_lossy(&buffer[..n]);
                    output_acc.push_str(&text);

                    let clean = strip_ansi(&output_acc);
                    let (found_url, found_addr) = extract_playit_data(&clean);

                    let mut updated = false;

                    if let Some(url) = found_url {
                        let mut url_guard = url_s2.lock().await;
                        if url_guard.is_none() {
                            // First detection: auto-open the browser
                            let _ = h2.opener().open_url(&url, None::<&str>);
                            *url_guard = Some(url);
                            updated = true;
                        } else if url_guard.as_deref() != Some(&url) {
                            *url_guard = Some(url);
                            updated = true;
                        }
                    }

                    if let Some(addr) = found_addr {
                        let mut addr_guard = addr_s2.lock().await;
                        if addr_guard.as_deref() != Some(&addr) {
                            *addr_guard = Some(addr);
                            updated = true;
                        }
                    }

                    if updated {
                        let _ = h2.emit(
                            "tunnel-status",
                            TunnelStatus {
                                active: true,
                                address: addr_s2.lock().await.clone(),
                                claim_url: url_s2.lock().await.clone(),
                            },
                        );
                    }

                    if output_acc.len() > 4096 {
                        output_acc = output_acc[2048..].to_string();
                    }
                }
                Err(_) => break,
            }
        }
    });

    Ok(state.get_status().await)
}

#[tauri::command]
pub async fn stop_tunnel(state: State<'_, TunnelManager>) -> Result<(), String> {
    let mut child_guard = state.child.lock().await;
    if let Some(mut child) = child_guard.take() {
        let _ = child.kill().await;
    }

    let mut addr_guard = state.address.lock().await;
    *addr_guard = None;

    let mut claim_guard = state.claim_url.lock().await;
    *claim_guard = None;

    Ok(())
}

#[tauri::command]
pub async fn get_tunnel_status(state: State<'_, TunnelManager>) -> Result<TunnelStatus, String> {
    Ok(state.get_status().await)
}
