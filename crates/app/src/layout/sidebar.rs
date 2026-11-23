use gpui::{prelude::FluentBuilder as _, *};
use ui::theme::ActiveUiTheme;

use crate::{
    layout::sidebar_item::NavigationItem,
    modals::{CreateServerModalState, DeleteInstanceModalState, SettingsModalState},
};

#[derive(IntoElement)]
pub(crate) struct Sidebar;

impl RenderOnce for Sidebar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let active = NavigationItem::active(window, cx);
        let palette = cx.ui_palette();

        div()
            .flex()
            .flex_col()
            .w(px(200.0))
            .h_full()
            .bg(palette.sidebar)
            .px_4()
            .pb_5()
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .pt_4()
                    .gap_3()
                    .child(
                        NavigationItem::Instances
                            .sidebar_button(active == Some(NavigationItem::Instances), cx),
                    )
                    .child(
                        NavigationItem::Runtimes
                            .sidebar_button(active == Some(NavigationItem::Runtimes), cx),
                    )
                    .child(
                        NavigationItem::Playit
                            .sidebar_button(active == Some(NavigationItem::Playit), cx),
                    ),
            )
            .child(
                div().flex().flex_col().gap_2().child(
                    NavigationItem::Settings
                        .sidebar_button(active == Some(NavigationItem::Settings), cx),
                ),
            )
    }
}

const ITEM_SIZE: Pixels = px(42.0);
const ITEM_GAP: Pixels = px(4.0);
const SELECTION_SIZE: Pixels = px(36.0);
const DOCK_MARGIN: Pixels = px(12.0);
const EDGE_HEIGHT: Pixels = px(8.0);
const REVEALED_HEIGHT: Pixels = px(70.0);

struct DockState {
    pointer_inside: bool,
    focus: FocusHandle,
}

fn observe_dock_pointer(state: Entity<DockState>) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, (), window, _| {
            let motion_state = state.clone();
            window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                if phase != DispatchPhase::Capture {
                    return;
                }

                let height = if motion_state.read(cx).pointer_inside {
                    REVEALED_HEIGHT
                } else {
                    EDGE_HEIGHT
                };
                let inside = !cx.has_active_drag()
                    && bounds.contains(&event.position)
                    && event.position.y >= bounds.bottom() - height;
                set_pointer_inside(&motion_state, inside, cx);
            });
            window.on_mouse_event(move |_: &MouseExitEvent, phase, _, cx| {
                if phase == DispatchPhase::Capture {
                    set_pointer_inside(&state, false, cx);
                }
            });
        },
    )
    .absolute()
    .inset_0()
}

fn set_pointer_inside(state: &Entity<DockState>, inside: bool, cx: &mut App) {
    if state.read(cx).pointer_inside != inside {
        state.update(cx, |state, cx| {
            state.pointer_inside = inside;
            cx.notify();
        });
    }
}

#[derive(IntoElement)]
pub(crate) struct Dock;

impl RenderOnce for Dock {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let labels = NavigationItem::ALL.map(NavigationItem::label);
        let state = window.use_keyed_state("navigation-dock-state", cx, |_, cx| DockState {
            pointer_inside: false,
            focus: cx.focus_handle(),
        });
        let dock_state = state.read(cx);
        let focus = dock_state.focus.clone();
        let keyboard_inside =
            window.last_input_was_keyboard() && focus.contains_focused(window, cx);
        let modal_open = cx
            .try_global::<SettingsModalState>()
            .is_some_and(|state| state.open)
            || cx
                .try_global::<CreateServerModalState>()
                .is_some_and(|state| state.is_open())
            || cx
                .try_global::<DeleteInstanceModalState>()
                .is_some_and(|state| state.open);
        let revealed = window.is_window_active()
            && !modal_open
            && (dock_state.pointer_inside || keyboard_inside);
        let reveal_spring = SpringAnimation::new(SpringConfig::new(300.0, 34.0, 1.0)).to(revealed);
        let palette = cx.ui_palette();
        let active = NavigationItem::active(window, cx);
        let selected_index = NavigationItem::ALL
            .iter()
            .position(|&item| Some(item) == active);
        let slot_width = ITEM_SIZE + ITEM_GAP;
        let target = slot_width * selected_index.unwrap_or_default() as f32;
        let last_slot = slot_width * (NavigationItem::ALL.len() - 1) as f32;
        let spring = SpringAnimation::new(SpringConfig::new(300.0, 34.0, 1.0)).to(target);
        let text = palette.text;
        let muted = palette.muted;
        let surface = palette.surface;
        let raised = palette.surface_hover;
        let selection = raised.blend(muted.opacity(0.14));
        let selection_inset = (ITEM_SIZE - SELECTION_SIZE) / 2.0;

        let dock = div()
            .id("navigation-dock")
            .track_focus(&focus)
            .occlude()
            .rounded_full()
            .bg(linear_gradient(
                180.0,
                linear_color_stop(raised, 0.0),
                linear_color_stop(surface, 1.0),
            ))
            .border_1()
            .border_color(palette.border)
            .shadow_md()
            .p_1()
            .child(div().relative().flex().gap(ITEM_GAP).with_spring(
                "dock-selection-position",
                spring,
                move |row, position| {
                    let position = position.clamp(px(0.0), last_slot);
                    row.when(selected_index.is_some(), |row| {
                        row.child(
                            div()
                                .absolute()
                                .top(selection_inset)
                                .left(position + selection_inset)
                                .size(SELECTION_SIZE)
                                .rounded_full()
                                .bg(selection),
                        )
                    })
                    .children(
                        NavigationItem::ALL
                            .into_iter()
                            .enumerate()
                            .map(|(index, item)| {
                                let left = slot_width * index as f32;
                                let coverage = if selected_index.is_some() {
                                    (1.0 - f32::from(position - left).abs()
                                        / f32::from(SELECTION_SIZE))
                                    .max(0.0)
                                } else {
                                    0.0
                                };
                                item.dock_button(
                                    ITEM_SIZE,
                                    muted.blend(text.opacity(coverage)),
                                    (coverage < 0.5).then_some(text),
                                    labels[index].clone(),
                                )
                            }),
                    )
                },
            ));

        div()
            .absolute()
            .inset_0()
            .child(observe_dock_pointer(state))
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .bottom(DOCK_MARGIN)
                    .flex()
                    .justify_center()
                    .child(dock.with_spring(
                        "navigation-dock-reveal",
                        reveal_spring,
                        |dock, phase| {
                            let progress = phase.interpolate_clamped(0.0, 1.0);
                            dock.relative()
                                .top(px(28.0 * (1.0 - progress)))
                                .opacity(progress)
                        },
                    )),
            )
    }
}
