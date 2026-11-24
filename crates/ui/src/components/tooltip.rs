use gpui::*;

use crate::theme::ActiveUiTheme;

pub struct ActionTooltip {
    label: SharedString,
    shortcut: Option<SharedString>,
}

impl ActionTooltip {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            shortcut: None,
        }
    }

    pub fn shortcut(mut self, shortcut: impl Into<SharedString>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }
}

impl Render for ActionTooltip {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.ui_palette();

        div()
            .flex()
            .items_center()
            .gap_2()
            .px_2p5()
            .py_1()
            .rounded(px(16.0))
            .bg(palette.surface)
            .border_1()
            .border_color(palette.border)
            .shadow_md()
            .text_xs()
            .child(div().text_color(palette.text).child(self.label.clone()))
            .children(self.shortcut.as_ref().map(|shortcut| {
                div()
                    .px_1p5()
                    .py_0p5()
                    .rounded(px(8.0))
                    .bg(palette.surface_hover)
                    .text_color(palette.muted)
                    .child(shortcut.clone())
            }))
    }
}
