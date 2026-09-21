use gpui::{Context, FocusHandle, IntoElement, Render, Window, div, prelude::*};
use services::AppState;

use crate::{modals::settings, views, widgets::sidebar};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    Instances,
    Runtimes,
}

pub struct Workspace {
    pub active_page: Page,
    pub settings_open: bool,
    pub settings_focus: FocusHandle,
    previous_focus: Option<FocusHandle>,
}

impl Workspace {
    pub fn new(cx: &mut Context<Self>) -> Self {
        cx.observe_global::<AppState>(|_, cx| cx.notify()).detach();
        Self {
            active_page: Page::default(),
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.global::<AppState>().theme_manager.palette();
        div()
            .relative()
            .flex()
            .size_full()
            .bg(palette.background)
            .text_color(palette.text)
            .font_family("Geist")
            .text_sm()
            .child(sidebar::render(self.active_page, cx))
            .child(match self.active_page {
                Page::Instances => views::instances::render(palette),
                Page::Runtimes => views::runtimes::render(palette),
            })
            .when(self.settings_open, |element| {
                element.child(settings::render(self, cx))
            })
    }
}
