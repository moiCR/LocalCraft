use gpui::{
    AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement, Render, Window,
    div, prelude::*, px, svg,
};
use gpui_kit::component::input::{Textarea, TextareaState};
use services::{
    AppState,
    instance::{ServerInstance, files},
};
use std::path::PathBuf;
use ui::components::button::button;

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
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.global::<AppState>().theme_manager.palette();
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .gap_3()
            .track_focus(&self.focus)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_xs()
                            .text_color(palette.muted)
                            .truncate()
                            .child(self.path.to_string_lossy().into_owned()),
                    )
                    .child(
                        button("save-instance-file", "", palette, true, !self.busy)
                            .w(px(36.))
                            .h(px(36.))
                            .p_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .flex_shrink_0()
                            .child(
                                svg()
                                    .path("icons/save.svg")
                                    .size_4()
                                    .text_color(palette.background),
                            )
                            .on_click(cx.listener(|this, _, _, cx| this.save(cx))),
                    )
                    .child(
                        button("close-instance-file", "", palette, false, !self.busy)
                            .w(px(36.))
                            .h(px(36.))
                            .p_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .flex_shrink_0()
                            .child(
                                svg()
                                    .path("icons/close.svg")
                                    .size_4()
                                    .text_color(palette.text),
                            )
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
                    .text_color(gpui::rgb(0xe87878))
                    .child(error.clone())
            }))
            .child(
                div()
                    .id("file-editor-lines")
                    .flex_1()
                    .min_h_0()
                    .bg(palette.sidebar)
                    .rounded_md()
                    .child(
                        Textarea::new(&self.contents)
                            .h_full()
                            .appearance(false)
                            .bordered(false),
                    ),
            )
    }
}
