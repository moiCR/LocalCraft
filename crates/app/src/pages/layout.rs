use gpui::{
    AnyView, AppContext, Context, FocusHandle, IntoElement, Subscription, Window, div, prelude::*,
};
use gpui_pages::PageLayout;
use services::AppState;

use crate::{
    modals::settings,
    views::AppViews,
    widgets::{sidebar, titlebar},
};

pub struct RootLayout {
    child: AnyView,
    pub sidebar_motion: crate::shell::SidebarMotion,
    pub settings_open: bool,
    pub settings_focus: FocusHandle,
    pub settings_preferences: gpui::Entity<crate::modals::settings_preferences::SettingsPreferences>,
    pub updater: gpui::Entity<crate::widgets::updater::Updater>,
    sidebar_collapsed_setting: bool,
    previous_focus: Option<FocusHandle>,
    _updater_subscription: Subscription,
}

impl RootLayout {
    pub fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.previous_focus = window.focused(cx);
        self.settings_open = true;
        window.focus(&self.settings_focus, cx);
        cx.notify();
    }

    pub fn close_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.settings_open = false;
        if let Some(focus) = self.previous_focus.take() {
            window.focus(&focus, cx);
        } else {
            window.blur(cx);
        }
        cx.notify();
    }
}

impl PageLayout for RootLayout {
    fn new(child: AnyView, cx: &mut Context<Self>) -> Self {
        AppViews::init(cx);
        let sidebar_collapsed_setting = cx.global::<AppState>().preferences.sidebar_collapsed;
        let updater = cx.new(crate::widgets::updater::Updater::new);
        let updater_subscription = cx.observe(&updater, |_, _, cx| cx.notify());
        cx.observe_global::<AppState>(|this, cx| {
            let collapsed = cx.global::<AppState>().preferences.sidebar_collapsed;
            if this.sidebar_collapsed_setting != collapsed {
                this.sidebar_collapsed_setting = collapsed;
                this.sidebar_motion.set_visible(!collapsed);
            }
            cx.notify();
        })
        .detach();
        Self {
            child,
            sidebar_motion: crate::shell::SidebarMotion::with_visibility(
                !sidebar_collapsed_setting,
            ),
            settings_open: false,
            settings_focus: cx.focus_handle(),
            settings_preferences: cx
                .new(crate::modals::settings_preferences::SettingsPreferences::new),
            updater,
            sidebar_collapsed_setting,
            previous_focus: None,
            _updater_subscription: updater_subscription,
        }
    }

    fn set_child(&mut self, child: AnyView) {
        self.child = child;
    }

    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.global::<AppState>().theme_manager.palette();
        div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .bg(palette.background)
            .text_color(palette.text)
            .font_family("Geist")
            .text_sm()
            .child(titlebar::render(self, window, cx))
            .child(
                div()
                    .relative()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .child(sidebar::panel(self, window, cx))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .size_full()
                            .child(self.child.clone()),
                    )
                    .when(self.settings_open, |element| {
                        element.child(settings::render(self, cx))
                    }),
            )
            .when(
                !window.is_maximized() && !window.is_fullscreen(),
                |element| element.child(titlebar::resize_handles()),
            )
    }
}
