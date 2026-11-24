use gpui::{App, Context, Entity, IntoElement, Render, Subscription, Window, div, prelude::*, px};
use gpui_kit::component::switch::Switch;
use services::AppState;
use ui::{
    components::{
        Button, ButtonVariants,
        input::{Input, InputEvent},
    },
    theme::ActiveUiTheme,
};

const SOFTWARE: [&str; 6] = ["Paper", "Purpur", "Fabric", "Forge", "Vanilla", "Pumpkin"];
const JAVA_VERSIONS: [Option<u8>; 5] = [None, Some(8), Some(17), Some(21), Some(25)];

pub struct SettingsPreferences {
    memory: Entity<Input>,
    _memory_subscription: Subscription,
}

impl SettingsPreferences {
    pub fn new(cx: &mut Context<Self>) -> Self {
        cx.observe_global::<AppState>(|_, cx| cx.notify()).detach();
        let initial_memory = cx
            .global::<AppState>()
            .preferences
            .default_ram_mib
            .to_string();
        let memory = cx.new(|cx| Input::new(initial_memory, "MiB", cx));
        let memory_subscription = cx.subscribe(&memory, |_, memory, event, cx| {
            if !matches!(event, InputEvent::Changed) {
                return;
            }
            let Ok(value) = memory.read(cx).value().parse::<u32>() else {
                return;
            };
            if !(512..=262_144).contains(&value) {
                return;
            }
            cx.update_global::<AppState, _>(|state, _| {
                if state.preferences.default_ram_mib != value {
                    state.preferences.default_ram_mib = value;
                    state.save_preferences();
                }
            });
        });
        Self {
            memory,
            _memory_subscription: memory_subscription,
        }
    }

    fn select_software(&self, software: &'static str, cx: &mut App) {
        cx.update_global::<AppState, _>(|state, _| {
            if state.preferences.default_software != software {
                state.preferences.default_software = software.into();
                state.save_preferences();
            }
        });
    }

    fn select_java(&self, java: Option<u8>, cx: &mut App) {
        cx.update_global::<AppState, _>(|state, _| {
            if state.preferences.default_java_major != java {
                state.preferences.default_java_major = java;
                state.save_preferences();
            }
        });
    }

    fn set_timestamps(&self, enabled: bool, cx: &mut App) {
        cx.update_global::<AppState, _>(|state, _| {
            if state.preferences.console_timestamps != enabled {
                state.preferences.console_timestamps = enabled;
                state.save_preferences();
            }
        });
    }

    fn set_auto_scroll(&self, enabled: bool, cx: &mut App) {
        cx.update_global::<AppState, _>(|state, _| {
            if state.preferences.console_auto_scroll != enabled {
                state.preferences.console_auto_scroll = enabled;
                state.save_preferences();
            }
        });
    }
}

impl Render for SettingsPreferences {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let preferences = &cx.global::<AppState>().preferences;
        let palette = cx.ui_palette();
        let selected_software = preferences.default_software.as_str();
        let selected_java = preferences.default_java_major;
        let timestamps = preferences.console_timestamps;
        let auto_scroll = preferences.console_auto_scroll;

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
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
                            .flex_col()
                            .gap_2()
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
                                            .child("Default software"),
                                    )
                                    .child(
                                        div().text_xs().text_color(palette.muted).child(
                                            "Preselected server software for new instances.",
                                        ),
                                    ),
                            )
                            .child(div().flex().flex_wrap().gap_1p5().children(
                                SOFTWARE.into_iter().map(|software| {
                                    let selected = selected_software == software;
                                    let button = Button::new(software)
                                        .label(if software == "Pumpkin" {
                                            "Pumpkin · Exp"
                                        } else {
                                            software
                                        })
                                        .rounded(px(16.0))
                                        .h(px(30.0))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.select_software(software, cx);
                                        }));
                                    if selected { button.primary() } else { button }
                                }),
                            )),
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
                                            .child("Default memory"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(palette.muted)
                                            .child("512–262144 MiB"),
                                    ),
                            )
                            .child(div().w(px(130.)).child(self.memory.clone())),
                    )
                    .child(div().h(px(1.0)).w_full().bg(palette.border).opacity(0.6))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .py_2p5()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(palette.text)
                                    .child("Default Java"),
                            )
                            .child(div().flex().flex_wrap().gap_1p5().children(
                                JAVA_VERSIONS.into_iter().map(|version| {
                                    let selected = selected_java == version;
                                    let label = version
                                        .map(|value| format!("Java {value}"))
                                        .unwrap_or_else(|| "Recommended".into());
                                    let button = Button::new(format!(
                                        "default-java-{}",
                                        version.unwrap_or(0)
                                    ))
                                    .label(label)
                                    .rounded(px(16.0))
                                    .h(px(30.0))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.select_java(version, cx);
                                    }));
                                    if selected { button.primary() } else { button }
                                }),
                            )),
                    ),
            )
            .child(
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
                                            .child("Show timestamps"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(palette.muted)
                                            .child("Display timestamps on console lines."),
                                    ),
                            )
                            .child(
                                Switch::new("console-timestamps")
                                    .checked(timestamps)
                                    .accessibility_label("Show timestamps")
                                    .on_change(cx.listener(|this, checked, _, cx| {
                                        this.set_timestamps(*checked, cx);
                                    })),
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
                                            .child("Auto-scroll"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(palette.muted)
                                            .child("Keep console pinned to the latest output."),
                                    ),
                            )
                            .child(
                                Switch::new("console-auto-scroll")
                                    .checked(auto_scroll)
                                    .accessibility_label("Auto-scroll")
                                    .on_change(cx.listener(|this, checked, _, cx| {
                                        this.set_auto_scroll(*checked, cx);
                                    })),
                            ),
                    ),
            )
    }
}
