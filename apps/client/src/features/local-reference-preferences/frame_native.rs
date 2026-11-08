//! Frame pacing is applied by the shell; all platform calls stay on its thread.
use super::Application;
use oteryn_client::play::FramePacer;
use std::time::{Duration, Instant};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
pub(super) struct FrameTiming {
    epoch: Instant,
    pacer: FramePacer,
    last_title: String,
}
impl Default for FrameTiming {
    fn default() -> Self {
        Self {
            epoch: Instant::now(),
            pacer: FramePacer::default(),
            last_title: String::new(),
        }
    }
}
impl FrameTiming {
    fn now(&self) -> u64 {
        u64::try_from(self.epoch.elapsed().as_nanos()).unwrap_or(u64::MAX)
    }
    pub(super) fn presented(&mut self) {
        self.pacer.presented(self.now());
    }
}
impl Application {
    pub(super) fn schedule_frames(&mut self, event_loop: &ActiveEventLoop) {
        let prefs = self.settings.frame_preferences();
        let now = self.frame_timing.now();
        let focused = self
            .window
            .as_ref()
            .is_some_and(|window| window.has_focus());
        let (due, deadline) = self.frame_timing.pacer.request(now, prefs, focused);
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_nanos(deadline.saturating_sub(now)),
        ));
        if let Some(window) = &self.window {
            if due
                && super::redraw_eligible(
                    self.renderer
                        .as_ref()
                        .map(|renderer| renderer.state().phase()),
                )
            {
                window.request_redraw();
            }
            if let Some(play) = &self.play {
                let indicator = prefs.indicator(
                    self.frame_timing.pacer.fps(),
                    play.link.performance_latency(),
                );
                let title = format!(
                    "Oteryn — {}{}{} — Ctrl+G: Guild — Ctrl+Shift+A: achievements",
                    indicator,
                    if indicator.is_empty() { "" } else { " — " },
                    play.status
                );
                if self.frame_timing.last_title != title {
                    window.set_title(&title);
                    self.frame_timing.last_title = title;
                }
            } else {
                self.frame_timing.last_title.clear();
            }
        }
    }
}
