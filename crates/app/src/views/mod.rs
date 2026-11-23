pub mod playit;
pub mod runtimes;
pub mod servers;

use gpui::{
    App, AppContext, Div, Entity, FontWeight, Global, IntoElement, Rgba, div, linear_color_stop,
    linear_gradient, prelude::*, px,
};
use ui::theme::Palette;

pub struct AppViews {
    pub servers: Entity<servers::Servers>,
    pub runtimes: Entity<runtimes::Runtimes>,
    pub playit: Entity<playit::Playit>,
}

impl Global for AppViews {}

impl AppViews {
    pub fn init(cx: &mut App) {
        if cx.try_global::<Self>().is_none() {
            let views = Self {
                servers: cx.new(servers::Servers::new),
                runtimes: cx.new(runtimes::Runtimes::new),
                playit: cx.new(playit::Playit::new),
            };
            cx.set_global(views);
        }
    }
}

pub(super) fn panel_outer(palette: &Palette) -> Div {
    div()
        .flex()
        .flex_col()
        .size_full()
        .bg(palette.background)
        .text_color(palette.text)
        .p_5()
}

pub(super) fn panel_surface(palette: &Palette) -> Div {
    div()
        .relative()
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .w_full()
        .rounded(px(16.0))
        .bg(palette.surface)
        .border_1()
        .border_color(palette.border)
        .overflow_hidden()
}

pub(super) fn top_scroll_fade(surface: Rgba) -> Div {
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

pub(super) fn bottom_scroll_fade(surface: Rgba) -> Div {
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

pub(super) fn section_header(
    title: &'static str,
    description: &'static str,
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
                .child(div().text_xs().text_color(palette.muted).child(description)),
        )
        .child(right)
}

pub(super) fn count_badge(label: impl IntoElement, palette: &Palette) -> Div {
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
