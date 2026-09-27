use gpui::AppContext;
use gpui::{Context, Entity, EventEmitter, FocusHandle, Subscription};
use services::{
    AppState,
    instance::{ServerInstance, configuration::CreateServer},
    java::JavaProgress,
    software::{DownloadProgress, DownloadStage, SoftwareService, catalog::BuildOption},
};
use ui::components::{
    input::Input,
    select::{Select, Selected},
};

pub const SOFTWARE: [&str; 5] = ["Paper", "Purpur", "Fabric", "Forge", "Vanilla"];

pub enum CreationEvent {
    Created(ServerInstance),
    Close,
}
enum Update {
    Progress(String),
    Done(Result<ServerInstance, String>),
}

pub struct CreateServerModal {
    pub name: Entity<Input>,
    pub ram: Entity<Input>,
    pub port: Entity<Input>,
    pub checksum: Entity<Input>,
    pub software: Entity<Select>,
    pub version: Entity<Select>,
    pub build: Entity<Select>,
    pub java: Entity<Select>,
    pub focus: FocusHandle,
    pub accepted_eula: bool,
    pub busy: bool,
    pub loading: bool,
    pub status: Option<String>,
    pub error: Option<String>,
    pub builds: Vec<BuildOption>,
    pub minimum_java: u8,
    generation: u64,
    _subscriptions: Vec<Subscription>,
}
impl EventEmitter<CreationEvent> for CreateServerModal {}

