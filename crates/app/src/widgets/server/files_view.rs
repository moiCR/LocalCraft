use smol::Timer;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use gpui::{
    Anchor, Animation, AnimationExt, AppContext, Context, Entity, ExternalPaths, IntoElement,
    MouseButton, PathPromptOptions, Pixels, Point, Render, Subscription, Window, anchored, div,
    ease_out_quint, point, prelude::*, px, svg,
};
use services::{
    AppState,
    instance::{
        ServerInstance,
        files::{self, FileEntry},
    },
};
use ui::{
    components::{button::button, input::Input},
    theme::Palette,
};

use super::file_editor::{EditorEvent, FileEditor};

const ROW_HEIGHT: f32 = 42.;

enum Dialog {
    CreateFolder(Entity<Input>),
    Rename {
        entry: FileEntry,
        input: Entity<Input>,
    },
    Delete(FileEntry),
}

enum Action {
    CreateFolder { parent: PathBuf, name: String },
    Rename { path: PathBuf, name: String },
    Delete(PathBuf),
}

pub struct FilesView {
    server: ServerInstance,
    directory: PathBuf,
    entries: Vec<FileEntry>,
    loading: bool,
    busy: bool,
    error: Option<String>,
    generation: u64,
    dialog: Option<Dialog>,
    editor: Option<Entity<FileEditor>>,
    editor_subscription: Option<Subscription>,
    menu: Option<(PathBuf, Point<Pixels>)>,
    closing: bool,
    close_generation: u64,
}

