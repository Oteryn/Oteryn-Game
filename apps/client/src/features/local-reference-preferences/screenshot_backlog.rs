//! One frame per second, bounded by age, count and 64 MiB, scoped to the current session.
use super::{CaptureSource, Pixels, SettingsState, screenshot_directory};
use crate::play::PlayLink;
use std::{
    collections::VecDeque,
    io,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
#[derive(Default)]
pub(in super::super) struct ScreenshotRuntime {
    frames: VecDeque<(Instant, Arc<Pixels>)>,
    last_frame: Option<Instant>,
    last_query: Option<Instant>,
    last_pvp_query: Option<Instant>,
    incoming_situations: Option<usize>,
    previous: Option<Signals>,
    bestiary: BestiaryCapture,
    automatic_pending: bool,
    game_only: Option<bool>,
}
#[derive(Default)]
struct BestiaryCapture {
    stages: Option<std::collections::BTreeMap<u32, u8>>,
    queried_kills: Option<u64>,
    last_request: Option<Instant>,
    awaiting: bool,
}
impl BestiaryCapture {
    fn observe(&mut self, now: Instant, link: &PlayLink, kills: u64) -> (bool, bool, bool) {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        let panel = &mailbox.properties.charm_progression;
        if !panel.selected || panel.pending {
            return (panel.selected, false, false);
        }
        let mut unlocked = false;
        let mut completed = false;
        if self.awaiting {
            self.awaiting = false;
            if let Some(reply) = &panel.reply {
                let stages = reply
                    .bestiary
                    .iter()
                    .map(|r| (r.race.get(), r.unlocked_stage()))
                    .collect::<std::collections::BTreeMap<_, _>>();
                if let Some(old) = &self.stages {
                    for (&race, &stage) in &stages {
                        if stage > old.get(&race).copied().unwrap_or(0) {
                            completed |= stage == 3;
                            unlocked |= stage < 3;
                        }
                    }
                }
                self.stages = Some(stages);
            } else {
                self.queried_kills = None;
            }
        }
        if self.queried_kills != Some(kills)
            && self
                .last_request
                .is_none_or(|last| now.saturating_duration_since(last) >= Duration::from_secs(2))
        {
            crate::play::feature_0067_bestiary_charm_progression::input::submit(
                &mut mailbox,
                &link.commands,
                oteryn_client_ui::charm_progression::Intent::Query,
            );
            self.queried_kills = Some(kills);
            self.last_request = Some(now);
            self.awaiting = true;
        }
        (true, unlocked, completed)
    }
}
struct Signals {
    events: super::CaptureObservations,
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
            events: link.capture_observations(),
            deaths: facts.deaths,
            achievement: facts.achievement,
            level: panel.complete.map(|s| s.level),
            skills: panel.snapshot.map(|s| s.skills.map(|skill| skill.level)),
        }
    }
}
pub(crate) enum CaptureAction {
    Idle,
    Buffer,
    Save,
}

pub(crate) struct ScreenshotBatch {
    directory: String,
    frames: VecDeque<(Instant, Arc<Pixels>)>,
}

impl ScreenshotBatch {
    pub(crate) fn save(&self, current: &Pixels) -> io::Result<Vec<PathBuf>> {
        let folder = screenshot_directory(Path::new(&self.directory))?;
        self.frames
            .iter()
            .map(|(_, frame)| frame.as_ref())
            .chain([current])
            .map(|frame| frame.save_to(&folder))
            .collect()
    }
}

impl SettingsState {
    fn observe_impact_records(&mut self, maxima: [u32; 2]) {
        #[cfg(bevy_shell)]
        if self.preference_transfer_pending() {
            return;
        }
        let previous = self.preferences.impact_records;
        if maxima.iter().zip(previous).all(|(new, old)| *new <= old) {
            return;
        }
        let mut preferences = self.preferences.clone();
        let options = &preferences.screenshots;
        self.screenshot_runtime.automatic_pending |= !options.automatic_disabled
            && ((options.on_record_damage && maxima[0] > previous[0])
                || (options.on_record_healing && maxima[1] > previous[1]));
        preferences.impact_records = [maxima[0].max(previous[0]), maxima[1].max(previous[1])];
        self.apply(preferences);
    }
    pub fn cancel_screenshot(&mut self) {
        self.screenshot_runtime.automatic_pending = false;
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
        let action = self.prepare_screenshots(now, link);
        let game_only = self.preferences.screenshots.effective().game_only;
        match action {
            CaptureAction::Idle => Ok(Vec::new()),
            CaptureAction::Buffer => {
                self.buffer_screenshot(now, source.capture(game_only)?);
                Ok(Vec::new())
            }
            CaptureAction::Save => {
                let current = source.capture(game_only)?;
                let batch = self.take_screenshot_batch();
                match batch.save(&current) {
                    Ok(paths) => {
                        self.screenshot_runtime.last_frame = Some(now);
                        self.cancel_screenshot();
                        Ok(paths)
                    }
                    Err(error) => {
                        self.restore_screenshot_batch(batch);
                        Err(error)
                    }
                }
            }
        }
    }

