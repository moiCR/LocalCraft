use gpui::{App, IntoElement, div, prelude::*, px, svg};
use gpui_kit::component::switch::Switch;
use services::AppState;
use ui::theme::{ActiveUiTheme, Appearance};

use crate::layout::NavigationStyle;

pub fn set_app_appearance(state: &mut AppState, cx: &mut App, appearance: Appearance) {
    state.theme_manager.set_appearance(appearance);
    state.preferences.dark_theme = appearance == Appearance::Dark;
    state.save_preferences();
    ui::theme::sync_component_theme(appearance, cx);
}

pub fn render(cx: &App) -> impl IntoElement {
    let palette = cx.ui_palette();
    let active = cx.global::<AppState>().theme_manager.appearance();
    let preferences = &cx.global::<AppState>().preferences;
    let navigation_style = NavigationStyle::from_key(&preferences.navigation_style);
    let sidebar_collapsed = preferences.sidebar_collapsed;

    let mut card = div()
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
                                .child("Navigation"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(palette.muted)
                                .child("Choose between sidebar or floating dock."),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_0p5()
                        .p_0p5()
                        .rounded(px(16.0))
                        .bg(palette.surface)
                        .border_1()
                        .border_color(palette.border)
                        .child(navigation_option(
                            NavigationStyle::Normal,
                            "Normal",
                            navigation_style,
                            palette,
                        ))
                        .child(navigation_option(
                            NavigationStyle::Dock,
                            "Dock",
                            navigation_style,
                            palette,
                        )),
                ),
        )
        .child(div().h(px(1.0)).w_full().bg(palette.border).opacity(0.6))
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
        );

    if navigation_style == NavigationStyle::Normal {
        card = card
            .child(div().h(px(1.0)).w_full().bg(palette.border).opacity(0.6))
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
                                    .child("Collapse sidebar"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(palette.muted)
                                    .child("Hide the sidebar to maximize workspace area."),
                            ),
                    )
                    .child(
                        Switch::new("sidebar-collapse")
                            .checked(sidebar_collapsed)
                            .accessibility_label("Collapse sidebar")
                            .on_change(|checked, window, cx| {
                                let collapsed = *checked;
                                cx.set_global(crate::layout::SidebarState { collapsed });
                                cx.update_global::<AppState, _>(|state, _| {
                                    state.preferences.sidebar_collapsed = collapsed;
                                    state.save_preferences();
                                });
                                window.refresh();
                            }),
                    ),
            );
    }

    card
}

fn navigation_option(
    style: NavigationStyle,
    label: &'static str,
    current: NavigationStyle,
    palette: ui::theme::Palette,
) -> impl IntoElement {
    let selected = current == style;
    let key = style.key();
    div()
        .id(("appearance-nav", style as usize))
        .flex()
        .items_center()
        .justify_center()
        .h(px(28.0))
        .px_3()
        .rounded(px(12.0))
        .text_xs()
        .font_weight(if selected {
            gpui::FontWeight::MEDIUM
        } else {
            gpui::FontWeight::NORMAL
        })
        .text_color(if selected {
            palette.text
        } else {
            palette.muted
        })
        .when(selected, |el| el.bg(palette.surface_hover))
        .cursor_pointer()
        .when(!selected, |el| {
            el.hover(move |s| s.text_color(palette.text))
        })
        .on_click(move |_, window, cx| {
            cx.update_global::<AppState, _>(|state, _| {
                if state.preferences.navigation_style != key {
                    state.preferences.navigation_style = key.to_owned();
                    state.save_preferences();
                }
            });
            window.refresh();
        })
        .child(label)
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
