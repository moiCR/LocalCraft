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
}

impl Page {
    pub fn path(self) -> &'static str {
        match self {
            Self::Instances => "/instances",
            Self::Runtimes => "/runtimes",
        }
    }
}

pub struct Workspace {
    pub servers: gpui::Entity<views::servers::Servers>,
    pub runtimes: gpui::Entity<views::runtimes::Runtimes>,
    pub navigation: crate::shell::Navigation,
    pub sidebar_motion: crate::shell::SidebarMotion,
    pub settings_open: bool,
    pub settings_focus: FocusHandle,
    previous_focus: Option<FocusHandle>,
}

impl Workspace {
    pub fn new(cx: &mut Context<Self>) -> Self {
        cx.observe_global::<AppState>(|_, cx| cx.notify()).detach();
        Self {
            servers: cx.new(views::servers::Servers::new),
            runtimes: cx.new(views::runtimes::Runtimes::new),
            navigation: crate::shell::Navigation::default(),
            sidebar_motion: crate::shell::SidebarMotion::default(),
            settings_open: false,
            settings_focus: cx.focus_handle(),
            previous_focus: None,
        }
    }

    pub fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.previous_focus = window.focused(cx);
        self.settings_open = true;
        window.focus(&self.settings_focus);
        cx.notify();
    }

    pub fn close_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.settings_open = false;
        if let Some(focus) = self.previous_focus.take() {
            window.focus(&focus);
        } else {
            window.blur();
        }
        cx.notify();
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.global::<AppState>().theme_manager.palette();
        let route_animation = if cx
            .global::<gpui_router::RouterState>()
            .location
            .pathname
            .as_ref()
            == "/runtimes"
        {
            "runtimes"
        } else {
            "instances"
        };
        let servers = self.servers.clone();
        let runtimes = self.runtimes.clone();
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
