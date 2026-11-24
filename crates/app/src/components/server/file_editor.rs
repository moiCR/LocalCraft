use gpui::{
    AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement, Render, Window,
    div, prelude::*, px,
};
use gpui_kit::component::input::{Textarea, TextareaState};
use services::{
    AppState,
    instance::{ServerInstance, files},
};
use std::path::PathBuf;
use ui::{components::IconButton, theme::ActiveUiTheme};

pub enum EditorEvent {
    Saved,
    Close,
}

pub struct FileEditor {
    server: ServerInstance,
    path: PathBuf,
    contents: Entity<TextareaState>,
    focus: FocusHandle,
    busy: bool,
    error: Option<String>,
}

impl EventEmitter<EditorEvent> for FileEditor {}

impl FileEditor {
    pub fn new(
        window: &mut Window,
        server: ServerInstance,
        path: PathBuf,
        contents: &str,
        cx: &mut Context<Self>,
    ) -> Self {
        let contents = cx.new(|cx| {
            TextareaState::new(window, cx)
                .default_value(contents)
                .soft_wrap(false)
        });
        Self {
            server,
            path,
            contents,
            focus: cx.focus_handle(),
            busy: false,
            error: None,
        }
    }

    fn save(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let contents = self.contents.read(cx).value().to_owned();
        let server = self.server.clone();
        let path = self.path.clone();
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        self.busy = true;
        self.error = None;
        cx.global::<AppState>()
            .background_runtime
            .spawn(async move {
                let result = files::save_text(&server, &path, &contents)
                    .await
                    .map_err(|error| format!("{error:#}"));
                let _ = tx.send(result).await;
            });
        cx.spawn(async move |this, cx| {
            if let Some(result) = rx.recv().await {
                let _ = this.update(cx, |this, cx| {
                    this.busy = false;
                    match result {
                        Ok(()) => cx.emit(EditorEvent::Saved),
                        Err(error) => this.error = Some(error),
                    }
                    cx.notify();
                });
            }
        })
        .detach();
        cx.notify();
    }
}

impl Focusable for FileEditor {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for FileEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.ui_palette();
        let viewport = window.viewport_size();
        let vw = f32::from(viewport.width);
        let vh = f32::from(viewport.height);
        let compact = vw < 860.0 || vh < 580.0;
        let editor_text_size = if vw < 760.0 || vh < 520.0 {
            px(11.5)
        } else if compact {
            px(12.5)
        } else {
            px(13.5)
        };
        let path_text_size = if compact { px(11.0) } else { px(12.0) };

        div()
            .size_full()
            .flex()
            .flex_col()
            .min_h_0()
            .min_w_0()
            .gap(if compact { px(8.0) } else { px(12.0) })
            .track_focus(&self.focus)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .flex_shrink_0()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(path_text_size)
                            .text_color(palette.muted)
                            .truncate()
                            .child(self.path.to_string_lossy().into_owned()),
                    )
                    .child(
                        IconButton::new("save-instance-file", "icons/save.svg")
                            .tooltip("Save file")
                            .color(palette.accent)
                            .disabled(self.busy)
                            .on_click(cx.listener(|this, _, _, cx| this.save(cx))),
                    )
                    .child(
                        IconButton::new("close-instance-file", "icons/x.svg")
                            .tooltip("Close editor")
                            .disabled(self.busy)
                            .on_click(cx.listener(|this, _, _, cx| {
                                if !this.busy {
                                    cx.emit(EditorEvent::Close);
                                }
                            })),
                    ),
            )
            .children(self.error.as_ref().map(|error| {
                div()
                    .text_xs()
                    .text_color(palette.error)
                    .child(error.clone())
            }))
            .child(
                div()
                    .id("file-editor-lines")
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .w_full()
                    .bg(palette.card_surface())
                    .border_1()
                    .border_color(palette.border)
                    .rounded(px(16.0))
                    .overflow_hidden()
                    .text_size(editor_text_size)
                    .child(
                        Textarea::new(&self.contents)
                            .h_full()
                            .text_size(editor_text_size)
                            .appearance(false)
                            .bordered(false),
                    ),
            )
    }
}
