use super::create_server_view::input_field;
use gpui::{
    AppContext, Context, Entity, EventEmitter, FocusHandle, FontWeight, IntoElement, KeyDownEvent,
    Render, Window, div, prelude::*, px,
};
use gpui_kit::base::Disableable;
use services::{
    AppState,
    instance::{ServerInstance, configuration::validate_settings},
    software::pumpkin,
};
use ui::{
    components::{Button, ButtonVariants, IconButton, Modal, input::Input},
    theme::ActiveUiTheme,
};

pub enum SettingsEvent {
    Saved(ServerInstance),
    Close,
}

pub struct ServerSettings {
    server: ServerInstance,
    name: Entity<Input>,
    ram: Entity<Input>,
    port: Entity<Input>,
    pub focus: FocusHandle,
    running: bool,
    busy: bool,
    error: Option<String>,
}

impl EventEmitter<SettingsEvent> for ServerSettings {}

impl ServerSettings {
    pub fn new(server: &ServerInstance, running: bool, cx: &mut Context<Self>) -> Self {
        Self {
            server: server.clone(),
            name: cx.new(|cx| Input::new(server.name.clone(), "Server name", cx)),
            ram: cx.new(|cx| Input::new(server.ram.clone(), "Memory in MiB", cx)),
            port: cx.new(|cx| Input::new(server.port.to_string(), "Port", cx)),
            focus: cx.focus_handle(),
            running,
            busy: false,
            error: None,
        }
    }

    fn close(&self, cx: &mut Context<Self>) {
        if !self.busy {
            cx.emit(SettingsEvent::Close);
        }
    }

    fn save(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.running {
            return;
        }
        let name = self.name.read(cx).value().trim().to_owned();
        let ram = if pumpkin::is_pumpkin(&self.server.software) {
            Some(2048)
        } else {
            self.ram.read(cx).value().parse::<u32>().ok()
        };
        let parsed = ram.zip(self.port.read(cx).value().parse::<u16>().ok());
        let Some((ram, port)) = parsed else {
            self.error = Some("Enter valid memory and port numbers".into());
            cx.notify();
            return;
        };
        if let Err(error) = validate_settings(&name, ram, port) {
            self.error = Some(error.to_string());
            cx.notify();
            return;
        }
        if cx
            .global::<AppState>()
            .instance_service
            .servers()
            .iter()
            .any(|s| s.id != self.server.id && s.port == port)
        {
            self.error = Some("Another server is using this port".into());
            cx.notify();
            return;
        }
        self.busy = true;
        self.error = None;
        let mut server = self.server.clone();
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        cx.global::<AppState>()
            .background_runtime
            .spawn(async move {
                let result = server
                    .update_settings(name, ram, port)
                    .await
                    .map(|_| server)
                    .map_err(|e| format!("{e:#}"));
                let _ = tx.send(result).await;
            });
        cx.spawn(async move |this, cx| {
            if let Some(result) = rx.recv().await {
                let _ = this.update(cx, |this, cx| {
                    this.busy = false;
                    match result {
                        Ok(server) => cx.emit(SettingsEvent::Saved(server)),
                        Err(error) => this.error = Some(error),
                    }
                    cx.notify();
                });
            }
        })
        .detach();
        cx.notify();
    }
}

impl Render for ServerSettings {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.ui_palette();
        let viewport = window.viewport_size();
        let is_pumpkin = pumpkin::is_pumpkin(&self.server.software);
        let runtime = if is_pumpkin {
            "Native"
        } else {
            self.server
                .java_version
                .as_deref()
                .map_or("Java not selected", |version| match version {
                    "8" => "Java 8",
                    "17" => "Java 17",
                    "21" => "Java 21",
                    "25" => "Java 25",
                    _ => "Java",
                })
        };
        let entity = cx.entity().downgrade();

        div()
            .id("server-settings-modal-root")
            .absolute()
            .inset_0()
            .size_full()
            .track_focus(&self.focus)
            .capture_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" {
                    this.close(cx);
                    cx.stop_propagation();
                }
            }))
            .child(
                Modal::new("server-settings-modal")
                    .open(true)
                    .compact()
                    .fit_content()
                    .width((viewport.width - px(32.0)).clamp(px(0.0), px(480.0)))
                    .max_height((viewport.height - px(32.0)).max(px(0.0)))
                    .header(
                        div()
                            .flex()
                            .flex_none()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .pl_1()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_0p5()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(palette.text)
                                            .child("Server settings"),
                                    )
                                    .child(div().text_xs().text_color(palette.muted).child(
                                        format!(
                                            "{} · {} · {}",
                                            self.server.software, self.server.version, runtime
                                        ),
                                    )),
                            )
                            .child(
                                IconButton::new("btn-close-server-settings", "icons/x.svg")
                                    .size(px(32.0))
                                    .color(palette.muted)
                                    .hover_bg(palette.surface_hover)
                                    .disabled(self.busy)
                                    .tooltip("Close")
                                    .on_click(cx.listener(|this, _, _, cx| this.close(cx))),
                            ),
                    )
                    .on_close(move |_, cx| {
                        let _ = entity.update(cx, |this, cx| this.close(cx));
                    })
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_3()
                                    .rounded(px(16.0))
                                    .bg(palette.card_surface())
                                    .border_1()
                                    .border_color(palette.border)
                                    .p_4()
                                    .text_color(palette.muted)
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_1p5()
                                            .child(div().text_xs().child("Name"))
                                            .child(self.name.clone()),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .gap_3()
                                            .when(!is_pumpkin, |row| {
                                                row.child(input_field("Memory (MiB)", &self.ram))
                                            })
                                            .child(input_field("Port", &self.port)),
                                    ),
                            )
                            .when(self.running, |el| {
                                el.child(
                                    div()
                                        .px_1()
                                        .text_xs()
                                        .text_color(palette.warning)
                                        .child("Stop the server to edit its settings."),
                                )
                            })
                            .children(self.error.as_ref().map(|error| {
                                div()
                                    .px_1()
                                    .text_xs()
                                    .text_color(palette.error)
                                    .child(error.clone())
                            })),
                    )
                    .footer(
                        div()
                            .flex()
                            .flex_none()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new("close-server-settings")
                                    .label("Cancel")
                                    .rounded(px(16.0))
                                    .h(px(32.0))
                                    .disabled(self.busy)
                                    .on_click(cx.listener(|this, _, _, cx| this.close(cx))),
                            )
                            .child(
                                Button::new("save-server-settings")
                                    .label(if self.busy { "Saving…" } else { "Save" })
                                    .primary()
                                    .rounded(px(16.0))
                                    .h(px(32.0))
                                    .disabled(self.busy || self.running)
                                    .on_click(cx.listener(|this, _, _, cx| this.save(cx))),
                            ),
                    ),
            )
    }
}
