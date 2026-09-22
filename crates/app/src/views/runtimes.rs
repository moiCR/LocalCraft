use gpui::{
    Animation, AnimationExt, Context, Div, IntoElement, Render, Timer, Window, div, ease_out_quint,
    prelude::*,
};
use services::{
    AppState,
    java::{JavaProgress, JavaStage},
};
use std::{path::PathBuf, process::Stdio, time::Duration};
use tokio::sync::watch;

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
        let workspace = self;
        let state = cx.global::<AppState>();
        let palette = state.theme_manager.palette();
        let installations = match state.java_service.installations().read() {
            Ok(installations) => {
                let mut installations: Vec<_> = installations.values().cloned().collect();
                installations.sort_by_key(|installation| installation.major_version);
                installations
            }
            Err(_) => Vec::new(),
        };
        let count = format!("{} runtimes", installations.len());

        super::page(
        "Runtimes",
        "Java installations for your Minecraft servers.",
        count,
        palette,
    )
    .child(
        div()
            .flex()
            .flex_col()
            .gap_5()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_5()
                    .rounded_lg()
                    .border_1()
                    .border_color(palette.border)
                    .bg(palette.surface)
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .child("Install Java runtime"),
                    )
                    .child(div().text_sm().text_color(palette.muted).child(
                        "Choose a major version. Installed versions are shared between instances.",
                    ))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .flex_wrap()
                            .gap_2()
                            .children([8_u8, 17, 21, 25].into_iter().map(|version| {
                                let selected = workspace.runtime_version == version;
                                div()
                                    .id(("java-version", version as u32))
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
                                        palette.background
                                    } else {
                                        palette.surface
                                    })
                                    .cursor_pointer()
                                    .hover(|style| style.border_color(palette.text))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.select_runtime_version(version, cx)
                                    }))
                                    .child(format!("Java {version}"))
                            }))
                            .child(
                                div()
                                    .id("install-java-runtime")
                                    .px_4()
                                    .py_2()
                                    .rounded_md()
                                    .bg(palette.text)
                                    .text_color(palette.background)
                                    .cursor_pointer()
                                    .hover(|style| style.opacity(0.85))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        let version = this.runtime_version;
                                        this.install_runtime(version, cx);
                                    }))
                                    .child("Install"),
                            ),
                    )
                    .children(workspace.runtime_progress.as_ref().map(|progress| {
                        let version = progress.version;
                        let dismissing = workspace.runtime_notice_dismissing;
                        div()
                            .id(("java-install-progress", version as u32))
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(render_progress(progress))
                            .children(workspace.runtime_status.as_ref().map(|status| {
                                div()
                                    .text_sm()
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
                    .when(workspace.runtime_progress.is_none(), |section| {
                        section.children(workspace.runtime_status.as_ref().map(|status| {
                            div()
                                .text_sm()
                                .text_color(palette.muted)
                                .child(status.clone())
                        }))
                    }),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .children(if installations.is_empty() {
                        vec![empty_state::render_icon(
                            "icons/coffee.svg",
                            "No runtimes installed",
                            "Install a Java runtime to use it with your server instances.",
                            palette,
                        )]
                    } else {
                        installations
                            .into_iter()
                            .map(|installation| {
                                let version = installation.major_version;
                                let folder =
                                    installation.directory().map(std::path::Path::to_path_buf);
                                let open = cx.listener(move |this, _, _, cx| {
                                    if let Some(folder) = folder.clone() {
                                        this.open_runtime_folder(folder, cx);
                                    }
                                });
                                let delete = cx.listener(move |this, _, _, cx| {
                                    this.delete_runtime(version, cx)
                                });
                                div().w_full().child(
                                    RuntimeItem::new(installation).render(palette, open, delete),
                                )
                            })
                            .collect()
                    }),
            ),
    )
    }
}

fn render_progress(progress: &JavaProgress) -> Div {
    let (label, percent) = match &progress.stage {
        JavaStage::Resolving => ("Looking up Java release…", None),
        JavaStage::Cached => ("Using cached runtime…", Some(100.0)),
        JavaStage::Downloading => (
            "Downloading Java…",
            progress
                .total
                .filter(|total| *total > 0)
                .map(|total| progress.downloaded as f32 / total as f32 * 100.0),
        ),
        JavaStage::Verifying => ("Verifying Java archive…", Some(100.0)),
        JavaStage::Extracting => ("Extracting Java…", None),
        JavaStage::Linking => ("Linking runtime to instance…", None),
        JavaStage::Complete => ("Java installed.", Some(100.0)),
        JavaStage::Failed(error) => (error.as_str(), None),
    };
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(div().text_sm().child(label.to_owned()))
        .when_some(percent, |element, percent| {
            element.child(
                div()
                    .w_full()
                    .h_1()
                    .rounded_full()
                    .bg(gpui::rgb(0x333333))
                    .child(
                        div()
                            .h_full()
                            .rounded_full()
                            .bg(gpui::rgb(0x43b581))
                            .w(gpui::relative(percent.clamp(0.0, 100.0) / 100.0)),
                    ),
            )
        })
}
