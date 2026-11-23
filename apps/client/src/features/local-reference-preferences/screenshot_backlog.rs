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
    last_pvp_observation: Option<Instant>,
    last_bestiary_query: Option<Instant>,
    previous: Option<Signals>,
    queried_kills: Option<u64>,
    stages_at_request: Option<oteryn_client_ui::charm_progression::BestiaryStages>,
    automatic_pending: bool,
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

impl SettingsState {
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
        let runtime = &mut self.screenshot_runtime;
        if preferences.automatic_disabled {
            runtime.automatic_pending = false;
        }
        if link.is_none() || runtime.game_only != Some(preferences.game_only) {
            *runtime = ScreenshotRuntime::default();
            runtime.game_only = Some(preferences.game_only);
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
                    && let Some(count) = panel.incoming_situations
                {
                    runtime.last_pvp_observation = panel.status_received_at;
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
                runtime.last_pvp_observation = None;
            }
            let (boss_baseline, boss_advanced) = link.boss_capture(
                !preferences.automatic_disabled && preferences.on_boss_defeat,
                now,
            );
            runtime.automatic_pending |= boss_advanced;
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
                runtime.automatic_pending |= (preferences.on_death && death)
                    || (preferences.on_bestiary_unlock
                        && !full
                        && signals.events.tracker[0] != previous.events.tracker[0])
                    || (preferences.on_bestiary_complete
                        && !full
                        && signals.events.tracker[1] != previous.events.tracker[1])
                    || (preferences.on_boss_defeat
                        && !boss_baseline
                        && signals.events.tracker[2] != previous.events.tracker[2])
                    || (preferences.on_valuable_loot
                        && previous.events.tracked_loot != signals.events.tracked_loot)
                    || (preferences.on_low_health
                        && previous.events.low_health != signals.events.low_health)
                    || (preferences.on_level && level)
                    || (preferences.on_skill && skill)
                    || (preferences.on_achievement && achievement)
                    || previous
                        .bestiary
                        .as_ref()
                        .zip(signals.bestiary.as_ref())
                        .is_some_and(|(old, current)| {
                            !Arc::ptr_eq(old, current)
                                && current.iter().any(|(race, stage)| {
                                    let last = old
                                        .binary_search_by_key(race, |(id, _)| *id)
                                        .map_or(0, |index| old[index].1);
                                    *stage > last
                                        && if *stage == 3 {
                                            preferences.on_bestiary_complete
                                        } else {
                                            preferences.on_bestiary_unlock
                                        }
                                })
                        })
                    || (preferences.reward_chest
                        && signals.combat.reward_collections > previous.combat.reward_collections)
                    || (preferences.on_record_damage && records[0])
                    || (preferences.on_record_healing && records[1])
                    || (preferences.on_player_attacking
                        && signals.combat.player_hits > previous.combat.player_hits);
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
