use crate::{
    modals::create_server::{CreateServerModal, CreationEvent},
    widgets::{
        empty_state,
        server::screen::{Back, ServerScreen},
    },
};
use gpui::{
    Animation, AnimationExt, AnyWindowHandle, AppContext, Bounds, Context, Entity, FocusHandle,
    IntoElement, MouseButton, Pixels, Render, Subscription, Timer, Window, canvas, div,
    ease_out_quint, point, prelude::*, px, rgba, size,
};
use services::{AppState, instance::ServerInstance};
use std::{cell::Cell, collections::HashMap, rc::Rc, time::Duration};
use ui::components::button::button;

pub struct Servers {
    screens: HashMap<String, Entity<ServerScreen>>,
    selected: Option<String>,
    modal: Option<Entity<CreateServerModal>>,
    modal_subscription: Option<Subscription>,
    subscriptions: Vec<Subscription>,
    closing: bool,
    origin: Bounds<Pixels>,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    button_bounds: Rc<Cell<Bounds<Pixels>>>,
    focus: FocusHandle,
    window: Option<AnyWindowHandle>,
}
impl Servers {
    pub fn new(cx: &mut Context<Self>) -> Self {
        cx.observe_global::<AppState>(|_, cx| cx.notify()).detach();
        Self {
            screens: HashMap::new(),
            selected: None,
            modal: None,
            modal_subscription: None,
            subscriptions: Vec::new(),
            closing: false,
            origin: Bounds::default(),
            bounds: Rc::default(),
            button_bounds: Rc::default(),
            focus: cx.focus_handle().tab_index(0),
            window: None,
        }
    }
    fn add(&mut self, server: ServerInstance, cx: &mut Context<Self>) {
        if self.screens.contains_key(&server.id) {
            return;
        }
        let id = server.id.clone();
        let screen = cx.new(|cx| ServerScreen::new(server, cx));
        self.subscriptions
            .push(cx.subscribe(&screen, |this, _, _: &Back, cx| {
                this.selected = None;
                cx.notify();
            }));
        self.screens.insert(id, screen);
    }
    fn open_server(&mut self, server: ServerInstance, cx: &mut Context<Self>) {
        let id = server.id.clone();
        if self.screens.contains_key(&id) {
            if let Some(screen) = self.screens.get(&id) {
                screen.update(cx, |screen, cx| screen.refresh_console(cx));
            }
        } else {
            self.add(server, cx);
        }
        self.selected = Some(id);
        cx.notify();
    }
    fn open_create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.modal.is_some() {
            return;
        }
        self.window = Some(window.window_handle());
        let parent = self.bounds.get();
        let button = self.button_bounds.get();
        self.origin = Bounds::new(button.origin - parent.origin, button.size);
        self.closing = false;
        let modal = cx.new(CreateServerModal::new);
        window.focus(&modal.read(cx).focus);
        self.modal_subscription = Some(cx.subscribe(&modal, |this, _, event, cx| match event {
            CreationEvent::Created(server) => {
                cx.update_global::<AppState, _>(|state, _| {
                    state.instance_service.insert(server.clone())
                });
                this.add(server.clone(), cx);
                this.selected = Some(server.id.clone());
                this.close_create(cx);
            }
            CreationEvent::Close => this.close_create(cx),
        }));
        self.modal = Some(modal);
        cx.notify();
    }
    fn close_create(&mut self, cx: &mut Context<Self>) {
        if self.closing || self.modal.as_ref().is_some_and(|m| m.read(cx).busy) {
            return;
        }
        self.closing = true;
        cx.notify();
        let window = self.window;
        let focus = self.focus.clone();
        cx.spawn(async move |this, cx| {
            Timer::after(Duration::from_millis(300)).await;
            let _ = this.update(cx, |this, cx| {
                this.modal = None;
                this.modal_subscription = None;
                this.closing = false;
                cx.notify();
            });
            if let Some(window) = window {
                let _ = window.update(cx, |_, window, _| window.focus(&focus));
            }
        })
        .detach();
    }
    fn morph(&self, modal: Entity<CreateServerModal>, cx: &Context<Self>) -> impl IntoElement {
        let palette = cx.global::<AppState>().theme_manager.palette();
        let bounds = self.bounds.get();
        let width = (bounds.size.width - px(24.)).min(px(560.)).max(px(1.));
        let height = (bounds.size.height - px(24.)).min(px(680.)).max(px(1.));
        let target = Bounds::new(
            point(
                (bounds.size.width - width) / 2.,
                (bounds.size.height - height) / 2.,
            ),
            size(width, height),
        );
        let source = self.origin;
        let closing = self.closing;
        div()
            .id("server-create-overlay")
            .absolute()
            .inset_0()
            .size_full()
            .occlude()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| this.close_create(cx)),
            )
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .bg(rgba(0x00000088))
                    .with_animation(
                        ("create-backdrop", usize::from(closing)),
                        Animation::new(Duration::from_millis(300)),
                        move |el, p| el.opacity(if closing { 1. - p } else { p }),
                    ),
            )
            .child(
                div()
                    .id("create-morph")
                    .absolute()
                    .overflow_hidden()
                    .rounded_lg()
                    .bg(palette.background)
                    .border_1()
                    .border_color(palette.border)
                    .shadow_lg()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(div().size_full().child(modal).with_animation(
                        ("create-content", usize::from(closing)),
                        Animation::new(Duration::from_millis(300)),
                        move |el, p| {
                            el.opacity(if closing {
                                (1. - p * 3.).max(0.)
                            } else {
                                ((p - 0.35) / 0.65).max(0.)
                            })
                        },
                    ))
                    .with_animation(
                        ("create-morph-motion", usize::from(closing)),
                        Animation::new(Duration::from_millis(300)).with_easing(ease_out_quint()),
                        move |el, p| {
                            let p = if closing { 1. - p } else { p };
                            el.left(source.origin.x + (target.origin.x - source.origin.x) * p)
                                .top(source.origin.y + (target.origin.y - source.origin.y) * p)
                                .w(source.size.width + (target.size.width - source.size.width) * p)
                                .h(source.size.height
                                    + (target.size.height - source.size.height) * p)
                        },
                    ),
            )
    }
}
impl Render for Servers {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.global::<AppState>().theme_manager.palette();
        let bounds = self.bounds.clone();
        let button_bounds = self.button_bounds.clone();
        let screen = self
            .selected
            .as_ref()
            .and_then(|id| self.screens.get(id))
            .cloned();
        let mut servers = cx
            .global::<AppState>()
            .instance_service
            .servers()
            .iter()
            .collect::<Vec<_>>();
        servers.sort_by(|a, b| a.name.cmp(&b.name));
        div()
            .relative()
            .size_full()
            .min_h_0()
            .min_w_0()
            .child(
                canvas(
                    move |bounds_value, _, _| bounds.set(bounds_value),
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
            .map(|root| {
                if let Some(screen) = screen {
                    root.child(screen)
                } else {
                    root.child(
                        div()
                            .size_full()
                            .flex()
                            .flex_col()
                            .p_8()
                            .gap_6()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap_4()
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_2()
                                            .child(div().text_2xl().child("Servers"))
                                            .child(
                                                div().text_color(palette.muted).child(format!(
                                                    "{} local servers",
                                                    servers.len()
                                                )),
                                            ),
                                    )
                                    .child(
                                        button(
                                            "create-server-button",
                                            "Create server",
                                            palette,
                                            true,
                                            self.modal.is_none(),
                                        )
                                        .relative()
                                        .track_focus(&self.focus)
                                        .opacity(if self.modal.is_some() { 0. } else { 1. })
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.open_create(window, cx)
                                        }))
                                        .on_key_down(cx.listener(
                                            |this, event: &gpui::KeyDownEvent, window, cx| {
                                                if matches!(
                                                    event.keystroke.key.as_str(),
                                                    "enter" | "space"
                                                ) {
                                                    this.open_create(window, cx);
                                                    cx.stop_propagation();
                                                }
                                            },
                                        ))
                                        .child(
                                            canvas(
                                                move |bounds, _, _| button_bounds.set(bounds),
                                                |_, _, _, _| {},
                                            )
                                            .absolute()
                                            .inset_0()
                                            .size_full(),
                                        ),
                                    ),
                            )
                            .when(servers.is_empty(), |el| {
                                el.child(empty_state::render_icon(
                                    "icons/server.svg",
                                    "No servers yet",
                                    "Create your first Minecraft server to get started.",
                                    palette,
                                ))
                            })
                            .child(
                                div()
                                    .id("servers-list")
                                    .flex_1()
                                    .min_h_0()
                                    .overflow_y_scroll()
                                    .flex()
                                    .flex_col()
                                    .gap_3()
                                    .children(servers.iter().map(|server| {
                                        let id = server.id.clone();
                                        let running = self
                                            .screens
                                            .get(&id)
                                            .is_some_and(|screen| screen.read(cx).running);
                                        div()
                                            .id(gpui::SharedString::from(id.clone()))
                                            .p_5()
                                            .rounded_lg()
                                            .border_1()
                                            .border_color(palette.border)
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .gap_3()
                                            .cursor_pointer()
                                            .hover(|style| style.bg(palette.surface))
                                            .on_click(cx.listener({
                                                let server = (*server).clone();
                                                move |this, _, _, cx| {
                                                    this.open_server(server.clone(), cx)
                                                }
                                            }))
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    .gap_2()
                                                    .child(server.name.clone())
                                                    .child(
                                                        div()
                                                            .text_sm()
                                                            .text_color(palette.muted)
                                                            .child(format!(
                                                                "{} · {} · {} MiB",
                                                                server.version,
                                                                server.software,
                                                                server.ram
                                                            )),
                                                    ),
                                            )
                                            .child(
                                                div().text_xs().text_color(palette.muted).child(
                                                    if running { "Running" } else { "Stopped" },
                                                ),
                                            )
                                    })),
                            ),
                    )
                }
            })
            .when_some(self.modal.clone(), |root, modal| {
                root.child(self.morph(modal, cx))
            })
    }
}
