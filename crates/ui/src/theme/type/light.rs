use crate::theme::{Appearance, Palette, PrimaryColors, Theme};
use gpui::rgb;

pub struct LightTheme {
    palette: Palette,
}

impl Default for LightTheme {
    fn default() -> Self {
        let primary_fill = rgb(0x19365f);
        Self {
            palette: Palette {
                background: rgb(0xf0f4fa),
                sidebar: rgb(0xf0f4fa),
                surface: rgb(0xffffff),
                input_background: None,
                surface_hover: rgb(0xe7effa),
                border: rgb(0xd3dfee),
                text: rgb(0x152840),
                muted: rgb(0x516780),
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
