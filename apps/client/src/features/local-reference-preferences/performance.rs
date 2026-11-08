//! Local frame pacing and measured presentation statistics. No simulated network samples.
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PerformanceSettings {
    pub foreground_fps: u16,
    pub background_fps: u16,
    pub show_fps: bool,
    pub show_latency: bool,
}
impl Default for PerformanceSettings {
    fn default() -> Self {
        Self {
            foreground_fps: 60,
            background_fps: 15,
            show_fps: false,
            show_latency: false,
        }
    }
}
impl PerformanceSettings {
    pub fn valid(self) -> bool {
        self.foreground_fps <= 360 && (1..=120).contains(&self.background_fps)
    }
    pub fn interval(self, active: bool) -> Duration {
        let fps = if active {
            self.foreground_fps
        } else {
            self.background_fps
        };
        if fps == 0 {
            Duration::ZERO
        } else {
            Duration::from_nanos(1_000_000_000 / u64::from(fps))
        }
    }
}
#[derive(Debug)]
pub struct FramePacer {
    focused: bool,
    occluded: bool,
    last_frame: Option<Instant>,
    last_redraw: Option<Instant>,
    sample_start: Option<Instant>,
    frames: u64,
    fps: Option<u64>,
}
impl Default for FramePacer {
    fn default() -> Self {
        Self {
            focused: true,
            occluded: false,
            last_frame: None,
            last_redraw: None,
            sample_start: None,
            frames: 0,
            fps: None,
        }
    }
}
impl FramePacer {
    pub fn focus(&mut self, focused: bool) {
        self.focused = focused;
    }
    pub fn occluded(&mut self, occluded: bool) {
        self.occluded = occluded;
    }
    pub fn deadline(&self, now: Instant, settings: PerformanceSettings) -> Instant {
        self.last_redraw
            .and_then(|at| at.checked_add(settings.interval(self.focused && !self.occluded)))
            .unwrap_or(now)
    }
    pub fn due(&self, now: Instant, settings: PerformanceSettings) -> bool {
        now >= self.deadline(now, settings)
    }
    pub fn begin_redraw(&mut self, now: Instant, settings: PerformanceSettings) -> bool {
        if !self.due(now, settings) {
            return false;
        }
        self.last_redraw = Some(now);
        true
    }
    /// Called only after a successful presentation, never merely a redraw request.
    pub fn presented(&mut self, now: Instant) {
        self.last_frame = Some(now);
        let Some(start) = self.sample_start else {
            self.sample_start = Some(now);
            return;
        };
        self.frames = self.frames.saturating_add(1);
        let elapsed = now.saturating_duration_since(start);
        if elapsed >= Duration::from_secs(1) {
            self.fps = Some(
                u64::try_from(u128::from(self.frames) * 1_000_000_000 / elapsed.as_nanos().max(1))
                    .unwrap_or(u64::MAX),
            );
            self.frames = 0;
            self.sample_start = Some(now);
        }
    }
    pub fn indicator(
        &self,
        now: Instant,
        settings: PerformanceSettings,
        latency: Option<(Instant, Duration)>,
    ) -> Option<String> {
        let mut parts = Vec::new();
        if settings.show_fps {
            let fresh = self
                .last_frame
                .is_some_and(|at| now.saturating_duration_since(at) <= Duration::from_secs(2));
            let value = self
                .fps
                .filter(|_| fresh)
                .map_or_else(|| "—".into(), |value| value.to_string());
            parts.push(format!("FPS: {value}"));
        }
        if settings.show_latency {
            let value = latency
                .filter(|(at, _)| now.saturating_duration_since(*at) <= Duration::from_secs(15))
                .map_or_else(
                    || "—".into(),
                    |(_, duration)| format!("{} ms", duration.as_millis()),
                );
            parts.push(format!("Command RTT: {value}"));
        }
        (!parts.is_empty()).then(|| parts.join(" | "))
    }
}
impl crate::play::PlayLink {
    pub fn performance_latency(&self) -> Option<(Instant, Duration)> {
        crate::play::mailbox::lock(&self.mailbox).command_latency
    }
}
impl crate::play::Outbox {
    pub(in crate::play) fn performance_reset(&self) {
        crate::play::mailbox::lock(&self.0).command_latency = None;
    }
    pub(in crate::play) fn performance_command_result(&self, elapsed: Duration) {
        crate::play::mailbox::lock(&self.0).command_latency = Some((Instant::now(), elapsed));
    }
}
