use std::{cell::Cell, collections::HashMap, rc::Rc};

use gpui::{
    Anchor, AppContext, Bounds, Context, Edges, Entity, FocusHandle, FontWeight, IntoElement,
    MouseButton, Pixels, Point, Render, ScrollDelta, SharedString, Subscription, Window, anchored,
    canvas, div, point, prelude::*, px, svg,
};
use services::{
    AppState,
    instance::ServerInstance,
    topology::{NodePosition, PinSide, ServerConnection, SpatialTopology},
};
use ui::{
    components::{Button, ButtonVariants, IconButton, card_surface},
    theme::ActiveUiTheme,
};

use super::spatial_canvas::{
    DraftRope, MAX_ZOOM, MIN_ZOOM, NODE_HEIGHT, NODE_WIDTH, PaintRope, fit_viewport, hit_test_rope,
    paint_canvas, pin_screen_position, rope_cubic,
};
use crate::{
    modals::{CreateServerModal, DeleteInstanceModal},
    widgets::{
        empty_state,
        server::screen::{Back, ServerScreen},
    },
};

#[derive(Clone, Debug, PartialEq)]
enum Interaction {
    Idle,
    Panning {
        start_cursor: Point<Pixels>,
        start_pan: (f32, f32),
    },
    DraggingNode {
        id: String,
        start_cursor: Point<Pixels>,
        start_pos: NodePosition,
        moved: bool,
    },
    Connecting {
        from_id: String,
        from_side: PinSide,
        cursor_screen: Point<Pixels>,
    },
}

pub struct Servers {
    screens: HashMap<String, Entity<ServerScreen>>,
    selected: Option<String>,
    subscriptions: HashMap<String, Vec<Subscription>>,
    instance_menu: Option<(String, Point<Pixels>)>,
    canvas_bounds: Rc<Cell<Bounds<Pixels>>>,
    interaction: Interaction,
    hovered_connection: Option<usize>,
    snap_target: Option<(String, PinSide)>,
    focus: FocusHandle,
}

impl Servers {
    pub fn new(cx: &mut Context<Self>) -> Self {
        cx.observe_global::<AppState>(|_, cx| cx.notify()).detach();
        let ordered_ids = sorted_server_ids(cx.global::<AppState>().instance_service.servers());
        cx.update_global::<AppState, _>(|state, _| {
            if state.topology.ensure_positions(&ordered_ids) {
                state.save_topology();
            }
        });
        Self {
            screens: HashMap::new(),
            selected: None,
            subscriptions: HashMap::new(),
            instance_menu: None,
            canvas_bounds: Rc::default(),
            interaction: Interaction::Idle,
            hovered_connection: None,
            snap_target: None,
            focus: cx.focus_handle().tab_index(0),
        }
    }

    pub fn active_screen(&self) -> Option<Entity<ServerScreen>> {
        self.selected
            .as_ref()
            .and_then(|id| self.screens.get(id))
            .cloned()
    }

    pub fn close_active_server(&mut self, cx: &mut Context<Self>) {
        if self.selected.take().is_some() {
            cx.notify();
        }
    }

    fn add(&mut self, server: ServerInstance, cx: &mut Context<Self>) {
        if self.screens.contains_key(&server.id) {
            return;
        }
        let id = server.id.clone();
        let screen = cx.new(|cx| ServerScreen::new(server, cx));
        let back_sub = cx.subscribe(&screen, |this, _, _: &Back, cx| {
            this.selected = None;
            cx.notify();
        });
        let state_sub = cx.observe(&screen, |_, _, cx| cx.notify());
        self.subscriptions
            .insert(id.clone(), vec![back_sub, state_sub]);
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
        let ordered_ids = sorted_server_ids(cx.global::<AppState>().instance_service.servers());
        cx.update_global::<AppState, _>(|state, _| {
            if state.topology.ensure_positions(&ordered_ids) {
                state.save_topology();
            }
        });
        self.selected = Some(server.id);
        cx.notify();
    }

    pub fn on_instance_deleted(&mut self, id: &str, cx: &mut Context<Self>) {
        self.screens.remove(id);
        self.subscriptions.remove(id);
        if self.selected.as_deref() == Some(id) {
            self.selected = None;
        }
        if self
            .instance_menu
            .as_ref()
            .is_some_and(|(open_id, _)| open_id == id)
        {
            self.instance_menu = None;
        }
        cx.update_global::<AppState, _>(|state, _| {
            if state.topology.remove_server(id) {
                state.save_topology();
            }
        });
        cx.notify();
    }

