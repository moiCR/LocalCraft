use crate::workspace::{Page, Workspace};
use gpui::{Context, IntoElement, div, prelude::*, px, svg};
use services::AppState;

pub fn render(active: Page, cx: &Context<Workspace>) -> impl IntoElement {
    let palette = cx.global::<AppState>().theme_manager.palette();
    div()
        .flex()
        .flex_col()
        .w(px(224.))
        .h_full()
        .flex_shrink_0()
        .bg(palette.sidebar)
        .border_r_1()
        .border_color(palette.border)
        .p_4()
        .gap_8()
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .px_2()
                .py_4()
                .child(
                    div()
                        .size_8()
                        .rounded_lg()
                        .bg(palette.accent)
                        .text_color(palette.background)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child("L"),
                )
                .child(div().text_base().child("LocalCraft")),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .px_3()
                        .pb_2()
                        .text_xs()
                        .text_color(palette.muted)
                        .child("Workspace"),
                )
                .children(
                    [
                        (
                            Page::Instances,
                            "Instances",
                            "instances",
                            "icons/server.svg",
                        ),
                        (Page::Runtimes, "Runtimes", "runtimes", "icons/coffee.svg"),
                    ]
                    .into_iter()
                    .map(|(page, label, id, icon)| {
                        div()
                            .id(id)
                            .flex()
                            .items_center()
                            .gap_3()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .cursor_pointer()
                            .bg(if active == page {
                                palette.surface
                            } else {
                                palette.sidebar
                            })
                            .text_color(if active == page {
                                palette.text
                            } else {
                                palette.muted
                            })
                            .hover(|style| style.bg(palette.surface).text_color(palette.text))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.active_page = page;
                                cx.notify();
                            }))
                            .child(svg().path(icon).size_4().flex_shrink_0())
                            .child(label)
                    }),
                ),
        )
        .child(div().flex_1())
        .child(
            div()
                .border_t_1()
                .border_color(palette.border)
                .pt_4()
                .px_2()
                .child(
                    div()
                        .id("open-settings")
                        .flex()
                        .items_center()
                        .justify_between()
                        .w_full()
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .cursor_pointer()
                        .text_color(palette.muted)
                        .hover(|style| style.bg(palette.surface).text_color(palette.text))
                        .on_click(cx.listener(|this, _, window, cx| this.open_settings(window, cx)))
                        .child("Settings")
                        .child(div().text_xs().child("Preferences")),
                ),
        )
}
