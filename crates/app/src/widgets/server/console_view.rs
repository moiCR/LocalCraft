use super::console::{Console, ROW_HEIGHT};
use gpui::{
    Animation, AnimationExt, AnyView, AppContext, Context, Entity, IntoElement, ListSizingBehavior,
    Render, StyleRefinement, StyledText, Transformation, Window, div, percentage, prelude::*, px,
    svg, uniform_list,
};
use services::{AppState, instance::console::ConsoleSnapshot};
use std::{sync::Arc, time::Duration};
use ui::components::input::{Input, InputEvent};

#[cfg(test)]
#[path = "console_view_tests.rs"]
mod tests;

pub struct ConsoleView {
    console: Console,
    auto_scroll_setting: bool,
    loading: bool,
    search: Entity<Input>,
    _search_subscription: gpui::Subscription,
    _stream_task: gpui::Task<()>,
    #[cfg(test)]
    render_count: usize,
    #[cfg(test)]
    rendered_rows: std::rc::Rc<std::cell::Cell<usize>>,
}

impl ConsoleView {
    pub fn new(
        mut updates: tokio::sync::watch::Receiver<Arc<ConsoleSnapshot>>,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut console = Console::default();
        let auto_scroll_setting = cx.global::<AppState>().preferences.console_auto_scroll;
        console.follow = auto_scroll_setting;
        cx.observe_global::<AppState>(|this, cx| {
            let auto_scroll = cx.global::<AppState>().preferences.console_auto_scroll;
            if this.auto_scroll_setting != auto_scroll {
                this.auto_scroll_setting = auto_scroll;
                this.console.follow = auto_scroll;
                if auto_scroll {
                    this.console.follow_tail();
                }
            }
            cx.notify();
        })
        .detach();
        let search = cx.new(|cx| Input::new("", "Search console…", cx));
        let search_subscription = cx.subscribe(&search, |this, search, event, cx| {
            if matches!(event, InputEvent::Changed) {
                this.console
                    .set_filter(search.read(cx).value().trim().to_lowercase());
                cx.notify();
            }
        });
        let stream_task = cx.spawn(async move |this, cx| {
            let mut first = true;
            loop {
                let snapshot = updates.borrow_and_update().clone();
                if this
                    .update(cx, |this, cx| {
                        if first {
                            this.set_snapshot(&snapshot, cx);
                        } else if this.console.append_snapshot(&snapshot) {
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
                first = false;
                if updates.changed().await.is_err() {
                    break;
                }
            }
        });
        Self {
            console,
            auto_scroll_setting,
            loading: true,
            search,
            _search_subscription: search_subscription,
            _stream_task: stream_task,
            #[cfg(test)]
            render_count: 0,
            #[cfg(test)]
            rendered_rows: Default::default(),
        }
    }

    pub fn cached(view: Entity<Self>) -> impl IntoElement {
        AnyView::from(view).cached(
            StyleRefinement::default()
                .flex_1()
                .min_h_0()
                .min_w_0()
                .w_full(),
        )
    }

    pub fn set_snapshot(&mut self, snapshot: &ConsoleSnapshot, cx: &mut Context<Self>) {
        self.console.replace(snapshot);
        self.loading = false;
        cx.notify();
    }

    pub fn echo(&mut self, text: String, cx: &mut Context<Self>) {
        self.console.echo(text);
        cx.notify();
    }

    fn clear(&mut self, cx: &mut Context<Self>) {
        self.console.clear();
        cx.notify();
    }
}

impl Render for ConsoleView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[cfg(test)]
        {
            self.render_count += 1;
        }
        #[cfg(test)]
        let rendered_rows = self.rendered_rows.clone();
        let palette = cx.global::<AppState>().theme_manager.palette();
        let timestamp_color = palette.muted;
        let show_timestamps = cx.global::<AppState>().preferences.console_timestamps;
        let entity = cx.entity();
        div()
            .flex()
            .flex_col()
            .size_full()
            .min_h_0()
            .min_w_0()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(div().text_xs().text_color(palette.muted).child("CONSOLE"))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .w(px(460.))
                            .child(div().flex_1().min_w_0().child(self.search.clone()))
                            .child(
                                div()
                                    .id("copy-console")
                                    .text_xs()
                                    .cursor_pointer()
                                    .text_color(palette.muted)
                                    .child("Copy all")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        let text = this
                                            .console
                                            .rows
                                            .iter()
                                            .map(|(_, line)| line.text.as_ref())
                                            .collect::<Vec<_>>()
                                            .join("\n");
                                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                            text,
                                        ));
                                    })),
                            )
                            .child(
                                div()
                                    .id("clear-console")
                                    .text_xs()
                                    .cursor_pointer()
                                    .text_color(palette.muted)
                                    .child("Clear")
                                    .on_click(cx.listener(|this, _, _, cx| this.clear(cx))),
                            )
                            .child(
                                div()
                                    .id("follow-console")
                                    .text_xs()
                                    .cursor_pointer()
                                    .text_color(palette.muted)
                                    .child(if self.console.follow {
                                        "Auto-scroll on"
                                    } else {
                                        "Auto-scroll off"
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.console.follow = !this.console.follow;
                                        this.console.follow_tail();
                                        let enabled = this.console.follow;
                                        cx.update_global::<AppState, _>(|state, _| {
                                            state.preferences.console_auto_scroll = enabled;
                                            state.save_preferences();
                                        });
                                        cx.notify();
                                    })),
                            ),
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
                            svg()
                                .path("icons/loader.svg")
                                .size_6()
                                .text_color(palette.muted)
                                .with_animation(
                                    "server-console-loader",
                                    Animation::new(Duration::from_millis(900)).repeat(),
                                    |icon, progress| {
                                        icon.with_transformation(Transformation::rotate(
                                            percentage(progress),
                                        ))
                                    },
                                ),
                        )
                    })
                    .when(!self.loading, |element| {
                        element.child(
                            uniform_list(
                                "server-console",
                                self.console.visible_len(),
                                move |range, _, cx| {
                                    #[cfg(test)]
                                    rendered_rows.set(rendered_rows.get() + range.len());
                                    let console = entity.read(cx);
                                    range
                                        .filter_map(|index| console.console.visible_row(index))
                                        .map(|(id, line)| {
                                            let copy_line = line.text.clone();
                                            let timestamp =
                                                console.console.timestamp(*id).map(str::to_owned);
                                            div()
                                                .id(("console-row", *id))
                                                .h(px(ROW_HEIGHT))
                                                .line_height(px(ROW_HEIGHT))
                                                .w_full()
                                                .min_w_0()
                                                .overflow_hidden()
                                                .px_3()
                                                .text_xs()
                                                .font_family("monospace")
                                                .truncate()
                                                .child(
                                                    div()
                                                        .flex()
                                                        .items_center()
                                                        .gap_2()
                                                        .when(show_timestamps, |element| {
                                                            element.child(
                                                                div()
                                                                    .flex_shrink_0()
                                                                    .text_color(timestamp_color)
                                                                    .child(
                                                                        timestamp
                                                                            .unwrap_or_default(),
                                                                    ),
                                                            )
                                                        })
                                                        .child(
                                                            StyledText::new(line.text.clone())
                                                                .with_highlights(
                                                                    line.highlights.iter().cloned(),
                                                                ),
                                                        ),
                                                )
                                                .cursor_pointer()
                                                .hover(|style| style.bg(gpui::rgba(0xffffff0a)))
                                                .on_click(move |_, _, cx| {
                                                    cx.write_to_clipboard(
                                                        gpui::ClipboardItem::new_string(
                                                            copy_line.to_string(),
                                                        ),
                                                    );
                                                })
                                        })
                                        .collect::<Vec<_>>()
                                },
                            )
                            .with_sizing_behavior(ListSizingBehavior::Auto)
                            .absolute()
                            .inset_0()
                            .size_full()
                            .track_scroll(&self.console.scroll)
                            .on_scroll_wheel(cx.listener(
                                |this, _, _, _| {
                                    this.console.pause_follow();
                                },
                            )),
                        )
                    }),
            )
    }
}
