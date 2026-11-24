use gpui::{Context, IntoElement, Window, div, prelude::*};
use gpui_pages::{Page, PageAnimation, PageOptions, router};

use crate::components::server_sessions::ServerSessions;

pub struct ServerLogsPage {
    refreshed_id: Option<String>,
}

impl Page for ServerLogsPage {
    fn new(cx: &mut Context<Self>) -> Self {
        ServerSessions::init(cx);
        Self { refreshed_id: None }
    }

    fn options() -> PageOptions {
        PageOptions::default().on_open(PageAnimation::Fade)
    }

    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let server_id =
            router(window, cx).and_then(|manager| manager.read(cx).param("id").map(str::to_owned));
        let screen = server_id.as_deref().and_then(|id| {
            cx.try_global::<ServerSessions>()
                .and_then(|sessions| sessions.0.read(cx).screen(id))
        });

        let Some(screen) = screen else {
            return div().size_full().into_any_element();
        };

        if let Some(id) = server_id
            && self.refreshed_id.as_deref() != Some(id.as_str())
        {
            self.refreshed_id = Some(id);
            let refresh_screen = screen.clone();
            window.defer(cx, move |_, cx| {
                refresh_screen.update(cx, |this, cx| this.refresh_logs(cx));
            });
        }

        let logs = screen.read(cx).logs.clone();
        div()
            .size_full()
            .flex()
            .flex_col()
            .min_h_0()
            .min_w_0()
            .child(logs)
            .into_any_element()
    }
}
