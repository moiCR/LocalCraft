use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_pages::router;
use ui::{
    components::IconButton,
    theme::{ActiveUiTheme, Palette},
};

use crate::{
    modals::{CreateServerModal, SettingsModalState, SettingsTab},
    views::AppViews,
    widgets::{
        server::screen::ServerSection,
        updater::{AppUpdater, UpdateStatus},
    },
};

#[derive(IntoElement)]
pub struct Titlebar;

impl Titlebar {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Titlebar {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for Titlebar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let palette = cx.ui_palette();
        let current_path = router(window, cx)
            .map(|manager| manager.read(cx).current_path().to_owned())
            .unwrap_or_else(|| "/".to_owned());

        let servers_entity = cx
            .try_global::<AppViews>()
            .map(|views| views.servers.clone());
        let active_screen = if current_path == "/" {
            servers_entity
                .as_ref()
                .and_then(|entity| entity.read(cx).active_screen())
        } else {
            None
        };

        let update_badge = cx.try_global::<AppUpdater>().and_then(|app_updater| {
            let updater = app_updater.0.read(cx);
            match updater.status() {
                UpdateStatus::Available => Some((
                    updater
                        .version()
                        .map(|v| format!("v{v} available"))
                        .unwrap_or_else(|| "Update available".to_owned()),
                    false,
                )),
                UpdateStatus::Downloading => Some(("Updating…".to_owned(), true)),
                _ => None,
            }
        });

        let (left_controls, center_switcher, right_action) = if let Some(screen) =
            active_screen.clone()
        {
            let server = screen.read(cx);
            let server_name = server.server.name.clone();
            let active_section = server.section;
            let busy = server.busy;
            let running = server.running;
            let status_color = if busy {
                palette.warning
            } else if running {
                palette.running
            } else {
                palette.muted
            };

            let back_entity = servers_entity.clone();
            let left = div()
                .flex()
                .items_center()
                .gap_2()
                .pl_3()
                .pr_2()
                .min_w_0()
                .child(
                    IconButton::new("back-to-servers", "icons/chevron-left.svg")
                        .tooltip("Back to servers")
                        .on_click(move |_, _, cx| {
                            if let Some(entity) = back_entity.as_ref() {
                                entity.update(cx, |this, cx| this.close_active_server(cx));
                            }
                        }),
                )
                .child(
                    div()
                        .size(px(8.0))
                        .rounded_full()
                        .flex_shrink_0()
                        .bg(status_color),
                )
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(palette.text)
                        .truncate()
                        .min_w_0()
                        .max_w(px(220.0))
                        .child(server_name),
                )
                .into_any_element();

            let tabs = [
                (
                    ServerSection::Console,
                    "server-tab-console",
                    "Console",
                    "icons/square-terminal.svg",
                ),
                (
                    ServerSection::Logs,
                    "server-tab-logs",
                    "Logs",
                    "icons/logs.svg",
                ),
                (
                    ServerSection::Files,
                    "server-tab-files",
                    "Files",
                    "icons/files.svg",
                ),
            ];

            let center = switcher_track(&palette)
                .children(tabs.into_iter().map(|(section, id, label, icon)| {
                    let screen_target = screen.clone();
                    switcher_tab(id, label, icon, active_section == section, &palette).on_click(
                        move |_, _, cx| {
                            screen_target.update(cx, |this, cx| this.change_section(section, cx));
                        },
                    )
                }))
                .into_any_element();

            let settings_screen = screen;
            let right = IconButton::new("server-settings", "icons/settings.svg")
                .tooltip("Server settings")
                .disabled(busy)
                .on_click(move |_, window, cx| {
                    settings_screen.update(cx, |this, cx| this.open_settings(window, cx));
                })
                .into_any_element();

            (left, center, right)
        } else {
            let left = div()
                .flex()
                .items_center()
                .gap_1p5()
                .pl_3()
                .pr_2()
                .flex_shrink_0()
                .child(
                    IconButton::new("btn-new-server", "icons/plus.svg")
                        .tooltip("Create server")
                        .shortcut("Ctrl+N")
                        .color(palette.accent)
                        .on_click(|_, window, cx| CreateServerModal::open(window, cx)),
                )
                .into_any_element();

            let routes = [
                ("nav-servers", "Servers", "icons/server.svg", "/"),
                ("nav-runtimes", "Runtimes", "icons/coffee.svg", "/runtimes"),
                ("nav-playit", "Playit", "icons/computer.svg", "/playit"),
            ];

            let center = switcher_track(&palette)
                .children(routes.into_iter().map(|(id, label, icon, path)| {
                    switcher_tab(id, label, icon, current_path == path, &palette).on_click(
                        move |_, window, cx| {
                            if let Some(manager) = router(window, cx) {
                                manager.update(cx, |manager, cx| {
                                    if manager.current_path() != path
                                        && let Err(error) = manager.navigate(path, cx)
                                    {
                                        eprintln!("{error}");
                                    }
                                });
                            }
                        },
                    )
                }))
                .into_any_element();

            let settings_open = cx
                .try_global::<SettingsModalState>()
                .is_some_and(|state| state.open);
            let right = IconButton::new("btn-open-settings", "icons/settings.svg")
                .tooltip("Settings")
                .selected(settings_open)
                .on_click(|_, window, cx| {
                    cx.set_global(SettingsModalState {
                        open: true,
                        tab: SettingsTab::General,
                    });
                    window.refresh();
                })
                .into_any_element();

            (left, center, right)
        };

