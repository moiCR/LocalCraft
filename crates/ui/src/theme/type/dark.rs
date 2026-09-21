use crate::theme::{Appearance, Palette, Theme};
use gpui::rgb;

pub struct DarkTheme {
    palette: Palette,
}

impl Default for DarkTheme {
    fn default() -> Self {
        Self {
            palette: Palette {
                background: rgb(0x0a0a0a),
                sidebar: rgb(0x0e0e0e),
                surface: rgb(0x1a1a1a),
                border: rgb(0x292929),
                text: rgb(0xededed),
                muted: rgb(0xa1a1a1),
                accent: rgb(0xffffff),
            },
        }
    }
}

impl Theme for DarkTheme {
    fn appearance(&self) -> Appearance {
        Appearance::Dark
    }

    fn palette(&self) -> &Palette {
        &self.palette
    }
}
