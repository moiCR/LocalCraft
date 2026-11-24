use std::{sync::Arc, time::Duration};

use gpui::{
    App, AppContext, Context, Entity, Global, IntoElement, Render, Window, div, prelude::*, px, svg,
};
use gpui_kit::{base::Disableable, component::scroll::ScrollableElement};
use services::{
    AppState,
    updater::{UpdatePhase, UpdateProgress, UpdaterService},
};
use ui::{
    components::{Button, ButtonVariants, ProgressBar},
    theme::ActiveUiTheme,
};

#[derive(Clone)]
pub struct AppUpdater(pub Entity<Updater>);

impl Global for AppUpdater {}

impl AppUpdater {
    pub fn init(cx: &mut App) {
        if cx.try_global::<Self>().is_none() {
            let updater = cx.new(Updater::new);
            cx.set_global(Self(updater));
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UpdateStatus {
    Idle,
    Checking,
    Available,
    Downloading,
    Verifying,
    Installing,
    UpToDate,
    Error,
}

pub struct Updater {
    service: Option<UpdaterService>,
    update: Option<services::updater::AvailableUpdate>,
    status: UpdateStatus,
    error: Option<String>,
    progress: Option<Arc<UpdateProgress>>,
    downloaded_bytes: u64,
    total_bytes: u64,
}

impl Updater {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let service = UpdaterService::new();
        let mut updater = Self {
            service: service.ok(),
            update: None,
            status: UpdateStatus::Idle,
            error: None,
            progress: None,
            downloaded_bytes: 0,
            total_bytes: 0,
        };
        if updater.service.is_some() {
            updater.check(true, cx);
        } else {
            updater.status = UpdateStatus::Error;
            updater.error = Some("Could not initialize the update service.".into());
        }
        updater
    }

    pub fn status(&self) -> UpdateStatus {
        self.status
    }

    pub fn version(&self) -> Option<&str> {
        self.update.as_ref().map(|update| update.version.as_str())
    }

    pub fn progress_percent(&self) -> Option<u32> {
        if self.total_bytes == 0 {
            return None;
        }
        Some(((self.downloaded_bytes.saturating_mul(100)) / self.total_bytes).min(100) as u32)
    }

    pub fn activate(&mut self, cx: &mut Context<Self>) {
        match self.status {
            UpdateStatus::Available => self.install(cx),
            UpdateStatus::Checking
            | UpdateStatus::Downloading
            | UpdateStatus::Verifying
            | UpdateStatus::Installing => {}
            UpdateStatus::Idle | UpdateStatus::UpToDate | UpdateStatus::Error => {
                self.check(false, cx)
            }
        }
    }

    fn check(&mut self, silent: bool, cx: &mut Context<Self>) {
        let Some(service) = self.service.clone() else {
            return;
        };
        if matches!(
            self.status,
            UpdateStatus::Checking
                | UpdateStatus::Downloading
                | UpdateStatus::Verifying
                | UpdateStatus::Installing
        ) {
            return;
        }
        self.status = UpdateStatus::Checking;
        self.error = None;
        cx.notify();
        cx.refresh_windows();

        let runtime = cx.global::<AppState>().background_runtime.clone();
        let task = runtime.spawn(async move { service.check(env!("CARGO_PKG_VERSION")).await });
        cx.spawn(async move |this, cx| {
            let result = task
                .await
                .map_err(|error| format!("Update check task failed: {error}"))
                .and_then(|result| result.map_err(|error| format!("{error:#}")));
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(Some(update)) => {
                        this.update = Some(update);
                        this.status = UpdateStatus::Available;
                        this.error = None;
                    }
                    Ok(None) => {
                        this.update = None;
                        this.status = if silent {
                            UpdateStatus::Idle
                        } else {
                            UpdateStatus::UpToDate
                        };
                        this.error = None;
                    }
                    Err(error) => {
                        this.status = if silent {
                            UpdateStatus::Idle
                        } else {
                            UpdateStatus::Error
                        };
                        this.error = (!silent).then_some(error);
                    }
                }
                cx.notify();
                cx.refresh_windows();
            });
        })
        .detach();
    }

    fn install(&mut self, cx: &mut Context<Self>) {
        let (Some(service), Some(update)) = (self.service.clone(), self.update.clone()) else {
            return;
        };
        if !update.can_auto_install() {
            cx.open_url(&update.release_page);
            return;
        }
        if self.progress.is_some() {
            return;
        }

        let progress = Arc::new(UpdateProgress::default());
        self.progress = Some(Arc::clone(&progress));
        self.downloaded_bytes = 0;
        self.total_bytes = update.expected_size.unwrap_or(0);
        self.status = UpdateStatus::Downloading;
        self.error = None;
        cx.notify();
        cx.refresh_windows();

        let progress_poll = Arc::clone(&progress);
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
                let updated = this.update(cx, |this, cx| {
                    let Some(active) = &this.progress else {
                        return false;
                    };
                    if !Arc::ptr_eq(active, &progress_poll) {
                        return false;
                    }
                    let downloaded = progress_poll.downloaded();
                    let total = progress_poll.total();
                    let next_status = match progress_poll.phase() {
                        UpdatePhase::Downloading => UpdateStatus::Downloading,
                        UpdatePhase::Verifying => UpdateStatus::Verifying,
                        UpdatePhase::Installing => UpdateStatus::Installing,
                    };
                    if next_status != this.status
                        || downloaded != this.downloaded_bytes
                        || total != this.total_bytes
                    {
                        this.status = next_status;
                        this.downloaded_bytes = downloaded;
                        this.total_bytes = total;
                        cx.notify();
                        cx.refresh_windows();
                    }
                    true
                });
                if !matches!(updated, Ok(true)) {
                    break;
                }
            }
        })
        .detach();

        let runtime = cx.global::<AppState>().background_runtime.clone();
        let task =
            runtime.spawn(async move { service.download_and_install(&update, progress).await });
        cx.spawn(async move |this, cx| {
            let result = task
                .await
                .map_err(|error| format!("Update installation task failed: {error}"))
                .and_then(|result| result.map_err(|error| format!("{error:#}")));
            let _ = this.update(cx, |this, cx| {
                this.progress = None;
                match result {
                    Ok(()) => cx.quit(),
                    Err(error) => {
                        this.status = UpdateStatus::Error;
                        this.error = Some(error);
                        cx.notify();
                        cx.refresh_windows();
                    }
                }
            });
        })
        .detach();
    }
}

