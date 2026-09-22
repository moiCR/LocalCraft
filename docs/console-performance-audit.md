# Console performance audit

This audit covers the code present after the earlier console fixes. The findings below distinguish demonstrated work and delays from hardware-dependent frame performance.

## Findings

| Vector | Before this refactor | Result |
| --- | --- | --- |
| Virtualization | Already used `uniform_list`, 22 px rows, stable IDs, and no wrapping. There was no overflowing `div` containing the entire history. | Keep virtualization. Explicitly constrain the viewport and use `ListSizingBehavior::Auto`. |
| GPUI caching | `ConsoleView` was a separate entity, but inserted directly as an element. GPUI 0.2.2 calls an uncached entity's `render()` during layout. Separating entities alone did not prevent console reconstruction during input frames. | Insert the console as `AnyView::cached`. A headless comparison produced 40 uncached console renders versus 0 cached renders for 20 input updates. |
| Text layout | Wrapping was already disabled. Long lines were still shaped in full before being clipped. | Truncate displayed text to the row width before shaping. Copy retains the full stored line. |
| Ingestion | The supervisor batched for 75 ms, then `ServerScreen` batched again for 250 ms. Its FIFO channel could retain 16 pending updates. | Publish at one 33 ms cadence. A `watch` channel holds the latest bounded snapshot, so a stalled UI does not replay obsolete batches. |
| Notification rate | No per-line `cx.notify()` in the original ingestion path. The redundant second batching stage added latency rather than solving an unthrottled stream. | Notify once for a snapshot that changes visible console contents. No notifications for unchanged snapshots. |
| Storage | Backend and UI already used bounded 2,000-line rings. However, broadcasts and snapshots cloned strings. Search lowercased and rescanned all retained rows on each batch when a filter was active. | Share immutable parsed lines using `Arc`. Cache lowercase search text once. Match only newly appended lines, and retain stable IDs when evicting rows. |
| ANSI | ANSI was never parsed, including during `render()`. Raw escapes were passed directly to GPUI text. | Parse on Tokio before publication. Cache clean text, byte ranges, colors, and emphasis. Rendering consumes cached spans. |
| Stdin | Writes already ran asynchronously on the dedicated Tokio runtime. However, the supervisor awaited a stdin write in the same loop responsible for draining and publishing logs. A stalled write could suspend that work for the 2-second write timeout. | Give log readers and the collector independent Tokio tasks. A full command queue fails immediately instead of waiting to enqueue. |
| Lifecycle | The supervisor published `Exited` before updating the completion watch. A consumer could observe the exit event with a stale running flag. | Publish the completion flag before the exit event, after the final log drain. |

The previous 300 ms initial-load delay and 120 ms refresh delay are also removed. Console snapshots now contain prepared, shared data and need no disk access or reparsing when a view opens. The loader remains available until the initial snapshot is applied.

## Refactor steps

### 1. Cache the view and bound its list

Implemented in `crates/app/src/widgets/server/console_view.rs` and used by `screen_view.rs`:

```rust
pub fn cached(view: Entity<Self>) -> AnyView {
    AnyView::from(view).cached(
        StyleRefinement::default()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .w_full(),
    )
}
```

The console root fills those bounds. Its list occupies an absolutely positioned viewport inside a relative flex container. `uniform_list` builds only the requested row range; each row has a fixed height and line height of 22 px, a stable ID, and `.truncate()`.

```rust
uniform_list("server-console", row_count, render_visible_rows)
    .with_sizing_behavior(ListSizingBehavior::Auto)
    .absolute()
    .inset_0()
    .size_full()
    .track_scroll(scroll_handle)
```

`render_visible_rows` above abbreviates the actual closure in `ConsoleView::render`. It creates `StyledText` from cached content and spans. There is no parsing, filesystem access, or task creation in that closure.

Native wheel handling invalidates the console view when its scroll offset changes. Incoming logs and search changes call `cx.notify()` explicitly. Theme changes also invalidate the cache. Unrelated input notifications can reuse the console's layout and paint.

### 2. Parse and publish on Tokio

Implemented in `crates/services/src/instance/ansi.rs` and `console.rs`:

```rust
pub const CONSOLE_FLUSH_INTERVAL: Duration = Duration::from_millis(33);

#[derive(Clone, Debug)]
pub struct ConsoleLine {
    pub sequence: u64,
    pub content: Arc<ConsoleText>,
}

#[derive(Default, Debug)]
pub struct ConsoleSnapshot {
    pub lines: Vec<ConsoleLine>,
}
```

