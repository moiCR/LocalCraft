use super::console_view::ConsoleView;
use super::screen::{Back, Operation, ServerScreen};
use gpui::{Context, IntoElement, Render, Window, div, prelude::*};
use services::AppState;
use ui::components::button::button;

impl Render for ServerScreen {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.global::<AppState>().theme_manager.palette();
        div()
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .p_6()
            .gap_4()
            .min_h_0()
            .min_w_0()
            .child(
                div()
                    .id("back-to-servers")
                    .text_color(palette.muted)
                    .cursor_pointer()
                    .child("‹ Servers")
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(Back))),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .min_w_0()
                            .child(div().text_2xl().child(self.server.name.clone()))
                            .child(div().text_color(palette.muted).child(format!(
                                "{} · {} · localhost:{}",
                                self.server.version, self.server.software, self.server.port
                            ))),
                    )
                    .child(
                        button("server-settings", "Settings", palette, false, !self.busy).on_click(
                            cx.listener(|this, _, window, cx| this.open_settings(window, cx)),
                        ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .py_3()
                    .border_y_1()
                    .border_color(palette.border)
                    .child(
                        button(
                            "start-server",
                            "Start",
                            palette,
                            true,
                            !self.running && !self.busy,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.operate(Operation::Start, cx))),
                    )
                    .child(
                        button(
                            "stop-server",
                            "Stop",
                            palette,
                            false,
                            self.running && !self.busy,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.operate(Operation::Stop, cx))),
                    )
                    .child(
                        button(
                            "restart-server",
                            "Restart",
                            palette,
                            false,
                            self.running && !self.busy,
                        )
                        .on_click(
                            cx.listener(|this, _, _, cx| this.operate(Operation::Restart, cx)),
                        ),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_xs()
                            .text_color(palette.muted)
                            .child(if self.busy {
                                "Working…"
                            } else if self.running {
                                "Running"
                            } else {
                                "Stopped"
                            }),
                    ),
            )
            .children(self.status.as_ref().map(|status| {
                div()
                    .text_xs()
                    .text_color(palette.muted)
                    .child(status.clone())
            }))
            .child(ConsoleView::cached(self.console.clone()))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .pt_3()
                    .border_t_1()
                    .border_color(palette.border)
                    .child(div().flex_1().min_w_0().child(self.command.clone()))
                    .child(
                        button(
                            "send-command",
                            "Send",
                            palette,
                            true,
                            self.running && !self.busy,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.send_command(cx))),
                    ),
            )
            .when_some(self.settings.clone(), |element, settings| {
                element.child(settings)
            })
    }
}