impl Render for Updater {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.ui_palette();
        let can_auto_install = self
            .update
            .as_ref()
            .is_some_and(services::updater::AvailableUpdate::can_auto_install);
        let release_page = self
            .update
            .as_ref()
            .map(|update| update.release_page.clone());

        let button_label = match self.status {
            UpdateStatus::Idle => "Check for updates".to_owned(),
            UpdateStatus::Checking => "Checking…".into(),
            UpdateStatus::Available => {
                if can_auto_install {
                    self.version()
                        .map(|version| format!("Install v{version}"))
                        .unwrap_or_else(|| "Install update".into())
                } else {
                    "View release".into()
                }
            }
            UpdateStatus::Downloading => "Downloading…".into(),
            UpdateStatus::Verifying => "Verifying…".into(),
            UpdateStatus::Installing => "Installing…".into(),
            UpdateStatus::UpToDate => "Check again".into(),
            UpdateStatus::Error => {
                if can_auto_install {
                    "Retry".into()
                } else {
                    "Check for updates".into()
                }
            }
        };

        let description = match self.status {
            UpdateStatus::Idle => format!(
                "Current version v{}. Check GitHub for new releases.",
                env!("CARGO_PKG_VERSION")
            ),
            UpdateStatus::Checking => "Checking for available updates…".to_owned(),
            UpdateStatus::Available => self
                .version()
                .map(|version| format!("Version v{version} is ready to install."))
                .unwrap_or_else(|| "A new update is available.".to_owned()),
            UpdateStatus::Downloading => {
                let downloaded_mb = self.downloaded_bytes as f64 / 1_048_576.0;
                if self.total_bytes > 0 {
                    let total_mb = self.total_bytes as f64 / 1_048_576.0;
                    let pct = self.progress_percent().unwrap_or(0);
                    format!("Downloading update: {downloaded_mb:.1} MB / {total_mb:.1} MB ({pct}%)")
                } else {
                    format!("Downloading update: {downloaded_mb:.1} MB…")
                }
            }
            UpdateStatus::Verifying => "Verifying package signature…".to_owned(),
            UpdateStatus::Installing => "Installing update and restarting…".to_owned(),
            UpdateStatus::UpToDate => {
                format!("LocalCraft v{} is up to date.", env!("CARGO_PKG_VERSION"))
            }
            UpdateStatus::Error => "Could not complete update.".to_owned(),
        };

