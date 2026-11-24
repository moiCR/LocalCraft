use gpui::{prelude::FluentBuilder as _, *};
use gpui_kit::base::Disableable;
use services::{AppState, instance::ServerInstance};
use ui::{
    components::{Button, ButtonVariants, Modal},
    theme::ActiveUiTheme,
};

use crate::components::server_sessions::ServerSessions;

#[derive(Default, Clone)]
pub struct DeleteInstanceModalState {
    pub open: bool,
    pub server: Option<ServerInstance>,
    pub busy: bool,
    pub error: Option<String>,
}

impl Global for DeleteInstanceModalState {}

#[derive(IntoElement)]
pub struct DeleteInstanceModal;

impl DeleteInstanceModal {
    pub fn new() -> Self {
        Self
    }

    pub fn open(server: ServerInstance, window: &mut Window, cx: &mut App) {
        cx.set_global(DeleteInstanceModalState {
            open: true,
            server: Some(server),
            busy: false,
            error: None,
        });
        window.refresh();
    }

    fn close(window: &mut Window, cx: &mut App) {
        if cx
            .try_global::<DeleteInstanceModalState>()
            .is_some_and(|state| state.busy)
        {
            return;
        }
        cx.set_global(DeleteInstanceModalState::default());
        window.blur(cx);
        window.refresh();
    }

    fn confirm(cx: &mut App) {
        let state = cx
            .try_global::<DeleteInstanceModalState>()
            .cloned()
            .unwrap_or_default();
        if state.busy {
            return;
        }
        let Some(server) = state.server else {
            return;
        };
        let id = server.id.clone();
        let runtime = cx.global::<AppState>().background_runtime.clone();
        cx.set_global(DeleteInstanceModalState {
            open: true,
            server: Some(server.clone()),
            busy: true,
            error: None,
        });
        cx.refresh_windows();

        let task = runtime.spawn(async move { server.delete().await });
        cx.spawn(async move |cx| {
            let result = task
                .await
                .map_err(|error| format!("Instance deletion task failed: {error}"))
                .and_then(|result| result.map_err(|error| format!("{error:#}")));
            cx.update(|cx| {
                match result {
                    Ok(()) => {
                        cx.update_global::<AppState, _>(|state, _| {
                            state.instance_service.remove(&id);
                        });
                        if let Some(sessions) = cx.try_global::<ServerSessions>() {
                            let store = sessions.0.clone();
                            store.update(cx, |this, cx| this.on_instance_deleted(&id, cx));
                        }
                        cx.set_global(DeleteInstanceModalState::default());
                    }
                    Err(error) => {
                        let prev = cx
                            .try_global::<DeleteInstanceModalState>()
                            .and_then(|state| state.server.clone());
                        cx.set_global(DeleteInstanceModalState {
                            open: true,
                            server: prev,
                            busy: false,
                            error: Some(error),
                        });
                    }
                }
                cx.refresh_windows();
            });
        })
        .detach();
    }
}

impl Default for DeleteInstanceModal {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for DeleteInstanceModal {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let modal_state = cx
            .try_global::<DeleteInstanceModalState>()
            .cloned()
            .unwrap_or_default();
        let is_open = modal_state.open && modal_state.server.is_some();
        let busy = modal_state.busy;
        let error = modal_state.error;
        let (server_name, server_meta) = modal_state
            .server
            .as_ref()
            .map(|server| {
                let meta = if services::software::pumpkin::is_pumpkin(&server.software) {
                    format!("{} · {} · Native runtime", server.version, server.software)
                } else {
                    format!(
                        "{} · {} · {} MiB",
                        server.version, server.software, server.ram
                    )
                };
                (server.name.clone(), meta)
            })
            .unwrap_or_default();

        let palette = cx.ui_palette();
        let viewport = window.viewport_size();

        div()
            .id("delete-instance-modal-root")
            .absolute()
            .inset_0()
            .size_full()
            .capture_key_down(move |event: &KeyDownEvent, window, cx| {
                if is_open && event.keystroke.key == "escape" {
                    Self::close(window, cx);
                    cx.stop_propagation();
                }
            })
            .child(
                Modal::new("delete-instance-modal")
                    .open(is_open)
                    .compact()
                    .fit_content()
                    .width((viewport.width - px(32.0)).clamp(px(0.0), px(480.0)))
                    .max_height((viewport.height - px(32.0)).max(px(0.0)))
                    .header(
                        div()
                            .flex()
                            .flex_none()
                            .items_center()
                            .px_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(palette.text)
                                    .child("Delete instance"),
                            ),
                    )
                    .on_close(Self::close)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .rounded(px(16.0))
                                    .bg(palette.card_surface())
                                    .border_1()
                                    .border_color(palette.border)
                                    .px_4()
                                    .py_3()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .gap_3()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(palette.text)
                                                    .truncate()
                                                    .child(server_name),
                                            )
                                            .child(
                                                div()
                                                    .flex_none()
                                                    .text_xs()
                                                    .text_color(palette.muted)
                                                    .child(server_meta),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(palette.muted)
                                            .child("This permanently removes all server files and stops the server if running."),
                                    ),
                            )
                            .when_some(error, |container, error| {
                                container.child(
                                    div()
                                        .text_xs()
                                        .text_color(palette.error)
                                        .px_1()
                                        .child(error),
                                )
                            }),
                    )
                    .footer(
                        div()
                            .flex()
                            .flex_none()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new("btn-cancel-delete-instance")
                                    .label("Cancel")
                                    .rounded(px(16.0))
                                    .h(px(32.0))
                                    .disabled(busy)
                                    .on_click(move |_event, window, cx| {
                                        Self::close(window, cx);
                                    }),
                            )
                            .child(
                                Button::new("btn-confirm-delete-instance")
                                    .label(if busy {
                                        "Deleting…"
                                    } else {
                                        "Delete permanently"
                                    })
                                    .danger()
                                    .rounded(px(16.0))
                                    .h(px(32.0))
                                    .disabled(busy)
                                    .on_click(move |_event, _window, cx| {
                                        Self::confirm(cx);
                                    }),
                            ),
                    ),
            )
    }
}
