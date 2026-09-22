use gpui::{
    App, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement, Render, SharedString,
    Window, div, prelude::*, px, rgba,
};
use std::{cell::Cell, rc::Rc};

pub struct Select {
    pub options: Vec<SharedString>,
    pub selected: Option<usize>,
    pub enabled: bool,
    pub placeholder: SharedString,
    open: bool,
    menu_bounds: Rc<Cell<gpui::Bounds<gpui::Pixels>>>,
    focus: FocusHandle,
}

pub struct Selected(pub usize);
impl EventEmitter<Selected> for Select {}
impl Select {
    pub fn new(placeholder: &str, cx: &mut Context<Self>) -> Self {
        Self {
            options: Vec::new(),
            selected: None,
            enabled: true,
            placeholder: placeholder.to_owned().into(),
            open: false,
            menu_bounds: Rc::default(),
            focus: cx.focus_handle().tab_index(0),
        }
    }
    pub fn set_options(&mut self, options: Vec<SharedString>, cx: &mut Context<Self>) {
        self.options = options;
        self.selected = None;
        self.open = false;
        cx.notify();
    }
    pub fn value(&self) -> Option<&str> {
        self.selected
            .and_then(|index| self.options.get(index))
            .map(|v| v.as_ref())
    }
    fn choose(&mut self, index: usize, cx: &mut Context<Self>) {
        if !self.enabled || self.options.get(index).is_none() {
            return;
        }
        self.selected = Some(index);
        self.open = false;
        cx.emit(Selected(index));
        cx.notify();
    }
}
impl Focusable for Select {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
impl Render for Select {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let menu_bounds = self.menu_bounds.clone();
        let text_color = window.text_style().color;
        let dark = text_color.l > 0.5;
        let background = if dark {
            gpui::rgb(0x141414)
        } else {
            gpui::rgb(0xffffff)
        };
        div()
            .id("select")
            .relative()
            .w_full()
            .min_w_0()
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                if !this.enabled {
                    return;
                }
                match event.keystroke.key.as_str() {
                    "escape" => {
                        this.open = false;
                        cx.notify();
                    }
                    "enter" | "space" => {
                        this.open = !this.open;
                        cx.notify();
                    }
                    "up" if !this.options.is_empty() => {
                        this.choose(this.selected.unwrap_or(0).saturating_sub(1), cx)
                    }
                    "down" if !this.options.is_empty() => this.choose(
                        this.selected
                            .map_or(0, |v| (v + 1).min(this.options.len().saturating_sub(1))),
                        cx,
                    ),
                    "tab" => {
                        this.open = false;
                        if event.keystroke.modifiers.shift {
                            window.focus_prev();
                        } else {
                            window.focus_next();
                        }
                        cx.notify();
                    }
                    _ => return,
                }
                cx.stop_propagation();
            }))
            .on_mouse_down_out(cx.listener(|this, event: &gpui::MouseDownEvent, _, cx| {
                if this.open && !this.menu_bounds.get().contains(&event.position) {
                    this.open = false;
                    cx.notify();
                }
            }))
            .child(
                div()
                    .id("select-trigger")
                    .flex()
                    .items_center()
                    .justify_between()
                    .h(px(38.))
                    .px_3()
                    .border_1()
                    .border_color(rgba(0x88888855))
                    .rounded_md()
                    .opacity(if self.enabled { 1. } else { 0.45 })
                    .when(self.enabled, |element| element.cursor_pointer())
                    .on_click(cx.listener(|this, _, window, cx| {
                        if this.enabled {
                            window.focus(&this.focus);
                            this.open = !this.open;
                            cx.notify();
                        }
                    }))
                    .child(
                        self.value()
                            .map(str::to_owned)
                            .map(SharedString::from)
                            .unwrap_or_else(|| self.placeholder.clone()),
                    )
                    .child("⌄"),
            )
            .when(self.open && self.enabled, |element| {
                element.child(
                    gpui::deferred(
                        div()
                            .id("select-menu")
                            .absolute()
                            .top_full()
                            .left_0()
                            .w_full()
                            .mt_1()
                            .max_h(px(180.))
                            .overflow_y_scroll()
                            .bg(background)
                            .text_color(text_color)
                            .border_1()
                            .border_color(rgba(0x88888855))
                            .rounded_md()
                            .shadow_lg()
                            .occlude()
                            .child(
                                gpui::canvas(
                                    move |bounds, _, _| menu_bounds.set(bounds),
                                    |_, _, _, _| {},
                                )
                                .absolute()
                                .inset_0()
                                .size_full(),
                            )
                            .children(self.options.iter().enumerate().map(|(index, option)| {
                                div()
                                    .id(option.clone())
                                    .px_3()
                                    .py_2()
                                    .cursor_pointer()
                                    .hover(|style| style.bg(rgba(0x88888830)))
                                    .on_click(
                                        cx.listener(move |this, _, _, cx| this.choose(index, cx)),
                                    )
                                    .child(option.clone())
                            })),
                    )
                    .with_priority(10),
                )
            })
    }
}

pub fn field(label: &'static str, select: &Entity<Select>) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w_0()
        .gap_2()
        .child(label)
        .child(select.clone())
}
