use crate::theme::{Appearance, Palette, PrimaryColors, Theme};
use gpui::rgb;

pub struct DarkTheme {
    palette: Palette,
}

impl Default for DarkTheme {
    fn default() -> Self {
        Self {
            palette: Palette {
                background: rgb(0x070807),
                sidebar: rgb(0x070807),
                surface: rgb(0x0d110e),
                input_background: Some(rgb(0x151a17)),
                surface_hover: rgb(0x1c211e),
                border: rgb(0x272c29),
                text: rgb(0xa0a8a3),
                muted: rgb(0x77807a),
                accent: rgb(0x91d4ad),
                primary: PrimaryColors {
                    fill: rgb(0x22613c),
                    border: rgb(0x53946e),
                    text: rgb(0xf2fcf6),
                },
                running: rgb(0x6ed9ad),
                warning: rgb(0xf4bc64),
                error: rgb(0xff9c9c),
                info: rgb(0x4e909c),
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
