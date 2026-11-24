use crate::theme::{Appearance, Palette, PrimaryColors, Theme};
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
                surface: rgb(0x121212),
                input_background: Some(rgb(0x1a1a1a)),
                surface_hover: rgb(0x222222),
                border: rgb(0x292929),
                text: rgb(0xededed),
                muted: rgb(0xa1a1a1),
                accent: rgb(0xffffff),
                primary: PrimaryColors {
                    fill: rgb(0xededed),
                    border: rgb(0xffffff),
                    text: rgb(0x0a0a0a),
                },
                running: rgb(0x4ade80),
                warning: rgb(0xfbbf24),
                error: rgb(0xf87171),
                info: rgb(0x60a5fa),
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
