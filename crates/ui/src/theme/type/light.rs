use crate::theme::{Appearance, Palette, PrimaryColors, Theme};
use gpui::rgb;

pub struct LightTheme {
    palette: Palette,
}

impl Default for LightTheme {
    fn default() -> Self {
        let primary_fill = rgb(0x141414);
        Self {
            palette: Palette {
                background: rgb(0xfafafa),
                sidebar: rgb(0xf4f4f5),
                surface: rgb(0xffffff),
                input_background: None,
                surface_hover: rgb(0xececef),
                border: rgb(0xd4d4d8),
                text: rgb(0x0d0d0d),
                muted: rgb(0x666666),
                accent: primary_fill,
                primary: PrimaryColors {
                    fill: primary_fill,
                    border: primary_fill,
                    text: rgb(0xffffff),
                },
                running: rgb(0x16a34a),
                warning: rgb(0xb45309),
                error: rgb(0xdc2626),
                info: rgb(0x2563eb),
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
