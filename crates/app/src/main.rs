mod modals;
mod views;
mod widgets;
mod workspace;

use gpui::{App, AppContext, Application, Bounds, WindowBounds, WindowOptions, px, size};
use workspace::Workspace;

fn main() {
    Application::new()
        .with_assets(assets::Assets)
        .run(|cx: &mut App| {
            cx.set_global(services::AppState::new());
            if let Err(error) = cx.text_system().add_fonts(assets::load_fonts()) {
                eprintln!("Failed to load application fonts: {error}");
            }
            let bounds = Bounds::centered(None, size(px(1100.), px(720.)), cx);
            let result = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(720.), px(480.))),
                    titlebar: Some(gpui::TitlebarOptions {
                        title: Some("LocalCraft".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |_, cx| cx.new(Workspace::new),
            );
            if let Err(error) = result {
                eprintln!("Failed to open LocalCraft: {error}");
                cx.quit();
                return;
            }
            cx.on_window_closed(|cx| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            cx.activate(true);
        });
}
