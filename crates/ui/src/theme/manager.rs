use super::{
    Appearance, Palette, Theme,
    r#type::{dark::DarkTheme, light::LightTheme},
};

pub struct ThemeManager {
    current_theme: Box<dyn Theme>,
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new(Box::<DarkTheme>::default())
    }
}

impl ThemeManager {
    pub fn new(theme: Box<dyn Theme>) -> Self {
        Self {
            current_theme: theme,
        }
    }

    pub fn appearance(&self) -> Appearance {
        self.current_theme.appearance()
    }

    pub fn set_appearance(&mut self, appearance: Appearance) {
        if self.appearance() == appearance {
            return;
        }
        self.set_theme(match appearance {
            Appearance::Dark => Box::<DarkTheme>::default(),
            Appearance::Light => Box::<LightTheme>::default(),
        });
    }

    pub fn palette(&self) -> &Palette {
        self.current_theme.palette()
    }

    pub fn set_theme(&mut self, theme: Box<dyn Theme>) {
        self.current_theme = theme;
    }
}