    fn resolved_topology(
        &self,
        cx: &AppState,
    ) -> (
        Vec<ServerInstance>,
        HashMap<String, NodePosition>,
        SpatialTopology,
    ) {
        let mut servers = cx.instance_service.servers().to_vec();
        servers.sort_by(|a, b| a.name.cmp(&b.name));
        let positions = resolve_positions(&servers, &cx.topology.positions);
        let topology = cx.topology.clone();
        (servers, positions, topology)
    }

    fn hit_connection_index(&self, cursor: Point<Pixels>, cx: &AppState) -> Option<usize> {
        let (_, positions, topology) = self.resolved_topology(cx);
        let origin = self.canvas_bounds.get().origin;
        for (index, connection) in topology.connections.iter().enumerate() {
            let (Some(&from_pos), Some(&to_pos)) = (
                positions.get(&connection.from_id),
                positions.get(&connection.to_id),
            ) else {
                continue;
            };
            let from = pin_screen_position(
                from_pos,
                connection.from_side,
                topology.pan_x,
                topology.pan_y,
                topology.zoom,
                origin,
            );
            let to = pin_screen_position(
                to_pos,
                connection.to_side,
                topology.pan_x,
                topology.pan_y,
                topology.zoom,
                origin,
            );
            let (p0, c1, c2, p3) = rope_cubic(
                from,
                connection.from_side,
                to,
                Some(connection.to_side),
                topology.zoom,
            );
            if hit_test_rope(
                cursor,
                p0,
                c1,
                c2,
                p3,
                (9.0 * topology.zoom).clamp(7.0, 14.0),
            ) {
                return Some(index);
            }
        }
        None
    }

    fn find_snap_target(
        &self,
        from_id: &str,
        cursor: Point<Pixels>,
        cx: &AppState,
    ) -> Option<(String, PinSide)> {
        let (servers, positions, topology) = self.resolved_topology(cx);
        let origin = self.canvas_bounds.get().origin;
        let snap_radius_sq = (28.0_f32).powi(2);
        let mut best: Option<(String, PinSide, f32)> = None;

        for server in &servers {
            if server.id == from_id {
                continue;
            }
            let Some(&pos) = positions.get(&server.id) else {
                continue;
            };
            for side in [PinSide::Left, PinSide::Right] {
                let pin_pos = pin_screen_position(
                    pos,
                    side,
                    topology.pan_x,
                    topology.pan_y,
                    topology.zoom,
                    origin,
                );
                let dx = f32::from(cursor.x - pin_pos.x);
                let dy = f32::from(cursor.y - pin_pos.y);
                let dist_sq = dx * dx + dy * dy;
                if dist_sq <= snap_radius_sq
                    && best
                        .as_ref()
                        .is_none_or(|(_, _, best_sq)| dist_sq < *best_sq)
                {
                    best = Some((server.id.clone(), side, dist_sq));
                }
            }
        }

        if let Some((id, side, _)) = best {
            return Some((id, side));
        }

        for server in &servers {
            if server.id == from_id {
                continue;
            }
            let Some(&pos) = positions.get(&server.id) else {
                continue;
            };
            let left = f32::from(origin.x) + topology.pan_x + pos.x * topology.zoom;
            let top = f32::from(origin.y) + topology.pan_y + pos.y * topology.zoom;
            let width = NODE_WIDTH * topology.zoom;
            let height = NODE_HEIGHT * topology.zoom;
            let cx_pt = f32::from(cursor.x);
            let cy_pt = f32::from(cursor.y);
            if cx_pt >= left && cx_pt <= left + width && cy_pt >= top && cy_pt <= top + height {
                let side = if cx_pt < left + width * 0.5 {
                    PinSide::Left
                } else {
                    PinSide::Right
                };
                return Some((server.id.clone(), side));
            }
        }

        None
    }

