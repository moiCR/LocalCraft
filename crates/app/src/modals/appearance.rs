use gpui::{App, IntoElement, div, prelude::*, px, svg};
use services::AppState;
use ui::theme::{ActiveUiTheme, Appearance};

pub fn set_app_appearance(state: &mut AppState, cx: &mut App, appearance: Appearance) {
    state.theme_manager.set_appearance(appearance);
    state.preferences.dark_theme = appearance == Appearance::Dark;
    state.save_preferences();
    ui::theme::sync_component_theme(appearance, cx);
}

pub fn render(cx: &App) -> impl IntoElement {
    let palette = cx.ui_palette();
    let active = cx.global::<AppState>().theme_manager.appearance();

    div()
        .flex()
        .flex_col()
        .rounded(px(16.0))
        .bg(palette.card_surface())
        .border_1()
        .border_color(palette.border)
        .px_4()
        .py_1()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_4()
                .py_2p5()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(palette.text)
                                .child("Theme"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(palette.muted)
                                .child("Switch between dark and light surfaces."),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .p_0p5()
                        .rounded_full()
                        .bg(palette.surface)
                        .border_1()
                        .border_color(palette.border)
                        .child(theme_option(
                            Appearance::Dark,
                            "theme-dark",
                            "icons/moon.svg",
                            active,
                            palette,
                        ))
                        .child(theme_option(
                            Appearance::Light,
                            "theme-light",
                            "icons/sun.svg",
                            active,
                            palette,
                        )),
                ),
        )
}

fn theme_option(
    appearance: Appearance,
    id: &'static str,
    icon: &'static str,
    active: Appearance,
    palette: ui::theme::Palette,
) -> impl IntoElement {
    let selected = active == appearance;
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .size(px(28.0))
        .rounded_full()
        .when(selected, |el| el.bg(palette.surface_hover))
        .cursor_pointer()
        .on_click(move |_, window, cx| {
            cx.update_global::<AppState, _>(|state, cx| {
                set_app_appearance(state, cx, appearance);
            });
            window.refresh();
        })
        .child(svg().path(icon).size(px(15.0)).text_color(if selected {
            palette.text
        } else {
            palette.muted
        }))
}
