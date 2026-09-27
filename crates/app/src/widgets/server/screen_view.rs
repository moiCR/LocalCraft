use super::console_view::ConsoleView;
use super::screen::{Back, Operation, ServerScreen, ServerSection};
use gpui::{Context, IntoElement, Render, Window, div, prelude::*, svg};
use services::AppState;
use ui::components::button::button;

impl Render for ServerScreen {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.global::<AppState>().theme_manager.palette();
        let console = self.console.clone();
        let logs = self.logs.clone();
        let files = self.files.clone();
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
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .id("back-to-servers")
                            .size_9()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_md()
                            .text_color(palette.muted)
                            .cursor_pointer()
                            .hover(|style| style.bg(palette.surface).text_color(palette.text))
                            .child(
                                svg()
                                    .path("icons/chevron-left.svg")
                                    .size_4()
                                    .text_color(palette.muted),
                            )
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(Back))),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                section_tab(
                                    "server-tab-console",
                                    "Console",
                                    "icons/square-terminal.svg",
                                    self.section == ServerSection::Console,
                                    palette,
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
                                    palette,
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
                                    palette,
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
    palette: &ui::theme::Palette,
) -> gpui::Stateful<gpui::Div> {
    let color = if active { palette.text } else { palette.muted };
    div()
        .id(id)
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .py_2()
        .rounded_md()
        .text_color(color)
        .when(active, |tab| tab.bg(palette.surface))
        .when(!active, |tab| {
            tab.hover(|style| style.bg(palette.surface).text_color(palette.text))
        })
        .child(svg().path(icon).size_4().text_color(color))
        .child(label)
}
