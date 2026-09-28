use gpui::{Context, IntoElement, Render, Window, div, prelude::*, px, svg};
use gpui_kit::component::scroll::ScrollableElement;
use services::{AppState, updater::UpdaterService};
use ui::theme::Palette;

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
        let palette = cx.global::<AppState>().theme_manager.palette();
        let label = match self.status {
            UpdateStatus::Idle | UpdateStatus::Error => "Check for updates".to_owned(),
            UpdateStatus::Checking => "Checking…".into(),
            UpdateStatus::Available => self
                .version()
                .map(|version| format!("Install update v{version}"))
                .unwrap_or_else(|| "Install update".into()),
            UpdateStatus::Downloading => "Downloading update…".into(),
            UpdateStatus::UpToDate => "LocalCraft is up to date".into(),
        };
        let busy = matches!(
            self.status,
            UpdateStatus::Checking | UpdateStatus::Downloading
        );

        div()
            .border_t_1()
            .border_color(palette.border)
            .pt_4()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div().flex().flex_col().gap_1().child("Updates").child(
                    div()
                        .text_sm()
                        .text_color(palette.muted)
                        .child("Keep LocalCraft up to date."),
                ),
            )
            .child(
                div()
                    .id("check-app-updates")
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .border_1()
                    .border_color(palette.border)
                    .when(!busy, |button| button.cursor_pointer())
                    .when(!busy, |button| {
                        button.hover(|style| style.bg(palette.surface))
                    })
                    .when(!busy, |button| {
                        button.on_click(cx.listener(|this, _, _, cx| this.activate(cx)))
                    })
                    .child(
                        svg()
                            .path(if busy {
                                "icons/loader.svg"
                            } else if self.status == UpdateStatus::Available {
                                "icons/download.svg"
                            } else {
                                "icons/refresh-ccw.svg"
                            })
                            .size_4()
                            .text_color(palette.muted),
                    )
                    .child(div().text_color(palette.text).child(label)),
            )
            .when_some(self.error.as_ref(), |element, error| {
                element.child(
                    div()
                        .text_xs()
                        .text_color(palette.accent)
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
                            .max_h(px(72.))
                            .overflow_y_scrollbar()
                            .text_xs()
                            .text_color(palette.muted)
                            .child(notes.clone()),
                    )
                },
            )
    }
}

pub fn titlebar_control(
    workspace: &crate::workspace::Workspace,
    palette: &Palette,
    cx: &Context<crate::workspace::Workspace>,
) -> impl IntoElement {
    let status = workspace.updater.read(cx).status();
    let show = matches!(
        status,
        UpdateStatus::Available
            | UpdateStatus::Checking
            | UpdateStatus::Downloading
            | UpdateStatus::Error
    );
    div().id("titlebar-update").when(show, |control| {
        control
            .w(px(36.))
            .h(px(32.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .cursor_pointer()
            .hover(|style| style.bg(palette.surface))
            .on_click(cx.listener(|this, _, _, cx| {
                this.updater.update(cx, |updater, cx| updater.activate(cx));
            }))
            .child(
                svg()
                    .path(match status {
                        UpdateStatus::Available => "icons/download.svg",
                        UpdateStatus::Checking | UpdateStatus::Downloading => "icons/loader.svg",
                        UpdateStatus::Error => "icons/refresh-ccw.svg",
                        UpdateStatus::Idle | UpdateStatus::UpToDate => "icons/refresh-ccw.svg",
                    })
                    .size_4()
                    .text_color(palette.text),
            )
    })
}