    fn apply_zoom(&mut self, next_zoom: f32, anchor: Point<Pixels>, cx: &mut Context<Self>) {
        let clamped = next_zoom.clamp(MIN_ZOOM, MAX_ZOOM);
        let origin = self.canvas_bounds.get().origin;
        cx.update_global::<AppState, _>(|state, _| {
            let old_zoom = state.topology.zoom;
            if (old_zoom - clamped).abs() <= f32::EPSILON {
                return;
            }
            let local_x = f32::from(anchor.x - origin.x);
            let local_y = f32::from(anchor.y - origin.y);
            let world_x = (local_x - state.topology.pan_x) / old_zoom;
            let world_y = (local_y - state.topology.pan_y) / old_zoom;
            state.topology.zoom = clamped;
            state.topology.pan_x = local_x - world_x * clamped;
            state.topology.pan_y = local_y - world_y * clamped;
            state.save_topology();
        });
        cx.notify();
    }

    fn fit_all(&mut self, cx: &mut Context<Self>) {
        let bounds = self.canvas_bounds.get();
        let state = cx.global::<AppState>();
        let (servers, positions, _) = self.resolved_topology(state);
        if servers.is_empty() {
            return;
        }
        let ordered_positions: Vec<_> = servers
            .iter()
            .filter_map(|server| positions.get(&server.id))
            .collect();
        let (pan_x, pan_y, zoom) = fit_viewport(ordered_positions.into_iter(), bounds.size);
        cx.update_global::<AppState, _>(|state, _| {
            state.topology.pan_x = pan_x;
            state.topology.pan_y = pan_y;
            state.topology.zoom = zoom;
            state.save_topology();
        });
        cx.notify();
    }
}

impl Render for Servers {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.ui_palette();
        if let Some(screen) = self.active_screen() {
            return super::canvas_outer(&palette)
                .child(screen)
                .into_any_element();
        }

        let (servers, positions, topology) = self.resolved_topology(cx.global::<AppState>());
        let zoom = topology.zoom;
        let pan_x = topology.pan_x;
        let pan_y = topology.pan_y;
        let origin = self.canvas_bounds.get().origin;

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

        let ropes: Vec<PaintRope> = topology
            .connections
            .iter()
            .enumerate()
            .filter_map(|(index, connection)| {
                let from_pos = positions.get(&connection.from_id).copied()?;
                let to_pos = positions.get(&connection.to_id).copied()?;
                let active = running_map
                    .get(&connection.from_id)
                    .copied()
                    .unwrap_or(false)
                    || running_map.get(&connection.to_id).copied().unwrap_or(false);
                Some(PaintRope {
                    from: pin_screen_position(
                        from_pos,
                        connection.from_side,
                        pan_x,
                        pan_y,
                        zoom,
                        origin,
                    ),
                    from_side: connection.from_side,
                    to: pin_screen_position(to_pos, connection.to_side, pan_x, pan_y, zoom, origin),
                    to_side: connection.to_side,
                    active,
                    hovered: self.hovered_connection == Some(index),
                })
            })
            .collect();

        let draft_rope = match &self.interaction {
            Interaction::Connecting {
                from_id,
                from_side,
                cursor_screen,
            } => positions.get(from_id).copied().map(|from_pos| {
                let (target_cursor, snap_side) = match self.snap_target.as_ref() {
                    Some((target_id, target_side)) => positions
                        .get(target_id)
                        .copied()
                        .map(|pos| {
                            (
                                pin_screen_position(pos, *target_side, pan_x, pan_y, zoom, origin),
                                Some(*target_side),
                            )
                        })
                        .unwrap_or((*cursor_screen, None)),
                    None => (*cursor_screen, None),
                };
                DraftRope {
                    from: pin_screen_position(from_pos, *from_side, pan_x, pan_y, zoom, origin),
                    from_side: *from_side,
                    cursor: target_cursor,
                    snap_side,
                }
            }),
            _ => None,
        };

        let bounds_cell = self.canvas_bounds.clone();
        let paint_palette = palette;
        let canvas_bg = canvas(
            move |bounds, _, _| {
                bounds_cell.set(bounds);
            },
            move |bounds, _, window, _| {
                paint_canvas(
                    bounds,
                    pan_x,
                    pan_y,
                    zoom,
                    paint_palette,
                    &ropes,
                    draft_rope,
                    window,
                );
            },
        )
        .absolute()
        .size_full();

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

