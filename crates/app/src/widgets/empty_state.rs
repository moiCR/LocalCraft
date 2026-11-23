use gpui::{Div, FontWeight, IntoElement, SharedString, div, prelude::*, px, svg};
use ui::theme::Palette;

pub fn render_icon(
    icon_path: &'static str,
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
    palette: &Palette,
) -> Div {
    base(icon_path, title, description, palette)
}

pub fn render_icon_with_action(
    icon_path: &'static str,
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
    palette: &Palette,
    action: impl IntoElement,
) -> Div {
    base(icon_path, title, description, palette).child(div().pt_1().child(action))
}

fn base(
    icon_path: &'static str,
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
    palette: &Palette,
) -> Div {
    div()
        .w_full()
        .py_12()
        .px_6()
        .rounded(px(16.0))
        .bg(palette.card_surface())
        .border_1()
        .border_color(palette.border)
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .text_center()
        .gap_3()
        .child(
            div()
                .size(px(44.0))
                .rounded(px(16.0))
                .bg(palette.surface)
                .border_1()
                .border_color(palette.border)
                .flex()
                .items_center()
                .justify_center()
                .child(
                    svg()
                        .path(icon_path)
                        .size(px(20.0))
                        .text_color(palette.muted),
                ),
        )
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(palette.text)
                .child(title.into()),
        )
        .child(
            div()
                .max_w(px(360.0))
                .text_xs()
                .text_color(palette.muted)
                .child(description.into()),
        )
}
