use super::console_view::ConsoleView;
use super::screen::{Back, Operation, ServerScreen, ServerSection};
use gpui::{Context, FontWeight, IntoElement, Render, Window, div, prelude::*, px, svg};
use gpui_kit::base::Disableable;
use ui::{
    components::{Button, ButtonVariants, IconButton},
    theme::{ActiveUiTheme, Palette},
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
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        IconButton::new("back-to-servers", "icons/chevron-left.svg")
                            .tooltip("Back to servers")
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(Back))),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .p_1()
                            .rounded(px(16.0))
                            .bg(palette.card_surface())
                            .border_1()
                            .border_color(palette.border)
                            .child(
                                section_tab(
                                    "server-tab-console",
                                    "Console",
                                    "icons/square-terminal.svg",
                                    self.section == ServerSection::Console,
                                    &palette,
                                )
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.change_section(ServerSection::Console, cx)
                                    },
                                )),
                            )
                            .child(
                                section_tab(
                                    "server-tab-logs",
                                    "Logs",
                                    "icons/logs.svg",
                                    self.section == ServerSection::Logs,
                                    &palette,
                                )
                                .on_click(cx.listener(
                                    |this, _, _, cx| this.change_section(ServerSection::Logs, cx),
                                )),
                            )
                            .child(
                                section_tab(
                                    "server-tab-files",
                                    "Files",
                                    "icons/files.svg",
                                    self.section == ServerSection::Files,
                                    &palette,
                                )
                                .on_click(cx.listener(
                                    |this, _, _, cx| this.change_section(ServerSection::Files, cx),
                                )),
                            ),
                    ),
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
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(palette.text)
                                    .truncate()
                                    .child(self.server.name.clone()),
                            )
                            .child(div().text_xs().text_color(palette.muted).child(format!(
                                "{} · {} · localhost:{}",
                                self.server.version, self.server.software, self.server.port
                            ))),
                    )
                    .child(
                        IconButton::new("server-settings", "icons/settings.svg")
                            .tooltip("Server settings")
                            .disabled(self.busy)
                            .on_click(
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
                        Button::new("start-server")
                            .label("Start")
                            .primary()
                            .rounded(px(16.0))
                            .h(px(32.0))
                            .disabled(self.running || self.busy)
                            .on_click(
                                cx.listener(|this, _, _, cx| this.operate(Operation::Start, cx)),
                            ),
                    )
                    .child(
                        Button::new("stop-server")
                            .label("Stop")
                            .danger()
                            .rounded(px(16.0))
                            .h(px(32.0))
                            .disabled(!self.running || self.busy)
                            .on_click(
                                cx.listener(|this, _, _, cx| this.operate(Operation::Stop, cx)),
                            ),
                    )
                    .child(
                        Button::new("restart-server")
                            .label("Restart")
                            .rounded(px(16.0))
                            .h(px(32.0))
                            .disabled(!self.running || self.busy)
                            .on_click(
                                cx.listener(|this, _, _, cx| this.operate(Operation::Restart, cx)),
                            ),
                    )
                    .child(div().flex_1())
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
                            .child(div().size(px(8.0)).rounded_full().bg(status_color))
                            .child(div().text_xs().text_color(status_color).child(status_label)),
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

fn section_tab(
    id: &'static str,
    label: &'static str,
    icon: &'static str,
    active: bool,
    palette: &Palette,
) -> gpui::Stateful<gpui::Div> {
    let color = if active { palette.text } else { palette.muted };
    let surface_hover = palette.surface_hover;
    let text_color = palette.text;
    div()
        .id(id)
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .py_1p5()
        .rounded(px(12.0))
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .cursor_pointer()
        .text_color(color)
        .when(active, |tab| tab.bg(surface_hover))
        .when(!active, |tab| {
            tab.hover(move |style| style.text_color(text_color))
        })
        .child(svg().path(icon).size_4().text_color(color))
        .child(label)
}
