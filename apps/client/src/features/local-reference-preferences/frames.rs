//! Local frame scheduling and measured presentation statistics.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FramePreferences {
    pub foreground: u16,
    pub background: u16,
    pub show_fps: bool,
    pub show_latency: bool,
}
impl Default for FramePreferences {
    fn default() -> Self {
        Self {
            foreground: 60,
            background: 15,
            show_fps: false,
            show_latency: false,
        }
    }
}
impl FramePreferences {
    pub fn valid(self) -> bool {
        (1..=500).contains(&self.foreground) && (1..=500).contains(&self.background)
    }
    pub fn indicator(self, fps: Option<u32>, latency: Option<u32>) -> String {
        let mut labels = Vec::new();
        if self.show_fps {
            labels.push(fps.map_or_else(|| "FPS: measuring".into(), |fps| format!("FPS: {fps}")));
        }
        if self.show_latency {
            labels.push(latency.map_or_else(
                || "Latency: unavailable".into(),
                |ms| format!("Latency: ~{ms} ms"),
            ));
        }
        labels.join(" · ")
    }
}
#[derive(Debug, Default)]
pub struct FramePacer {
    last_request: Option<u64>,
    sample_start: Option<u64>,
    presented: u32,
    fps: Option<u32>,
}
impl FramePacer {
    /// Returns whether another redraw is due and the next monotonic nanosecond deadline.
    pub fn request(
        &mut self,
        now: u64,
        preferences: FramePreferences,
        focused: bool,
    ) -> (bool, u64) {
        let limit = if focused {
            preferences.foreground
        } else {
            preferences.background
        }
        .clamp(1, 500);
        let period = 1_000_000_000 / u64::from(limit);
        let due = self
            .last_request
            .map_or(now, |last| last.saturating_add(period));
        if now >= due {
            self.last_request = Some(now);
            (true, now.saturating_add(period))
        } else {
            (false, due)
        }
    }
    /// Count only completed renderer presentations, never requested redraws.
    pub fn presented(&mut self, now: u64) {
        let Some(start) = self.sample_start else {
            self.sample_start = Some(now);
            return;
        };
        self.presented = self.presented.saturating_add(1);
        let elapsed = now.saturating_sub(start);
        if elapsed >= 1_000_000_000 {
            self.fps = Some(
                u32::try_from(u64::from(self.presented) * 1_000_000_000 / elapsed)
                    .unwrap_or(u32::MAX),
            );
            self.sample_start = Some(now);
            self.presented = 0;
        }
    }
    pub const fn fps(&self) -> Option<u32> {
        self.fps
    }
}
impl crate::play::PlayLink {
    pub fn performance_latency(&self) -> Option<u32> {
        crate::play::mailbox::lock(&self.mailbox)
            .spell_bar
            .bar
            .cooldowns
            .as_ref()
            .and_then(|cooldowns| cooldowns.rtt_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play::SettingsState;
    #[test]
    fn frame_limits_follow_focus_and_report_measured_frames_and_server_latency()
    -> Result<(), Box<dyn std::error::Error>> {
        let prefs = FramePreferences {
            foreground: 60,
            background: 10,
            show_fps: true,
            show_latency: true,
        };
        let mut clock = FramePacer::default();
        assert_eq!(clock.request(0, prefs, true), (true, 16_666_666));
        assert_eq!(clock.request(5_000_000, prefs, true), (false, 16_666_666));
        assert_eq!(
            clock.request(50_000_000, prefs, false),
            (false, 100_000_000)
        );
        assert_eq!(
            clock.request(100_000_000, prefs, false),
            (true, 200_000_000)
        );
        assert_eq!(clock.request(120_000_000, prefs, true), (true, 136_666_666));
        assert_eq!(clock.fps(), None);
        for frame in 0..=60 {
            clock.presented(frame * 1_000_000_000 / 60);
        }
        assert_eq!(clock.fps(), Some(60));
        assert_eq!(
            prefs.indicator(clock.fps(), Some(42)),
            "FPS: 60 · Latency: ~42 ms"
        );
        assert!(prefs.indicator(None, None).contains("Latency: unavailable"));
        let (link, _, outbox) = crate::play::play_channel();
        assert_eq!(link.performance_latency(), None);
        outbox.spell_bar_initialize(Some(
            oteryn_protocol_oteryn::presentation::encode_actor_cooldowns(
                &oteryn_session::ActorCooldowns {
                    server_now_ms: 10,
                    rtt_ms: Some(42),
                    entries: vec![],
                },
            )
            .map_err(|_| "cooldown encode")?,
        ));
        assert_eq!(link.performance_latency(), Some(42));
        let path =
            std::env::temp_dir().join(format!("frame-preferences-{}.json", std::process::id()));
        let mut settings = SettingsState::new(Some(path.clone()));
        let mut updated = settings.preferences.clone();
        updated.frames = prefs;
        settings.apply(updated.clone());
        assert_eq!(
            SettingsState::new(Some(path.clone())).preferences.frames,
            prefs
        );
        updated.frames.foreground = 0;
        settings.apply(updated);
        assert_eq!(settings.preferences.frames, prefs);
        std::fs::remove_file(path)?;
        Ok(())
    }
}
