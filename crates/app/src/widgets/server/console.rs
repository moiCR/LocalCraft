use gpui::{ScrollStrategy, UniformListScrollHandle, point, px};
use services::instance::{
    CONSOLE_CAPACITY,
    console::{ConsoleSnapshot, ConsoleText},
};
use std::{collections::VecDeque, sync::Arc};

pub const ROW_HEIGHT: f32 = 22.;

pub struct Console {
    pub rows: VecDeque<(usize, Arc<ConsoleText>)>,
    visible_rows: VecDeque<usize>,
    filter: String,
    next_id: usize,
    last_sequence: Option<u64>,
    pub follow: bool,
    pub scroll: UniformListScrollHandle,
}

impl Default for Console {
    fn default() -> Self {
        Self {
            rows: VecDeque::with_capacity(CONSOLE_CAPACITY),
            visible_rows: VecDeque::new(),
            filter: String::new(),
            next_id: 0,
            last_sequence: None,
            follow: true,
            scroll: UniformListScrollHandle::new(),
        }
    }
}

impl Console {
    pub fn replace(&mut self, snapshot: &ConsoleSnapshot) {
        self.clear();
        self.last_sequence = None;
        self.append_snapshot(snapshot);
    }

    pub fn append_snapshot(&mut self, snapshot: &ConsoleSnapshot) -> bool {
        let start = self.last_sequence.map_or(0, |sequence| {
            snapshot
                .lines
                .partition_point(|line| line.sequence <= sequence)
        });
        let mut changed = false;
        for line in snapshot.lines.iter().skip(start) {
            changed |= self.push(line.content.clone());
            self.last_sequence = Some(line.sequence);
        }
        if changed {
            self.follow_tail();
        }
        changed
    }

    pub fn echo(&mut self, text: String) {
        if self.push(ConsoleText::plain(text)) {
            self.follow_tail();
        }
    }

    fn push(&mut self, line: Arc<ConsoleText>) -> bool {
        let mut removed_visible = false;
        if self.rows.len() == CONSOLE_CAPACITY
            && let Some((id, _)) = self.rows.pop_front()
        {
            removed_visible = self.filter.is_empty() || self.visible_rows.front() == Some(&id);
            if self.visible_rows.front() == Some(&id) {
                self.visible_rows.pop_front();
            }
        }
        let matches = self.filter.is_empty() || line.search_text.contains(&self.filter);
        if !self.filter.is_empty() && matches {
            self.visible_rows.push_back(self.next_id);
        }
        self.rows.push_back((self.next_id, line));
        self.next_id = self.next_id.wrapping_add(1);
        if removed_visible && !self.follow {
            // Eviction must not move the text a user is reading under their cursor.
            let handle = &self.scroll.0.borrow().base_handle;
            let offset = handle.offset();
            handle.set_offset(point(offset.x, (offset.y + px(ROW_HEIGHT)).min(px(0.))));
        }
        matches || removed_visible
    }

    pub fn set_filter(&mut self, filter: String) {
        if self.filter == filter {
            return;
        }
        self.filter = filter;
        self.visible_rows.clear();
        if !self.filter.is_empty() {
            self.visible_rows.extend(
                self.rows.iter().filter_map(|(id, line)| {
                    line.search_text.contains(&self.filter).then_some(*id)
                }),
            );
        }
        self.follow_tail();
    }

    pub fn visible_len(&self) -> usize {
        if self.filter.is_empty() {
            self.rows.len()
        } else {
            self.visible_rows.len()
        }
    }

    pub fn visible_row(&self, index: usize) -> Option<&(usize, Arc<ConsoleText>)> {
        if self.filter.is_empty() {
            self.rows.get(index)
        } else {
            let id = *self.visible_rows.get(index)?;
            let first = self.rows.front()?.0;
            self.rows.get(id.checked_sub(first)?)
        }
    }

    pub fn clear(&mut self) {
        self.rows.clear();
        self.visible_rows.clear();
        self.scroll
            .0
            .borrow()
            .base_handle
            .set_offset(point(px(0.), px(0.)));
    }

    pub fn follow_tail(&self) {
        if self.follow && self.visible_len() > 0 {
            self.scroll
                .scroll_to_item(self.visible_len().saturating_sub(1), ScrollStrategy::Bottom);
        }
    }

    pub fn pause_follow(&mut self) {
        self.follow = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use services::instance::console::ConsoleLine;

    fn snapshot(range: std::ops::Range<u64>) -> ConsoleSnapshot {
        ConsoleSnapshot {
            lines: range
                .map(|sequence| ConsoleLine {
                    sequence,
                    content: ConsoleText::plain(format!("Row {sequence}")),
                })
                .collect(),
        }
    }

    #[test]
    fn snapshots_only_append_new_lines_and_clear_does_not_replay_history() {
        let mut console = Console::default();
        assert!(console.append_snapshot(&snapshot(0..100)));
        assert!(!console.append_snapshot(&snapshot(0..100)));
        console.clear();
        assert!(console.append_snapshot(&snapshot(0..102)));
        assert_eq!(console.rows.len(), 2);
        assert_eq!(
            console.rows.front().map(|row| row.1.text.as_ref()),
            Some("Row 100")
        );
    }

    #[test]
    fn filtering_and_eviction_keep_stable_ids_and_scroll_position() {
        let mut console = Console::default();
        console.append_snapshot(&snapshot(0..CONSOLE_CAPACITY as u64));
        console.pause_follow();
        console
            .scroll
            .0
            .borrow()
            .base_handle
            .set_offset(point(px(0.), px(-220.)));
        console.append_snapshot(&snapshot(0..CONSOLE_CAPACITY as u64 + 3));
        assert_eq!(console.rows.len(), CONSOLE_CAPACITY);
        assert_eq!(console.rows.front().map(|row| row.0), Some(3));
        assert_eq!(console.scroll.0.borrow().base_handle.offset().y, px(-154.));
        console.set_filter("row 200".into());
        assert_eq!(console.visible_len(), 4);
        assert_eq!(
            console.visible_row(0).map(|row| row.1.text.as_ref()),
            Some("Row 200")
        );
        console.append_snapshot(&snapshot(0..CONSOLE_CAPACITY as u64 + 4));
        assert_eq!(console.visible_len(), 5);
    }
}
