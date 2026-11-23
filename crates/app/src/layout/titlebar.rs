use gpui::prelude::FluentBuilder;
use gpui::*;
use ui::{components::IconButton, theme::ActiveUiTheme};

use crate::{
    layout::toggle_sidebar,
    modals::{CreateServerModal, SettingsModalState, SettingsTab},
    widgets::updater::{AppUpdater, UpdateStatus},
};

#[derive(IntoElement)]
pub struct Titlebar {
    show_sidebar_toggle: bool,
}

impl Titlebar {
    pub fn new() -> Self {
        Self {
            show_sidebar_toggle: true,
        }
    }

    pub(crate) fn sidebar_toggle(mut self, show: bool) -> Self {
        self.show_sidebar_toggle = show;
        self
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
        let background = palette.background;
        let surface_hover = palette.surface_hover;
        let text_color = palette.text;
        let text_muted = palette.muted;
        let error = palette.error;
        let accent = palette.accent;

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

        div()
            .flex()
            .items_center()
            .justify_between()
            .h_12()
            .w_full()
            .bg(background)
            .text_xs()
            .text_color(text_muted)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .px_4()
                    .h_full()
                    .flex_shrink_0()
                    .children(self.show_sidebar_toggle.then(|| {
                        IconButton::new("btn-collapse-sidebar", "icons/panel-left.svg")
                            .tooltip("Toggle sidebar")
                            .shortcut("Ctrl+B")
                            .on_click(|_, window, cx| toggle_sidebar(window, cx))
                    }))
                    .child(
                        IconButton::new("btn-new-server", "icons/plus.svg")
                            .tooltip("Create server")
                            .shortcut("Ctrl+N")
                            .on_click(|_, window, cx| CreateServerModal::open(window, cx)),
                    ),
            )
            .child(
                div()
                    .id("titlebar-drag-left")
                    .window_control_area(WindowControlArea::Drag)
                    .when(cfg!(target_os = "linux"), |element| {
                        let should_move = window.use_state(cx, |_, _| false);
                        element
                            .on_mouse_down_out(
                                window.listener_for(&should_move, |should_move, _, _, _| {
                                    *should_move = false
                                }),
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
                                window.listener_for(&should_move, |should_move, _, _, _| {
                                    *should_move = false;
                                }),
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
                    .h_full(),
            )
            .child(div().w(px(280.0)).h_full().flex_shrink_0())
            .child(
                div()
                    .id("titlebar-drag-right")
                    .window_control_area(WindowControlArea::Drag)
                    .flex()
                    .items_center()
                    .flex_1()
                    .h_full(),
            )
            .children(update_badge.map(|(label, is_downloading)| {
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
            .child(
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
                                    .group_hover("window-min", move |style| {
                                        style.text_color(text_color)
                                    }),
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
                                    .group_hover("window-max", move |style| {
                                        style.text_color(text_color)
                                    }),
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
                                    .group_hover("window-close", |style| {
                                        style.text_color(rgb(0xffffff))
                                    }),
                            ),
                    ),
            )
    }
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
