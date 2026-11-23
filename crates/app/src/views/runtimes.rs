use gpui::{
    Animation, AnimationExt, Context, Div, FontWeight, IntoElement, Render, Window, div,
    ease_out_quint, prelude::*, px,
};
use services::{
    AppState,
    java::{JavaProgress, JavaStage},
};
use smol::Timer;
use std::{path::PathBuf, process::Stdio, time::Duration};
use tokio::sync::watch;
use ui::{
    components::{Button, ButtonVariants, ProgressBar, card_surface},
    theme::{ActiveUiTheme, Palette},
};

use crate::widgets::{empty_state, runtime_item::RuntimeItem};

pub struct Runtimes {
    pub runtime_version: u8,
    pub runtime_progress: Option<JavaProgress>,
    pub runtime_status: Option<String>,
    pub runtime_notice_dismissing: bool,
    runtime_notice_id: u64,
}

impl Runtimes {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Self {
            runtime_version: 21,
            runtime_progress: None,
            runtime_status: None,
            runtime_notice_dismissing: false,
            runtime_notice_id: 0,
        }
    }

    fn select_runtime_version(&mut self, version: u8, cx: &mut Context<Self>) {
        self.runtime_version = version;
        cx.notify();
    }

    fn install_runtime(&mut self, version: u8, cx: &mut Context<Self>) {
        if self.runtime_progress.as_ref().is_some_and(|progress| {
            !matches!(progress.stage, JavaStage::Complete | JavaStage::Failed(_))
        }) {
            return;
        }
        let (progress, mut updates) = watch::channel(JavaProgress::new(version));
        let service = cx.global::<AppState>().java_service.clone();
        let runtime = cx.global::<AppState>().background_runtime.clone();
        let install = runtime.spawn(async move { service.download(version, &progress).await });
        self.runtime_progress = Some(JavaProgress::new(version));
        self.runtime_status = None;
        self.runtime_notice_dismissing = false;
        self.runtime_notice_id = self.runtime_notice_id.wrapping_add(1);
        let notice_id = self.runtime_notice_id;
        cx.notify();
        cx.spawn(async move |this, cx| {
            loop {
                let snapshot = updates.borrow().clone();
                let terminal = matches!(snapshot.stage, JavaStage::Complete | JavaStage::Failed(_));
                let _ = this.update(cx, |runtimes, cx| {
                    runtimes.runtime_progress = Some(snapshot);
                    cx.notify();
                });
                if terminal || updates.changed().await.is_err() {
                    break;
                }
            }
            let result = install.await;
            let succeeded = matches!(result, Ok(Ok(_)));
            let success_message = format!("Java {version} installed.");
            let _ = this.update(cx, |runtimes, cx| {
                runtimes.runtime_status = Some(match result {
                    Ok(Ok(_)) => success_message.clone(),
                    Ok(Err(error)) => format!("Could not install Java {version}: {error:#}"),
                    Err(error) => format!("Java installation task failed: {error}"),
                });
                cx.notify();
            });
            if succeeded {
                Timer::after(Duration::from_millis(1800)).await;
                let _ = this.update(cx, |runtimes, cx| {
                    let is_this_notice = runtimes.runtime_notice_id == notice_id
                        && runtimes.runtime_progress.as_ref().is_some_and(|progress| {
                            progress.version == version
                                && matches!(progress.stage, JavaStage::Complete)
                        })
                        && runtimes.runtime_status.as_deref() == Some(success_message.as_str());
                    if is_this_notice {
                        runtimes.runtime_notice_dismissing = true;
                        cx.notify();
                    }
                });
                Timer::after(Duration::from_millis(250)).await;
                let _ = this.update(cx, |runtimes, cx| {
                    let is_this_notice = runtimes.runtime_notice_id == notice_id
                        && runtimes.runtime_progress.as_ref().is_some_and(|progress| {
                            progress.version == version
                                && matches!(progress.stage, JavaStage::Complete)
                        })
                        && runtimes.runtime_status.as_deref() == Some(success_message.as_str());
                    if is_this_notice {
                        runtimes.runtime_progress = None;
                        runtimes.runtime_status = None;
                        runtimes.runtime_notice_dismissing = false;
                        cx.notify();
                    }
                });
            }
        })
        .detach();
    }

    fn delete_runtime(&mut self, version: u8, cx: &mut Context<Self>) {
        let service = cx.global::<AppState>().java_service.clone();
        let runtime = cx.global::<AppState>().background_runtime.clone();
        self.runtime_status = Some(format!("Deleting Java {version}…"));
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move { service.delete(version).await })
                .await;
            let _ = this.update(cx, |runtimes, cx| {
                runtimes.runtime_status = Some(match result {
                    Ok(Ok(())) => format!("Java {version} removed."),
                    Ok(Err(error)) => format!("Could not remove Java {version}: {error:#}"),
                    Err(error) => format!("Java removal task failed: {error}"),
                });
                cx.notify();
            });
        })
        .detach();
    }

    fn open_runtime_folder(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let runtime = cx.global::<AppState>().background_runtime.clone();
        cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move {
                    let mut command = if cfg!(target_os = "windows") {
                        let mut command = tokio::process::Command::new("explorer");
                        command.arg(path);
                        command
                    } else if cfg!(target_os = "macos") {
                        let mut command = tokio::process::Command::new("open");
                        command.arg(path);
                        command
                    } else {
                        let mut command = tokio::process::Command::new("xdg-open");
                        command.arg(path);
                        command
                    };
                    command
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .kill_on_drop(false)
                        .spawn()
                        .map_err(|error| error.to_string())
                })
                .await;
            if let Err(error) = result
                .map_err(|error| error.to_string())
                .and_then(|result| result)
            {
                let _ = this.update(cx, |runtimes, cx| {
                    runtimes.runtime_status =
                        Some(format!("Could not open runtime folder: {error:#}"));
                    cx.notify();
                });
            }
        })
        .detach();
    }
}