        let busy = matches!(
            self.status,
            UpdateStatus::Checking
                | UpdateStatus::Downloading
                | UpdateStatus::Verifying
                | UpdateStatus::Installing
        );
        let available = self.status == UpdateStatus::Available;
        let retry_install = self.status == UpdateStatus::Error && can_auto_install;
        let button = Button::new("check-app-updates")
            .label(button_label)
            .rounded(px(16.0))
            .h(px(32.0))
            .disabled(busy)
            .on_click(cx.listener(move |this, _, _, cx| {
                if retry_install {
                    this.install(cx);
                } else {
                    this.activate(cx);
                }
            }));
        let button = if available { button.primary() } else { button };

        let secondary_release = release_page.filter(|_| {
            matches!(self.status, UpdateStatus::Available if can_auto_install)
                || self.status == UpdateStatus::Error
        });

        let progress_percent = (self.status == UpdateStatus::Downloading && self.total_bytes > 0)
            .then(|| {
                ((self.downloaded_bytes as f64 / self.total_bytes as f64) * 100.0).clamp(0.0, 100.0)
                    as f32
            });

        div()
            .flex()
            .flex_col()
            .gap_3()
            .rounded(px(16.0))
            .bg(palette.card_surface())
            .border_1()
            .border_color(palette.border)
            .px_4()
            .py_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        svg()
                                            .path(if busy {
                                                "icons/loader.svg"
                                            } else if available {
                                                "icons/download.svg"
                                            } else {
                                                "icons/refresh-ccw.svg"
                                            })
                                            .size_3p5()
                                            .text_color(if available {
                                                palette.accent
                                            } else {
                                                palette.muted
                                            }),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .text_color(palette.text)
                                            .child("Updates"),
                                    ),
                            )
                            .child(div().text_xs().text_color(palette.muted).child(description)),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .when_some(secondary_release, |row, url| {
                                row.child(
                                    Button::new("view-app-release")
                                        .label("View release")
                                        .rounded(px(16.0))
                                        .h(px(32.0))
                                        .on_click(move |_, _, cx| cx.open_url(&url)),
                                )
                            })
                            .child(button),
                    ),
            )
            .when_some(progress_percent, |element, percent| {
                element.child(
                    ProgressBar::new("updater-download-progress", percent).color(palette.accent),
                )
            })
            .when_some(self.error.as_ref(), |element, error| {
                element.child(
                    div()
                        .text_xs()
                        .text_color(palette.error)
                        .child(error.clone()),
                )
            })
            .when_some(
                self.update
                    .as_ref()
                    .and_then(|update| update.notes.as_ref()),
                |element, notes| {
                    element.child(
                        div()
                            .border_t_1()
                            .border_color(palette.border)
                            .pt_2p5()
                            .max_h(px(140.))
                            .overflow_y_scrollbar()
                            .text_xs()
                            .text_color(palette.muted)
                            .child(notes.clone()),
                    )
                },
            )
    }
}
