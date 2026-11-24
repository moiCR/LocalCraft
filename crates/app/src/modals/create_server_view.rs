use super::create_server::CreateServerForm;
use gpui::{prelude::FluentBuilder as _, *};
use gpui_kit::{
    base::{Disableable, Scrollbar, ScrollbarMode},
    component::switch::Switch,
};
use ui::{
    components::{Button, ButtonVariants, IconButton, Modal, input::Input, select},
    theme::ActiveUiTheme,
};

pub fn input_field(label: &'static str, input: &Entity<Input>) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w_0()
        .gap_1p5()
        .child(div().text_xs().child(label))
        .child(input.clone())
}

impl Render for CreateServerForm {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.ui_palette();
        let viewport = window.viewport_size();
        let is_pumpkin = self
            .software
            .read(cx)
            .value()
            .is_some_and(services::software::pumpkin::is_pumpkin);
        let form = cx.entity().downgrade();
        let locked = self.busy || self.closing;

        div()
            .size_full()
            .capture_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    this.close(window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(
                Modal::new("create-server-modal")
                    .open(!self.closing)
                    .compact()
                    .fit_content()
                    .width((viewport.width - px(32.0)).clamp(px(0.0), px(560.0)))
                    .max_height((viewport.height - px(32.0)).max(px(0.0)))
                    .header(
                        div()
                            .flex()
                            .flex_none()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .pl_1()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_0p5()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(palette.text)
                                            .child("Create server"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(palette.muted)
                                            .child("Your next world starts here."),
                                    ),
                            )
                            .child(
                                IconButton::new("dismiss-create", "icons/x.svg")
                                    .size(px(32.0))
                                    .color(palette.muted)
                                    .hover_bg(palette.surface_hover)
                                    .disabled(self.busy)
                                    .tooltip("Close")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.close(window, cx);
                                    })),
                            ),
                    )
                    .on_close(move |window, cx| {
                        let _ = form.update(cx, |this, cx| this.close(window, cx));
                    })
                    .child(
                        div()
                            .relative()
                            .min_h_0()
                            .child(
                                div()
                                    .id("create-server-fields")
                                    .max_h((viewport.height - px(160.0)).max(px(0.0)))
                                    .overflow_y_scroll()
                                    .track_scroll(&self.scroll)
                                    .pr_2()
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_3()
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    .gap_3()
                                                    .rounded(px(16.0))
                                                    .bg(palette.card_surface())
                                                    .border_1()
                                                    .border_color(palette.border)
                                                    .p_4()
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .flex_col()
                                                            .gap_1p5()
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .text_color(palette.muted)
                                                                    .child("Name"),
                                                            )
                                                            .child(self.name.clone()),
                                                    )
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .gap_3()
                                                            .text_color(palette.muted)
                                                            .child(select::field(
                                                                "Software",
                                                                &self.software,
                                                            ))
                                                            .child(select::field(
                                                                "Minecraft version",
                                                                &self.version,
                                                            ))
                                                            .when(!is_pumpkin, |row| {
                                                                row.child(select::field(
                                                                    "Java version",
                                                                    &self.java,
                                                                ))
                                                            }),
                                                    )
                                                    .when(is_pumpkin, |el| {
                                                        el.child(
                                                            div()
                                                                .text_xs()
                                                                .text_color(palette.muted)
                                                                .child("Experimental. Latest Minecraft release only. No Java required; Bedrock is disabled."),
                                                        )
                                                    })
                                                    .when(self.loading, |el| {
                                                        el.child(
                                                            div()
                                                                .text_xs()
                                                                .text_color(palette.muted)
                                                                .child("Fetching available releases…"),
                                                        )
                                                    })
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .gap_3()
                                                            .text_color(palette.muted)
                                                            .when(!is_pumpkin, |row| {
                                                                row.child(input_field(
                                                                    "Memory (MiB)",
                                                                    &self.ram,
                                                                ))
                                                            })
                                                            .child(input_field(
                                                                "Port",
                                                                &self.port,
                                                            )),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .justify_between()
                                                    .gap_4()
                                                    .rounded(px(16.0))
                                                    .bg(palette.card_surface())
                                                    .border_1()
                                                    .border_color(palette.border)
                                                    .px_4()
                                                    .py_3()
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .flex_col()
                                                            .gap_0p5()
                                                            .child(
                                                                div()
                                                                    .flex()
                                                                    .items_center()
                                                                    .gap_1p5()
                                                                    .text_sm()
                                                                    .font_weight(FontWeight::MEDIUM)
                                                                    .text_color(palette.text)
                                                                    .child("Accept Minecraft")
                                                                    .child(
                                                                        div()
                                                                            .id("minecraft-eula")
                                                                            .cursor_pointer()
                                                                            .underline()
                                                                            .text_color(palette.accent)
                                                                            .child("EULA")
                                                                            .on_click(|_, _, cx| {
                                                                                cx.open_url(
                                                                                    "https://www.minecraft.net/eula",
                                                                                );
                                                                            }),
                                                                    ),
                                                            )
                                                            .when(!is_pumpkin, |col| {
                                                                col.child(
                                                                    div()
                                                                        .text_xs()
                                                                        .text_color(palette.muted)
                                                                        .child("Java is installed automatically when needed."),
                                                                )
                                                            }),
                                                    )
                                                    .child(
                                                        Switch::new("accept-eula")
                                                            .checked(self.accepted_eula)
                                                            .disabled(locked)
                                                            .accessibility_label("Accept Minecraft EULA")
                                                            .on_change(cx.listener(
                                                                |this, checked, _, cx| {
                                                                    if !this.busy {
                                                                        this.accepted_eula = *checked;
                                                                        cx.notify();
                                                                    }
                                                                },
                                                            )),
                                                    ),
                                            )
                                            .when_some(self.error.as_ref(), |el, error| {
                                                el.child(
                                                    div()
                                                        .flex()
                                                        .items_center()
                                                        .justify_between()
                                                        .gap_3()
                                                        .px_1()
                                                        .child(
                                                            div()
                                                                .flex_1()
                                                                .text_xs()
                                                                .text_color(palette.error)
                                                                .child(error.clone()),
                                                        )
                                                        .when(!self.busy, |row| {
                                                            row.child(
                                                                Button::new("retry-catalog")
                                                                    .label("Reload versions")
                                                                    .rounded(px(16.0))
                                                                    .h(px(28.0))
                                                                    .on_click(cx.listener(
                                                                        |this, _, _, cx| {
                                                                            this.fetch_versions(cx);
                                                                        },
                                                                    )),
                                                            )
                                                        }),
                                                )
                                            })
                                            .when_some(self.status.as_ref(), |el, status| {
                                                el.child(
                                                    div()
                                                        .px_1()
                                                        .text_xs()
                                                        .text_color(palette.muted)
                                                        .child(status.clone()),
                                                )
                                            }),
                                    ),
                            )
                            .child(Scrollbar::vertical(&self.scroll).mode(ScrollbarMode::Always)),
                    )
                    .footer(
                        div()
                            .flex()
                            .flex_none()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new("cancel-create")
                                    .label("Cancel")
                                    .rounded(px(16.0))
                                    .h(px(32.0))
                                    .disabled(self.busy)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.close(window, cx);
                                    })),
                            )
                            .child(
                                Button::new("confirm-create")
                                    .label(if self.busy {
                                        "Creating…"
                                    } else {
                                        "Create server"
                                    })
                                    .primary()
                                    .rounded(px(16.0))
                                    .h(px(32.0))
                                    .disabled(locked || self.loading)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.create(window, cx);
                                    })),
                            ),
                    ),
            )
    }
}
