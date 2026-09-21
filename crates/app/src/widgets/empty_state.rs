use gpui::{Div, div, prelude::*, px};
use ui::theme::Palette;

pub fn render(
    symbol: &'static str,
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
                .text_xl()
                .text_color(palette.accent)
                .child(symbol),
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
