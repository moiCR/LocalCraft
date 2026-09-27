use crate::workspace::Workspace;
use gpui::{
    App, Context, CursorStyle, IntoElement, MouseButton, ResizeEdge, Window, WindowControlArea,
    div, prelude::*, px, rgb, svg,
};
use gpui_router::RouterState;
use services::AppState;
use ui::theme::Palette;

pub fn render(workspace: &Workspace, window: &Window, cx: &Context<Workspace>) -> impl IntoElement {
    let palette = cx.global::<AppState>().theme_manager.palette();
    let maximized = window.is_maximized();
    div()
        .flex()
        .items_center()
        .h(px(40.))
        .flex_shrink_0()
        .bg(palette.sidebar)
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .px_3()
                .child(
                    navigation_control(
                        "toggle-sidebar",
                        if workspace.sidebar_motion.visible {
                            "icons/panel-right-close.svg"
                        } else {
                            "icons/panel-right-open.svg"
                        },
                        true,
                        palette,
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.sidebar_motion.toggle();
                        cx.notify();
                    })),
                )
                .child(
                    navigation_control(
                        "navigate-back",
                        "icons/chevron-left.svg",
                        workspace.navigation.can_go_back(),
                        palette,
                    )
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.navigation.go_back();
                        let path = this.navigation.current.path();
                        cx.update_global::<RouterState, _>(|router, _| {
                            router.with_path(path.into());
                        });
                        window.refresh();
                        cx.notify();
                    })),
                )
                .child(
                    navigation_control(
                        "navigate-forward",
                        "icons/chevron-right.svg",
                        workspace.navigation.can_go_forward(),
                        palette,
                    )
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.navigation.go_forward();
                        let path = this.navigation.current.path();
                        cx.update_global::<RouterState, _>(|router, _| {
                            router.with_path(path.into());
                        });
                        window.refresh();
                        cx.notify();
                    })),
                ),
        )
        .child({
            let drag_area = div()
                .id("window-drag-area")
                .window_control_area(WindowControlArea::Drag)
                .flex_1()
                .h_full()
                .flex()
                .items_center()
                .px_4()
                .text_xs()
                .text_color(palette.muted);
            drag_area
                .when(!cfg!(target_os = "windows"), |element| {
                    element.on_mouse_down(MouseButton::Left, |event, window, _| {
                        if event.click_count == 2 {
                            window.zoom_window();
                        } else {
                            window.start_window_move();
                        }
                    })
                })
                .on_mouse_down(MouseButton::Right, |event, window, _| {
                    window.show_window_menu(event.position);
                })
        })
        .child(control(
            "window-minimize",
            "icons/minimize.svg",
            palette,
            Some(WindowControlArea::Min),
            false,
            |window, _| window.minimize_window(),
        ))
        .child(control(
            "window-maximize",
            if maximized {
                "icons/restore.svg"
            } else {
                "icons/maximize.svg"
            },
            palette,
            Some(WindowControlArea::Max),
            false,
            |window, _| window.zoom_window(),
        ))
        .child(control(
            "window-close",
            "icons/close.svg",
            palette,
            None,
            true,
            |window, _| window.remove_window(),
        ))
}

fn navigation_control(
    id: &'static str,
    icon: &'static str,
    enabled: bool,
    palette: &Palette,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .size_7()
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .when(enabled, |button| {
            button
                .cursor_pointer()
                .hover(|style| style.bg(palette.surface))
        })
        .opacity(if enabled { 1. } else { 0.3 })
        .child(svg().path(icon).size_4().text_color(palette.muted))
}

fn control(
    id: &'static str,
    icon: &'static str,
    palette: &Palette,
    window_control: Option<WindowControlArea>,
    close: bool,
    action: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let hover = if close {
        rgb(0xc42b1c)
    } else {
        palette.surface
    };
    div()
        .id(id)
        .w(px(44.))
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .when_some(window_control, |element, area| {
            element.window_control_area(area)
        })
        .when(
            !cfg!(target_os = "windows") || window_control.is_none(),
            |element| element.on_click(move |_, window, cx| action(window, cx)),
        )
        .child(svg().path(icon).size_4().text_color(palette.text))
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
