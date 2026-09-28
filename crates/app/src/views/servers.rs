use crate::{
    modals::create_server::{CreateServerModal, CreationEvent},
    widgets::{
        empty_state,
        server::screen::{Back, ServerScreen},
    },
};
use gpui::{
    Anchor, Animation, AnimationExt, AnyWindowHandle, AppContext, Bounds, Context, Entity,
    FocusHandle, IntoElement, MouseButton, Pixels, Point, Render, Subscription, Window, anchored,
    canvas, div, ease_out_quint, point, prelude::*, px, rgba, size, svg,
};
use services::{AppState, instance::ServerInstance};
use smol::Timer;
use std::{cell::Cell, collections::HashMap, rc::Rc, time::Duration};
use ui::components::button::button;

pub struct Servers {
    screens: HashMap<String, Entity<ServerScreen>>,
    selected: Option<String>,
    modal: Option<Entity<CreateServerModal>>,
    modal_subscription: Option<Subscription>,
    subscriptions: HashMap<String, Subscription>,
    closing: bool,
    instance_menu: Option<(String, Point<Pixels>)>,
    delete_target: Option<ServerInstance>,
    delete_busy: bool,
    delete_error: Option<String>,
    delete_closing: bool,
    delete_generation: u64,
    bounds: Rc<Cell<Bounds<Pixels>>>,
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
            subscriptions: HashMap::new(),
            closing: false,
            instance_menu: None,
            delete_target: None,
            delete_busy: false,
            delete_error: None,
            delete_closing: false,
            delete_generation: 0,
            bounds: Rc::default(),
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
        self.subscriptions.insert(
            id.clone(),
            cx.subscribe(&screen, |this, _, _: &Back, cx| {
                this.selected = None;
                cx.notify();
            }),
        );
        self.screens.insert(id, screen);
    }
    fn open_server(&mut self, server: ServerInstance, cx: &mut Context<Self>) {
        self.instance_menu = None;
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

    fn open_delete_dialog(&mut self, server: ServerInstance, cx: &mut Context<Self>) {
        self.instance_menu = None;
        self.delete_generation = self.delete_generation.wrapping_add(1);
        self.delete_closing = false;
        self.delete_busy = false;
        self.delete_error = None;
        self.delete_target = Some(server);
        cx.notify();
    }

    fn close_delete_dialog(&mut self, cx: &mut Context<Self>) {
        if self.delete_busy || self.delete_closing || self.delete_target.is_none() {
            return;
        }
        self.delete_closing = true;
        self.delete_generation = self.delete_generation.wrapping_add(1);
        let generation = self.delete_generation;
        cx.notify();
        cx.spawn(async move |this, cx| {
            Timer::after(Duration::from_millis(260)).await;
            let _ = this.update(cx, |this, cx| {
                if this.delete_generation != generation {
                    return;
                }
                this.delete_target = None;
                this.delete_error = None;
                this.delete_closing = false;
                cx.notify();
            });
        })
        .detach();
    }

    fn delete_instance(&mut self, cx: &mut Context<Self>) {
        if self.delete_busy {
            return;
        }
        let Some(server) = self.delete_target.clone() else {
            return;
        };
        let id = server.id.clone();
        let runtime = cx.global::<AppState>().background_runtime.clone();
        self.delete_busy = true;
        self.delete_error = None;
        cx.notify();
        let task = runtime.spawn(async move { server.delete().await });
        cx.spawn(async move |this, cx| {
            let result = task
                .await
                .map_err(|error| format!("Instance deletion task failed: {error}"))
                .and_then(|result| result.map_err(|error| format!("{error:#}")));
            let _ = this.update(cx, |this, cx| {
                this.delete_busy = false;
                match result {
                    Ok(()) => {
                        cx.update_global::<AppState, _>(|state, _| {
                            state.instance_service.remove(&id);
                        });
                        this.screens.remove(&id);
                        this.subscriptions.remove(&id);
                        if this.selected.as_deref() == Some(id.as_str()) {
                            this.selected = None;
                        }
                        this.close_delete_dialog(cx);
                    }
                    Err(error) => this.delete_error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn open_create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.modal.is_some() {
            return;
        }
        self.window = Some(window.window_handle());
        self.closing = false;
        let modal = cx.new(CreateServerModal::new);
        let focus = modal.read(cx).focus.clone();
        window.focus(&focus, cx);
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
                let _ = window.update(cx, |_, window, cx| window.focus(&focus, cx));
            }
        })
        .detach();
    }
    fn create_overlay(
        &self,
        modal: Entity<CreateServerModal>,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let palette = cx.global::<AppState>().theme_manager.palette();
        let bounds = self.bounds.get();
        let width = (bounds.size.width - px(24.)).min(px(560.)).max(px(1.));
        let height = (bounds.size.height - px(24.)).min(px(500.)).max(px(1.));
        let target = Bounds::new(
            point(
                (bounds.size.width - width) / 2.,
                (bounds.size.height - height) / 2.,
            ),
            size(width, height),
        );
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
                    .id("create-dialog")
                    .absolute()
                    .left(target.origin.x)
                    .top(target.origin.y)
                    .w(width)
                    .h(height)
                    .overflow_hidden()
                    .rounded_lg()
                    .bg(palette.background)
                    .border_1()
                    .border_color(palette.border)
                    .shadow_lg()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(div().size_full().child(modal))
                    .with_animation(
                        ("create-dialog-fade", usize::from(closing)),
                        Animation::new(Duration::from_millis(300)).with_easing(ease_out_quint()),
                        move |dialog, progress| {
                            let visibility = if closing { 1. - progress } else { progress };
                            dialog
                                .top(target.origin.y + px(10. * (1. - visibility)))
                                .opacity(visibility)
                        },
                    ),
            )
    }

    fn delete_overlay(&self, server: ServerInstance, cx: &Context<Self>) -> impl IntoElement {
        let palette = cx.global::<AppState>().theme_manager.palette();
        let busy = self.delete_busy;
        let closing = self.delete_closing;
        let error = self.delete_error.clone();
        let entity = cx.entity();
        let cancel_entity = entity.clone();
        let backdrop_entity = entity.clone();
        div()
            .id("instance-delete-overlay")
            .absolute()
            .inset_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x00000099))
            .occlude()
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                cx.stop_propagation();
                backdrop_entity.update(cx, |this, cx| this.close_delete_dialog(cx));
            })
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .w_full()
                    .max_w(px(460.))
                    .mx_4()
                    .p_5()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .rounded_lg()
                    .border_1()
                    .border_color(palette.border)
                    .bg(palette.background)
                    .shadow_lg()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .child("Delete instance?"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(palette.muted)
                            .child(format!(
                                "This permanently deletes {} and all its server files. This cannot be undone. The server will stop first if it is running.",
                                server.name
                            )),
                    )
                    .when_some(error, |panel, error| {
                        panel.child(
                            div()
                                .text_sm()
                                .text_color(gpui::rgb(0xe06c75))
                                .child(error),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .justify_end()
                            .gap_2()
                            .child(
                                button("cancel-instance-delete", "Cancel", palette, false, !busy)
                                    .when(!busy, |button| {
                                        button.cursor_pointer().on_click(move |_, _, cx| {
                                            cx.stop_propagation();
                                            cancel_entity.update(cx, |this, cx| {
                                                this.close_delete_dialog(cx)
                                            });
                                        })
                                    }),
                            )
                            .child(
                                div()
                                    .id("confirm-instance-delete")
                                    .px_4()
                                    .py_2()
                                    .rounded_md()
                                    .bg(gpui::rgb(0xc94b4b))
                                    .text_color(gpui::rgb(0xffffff))
                                    .when(busy, |button| button.opacity(0.6))
                                    .when(!busy, |button| {
                                        button.cursor_pointer().hover(|style| {
                                            style.bg(gpui::rgb(0xb63d3d))
                                        })
                                    })
                                    .on_click(move |_, _, cx| {
                                        cx.stop_propagation();
                                        if !busy {
                                            entity.update(cx, |this, cx| {
                                                this.delete_instance(cx)
                                            });
                                        }
                                    })
                                    .child(if busy {
                                        "Deleting…"
                                    } else {
                                        "Delete permanently"
                                    }),
                            ),
                    )
                    .with_animation(
                        ("instance-delete-panel", usize::from(closing)),
                        Animation::new(Duration::from_millis(260)).with_easing(ease_out_quint()),
                        move |panel, progress| {
                            panel
                                .relative()
                                .top(px(10. * (1. - progress)))
                                .opacity(if closing { 1. - progress } else { progress })
                        },
                    ),
            )
            .with_animation(
                ("instance-delete-backdrop", usize::from(closing)),
                Animation::new(Duration::from_millis(260)),
                move |backdrop, progress| {
                    backdrop.opacity(if closing { 1. - progress } else { progress })
                },
            )
    }
}
impl Render for Servers {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.global::<AppState>().theme_manager.palette();
        let bounds = self.bounds.clone();
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
        let entity = cx.entity();
        let instance_menu = self.instance_menu.as_ref().and_then(|(id, position)| {
            let server = servers
                .iter()
                .find(|server| server.id == *id)
                .map(|server| (*server).clone())?;
            let delete_entity = entity.clone();
            Some(
                anchored()
                    .position(point(position.x - px(168.), position.y + px(8.)))
                    .anchor(Anchor::TopLeft)
                    .snap_to_window_with_margin(gpui::Edges::all(px(8.)))
                    .child(
                        div()
                            .w(px(168.))
                            .p_1()
                            .flex()
                            .flex_col()
                            .rounded_md()
                            .border_1()
                            .border_color(palette.border)
                            .bg(palette.background)
                            .occlude()
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .child(
                                div()
                                    .id("delete-server-menu-item")
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .px_3()
                                    .py_2()
                                    .rounded_md()
                                    .text_color(gpui::rgb(0xe06c75))
                                    .cursor_pointer()
                                    .hover(|style| style.bg(palette.surface))
                                    .on_click(move |_, _, cx| {
                                        cx.stop_propagation();
                                        delete_entity.update(cx, |this, cx| {
                                            this.open_delete_dialog(server.clone(), cx);
                                        });
                                    })
                                    .child(
                                        svg()
                                            .path("icons/trash.svg")
                                            .size_4()
                                            .text_color(gpui::rgb(0xe06c75)),
                                    )
                                    .child("Delete instance"),
                            ),
                    )
                    .into_any_element(),
            )
        });
        let delete_overlay = self
            .delete_target
            .clone()
            .map(|server| self.delete_overlay(server, cx));
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
                                            true,
                                        )
                                        .relative()
                                        .track_focus(&self.focus)
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.open_create(window, cx)
                                        }))
                                        .on_key_down(
                                            cx.listener(
                                                |this, event: &gpui::KeyDownEvent, window, cx| {
                                                    if matches!(
                                                        event.keystroke.key.as_str(),
                                                        "enter" | "space"
                                                    ) {
                                                        this.open_create(window, cx);
                                                        cx.stop_propagation();
                                                    }
                                                },
                                            ),
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
                                        let menu_entity = entity.clone();
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
                                                                "{} · {} · {}",
                                                                server.version,
                                                                server.software,
                                                                if services::software::pumpkin::is_pumpkin(&server.software) {
                                                                    "Native runtime".to_owned()
                                                                } else {
                                                                    format!("{} MiB", server.ram)
                                                                }
                                                            )),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap_3()
                                                    .flex_shrink_0()
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .text_color(palette.muted)
                                                            .child(if running {
                                                                "Running"
                                                            } else {
                                                                "Stopped"
                                                            }),
                                                    )
                                                    .child(
                                                        div()
                                                            .id(format!("server-actions-{id}"))
                                                            .size_8()
                                                            .flex()
                                                            .items_center()
                                                            .justify_center()
                                                            .rounded_md()
                                                            .cursor_pointer()
                                                            .text_color(palette.muted)
                                                            .hover(|style| {
                                                                style
                                                                    .bg(palette.background)
                                                                    .text_color(palette.text)
                                                            })
                                                            .on_mouse_down(
                                                                MouseButton::Left,
                                                                |_, _, cx| cx.stop_propagation(),
                                                            )
                                                            .on_click(move |event, _, cx| {
                                                                cx.stop_propagation();
                                                                let position = event.position();
                                                                menu_entity.update(
                                                                    cx,
                                                                    |this, cx| {
                                                                        this.instance_menu = if this
                                                                            .instance_menu
                                                                            .as_ref()
                                                                            .is_some_and(
                                                                                |(open_id, _)| {
                                                                                    open_id == &id
                                                                                },
                                                                            ) {
                                                                            None
                                                                        } else {
                                                                            Some((
                                                                                id.clone(),
                                                                                position,
                                                                            ))
                                                                        };
                                                                        cx.notify();
                                                                    },
                                                                );
                                                            })
                                                            .child(
                                                                svg()
                                                                    .path("icons/ellipsis.svg")
                                                                    .size_4()
                                                                    .text_color(palette.text),
                                                            ),
                                                    ),
                                            )
                                    })),
                            ),
                    )
                }
            })
            .when_some(self.modal.clone(), |root, modal| {
                root.child(self.create_overlay(modal, cx))
            })
            .when_some(instance_menu, |root, menu| root.child(menu))
            .when_some(delete_overlay, |root, overlay| root.child(overlay))
    }
}
