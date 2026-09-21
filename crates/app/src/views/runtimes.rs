use crate::widgets::empty_state;
use gpui::{Div, prelude::*};

pub fn render(palette: &ui::theme::Palette) -> Div {
    super::page(
        "Runtimes",
        "Java installations for your Minecraft servers.",
        "0 runtimes",
        palette,
    )
    .child(empty_state::render(
        "J",
        "No runtimes added",
        "Manage the Java installations used by your instances here.",
        palette,
    ))
}
