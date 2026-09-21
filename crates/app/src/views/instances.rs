use crate::widgets::empty_state;
use gpui::{Div, prelude::*};

pub fn render(palette: &ui::theme::Palette) -> Div {
    super::page(
        "Instances",
        "Your Minecraft servers, all in one place.",
        "0 servers",
        palette,
    )
    .child(empty_state::render(
        "◇",
        "No instances yet",
        "Your local Minecraft servers will appear here.",
        palette,
    ))
}
