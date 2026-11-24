use gpui::{
    Div, FontWeight, IntoElement, Rgba, SharedString, div, linear_color_stop, linear_gradient,
    prelude::*, px,
};
use ui::theme::Palette;

pub fn canvas_outer(palette: &Palette) -> Div {
    div()
        .relative()
        .flex()
        .flex_col()
        .size_full()
        .min_h_0()
        .min_w_0()
        .overflow_hidden()
        .bg(palette.background)
        .text_color(palette.text)
}

pub fn top_scroll_fade(surface: Rgba) -> Div {
    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .h(px(20.0))
        .bg(linear_gradient(
            180.0,
            linear_color_stop(surface, 0.0),
            linear_color_stop(surface.opacity(0.0), 1.0),
        ))
}

pub fn bottom_scroll_fade(surface: Rgba) -> Div {
    div()
        .absolute()
        .bottom_0()
        .left_0()
        .right_0()
        .h(px(56.0))
        .bg(linear_gradient(
            180.0,
            linear_color_stop(surface.opacity(0.0), 0.0),
            linear_color_stop(surface, 1.0),
        ))
}

pub fn section_header(
    title: &'static str,
    description: impl Into<SharedString>,
    right: impl IntoElement,
    palette: &Palette,
) -> Div {
    div()
        .w_full()
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
                        .child(title),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(palette.muted)
                        .child(description.into()),
                ),
        )
        .child(right)
}

pub fn count_badge(label: impl IntoElement, palette: &Palette) -> Div {
    div()
        .px_3()
        .py_1()
        .rounded(px(16.0))
        .bg(palette.card_surface())
        .border_1()
        .border_color(palette.border)
        .text_xs()
        .text_color(palette.muted)
        .child(label)
}
