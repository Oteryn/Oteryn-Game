//! One frame per second, bounded by age, count and 64 MiB, scoped to the current session.
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
    frames: VecDeque<(Instant, Pixels)>,
    last_frame: Option<Instant>,
    last_query: Option<Instant>,
    previous: Option<Signals>,
    game_only: Option<bool>,
}
struct Signals {
    deaths: u64,
    achievement: Option<u32>,
    level: Option<u32>,
    skills: Option<[u16; 8]>,
}
impl Signals {
    fn read(link: &PlayLink) -> Self {
        let panel = link.skill_progress_panel();
        let facts = link.screenshot_facts();
        Self {
            deaths: facts.deaths,
            achievement: facts.achievement,
            level: panel.complete.map(|s| s.level),
            skills: panel.snapshot.map(|s| s.skills.map(|skill| skill.level)),
        }
    }
}
impl SettingsState {
    pub fn cancel_screenshot(&mut self) {
        self.screenshot_requested = false;
    }
    pub fn reset_screenshot_session(&mut self) {
        self.screenshot_runtime = ScreenshotRuntime::default();
        self.screenshot_requested = false;
    }
    pub fn process_screenshots(
        &mut self,
        now: Instant,
        link: Option<&PlayLink>,
        source: &mut impl CaptureSource,
    ) -> io::Result<Vec<PathBuf>> {
        let preferences = self.preferences.screenshots.effective();
        let runtime = &mut self.screenshot_runtime;
        if link.is_none() || runtime.game_only != Some(preferences.game_only) {
            *runtime = ScreenshotRuntime::default();
            runtime.game_only = Some(preferences.game_only);
        }
        if let Some(link) = link {
            let signals = Signals::read(link);
            if let Some(previous) = &runtime.previous {
                let death = signals.deaths > previous.deaths;
                let level = previous
                    .level
                    .zip(signals.level)
                    .is_some_and(|(old, new)| new > old);
                let skill = previous
                    .skills
                    .zip(signals.skills)
                    .is_some_and(|(old, new)| old.into_iter().zip(new).any(|(a, b)| b > a));
                let achievement = previous
                    .achievement
                    .zip(signals.achievement)
                    .is_some_and(|(old, new)| new > old);
                self.screenshot_requested |= (preferences.on_death && death)
                    || (preferences.on_level && level)
                    || (preferences.on_skill && skill)
                    || (preferences.on_achievement && achievement);
            }
            runtime.previous = Some(signals);
            if (preferences.on_level || preferences.on_skill)
                && runtime.last_query.is_none_or(|last| {
                    now.saturating_duration_since(last) >= Duration::from_secs(2)
                })
            {
                link.skill_progress_request();
                runtime.last_query = Some(now);
            }
        }
        let limit = usize::from(preferences.backlog_frames);
        while runtime.frames.len() > limit
            || (preferences.backlog_seconds > 0
                && runtime.frames.front().is_some_and(|(at, _)| {
                    now.saturating_duration_since(*at)
                        > Duration::from_secs(u64::from(preferences.backlog_seconds))
                }))
        {
            runtime.frames.pop_front();
        }
        let mut paths = Vec::new();
        if self.screenshot_requested {
            for (_, frame) in &runtime.frames {
                paths.push(frame.save(&preferences.folder)?);
            }
            paths.push(
                source
                    .capture(preferences.game_only)?
                    .save(&preferences.folder)?,
            );
            runtime.frames.clear();
            runtime.last_frame = Some(now);
            self.screenshot_requested = false;
        } else if link.is_some()
            && limit > 0
            && runtime
                .last_frame
                .is_none_or(|last| now.saturating_duration_since(last) >= Duration::from_secs(1))
        {
            runtime.last_frame = Some(now);
            let pixels = source.capture(preferences.game_only)?;
            runtime.frames.push_back((now, pixels));
            while runtime.frames.len() > limit
                || runtime
                    .frames
                    .iter()
                    .map(|(_, frame)| frame.rgba.len())
                    .sum::<usize>()
                    > 64 << 20
            {
                runtime.frames.pop_front();
            }
        }
        Ok(paths)
    }
}
