use gpui::{
    Context, IntoElement, ListSizingBehavior, Render, ScrollStrategy, UniformListScrollHandle,
    Window, div, prelude::*, px, uniform_list,
};
use services::{AppState, instance::ServerInstance};

const LOG_CAPACITY: usize = 5_000;
const LOG_ROW_HEIGHT: f32 = 22.;

pub struct LogsView {
    server: ServerInstance,
    lines: Vec<String>,
    loading: bool,
    error: Option<String>,
    generation: u64,
    scroll: UniformListScrollHandle,
}

impl LogsView {
    pub fn new(server: ServerInstance) -> Self {
        Self {
            server,
            lines: Vec::new(),
            loading: false,
            error: None,
            generation: 0,
            scroll: UniformListScrollHandle::new(),
        }
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let log_paths = match self.server.directory() {
            Ok(directory) => {
                let logs = directory.join("logs");
                (logs.join("latest.txt"), logs.join("latest.log"))
            }
            Err(error) => {
                self.error = Some(format!("Could not locate instance logs: {error:#}"));
                cx.notify();
                return;
            }
        };
        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;
        self.loading = true;
        self.error = None;
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        cx.global::<AppState>()
            .background_runtime
            .spawn(async move {
                let result = match tokio::fs::read_to_string(&log_paths.0).await {
                    Ok(contents) => Ok(contents),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        match tokio::fs::read_to_string(&log_paths.1).await {
                            Ok(contents) => Ok(contents),
                            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                                Ok(String::new())
                            }
                            Err(error) => {
                                Err(format!("Could not read {}: {error}", log_paths.1.display()))
                            }
                        }
                    }
                    Err(error) => Err(format!("Could not read {}: {error}", log_paths.0.display())),
                };
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
                        Ok(contents) => {
                            this.lines = contents
                                .lines()
                                .rev()
                                .take(LOG_CAPACITY)
                                .map(str::to_owned)
                                .collect();
                            this.lines.reverse();
                            if let Some(last) = this.lines.len().checked_sub(1) {
                                this.scroll.scroll_to_item(last, ScrollStrategy::Bottom);
                            }
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
}

impl Render for LogsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        let palette = cx.global::<AppState>().theme_manager.palette();
        let scroll = self.scroll.clone();
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
                    .justify_between()
                    .child(
                        div()
                            .text_xs()
                            .text_color(palette.muted)
                            .child("INSTANCE LOG"),
                    )
                    .child(
                        div()
                            .id("refresh-server-logs")
                            .text_xs()
                            .text_color(palette.muted)
                            .cursor_pointer()
                            .hover(|style| style.text_color(palette.text))
                            .child("Refresh")
                            .on_click(cx.listener(|this, _, _, cx| this.refresh(cx))),
                    ),
            )
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .overflow_hidden()
                    .bg(palette.sidebar)
                    .rounded_md()
                    .when(self.loading, |element| {
                        element.flex().items_center().justify_center().child(
                            div()
                                .text_xs()
                                .text_color(palette.muted)
                                .child("Loading logs…"),
                        )
                    })
                    .when(!self.loading && self.error.is_some(), |element| {
                        element.flex().items_center().justify_center().p_4().child(
                            div()
                                .text_xs()
                                .text_color(palette.muted)
                                .child(self.error.clone().unwrap_or_default()),
                        )
                    })
                    .when(
                        !self.loading && self.error.is_none() && self.lines.is_empty(),
                        |element| {
                            element.flex().items_center().justify_center().child(
                                div()
                                    .text_xs()
                                    .text_color(palette.muted)
                                    .child("No logs yet."),
                            )
                        },
                    )
                    .when(!self.loading && !self.lines.is_empty(), |element| {
                        element.child(
                            uniform_list("instance-logs", self.lines.len(), move |range, _, cx| {
                                let logs = entity.read(cx);
                                range
                                    .filter_map(|index| {
                                        logs.lines.get(index).map(|line| (index, line.clone()))
                                    })
                                    .map(|(index, line)| {
                                        div()
                                            .id(("instance-log-line", index))
                                            .h(px(LOG_ROW_HEIGHT))
                                            .line_height(px(LOG_ROW_HEIGHT))
                                            .w_full()
                                            .min_w_0()
                                            .overflow_hidden()
                                            .px_3()
                                            .text_xs()
                                            .font_family("monospace")
                                            .truncate()
                                            .child(line)
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .with_sizing_behavior(ListSizingBehavior::Auto)
                            .absolute()
                            .inset_0()
                            .size_full()
                            .track_scroll(&scroll),
                        )
                    }),
            )
    }
}
