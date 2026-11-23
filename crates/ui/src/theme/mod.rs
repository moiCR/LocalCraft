pub mod manager;
pub mod r#type;

use gpui::{App, Context, Global, Rgba};
use gpui_kit::component::{Theme as ComponentTheme, ThemeMode};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Appearance {
    Dark,
    Light,
}

#[derive(Clone, Copy)]
pub struct PrimaryColors {
    pub fill: Rgba,
    pub border: Rgba,
    pub text: Rgba,
}

pub trait Theme {
    fn appearance(&self) -> Appearance;
    fn palette(&self) -> &Palette;
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub background: Rgba,
    pub sidebar: Rgba,
    pub surface: Rgba,
    pub input_background: Option<Rgba>,
    pub surface_hover: Rgba,
    pub border: Rgba,
    pub text: Rgba,
    pub muted: Rgba,
    pub accent: Rgba,
    pub primary: PrimaryColors,
    pub running: Rgba,
    pub warning: Rgba,
    pub error: Rgba,
    pub info: Rgba,
}

impl Palette {
    pub fn card_surface(&self) -> Rgba {
        self.input_background.unwrap_or(self.background)
    }
}

#[derive(Clone, Copy)]
pub struct UiTheme(pub Palette);

impl Global for UiTheme {}

impl Default for UiTheme {
    fn default() -> Self {
        Self(*r#type::dark::DarkTheme::default().palette())
    }
}

pub trait ActiveUiTheme {
    fn ui_palette(&self) -> Palette;
}

impl ActiveUiTheme for App {
    fn ui_palette(&self) -> Palette {
        self.try_global::<UiTheme>()
            .map(|theme| theme.0)
            .unwrap_or_else(|| UiTheme::default().0)
    }
}

impl<'a, T> ActiveUiTheme for Context<'a, T> {
    fn ui_palette(&self) -> Palette {
        self.try_global::<UiTheme>()
            .map(|theme| theme.0)
            .unwrap_or_else(|| UiTheme::default().0)
    }
}

pub fn sync_component_theme(appearance: Appearance, cx: &mut App) {
    let (palette, mode) = match appearance {
        Appearance::Dark => (
            *r#type::dark::DarkTheme::default().palette(),
            ThemeMode::Dark,
        ),
        Appearance::Light => (
            *r#type::light::LightTheme::default().palette(),
            ThemeMode::Light,
        ),
    };
    cx.set_global(UiTheme(palette));
    ComponentTheme::change(mode, None, cx);

    if mode.is_dark() {
        ComponentTheme::update(cx, |theme| {
            theme.colors.background = palette.background.into();
            theme.colors.popover = palette.surface.into();
            theme.colors.list_hover = palette.surface_hover.into();
            theme.colors.border = palette.border.into();
            theme.colors.input = palette.border.into();
            theme.colors.foreground = palette.text.into();
            theme.colors.muted_foreground = palette.muted.into();
        });
    }
}
