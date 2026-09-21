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
                sidebar: rgb(0xfafafa),
                surface: rgb(0xf0f0f0),
                border: rgb(0xe5e5e5),
                text: rgb(0x171717),
                muted: rgb(0x666666),
                accent: rgb(0x171717),
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