        div()
            .flex()
            .items_center()
            .h(px(48.0))
            .w_full()
            .flex_shrink_0()
            .bg(palette.surface)
            .border_b_1()
            .border_color(palette.border)
            .text_xs()
            .text_color(palette.muted)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .items_center()
                    .child(left_controls)
                    .child(drag_area("titlebar-drag-left", window, cx)),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .px_2()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(center_switcher),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .items_center()
                    .justify_end()
                    .child(drag_area("titlebar-drag-right", window, cx))
                    .children(update_badge.map(|(label, is_downloading)| {
                        let accent = palette.accent;
                        div()
                            .id("btn-titlebar-update-badge")
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .px_2p5()
                            .py_1()
                            .mr_2()
                            .rounded_full()
                            .bg(accent.opacity(0.15))
                            .border_1()
                            .border_color(accent.opacity(0.35))
                            .text_color(accent)
                            .text_xs()
                            .cursor_pointer()
                            .flex_shrink_0()
                            .hover(move |s| s.bg(accent.opacity(0.25)))
                            .on_click(move |_event, window, cx| {
                                cx.set_global(SettingsModalState {
                                    open: true,
                                    tab: SettingsTab::Updates,
                                });
                                window.refresh();
                            })
                            .child(
                                svg()
                                    .path(if is_downloading {
                                        "icons/loader.svg"
                                    } else {
                                        "icons/download.svg"
                                    })
                                    .size_3()
                                    .text_color(accent),
                            )
                            .child(label)
                    }))
                    .child(div().pr_2().flex_shrink_0().child(right_action))
                    .child(window_controls(&palette)),
            )
    }
}

fn switcher_track(palette: &Palette) -> Div {
    div()
        .flex()
        .items_center()
        .gap_1()
        .p_1()
        .rounded(px(16.0))
        .bg(palette.card_surface())
        .border_1()
        .border_color(palette.border)
}

fn switcher_tab(
    id: &'static str,
    label: &'static str,
    icon: &'static str,
    active: bool,
    palette: &Palette,
) -> Stateful<Div> {
    let text_color = palette.text;
    let surface_hover = palette.surface_hover;
    let icon_color = if active {
        palette.accent
    } else {
        palette.muted
    };
    div()
        .id(id)
        .flex()
        .items_center()
        .gap_1p5()
        .h(px(28.0))
        .px_3()
        .rounded(px(12.0))
        .text_xs()
        .font_weight(if active {
            FontWeight::SEMIBOLD
        } else {
            FontWeight::MEDIUM
        })
        .text_color(if active { text_color } else { palette.muted })
        .when(active, |tab| tab.bg(surface_hover))
        .when(!active, |tab| {
            tab.cursor_pointer()
                .hover(move |style| style.text_color(text_color))
        })
        .when(active, |tab| tab.cursor_pointer())
        .child(svg().path(icon).size(px(14.0)).text_color(icon_color))
        .child(label)
}

fn drag_area(id: &'static str, window: &mut Window, cx: &mut App) -> Stateful<Div> {
    div()
        .id(id)
        .window_control_area(WindowControlArea::Drag)
        .when(cfg!(target_os = "linux"), |element| {
            let should_move = window.use_state(cx, |_, _| false);
            element
                .on_mouse_down_out(
                    window.listener_for(&should_move, |should_move, _, _, _| *should_move = false),
                )
                .on_mouse_down(
                    MouseButton::Left,
                    window.listener_for(
                        &should_move,
                        |should_move, event: &MouseDownEvent, window, _| {
                            *should_move = event.click_count == 1;
                            if event.click_count == 2 {
                                window.zoom_window();
                            }
                        },
                    ),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    window.listener_for(&should_move, |should_move, _, _, _| *should_move = false),
                )
                .on_mouse_move(window.listener_for(
                    &should_move,
                    |should_move, event: &MouseMoveEvent, window, _| {
                        if *should_move {
                            *should_move = false;
                            if event.pressed_button == Some(MouseButton::Left) {
                                window.start_window_move();
                            }
                        }
                    },
                ))
        })
        .flex()
        .items_center()
        .flex_1()
        .min_w(px(24.0))
        .h_full()
}

