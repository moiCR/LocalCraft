use crate::shell::SIDEBAR_DURATION;
use crate::workspace::{Page, Workspace};
use std::time::Duration;

use gpui::{
    Animation, AnimationExt, Context, IntoElement, div, ease_out_quint, prelude::*, px, svg,
};
use gpui_router::RouterState;
use services::AppState;

pub fn panel(workspace: &Workspace, cx: &Context<Workspace>) -> impl IntoElement {
    let motion = &workspace.sidebar_motion;
    let revision = motion.revision;
    let (snapshot, target) = motion.endpoints();
    div()
        .h_full()
        .flex_shrink_0()
        .overflow_hidden()
        .child(render(cx).with_animation(
            ("sidebar-slide", revision),
            Animation::new(SIDEBAR_DURATION),
            move |sidebar, progress| {
                let eased = 1. - (1. - progress).powi(5);
                let visible = snapshot + (target - snapshot) * eased;
                sidebar
                    .relative()
                    .left(px(-224. * (1. - visible)))
                    .opacity(visible)
            },
        ))
        .with_animation(
            ("sidebar-space", revision),
            Animation::new(SIDEBAR_DURATION),
            move |container, progress| {
                let eased = 1. - (1. - progress).powi(5);
                container.w(px(224. * (snapshot + (target - snapshot) * eased)))
            },
        )
}

fn render(cx: &Context<Workspace>) -> gpui::Div {
    let palette = cx.global::<AppState>().theme_manager.palette();
    let active_path = cx.global::<RouterState>().location.pathname.clone();
    div()
        .flex()
        .flex_col()
        .w(px(224.))
        .h_full()
        .flex_shrink_0()
        .bg(palette.sidebar)
        .p_4()
        .gap_8()
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .w_full()
                .px_2()
                .pt_8()
                .pb_4()
                .child(
                    svg()
                        .path("icons/logo.svg")
                        .size_10()
                        .flex_shrink_0()
                        .text_color(palette.text),
                ),
        )
        .child(
            div().flex().flex_col().gap_1().children(
                [
                    (
                        Page::Instances,
                        "Instances",
                        "instances",
                        "icons/server.svg",
                    ),
                    (Page::Runtimes, "Runtimes", "runtimes", "icons/coffee.svg"),
                ]
                .into_iter()
                .map(|(page, label, id, icon)| {
                    let selected = active_path.as_ref() == page.path()
                        || (page == Page::Instances && active_path.as_ref() == "/");
                    let surface = gpui::Hsla::from(palette.surface);
                    div()
                        .id(id)
                        .flex()
                        .items_center()
                        .gap_3()
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .cursor_pointer()
                        .bg(if selected {
                            palette.surface
                        } else {
                            palette.sidebar
                        })
                        .text_color(if selected {
                            palette.text
                        } else {
                            palette.muted
                        })
                        .hover(|style| style.bg(palette.surface).text_color(palette.text))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.navigation.visit(page);
                            cx.update_global::<RouterState, _>(|router, _| {
                                router.with_path(page.path().into());
                            });
                            window.refresh();
                            cx.notify();
                        }))
                        .child(
                            svg()
                                .path(icon)
                                .size_4()
                                .flex_shrink_0()
                                .text_color(if selected {
                                    palette.text
                                } else {
                                    palette.muted
                                }),
                        )
                        .child(label)
                        .with_animation(
                            (id, usize::from(selected)),
                            Animation::new(Duration::from_millis(300))
                                .with_easing(ease_out_quint()),
                            move |item, progress| {
                                item.when(selected, |item| item.bg(surface.opacity(progress)))
                            },
                        )
                }),
            ),
        )
        .child(div().flex_1())
        .child(
            div()
                .border_t_1()
                .border_color(palette.border)
                .pt_4()
                .px_2()
                .child(
                    div()
                        .id("open-settings")
                        .flex()
                        .items_center()
                        .justify_between()
                        .w_full()
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .cursor_pointer()
                        .text_color(palette.muted)
                        .hover(|style| style.bg(palette.surface).text_color(palette.text))
                        .on_click(cx.listener(|this, _, window, cx| this.open_settings(window, cx)))
                        .child("Settings")
                        .child(div().text_xs().child("Preferences")),
                ),
        )
}
