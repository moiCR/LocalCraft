use gpui::AppContext;

use gpui::{Context, FocusHandle, IntoElement, Render, Window, div, prelude::*};
use gpui_router::{Route, Routes};
use services::AppState;

use crate::{
    modals::settings,
    views,
    widgets::{sidebar, titlebar},
};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    Instances,
    Runtimes,
    Playit,
}

impl Page {
    pub fn path(self) -> &'static str {
        match self {
            Self::Instances => "/instances",
            Self::Runtimes => "/runtimes",
            Self::Playit => "/playit",
        }
    }
}

pub struct Workspace {
    pub servers: gpui::Entity<views::servers::Servers>,
    pub runtimes: gpui::Entity<views::runtimes::Runtimes>,
    pub playit: gpui::Entity<views::playit::Playit>,
    pub navigation: crate::shell::Navigation,
    pub sidebar_motion: crate::shell::SidebarMotion,
    pub settings_open: bool,
    pub settings_focus: FocusHandle,
    pub settings_preferences:
        gpui::Entity<crate::modals::settings_preferences::SettingsPreferences>,
    sidebar_collapsed_setting: bool,
    previous_focus: Option<FocusHandle>,
}

impl Workspace {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let sidebar_collapsed_setting = cx.global::<AppState>().preferences.sidebar_collapsed;
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
            servers: cx.new(views::servers::Servers::new),
            runtimes: cx.new(views::runtimes::Runtimes::new),
            playit: cx.new(views::playit::Playit::new),
            navigation: crate::shell::Navigation::default(),
            sidebar_motion: crate::shell::SidebarMotion::with_visibility(
                !sidebar_collapsed_setting,
            ),
            settings_open: false,
            settings_focus: cx.focus_handle(),
            settings_preferences: cx
                .new(crate::modals::settings_preferences::SettingsPreferences::new),
            sidebar_collapsed_setting,
            previous_focus: None,
        }
    }

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

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.global::<AppState>().theme_manager.palette();
        let route_path = cx
            .global::<gpui_router::RouterState>()
            .location
            .pathname
            .clone();
        let route_animation = match route_path.as_ref() {
            "/runtimes" => "runtimes",
            "/playit" => "playit",
            _ => "instances",
        };
        let servers = self.servers.clone();
        let runtimes = self.runtimes.clone();
        let playit = self.playit.clone();
        let routes = Routes::new()
            .basename("/")
            .child(Route::new().index().element({
                let servers = servers.clone();
                move |_, _| views::instances::render(&servers)
            }))
            .child(Route::new().path("instances").element({
                let servers = servers.clone();
                move |_, _| views::instances::render(&servers)
            }))
            .child(
                Route::new()
                    .path("runtimes")
                    .element(move |_, _| runtimes.clone()),
            )
            .child(
                Route::new()
                    .path("playit")
                    .element(move |_, _| playit.clone()),
            )
            .child(Route::new().path("{*not_found}").element({
                let servers = servers.clone();
                move |_, _| views::instances::render(&servers)
            }));
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
                    .child(sidebar::panel(self, cx))
                    .child(views::animate_page(
                        div().size_full().child(routes),
                        route_animation,
                    ))
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