Each stdout/stderr reader accumulates complete lines across pipe reads, parses ANSI once, and sends batches of up to 32 lines through a bounded channel. Each stream has its own parser state. The collector maintains the 2,000-line ring and publishes at 33 ms intervals while dirty. A final flush preserves trailing output on exit.

```rust
fn publish(
    ring: &VecDeque<ConsoleLine>,
    published: &watch::Sender<Arc<ConsoleSnapshot>>,
) {
    published.send_replace(Arc::new(ConsoleSnapshot {
        lines: ring.iter().cloned().collect(),
    }));
}
```

Snapshot creation copies sequence numbers and `Arc` handles, not text buffers. Parsing stores clean `SharedString` text, cached lowercase search text, and GPUI `HighlightStyle` spans. Supported styling includes standard, bright, indexed, and RGB colors; resets; bold; dim; italic; and underline. OSC/control sequences are hidden. This is a log display, not a full terminal emulator.

Subprocess lines are bounded to 8,192 bytes and 128 styled spans. The parser uses UTF-8 byte boundaries for GPUI spans. Very long lines and incomplete escape sequences cannot create unbounded allocations.

### 3. Apply only unseen rows in GPUI

Implemented in `crates/app/src/widgets/server/console.rs` and `console_view.rs`:

```rust
let start = self.last_sequence.map_or(0, |sequence| {
    snapshot.lines.partition_point(|line| line.sequence <= sequence)
});

let mut changed = false;
for line in snapshot.lines.iter().skip(start) {
    changed |= self.push(line.content.clone());
    self.last_sequence = Some(line.sequence);
}
```

The view waits on `watch::Receiver::changed()` inside `cx.spawn`, clones the latest snapshot handle, and releases the watch borrow before updating the entity or awaiting again. That GPUI task only bridges channel data. Pipe I/O and parsing remain on Tokio.

Search matches are updated incrementally using cached lowercase text. Clearing the console keeps the stream cursor, preventing the next snapshot from restoring old lines. When old rows are evicted while auto-scroll is paused, the scroll offset is adjusted to preserve the text being read.

### 4. Keep command dispatch independent

Implemented in `crates/services/src/instance/supervisor.rs`, `server_instance.rs`, and the existing `ServerScreen::send_command` callback:

```rust
let (lines, incoming) = mpsc::channel(64);
let mut readers = tokio::task::JoinSet::new();
readers.spawn(console::read_output(stdout, lines.clone()));
readers.spawn(console::read_output(stderr, lines));
let console_task = tokio::spawn(console::collect(incoming, console));
```

The supervisor handles process exit, shutdown, and stdin requests independently of that collector. Command enqueueing is bounded and immediate:

```rust
sender
    .try_send(supervisor::Request::Command(command, reply))
    .context("Server command queue is full or closed")?;
response.await.context("Server has exited")?
```

The UI echoes the command and clears the input immediately, then runs `server.send_command` on `AppState.background_runtime`. It remains possible to type and submit another command. Failed sends are reported in status. The 2-second pipe-write timeout, 5-second UI request timeout, and cancellation check remain in place.

Tokio is intentional here: the repository requires its dedicated runtime for child-process I/O. Moving Tokio process handles onto GPUI's executor would not improve this design.

## Validation

Executed on this workspace:

```text
cargo test --workspace -- --nocapture
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

Results: 30 tests passed; one existing provider-network test stayed ignored. Clippy and diff checks passed. Two download tests needed permission to bind their local HTTP test servers outside the sandbox; they passed in that run.

Measured regression cases:

- 20 input updates: 40 console renders with the uncached element; 0 with the cached view.
- 2,000 retained lines in an 800 × 480 harness: at most 18 rows built per scroll frame, including measurement rows.
- 30 synthetic scroll frames: approximately 142 ms total in the final run. This includes the headless test driver and is not a GPU FPS measurement.
- A 10,000-line ANSI subprocess burst: approximately 69 µs for stdin acknowledgement and 67 ms until the command response appeared in the published snapshot.
- A subprocess that does not read stdin: a 1 MB write remains pending while its subsequent stdout line still reaches the console within the test deadline.
- Additional cases cover ANSI split across pipe reads, RGB/Unicode span boundaries, OSC removal, style/line limits, ring eviction, slow subscribers, duplicate snapshots, clear semantics, theme invalidation, final shutdown output, and copying a clipped 8,192-byte line in full.

These tests demonstrate the corrected CPU work and channel behavior. They do not measure the real JVM's response time, the desktop compositor, or the user's GPU. Persistent platform-specific frame stalls would require profiling the running desktop application.
