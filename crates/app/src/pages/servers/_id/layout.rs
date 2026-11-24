use gpui::{AnyView, Context, FontWeight, IntoElement, Window, div, prelude::*, px};
use gpui_pages::{PageLayout, router};
use ui::{
    components::{Button, ButtonVariants, IconButton, card_surface},
    theme::ActiveUiTheme,
};

use crate::components::{
    empty_state,
    server::screen::Operation,
    server_sessions::ServerSessions,
};

pub struct ServerLayout {
    child: AnyView,
}

impl PageLayout for ServerLayout {
    fn new(child: AnyView, cx: &mut Context<Self>) -> Self {
        ServerSessions::init(cx);
        let sessions = cx.global::<ServerSessions>().0.clone();
        cx.observe(&sessions, |_, _, cx| cx.notify()).detach();
        Self { child }
    }

    fn set_child(&mut self, child: AnyView) {
        self.child = child;
    }

    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.ui_palette();
        let server_id =
            router(window, cx).and_then(|manager| manager.read(cx).param("id").map(str::to_owned));
        let screen = server_id.as_deref().and_then(|id| {
            cx.try_global::<ServerSessions>()
                .and_then(|sessions| sessions.0.read(cx).screen(id))
        });

        let Some(screen) = screen else {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .p_6()
                .bg(palette.background)
                .text_color(palette.text)
                .child(div().w(px(420.0)).max_w_full().child(
                    empty_state::render_icon_with_action(
                        "icons/server.svg",
                        "Server not found",
                        "This server instance is no longer available.",
                        &palette,
                        Button::new("missing-server-back")
                            .label("Back to servers")
                            .primary()
                            .rounded(px(16.0))
                            .h(px(32.0))
                            .on_click(|_, window, cx| {
                                if let Some(manager) = router(window, cx) {
                                    manager.update(cx, |manager, cx| {
                                        let _ = manager.navigate("/", cx);
                                    });
                                }
                            }),
                    ),
                ))
                .into_any_element();
        };

        let (metadata, running, busy, status, settings) = {
            let state = screen.read(cx);
            (
                format!(
                    "{} · {} · localhost:{}",
                    state.server.version, state.server.software, state.server.port
                ),
                state.running,
                state.busy,
                state.status.clone(),
                state.settings.clone(),
            )
        };

        let status_color = if busy {
            palette.warning
        } else if running {
            palette.running
        } else {
            palette.muted
        };
        let status_label = if busy {
            "Working…"
        } else if running {
            "Running"
        } else {
            "Stopped"
        };

        let start_screen = screen.clone();
        let stop_screen = screen.clone();
        let restart_screen = screen;

        div()
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .p_5()
            .gap_4()
            .min_h_0()
            .min_w_0()
            .bg(palette.background)
            .text_color(palette.text)
            .child(
                card_surface(palette.surface, palette.surface_hover, palette.border)
                    .px_4()
                    .py_2p5()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .child(
                                IconButton::new("start-server", "icons/resume.svg")
                                    .tooltip("Start server")
                                    .color(palette.running)
                                    .background(palette.card_surface())
                                    .disabled(running || busy)
                                    .on_click(move |_, _, cx| {
                                        start_screen.update(cx, |this, cx| {
                                            this.operate(Operation::Start, cx)
                                        });
                                    }),
                            )
                            .child(
                                IconButton::new("stop-server", "icons/square.svg")
                                    .tooltip("Stop server")
                                    .color(palette.error)
                                    .background(palette.card_surface())
                                    .disabled(!running || busy)
                                    .on_click(move |_, _, cx| {
                                        stop_screen.update(cx, |this, cx| {
                                            this.operate(Operation::Stop, cx)
                                        });
                                    }),
                            )
                            .child(
                                IconButton::new("restart-server", "icons/refresh-ccw.svg")
                                    .tooltip("Restart server")
                                    .background(palette.card_surface())
                                    .disabled(!running || busy)
                                    .on_click(move |_, _, cx| {
                                        restart_screen.update(cx, |this, cx| {
                                            this.operate(Operation::Restart, cx)
                                        });
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .min_w_0()
                            .child(
                                div()
                                    .font_family("Geist Mono")
                                    .text_xs()
                                    .text_color(palette.muted)
                                    .truncate()
                                    .child(metadata),
                            )
                            .child(
                                div()
                                    .px_3()
                                    .py_1()
                                    .rounded(px(16.0))
                                    .bg(palette.card_surface())
                                    .border_1()
                                    .border_color(palette.border)
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .flex_shrink_0()
                                    .child(div().size(px(8.0)).rounded_full().bg(status_color))
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(status_color)
                                            .child(status_label),
                                    ),
                            ),
                    ),
            )
            .children(status.map(|status| {
                div()
                    .text_xs()
                    .text_color(palette.muted)
                    .child(status)
            }))
            .child(
                div()
                    .relative()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .w_full()
                    .min_h_0()
                    .min_w_0()
                    .overflow_hidden()
                    .child(self.child.clone()),
            )
            .when_some(settings, |element, settings| element.child(settings))
            .into_any_element()
    }
}
