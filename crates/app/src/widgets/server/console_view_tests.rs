use super::*;
use gpui::{ScrollDelta, ScrollWheelEvent, TestAppContext, TouchPhase, VisualTestContext, point};
use services::{
    instance::{
        CONSOLE_CAPACITY, InstancesService,
        console::{ConsoleLine, ConsoleText},
    },
    java::JavaService,
    software::SoftwareService,
};

struct Harness {
    console: Entity<ConsoleView>,
    input: Entity<Input>,
    cached: bool,
}

impl Render for Harness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(800.))
            .h(px(480.))
            .flex()
            .flex_col()
            .child(div().h(px(40.)).flex_shrink_0().child("Server controls"))
            .child(if self.cached {
                ConsoleView::cached(self.console.clone()).into_any_element()
            } else {
                div()
                    .flex_1()
                    .min_h_0()
                    .child(self.console.clone())
                    .into_any_element()
            })
            .child(self.input.clone())
    }
}

fn initialize(cx: &mut TestAppContext) -> anyhow::Result<tokio::runtime::Runtime> {
    let runtime = tokio::runtime::Runtime::new()?;
    cx.set_global(AppState {
        theme_manager: Default::default(),
        instance_service: InstancesService::default(),
        java_service: Arc::new(JavaService::new()),
        background_runtime: runtime.handle().clone(),
        software_service: SoftwareService::new(),
        playit_service: Arc::new(services::playit::PlayitService::new()),
        preferences: Default::default(),
        preferences_store: Default::default(),
        preferences_revision: std::sync::atomic::AtomicU64::new(0),
    });
    Ok(runtime)
}

fn snapshot(count: usize) -> Arc<ConsoleSnapshot> {
    Arc::new(ConsoleSnapshot {
        lines: (0..count)
            .map(|sequence| ConsoleLine {
                sequence: sequence as u64,
                timestamp: services::instance::console::timestamp_now(),
                content: ConsoleText::plain(format!(
                    "[server/INFO] Log {sequence}: {}",
                    "x".repeat(200)
                )),
            })
            .collect(),
    })
}

fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

#[gpui::test]
fn console_cache_skips_input_frames_and_invalidates_for_logs(cx: &mut TestAppContext) {
    let runtime = initialize(cx);
    assert!(runtime.is_ok(), "Could not initialize test runtime");
    let (published, updates) = tokio::sync::watch::channel(snapshot(CONSOLE_CAPACITY));
    let (view, cx) = cx.add_window_view(|_, cx| Harness {
        console: cx.new(|cx| ConsoleView::new(updates, cx)),
        input: cx.new(|cx| Input::new("command", "Command", cx)),
        cached: true,
    });
    draw(cx);
    let (console, input) = cx.update(|_, cx| {
        let view = view.read(cx);
        (view.console.clone(), view.input.clone())
    });
    let initial = cx.update(|_, cx| console.read(cx).render_count);
    for _ in 0..20 {
        input.update(cx, |input, cx| input.clear(cx));
        draw(cx);
    }
    let cached = cx.update(|_, cx| console.read(cx).render_count - initial);
    assert_eq!(
        cached, 0,
        "Unchanged logs were rendered during input updates"
    );

    published.send_replace(snapshot(CONSOLE_CAPACITY + 1));
    draw(cx);
    assert!(cx.update(|_, cx| console.read(cx).render_count) > initial);

    let before_theme = cx.update(|_, cx| console.read(cx).render_count);
    cx.update_global::<AppState, _>(|state, _| {
        state
            .theme_manager
            .set_appearance(ui::theme::Appearance::Light)
    });
    draw(cx);
    assert!(cx.update(|_, cx| console.read(cx).render_count) > before_theme);

    view.update(cx, |view, cx| {
        view.cached = false;
        cx.notify();
    });
    draw(cx);
    let initial = cx.update(|_, cx| console.read(cx).render_count);
    for _ in 0..20 {
        input.update(cx, |input, cx| input.clear(cx));
        draw(cx);
    }
    let uncached = cx.update(|_, cx| console.read(cx).render_count - initial);
    assert!(uncached >= 20);
    eprintln!(
        "20 input updates: cached console renders={cached}; uncached console renders={uncached}"
    );
}

