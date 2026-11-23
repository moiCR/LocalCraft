use gpui::{
    AnimationExt, AnyView, Context, FocusHandle, IntoElement, SpringAnimation, SpringConfig,
    Window, div, prelude::*, px,
};
use gpui_pages::PageLayout;
use services::AppState;
use ui::theme::ActiveUiTheme;

use crate::{
    layout::{
        NavigationStyle, SidebarState,
        sidebar::{Dock, Sidebar},
        titlebar::{Titlebar, resize_handles},
        toggle_sidebar,
    },
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
        let collapsed = cx.global::<AppState>().preferences.sidebar_collapsed;
        if cx.try_global::<SidebarState>().is_none() {
            cx.set_global(SidebarState { collapsed });
        }
        Self { child }
    }

    fn set_child(&mut self, child: AnyView) {
        self.child = child;
    }

    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.ui_palette();
        let dock = NavigationStyle::from_key(&cx.global::<AppState>().preferences.navigation_style)
            == NavigationStyle::Dock;
        let collapsed = cx
            .try_global::<SidebarState>()
            .map(|state| state.collapsed)
            .unwrap_or(false);

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

        let spring_config = SpringConfig::new(200.0, 24.0, 1.0);
        let collapse_spring = SpringAnimation::new(spring_config).to(!collapsed);

        div()
            .relative()
            .track_focus(&focus_handle)
            .key_context("AppLayout")
            .on_key_down(move |event, window, cx| {
                if !dock
                    && event.keystroke.modifiers.control
                    && event.keystroke.key.eq_ignore_ascii_case("b")
                {
                    toggle_sidebar(window, cx);
                    cx.stop_propagation();
                } else if event.keystroke.modifiers.control
                    && event.keystroke.key.eq_ignore_ascii_case("n")
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
            .child(Titlebar::new().sidebar_toggle(!dock))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .children((!dock).then(|| {
                        div()
                            .id("sidebar-container")
                            .overflow_hidden()
                            .h_full()
                            .child(Sidebar)
                            .with_spring(
                                "sidebar-collapse-animation",
                                collapse_spring,
                                move |this, phase| {
                                    let width = phase.interpolate_clamped(px(0.0), px(200.0));
                                    this.w(width)
                                },
                            )
                    }))
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .size_full()
                            .overflow_hidden()
                            .child(self.child.clone()),
                    ),
            )
            .children(dock.then_some(Dock))
            .child(SettingsModal::new())
            .child(CreateServerModal::new())
            .child(DeleteInstanceModal::new())
            .when(
                !window.is_maximized() && !window.is_fullscreen(),
                |element| element.child(resize_handles()),
            )
    }
}
