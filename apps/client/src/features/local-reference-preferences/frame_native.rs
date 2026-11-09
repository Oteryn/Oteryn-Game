//! One shell clock drives both the merged graphics controls and presentation indicators.
use super::Application;
use oteryn_client::play::{PerformancePacer, PerformanceSettings};
use std::time::{Duration, Instant};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
#[derive(Default)]
pub(super) struct FrameTiming {
    pacer: PerformancePacer,
    last_title: String,
}
impl FrameTiming {
    pub(super) fn begin_redraw(&mut self, settings: PerformanceSettings) -> bool {
        self.pacer.begin_redraw(Instant::now(), settings)
    }
    pub(super) fn presented(&mut self) {
        self.pacer.presented(Instant::now());
    }
    pub(super) fn occluded(&mut self, value: bool) {
        self.pacer.occluded(value);
    }
}
impl Application {
    pub(super) fn schedule_frames(&mut self, event_loop: &ActiveEventLoop) {
        let settings = self.settings.performance();
        let now = Instant::now();
        self.frame_timing.pacer.focus(
            self.window
                .as_ref()
                .is_some_and(|window| window.has_focus()),
        );
        let deadline = self.frame_timing.pacer.deadline(now, settings);
        let wake = if deadline > now {
            deadline.min(now + Duration::from_millis(20))
        } else {
            now + Duration::from_millis(1)
        };
        event_loop.set_control_flow(ControlFlow::WaitUntil(wake));
        let command_latency = self
            .play
            .as_ref()
            .and_then(|play| play.link.performance_command_latency());
        let mut indicator = self
            .frame_timing
            .pacer
            .indicator(now, settings, command_latency)
            .unwrap_or_default();
        // Preserve the server's independently measured estimate alongside command turnaround.
        if settings.show_latency
            && let Some(ms) = self
                .play
                .as_ref()
                .and_then(|play| play.link.performance_latency())
        {
            indicator.push_str(&format!(" · Server RTT: ~{ms} ms"));
        }
        self.settings
            .performance_indicator((!indicator.is_empty()).then_some(indicator.as_str()));
        if let Some(window) = &self.window {
            if self.frame_timing.pacer.due(now, settings)
                && super::redraw_eligible(
                    self.renderer
                        .as_ref()
                        .map(|renderer| renderer.state().phase()),
                )
            {
                window.request_redraw();
            }
            if let Some(play) = &self.play {
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
