use std::collections::HashMap;

use gpui::{
    Anchor, AppContext, Context, Edges, Entity, FocusHandle, FontWeight, IntoElement, KeyDownEvent,
    MouseButton, Pixels, Point, Render, SharedString, Subscription, Window, anchored,
    container_query, div, point, prelude::*, px, svg,
};
use services::{AppState, instance::ServerInstance};
use ui::{
    components::{Button, ButtonVariants, IconButton, card_surface},
    theme::ActiveUiTheme,
};

use crate::{
    modals::{CreateServerModal, DeleteInstanceModal},
    widgets::{
        empty_state,
        server::screen::{Back, ServerScreen},
    },
};

pub struct Servers {
    screens: HashMap<String, Entity<ServerScreen>>,
    selected: Option<String>,
    subscriptions: HashMap<String, Subscription>,
    instance_menu: Option<(String, Point<Pixels>)>,
    focus: FocusHandle,
}

impl Servers {
    pub fn new(cx: &mut Context<Self>) -> Self {
        cx.observe_global::<AppState>(|_, cx| cx.notify()).detach();
        Self {
            screens: HashMap::new(),
            selected: None,
            subscriptions: HashMap::new(),
            instance_menu: None,
            focus: cx.focus_handle().tab_index(0),
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

    pub fn open_created_server(&mut self, server: ServerInstance, cx: &mut Context<Self>) {
        self.add(server.clone(), cx);
        self.selected = Some(server.id);
        cx.notify();
    }

    pub fn on_instance_deleted(&mut self, id: &str, cx: &mut Context<Self>) {
        self.screens.remove(id);
        self.subscriptions.remove(id);
        if self.selected.as_deref() == Some(id) {
            self.selected = None;
        }
        cx.notify();
    }
}

impl Render for Servers {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.ui_palette();
        let screen = self
            .selected
            .as_ref()
            .and_then(|id| self.screens.get(id))
            .cloned();

        if let Some(screen) = screen {
            return super::panel_outer(&palette)
                .child(super::panel_surface(&palette).child(screen))
                .into_any_element();
        }

        let mut servers = cx.global::<AppState>().instance_service.servers().to_vec();
        servers.sort_by(|a, b| a.name.cmp(&b.name));

        let entity = cx.entity();
        let instance_menu = self.instance_menu.as_ref().and_then(|(id, position)| {
            let server = servers.iter().find(|server| server.id == *id)?.clone();
            let menu_entity = entity.clone();
            Some(
                anchored()
                    .position(point(position.x - px(168.0), position.y + px(8.0)))
                    .anchor(Anchor::TopLeft)
                    .snap_to_window_with_margin(Edges::all(px(8.0)))
                    .child(
                        div()
                            .w(px(168.0))
                            .p_1()
                            .flex()
                            .flex_col()
                            .rounded(px(16.0))
                            .border_1()
                            .border_color(palette.border)
                            .bg(palette.surface)
                            .shadow_lg()
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
                                    .rounded(px(12.0))
                                    .text_xs()
                                    .text_color(palette.error)
                                    .cursor_pointer()
                                    .hover(|style| style.bg(palette.surface_hover))
                                    .on_click(move |_, window, cx| {
                                        cx.stop_propagation();
                                        menu_entity.update(cx, |this, cx| {
                                            this.instance_menu = None;
                                            cx.notify();
                                        });
                                        DeleteInstanceModal::open(server.clone(), window, cx);
                                    })
                                    .child(
                                        svg()
                                            .path("icons/trash.svg")
                                            .size_4()
                                            .text_color(palette.error),
                                    )
                                    .child("Delete instance"),
                            ),
                    )
                    .into_any_element(),
            )
        });

        let has_menu = self.instance_menu.is_some();
        let is_empty = servers.is_empty();
        let count_text = if servers.len() == 1 {
            "1 server".to_owned()
        } else {
            format!("{} servers", servers.len())
        };
        let running_map: HashMap<String, bool> = servers
            .iter()
            .map(|server| {
                let running = self
                    .screens
                    .get(&server.id)
                    .is_some_and(|screen| screen.read(cx).running);
                (server.id.clone(), running)
            })
            .collect();

        let header_actions = div()
            .flex()
            .items_center()
            .gap_2()
            .child(super::count_badge(count_text, &palette))
            .child(
                div()
                    .track_focus(&self.focus)
                    .on_key_down(|event: &KeyDownEvent, window, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            CreateServerModal::open(window, cx);
                            cx.stop_propagation();
                        }
                    })
                    .child(
                        Button::new("create-server-button")
                            .label("Create server")
                            .primary()
                            .rounded(px(16.0))
                            .h(px(32.0))
                            .on_click(|_, window, cx| {
                                CreateServerModal::open(window, cx);
                            }),
                    ),
            );

        let grid_entity = entity.clone();
        let list_content = if is_empty {
            empty_state::render_icon_with_action(
                "icons/server.svg",
                "No servers yet",
                "Create your first Minecraft server to get started.",
                &palette,
                Button::new("empty-create-server-button")
                    .label("Create server")
                    .primary()
                    .rounded(px(16.0))
                    .h(px(32.0))
                    .on_click(|_, window, cx| {
                        CreateServerModal::open(window, cx);
                    }),
            )
            .into_any_element()
        } else {
            let cards = servers;
            let surface = palette.surface;
            let surface_hover = palette.surface_hover;
            let border = palette.border;
            let text_color = palette.text;
            let muted = palette.muted;
            let running_color = palette.running;

            container_query(move |size, _window, _cx| {
                let columns = (((f32::from(size.width) + 16.0) / 312.0).floor() as u16).max(1);
                div()
                    .w_full()
                    .grid()
                    .grid_cols(columns)
                    .content_start()
                    .items_start()
                    .gap_4()
                    .children(cards.iter().map(|server| {
                        let id = server.id.clone();
                        let menu_id = id.clone();
                        let open_server = server.clone();
                        let click_entity = grid_entity.clone();
                        let menu_entity = grid_entity.clone();
                        let running = running_map.get(&id).copied().unwrap_or(false);
                        let runtime_label =
                            if services::software::pumpkin::is_pumpkin(&server.software) {
                                "Native runtime".to_owned()
                            } else {
                                format!("{} MiB", server.ram)
                            };
                        let metadata = format!(
                            "{} · {} · {}",
                            server.version, server.software, runtime_label
                        );

                        card_surface(surface, surface_hover, border)
                            .id(SharedString::from(id.clone()))
                            .w_full()
                            .min_w_0()
                            .p_4()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .cursor_pointer()
                            .on_click(move |_, _, cx| {
                                click_entity.update(cx, |this, cx| {
                                    this.open_server(open_server.clone(), cx);
                                });
                            })
                            .child(
                                div()
                                    .flex()
                                    .items_start()
                                    .justify_between()
                                    .gap_2()
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .min_w_0()
                                            .flex_1()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .font_weight(FontWeight::SEMIBOLD)
                                                    .text_color(text_color)
                                                    .truncate()
                                                    .child(server.name.clone()),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(muted)
                                                    .truncate()
                                                    .child(metadata),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex_shrink_0()
                                            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                                cx.stop_propagation();
                                            })
                                            .child(
                                                IconButton::new(
                                                    SharedString::from(format!(
                                                        "server-actions-{id}"
                                                    )),
                                                    "icons/ellipsis.svg",
                                                )
                                                .tooltip("Instance actions")
                                                .on_click(move |event, _, cx| {
                                                    cx.stop_propagation();
                                                    let position = event.position();
                                                    let target_id = menu_id.clone();
                                                    menu_entity.update(cx, |this, cx| {
                                                        this.instance_menu = if this
                                                            .instance_menu
                                                            .as_ref()
                                                            .is_some_and(|(open_id, _)| {
                                                                open_id == &target_id
                                                            }) {
                                                            None
                                                        } else {
                                                            Some((target_id, position))
                                                        };
                                                        cx.notify();
                                                    });
                                                }),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(div().size(px(8.0)).rounded_full().bg(if running {
                                        running_color
                                    } else {
                                        muted
                                    }))
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(if running { running_color } else { muted })
                                            .child(if running { "Running" } else { "Stopped" }),
                                    ),
                            )
                    }))
            })
            .into_any_element()
        };

        super::panel_outer(&palette)
            .child(
                super::panel_surface(&palette)
                    .when(has_menu, |panel| {
                        panel.on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                if this.instance_menu.take().is_some() {
                                    cx.notify();
                                }
                            }),
                        )
                    })
                    .child(
                        div()
                            .id("servers-list")
                            .flex_1()
                            .min_h_0()
                            .w_full()
                            .overflow_y_scroll()
                            .px_5()
                            .pt_5()
                            .pb_12()
                            .flex()
                            .flex_col()
                            .gap_5()
                            .child(super::section_header(
                                "Servers",
                                "Create and manage local Minecraft server instances.",
                                header_actions,
                                &palette,
                            ))
                            .child(list_content),
                    )
                    .when(!is_empty, |panel| {
                        panel
                            .child(super::top_scroll_fade(palette.surface))
                            .child(super::bottom_scroll_fade(palette.surface))
                    })
                    .when_some(instance_menu, |panel, menu| panel.child(menu)),
            )
            .into_any_element()
    }
}
