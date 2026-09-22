use super::console_view::ConsoleView;
use crate::modals::server_settings::{ServerSettings, SettingsEvent};
use gpui::BorrowAppContext;
use gpui::{AppContext, Context, Entity, EventEmitter, Subscription};
use services::{
    AppState,
    instance::{ServerEvent, ServerInstance},
};
use std::time::Duration;
use ui::components::input::{Input, InputEvent};

#[derive(Clone, Copy)]
pub enum Operation {
    Start,
    Stop,
    Restart,
}
pub struct Back;
impl EventEmitter<Back> for ServerScreen {}

enum Update {
    State(bool),
    Event(ServerEvent, bool),
}

pub struct ServerScreen {
    pub server: ServerInstance,
    pub console: Entity<ConsoleView>,
    pub running: bool,
    pub busy: bool,
    pub status: Option<String>,
    pub command: Entity<Input>,
    pub settings: Option<Entity<ServerSettings>>,
    _command_subscription: Subscription,
    settings_subscription: Option<Subscription>,
}
impl ServerScreen {
    pub fn new(server: ServerInstance, cx: &mut Context<Self>) -> Self {
        let command = cx.new(|cx| Input::new("", "Type a command…", cx));
        let console = cx.new(|cx| ConsoleView::new(server.subscribe_console(), cx));
        let subscription = cx.subscribe(&command, |this, _, event, cx| {
            if matches!(event, InputEvent::Submitted) {
                this.send_command(cx);
            }
        });
        let (tx, mut rx) = tokio::sync::mpsc::channel(4);
        let mut events = server.subscribe();
        let worker = server.clone();
        cx.global::<AppState>().background_runtime.spawn(async move {
            if tx.send(Update::State(worker.is_running().await)).await.is_err() { return; }
            loop {
                tokio::select! {
                    _ = tx.closed() => break,
                    event = events.recv() => {
                        let update = match event {
                            Ok(event) => Update::Event(event, worker.is_running().await),
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => Update::State(worker.is_running().await),
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                        };
                        if tx.send(update).await.is_err() { break; }
                    }
                }
            }
        });
        cx.spawn(async move |this, cx| {
            while let Some(update) = rx.recv().await {
                if this
                    .update(cx, |this, cx| {
                        match update {
                            Update::State(running) => this.running = running,
                            Update::Event(event, running) => {
                                this.running = running;
                                match event {
                                    ServerEvent::Error(error) => this.status = Some(error),
                                    ServerEvent::Exited(code) => {
                                        this.console.update(cx, |console, cx| {
                                            console.echo(
                                                format!("[LocalCraft] Process exited: {code:?}"),
                                                cx,
                                            );
                                        });
                                        if !this.busy {
                                            this.status =
                                                Some(format!("Server stopped (exit: {code:?})."));
                                        }
                                    }
                                }
                            }
                        }
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        Self {
            server,
            console,
            running: false,
            busy: false,
            status: None,
            command,
            settings: None,
            _command_subscription: subscription,
            settings_subscription: None,
        }
    }

    pub fn refresh_console(&mut self, cx: &mut Context<Self>) {
        let snapshot = self.server.console();
        self.console
            .update(cx, |console, cx| console.set_snapshot(&snapshot, cx));
    }

    pub fn operate(&mut self, operation: Operation, cx: &mut Context<Self>) {
        if self.busy || self.settings.is_some() {
            return;
        }
        if matches!(operation, Operation::Start) && self.running {
            return;
        }
        if matches!(operation, Operation::Stop | Operation::Restart) && !self.running {
            return;
        }
        self.busy = true;
        self.status = Some(
            match operation {
                Operation::Start => "Starting…",
                Operation::Stop => "Stopping…",
                Operation::Restart => "Restarting…",
            }
            .into(),
        );
        let server = self.server.clone();
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        cx.global::<AppState>()
            .background_runtime
            .spawn(async move {
                let result = async {
                    if matches!(operation, Operation::Stop | Operation::Restart) {
                        server.stop(Duration::from_secs(30)).await?;
                    }
                    if matches!(operation, Operation::Start | Operation::Restart) {
                        let java = server.java_binary().await?;
                        server.start(&java).await?;
                    }
                    Ok::<_, anyhow::Error>(())
                }
                .await;
                let _ = tx
                    .send((
                        result.map_err(|e| format!("{e:#}")),
                        server.is_running().await,
                    ))
                    .await;
            });
        cx.spawn(async move |this, cx| {
            if let Some((result, running)) = rx.recv().await {
                let _ = this.update(cx, |this, cx| {
                    this.busy = false;
                    this.running = running;
                    this.status = match result {
                        Ok(()) => Some(
                            if running {
                                "Process started. Watch the console for server readiness."
                            } else {
                                "Server stopped."
                            }
                            .into(),
                        ),
                        Err(error) => Some(error),
                    };
                    cx.notify();
                });
            }
        })
        .detach();
        cx.notify();
    }
    pub fn send_command(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        if !self.running {
            self.status = Some("Server is not running. Start it before sending commands.".into());
            cx.notify();
            return;
        }
        let command = self.command.read(cx).value().trim().to_owned();
        if command.is_empty() {
            return;
        }
        self.console.update(cx, |console, cx| {
            console.echo(format!("> {command}"), cx);
        });
        self.command.update(cx, |input, cx| input.clear(cx));
        let server = self.server.clone();
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        cx.global::<AppState>()
            .background_runtime
            .spawn(async move {
                let result =
                    tokio::time::timeout(Duration::from_secs(5), server.send_command(command))
                        .await
                        .map_err(|_| "Command request timed out after 5 seconds".to_owned())
                        .and_then(|result| result.map_err(|error| format!("{error:#}")));
                let _ = tx.send(result).await;
            });
        cx.spawn(async move |this, cx| {
            if let Some(result) = rx.recv().await {
                let _ = this.update(cx, |this, cx| {
                    if let Err(error) = result {
                        this.status = Some(error);
                        cx.notify();
                    }
                });
            }
        })
        .detach();
        cx.notify();
    }
    pub fn open_settings(&mut self, window: &mut gpui::Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let settings = cx.new(|cx| ServerSettings::new(&self.server, self.running, cx));
        window.focus(&settings.read(cx).focus);
        self.settings_subscription = Some(cx.subscribe(&settings, |this, _, event, cx| {
            match event {
                SettingsEvent::Saved(server) => {
                    this.server = server.clone();
                    cx.update_global::<AppState, _>(|state, _| {
                        state.instance_service.insert(server.clone())
                    });
                    this.settings = None;
                }
                SettingsEvent::Close => this.settings = None,
            }
            cx.notify();
        }));
        self.settings = Some(settings);
        cx.notify();
    }
}