    pub(crate) fn prepare_screenshots(
        &mut self,
        now: Instant,
        link: Option<&PlayLink>,
    ) -> CaptureAction {
        let preferences = self.preferences.screenshots.effective();
        let runtime = &mut self.screenshot_runtime;
        if link.is_none() || runtime.game_only != Some(preferences.game_only) {
            *runtime = ScreenshotRuntime::default();
            runtime.game_only = Some(preferences.game_only);
        }
        if let Some(link) = link
            && self.character_scope().is_some()
        {
            self.observe_impact_records(link.capture_observations().impact);
        }
        let runtime = &mut self.screenshot_runtime;
        if preferences.automatic_disabled {
            runtime.automatic_pending = false;
        }
        if let Some(link) = link {
            if preferences.on_player_attacking && !preferences.automatic_disabled {
                let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
                let panel = &mailbox.properties.party_skull;
                if let Some(count) = panel.incoming_situations {
                    runtime.automatic_pending |= runtime
                        .incoming_situations
                        .replace(count)
                        .is_some_and(|old| count > old);
                }
                if panel.selected
                    && !panel.pending
                    && runtime.last_pvp_query.is_none_or(|last| {
                        now.saturating_duration_since(last) >= Duration::from_secs(2)
                    })
                {
                    crate::play::feature_0134_party_skull::input::submit(
                        &mut mailbox,
                        &link.commands,
                        oteryn_client_ui::party_skull::Intent::PvpStatus,
                    );
                    runtime.last_pvp_query = Some(now);
                }
            } else {
                runtime.incoming_situations = None;
                runtime.last_pvp_query = None;
            }
            let signals = Signals::read(link);
            let (full, unlocked, completed) = if !preferences.automatic_disabled
                && (preferences.on_bestiary_unlock || preferences.on_bestiary_complete)
            {
                runtime
                    .bestiary
                    .observe(now, link, signals.events.credited_kills)
            } else {
                runtime.bestiary = BestiaryCapture::default();
                (false, false, false)
            };
            runtime.automatic_pending |= (preferences.on_bestiary_unlock && unlocked)
                || (preferences.on_bestiary_complete && completed);
            if !preferences.automatic_disabled
                && let Some(previous) = &runtime.previous
            {
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
                runtime.automatic_pending |= (preferences.on_death && death)
                    || (preferences.on_bestiary_unlock
                        && !full
                        && signals.events.tracker[0] != previous.events.tracker[0])
                    || (preferences.on_bestiary_complete
                        && !full
                        && signals.events.tracker[1] != previous.events.tracker[1])
                    || (preferences.on_boss_defeat
                        && signals.events.tracker[2] != previous.events.tracker[2])
                    || (preferences.on_valuable_loot
                        && previous.events.tracked_loot != signals.events.tracked_loot)
                    || (preferences.on_low_health
                        && previous.events.low_health != signals.events.low_health)
                    || (preferences.on_level && level)
                    || (preferences.on_skill && skill)
                    || (preferences.on_achievement && achievement);
            }
            runtime.previous = Some(signals);
            if !preferences.automatic_disabled
                && (preferences.on_level || preferences.on_skill)
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
        if self.screenshot_requested || runtime.automatic_pending {
            CaptureAction::Save
        } else if link.is_some()
            && limit > 0
            && runtime
                .last_frame
                .is_none_or(|last| now.saturating_duration_since(last) >= Duration::from_secs(1))
        {
            CaptureAction::Buffer
        } else {
            CaptureAction::Idle
        }
    }

    pub(crate) fn take_screenshot_batch(&mut self) -> ScreenshotBatch {
        ScreenshotBatch {
            directory: self.preferences.screenshots.effective().directory,
            frames: std::mem::take(&mut self.screenshot_runtime.frames),
        }
    }

    pub(crate) fn restore_screenshot_batch(&mut self, batch: ScreenshotBatch) {
        self.screenshot_runtime.frames = batch.frames;
    }

    pub(crate) fn buffer_screenshot(&mut self, now: Instant, pixels: Pixels) {
        self.screenshot_runtime.last_frame = Some(now);
        let frames = &mut self.screenshot_runtime.frames;
        frames.push_back((now, Arc::new(pixels)));
        let limit = usize::from(self.preferences.screenshots.effective().backlog_frames);
        while frames.len() > limit
            || frames
                .iter()
                .map(|(_, frame)| frame.rgba.len())
                .sum::<usize>()
                > 64 << 20
        {
            frames.pop_front();
        }
    }

    #[cfg(bevy_shell)]
    pub(crate) fn submit_screenshot(&mut self, now: Instant) -> ScreenshotBatch {
        self.screenshot_runtime.last_frame = Some(now);
        self.cancel_screenshot();
        self.take_screenshot_batch()
    }

    #[cfg(bevy_shell)]
    pub(crate) fn manual_screenshot_batch(
        &mut self,
        now: Instant,
        directory: String,
    ) -> (ScreenshotBatch, bool) {
        self.screenshot_runtime.last_frame = Some(now);
        let consumed = !(self.screenshot_requested || self.screenshot_runtime.automatic_pending);
        let frames = if consumed {
            std::mem::take(&mut self.screenshot_runtime.frames)
        } else {
            self.screenshot_runtime.frames.clone()
        };
        (ScreenshotBatch { directory, frames }, consumed)
    }
}
