use gpui::{Div, IntoElement, div, prelude::*, px, svg};
use ui::theme::Palette;

pub fn render_icon(
    icon_path: &'static str,
    title: &'static str,
    description: &'static str,
    palette: &Palette,
) -> Div {
    render_with_icon(
        svg().path(icon_path).size_6().text_color(palette.accent),
        title,
        description,
        palette,
    )
}

fn render_with_icon(
    icon: impl IntoElement,
    title: &'static str,
    description: &'static str,
    palette: &Palette,
) -> Div {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_3()
        .border_1()
        .border_color(palette.border)
        .rounded_md()
        .p_8()
        .child(
            div()
                .size(px(56.))
                .rounded_md()
                .bg(palette.surface)
                .flex()
                .items_center()
                .justify_center()
                .child(icon),
        )
        .child(div().mt_3().text_lg().child(title))
        .child(
            div()
                .max_w(px(360.))
                .text_center()
                .text_color(palette.muted)
                .child(description),
        )
}
