#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod components;
mod layout;
mod modals;
#[rustfmt::skip]
#[path = "pages_modules.rs"]
pub mod pages;

use gpui::{App, AppContext, Bounds, WindowBounds, WindowOptions, px, size};
use gpui_pages::{ApplicationExt, PageManager};

gpui_pages::include_pages!();

fn main() {
    let runtime = match services::instance::InstancesService::runtime() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("Failed to initialize background runtime: {error:#}");
            return;
        }
    };
    let state = match runtime.block_on(services::AppState::new()) {
        Ok(state) => state,
        Err(error) => {
            eprintln!("Failed to initialize application state: {error:#}");
            return;
        }
    };
    let shutdown = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let shutdown_capture = shutdown.clone();
    gpui_kit::application()
        .with_assets(assets::Assets)
        .use_pages()
        .run_with(move |cx: &mut App| {
            let appearance = state.theme_manager.appearance();
            cx.set_global(state);
            gpui_kit::init(cx);
            ui::theme::sync_component_theme(appearance, cx);
            ui::components::input::init(cx);
            cx.on_app_quit(move |cx| {
                let mut servers = shutdown_capture
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                *servers = cx
                    .global::<services::AppState>()
                    .instance_service
                    .servers()
                    .to_vec();
                async {}
            })
            .detach();
            if let Err(error) = cx.text_system().add_fonts(assets::load_fonts()) {
                eprintln!("Failed to load application fonts: {error}");
            }
            components::server_sessions::ServerSessions::init(cx);
            components::updater::AppUpdater::init(cx);
            let manager = match PageManager::new(PAGE_ROUTES, "/", cx) {
                Ok(manager) => cx.new(|_| manager),
                Err(error) => {
                    eprintln!("Failed to initialize pages: {error}");
                    cx.quit();
                    return;
                }
            };
            let bounds = Bounds::centered(None, size(px(1100.), px(720.)), cx);
            let result = gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(720.), px(480.))),
                    titlebar: None,
                    window_decorations: Some(gpui::WindowDecorations::Client),
                    ..Default::default()
                },
                cx,
                move |window, cx| {
                    PageManager::attach(&manager, window, cx);
                    window.set_window_title("LocalCraft");
                    manager
                },
            );
            if let Err(error) = result {
                eprintln!("Failed to open LocalCraft: {error}");
                cx.quit();
                return;
            }
            cx.on_window_closed(|cx, _window_id| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            cx.activate(true);
        });
    let servers = std::mem::take(
        &mut *shutdown
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()),
    );
    runtime.block_on(async move {
        let mut stops = tokio::task::JoinSet::new();
        for server in servers {
            stops.spawn(async move { server.stop(std::time::Duration::from_secs(30)).await });
        }
        while let Some(result) = stops.join_next().await {
            if let Err(error) = result
                .map_err(anyhow::Error::from)
                .and_then(|result| result)
            {
                eprintln!("Could not stop server during shutdown: {error:#}");
            }
        }
    });
}