        let is_empty = servers.is_empty();
        let node_w = NODE_WIDTH * zoom;
        let node_h = NODE_HEIGHT * zoom;
        let node_radius = (16.0 * zoom).clamp(8.0, 20.0);
        let node_pad = (14.0 * zoom).clamp(7.0, 20.0);
        let pin_size = (14.0 * zoom).clamp(10.0, 18.0);
        let show_actions = zoom >= 0.6;
        let show_meta = zoom >= 0.55;
        let active_drag_id = match &self.interaction {
            Interaction::DraggingNode {
                id, moved: true, ..
            } => Some(id.clone()),
            _ => None,
        };
        let connecting_from = match &self.interaction {
            Interaction::Connecting {
                from_id, from_side, ..
            } => Some((from_id.clone(), *from_side)),
            _ => None,
        };

        let zoom_percent = format!("{}%", (zoom * 100.0).round() as u32);
        let zoom_in_val = (zoom * 1.15).clamp(MIN_ZOOM, MAX_ZOOM);
        let zoom_out_val = (zoom / 1.15).clamp(MIN_ZOOM, MAX_ZOOM);

        let hud = div()
            .absolute()
            .right_4()
            .bottom_4()
            .flex()
            .items_center()
            .gap_1()
            .p_1()
            .rounded(px(16.0))
            .bg(palette.surface)
            .border_1()
            .border_color(palette.border)
            .shadow_lg()
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_scroll(|_, _, cx| cx.stop_propagation())
            .child(
                IconButton::new("canvas-zoom-out", "icons/minus.svg")
                    .size(px(28.0))
                    .icon_size(px(14.0))
                    .tooltip("Zoom out")
                    .disabled(zoom <= MIN_ZOOM + 0.01)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let center = this.canvas_bounds.get().center();
                        this.apply_zoom(zoom_out_val, center, cx);
                    })),
            )
            .child(
                div()
                    .id("canvas-zoom-reset")
                    .px_2p5()
                    .h(px(28.0))
                    .rounded(px(12.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(palette.text)
                    .cursor_pointer()
                    .hover(|s| s.bg(palette.surface_hover))
                    .on_click(cx.listener(|this, _, _, cx| {
                        let center = this.canvas_bounds.get().center();
                        this.apply_zoom(1.0, center, cx);
                    }))
                    .child(zoom_percent),
            )
            .child(
                IconButton::new("canvas-zoom-in", "icons/plus.svg")
                    .size(px(28.0))
                    .icon_size(px(14.0))
                    .tooltip("Zoom in")
                    .disabled(zoom >= MAX_ZOOM - 0.01)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let center = this.canvas_bounds.get().center();
                        this.apply_zoom(zoom_in_val, center, cx);
                    })),
            )
            .child(div().w(px(1.0)).h(px(16.0)).bg(palette.border).opacity(0.6))
            .child(
                div()
                    .id("canvas-zoom-fit")
                    .px_2p5()
                    .h(px(28.0))
                    .rounded(px(12.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(palette.muted)
                    .cursor_pointer()
                    .hover(|s| s.bg(palette.surface_hover).text_color(palette.text))
                    .on_click(cx.listener(|this, _, _, cx| this.fit_all(cx)))
                    .child("Fit"),
            );

        super::canvas_outer(&palette)
            .child(
                div()
                    .id("servers-spatial-canvas")
                    .relative()
                    .flex_1()
                    .size_full()
                    .overflow_hidden()
                    .track_focus(&self.focus)
                    .on_scroll(cx.listener(|this, event: &gpui::ScrollEvent, _, cx| {
                        if this.selected.is_some() {
                            return;
                        }
                        let delta_y = match event.delta {
                            ScrollDelta::Pixels(delta) => f32::from(delta.y),
                            ScrollDelta::Lines(delta) => delta.y * 28.0,
                        };
                        if delta_y.abs() <= f32::EPSILON {
                            return;
                        }
                        let factor = if delta_y > 0.0 { 1.12 } else { 1.0 / 1.12 };
                        let old_zoom = cx.global::<AppState>().topology.zoom;
                        this.apply_zoom(old_zoom * factor, event.position, cx);
                    }))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, event: &gpui::MouseDownEvent, _, cx| {
                            let had_menu = this.instance_menu.take().is_some();
                            if let Some(hit_idx) =
                                this.hit_connection_index(event.position, cx.global::<AppState>())
                            {
                                cx.update_global::<AppState, _>(|state, _| {
                                    if state.topology.remove_connection(hit_idx) {
                                        state.save_topology();
                                    }
                                });
                                this.hovered_connection = None;
                                cx.notify();
                                return;
                            }
                            let topo = &cx.global::<AppState>().topology;
                            this.interaction = Interaction::Panning {
                                start_cursor: event.position,
                                start_pan: (topo.pan_x, topo.pan_y),
                            };
                            if had_menu {
                                cx.notify();
                            }
                        }),
                    )
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(|this, event: &gpui::MouseDownEvent, _, cx| {
                            if let Some(hit_idx) =
                                this.hit_connection_index(event.position, cx.global::<AppState>())
                            {
                                cx.update_global::<AppState, _>(|state, _| {
                                    if state.topology.remove_connection(hit_idx) {
                                        state.save_topology();
                                    }
                                });
                                this.hovered_connection = None;
                                cx.notify();
                            }
                        }),
                    )
                    .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _, cx| {
                        match this.interaction.clone() {
                            Interaction::Panning {
                                start_cursor,
                                start_pan,
                            } => {
                                let dx = f32::from(event.position.x - start_cursor.x);
                                let dy = f32::from(event.position.y - start_cursor.y);
                                cx.update_global::<AppState, _>(|state, _| {
                                    state.topology.pan_x = start_pan.0 + dx;
                                    state.topology.pan_y = start_pan.1 + dy;
                                });
                                cx.notify();
                            }
                            Interaction::DraggingNode {
                                id,
                                start_cursor,
                                start_pos,
                                moved,
                            } => {
                                let dx_px = f32::from(event.position.x - start_cursor.x);
                                let dy_px = f32::from(event.position.y - start_cursor.y);
                                let is_moved = moved || (dx_px * dx_px + dy_px * dy_px) >= 25.0;
                                if is_moved {
                                    let zoom = cx.global::<AppState>().topology.zoom;
                                    let ordered_ids = sorted_server_ids(
                                        cx.global::<AppState>().instance_service.servers(),
                                    );
                                    cx.update_global::<AppState, _>(|state, _| {
                                        state.topology.ensure_positions(&ordered_ids);
                                        state.topology.positions.insert(
                                            id.clone(),
                                            NodePosition {
                                                x: start_pos.x + dx_px / zoom,
                                                y: start_pos.y + dy_px / zoom,
                                            },
                                        );
                                    });
                                }
                                this.interaction = Interaction::DraggingNode {
                                    id,
                                    start_cursor,
                                    start_pos,
                                    moved: is_moved,
                                };
                                cx.notify();
                            }
                            Interaction::Connecting {
                                from_id, from_side, ..
                            } => {
                                let target = this.find_snap_target(
                                    &from_id,
                                    event.position,
                                    cx.global::<AppState>(),
                                );
                                this.snap_target = target;
                                this.interaction = Interaction::Connecting {
                                    from_id,
                                    from_side,
                                    cursor_screen: event.position,
                                };
                                cx.notify();
                            }
                            Interaction::Idle => {
                                let hovered = this
                                    .hit_connection_index(event.position, cx.global::<AppState>());
                                if hovered != this.hovered_connection {
                                    this.hovered_connection = hovered;
                                    cx.notify();
                                }
                            }
                        }
                    }))
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            let previous =
                                std::mem::replace(&mut this.interaction, Interaction::Idle);
                            match previous {
                                Interaction::Panning { .. } => {
                                    cx.update_global::<AppState, _>(|state, _| {
                                        state.save_topology();
                                    });
                                }
                                Interaction::DraggingNode { id, moved, .. } => {
                                    if moved {
                                        cx.update_global::<AppState, _>(|state, _| {
                                            state.save_topology();
                                        });
                                    } else if let Some(server) = cx
                                        .global::<AppState>()
                                        .instance_service
                                        .servers()
                                        .iter()
                                        .find(|server| server.id == id)
                                        .cloned()
                                    {
                                        this.open_server(server, cx);
                                    }
                                    cx.notify();
                                }
                                Interaction::Connecting {
                                    from_id, from_side, ..
                                } => {
                                    if let Some((to_id, to_side)) = this.snap_target.take() {
                                        let ordered_ids = sorted_server_ids(
                                            cx.global::<AppState>().instance_service.servers(),
                                        );
                                        cx.update_global::<AppState, _>(|state, _| {
                                            state.topology.ensure_positions(&ordered_ids);
                                            if state.topology.add_connection(ServerConnection {
                                                from_id,
                                                from_side,
                                                to_id,
                                                to_side,
                                            }) {
                                                state.save_topology();
                                            }
                                        });
                                    }
                                    cx.notify();
                                }
                                Interaction::Idle => {}
                            }
                        }),
                    )
                    .child(canvas_bg)
                    .when(is_empty, |canvas_el| {
                        canvas_el.child(
                            div().absolute().inset_0().flex().items_center().justify_center().p_6().child(
                                div().w(px(420.0)).max_w_full().child(
                                    empty_state::render_icon_with_action(
                                        "icons/server.svg",
                                        "No servers yet",
                                        "Create your first Minecraft server and arrange instances in your spatial workspace.",
                                        &palette,
                                        Button::new("empty-create-server-button")
                                            .label("Create server")
                                            .primary()
                                            .rounded(px(16.0))
                                            .h(px(32.0))
                                            .on_click(|_, window, cx| {
                                                CreateServerModal::open(window, cx);
                                            }),
                                    ),
                                ),
                            ),
                        )
                    })
                    .when(!is_empty, |canvas_el| {
                        canvas_el.children(servers.iter().filter_map(|server| {
                            let id = server.id.clone();
                            let pos = *positions.get(&id)?;
                            let left = pan_x + pos.x * zoom;
                            let top = pan_y + pos.y * zoom;
                            let running = running_map.get(&id).copied().unwrap_or(false);
                            let is_dragging = active_drag_id.as_deref() == Some(id.as_str());
                            let is_snap_left = self.snap_target.as_ref()
                                == Some(&(id.clone(), PinSide::Left));
                            let is_snap_right = self.snap_target.as_ref()
                                == Some(&(id.clone(), PinSide::Right));
                            let is_from_left = connecting_from.as_ref()
                                == Some(&(id.clone(), PinSide::Left));
                            let is_from_right = connecting_from.as_ref()
                                == Some(&(id.clone(), PinSide::Right));

                            let border = if is_snap_left || is_snap_right || is_dragging {
                                palette.accent
                            } else if running {
                                palette.running.opacity(0.65)
                            } else {
                                palette.border
                            };

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

                            let drag_id = id.clone();
                            let menu_id = id.clone();
                            let menu_entity = entity.clone();

                            Some(
                                card_surface(palette.surface, palette.surface_hover, border)
                                    .id(SharedString::from(id.clone()))
                                    .absolute()
                                    .left(px(left))
                                    .top(px(top))
                                    .w(px(node_w))
                                    .h(px(node_h))
                                    .rounded(px(node_radius))
                                    .p(px(node_pad))
                                    .flex()
                                    .flex_col()
                                    .justify_between()
                                    .cursor(if is_dragging {
                                        gpui::CursorStyle::ClosedHand
                                    } else {
                                        gpui::CursorStyle::PointingHand
                                    })
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                                            cx.stop_propagation();
                                            this.instance_menu = None;
                                            this.interaction = Interaction::DraggingNode {
                                                id: drag_id.clone(),
                                                start_cursor: event.position,
                                                start_pos: pos,
                                                moved: false,
                                            };
                                            cx.notify();
                                        }),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_start()
                                            .justify_between()
                                            .gap_2()
                                            .min_w_0()
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    .gap_1()
                                                    .min_w_0()
                                                    .flex_1()
                                                    .child(
                                                        div()
                                                            .text_size(px((14.0 * zoom).clamp(11.0, 18.0)))
                                                            .font_weight(FontWeight::SEMIBOLD)
                                                            .text_color(palette.text)
                                                            .truncate()
                                                            .child(server.name.clone()),
                                                    )
                                                    .when(show_meta, |col| {
                                                        col.child(
                                                            div()
                                                                .text_size(px((11.5 * zoom).clamp(10.0, 14.0)))
                                                                .text_color(palette.muted)
                                                                .truncate()
                                                                .child(metadata),
                                                        )
                                                    }),
                                            )
                                            .when(show_actions, |row| {
                                                row.child(
                                                    div()
                                                        .flex_shrink_0()
                                                        .on_mouse_down(
                                                            MouseButton::Left,
                                                            |_, _, cx| cx.stop_propagation(),
                                                        )
                                                        .child(
                                                            IconButton::new(
                                                                SharedString::from(format!(
                                                                    "server-actions-{id}"
                                                                )),
                                                                "icons/ellipsis.svg",
                                                            )
                                                            .size(px((28.0 * zoom).clamp(22.0, 32.0)))
                                                            .icon_size(px((15.0 * zoom).clamp(12.0, 18.0)))
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
                                                )
                                            }),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_1p5()
                                            .child(
                                                div()
                                                    .size(px((8.0 * zoom).clamp(6.0, 10.0)))
                                                    .rounded_full()
                                                    .bg(if running {
                                                        palette.running
                                                    } else {
                                                        palette.muted
                                                    }),
                                            )
                                            .when(show_meta, |status_row| {
                                                status_row.child(
                                                    div()
                                                        .text_size(px((11.5 * zoom).clamp(10.0, 14.0)))
                                                        .text_color(if running {
                                                            palette.running
                                                        } else {
                                                            palette.muted
                                                        })
                                                        .child(if running {
                                                            "Running"
                                                        } else {
                                                            "Stopped"
                                                        }),
                                                )
                                            }),
                                    )
                                    .child(node_pin(
                                        id.clone(),
                                        PinSide::Left,
                                        node_h,
                                        pin_size,
                                        is_snap_left || is_from_left,
                                        &palette,
                                        cx,
                                    ))
                                    .child(node_pin(
                                        id,
                                        PinSide::Right,
                                        node_h,
                                        pin_size,
                                        is_snap_right || is_from_right,
                                        &palette,
                                        cx,
                                    )),
                            )
                        }))
                    })
                    .when(self.hovered_connection.is_some(), |canvas_el| {
                        canvas_el.child(
                            div()
                                .absolute()
                                .left_4()
                                .bottom_4()
                                .px_3()
                                .py_1p5()
                                .rounded(px(16.0))
                                .bg(palette.surface)
                                .border_1()
                                .border_color(palette.error)
                                .shadow_md()
                                .text_xs()
                                .text_color(palette.error)
                                .child("Click rope to disconnect"),
                        )
                    })
                    .when(!is_empty, |canvas_el| canvas_el.child(hud))
                    .when_some(instance_menu, |canvas_el, menu| canvas_el.child(menu)),
            )
            .into_any_element()
    }
}

