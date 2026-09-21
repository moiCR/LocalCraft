use ui::theme::manager::ThemeManager;

#[derive(Default)]
pub struct AppState {
    pub theme_manager: ThemeManager,
}

impl gpui::Global for AppState {}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }
}
