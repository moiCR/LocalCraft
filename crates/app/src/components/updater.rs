use gpui::{
    App, AppContext, Context, Entity, Global, IntoElement, Render, Window, div, prelude::*, px, svg,
};
use gpui_kit::{base::Disableable, component::scroll::ScrollableElement};
use services::{AppState, updater::UpdaterService};
use ui::{
    components::{Button, ButtonVariants},
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
    UpToDate,
    Error,
}

pub struct Updater {
    service: Option<UpdaterService>,
    update: Option<services::updater::AvailableUpdate>,
    status: UpdateStatus,
    error: Option<String>,
}

impl Updater {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let service = UpdaterService::new();
        let mut updater = Self {
            service: service.ok(),
            update: None,
            status: UpdateStatus::Idle,
            error: None,
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

    pub fn activate(&mut self, cx: &mut Context<Self>) {
        match self.status {
            UpdateStatus::Available => self.install(cx),
            UpdateStatus::Checking | UpdateStatus::Downloading => {}
            UpdateStatus::Idle | UpdateStatus::UpToDate | UpdateStatus::Error => {
                self.check(false, cx)
            }
        }
    }

    fn check(&mut self, silent: bool, cx: &mut Context<Self>) {
        let Some(service) = self.service.clone() else {
            return;
        };
        self.status = UpdateStatus::Checking;
        self.error = None;
        cx.notify();

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
            });
        })
        .detach();
    }

    fn install(&mut self, cx: &mut Context<Self>) {
        let (Some(service), Some(update)) = (self.service.clone(), self.update.clone()) else {
            return;
        };
        self.status = UpdateStatus::Downloading;
        self.error = None;
        cx.notify();

        let runtime = cx.global::<AppState>().background_runtime.clone();
        let task = runtime.spawn(async move { service.download_and_install(&update).await });
        cx.spawn(async move |this, cx| {
            let result = task
                .await
                .map_err(|error| format!("Update installation task failed: {error}"))
                .and_then(|result| result.map_err(|error| format!("{error:#}")));
            let _ = this.update(cx, |this, cx| match result {
                Ok(()) => cx.quit(),
                Err(error) => {
                    this.status = UpdateStatus::Error;
                    this.error = Some(error);
                    cx.notify();
                }
            });
        })
        .detach();
    }
}

impl Render for Updater {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.ui_palette();
        let button_label = match self.status {
            UpdateStatus::Idle | UpdateStatus::Error => "Check for updates".to_owned(),
            UpdateStatus::Checking => "Checking…".into(),
            UpdateStatus::Available => self
                .version()
                .map(|version| format!("Install v{version}"))
                .unwrap_or_else(|| "Install update".into()),
            UpdateStatus::Downloading => "Updating…".into(),
            UpdateStatus::UpToDate => "Check again".into(),
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
            UpdateStatus::Downloading => "Downloading and installing update…".to_owned(),
            UpdateStatus::UpToDate => {
                format!("LocalCraft v{} is up to date.", env!("CARGO_PKG_VERSION"))
            }
            UpdateStatus::Error => "Could not complete update check.".to_owned(),
        };
        let busy = matches!(
            self.status,
            UpdateStatus::Checking | UpdateStatus::Downloading
        );
        let available = self.status == UpdateStatus::Available;
        let button = Button::new("check-app-updates")
            .label(button_label)
            .rounded(px(16.0))
            .h(px(32.0))
            .disabled(busy)
            .on_click(cx.listener(|this, _, _, cx| this.activate(cx)));
        let button = if available { button.primary() } else { button };

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
                    .child(button),
            )
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
