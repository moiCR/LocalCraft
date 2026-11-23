use crate::theme::{Appearance, Palette, PrimaryColors, Theme};
use gpui::rgb;

pub struct LightTheme {
    palette: Palette,
}

impl Default for LightTheme {
    fn default() -> Self {
        let primary_fill = rgb(0x195f36);
        Self {
            palette: Palette {
                background: rgb(0xf0faf4),
                sidebar: rgb(0xf0faf4),
                surface: rgb(0xffffff),
                input_background: None,
                surface_hover: rgb(0xe7faef),
                border: rgb(0xd3eede),
                text: rgb(0x154027),
                muted: rgb(0x518065),
                accent: primary_fill,
                primary: PrimaryColors {
                    fill: primary_fill,
                    border: primary_fill,
                    text: rgb(0xffffff),
                },
                running: rgb(0x166b50),
                warning: rgb(0x8f530c),
                error: rgb(0xac3443),
                info: rgb(0x006d83),
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
