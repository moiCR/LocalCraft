pub mod instances;
pub mod playit;
pub mod runtimes;
pub mod servers;

use gpui::{Animation, AnimationExt, Div, IntoElement, div, ease_out_quint, prelude::*, px};
use std::time::Duration;
use ui::theme::Palette;

fn page(
    title: &'static str,
    description: &'static str,
    count: impl gpui::IntoElement,
    palette: &Palette,
) -> Div {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w_0()
        .h_full()
        .p_8()
        .gap_8()
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
                        .gap_2()
                        .child(div().text_2xl().child(title))
                        .child(div().text_color(palette.muted).child(description)),
                )
                .child(
                    div()
                        .px_3()
                        .py_1()
                        .rounded_full()
                        .bg(palette.surface)
                        .text_xs()
                        .text_color(palette.muted)
                        .child(count),
                ),
        )
}

pub fn animate_page(page: Div, name: &'static str) -> impl IntoElement {
    page.with_animation(
        name,
        Animation::new(Duration::from_millis(240)).with_easing(ease_out_quint()),
        |page, progress| {
            page.relative()
                .top(px(10.0 * (1.0 - progress)))
                .opacity(progress)
        },
    )
}
