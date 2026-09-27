use gpui::{App, Context, IntoElement, div, prelude::*, svg};
use services::AppState;
use ui::theme::Appearance;

pub fn set_app_appearance(state: &mut AppState, cx: &mut App, appearance: Appearance) {
    state.theme_manager.set_appearance(appearance);
    state.preferences.dark_theme = appearance == Appearance::Dark;
    state.save_preferences();
    let mode = match appearance {
        Appearance::Dark => gpui_kit::component::theme::ThemeMode::Dark,
        Appearance::Light => gpui_kit::component::theme::ThemeMode::Light,
    };
    gpui_kit::component::theme::Theme::change(mode, None, cx);
}

use crate::workspace::Workspace;

pub fn render(cx: &Context<Workspace>) -> impl IntoElement {
    let manager = &cx.global::<AppState>().theme_manager;
    let palette = manager.palette();
    let active = manager.appearance();

    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div().flex().flex_col().gap_1().child("Appearance").child(
                div()
                    .text_color(palette.muted)
                    .text_sm()
                    .child("Choose the theme for your workspace."),
            ),
        )
        .child(
            div().flex().gap_3().children(
                [
                    (Appearance::Dark, "Dark", "theme-dark", "icons/moon.svg"),
                    (Appearance::Light, "Light", "theme-light", "icons/sun.svg"),
                ]
                .into_iter()
                .map(|(appearance, label, id, icon)| {
                    let selected = active == appearance;
                    div()
                        .id(id)
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_between()
                        .px_4()
                        .py_3()
                        .rounded_md()
                        .border_1()
                        .border_color(if selected {
                            palette.text
                        } else {
                            palette.border
                        })
                        .bg(if selected {
                            palette.surface
                        } else {
                            palette.background
                        })
                        .cursor_pointer()
                        .hover(|style| style.border_color(palette.text))
                        .on_click(move |_, _, cx| {
                            cx.update_global::<AppState, _>(|state, cx| {
                                set_app_appearance(state, cx, appearance);
                            });
                        })
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    svg()
                                        .path(icon)
                                        .size_4()
                                        .flex_shrink_0()
                                        .text_color(palette.text),
                                )
                                .child(label),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(palette.muted)
                                .child(if selected { "Selected" } else { "" }),
                        )
                }),
            ),
        )
}
