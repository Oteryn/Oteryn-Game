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
    readback_epoch: u64,
    frames: VecDeque<(Instant, Arc<Pixels>)>,
    last_frame: Option<Instant>,
    last_pvp_query: Option<Instant>,
    incoming_situations: Option<Vec<u64>>,
    last_pvp_observation: Option<Instant>,
    activity: Option<u64>,
    previous: Option<Signals>,
    options: Option<[bool; 12]>,
    trigger_revisions: [u64; 12],
    bestiary: BestiaryCapture,
    automatic_pending: [bool; 12],
    game_only: Option<bool>,
}
#[derive(Default)]
struct BestiaryCapture {
    stages: Option<std::collections::BTreeMap<u32, u8>>,
    queried_kills: Option<u64>,
    last_request: Option<Instant>,
    awaiting: bool,
    reply_revision: u64,
}
impl BestiaryCapture {
    fn observe(&mut self, now: Instant, link: &PlayLink, kills: u64) -> (bool, bool, bool) {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        let kill_feed = mailbox.analyser.selected;
        let panel = &mailbox.properties.charm_progression;
        if !panel.selected || panel.pending {
            return (panel.selected && self.stages.is_some(), false, false);
        }
        let mut unlocked = false;
        let mut completed = false;
        // A retained panel reply can predate the moment this option was enabled.
        if self.last_request.is_none() {
            self.reply_revision = panel.reply_revision;
        }
        if self.reply_revision != panel.reply_revision {
            self.reply_revision = panel.reply_revision;
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
            }
        } else if self.awaiting {
            // The owner retains the previous good reply on a refusal.
            self.queried_kills = None;
        }
        self.awaiting = false;
        let refresh = self.queried_kills != Some(kills)
            || (!kill_feed
                && self
                    .last_request
                    .is_none_or(|at| now.saturating_duration_since(at) >= Duration::from_secs(5)));
        if refresh
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
        (self.stages.is_some(), unlocked, completed)
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

impl PlayLink {
    pub(crate) fn activity_generation(&self) -> u64 {
        crate::play::mailbox::lock(&self.mailbox).bar_actions.scope
    }
}

impl ScreenshotRuntime {
    fn invalidate(&mut self) {
        *self = Self {
            readback_epoch: self.readback_epoch.wrapping_add(1),
            ..Default::default()
        };
    }
}

impl SettingsState {
    fn observe_impact_records(&mut self, maxima: [u32; 2], enabled: [bool; 12]) {
        #[cfg(bevy_shell)]
        if self.preference_transfer_pending() {
            return;
        }
        let previous = self.preferences.impact_records;
        if maxima.iter().zip(previous).all(|(new, old)| *new <= old) {
            return;
        }
        let mut preferences = self.preferences.clone();
        for i in 0..2 {
            self.screenshot_runtime.automatic_pending[4 + i] |=
                enabled[4 + i] && maxima[i] > previous[i];
        }
        preferences.impact_records = [maxima[0].max(previous[0]), maxima[1].max(previous[1])];
        self.apply(preferences);
    }
    pub(in super::super) fn change_screenshot_triggers(&mut self, next: [bool; 12]) {
        let previous = self.preferences.screenshots.triggers();
        let runtime = &mut self.screenshot_runtime;
        for i in 0..next.len() {
            if previous[i] != next[i] {
                runtime.trigger_revisions[i] = runtime.trigger_revisions[i].wrapping_add(1);
                runtime.automatic_pending[i] = false;
                if let Some(options) = &mut runtime.options {
                    options[i] = false;
                }
            }
        }
    }
    #[cfg(bevy_shell)]
    pub(crate) fn pending_screenshot_triggers(&self) -> [Option<u64>; 12] {
        let runtime = &self.screenshot_runtime;
        let enabled = self.preferences.screenshots.triggers();
        std::array::from_fn(|i| {
            (enabled[i] && runtime.automatic_pending[i]).then_some(runtime.trigger_revisions[i])
        })
    }
    pub fn cancel_screenshot(&mut self) {
        self.screenshot_runtime.automatic_pending = [false; 12];
        self.screenshot_requested = false;
    }
    #[cfg(bevy_shell)]
    pub(crate) fn screenshot_readback_epoch(&self) -> u64 {
        self.screenshot_runtime.readback_epoch
    }