impl FilesView {
    pub fn new(server: ServerInstance, cx: &mut Context<Self>) -> Self {
        let mut view = Self {
            server,
            directory: PathBuf::new(),
            entries: Vec::new(),
            loading: false,
            busy: false,
            error: None,
            generation: 0,
            dialog: None,
            editor: None,
            editor_subscription: None,
            menu: None,
            closing: false,
            close_generation: 0,
        };
        view.refresh(cx);
        view
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;
        let directory = self.directory.clone();
        let server = self.server.clone();
        self.loading = true;
        self.error = None;
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        cx.global::<AppState>()
            .background_runtime
            .spawn(async move {
                let result = files::list(&server, &directory)
                    .await
                    .map_err(|error| format!("{error:#}"));
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
                        Ok(entries) => this.entries = entries,
                        Err(error) => this.error = Some(error),
                    }
                    cx.notify();
                });
            }
        })
        .detach();
        cx.notify();
    }

    fn navigate(&mut self, directory: PathBuf, cx: &mut Context<Self>) {
        self.directory = directory;
        self.menu = None;
        self.refresh(cx);
    }

    fn close_overlay(&mut self, cx: &mut Context<Self>) {
        if self.closing || (self.dialog.is_none() && self.editor.is_none()) {
            return;
        }
        self.closing = true;
        self.close_generation = self.close_generation.wrapping_add(1);
        let generation = self.close_generation;
        cx.notify();
        cx.spawn(async move |this, cx| {
            Timer::after(Duration::from_millis(260)).await;
            let _ = this.update(cx, |this, cx| {
                if this.close_generation != generation {
                    return;
                }
                this.dialog = None;
                this.editor = None;
                this.editor_subscription = None;
                this.closing = false;
                cx.notify();
            });
        })
        .detach();
    }

    fn start_overlay(&mut self) {
        self.close_generation = self.close_generation.wrapping_add(1);
        self.closing = false;
    }

    fn open_entry(
        &mut self,
        entry: FileEntry,
        window_handle: gpui::AnyWindowHandle,
        cx: &mut Context<Self>,
    ) {
        if entry.is_directory {
            self.navigate(entry.path, cx);
            return;
        }
        if entry.is_symlink || self.busy {
            return;
        }
        self.busy = true;
        self.error = None;
        let server = self.server.clone();
        let path = entry.path;
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        cx.global::<AppState>()
            .background_runtime
            .spawn(async move {
                let result = files::read_text(&server, &path)
                    .await
                    .map(|contents| (path, contents))
                    .map_err(|error| format!("{error:#}"));
                let _ = tx.send(result).await;
            });
        cx.spawn(async move |this, cx| {
            if let Some(result) = rx.recv().await {
                let _ = cx.update_window(window_handle, |_, window, app| {
                    let _ = this.update(app, |this, cx| {
                        this.busy = false;
                        match result {
                            Ok((path, contents)) => {
                                this.start_overlay();
                                let editor = cx.new(|cx| {
                                    FileEditor::new(
                                        window,
                                        this.server.clone(),
                                        path,
                                        &contents,
                                        cx,
                                    )
                                });
                                let subscription =
                                    cx.subscribe(&editor, |this, _, event, cx| match event {
                                        EditorEvent::Saved => {
                                            this.refresh(cx);
                                            this.close_overlay(cx);
                                        }
                                        EditorEvent::Close => this.close_overlay(cx),
                                    });
                                this.editor_subscription = Some(subscription);
                                this.editor = Some(editor);
                            }
                            Err(error) => this.error = Some(error),
                        }
                        cx.notify();
                    });
                });
            }
        })
        .detach();
        cx.notify();
    }

    fn create_folder_dialog(&mut self, cx: &mut Context<Self>) {
        let input = cx.new(|cx| Input::new("", "Folder name", cx));
        self.start_overlay();
        self.dialog = Some(Dialog::CreateFolder(input));
        self.error = None;
        cx.notify();
    }

    fn rename_dialog(&mut self, entry: FileEntry, cx: &mut Context<Self>) {
        let input = cx.new(|cx| Input::new(&entry.name, "New name", cx));
        self.start_overlay();
        self.dialog = Some(Dialog::Rename { entry, input });
        self.error = None;
        cx.notify();
    }

    fn confirm_dialog(&mut self, cx: &mut Context<Self>) {
        let action = match self.dialog.as_ref() {
            Some(Dialog::CreateFolder(input)) => Some(Action::CreateFolder {
                parent: self.directory.clone(),
                name: input.read(cx).value().trim().to_owned(),
            }),
            Some(Dialog::Rename { entry, input }) => Some(Action::Rename {
                path: entry.path.clone(),
                name: input.read(cx).value().trim().to_owned(),
            }),
            Some(Dialog::Delete(entry)) => Some(Action::Delete(entry.path.clone())),
            None => None,
        };
        let Some(action) = action else {
            return;
        };
        if matches!(&action, Action::CreateFolder { name, .. } | Action::Rename { name, .. } if name.is_empty())
        {
            self.error = Some("Enter a name first".into());
            cx.notify();
            return;
        }
        self.perform(action, cx);
    }

    fn perform(&mut self, action: Action, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.error = None;
        let server = self.server.clone();
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        cx.global::<AppState>()
            .background_runtime
            .spawn(async move {
                let result = match action {
                    Action::CreateFolder { parent, name } => {
                        files::create_directory(&server, &parent, &name).await
                    }
                    Action::Rename { path, name } => files::rename(&server, &path, &name).await,
                    Action::Delete(path) => files::delete(&server, &path).await,
                }
                .map_err(|error| format!("{error:#}"));
                let _ = tx.send(result).await;
            });
        cx.spawn(async move |this, cx| {
            if let Some(result) = rx.recv().await {
                let _ = this.update(cx, |this, cx| {
                    this.busy = false;
                    match result {
                        Ok(()) => {
                            this.refresh(cx);
                            this.close_overlay(cx);
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

    fn select_upload(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some("Upload files".into()),
        });
        cx.spawn(async move |this, cx| match receiver.await {
            Ok(Ok(Some(paths))) if !paths.is_empty() => {
                let _ = this.update(cx, |this, cx| this.upload_paths(paths, cx));
            }
            Ok(Err(error)) => {
                let _ = this.update(cx, |this, cx| {
                    this.error = Some(format!("Could not open file picker: {error:#}"));
                    cx.notify();
                });
            }
            Ok(Ok(None)) | Ok(Ok(Some(_))) | Err(_) => {}
        })
        .detach();
    }

    fn upload_paths(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        if self.busy || paths.is_empty() {
            return;
        }
        self.busy = true;
        self.error = None;
        let server = self.server.clone();
        let directory = self.directory.clone();
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        cx.global::<AppState>()
            .background_runtime
            .spawn(async move {
                let mut result = Ok(());
                for path in paths {
                    if let Err(error) = files::upload_file(&server, &directory, &path).await {
                        result = Err(format!("Could not upload {}: {error:#}", path.display()));
                        break;
                    }
                }
                let _ = tx.send(result).await;
            });
        cx.spawn(async move |this, cx| {
            if let Some(result) = rx.recv().await {
                let _ = this.update(cx, |this, cx| {
                    this.busy = false;
                    this.refresh(cx);
                    if let Err(error) = result {
                        this.error = Some(error);
                    }
                    cx.notify();
                });
            }
        })
        .detach();
        cx.notify();
    }

    fn render_dialog(
        &self,
        palette: &Palette,
        viewport: gpui::Size<gpui::Pixels>,
        cx: &Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let dialog = self.dialog.as_ref()?;
        let (title, body, input, danger) = match dialog {
            Dialog::CreateFolder(input) => (
                "Create folder",
                "Choose a name for the new folder.".to_owned(),
                Some(input.clone()),
                false,
            ),
            Dialog::Rename { entry, input } => {
                ("Rename", entry.name.clone(), Some(input.clone()), false)
            }
            Dialog::Delete(entry) => (
                "Delete item?",
                if entry.is_directory {
                    "This permanently deletes the folder and all its contents.".to_owned()
                } else {
                    "This permanently deletes the file.".to_owned()
                },
                None,
                true,
            ),
        };
        let error = self.error.clone();
        let closing = self.closing;
        Some(
            anchored()
                .position(point(px(0.), px(0.)))
                .child(
                    div()
                        .w(viewport.width)
                        .h(viewport.height)
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(gpui::rgba(0x00000099))
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.close_overlay(cx);
                            }),
                        )
                        .child(
                            div()
                                .w(px(420.))
                                .max_w_full()
                                .p_5()
                                .flex()
                                .flex_col()
                                .gap_4()
                                .rounded_lg()
                                .bg(palette.surface)
                                .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| {
                                    cx.stop_propagation()
                                })
                                .child(div().text_lg().child(title))
                                .child(div().text_sm().text_color(palette.muted).child(body))
                                .when_some(input, |element, input| element.child(input))
                                .when_some(error, |element, error| {
                                    element.child(
                                        div()
                                            .text_xs()
                                            .text_color(gpui::rgb(0xe87878))
                                            .child(error),
                                    )
                                })
                                .child(
                                    div()
                                        .flex()
                                        .justify_end()
                                        .gap_2()
                                        .child(
                                            button(
                                                "cancel-file-dialog",
                                                "Cancel",
                                                palette,
                                                false,
                                                !self.busy,
                                            )
                                            .on_click(
                                                cx.listener(|this, _, _, cx| {
                                                    this.close_overlay(cx);
                                                }),
                                            ),
                                        )
                                        .child(
                                            button(
                                                "confirm-file-dialog",
                                                if self.busy {
                                                    "Working…"
                                                } else if danger {
                                                    "Delete"
                                                } else {
                                                    "Confirm"
                                                },
                                                palette,
                                                danger,
                                                !self.busy,
                                            )
                                            .on_click(
                                                cx.listener(|this, _, _, cx| {
                                                    this.confirm_dialog(cx)
                                                }),
                                            ),
                                        ),
                                )
                                .with_animation(
                                    ("files-dialog-panel", usize::from(closing)),
                                    Animation::new(Duration::from_millis(260))
                                        .with_easing(ease_out_quint()),
                                    move |panel, progress| {
                                        panel
                                            .relative()
                                            .top(px(10. * (1. - progress)))
                                            .opacity(if closing { 1. - progress } else { progress })
                                    },
                                ),
                        )
                        .with_animation(
                            ("files-dialog-backdrop", usize::from(closing)),
                            Animation::new(Duration::from_millis(260)),
                            move |backdrop, progress| {
                                backdrop.opacity(if closing { 1. - progress } else { progress })
                            },
                        )
                        .into_any_element(),
                )
                .into_any_element(),
        )
    }
}

impl Render for FilesView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.global::<AppState>().theme_manager.palette();
        let editor = self.editor.clone();
        let closing = self.closing;
        let entity = cx.entity();
        let rows = self.entries.len();
        let row_entries = self.entries.clone();
        let current = self.directory.clone();
        let mut crumbs = vec![("/".to_owned(), PathBuf::new())];
        let mut path = PathBuf::new();
        for component in self.directory.components() {
            if let Component::Normal(name) = component {
                path.push(name);
                crumbs.push((format!("{}/", name.to_string_lossy()), path.clone()));
            }
        }
        let crumb_count = crumbs.len();
        let viewport = window.viewport_size();
        let dialog = self.render_dialog(palette, viewport, cx);
        let content =
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .min_w_0()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            button(
                                "file-parent",
                                "",
                                palette,
                                false,
                                !current.as_os_str().is_empty(),
                            )
                            .w_9()
                            .h_9()
                            .p_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                svg()
                                    .path("icons/chevron-left.svg")
                                    .size_4()
                                    .text_color(palette.text),
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                let parent =
                                    this.directory.parent().unwrap_or_else(|| Path::new(""));
                                this.navigate(parent.to_path_buf(), cx);
                            })),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_1()
                                .min_w_0()
                                .items_center()
                                .gap_0()
                                .children(crumbs.into_iter().enumerate().map(
                                    |(index, (label, crumb_path))| {
                                        div()
                                            .id(("file-crumb", index))
                                            .flex()
                                            .items_center()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .id(("file-crumb-link", index))
                                                    .text_sm()
                                                    .text_color(if index + 1 == crumb_count {
                                                        palette.text
                                                    } else {
                                                        palette.muted
                                                    })
                                                    .cursor_pointer()
                                                    .hover(|s| s.text_color(palette.text))
                                                    .child(label)
                                                    .on_click(cx.listener(
                                                        move |this, _, _, cx| {
                                                            this.navigate(crumb_path.clone(), cx)
                                                        },
                                                    )),
                                            )
                                    },
                                )),
                        )
                        .child(
                            button("refresh-files", "", palette, false, !self.busy)
                                .w_9()
                                .h_9()
                                .p_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    svg()
                                        .path("icons/refresh-ccw.svg")
                                        .size_4()
                                        .text_color(palette.text),
                                )
                                .on_click(cx.listener(|this, _, _, cx| this.refresh(cx))),
                        )
                        .child(
                            button("new-folder", "", palette, true, !self.busy)
                                .w_9()
                                .h_9()
                                .p_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    svg()
                                        .path("icons/folder-plus.svg")
                                        .size_4()
                                        .text_color(palette.background),
                                )
                                .on_click(
                                    cx.listener(|this, _, _, cx| this.create_folder_dialog(cx)),
                                ),
                        )
                        .child(
                            button("upload-files", "", palette, false, !self.busy)
                                .w_9()
                                .h_9()
                                .p_0()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    svg()
                                        .path("icons/upload.svg")
                                        .size_4()
                                        .text_color(palette.text),
                                )
                                .on_click(cx.listener(|this, _, _, cx| this.select_upload(cx))),
                        ),
                )
                .child(
                    div()
                        .grid()
                        .grid_cols(6)
                        .items_center()
                        .gap_3()
                        .px_3()
                        .py_2()
                        .text_xs()
                        .text_color(palette.muted)
                        .border_b_1()
                        .border_color(palette.border)
                        .child(div().col_span(2).child("Name"))
                        .child("Type")
                        .child("Size")
                        .child("Modified")
                        .child(div()),
                )
                .when_some(self.error.clone(), |el, error| {
                    el.child(div().text_xs().text_color(gpui::rgb(0xe87878)).child(error))
                })
                .when(self.loading, |el| {
                    el.child(
                        div()
                            .p_4()
                            .text_sm()
                            .text_color(palette.muted)
                            .child("Loading…"),
                    )
                })
                .when(!self.loading, |el| {
                    el.child(
                        div()
                            .id("files-drop-target")
                            .relative()
                            .flex_1()
                            .min_h_0()
                            .min_w_0()
                            .on_drop::<ExternalPaths>(cx.listener(
                                |this, paths: &ExternalPaths, _, cx| {
                                    this.upload_paths(paths.paths().to_vec(), cx);
                                },
                            ))
                            .when(rows == 0, |el| {
                                el.child(
                                    div()
                                        .size_full()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .text_sm()
                                        .text_color(palette.muted)
                                        .child("This folder is empty."),
                                )
                            })
                            .when(rows > 0, |el| {
                                el.child(
                                    div()
                                        .id("files-scroll")
                                        .relative()
                                        .size_full()
                                        .min_h_0()
                                        .min_w_0()
                                        .overflow_y_scroll()
                                        .children(row_entries.into_iter().enumerate().map(
                                            |(index, entry)| {
                                                let open_entity = entity.clone();
                                                let open = entry.clone();
                                                let kind = if entry.is_symlink {
                                                    "Link"
                                                } else if entry.is_directory {
                                                    "Folder"
                                                } else {
                                                    "File"
                                                };
                                                div()
                                .id(("instance-file", index))
                                .h(px(ROW_HEIGHT))
                                .grid()
                                .grid_cols(6)
                                .items_center()
                                .gap_3()
                                .px_3()
                                .border_b_1()
                                .border_color(palette.border)
                                .child(
                                    div()
                                        .col_span(2)
                                        .h_full()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .min_w_0()
                                        .child(
                                            svg()
                                                .path(if entry.is_directory {
                                                    "icons/folder.svg"
                                                } else {
                                                    "icons/file.svg"
                                                })
                                                .size_4()
                                                .text_color(palette.muted),
                                        )
                                        .child(
                                            div()
                                                .id(("file-name", index))
                                                .flex_1()
                                                .min_w_0()
                                                .truncate()
                                                .text_sm()
                                                .cursor_pointer()
                                                .hover(|s| s.text_color(palette.text))
                                                .child(entry.name.clone())
                                                .on_click(move |_, window, cx| {
                                                    let window_handle = window.window_handle();
                                                    open_entity.update(cx, |this, cx| {
                                                        this.open_entry(
                                                            open.clone(),
                                                            window_handle,
                                                            cx,
                                                        )
                                                    });
                                                }),
                                        ),
                                )
                                .child(
                                    div()
                                        .h_full()
                                        .flex()
                                        .items_center()
                                        .text_xs()
                                        .text_color(palette.muted)
                                        .child(kind),
                                )
                                .child(
                                    div()
                                        .h_full()
                                        .flex()
                                        .items_center()
                                        .text_xs()
                                        .text_color(palette.muted)
                                        .child(if entry.is_directory {
                                            "—".to_owned()
                                        } else {
                                            format_size(entry.size)
                                        }),
                                )
                                .child(
                                    div()
                                        .h_full()
                                        .flex()
                                        .items_center()
                                        .text_xs()
                                        .text_color(palette.muted)
                                        .child(format_modified(entry.modified)),
                                )
                                .child(
                                    div()
                                        .relative()
                                        .h_full()
                                        .flex()
                                        .items_center()
                                        .justify_end()
                                        .child(
                                            div()
                                                .id(("file-actions", index))
                                                .size_8()
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .rounded_md()
                                                .cursor_pointer()
                                                .text_color(palette.muted)
                                                .hover(|style| {
                                                    style
                                                        .bg(palette.surface)
                                                        .text_color(palette.text)
                                                })
                                                .child(
                                                    svg()
                                                        .path("icons/ellipsis.svg")
                                                        .size_4()
                                                        .text_color(palette.text),
                                                )
                                                .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                                    cx.stop_propagation()
                                                })
                                                .on_click({
                                                    let entry_path = entry.path.clone();
                                                    let menu_entity = entity.clone();
                                                    move |event, _, cx| {
                                                        cx.stop_propagation();
                                                        let position = event.position();
                                                        menu_entity.update(cx, |this, cx| {
                                                            this.menu = if this
                                                                .menu
                                                                .as_ref()
                                                                .is_some_and(|(path, _)| {
                                                                    path == &entry_path
                                                                }) {
                                                                None
                                                            } else {
                                                                Some((entry_path.clone(), position))
                                                            };
                                                            cx.notify();
                                                        });
                                                    }
                                                }),
                                        ),
                                )
                                            },
                                        )),
                                )
                            })
                            .child(
                                div()
                                    .absolute()
                                    .inset_0()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .justify_center()
                                    .gap_3()
                                    .bg(gpui::rgba(0x101010b8))
                                    .opacity(0.)
                                    .drag_over::<ExternalPaths>(|style, _, _, _| style.opacity(1.))
                                    .child(
                                        svg()
                                            .path("icons/upload.svg")
                                            .size_10()
                                            .text_color(palette.text),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(palette.text)
                                            .child("Drop files to upload"),
                                    ),
                            ),
                    )
                });
        let context_menu = self.menu.as_ref().and_then(|(path, position)| {
            let entry = self
                .entries
                .iter()
                .find(|entry| &entry.path == path)?
                .clone();
            let rename_entity = entity.clone();
            let delete_entity = entity.clone();
            let rename = entry.clone();
            let delete = entry;
            Some(
                anchored()
                    .position(point(position.x - px(156.), position.y + px(12.)))
                    .anchor(Anchor::TopLeft)
                    .snap_to_window_with_margin(gpui::Edges::all(px(8.)))
                    .child(
                        div()
                            .w(px(156.))
                            .p_1()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .rounded_md()
                            .border_1()
                            .border_color(palette.border)
                            .bg(palette.background)
                            .occlude()
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .child(
                                menu_item("Rename", "icons/text-cursor.svg", palette).on_click(
                                    move |_, _, cx| {
                                        cx.stop_propagation();
                                        rename_entity.update(cx, |this, cx| {
                                            this.menu = None;
                                            this.rename_dialog(rename.clone(), cx);
                                        });
                                    },
                                ),
                            )
                            .child(menu_item("Delete", "icons/trash.svg", palette).on_click(
                                move |_, _, cx| {
                                    cx.stop_propagation();
                                    delete_entity.update(cx, |this, cx| {
                                        this.menu = None;
                                        this.dialog = Some(Dialog::Delete(delete.clone()));
                                        this.error = None;
                                        cx.notify();
                                    });
                                },
                            )),
                    )
                    .into_any_element(),
            )
        });
        div()
            .relative()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .child(content)
            .children(dialog)
            .children(context_menu)
            .when_some(editor, move |view, editor| {
                view.child(
                    anchored()
                        .position(point(px(0.), px(0.)))
                        .child(
                            div()
                                .w(viewport.width)
                                .h(viewport.height)
                                .flex()
                                .items_center()
                                .justify_center()
                                .bg(gpui::rgba(0x00000099))
                                .occlude()
                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                                .child(
                                    div()
                                        .w_full()
                                        .h_full()
                                        .max_w(px(1100.))
                                        .max_h(px(760.))
                                        .p_5()
                                        .flex()
                                        .bg(palette.background)
                                        .border_1()
                                        .border_color(palette.border)
                                        .rounded_lg()
                                        .child(editor)
                                        .with_animation(
                                            ("file-editor-panel", usize::from(closing)),
                                            Animation::new(Duration::from_millis(260))
                                                .with_easing(ease_out_quint()),
                                            move |panel, progress| {
                                                panel
                                                    .relative()
                                                    .top(px(10. * (1. - progress)))
                                                    .opacity(if closing {
                                                        1. - progress
                                                    } else {
                                                        progress
                                                    })
                                            },
                                        ),
                                )
                                .with_animation(
                                    ("file-editor-backdrop", usize::from(closing)),
                                    Animation::new(Duration::from_millis(260)),
                                    move |backdrop, progress| {
                                        backdrop.opacity(if closing {
                                            1. - progress
                                        } else {
                                            progress
                                        })
                                    },
                                )
                                .into_any_element(),
                        )
                        .into_any_element(),
                )
            })
            .into_any_element()
    }
}

fn menu_item(
    label: &'static str,
    icon: &'static str,
    palette: &Palette,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(label)
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .py_2()
        .rounded_sm()
        .cursor_pointer()
        .text_sm()
        .text_color(palette.text)
        .hover(|style| style.bg(palette.background))
        .child(svg().path(icon).size_4().text_color(palette.muted))
        .child(label)
}

fn format_size(size: u64) -> String {
    if size < 1024 {
        format!("{size} B")
    } else if size < 1024 * 1024 {
        format!("{:.1} KB", size as f64 / 1024.)
    } else {
        format!("{:.1} MB", size as f64 / (1024. * 1024.))
    }
}

fn format_modified(time: Option<std::time::SystemTime>) -> String {
    let Some(time) = time else {
        return "—".into();
    };
    let elapsed = std::time::SystemTime::now()
        .duration_since(time)
        .unwrap_or_default();
    let seconds = elapsed.as_secs();
    if seconds < 60 {
        "just now".into()
    } else if seconds < 3600 {
        format!("{} min ago", seconds / 60)
    } else if seconds < 86400 {
        format!("{} h ago", seconds / 3600)
    } else {
        format!("{} d ago", seconds / 86400)
    }
}