#[gpui::test]
fn console_layout_and_scroll_only_render_viewport_rows(cx: &mut TestAppContext) {
    let runtime = initialize(cx);
    assert!(runtime.is_ok(), "Could not initialize test runtime");
    let (_published, updates) = tokio::sync::watch::channel(snapshot(CONSOLE_CAPACITY));
    let (view, cx) = cx.add_window_view(|_, cx| Harness {
        console: cx.new(|cx| ConsoleView::new(updates, cx)),
        input: cx.new(|cx| Input::new("", "Command", cx)),
        cached: true,
    });
    draw(cx);
    let console = cx.update(|_, cx| view.read(cx).console.clone());
    let initial = cx.update(|_, cx| {
        console
            .read(cx)
            .console
            .scroll
            .0
            .borrow()
            .base_handle
            .offset()
            .y
    });
    assert!(
        initial < px(-30_000.),
        "Expected viewport to follow the end of 2,000 rows"
    );
    let mut most_rows = 0;
    let started = std::time::Instant::now();
    for _ in 0..30 {
        cx.update(|_, cx| console.read(cx).rendered_rows.set(0));
        cx.simulate_event(ScrollWheelEvent {
            position: point(px(200.), px(200.)),
            delta: ScrollDelta::Pixels(point(px(0.), px(66.))),
            touch_phase: TouchPhase::Moved,
            ..Default::default()
        });
        draw(cx);
        let rows = cx.update(|_, cx| console.read(cx).rendered_rows.get());
        assert!(rows < 50, "Rendered {rows} rows for a 480px viewport");
        most_rows = most_rows.max(rows);
    }
    let final_offset = cx.update(|_, cx| {
        console
            .read(cx)
            .console
            .scroll
            .0
            .borrow()
            .base_handle
            .offset()
            .y
    });
    assert!(final_offset > initial);
    assert!(!cx.update(|_, cx| console.read(cx).console.follow));
    eprintln!(
        "30 scroll frames: max rows built per frame={most_rows}, total={:?} (headless; not GPU FPS)",
        started.elapsed()
    );
}

#[gpui::test]
fn long_lines_stay_single_rows_and_copy_in_full(cx: &mut TestAppContext) {
    let runtime = initialize(cx);
    assert!(runtime.is_ok(), "Could not initialize test runtime");
    let text = ConsoleText::plain("x".repeat(8_192));
    let history = Arc::new(ConsoleSnapshot {
        lines: (0..CONSOLE_CAPACITY)
            .map(|sequence| ConsoleLine {
                sequence: sequence as u64,
                timestamp: services::instance::console::timestamp_now(),
                content: text.clone(),
            })
            .collect(),
    });
    let (_published, updates) = tokio::sync::watch::channel(history);
    let (view, cx) = cx.add_window_view(|_, cx| Harness {
        console: cx.new(|cx| ConsoleView::new(updates, cx)),
        input: cx.new(|cx| Input::new("", "Command", cx)),
        cached: true,
    });
    draw(cx);
    let console = cx.update(|_, cx| view.read(cx).console.clone());
    cx.update(|_, cx| console.read(cx).rendered_rows.set(0));
    cx.simulate_event(ScrollWheelEvent {
        position: point(px(200.), px(200.)),
        delta: ScrollDelta::Pixels(point(px(0.), px(66.))),
        touch_phase: TouchPhase::Moved,
        ..Default::default()
    });
    draw(cx);
    assert!(cx.update(|_, cx| console.read(cx).rendered_rows.get()) < 50);
    cx.simulate_event(gpui::MouseDownEvent {
        position: point(px(200.), px(200.)),
        button: gpui::MouseButton::Left,
        click_count: 1,
        ..Default::default()
    });
    cx.simulate_event(gpui::MouseUpEvent {
        position: point(px(200.), px(200.)),
        button: gpui::MouseButton::Left,
        click_count: 1,
        ..Default::default()
    });
    assert_eq!(
        cx.read_from_clipboard()
            .and_then(|item| item.text())
            .as_deref(),
        Some(text.text.as_ref())
    );
}
