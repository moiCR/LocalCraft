use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use crate::workspace::Page;

pub const SIDEBAR_DURATION: Duration = Duration::from_millis(300);
const HISTORY_LIMIT: usize = 64;

#[derive(Default)]
pub struct Navigation {
    pub current: Page,
    back: VecDeque<Page>,
    forward: Vec<Page>,
}

impl Navigation {
    pub fn visit(&mut self, page: Page) {
        if page == self.current {
            return;
        }
        if self.back.len() == HISTORY_LIMIT {
            self.back.pop_front();
        }
        self.back.push_back(self.current);
        self.forward.clear();
        self.current = page;
    }

    pub fn can_go_back(&self) -> bool {
        !self.back.is_empty()
    }
    pub fn can_go_forward(&self) -> bool {
        !self.forward.is_empty()
    }

    pub fn go_back(&mut self) {
        if let Some(page) = self.back.pop_back() {
            self.forward.push(self.current);
            self.current = page;
        }
    }

    pub fn go_forward(&mut self) {
        if let Some(page) = self.forward.pop() {
            self.back.push_back(self.current);
            self.current = page;
        }
    }
}

pub struct SidebarMotion {
    pub visible: bool,
    pub revision: usize,
    from: f32,
    started: Instant,
}

impl Default for SidebarMotion {
    fn default() -> Self {
        Self {
            visible: true,
            revision: 0,
            from: 1.,
            started: Instant::now(),
        }
    }
}

impl SidebarMotion {
    pub fn toggle(&mut self) {
        self.toggle_at(Instant::now());
    }

    fn toggle_at(&mut self, now: Instant) {
        self.from = self.value_at(now);
        self.visible = !self.visible;
        self.started = now;
        self.revision = self.revision.wrapping_add(1);
    }

    pub fn endpoints(&self) -> (f32, f32) {
        (self.from, if self.visible { 1. } else { 0. })
    }

    pub fn value_at(&self, now: Instant) -> f32 {
        let progress = (now.saturating_duration_since(self.started).as_secs_f32()
            / SIDEBAR_DURATION.as_secs_f32())
        .clamp(0., 1.);
        let eased = 1. - (1. - progress).powi(5);
        let target = if self.visible { 1. } else { 0. };
        self.from + (target - self.from) * eased
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_handles_boundaries_and_discards_forward_branch() {
        let mut navigation = Navigation::default();
        navigation.go_back();
        assert!(!navigation.can_go_back());
        navigation.visit(Page::Runtimes);
        navigation.go_back();
        assert!(navigation.current == Page::Instances);
        assert!(navigation.can_go_forward());
        navigation.visit(Page::Instances);
        assert!(navigation.can_go_forward());
        navigation.go_forward();
        assert!(navigation.current == Page::Runtimes);
        navigation.go_back();
        navigation.visit(Page::Runtimes);
        assert!(!navigation.can_go_forward());
    }

    #[test]
    fn sidebar_reverses_without_jumping_and_finishes_fully_hidden() {
        let mut motion = SidebarMotion::default();
        let start = Instant::now();
        motion.toggle_at(start);
        assert_eq!(motion.value_at(start), 1.);
        let middle = start + Duration::from_millis(100);
        let before = motion.value_at(middle);
        assert!(before > 0. && before < 1.);
        motion.toggle_at(middle);
        assert_eq!(motion.value_at(middle), before);
        assert_eq!(motion.value_at(middle + SIDEBAR_DURATION), 1.);
        motion.toggle_at(middle + SIDEBAR_DURATION);
        assert_eq!(motion.value_at(middle + SIDEBAR_DURATION * 2), 0.);
    }
}