impl Render for Runtimes {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.ui_palette();
        let state = cx.global::<AppState>();
        let installations = match state.java_service.installations().read() {
            Ok(installations) => {
                let mut installations: Vec<_> = installations.values().cloned().collect();
                installations.sort_by_key(|installation| installation.major_version);
                installations
            }
            Err(_) => Vec::new(),
        };
        let count = if installations.len() == 1 {
            "1 runtime".to_owned()
        } else {
            format!("{} runtimes", installations.len())
        };

        let is_empty = installations.is_empty();
        let list_content = if is_empty {
            vec![empty_state::render_icon(
                "icons/coffee.svg",
                "No runtimes installed",
                "Install a Java runtime to use it with your server instances.",
                &palette,
            )]
        } else {
            installations
                .into_iter()
                .map(|installation| {
                    let version = installation.major_version;
                    let folder = installation.directory().map(std::path::Path::to_path_buf);
                    let open = cx.listener(move |this, _, _, cx| {
                        if let Some(folder) = folder.clone() {
                            this.open_runtime_folder(folder, cx);
                        }
                    });
                    let delete =
                        cx.listener(move |this, _, _, cx| this.delete_runtime(version, cx));
                    div()
                        .w_full()
                        .child(RuntimeItem::new(installation).render(&palette, open, delete))
                })
                .collect()
        };

