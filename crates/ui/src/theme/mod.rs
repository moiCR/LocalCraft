pub mod manager;
pub mod r#type;

use gpui::{App, Context, Global, Rgba, rgb};
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

    let primary_hover = if mode.is_dark() {
        palette.primary.border
    } else {
        rgb(0x217545)
    };

    ComponentTheme::update(cx, |theme| {
        theme.colors.background = palette.background.into();
        theme.colors.popover = palette.surface.into();
        theme.colors.list_hover = palette.surface_hover.into();
        theme.colors.border = palette.border.into();
        theme.colors.input = palette.border.into();
        theme.colors.foreground = palette.text.into();
        theme.colors.muted_foreground = palette.muted.into();
        theme.colors.primary = palette.primary.fill.into();
        theme.colors.primary_hover = primary_hover.into();
        theme.colors.primary_active = palette.primary.fill.into();
        theme.colors.primary_foreground = palette.primary.text.into();
        theme.colors.button_primary = palette.primary.fill.into();
        theme.colors.button_primary_hover = primary_hover.into();
        theme.colors.button_primary_active = palette.primary.fill.into();
        theme.colors.button_primary_foreground = palette.primary.text.into();
        theme.colors.accent = palette.surface_hover.into();
        theme.colors.accent_foreground = palette.text.into();
        theme.colors.ring = palette.accent.into();
        theme.colors.progress_bar = palette.accent.into();
        theme.colors.slider_bar = palette.accent.into();
        theme.colors.link = palette.accent.into();
        theme.colors.link_hover = primary_hover.into();
        theme.colors.link_active = palette.primary.fill.into();
        theme.colors.caret = palette.accent.into();
    });
}
