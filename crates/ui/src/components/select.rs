use crate::theme::ActiveUiTheme;
use gpui::{
    App, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement, Render, SharedString,
    Window, div, prelude::*, px,
};
use std::{cell::Cell, rc::Rc};

pub struct Select {
    pub options: Vec<SharedString>,
    pub selected: Option<usize>,
    pub enabled: bool,
    pub placeholder: SharedString,
    open: bool,
    menu_bounds: Rc<Cell<gpui::Bounds<gpui::Pixels>>>,
    menu_scroll: gpui::ScrollHandle,
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
            menu_scroll: gpui::ScrollHandle::new(),
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let menu_bounds = self.menu_bounds.clone();
        let menu_scroll = self.menu_scroll.clone();
        let palette = cx.ui_palette();
        let background = palette.surface;
        let field_background = palette.card_surface();
        let hover_bg = palette.surface_hover;
        let border = palette.border;
        let text_color = palette.text;
        let muted = palette.muted;
        let has_value = self.value().is_some();
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
                            window.focus_prev(cx);
                        } else {
                            window.focus_next(cx);
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
                    .gap_2()
                    .h(px(32.))
                    .px_3()
                    .bg(field_background)
                    .border_1()
                    .border_color(border)
                    .rounded(px(16.))
                    .text_size(px(13.))
                    .text_color(if has_value { text_color } else { muted })
                    .opacity(if self.enabled { 1. } else { 0.45 })
                    .when(self.enabled, |element| element.cursor_pointer())
                    .on_click(cx.listener(|this, _, window, cx| {
                        if this.enabled {
                            window.focus(&this.focus, cx);
                            this.open = !this.open;
                            cx.notify();
                        }
                    }))
                    .child(
                        div().min_w_0().truncate().child(
                            self.value()
                                .map(str::to_owned)
                                .map(SharedString::from)
                                .unwrap_or_else(|| self.placeholder.clone()),
                        ),
                    )
                    .child(div().flex_none().text_color(muted).child("⌄")),
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
                            .p_1()
                            .max_h(px(240.))
                            .bg(background)
                            .text_color(text_color)
                            .border_1()
                            .border_color(border)
                            .rounded(px(16.))
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
                            .child(
                                div()
                                    .id("select-menu-options")
                                    .max_h(px(230.))
                                    .overflow_y_scroll()
                                    .scrollbar_width(px(8.))
                                    .track_scroll(&menu_scroll)
                                    .children(self.options.iter().enumerate().map(
                                        |(index, option)| {
                                            let is_selected = self.selected == Some(index);
                                            div()
                                                .id(option.clone())
                                                .px_3()
                                                .py_1p5()
                                                .rounded(px(12.))
                                                .text_size(px(13.))
                                                .when(is_selected, |row| row.bg(hover_bg))
                                                .cursor_pointer()
                                                .hover(move |style| style.bg(hover_bg))
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.choose(index, cx)
                                                }))
                                                .child(option.clone())
                                        },
                                    )),
                            ),
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
        .gap_1p5()
        .child(div().text_xs().child(label))
        .child(select.clone())
}
