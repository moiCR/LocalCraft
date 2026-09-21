pub mod instances;
pub mod runtimes;

use gpui::{Div, div, prelude::*};
use ui::theme::Palette;

fn page(
    title: &'static str,
    description: &'static str,
    count: &'static str,
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
