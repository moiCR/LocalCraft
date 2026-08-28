#[derive(Clone)]
pub struct AppState {}

impl gpui::Global for AppState {}

impl AppState{
    pub fn new() -> Self{

        Self{}
    }
}


