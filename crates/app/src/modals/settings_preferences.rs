use gpui::{App, Context, Entity, IntoElement, Render, Subscription, Window, div, prelude::*, px};
use services::AppState;
use ui::components::input::{Input, InputEvent};

const SOFTWARE: [&str; 5] = ["Paper", "Purpur", "Fabric", "Forge", "Vanilla"];
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

    fn toggle_timestamps(&self, cx: &mut App) {
        cx.update_global::<AppState, _>(|state, _| {
            state.preferences.console_timestamps = !state.preferences.console_timestamps;
            state.save_preferences();
        });
    }

    fn toggle_auto_scroll(&self, cx: &mut App) {
        cx.update_global::<AppState, _>(|state, _| {
            state.preferences.console_auto_scroll = !state.preferences.console_auto_scroll;
            state.save_preferences();
        });
    }

    fn toggle_sidebar_collapsed(&self, cx: &mut App) {
        cx.update_global::<AppState, _>(|state, _| {
            state.preferences.sidebar_collapsed = !state.preferences.sidebar_collapsed;
            state.save_preferences();
        });
    }
}

impl Render for SettingsPreferences {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let preferences = &cx.global::<AppState>().preferences;
        let palette = cx.global::<AppState>().theme_manager.palette();
        let selected_software = preferences.default_software.as_str();
        let selected_java = preferences.default_java_major;
        let timestamps = preferences.console_timestamps;
        let auto_scroll = preferences.console_auto_scroll;
        let sidebar_collapsed = preferences.sidebar_collapsed;

        div()
            .flex()
            .flex_col()
            .gap_5()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child("Server creation")
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(palette.muted)
                                    .child("Choose the defaults for new servers."),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(div().text_xs().text_color(palette.muted).child("Software"))
                            .child(div().flex().gap_2().children(SOFTWARE.into_iter().map(
                                |software| {
                                    let selected = selected_software == software;
                                    div()
                                        .id(software)
                                        .px_3()
                                        .py_2()
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
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.select_software(software, cx);
                                        }))
                                        .child(software)
                                },
                            ))),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_4()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child("Default memory")
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(palette.muted)
                                            .child("512–262144 MiB"),
                                    ),
                            )
                            .child(div().w(px(150.)).child(self.memory.clone())),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(div().text_xs().text_color(palette.muted).child("Java"))
                            .child(div().flex().gap_2().children(JAVA_VERSIONS.into_iter().map(
                                |version| {
                                    let selected = selected_java == version;
                                    let label = version
                                        .map(|value| value.to_string())
                                        .unwrap_or_else(|| "Recommended".into());
                                    div()
                                        .id(format!("default-java-{}", version.unwrap_or(0)))
                                        .flex_1()
                                        .flex()
                                        .justify_center()
                                        .px_2()
                                        .py_2()
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
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.select_java(version, cx);
                                        }))
                                        .child(label)
                                },
                            ))),
                    ),
            )
            .child(
                div()
                    .border_t_1()
                    .border_color(palette.border)
                    .pt_4()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div().flex().flex_col().gap_1().child("Workspace").child(
                            div()
                                .text_sm()
                                .text_color(palette.muted)
                                .child("Adjust the navigation sidebar."),
                        ),
                    )
                    .child(toggle(
                        "sidebar-collapse",
                        "Collapse sidebar",
                        sidebar_collapsed,
                        palette,
                        cx.listener(|this, _, _, cx| this.toggle_sidebar_collapsed(cx)),
                    )),
            )
            .child(
                div()
                    .border_t_1()
                    .border_color(palette.border)
                    .pt_4()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div().flex().flex_col().gap_1().child("Console").child(
                            div()
                                .text_sm()
                                .text_color(palette.muted)
                                .child("Choose how server output is displayed."),
                        ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_3()
                            .child(toggle(
                                "console-timestamps",
                                "Show timestamps",
                                timestamps,
                                palette,
                                cx.listener(|this, _, _, cx| this.toggle_timestamps(cx)),
                            ))
                            .child(toggle(
                                "console-auto-scroll",
                                "Auto-scroll",
                                auto_scroll,
                                palette,
                                cx.listener(|this, _, _, cx| this.toggle_auto_scroll(cx)),
                            )),
                    ),
            )
    }
}

fn toggle(
    id: &'static str,
    label: &'static str,
    selected: bool,
    palette: &ui::theme::Palette,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let track = div()
        .w(px(36.))
        .h(px(20.))
        .px_1()
        .rounded_full()
        .bg(if selected {
            palette.accent
        } else {
            palette.surface
        })
        .flex()
        .items_center()
        .when(!selected, |track| {
            track.border_1().border_color(palette.border)
        });
    let track = if selected {
        track.justify_end()
    } else {
        track.justify_start()
    };

    div()
        .id(id)
        .flex_1()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .min_w_0()
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(palette.border)
        .cursor_pointer()
        .hover(|style| style.bg(palette.surface).border_color(palette.text))
        .on_click(on_click)
        .child(label)
        .child(
            track.child(div().size(px(14.)).rounded_full().bg(if selected {
                palette.background
            } else {
                palette.text
            })),
        )
}
