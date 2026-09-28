use std::time::Duration;

use gpui::{
    Animation, AnimationExt, Context, IntoElement, MouseButton, div, ease_out_quint, prelude::*,
    px, rgba, svg,
};
use services::AppState;

use super::appearance;
use crate::workspace::Workspace;

pub fn render(workspace: &Workspace, cx: &Context<Workspace>) -> impl IntoElement {
    let palette = cx.global::<AppState>().theme_manager.palette();
    div()
        .id("settings-overlay")
        .absolute()
        .inset_0()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .p_6()
        .bg(rgba(0x00000099))
        .occlude()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _, window, cx| {
                this.close_settings(window, cx);
                cx.stop_propagation();
            }),
        )
        .child(
            div()
                .id("settings-dialog")
                .track_focus(&workspace.settings_focus)
                .w(px(480.))
                .max_w_full()
                .max_h_full()
                .min_h_0()
                .flex()
                .flex_col()
                .bg(palette.background)
                .border_1()
                .border_color(palette.border)
                .rounded_lg()
                .shadow_lg()
                .occlude()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                    match event.keystroke.key.as_str() {
                        "escape" => this.close_settings(window, cx),
                        "left" | "right" => {
                            cx.update_global::<AppState, _>(|state, cx| {
                                use ui::theme::Appearance;
                                let appearance = match state.theme_manager.appearance() {
                                    Appearance::Dark => Appearance::Light,
                                    Appearance::Light => Appearance::Dark,
                                };
                                appearance::set_app_appearance(state, cx, appearance);
                            });
                        }
                        _ => {}
                    }
                    cx.stop_propagation();
                }))
                .child(
                    div()
                        .flex()
                        .items_start()
                        .justify_between()
                        .p_6()
                        .gap_4()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(div().text_lg().child("Settings"))
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(palette.muted)
                                        .child("Make LocalCraft feel like home."),
                                ),
                        )
                        .child(
                            div()
                                .id("close-settings")
                                .w(px(36.))
                                .h(px(36.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .rounded_md()
                                .border_1()
                                .border_color(palette.border)
                                .text_color(palette.text)
                                .hover(|style| style.bg(palette.surface).text_color(palette.text))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.close_settings(window, cx)
                                }))
                                .child(
                                    svg()
                                        .path("icons/close.svg")
                                        .size_4()
                                        .text_color(palette.text),
                                ),
                        ),
                )
                .child(
                    div()
                        .id("settings-content")
                        .flex()
                        .flex_col()
                        .min_h_0()
                        .max_h(px(500.))
                        .overflow_y_scroll()
                        .child(
                            div()
                                .border_t_1()
                                .border_color(palette.border)
                                .p_6()
                                .child(appearance::render(cx)),
                        )
                        .child(
                            div()
                                .border_t_1()
                                .border_color(palette.border)
                                .p_6()
                                .child(workspace.settings_preferences.clone()),
                        )
                        .child(
                            div()
                                .border_t_1()
                                .border_color(palette.border)
                                .p_6()
                                .child(workspace.updater.clone()),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_4()
                        .border_t_1()
                        .border_color(palette.border)
                        .px_6()
                        .py_4()
                        .child(
                            div()
                                .text_xs()
                                .text_color(palette.muted)
                                .child("Changes are saved automatically."),
                        )
                        .child(
                            div()
                                .id("settings-done")
                                .px_4()
                                .py_2()
                                .rounded_md()
                                .bg(palette.text)
                                .text_color(palette.background)
                                .cursor_pointer()
                                .hover(|style| style.opacity(0.85))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.close_settings(window, cx)
                                }))
                                .child("Done"),
                        ),
                )
                .with_animation(
                    "settings-dialog-enter",
                    Animation::new(Duration::from_millis(300)).with_easing(ease_out_quint()),
                    |dialog, progress| dialog.relative().top(px(10. * (1. - progress))),
                ),
        )
        .with_animation(
            "settings-overlay-enter",
            Animation::new(Duration::from_millis(300)).with_easing(ease_out_quint()),
            |overlay, progress| overlay.opacity(progress),
        )
}