impl CreateServerModal {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let name = cx.new(|cx| Input::new("", "My Minecraft server", cx));
        let ram = cx.new(|cx| Input::new("2048", "MiB", cx));
        let port = cx.new(|cx| Input::new("25565", "Port", cx));
        let checksum = cx.new(|cx| Input::new("", "Trusted SHA256 (64 characters)", cx));
        let software = cx.new(|cx| Select::new("Choose software", cx));
        let version = cx.new(|cx| Select::new("Choose a version", cx));
        let build = cx.new(|cx| Select::new("Choose a build", cx));
        let java = cx.new(|cx| Select::new("Java runtime", cx));
        software.update(cx, |select, cx| {
            select.set_options(SOFTWARE.iter().map(|s| (*s).into()).collect(), cx);
            select.selected = Some(0);
        });
        let subscriptions = vec![
            cx.subscribe(&software, |this, _, _: &Selected, cx| {
                this.fetch_versions(cx)
            }),
            cx.subscribe(&version, |this, _, _: &Selected, cx| this.fetch_builds(cx)),
            cx.subscribe(&build, |this, _, _: &Selected, cx| {
                this.checksum.update(cx, |input, cx| input.clear(cx));
                cx.notify();
            }),
        ];
        let mut modal = Self {
            name,
            ram,
            port,
            checksum,
            software,
            version,
            build,
            java,
            focus: cx.focus_handle(),
            accepted_eula: false,
            busy: false,
            loading: false,
            status: None,
            error: None,
            builds: Vec::new(),
            minimum_java: 8,
            generation: 0,
            _subscriptions: subscriptions,
        };
        modal.fetch_versions(cx);
        modal
    }
    pub fn fetch_versions(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;
        self.error = None;
        self.loading = true;
        self.builds.clear();
        self.version.update(cx, |s, cx| {
            s.set_options(Vec::new(), cx);
            s.enabled = false;
        });
        self.build.update(cx, |s, cx| {
            s.set_options(Vec::new(), cx);
            s.enabled = false;
        });
        self.java.update(cx, |s, cx| {
            s.set_options(Vec::new(), cx);
            s.enabled = false;
        });
        let software = self.software.read(cx).value().unwrap_or("Paper").to_owned();
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        cx.global::<AppState>()
            .background_runtime
            .spawn(async move {
                let result = SoftwareService::new()
                    .versions(&software)
                    .await
                    .map_err(|e| format!("{e:#}"));
                let _ = tx.send(result).await;
            });
        cx.spawn(async move |this, cx| {
            if let Some(result) = rx.recv().await {
                let _ = this.update(cx, |this, cx| {
                    if this.generation != generation {
                        return;
                    }
                    this.loading = false;
                    match result {
                        Ok(options) => this.version.update(cx, |s, cx| {
                            s.set_options(options.into_iter().map(Into::into).collect(), cx);
                            s.enabled = true;
                        }),
                        Err(error) => this.error = Some(error),
                    }
                    cx.notify();
                });
            }
        })
        .detach();
        cx.notify();
    }
    pub fn fetch_builds(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let Some(version) = self.version.read(cx).value().map(str::to_owned) else {
            return;
        };
        let software = self.software.read(cx).value().unwrap_or("Paper").to_owned();
        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;
        self.loading = true;
        self.error = None;
        self.checksum.update(cx, |input, cx| input.clear(cx));
        self.builds.clear();
        self.build.update(cx, |s, cx| {
            s.set_options(Vec::new(), cx);
            s.enabled = false;
        });
        self.java.update(cx, |s, cx| {
            s.set_options(Vec::new(), cx);
            s.enabled = false;
        });
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        cx.global::<AppState>()
            .background_runtime
            .spawn(async move {
                let result = SoftwareService::new()
                    .builds(&software, &version)
                    .await
                    .map_err(|e| format!("{e:#}"));
                let _ = tx.send(result).await;
            });
        cx.spawn(async move |this, cx| {
            if let Some(result) = rx.recv().await {
                let _ = this.update(cx, |this, cx| {
                    if this.generation != generation {
                        return;
                    }
                    this.loading = false;
                    match result {
                        Ok((java, builds)) => {
                            this.minimum_java = java;
                            this.build.update(cx, |select, cx| {
                                select.set_options(
                                    builds.iter().map(|b| b.label.clone().into()).collect(),
                                    cx,
                                );
                                select.selected = Some(0);
                                select.enabled = true;
                            });
                            this.java.update(cx, |select, cx| {
                                let mut versions = vec![java, 8, 17, 21, 25];
                                versions.retain(|v| *v >= java);
                                versions.sort_unstable();
                                versions.dedup();
                                select.set_options(
                                    versions.iter().map(|v| v.to_string().into()).collect(),
                                    cx,
                                );
                                select.selected = Some(0);
                                select.enabled = true;
                            });
                            this.builds = builds;
                        }
                        Err(error) => this.error = Some(error),
                    }
                    cx.notify();
                });
            }
        })
        .detach();
        cx.notify();
    }
    pub fn needs_checksum(&self, cx: &gpui::App) -> bool {
        self.build
            .read(cx)
            .selected
            .and_then(|i| self.builds.get(i))
            .is_some_and(|b| b.download.sha256.is_none())
    }
    fn specification(&self, cx: &gpui::App) -> Result<CreateServer, String> {
        let get = |select: &Entity<Select>| {
            select
                .read(cx)
                .value()
                .map(str::to_owned)
                .ok_or("Complete all selections".to_owned())
        };
        let chosen = self
            .build
            .read(cx)
            .selected
            .and_then(|i| self.builds.get(i))
            .ok_or("Choose a build")?;
        let specification = CreateServer {
            name: self.name.read(cx).value().trim().to_owned(),
            version: get(&self.version)?,
            software: get(&self.software)?,
            build: chosen.label.clone(),
            ram: self
                .ram
                .read(cx)
                .value()
                .parse()
                .map_err(|_| "RAM must be a number in MiB")?,
            port: self
                .port
                .read(cx)
                .value()
                .parse()
                .map_err(|_| "Port must be between 1 and 65535")?,
            java: get(&self.java)?
                .parse()
                .map_err(|_| "Choose a Java runtime")?,
            accepted_eula: self.accepted_eula,
            checksum: Some(self.checksum.read(cx).value().trim().to_owned())
                .filter(|s| !s.is_empty()),
            download: chosen.download.clone(),
        };
        specification.validate().map_err(|e| e.to_string())?;
        Ok(specification)
    }
    pub fn create(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.loading {
            return;
        }
        let specification = match self.specification(cx) {
            Ok(specification) => specification,
            Err(error) => {
                self.error = Some(error);
                cx.notify();
                return;
            }
        };
        if cx
            .global::<AppState>()
            .instance_service
            .servers()
            .iter()
            .any(|s| s.port == specification.port)
        {
            self.error =
                Some("This port is assigned to another server. Choose a different port.".into());
            cx.notify();
            return;
        }
        self.busy = true;
        for input in [&self.name, &self.ram, &self.port, &self.checksum] {
            input.update(cx, |input, cx| {
                input.enabled = false;
                cx.notify();
            });
        }
        self.error = None;
        self.status = Some("Preparing server…".into());
        for select in [&self.software, &self.version, &self.build, &self.java] {
            select.update(cx, |s, cx| {
                s.enabled = false;
                cx.notify();
            });
        }
        let java = cx.global::<AppState>().java_service.clone();
        let (tx, mut rx) = tokio::sync::mpsc::channel(8);
        cx.global::<AppState>().background_runtime.spawn(async move {
            let (java_tx, mut java_rx) = tokio::sync::watch::channel(JavaProgress::new(specification.java));
            let (download_tx, mut download_rx) = tokio::sync::watch::channel(DownloadProgress {
                instance_id: String::new(), stage: DownloadStage::Resolving, downloaded: 0, total: None,
            });
            let operation = specification.create(java, &java_tx, &download_tx);
            tokio::pin!(operation);
            loop {
                tokio::select! {
                    result = &mut operation => {
                        let _ = tx.send(Update::Done(result.map_err(|e| format!("{e:#}")))).await;
                        break;
                    }
                    Ok(()) = java_rx.changed() => {
                        let p = java_rx.borrow().clone();
                        let _ = tx.try_send(Update::Progress(format!("Java {} · {:?}", p.version, p.stage)));
                    }
                    Ok(()) = download_rx.changed() => {
                        let p = download_rx.borrow().clone();
                        let suffix = p.total.filter(|v| *v > 0).map(|total| format!(" · {}%", p.downloaded.saturating_mul(100) / total)).unwrap_or_default();
                        let _ = tx.try_send(Update::Progress(format!("Server · {:?}{suffix}", p.stage)));
                    }
                }
            }
        });
        cx.spawn(async move |this, cx| {
            while let Some(update) = rx.recv().await {
                if this
                    .update(cx, |this, cx| {
                        match update {
                            Update::Progress(status) => this.status = Some(status),
                            Update::Done(result) => {
                                this.busy = false;
                                this.status = None;
                                for select in
                                    [&this.software, &this.version, &this.build, &this.java]
                                {
                                    select.update(cx, |s, cx| {
                                        s.enabled = true;
                                        cx.notify();
                                    });
                                }
                                match result {
                                    Ok(server) => cx.emit(CreationEvent::Created(server)),
                                    Err(error) => this.error = Some(error),
                                }
                            }
                        }
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        cx.notify();
    }
    pub fn close(&self, cx: &mut Context<Self>) {
        if !self.busy {
            cx.emit(CreationEvent::Close);
        }
    }
}
