use gpui::{prelude::FluentBuilder as _, *};
use gpui_kit::base::{Scrollbar, ScrollbarMode};
use ui::{
    components::{IconButton, Modal},
    theme::ActiveUiTheme,
};

use super::{appearance, settings_preferences::SettingsPreferences};
use crate::widgets::updater::AppUpdater;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    #[default]
    General,
    Appearance,
    Updates,
}

#[derive(Default, Clone, Copy)]
pub struct SettingsModalState {
    pub open: bool,
    pub tab: SettingsTab,
}

impl Global for SettingsModalState {}

#[derive(IntoElement)]
pub struct SettingsModal;

impl SettingsModal {
    pub fn new() -> Self {
        Self
    }

    fn close(window: &mut Window, cx: &mut App) {
        let tab = cx
            .try_global::<SettingsModalState>()
            .map_or(SettingsTab::default(), |state| state.tab);
        cx.set_global(SettingsModalState { open: false, tab });
        window.blur(cx);
        window.refresh();
    }

    fn render_header(tab: SettingsTab, palette: ui::theme::Palette) -> impl IntoElement {
        div()
            .flex()
            .flex_none()
            .items_center()
            .justify_between()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .p_1()
                    .rounded(px(16.0))
                    .bg(palette.card_surface())
                    .border_1()
                    .border_color(palette.border)
                    .children(
                        [
                            (SettingsTab::General, "General"),
                            (SettingsTab::Appearance, "Appearance"),
                            (SettingsTab::Updates, "Updates"),
                        ]
                        .map(|(section, label)| {
                            let selected = section == tab;
                            div()
                                .id(("settings-tab", section as usize))
                                .flex()
                                .items_center()
                                .justify_center()
                                .h(px(28.0))
                                .px_3()
                                .rounded(px(12.0))
                                .text_xs()
                                .font_weight(if selected {
                                    FontWeight::MEDIUM
                                } else {
                                    FontWeight::NORMAL
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
                                    if !selected {
                                        cx.set_global(SettingsModalState {
                                            open: true,
                                            tab: section,
                                        });
                                        window.refresh();
                                    }
                                })
                                .child(label)
                        }),
                    ),
            )
            .child(
                div().flex_none().child(
                    IconButton::new("btn-close-settings", "icons/x.svg")
                        .size(px(32.0))
                        .color(palette.muted)
                        .hover_bg(palette.surface_hover)
                        .tooltip("Close")
                        .on_click(|_, window, cx| Self::close(window, cx)),
                ),
            )
    }
}

impl Default for SettingsModal {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for SettingsModal {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let modal_state = cx
            .try_global::<SettingsModalState>()
            .copied()
            .unwrap_or_default();
        let is_open = modal_state.open;
        let tab_index = modal_state.tab as usize;
        let palette = cx.ui_palette();
        let viewport = window.viewport_size();

        let preferences = window.use_keyed_state("settings-preferences-entity", cx, |_, cx| {
            SettingsPreferences::new(cx)
        });
        let scroll_handle = window
            .use_keyed_state(("settings-scroll-handle", tab_index), cx, |_, _| {
                ScrollHandle::new()
            })
            .read(cx)
            .clone();

        let content: AnyElement = match modal_state.tab {
            SettingsTab::General => preferences.into_any_element(),
            SettingsTab::Appearance => appearance::render(cx).into_any_element(),
            SettingsTab::Updates => cx
                .try_global::<AppUpdater>()
                .map(|updater| updater.0.clone().into_any_element())
                .unwrap_or_else(|| div().into_any_element()),
        };

        div()
            .id("settings-modal-root")
            .absolute()
            .inset_0()
            .size_full()
            .capture_key_down(move |event: &KeyDownEvent, window, cx| {
                if is_open && event.keystroke.key == "escape" {
                    Self::close(window, cx);
                    cx.stop_propagation();
                }
            })
            .child(
                Modal::new("settings-modal")
                    .open(is_open)
                    .compact()
                    .width((viewport.width - px(32.0)).clamp(px(0.0), px(580.0)))
                    .max_height((viewport.height - px(32.0)).clamp(px(0.0), px(460.0)))
                    .header(Self::render_header(modal_state.tab, palette))
                    .on_close(Self::close)
                    .child(
                        div()
                            .relative()
                            .flex_1()
                            .min_h_0()
                            .pt_1()
                            .child(
                                div()
                                    .id(("settings-content", tab_index))
                                    .size_full()
                                    .overflow_y_scroll()
                                    .track_scroll(&scroll_handle)
                                    .pr_2()
                                    .child(content),
                            )
                            .child(Scrollbar::vertical(&scroll_handle).mode(ScrollbarMode::Always)),
                    ),
            )
    }
}