fn window_controls(palette: &Palette) -> Div {
    let surface_hover = palette.surface_hover;
    let text_color = palette.text;
    let text_muted = palette.muted;
    let error = palette.error;
    div()
        .flex()
        .items_center()
        .h_full()
        .flex_shrink_0()
        .child(
            div()
                .id("window-min")
                .group("window-min")
                .window_control_area(WindowControlArea::Min)
                .when(cfg!(target_os = "linux"), |element| {
                    element.on_click(|_, window, _| window.minimize_window())
                })
                .flex()
                .items_center()
                .justify_center()
                .w(px(46.0))
                .h_full()
                .hover(move |style| style.bg(surface_hover))
                .child(
                    svg()
                        .path("icons/minus.svg")
                        .size_3p5()
                        .text_color(text_muted)
                        .group_hover("window-min", move |style| style.text_color(text_color)),
                ),
        )
        .child(
            div()
                .id("window-max")
                .group("window-max")
                .window_control_area(WindowControlArea::Max)
                .when(cfg!(target_os = "linux"), |element| {
                    element.on_click(|_, window, _| window.zoom_window())
                })
                .flex()
                .items_center()
                .justify_center()
                .w(px(46.0))
                .h_full()
                .hover(move |style| style.bg(surface_hover))
                .child(
                    svg()
                        .path("icons/square.svg")
                        .size_3()
                        .text_color(text_muted)
                        .group_hover("window-max", move |style| style.text_color(text_color)),
                ),
        )
        .child(
            div()
                .id("window-close")
                .group("window-close")
                .window_control_area(WindowControlArea::Close)
                .when(cfg!(target_os = "linux"), |element| {
                    element.on_click(|_, window, _| window.remove_window())
                })
                .flex()
                .items_center()
                .justify_center()
                .w(px(46.0))
                .h_full()
                .hover(move |style| style.bg(error))
                .child(
                    svg()
                        .path("icons/x.svg")
                        .size_3p5()
                        .text_color(text_muted)
                        .group_hover("window-close", |style| style.text_color(rgb(0xffffff))),
                ),
        )
}

pub fn resize_handles() -> impl IntoElement {
    div().absolute().inset_0().children(
        [
            ("resize-top", ResizeEdge::Top, CursorStyle::ResizeUpDown),
            (
                "resize-bottom",
                ResizeEdge::Bottom,
                CursorStyle::ResizeUpDown,
            ),
            (
                "resize-left",
                ResizeEdge::Left,
                CursorStyle::ResizeLeftRight,
            ),
            (
                "resize-right",
                ResizeEdge::Right,
                CursorStyle::ResizeLeftRight,
            ),
            (
                "resize-top-left",
                ResizeEdge::TopLeft,
                CursorStyle::ResizeUpLeftDownRight,
            ),
            (
                "resize-top-right",
                ResizeEdge::TopRight,
                CursorStyle::ResizeUpRightDownLeft,
            ),
            (
                "resize-bottom-left",
                ResizeEdge::BottomLeft,
                CursorStyle::ResizeUpRightDownLeft,
            ),
            (
                "resize-bottom-right",
                ResizeEdge::BottomRight,
                CursorStyle::ResizeUpLeftDownRight,
            ),
        ]
        .into_iter()
        .map(|(id, edge, cursor)| {
            div()
                .id(id)
                .absolute()
                .cursor(cursor)
                .map(|handle| match edge {
                    ResizeEdge::Top => handle.top_0().left(px(8.)).right(px(8.)).h(px(4.)),
                    ResizeEdge::Bottom => handle.bottom_0().left(px(8.)).right(px(8.)).h(px(4.)),
                    ResizeEdge::Left => handle.left_0().top(px(8.)).bottom(px(8.)).w(px(4.)),
                    ResizeEdge::Right => handle.right_0().top(px(8.)).bottom(px(8.)).w(px(4.)),
                    ResizeEdge::TopLeft => handle.top_0().left_0().size(px(8.)),
                    ResizeEdge::TopRight => handle.top_0().right_0().size(px(8.)),
                    ResizeEdge::BottomLeft => handle.bottom_0().left_0().size(px(8.)),
                    ResizeEdge::BottomRight => handle.bottom_0().right_0().size(px(8.)),
                })
                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    window.start_window_resize(edge);
                    cx.stop_propagation();
                })
        }),
    )
}
