use std::sync::Arc;

use gpui::{
    App, ClickEvent, Context, FontWeight, IntoElement, Window, div, prelude::*, px, svg,
};
use gpui_kit::{base::Disableable, component::scroll::ScrollableElement};
use gpui_pages::{Page, PageAnimation, PageOptions};
use services::{AppState, playit::PlayitSnapshot};
use ui::{
    components::{Button, ButtonVariants, IconButton, card_surface},
    theme::ActiveUiTheme,
};

use crate::components::page_chrome;

const DASHBOARD_URL: &str = "https://playit.gg/account";
const DOWNLOAD_URL: &str = "https://playit.gg/download";

pub struct PlayitPage {
    snapshot: Arc<PlayitSnapshot>,
    busy: bool,
    status: Option<String>,
}

impl PlayitPage {
    fn build(cx: &mut Context<Self>) -> Self {
        let service = cx.global::<AppState>().playit_service.clone();
        let mut updates = service.subscribe();
        let snapshot = service.snapshot();
        let runtime = cx.global::<AppState>().background_runtime.clone();
        let refresh_service = service.clone();
        runtime.spawn(async move {
            let _ = refresh_service.refresh().await;
        });
        cx.spawn(async move |this, cx| {
            while updates.changed().await.is_ok() {
                let snapshot = updates.borrow_and_update().clone();
                if this
                    .update(cx, |this, cx| {
                        this.snapshot = snapshot;
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        Self {
            snapshot,
            busy: false,
            status: None,
        }
    }

    fn install(&mut self, cx: &mut Context<Self>) {
        self.run_agent_operation(
            "Downloading official Playit agent…",
            true,
            cx,
            |service| async move {
                service
                    .install()
                    .await
                    .map(|_| "Playit agent installed.".to_owned())
            },
        );
    }

    fn start(&mut self, cx: &mut Context<Self>) {
        self.run_agent_operation("Starting Playit agent…", true, cx, |service| async move {
            service
                .start()
                .await
                .map(|_| "Playit agent started.".to_owned())
        });
    }

    fn connect(&mut self, cx: &mut Context<Self>) {
        self.run_agent_operation(
            "Waiting for Playit account approval…",
            true,
            cx,
            |service| async move {
                service
                    .connect()
                    .await
                    .map(|_| "Playit account connected. Agent is running.".to_owned())
            },
        );
    }

    fn stop(&mut self, cx: &mut Context<Self>) {
        self.run_agent_operation("Stopping Playit agent…", true, cx, |service| async move {
            service
                .stop()
                .await
                .map(|_| "Playit agent stopped.".to_owned())
        });
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        self.run_agent_operation("Checking Playit agent…", true, cx, |service| async move {
            service
                .refresh()
                .await
                .map(|_| "Playit status refreshed.".to_owned())
        });
    }

    fn run_agent_operation<F, Fut>(
        &mut self,
        label: &'static str,
        busy: bool,
        cx: &mut Context<Self>,
        operation: F,
    ) where
        F: FnOnce(Arc<services::playit::PlayitService>) -> Fut + 'static,
        Fut: std::future::Future<Output = anyhow::Result<String>> + Send + 'static,
    {
        if self.busy {
            return;
        }
        self.busy = busy;
        self.status = Some(label.to_owned());
        let service = cx.global::<AppState>().playit_service.clone();
        let runtime = cx.global::<AppState>().background_runtime.clone();
        let task = runtime.spawn(operation(service));
        cx.spawn(async move |this, cx| {
            let result = task
                .await
                .map_err(|error| format!("Playit task failed: {error}"))
                .and_then(|result| result.map_err(|error| format!("{error:#}")));
            let _ = this.update(cx, |this, cx| {
                this.busy = false;
                this.status = Some(match result {
                    Ok(message) => message,
                    Err(error) => error,
                });
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
}

impl Page for PlayitPage {
    fn new(cx: &mut Context<Self>) -> Self {
        Self::build(cx)
    }

    fn options() -> PageOptions {
        PageOptions::default().on_open(PageAnimation::SlideUpFade)
    }

    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.ui_palette();
        let state = &self.snapshot;
        let instance_targets = cx
            .global::<AppState>()
            .instance_service
            .servers()
            .iter()
            .map(|server| (server.id.clone(), server.name.clone(), server.port))
            .collect::<Vec<_>>();
        let installed = state.installed_path.is_some();
        let running = state.running;
        let connected = state.connected;
        let service_status = if running {
            "Running"
        } else if installed {
            "Stopped"
        } else {
            "Not installed"
        };
        let service_color = if running {
            palette.running
        } else {
            palette.muted
        };
        let status = self.status.clone();
        let error = state.error.clone();
        let log_lines = state.logs.iter().map(|line| {
            div()
                .id(("playit-log", line.id))
                .w_full()
                .min_w_0()
                .max_w_full()
                .py_1()
                .font_family("Geist Mono")
                .text_xs()
                .text_color(palette.muted)
                .child(line.message.clone())
        });

        let status_pill = div()
            .px_3()
            .py_1()
            .rounded(px(16.0))
            .bg(palette.card_surface())
            .border_1()
            .border_color(palette.border)
            .flex()
            .items_center()
            .gap_2()
            .child(div().size(px(8.0)).rounded_full().bg(service_color))
            .child(
                div()
                    .text_xs()
                    .text_color(service_color)
                    .child(service_status),
            );

        page_chrome::canvas_outer(&palette)
            .child(
                div()
                    .id("playit-scroll")
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .overflow_y_scroll()
                    .px_6()
                    .pt_6()
                    .pb_12()
                    .flex()
                    .flex_col()
                    .gap_5()
                    .child(page_chrome::section_header(
                        "Playit",
                        "Share your server without port forwarding.",
                        status_pill,
                        &palette,
                    ))
                    .child(
                        card_surface(palette.surface, palette.surface_hover, palette.border)
                            .w_full()
                            .min_w_0()
                            .p_5()
                                .flex()
                                .flex_wrap()
                                .items_center()
                                .justify_between()
                                .gap_4()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .min_w_0()
                                        .flex_1()
                                        .gap_3()
                                        .child(
                                            div()
                                                .size(px(40.0))
                                                .rounded(px(16.0))
                                                .bg(palette.card_surface())
                                                .border_1()
                                                .border_color(palette.border)
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .flex_shrink_0()
                                                .child(
                                                    svg()
                                                        .path("icons/computer.svg")
                                                        .size_5()
                                                        .text_color(palette.text),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .flex()
                                                .flex_col()
                                                .min_w_0()
                                                .gap_1()
                                                .child(
                                                    div()
                                                        .text_sm()
                                                        .font_weight(FontWeight::SEMIBOLD)
                                                        .text_color(palette.text)
                                                        .child("Playit agent"),
                                                )
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(palette.muted)
                                                        .child(match &state.version {
                                                            Some(version) => {
                                                                format!("Official agent v{version}")
                                                            }
                                                            None if installed => {
                                                                "Official agent installed"
                                                                    .to_owned()
                                                            }
                                                            None => {
                                                                "Install official agent to expose your servers."
                                                                    .to_owned()
                                                            }
                                                        }),
                                                ),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_wrap()
                                        .items_center()
                                        .gap_2()
                                        .min_w_0()
                                        .justify_end()
                                        .when(!installed, |actions| {
                                            actions.child(
                                                IconButton::new(
                                                    "install-playit-agent",
                                                    "icons/download.svg",
                                                )
                                                .tooltip("Install agent")
                                                .color(palette.accent)
                                                .background(palette.card_surface())
                                                .disabled(self.busy)
                                                .on_click(
                                                    cx.listener(|this, _, _, cx| this.install(cx)),
                                                ),
                                            )
                                        })
                                        .when(installed && !running && !connected, |actions| {
                                            actions.child(
                                                action_button(
                                                    "connect-playit-account",
                                                    "Connect account",
                                                    self.busy,
                                                    cx.listener(|this, _, _, cx| this.connect(cx)),
                                                )
                                                .primary(),
                                            )
                                        })
                                        .when(installed && !running && connected, |actions| {
                                            actions
                                                .child(
                                                    IconButton::new(
                                                        "start-playit-agent",
                                                        "icons/resume.svg",
                                                    )
                                                    .tooltip("Start agent")
                                                    .color(palette.running)
                                                    .background(palette.card_surface())
                                                    .disabled(self.busy)
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.start(cx)
                                                    })),
                                                )
                                                .child(
                                                    IconButton::new(
                                                        "update-playit-agent",
                                                        "icons/download.svg",
                                                    )
                                                    .tooltip("Update agent")
                                                    .background(palette.card_surface())
                                                    .disabled(self.busy)
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.install(cx)
                                                    })),
                                                )
                                        })
                                        .when(installed && running, |actions| {
                                            actions.child(
                                                IconButton::new(
                                                    "stop-playit-agent",
                                                    "icons/square.svg",
                                                )
                                                .tooltip("Stop agent")
                                                .color(palette.error)
                                                .background(palette.card_surface())
                                                .disabled(self.busy)
                                                .on_click(
                                                    cx.listener(|this, _, _, cx| this.stop(cx)),
                                                ),
                                            )
                                        })
                                        .child(
                                            IconButton::new(
                                                "refresh-playit-agent",
                                                "icons/refresh-ccw.svg",
                                            )
                                            .tooltip("Refresh")
                                            .background(palette.card_surface())
                                            .disabled(self.busy)
                                            .on_click(
                                                cx.listener(|this, _, _, cx| this.refresh(cx)),
                                            ),
                                        ),
                                ),
                        )
                        .child(
                            card_surface(palette.surface, palette.surface_hover, palette.border)
                                .w_full()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .overflow_hidden()
                                .child(
                                    div()
                                        .w_full()
                                        .min_w_0()
                                        .p_4()
                                        .flex()
                                        .flex_wrap()
                                        .items_center()
                                        .justify_between()
                                        .gap_3()
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(palette.text)
                                                .child("Tunnel setup"),
                                        )
                                        .child(
                                            IconButton::new(
                                                "open-playit-dashboard",
                                                "icons/external-link.svg",
                                            )
                                            .tooltip("Open Playit dashboard")
                                            .color(palette.accent)
                                            .background(palette.card_surface())
                                            .on_click(|_, _, cx| cx.open_url(DASHBOARD_URL)),
                                        ),
                                )
                                .child(div().w_full().h(px(1.0)).bg(palette.border).opacity(0.6))
                                .child(
                                    div()
                                        .w_full()
                                        .min_w_0()
                                        .p_4()
                                        .flex()
                                        .flex_col()
                                        .gap_3()
                                        .child(
                                            div()
                                                .flex()
                                                .flex_col()
                                                .w_full()
                                                .min_w_0()
                                                .gap_1p5()
                                                .text_xs()
                                                .text_color(palette.muted)
                                                .child(if connected {
                                                    "1. Agent is linked. Create a Minecraft Java tunnel in the Playit dashboard for your server port."
                                                } else {
                                                    "1. Connect your Playit account. LocalCraft will generate a claim code and open its approval page."
                                                })
                                                .child("2. Approve the agent in your browser. LocalCraft saves the agent key locally and starts the agent.")
                                                .child("3. Create a Minecraft Java tunnel in the dashboard, then copy its public address.")
                                                .child("Tunnel creation and account settings stay in the official dashboard. LocalCraft stores the agent key locally so it can run the tunnel."),
                                        )
                                        .when_some(state.claim_code.clone(), |card, code| {
                                            let claim_url = state.claim_url.clone().unwrap_or_default();
                                            card.child(
                                                div()
                                                    .w_full()
                                                    .min_w_0()
                                                    .px_4()
                                                    .py_3()
                                                    .rounded(px(16.0))
                                                    .bg(palette.card_surface())
                                                    .border_1()
                                                    .border_color(palette.border)
                                                    .flex()
                                                    .flex_wrap()
                                                    .items_center()
                                                    .justify_between()
                                                    .gap_4()
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .flex_col()
                                                            .min_w_0()
                                                            .gap_1()
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .text_color(palette.muted)
                                                                    .child("Playit claim code"),
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_sm()
                                                                    .font_family("Geist Mono")
                                                                    .text_color(palette.text)
                                                                    .child(code),
                                                            ),
                                                    )
                                                    .child(
                                                        Button::new("open-playit-claim")
                                                            .label("Approve in browser")
                                                            .primary()
                                                            .rounded(px(16.0))
                                                            .h(px(32.0))
                                                            .on_click(move |_, _, cx| {
                                                                cx.open_url(&claim_url)
                                                            }),
                                                    ),
                                            )
                                        })
                                        .children(instance_targets.into_iter().map(
                                            |(id, name, port)| {
                                                div()
                                                    .id(format!("playit-target-{id}"))
                                                    .w_full()
                                                    .min_w_0()
                                                    .px_3()
                                                    .py_2()
                                                    .rounded(px(16.0))
                                                    .bg(palette.card_surface())
                                                    .border_1()
                                                    .border_color(palette.border)
                                                    .flex()
                                                    .flex_wrap()
                                                    .items_center()
                                                    .justify_between()
                                                    .gap_2()
                                                    .child(
                                                        div()
                                                            .min_w_0()
                                                            .text_xs()
                                                            .text_color(palette.text)
                                                            .child(name),
                                                    )
                                                    .child(
                                                        div()
                                                            .font_family("Geist Mono")
                                                            .text_xs()
                                                            .text_color(palette.muted)
                                                            .flex_shrink_0()
                                                            .child(format!("127.0.0.1:{port}")),
                                                    )
                                            },
                                        ))
                                        .children(if let Some(error) = error {
                                            vec![
                                                div()
                                                    .text_xs()
                                                    .text_color(palette.error)
                                                    .child(error),
                                            ]
                                        } else {
                                            Vec::new()
                                        })
                                        .children(status.map(|status| {
                                            div()
                                                .text_xs()
                                                .text_color(service_color)
                                                .child(status)
                                        })),
                                ),
                        )
                        .child(
                            card_surface(palette.surface, palette.surface_hover, palette.border)
                                .w_full()
                                .min_w_0()
                                .min_h_0()
                                .h(px(260.0))
                                .flex()
                                .flex_col()
                                .overflow_hidden()
                                .child(
                                    div()
                                        .w_full()
                                        .min_w_0()
                                        .px_4()
                                        .py_3()
                                        .flex()
                                        .flex_wrap()
                                        .items_center()
                                        .justify_between()
                                        .gap_2()
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(palette.text)
                                                .child("Agent output"),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(service_color)
                                                .child(service_status),
                                        ),
                                )
                                .child(div().w_full().h(px(1.0)).bg(palette.border).opacity(0.6))
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .flex_1()
                                        .w_full()
                                        .min_w_0()
                                        .min_h_0()
                                        .overflow_scrollbar()
                                        .px_4()
                                        .py_2()
                                        .children(if state.logs.is_empty() {
                                            vec![
                                                div()
                                                    .id("playit-empty-log")
                                                    .py_2()
                                                    .text_xs()
                                                    .text_color(palette.muted)
                                                    .child(if running {
                                                        "Waiting for Playit output…"
                                                    } else {
                                                        "Agent output will appear here."
                                                    }),
                                            ]
                                        } else {
                                            log_lines.collect()
                                        }),
                                ),
                        )
                        .child(
                            div().flex().justify_end().child(
                                div()
                                    .id("open-playit-downloads")
                                    .text_xs()
                                    .text_color(palette.muted)
                                    .cursor_pointer()
                                    .hover(|style| style.text_color(palette.text))
                                    .on_click(|_, _, cx| cx.open_url(DOWNLOAD_URL))
                                    .child("Playit downloads and documentation"),
                            ),
                        ),
            )
            .child(page_chrome::top_scroll_fade(palette.background))
            .child(page_chrome::bottom_scroll_fade(palette.background))
    }
}

fn action_button(
    id: &'static str,
    label: &'static str,
    disabled: bool,
    click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Button {
    Button::new(id)
        .label(label)
        .rounded(px(16.0))
        .h(px(32.0))
        .disabled(disabled)
        .on_click(click)
}