    pub fn reset_screenshot_session(&mut self) {
        self.screenshot_runtime.invalidate();
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
        let activity = link.map(PlayLink::activity_generation);
        let runtime = &mut self.screenshot_runtime;
        if link.is_none()
            || runtime.game_only != Some(preferences.game_only)
            || runtime.activity != activity
        {
            runtime.invalidate();
            runtime.game_only = Some(preferences.game_only);
            runtime.activity = activity;
        }
        let triggers = preferences.triggers();
        let previous = runtime.options.replace(triggers).unwrap_or(triggers);
        let enabled = std::array::from_fn(|i| previous[i] && triggers[i]);
        for (pending, active) in runtime.automatic_pending.iter_mut().zip(enabled) {
            *pending &= active;
        }
        if let Some(link) = link
            && self.character_scope().is_some()
        {
            self.observe_impact_records(link.capture_observations().impact, enabled);
        }
        let runtime = &mut self.screenshot_runtime;
        if let Some(link) = link {
            if preferences.on_player_attacking && !preferences.automatic_disabled {
                let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
                let panel = &mailbox.properties.party_skull;
                if runtime.last_pvp_query.is_none() {
                    runtime.last_pvp_observation = panel.status_received_at;
                }
                if panel.status_received_at != runtime.last_pvp_observation
                    && let Some(incoming) = &panel.incoming_situations
                {
                    runtime.last_pvp_observation = panel.status_received_at;
                    runtime.automatic_pending[3] |= enabled[3]
                        && runtime
                            .incoming_situations
                            .replace(incoming.clone())
                            .is_some_and(|old| {
                                incoming.len() > old.len()
                                    || incoming
                                        .iter()
                                        .zip(old)
                                        .any(|(current, previous)| *current > previous)
                            });
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
                runtime.last_pvp_observation = None;
            }
            let (boss_baseline, boss_advanced) = link.boss_capture(
                !preferences.automatic_disabled && preferences.on_boss_defeat,
                now,
            );
            runtime.automatic_pending[2] |= enabled[2] && boss_advanced;
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
            runtime.automatic_pending[0] |= enabled[0] && unlocked;
            runtime.automatic_pending[1] |= enabled[1] && completed;
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
                let events = [
                    (
                        0,
                        !full && signals.events.tracker[0] > previous.events.tracker[0],
                    ),
                    (
                        1,
                        !full && signals.events.tracker[1] > previous.events.tracker[1],
                    ),
                    (
                        2,
                        !boss_baseline && signals.events.tracker[2] > previous.events.tracker[2],
                    ),
                    (
                        6,
                        signals.events.tracked_loot > previous.events.tracked_loot,
                    ),
                    (7, signals.events.low_health > previous.events.low_health),
                    (8, skill),
                    (9, level),
                    (10, death),
                    (11, achievement),
                ];
                for (i, advanced) in events {
                    runtime.automatic_pending[i] |= enabled[i] && advanced;
                }
            }
            runtime.previous = Some(signals);
            if !preferences.automatic_disabled && (preferences.on_level || preferences.on_skill) {
                link.poll_skill_progress(Duration::from_secs(2));
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
        if self.screenshot_requested || runtime.automatic_pending.contains(&true) {
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
        let consumed = !(self.screenshot_requested
            || self.screenshot_runtime.automatic_pending.contains(&true));
        let frames = if consumed {
            std::mem::take(&mut self.screenshot_runtime.frames)
        } else {
            self.screenshot_runtime.frames.clone()
        };
        (ScreenshotBatch { directory, frames }, consumed)
    }
}
