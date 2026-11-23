use gpui::{AnyView, Context, FocusHandle, IntoElement, Window, div, prelude::*};
use gpui_pages::PageLayout;
use ui::theme::ActiveUiTheme;

use crate::{
    layout::titlebar::{Titlebar, resize_handles},
    modals::{CreateServerModal, DeleteInstanceModal, SettingsModal},
    views::AppViews,
    widgets::updater::AppUpdater,
};

pub struct RootLayout {
    child: AnyView,
}

impl PageLayout for RootLayout {
    fn new(child: AnyView, cx: &mut Context<Self>) -> Self {
        AppViews::init(cx);
        AppUpdater::init(cx);
        let servers = cx.global::<AppViews>().servers.clone();
        cx.observe(&servers, |_, _, cx| cx.notify()).detach();
        Self { child }
    }

    fn set_child(&mut self, child: AnyView) {
        self.child = child;
    }

    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.ui_palette();

        let focus_handle: FocusHandle = window
            .use_keyed_state("app-layout-focus", cx, |window, cx| {
                let handle = cx.focus_handle();
                let initial_focus = handle.clone();
                window.defer(cx, move |window, cx| {
                    if window.focused(cx).is_none() {
                        window.focus(&initial_focus, cx);
                    }
                });
                handle
            })
            .read(cx)
            .clone();

        div()
            .relative()
            .track_focus(&focus_handle)
            .key_context("AppLayout")
            .on_key_down(move |event, window, cx| {
                if event.keystroke.modifiers.control && event.keystroke.key.eq_ignore_ascii_case("n")
                {
                    CreateServerModal::open(window, cx);
                    cx.stop_propagation();
                }
            })
            .flex()
            .flex_col()
            .size_full()
            .font_family("Geist")
            .text_sm()
            .bg(palette.background)
            .text_color(palette.text)
            .child(Titlebar::new())
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .size_full()
                    .overflow_hidden()
                    .child(self.child.clone()),
            )
            .child(SettingsModal::new())
            .child(CreateServerModal::new())
            .child(DeleteInstanceModal::new())
            .when(
                !window.is_maximized() && !window.is_fullscreen(),
                |element| element.child(resize_handles()),
            )
    }
}
