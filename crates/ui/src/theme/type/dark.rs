use crate::theme::{Appearance, Palette, PrimaryColors, Theme};
use gpui::rgb;

pub struct DarkTheme {
    palette: Palette,
}

impl Default for DarkTheme {
    fn default() -> Self {
        Self {
            palette: Palette {
                background: rgb(0x070708),
                sidebar: rgb(0x070708),
                surface: rgb(0x0d0d11),
                input_background: Some(rgb(0x15151a)),
                surface_hover: rgb(0x1c1c21),
                border: rgb(0x27272c),
                text: rgb(0xa0a0a8),
                muted: rgb(0x777780),
                accent: rgb(0x91add4),
                primary: PrimaryColors {
                    fill: rgb(0x223c61),
                    border: rgb(0x536f94),
                    text: rgb(0xf2f6fc),
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
