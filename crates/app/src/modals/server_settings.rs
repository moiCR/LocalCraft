use super::create_server_view::input_field;
use gpui::{
    AppContext, Context, Entity, EventEmitter, FocusHandle, IntoElement, MouseButton, Render,
    Window, div, prelude::*, px, rgba,
};
use services::{
    AppState,
    instance::{ServerInstance, configuration::validate_settings},
};
use ui::components::{button::button, input::Input};

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
        let parsed = self
            .ram
            .read(cx)
            .value()
            .parse::<u32>()
            .ok()
            .zip(self.port.read(cx).value().parse::<u16>().ok());
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
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.global::<AppState>().theme_manager.palette();
        div()
            .id("server-settings-overlay")
            .absolute()
            .inset_0()
            .size_full()
            .bg(rgba(0x00000099))
            .flex()
            .items_center()
            .justify_center()
            .p_4()
            .occlude()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| this.close(cx)),
            )
            .child(
                div()
                    .id("server-settings-dialog")
                    .w(px(440.))
                    .max_w_full()
                    .bg(p.background)
                    .rounded_lg()
                    .border_1()
                    .border_color(p.border)
                    .p_6()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .track_focus(&self.focus)
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                        if event.keystroke.key == "escape" {
                            this.close(cx);
                            cx.stop_propagation();
                        }
                    }))
                    .child(div().text_lg().child("Server settings"))
                    .child(input_field("Name", &self.name))
                    .child(
                        div()
                            .flex()
                            .gap_3()
                            .child(input_field("Memory (MiB)", &self.ram))
                            .child(input_field("Port", &self.port)),
                    )
                    .child(div().text_xs().text_color(p.muted).child(format!(
                            "{} · {} · Java {}",
                            self.server.software,
                            self.server.version,
                            self.server
                                .java_version
                                .as_deref()
                                .unwrap_or("not selected")
                        )))
                    .when(self.running, |el| {
                        el.child(
                            div()
                                .text_color(p.muted)
                                .child("Stop the server to edit its settings."),
                        )
                    })
                    .children(
                        self.error.as_ref().map(|error| {
                            div().text_color(gpui::rgb(0xe87878)).child(error.clone())
                        }),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                button("close-server-settings", "Cancel", p, false, !self.busy)
                                    .on_click(cx.listener(|this, _, _, cx| this.close(cx))),
                            )
                            .child(
                                button(
                                    "save-server-settings",
                                    "Save",
                                    p,
                                    true,
                                    !self.busy && !self.running,
                                )
                                .on_click(cx.listener(|this, _, _, cx| this.save(cx))),
                            ),
                    ),
            )
    }
}
