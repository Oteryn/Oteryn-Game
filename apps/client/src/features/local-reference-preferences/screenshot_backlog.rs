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
    incoming_ids: Option<Vec<u64>>,
    last_pvp_observation: Option<Instant>,
    last_bestiary_query: Option<Instant>,
    activity: Option<u64>,
    previous: Option<Signals>,
    options: Option<[bool; 13]>,
    trigger_revisions: [u64; 13],
    queried_kills: Option<u64>,
    stages_at_request: Option<oteryn_client_ui::charm_progression::BestiaryStages>,
    automatic_pending: [bool; 13],
    game_only: Option<bool>,
}
struct Signals {
    events: super::CaptureObservations,
    bestiary: Option<oteryn_client_ui::charm_progression::BestiaryStages>,
    combat: crate::screenshots::CombatCaptureFacts,
    deaths: u64,
    achievement: Option<u32>,
    level: Option<u32>,
    skills: Option<[u16; 8]>,
}
impl Signals {
    fn read(link: &PlayLink, bestiary: bool) -> Self {
        let panel = link.skill_progress_panel();
        let facts = link.screenshot_facts();
        Self {
            events: link.capture_observations(),
            bestiary: bestiary.then(|| link.capture_bestiary_stages()).flatten(),
            combat: link.combat_capture_facts(),
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
    pub(in super::super) fn change_screenshot_triggers(&mut self, next: [bool; 13]) {
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
    pub(crate) fn pending_screenshot_triggers(&self) -> [Option<u64>; 13] {
        let runtime = &self.screenshot_runtime;
        let enabled = self.preferences.screenshots.triggers();
        std::array::from_fn(|i| {
            (enabled[i] && runtime.automatic_pending[i]).then_some(runtime.trigger_revisions[i])
        })
    }
    pub fn cancel_screenshot(&mut self) {
        self.screenshot_runtime.automatic_pending = [false; 13];
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
        let signals = link.map(|link| {
            if preferences.automatic_disabled
                || !(preferences.on_bestiary_complete || preferences.on_bestiary_unlock)
            {
                link.invalidate_capture_bestiary();
                self.screenshot_runtime.last_bestiary_query = None;
                self.screenshot_runtime.queried_kills = None;
                self.screenshot_runtime.stages_at_request = None;
            }
            Signals::read(
                link,
                !preferences.automatic_disabled
                    && (preferences.on_bestiary_complete || preferences.on_bestiary_unlock),
            )
        });
        let records = signals.as_ref().map_or([false; 2], |s| {
            let old = self
                .screenshot_runtime
                .previous
                .as_ref()
                .map(|p| p.combat)
                .unwrap_or_default();
            self.observe_impact_records(s.combat, old)
        });
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
        let enabled: [bool; 13] = std::array::from_fn(|i| previous[i] && triggers[i]);
        for (pending, active) in runtime.automatic_pending.iter_mut().zip(enabled) {
            *pending &= active;
        }
        if let (Some(link), Some(signals)) = (link, signals) {
            let bestiary = !preferences.automatic_disabled
                && (preferences.on_bestiary_complete || preferences.on_bestiary_unlock);
            let killed = runtime.queried_kills != Some(signals.events.credited_kills);
            // Each accepted reply publishes a new Arc even when no stage changed.
            // A refused query leaves the previous snapshot intact and still needs a retry.
            let unanswered = runtime
                .stages_at_request
                .as_ref()
                .zip(signals.bestiary.as_ref())
                .is_some_and(|(requested, latest)| Arc::ptr_eq(requested, latest));
            let interval = Duration::from_secs(if killed || unanswered { 2 } else { 4 });
            if bestiary
                && (killed || unanswered || !signals.events.kill_feed || signals.bestiary.is_none())
                && runtime
                    .last_bestiary_query
                    .is_none_or(|at| now.saturating_duration_since(at) >= interval)
                && link.refresh_capture_bestiary()
            {
                runtime.queried_kills = Some(signals.events.credited_kills);
                runtime.stages_at_request.clone_from(&signals.bestiary);
                runtime.last_bestiary_query = Some(now);
            }
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
                    let old_ids =
                        std::mem::replace(&mut runtime.incoming_ids, panel.incoming_ids.clone());
                    let legacy_change = runtime
                        .incoming_situations
                        .replace(incoming.clone())
                        .is_some_and(|old| {
                            incoming.len() > old.len()
                                || incoming
                                    .iter()
                                    .zip(old)
                                    .any(|(current, previous)| *current > previous)
                        });
                    runtime.automatic_pending[3] |= enabled[3]
                        && match (&panel.incoming_ids, old_ids) {
                            (Some(current), Some(previous)) => {
                                oteryn_client_ui::pvp_situation_id::renewed(current, &previous)
                            }
                            _ => legacy_change,
                        };
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
                runtime.incoming_ids = None;
                runtime.last_pvp_query = None;
                runtime.last_pvp_observation = None;
            }
            let (boss_baseline, boss_advanced) = link.boss_capture(
                !signals.events.personal_boss_events
                    && !preferences.automatic_disabled
                    && preferences.on_boss_defeat,
                now,
            );
            runtime.automatic_pending[2] |= enabled[2] && boss_advanced;
            let full = signals.bestiary.is_some();
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
                        oteryn_client_ui::boss_defeated::advanced(
                            signals.events.boss_revision,
                            previous.events.boss_revision,
                        ) || (!signals.events.personal_boss_events
                            && !boss_baseline
                            && signals.events.tracker[2] > previous.events.tracker[2]),
                    ),
                    (
                        6,
                        signals.events.tracked_loot > previous.events.tracked_loot,
                    ),
                    (7, signals.events.low_health > previous.events.low_health),
                    (3, signals.combat.player_hits > previous.combat.player_hits),
                    (4, records[0]),
                    (5, records[1]),
                    (
                        12,
                        signals.combat.reward_collections > previous.combat.reward_collections,
                    ),
                    (8, skill),
                    (9, level),
                    (10, death),
                    (11, achievement),
                ];
                for (i, advanced) in events {
                    runtime.automatic_pending[i] |= enabled[i] && advanced;
                }
                if let Some((old, current)) =
                    previous.bestiary.as_ref().zip(signals.bestiary.as_ref())
                    && !Arc::ptr_eq(old, current)
                {
                    for (race, stage) in current.iter() {
                        let last = old
                            .binary_search_by_key(race, |(id, _)| *id)
                            .map_or(0, |index| old[index].1);
                        let trigger = usize::from(*stage == 3);
                        runtime.automatic_pending[trigger] |= enabled[trigger] && *stage > last;
                    }
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
