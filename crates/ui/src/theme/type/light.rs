use crate::theme::{Appearance, Palette, Theme};
use gpui::rgb;

pub struct LightTheme {
    palette: Palette,
}

impl Default for LightTheme {
    fn default() -> Self {
        Self {
            palette: Palette {
                background: rgb(0xffffff),
                sidebar: rgb(0xCFCFCF),
                surface: rgb(0xCFCFCF),
                border: rgb(0x9E9E9E),
                text: rgb(0x0D0D0D),
                muted: rgb(0x666666),
                accent: rgb(0xB6B6B6),
            },
        }
    }
}

impl Theme for LightTheme {
    fn appearance(&self) -> Appearance {
        Appearance::Light
    }

    fn palette(&self) -> &Palette {
        &self.palette
    }
}
