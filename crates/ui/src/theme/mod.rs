pub mod manager;
pub mod r#type;

use gpui::Rgba;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Appearance {
    Dark,
    Light,
}

pub trait Theme {
    fn appearance(&self) -> Appearance;
    fn palette(&self) -> &Palette;
}

pub struct Palette {
    pub background: Rgba,
    pub sidebar: Rgba,
    pub surface: Rgba,
    pub border: Rgba,
    pub text: Rgba,
    pub muted: Rgba,
    pub accent: Rgba,
}
