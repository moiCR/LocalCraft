use super::console_view::ConsoleView;
use super::screen::{Operation, ServerScreen, ServerSection};
use gpui::{Context, FontWeight, IntoElement, Render, Window, div, prelude::*, px};
use gpui_kit::base::Disableable;
use ui::{
    components::{Button, ButtonVariants, card_surface},
    theme::ActiveUiTheme,
};

impl Render for ServerScreen {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.ui_palette();
        let console = self.console.clone();
        let logs = self.logs.clone();
        let files = self.files.clone();
        let status_color = if self.busy {
            palette.warning
        } else if self.running {
            palette.running
        } else {
            palette.muted
        };
        let status_label = if self.busy {
            "Working…"
        } else if self.running {
            "Running"
        } else {
            "Stopped"
        };

        div()
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .p_5()
            .gap_4()
            .min_h_0()
            .min_w_0()
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
                            .gap_2()
                            .child(
                                Button::new("start-server")
                                    .label("Start")
                                    .primary()
                                    .rounded(px(16.0))
                                    .h(px(32.0))
                                    .disabled(self.running || self.busy)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.operate(Operation::Start, cx)
                                    })),
                            )
                            .child(
                                Button::new("stop-server")
                                    .label("Stop")
                                    .danger()
                                    .rounded(px(16.0))
                                    .h(px(32.0))
                                    .disabled(!self.running || self.busy)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.operate(Operation::Stop, cx)
                                    })),
                            )
                            .child(
                                Button::new("restart-server")
                                    .label("Restart")
                                    .rounded(px(16.0))
                                    .h(px(32.0))
                                    .disabled(!self.running || self.busy)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.operate(Operation::Restart, cx)
                                    })),
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
                                    .child(format!(
                                        "{} · {} · localhost:{}",
                                        self.server.version, self.server.software, self.server.port
                                    )),
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
            .children(self.status.as_ref().map(|status| {
                div()
                    .text_xs()
                    .text_color(palette.muted)
                    .child(status.clone())
            }))
            .when(self.section == ServerSection::Console, |element| {
                element.child(ConsoleView::cached(console))
            })
            .when(self.section == ServerSection::Logs, |element| {
                element.child(logs)
            })
            .when(self.section == ServerSection::Files, |element| {
                element.child(files)
            })
            .when(self.section == ServerSection::Console, |element| {
                element.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .pt_3()
                        .border_t_1()
                        .border_color(palette.border)
                        .child(div().flex_1().min_w_0().child(self.command.clone()))
                        .child(
                            Button::new("send-command")
                                .label("Send")
                                .primary()
                                .rounded(px(16.0))
                                .h(px(32.0))
                                .disabled(!self.running || self.busy)
                                .on_click(cx.listener(|this, _, _, cx| this.send_command(cx))),
                        ),
                )
            })
            .when_some(self.settings.clone(), |element, settings| {
                element.child(settings)
            })
    }
}
