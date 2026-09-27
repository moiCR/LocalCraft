use crate::theme::Palette;
use gpui::{Div, ElementId, SharedString, Stateful, div, prelude::*};

pub fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    palette: &Palette,
    primary: bool,
    enabled: bool,
) -> Stateful<Div> {
    div()
        .id(id)
        .px_4()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(if primary {
            palette.text
        } else {
            palette.border
        })
        .bg(if primary {
            palette.text
        } else {
            palette.background
        })
        .text_color(if primary {
            palette.background
        } else {
            palette.text
        })
        .opacity(if enabled { 1. } else { 0.4 })
        .when(enabled, |button| {
            button.cursor_pointer().hover(|style| style.opacity(0.8))
        })
        .child(label.into())
}
