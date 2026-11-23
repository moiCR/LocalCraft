use std::time::{Duration, Instant};

pub const SIDEBAR_DURATION: Duration = Duration::from_millis(300);

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    Instances,
    Runtimes,
    Playit,
}

impl Page {
    pub fn path(self) -> &'static str {
        match self {
            Self::Instances => "/",
            Self::Runtimes => "/runtimes",
            Self::Playit => "/playit",
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
        Self::with_visibility(true)
    }
}

impl SidebarMotion {
    pub fn with_visibility(visible: bool) -> Self {
        let value = if visible { 1. } else { 0. };
        Self {
            visible,
            revision: 0,
            from: value,
            started: Instant::now(),
        }
    }

    pub fn toggle(&mut self) {
        self.toggle_at(Instant::now());
    }

    fn toggle_at(&mut self, now: Instant) {
        self.set_visible_at(!self.visible, now);
    }

    pub fn set_visible(&mut self, visible: bool) {
        if self.visible != visible {
            self.set_visible_at(visible, Instant::now());
        }
    }

    fn set_visible_at(&mut self, visible: bool, now: Instant) {
        self.from = self.value_at(now);
        self.visible = visible;
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