fn node_pin(
    server_id: String,
    side: PinSide,
    node_height: f32,
    pin_size: f32,
    highlighted: bool,
    palette: &ui::theme::Palette,
    cx: &mut Context<Servers>,
) -> impl IntoElement {
    let side_tag = match side {
        PinSide::Left => "left",
        PinSide::Right => "right",
    };
    let accent = palette.accent;
    let border = if highlighted { accent } else { palette.border };
    let fill = if highlighted {
        accent
    } else {
        palette.surface_hover
    };
    let pin_id: SharedString = format!("node-pin-{server_id}-{side_tag}").into();
    let pin_el = div()
        .id(pin_id)
        .absolute()
        .top(px(node_height * 0.5 - pin_size * 0.5))
        .size(px(pin_size))
        .rounded_full()
        .bg(fill)
        .border_1()
        .border_color(border)
        .cursor(gpui::CursorStyle::Crosshair)
        .hover(move |s| s.bg(accent).border_color(accent))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                cx.stop_propagation();
                this.instance_menu = None;
                this.snap_target = None;
                this.interaction = Interaction::Connecting {
                    from_id: server_id.clone(),
                    from_side: side,
                    cursor_screen: event.position,
                };
                cx.notify();
            }),
        );

    match side {
        PinSide::Left => pin_el.left(px(-pin_size * 0.5)),
        PinSide::Right => pin_el.right(px(-pin_size * 0.5)),
    }
}

fn sorted_server_ids(servers: &[ServerInstance]) -> Vec<String> {
    let mut sorted = servers.to_vec();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));
    sorted.into_iter().map(|server| server.id).collect()
}

fn resolve_positions(
    servers: &[ServerInstance],
    existing: &HashMap<String, NodePosition>,
) -> HashMap<String, NodePosition> {
    let mut resolved = existing.clone();
    let mut slot = 0usize;
    for server in servers {
        if resolved.contains_key(&server.id) {
            continue;
        }
        loop {
            let col = (slot % 3) as f32;
            let row = (slot / 3) as f32;
            let x = 80.0 + col * 340.0;
            let y = 80.0 + row * 190.0;
            slot += 1;
            let overlaps = resolved
                .values()
                .any(|pos| (pos.x - x).abs() < 160.0 && (pos.y - y).abs() < 90.0);
            if !overlaps {
                resolved.insert(server.id.clone(), NodePosition { x, y });
                break;
            }
        }
    }
    resolved
}
