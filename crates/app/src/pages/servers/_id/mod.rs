use gpui::{Context, IntoElement, Window, div, prelude::*, px};
use gpui_kit::base::Disableable;
use gpui_pages::{Page, PageAnimation, PageOptions, router};
use ui::{
    components::{Button, ButtonVariants},
    theme::ActiveUiTheme,
};

use crate::components::{
    server::console_view::ConsoleView, server_sessions::ServerSessions,
};

pub struct ServerConsolePage;

impl Page for ServerConsolePage {
    fn new(cx: &mut Context<Self>) -> Self {
        ServerSessions::init(cx);
        let sessions = cx.global::<ServerSessions>().0.clone();
        cx.observe(&sessions, |_, _, cx| cx.notify()).detach();
        Self
    }

    fn options() -> PageOptions {
        PageOptions::default().on_open(PageAnimation::Fade)
    }

    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.ui_palette();
        let screen = router(window, cx)
            .and_then(|manager| manager.read(cx).param("id").map(str::to_owned))
            .and_then(|id| {
                cx.try_global::<ServerSessions>()
                    .and_then(|sessions| sessions.0.read(cx).screen(&id))
            });

        let Some(screen) = screen else {
            return div().size_full().into_any_element();
        };

        let (console, command, running, busy) = {
            let state = screen.read(cx);
            (
                state.console.clone(),
                state.command.clone(),
                state.running,
                state.busy,
            )
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .min_h_0()
            .min_w_0()
            .gap_4()
            .child(ConsoleView::cached(console))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .pt_3()
                    .border_t_1()
                    .border_color(palette.border)
                    .child(div().flex_1().min_w_0().child(command))
                    .child(
                        Button::new("send-command")
                            .label("Send")
                            .primary()
                            .rounded(px(16.0))
                            .h(px(32.0))
                            .disabled(!running || busy)
                            .on_click(move |_, _, cx| {
                                screen.update(cx, |this, cx| this.send_command(cx));
                            }),
                    ),
            )
            .into_any_element()
    }
}
