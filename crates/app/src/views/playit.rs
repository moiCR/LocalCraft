use std::sync::Arc;

use gpui::{Context, IntoElement, Render, Window, div, prelude::*, px, svg};
use gpui_kit::component::scroll::ScrollableElement;
use services::{AppState, playit::PlayitSnapshot};

const DASHBOARD_URL: &str = "https://playit.gg/account";
const DOWNLOAD_URL: &str = "https://playit.gg/download";

pub struct Playit {
    snapshot: Arc<PlayitSnapshot>,
    busy: bool,
    status: Option<String>,
}

impl Playit {
    pub fn new(cx: &mut Context<Self>) -> Self {
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

impl Render for Playit {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.global::<AppState>().theme_manager.palette();
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
            gpui::rgb(0x70c98a)
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

        super::page(
            "Playit",
            "Share your server without port forwarding.",
            service_status,
            palette,
        )
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .w_full()
                .min_w_0()
                .gap_5()
                .overflow_y_scrollbar()
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .justify_between()
                        .w_full()
                        .min_w_0()
                        .gap_4()
                        .p_5()
                        .rounded_lg()
                        .border_1()
                        .border_color(palette.border)
                        .bg(palette.surface)
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .min_w_0()
                                .flex_1()
                                .gap_4()
                                .child(
                                    div()
                                        .size_11()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded_md()
                                        .bg(palette.background)
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
                                        .child(div().font_weight(gpui::FontWeight::MEDIUM).child("Playit agent"))
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(palette.muted)
                                                .child(match &state.version {
                                                    Some(version) => format!("Official agent v{version}"),
                                                    None if installed => "Official agent installed".to_owned(),
                                                    None => "Install official agent to expose your servers.".to_owned(),
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
                                .flex_1()
                                .justify_end()
                                .when(!installed, |actions| {
                                    actions.child(action_button(
                                        "install-playit-agent",
                                        "Install",
                                        palette,
                                        self.busy,
                                        cx.listener(|this, _, _, cx| this.install(cx)),
                                    ))
                                })
                                .when(installed && !running && !connected, |actions| {
                                    actions.child(action_button(
                                        "connect-playit-account",
                                        "Connect account",
                                        palette,
                                        self.busy,
                                        cx.listener(|this, _, _, cx| this.connect(cx)),
                                    ))
                                })
                                .when(installed && !running && connected, |actions| {
                                    actions.child(action_button(
                                        "start-playit-agent",
                                        "Start agent",
                                        palette,
                                        self.busy,
                                        cx.listener(|this, _, _, cx| this.start(cx)),
                                    ))
                                    .child(action_button(
                                        "update-playit-agent",
                                        "Update",
                                        palette,
                                        self.busy,
                                        cx.listener(|this, _, _, cx| this.install(cx)),
                                    ))
                                })
                                .when(installed && running, |actions| {
                                    actions.child(action_button(
                                        "stop-playit-agent",
                                        "Stop agent",
                                        palette,
                                        self.busy,
                                        cx.listener(|this, _, _, cx| this.stop(cx)),
                                    ))
                                })
                                .child(action_button(
                                    "refresh-playit-agent",
                                    "Refresh",
                                    palette,
                                    self.busy,
                                    cx.listener(|this, _, _, cx| this.refresh(cx)),
                                )),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .w_full()
                        .min_w_0()
                        .gap_4()
                        .p_5()
                        .rounded_lg()
                        .border_1()
                        .border_color(palette.border)
                        .bg(palette.surface)
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .items_center()
                                .justify_between()
                                .gap_3()
                                .w_full()
                                .min_w_0()
                                .child(div().font_weight(gpui::FontWeight::MEDIUM).child("Tunnel setup"))
                                .child(
                                    div()
                                        .id("open-playit-dashboard")
                                        .px_3()
                                        .py_2()
                                        .rounded_md()
                                        .bg(palette.text)
                                        .text_color(palette.background)
                                        .cursor_pointer()
                                        .hover(|style| style.opacity(0.85))
                                        .on_click(|_, _, cx| cx.open_url(DASHBOARD_URL))
                                        .child("Open Playit dashboard"),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .w_full()
                                .min_w_0()
                                .gap_2()
                                .text_sm()
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
                                    .flex()
                                    .flex_wrap()
                                    .items_center()
                                    .justify_between()
                                    .w_full()
                                    .min_w_0()
                                    .gap_4()
                                    .px_3()
                                    .py_3()
                                    .rounded_md()
                                    .bg(palette.background)
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .min_w_0()
                                            .gap_1()
                                            .child(div().text_xs().text_color(palette.muted).child("Playit claim code"))
                                            .child(div().font_family("Geist Mono").child(code)),
                                    )
                                    .child(
                                        div()
                                            .id("open-playit-claim")
                                            .px_3()
                                            .py_2()
                                            .flex_shrink_0()
                                            .rounded_md()
                                            .bg(palette.text)
                                            .text_color(palette.background)
                                            .cursor_pointer()
                                            .hover(|style| style.opacity(0.85))
                                            .on_click(move |_, _, cx| cx.open_url(&claim_url))
                                            .child("Approve in browser"),
                                    ),
                            )
                        })
                        .children(instance_targets.into_iter().map(|(id, name, port)| {
                            div()
                                .id(format!("playit-target-{id}"))
                                .flex()
                                .flex_wrap()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .w_full()
                                .min_w_0()
                                .px_3()
                                .py_2()
                                .rounded_md()
                                .bg(palette.background)
                                .child(div().min_w_0().child(name))
                                .child(
                                    div()
                                        .font_family("Geist Mono")
                                        .text_color(palette.text)
                                        .flex_shrink_0()
                                        .child(format!("127.0.0.1:{port}")),
                                )
                        }))
                        .children(if let Some(error) = error {
                            vec![div().text_sm().text_color(gpui::rgb(0xe06c75)).child(error)]
                        } else {
                            Vec::new()
                        })
                        .children(status.map(|status| {
                            div().text_sm().text_color(service_color).child(status)
                        })),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .w_full()
                        .min_w_0()
                        .min_h_0()
                        .h(px(260.))
                        .rounded_lg()
                        .border_1()
                        .border_color(palette.border)
                        .bg(palette.surface)
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .min_w_0()
                                .px_4()
                                .py_3()
                                .border_b_1()
                                .border_color(palette.border)
                                .child(div().font_weight(gpui::FontWeight::MEDIUM).child("Agent output"))
                                .child(div().text_xs().text_color(service_color).child(service_status)),
                        )
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
                                    vec![div()
                                        .id("playit-empty-log")
                                        .py_2()
                                        .text_color(palette.muted)
                                        .child(if running {
                                            "Waiting for Playit output…"
                                        } else {
                                            "Agent output will appear here."
                                        })]
                                } else {
                                    log_lines.collect()
                                }),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .justify_end()
                        .child(
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
    }
}

fn action_button(
    id: &'static str,
    label: &'static str,
    palette: &ui::theme::Palette,
    disabled: bool,
    click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(palette.border)
        .text_color(if disabled {
            palette.muted
        } else {
            palette.text
        })
        .when(!disabled, |button| {
            button
                .cursor_pointer()
                .hover(|style| style.bg(palette.background))
        })
        .when(disabled, |button| button.opacity(0.55))
        .on_click(click)
        .child(label)
}