        super::canvas_outer(&palette)
            .child(
                div()
                    .id("runtimes-scroll")
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
                    .child(super::section_header(
                        "Java Runtimes",
                        "Install and manage Java installations shared across server instances.",
                        super::count_badge(count, &palette),
                        &palette,
                    ))
                    .child(
                        card_surface(palette.surface, palette.surface_hover, palette.border)
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap_4()
                            .p_5()
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(palette.text)
                                                .child("Install Java runtime"),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(palette.muted)
                                                .child("Choose a major version. Installed versions are shared between instances."),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .flex_wrap()
                                        .gap_3()
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .flex_wrap()
                                                .gap_1()
                                                .p_1()
                                                .rounded(px(16.0))
                                                .bg(palette.surface)
                                                .border_1()
                                                .border_color(palette.border)
                                                .children([8_u8, 17, 21, 25].into_iter().map(
                                                    |version| {
                                                        let selected =
                                                            self.runtime_version == version;
                                                        div()
                                                            .id(("java-version", version as u32))
                                                            .px_3()
                                                            .py_1p5()
                                                            .rounded(px(12.0))
                                                            .text_xs()
                                                            .font_weight(FontWeight::MEDIUM)
                                                            .cursor_pointer()
                                                            .text_color(if selected {
                                                                palette.text
                                                            } else {
                                                                palette.muted
                                                            })
                                                            .when(selected, |item| {
                                                                item.bg(palette.surface_hover)
                                                            })
                                                            .when(!selected, |item| {
                                                                item.hover(|style| {
                                                                    style.text_color(palette.text)
                                                                })
                                                            })
                                                            .on_click(cx.listener(
                                                                move |this, _, _, cx| {
                                                                    this.select_runtime_version(
                                                                        version, cx,
                                                                    )
                                                                },
                                                            ))
                                                            .child(format!("Java {version}"))
                                                    },
                                                )),
                                        )
                                        .child(
                                            Button::new("install-java-runtime")
                                                .label("Install")
                                                .primary()
                                                .rounded(px(16.0))
                                                .h(px(32.0))
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    let version = this.runtime_version;
                                                    this.install_runtime(version, cx);
                                                })),
                                        ),
                                )
                                .children(self.runtime_progress.as_ref().map(|progress| {
                                    let version = progress.version;
                                    let dismissing = self.runtime_notice_dismissing;
                                    div()
                                        .id(("java-install-progress", version as u32))
                                        .flex()
                                        .flex_col()
                                        .gap_2()
                                        .child(render_progress(progress, &palette))
                                        .children(self.runtime_status.as_ref().map(|status| {
                                            div()
                                                .text_xs()
                                                .text_color(palette.muted)
                                                .child(status.clone())
                                        }))
                                        .with_animation(
                                            (
                                                if dismissing {
                                                    "java-install-dismiss"
                                                } else {
                                                    "java-install-notice"
                                                },
                                                version as u32,
                                            ),
                                            Animation::new(Duration::from_millis(250))
                                                .with_easing(ease_out_quint()),
                                            move |notice, progress| {
                                                if dismissing {
                                                    notice.opacity(1.0 - progress)
                                                } else {
                                                    notice
                                                }
                                            },
                                        )
                                }))
                                .when(self.runtime_progress.is_none(), |section| {
                                    section.children(self.runtime_status.as_ref().map(|status| {
                                        div()
                                            .text_xs()
                                            .text_color(palette.muted)
                                            .child(status.clone())
                                    }))
                                }),
                        )
                        .child(div().flex().flex_col().gap_3().children(list_content)),
            )
            .child(super::top_scroll_fade(palette.background))
            .child(super::bottom_scroll_fade(palette.background))
    }
}

fn render_progress(progress: &JavaProgress, palette: &Palette) -> Div {
    let (label, percent, failed) = match &progress.stage {
        JavaStage::Resolving => ("Looking up Java release…", None, false),
        JavaStage::Cached => ("Using cached runtime…", Some(100.0), false),
        JavaStage::Downloading => (
            "Downloading Java…",
            progress
                .total
                .filter(|total| *total > 0)
                .map(|total| progress.downloaded as f32 / total as f32 * 100.0),
            false,
        ),
        JavaStage::Verifying => ("Verifying Java archive…", Some(100.0), false),
        JavaStage::Extracting => ("Extracting Java…", None, false),
        JavaStage::Linking => ("Linking runtime to instance…", None, false),
        JavaStage::Complete => ("Java installed.", Some(100.0), false),
        JavaStage::Failed(error) => (error.as_str(), None, true),
    };
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_xs()
                .text_color(if failed { palette.error } else { palette.text })
                .child(label.to_owned()),
        )
        .when_some(percent, |element, percent| {
            element.child(
                ProgressBar::new(("java-progress-bar", progress.version as u32), percent).color(
                    if failed {
                        palette.error
                    } else {
                        palette.accent
                    },
                ),
            )
        })
}
