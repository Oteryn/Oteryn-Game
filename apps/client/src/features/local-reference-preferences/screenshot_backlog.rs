//! One frame per second, bounded by frame count and 64 MiB, scoped to the current session.
use super::{CaptureSource, Pixels, SettingsState};
use crate::play::PlayLink;
use std::{
    collections::VecDeque,
    io,
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Default)]
pub(in super::super) struct ScreenshotRuntime {
    frames: VecDeque<Pixels>,
    last_frame: Option<Instant>,
    last_query: Option<Instant>,
    previous: Option<Signals>,
    game_only: Option<bool>,
}
struct Signals {
    dead: Option<bool>,
    level: Option<u32>,
    skills: Option<[u16; 8]>,
}
impl Signals {
    fn read(link: &PlayLink) -> Self {
        let panel = link.skill_progress_panel();
        Self {
            dead: link
                .actor_vitals()
                .map(|v| v.max_health > 0 && v.health == 0),
            level: panel.complete.map(|s| s.level),
            skills: panel.snapshot.map(|s| s.skills.map(|skill| skill.level)),
        }
    }
}
impl SettingsState {
    pub fn process_screenshots(
        &mut self,
        now: Instant,
        link: Option<&PlayLink>,
        source: &mut impl CaptureSource,
    ) -> io::Result<Vec<PathBuf>> {
        let preferences = &self.preferences.screenshots;
        let runtime = &mut self.screenshot_runtime;
        if link.is_none() || runtime.game_only != Some(preferences.game_only) {
            *runtime = ScreenshotRuntime::default();
            runtime.game_only = Some(preferences.game_only);
        }
        if let Some(link) = link {
            let signals = Signals::read(link);
            if let Some(previous) = &runtime.previous {
                let death = previous.dead == Some(false) && signals.dead == Some(true);
                let level = previous
                    .level
                    .zip(signals.level)
                    .is_some_and(|(old, new)| new > old);
                let skill = previous
                    .skills
                    .zip(signals.skills)
                    .is_some_and(|(old, new)| old.into_iter().zip(new).any(|(a, b)| b > a));
                self.screenshot_requested |= (preferences.on_death && death)
                    || (preferences.on_level && level)
                    || (preferences.on_skill && skill);
            }
            runtime.previous = Some(signals);
            if (preferences.on_level || preferences.on_skill)
                && runtime.last_query.is_none_or(|last| {
                    now.saturating_duration_since(last) >= Duration::from_secs(5)
                })
            {
                link.skill_progress_request();
                runtime.last_query = Some(now);
            }
        }
        let limit = usize::from(preferences.backlog_frames);
        while runtime.frames.len() > limit {
            runtime.frames.pop_front();
        }
        let mut paths = Vec::new();
        if self.screenshot_requested {
            for frame in runtime.frames.drain(..) {
                paths.push(frame.save(&preferences.folder)?);
            }
            runtime.last_frame = Some(now);
            if let Some(path) = self.take_screenshot(source)? {
                paths.push(path);
            }
        } else if link.is_some()
            && limit > 0
            && runtime
                .last_frame
                .is_none_or(|last| now.saturating_duration_since(last) >= Duration::from_secs(1))
        {
            runtime.last_frame = Some(now);
            let pixels = source.capture(preferences.game_only)?;
            runtime.frames.push_back(pixels);
            while runtime.frames.len() > limit
                || runtime
                    .frames
                    .iter()
                    .map(|frame| frame.rgba.len())
                    .sum::<usize>()
                    > 64 << 20
            {
                runtime.frames.pop_front();
            }
        }
        Ok(paths)
    }
}
