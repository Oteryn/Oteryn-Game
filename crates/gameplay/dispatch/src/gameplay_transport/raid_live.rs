//! Channel effects of the node's durable raid firings. The existing monster,
//! Item/Map and server-message owners perform every physical effect.
use super::ComposedFreshAdmission;
use crate::durability::raid_runner::{
    CommittedRaidAction, CommittedRaidStep, RaidCreatureIdentity, RaidFiringCursor,
    RaidStepIdentity,
};
use crate::durability::raid_schedule::RaidWrite;
use crate::foundation::ScopeOwnershipGeneration;
use crate::map::boot::BundleCollisionIndexExt;
use crate::monster_owner_cycle::respawn_job::RespawnExit;
use oteryn_foundation::CancellationToken;
use oteryn_gameplay_encounter_rules::raid::{
    CatalogueSpawnMode, MAX_ANNOUNCEMENTS, MAX_CANDIDATES, MAX_WAVES, RaidCatalogue, SelectedRaid,
};
use oteryn_gameplay_entities::{RaidCreatureStatus, RaidSpawnIdentity, RaidSpawnOutcome};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

struct RaidRetry {
    key: crate::content::ProductionKey,
    terminal: Option<CommittedRaidStep>,
}

#[derive(Default)]
pub(super) struct RaidEffectProgress {
    seen_dead: BTreeSet<RaidSpawnIdentity>,
    closed_bosses: BTreeSet<[u8; 16]>,
    source_aborted: BTreeSet<[u8; 16]>,
    settled_ends: BTreeSet<[u8; 16]>,
    retries: BTreeMap<[u8; 16], RaidRetry>,
}

impl RaidEffectProgress {
    fn begin(&mut self, firing: [u8; 16], key: &crate::content::ProductionKey) -> Result<(), ()> {
        if let Some(retry) = self.retries.get(&firing) {
            return if &retry.key == key { Ok(()) } else { Err(()) };
        }
        if self.retries.len() >= MAX_CANDIDATES {
            return Err(());
        }
        self.retries.insert(
            firing,
            RaidRetry {
                key: key.clone(),
                terminal: None,
            },
        );
        Ok(())
    }

    fn retain_terminal(&mut self, step: &CommittedRaidStep) -> Result<(), ()> {
        let retry = self.retries.get_mut(&step.identity.firing_id).ok_or(())?;
        if let Some(previous) = &retry.terminal {
            if previous.identity == step.identity && previous.action != step.action {
                return Err(());
            }
            if previous.identity.generation == step.identity.generation
                && previous.identity.ordinal > step.identity.ordinal
            {
                return Err(());
            }
        }
        retry.terminal = Some(step.clone());
        Ok(())
    }

    fn terminal(&self, firing: [u8; 16]) -> Option<CommittedRaidStep> {
        self.retries.get(&firing)?.terminal.clone()
    }

    fn retry_work(&self) -> Vec<([u8; 16], crate::content::ProductionKey)> {
        self.retries
            .iter()
            .map(|(&firing, retry)| (firing, retry.key.clone()))
            .collect()
    }

    fn completed(&mut self, firing: [u8; 16]) {
        self.seen_dead
            .retain(|identity| identity.firing_id != firing);
        self.closed_bosses.remove(&firing);
        self.source_aborted.remove(&firing);
        self.settled_ends.remove(&firing);
    }

    fn acknowledge_step(&mut self, step: &CommittedRaidStep) -> bool {
        match step.action {
            CommittedRaidAction::Complete { cleanup } => {
                self.settled_ends.insert(step.identity.firing_id);
                if cleanup {
                    self.closed_bosses.insert(step.identity.firing_id);
                }
                true
            }
            CommittedRaidAction::Lost => {
                self.settled_ends.insert(step.identity.firing_id);
                true
            }
            _ => false,
        }
    }

    fn boss_end_open(&self, firing: [u8; 16]) -> bool {
        !self.closed_bosses.contains(&firing) && !self.source_aborted.contains(&firing)
    }

    fn may_compact(
        &self,
        firing: [u8; 16],
        selected: &SelectedRaid,
        statuses: &[RaidCreatureStatus],
    ) -> bool {
        // Partial boss deaths keep their evidence. A source abort also keeps
        // its failed Single receipt while a surviving boss could otherwise
        // trigger cleanup on retry. Ordinary surviving actors are retained by
        // the carrier's compaction itself.
        self.settled_ends.contains(&firing)
            && (selected.boss_entries.is_empty()
                || self.closed_bosses.contains(&firing)
                || !statuses.iter().any(|status| status.boss && status.alive))
    }
}

fn next_page_firing(
    pending: &mut VecDeque<([u8; 16], crate::content::ProductionKey)>,
    attempted: &BTreeSet<[u8; 16]>,
    progress: &RaidEffectProgress,
) -> Option<([u8; 16], crate::content::ProductionKey)> {
    while let Some((firing, _)) = pending.front() {
        if attempted.contains(firing) {
            pending.pop_front();
            continue;
        }
        // A full retry set applies backpressure without dropping the next
        // SQL row. The saved page also prevents a repeatedly failing prefix
        // from starving later rows when only part of a page fits this poll.
        if !progress.retries.contains_key(firing) && progress.retries.len() >= MAX_CANDIDATES {
            return None;
        }
        return pending.pop_front();
    }
    None
}

fn creature_identity(identity: RaidSpawnIdentity) -> RaidCreatureIdentity {
    RaidCreatureIdentity {
        wave: identity.wave,
        entry: identity.entry,
        index: identity.index,
    }
}

fn has_unrecorded_death(
    statuses: &[RaidCreatureStatus],
    recorded: &BTreeSet<RaidSpawnIdentity>,
) -> bool {
    statuses.iter().any(|status| {
        matches!(status.outcome, RaidSpawnOutcome::Placed(_))
            && !status.alive
            && !recorded.contains(&status.identity)
    })
}

/// All boss placement requests must have terminal receipts before a boss death
/// can close an encounter. Failed placements never count as a killed boss.
fn dead_bosses(
    selected: &SelectedRaid,
    statuses: &[RaidCreatureStatus],
) -> Option<Vec<RaidCreatureIdentity>> {
    let mut dead = Vec::new();
    for &(wave, entry) in &selected.boss_entries {
        let count = selected
            .definition
            .waves
            .get(usize::from(wave))?
            .entries
            .get(usize::from(entry))?
            .count;
        for index in 0..count {
            let status = statuses.iter().find(|status| {
                status.identity.wave == wave
                    && status.identity.entry == entry
                    && status.identity.index == index
                    && status.boss
            })?;
            match status.outcome {
                RaidSpawnOutcome::Placed(_) if status.alive => return None,
                RaidSpawnOutcome::Placed(_) => dead.push(creature_identity(status.identity)),
                RaidSpawnOutcome::Failed => {}
            }
        }
    }
    (!dead.is_empty()).then_some(dead)
}

impl ComposedFreshAdmission<'_, '_, '_> {
    fn raid_bundle_digest(&self) -> Option<[u8; 32]> {
        let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
            self.movement_cells.index()
        else {
            return None;
        };
        Some(index.map().overlay.base().digest())
    }

    async fn apply_raid_step(
        &self,
        step: &CommittedRaidStep,
        selected: Option<&SelectedRaid>,
        seed: [u8; 32],
        now_ms: i64,
        progress: &mut RaidEffectProgress,
    ) -> Result<(), ()> {
        if matches!(
            step.action,
            CommittedRaidAction::Complete { .. } | CommittedRaidAction::Lost
        ) {
            // The ACK may commit even when its result is lost. Retain the
            // exact terminal body before any await, independently of SQL's
            // pending-page filter, until the complete drive reconciles it.
            progress.retain_terminal(step)?;
        }
        let outcomes = match &step.action {
            CommittedRaidAction::Wave { .. } => {
                self.apply_raid_wave(step, selected.ok_or(())?, seed)
                    .await?
            }
            CommittedRaidAction::Announcement { .. } => {
                self.broadcast_raid_notice(step).await?;
                Vec::new()
            }
            CommittedRaidAction::Complete { cleanup } => {
                let mut runtime = self.runtime.lock().await;
                if runtime.binding().scope_generation() != step.identity.generation {
                    return Err(());
                }
                let stamp = runtime.issue_owner_work().map_err(|_| ())?;
                runtime
                    .finish_raid_run(stamp, step.identity.firing_id, *cleanup)
                    .map_err(|_| ())?;
                Vec::new()
            }
            CommittedRaidAction::Lost => {
                let mut runtime = self.runtime.lock().await;
                if runtime.binding().scope_generation() != step.identity.generation {
                    return Err(());
                }
                let stamp = runtime.issue_owner_work().map_err(|_| ())?;
                runtime
                    .finish_raid_run(stamp, step.identity.firing_id, true)
                    .map_err(|_| ())?;
                Vec::new()
            }
        };
        // An uncertain acknowledgement retains the live owner's receipt. The
        // next poll replays that exact result, not a new spawn or announcement.
        match self
            .root
            .acknowledge_raid_effect(self.holder, step.identity, outcomes, now_ms)
            .await
        {
            Ok(RaidWrite::Applied | RaidWrite::Replayed) => {
                if matches!(step.action, CommittedRaidAction::Announcement { .. }) {
                    self.acknowledge_raid_notice(step.identity)?;
                }
                if matches!(
                    step.action,
                    CommittedRaidAction::Complete { .. } | CommittedRaidAction::Lost
                ) {
                    self.acknowledge_raid_notice_end(step.identity)?;
                }
                Ok(())
            }
            Ok(RaidWrite::CompareFailed) | Err(_) => Err(()),
        }
    }

    async fn observe_raid_deaths(
        &self,
        identity: RaidStepIdentity,
        selected: &SelectedRaid,
        progress: &mut RaidEffectProgress,
        seed: [u8; 32],
        now_ms: i64,
    ) -> Result<(), ()> {
        let statuses = {
            let runtime = self.runtime.lock().await;
            if runtime.binding().scope_generation() != identity.generation {
                return Err(());
            }
            runtime.raid_creatures(identity.firing_id).map_err(|_| ())?
        };
        let newly_dead = statuses
            .iter()
            .filter(|status| {
                matches!(status.outcome, RaidSpawnOutcome::Placed(_))
                    && !status.alive
                    && !progress.seen_dead.contains(&status.identity)
            })
            .map(|status| status.identity)
            .collect::<Vec<_>>();
        if !newly_dead.is_empty() {
            let outcomes = newly_dead.iter().copied().map(creature_identity).collect();
            match self
                .root
                .record_raid_creature_deaths(self.holder, identity, outcomes)
                .await
            {
                Ok(RaidWrite::Applied | RaidWrite::Replayed) => {
                    progress.seen_dead.extend(newly_dead)
                }
                Ok(RaidWrite::CompareFailed) | Err(_) => return Err(()),
            }
        }
        if progress.boss_end_open(identity.firing_id)
            && let Some(bosses) = dead_bosses(selected, &statuses)
            && let Some(step) = self
                .root
                .finish_raid_after_boss_death(self.holder, identity, bosses, now_ms)
                .await
                .map_err(|_| ())?
        {
            self.apply_raid_step(&step, Some(selected), seed, now_ms, progress)
                .await?;
            if matches!(step.action, CommittedRaidAction::Complete { cleanup: true }) {
                progress.closed_bosses.insert(identity.firing_id);
                progress.settled_ends.insert(identity.firing_id);
            }
        }
        Ok(())
    }

    async fn compact_settled_raid(
        &self,
        identity: RaidStepIdentity,
        recorded: Option<&BTreeSet<RaidSpawnIdentity>>,
    ) -> Result<Option<bool>, ()> {
        let mut runtime = self.runtime.lock().await;
        if runtime.binding().scope_generation() != identity.generation {
            return Err(());
        }
        // Death can race the earlier SQL observation. Keep any new death
        // receipt until the next poll records it; this check and compaction
        // share the same runtime lock. An ACKed LOST already retired all rows.
        if let Some(recorded) = recorded
            && has_unrecorded_death(
                &runtime.raid_creatures(identity.firing_id).map_err(|_| ())?,
                recorded,
            )
        {
            return Ok(None);
        }
        let stamp = runtime.issue_owner_work().map_err(|_| ())?;
        runtime
            .acknowledge_raid_run_end(stamp, identity.firing_id)
            .map_err(|_| ())?;
        let survivors = runtime
            .raid_creatures(identity.firing_id)
            .map_err(|_| ())?
            .iter()
            .any(|status| status.alive);
        Ok(Some(survivors))
    }

    async fn abort_failed_single(
        &self,
        identity: RaidStepIdentity,
        selected: &SelectedRaid,
        progress: &mut RaidEffectProgress,
        seed: [u8; 32],
        now_ms: i64,
    ) -> Result<bool, ()> {
        let failed = {
            let runtime = self.runtime.lock().await;
            if runtime.binding().scope_generation() != identity.generation {
                return Err(());
            }
            runtime
                .raid_creatures(identity.firing_id)
                .map_err(|_| ())?
                .into_iter()
                .filter(|status| {
                    status.outcome == RaidSpawnOutcome::Failed
                        && selected
                            .spawn_modes
                            .get(&(status.identity.wave, status.identity.entry))
                            == Some(&CatalogueSpawnMode::Single)
                })
                .map(|status| creature_identity(status.identity))
                .collect::<Vec<_>>()
        };
        if failed.is_empty() {
            return Ok(false);
        }
        let Some(step) = self
            .root
            .finish_raid_after_failed_single(self.holder, identity, failed, now_ms)
            .await
            .map_err(|_| ())?
        else {
            return Ok(false);
        };
        self.apply_raid_step(&step, Some(selected), seed, now_ms, progress)
            .await?;
        // The donor stops the sequence without removing earlier actors. Boss
        // death after this source abort must not append a different end policy.
        progress.source_aborted.insert(identity.firing_id);
        progress.settled_ends.insert(identity.firing_id);
        Ok(true)
    }

    async fn apply_current_raid_step(
        &self,
        step: &CommittedRaidStep,
        selected: &SelectedRaid,
        progress: &mut RaidEffectProgress,
        seed: [u8; 32],
        now_ms: i64,
    ) -> Result<bool, ()> {
        self.apply_raid_step(step, Some(selected), seed, now_ms, progress)
            .await?;
        let terminal = progress.acknowledge_step(step);
        if matches!(
            step.action,
            CommittedRaidAction::Wave { .. } | CommittedRaidAction::Complete { cleanup: false }
        ) && self
            .abort_failed_single(step.identity, selected, progress, seed, now_ms)
            .await?
        {
            return Ok(true);
        }
        Ok(terminal)
    }

    pub(super) async fn drive_raid_firing(
        &self,
        firing: [u8; 16],
        key: &crate::content::ProductionKey,
        catalogue: &RaidCatalogue,
        seed: [u8; 32],
        now_ms: i64,
        progress: &mut RaidEffectProgress,
    ) -> Result<(), ()> {
        progress.begin(firing, key)?;
        let result = self
            .drive_retained_raid_firing(firing, key, catalogue, seed, now_ms, progress)
            .await;
        if result.is_ok() {
            progress.retries.remove(&firing);
        }
        result
    }

    async fn drive_retained_raid_firing(
        &self,
        firing: [u8; 16],
        key: &crate::content::ProductionKey,
        catalogue: &RaidCatalogue,
        seed: [u8; 32],
        now_ms: i64,
        progress: &mut RaidEffectProgress,
    ) -> Result<(), ()> {
        let generation: ScopeOwnershipGeneration = {
            let runtime = self.runtime.lock().await;
            runtime.owner_fence().map_err(|_| ())?;
            runtime.binding().scope_generation()
        };
        if let Some(step) = self
            .root
            .claim_lost_raid_run(self.holder, firing, self.channel_id, generation, now_ms)
            .await
            .map_err(|_| ())?
        {
            self.apply_raid_step(&step, None, seed, now_ms, progress)
                .await?;
            self.compact_settled_raid(step.identity, None).await?;
            progress.completed(firing);
            return Ok(());
        }
        if let Some(terminal) = progress.terminal(firing)
            && terminal.action == CommittedRaidAction::Lost
        {
            // A lost run may no longer exist in the admitted catalogue or in
            // SQL's pending page after an uncertain terminal ACK.
            self.apply_raid_step(&terminal, None, seed, now_ms, progress)
                .await?;
            self.compact_settled_raid(terminal.identity, None).await?;
            progress.completed(firing);
            return Ok(());
        }
        let raid = catalogue.get(key).filter(|raid| raid.active).ok_or(())?;
        let selected = raid.select(seed, firing).map_err(|_| ())?;
        let identity = RaidStepIdentity {
            firing_id: firing,
            channel: self.channel_id,
            generation,
            ordinal: 0,
        };
        if let Some(terminal) = progress.terminal(firing) {
            self.apply_current_raid_step(&terminal, &selected, progress, seed, now_ms)
                .await?;
        }
        // Drain every action already due in order. A one-action-per-second loop
        // would stretch a source wave with multiple events beyond its source delay.
        for _ in 0..MAX_WAVES + MAX_ANNOUNCEMENTS + 2 {
            // Resolve a prior physical effect before inspecting its deaths. A
            // boss may have died while its placement ACK was uncertain. This
            // fenced read never commits a new action or advances the cursor.
            if let Some(pending) = self
                .root
                .pending_raid_effect(self.holder, firing, self.channel_id, generation)
                .await
                .map_err(|_| ())?
            {
                if self
                    .apply_current_raid_step(&pending, &selected, progress, seed, now_ms)
                    .await?
                {
                    break;
                }
            } else if self
                .abort_failed_single(identity, &selected, progress, seed, now_ms)
                .await?
            {
                // Includes the replay of an ACKed source abort after the
                // previous future was cancelled before storing local progress.
                break;
            }
            // Every represented placement is now ACKed. Observe an earlier
            // boss death before committing any newly due announcement or wave,
            // even when a delayed poll crosses several source event times.
            self.observe_raid_deaths(identity, &selected, progress, seed, now_ms)
                .await?;
            if !progress.boss_end_open(firing) {
                break;
            }
            let Some(step) = self
                .root
                .advance_raid_run(
                    self.holder,
                    firing,
                    self.channel_id,
                    generation,
                    selected.definition.clone(),
                    now_ms,
                )
                .await
                .map_err(|_| ())?
            else {
                break;
            };
            if self
                .apply_current_raid_step(&step, &selected, progress, seed, now_ms)
                .await?
            {
                break;
            }
        }
        // A live actor may die before its wave acknowledgement succeeds.
        // Record death only after that placement has a durable receipt.
        self.observe_raid_deaths(identity, &selected, progress, seed, now_ms)
            .await?;
        // Retain partial boss-death evidence until all boss requests end. A
        // compacted dead receipt must never hide an unrecorded death or prevent
        // the last boss from closing a persistent encounter.
        let statuses = self
            .runtime
            .lock()
            .await
            .raid_creatures(firing)
            .map_err(|_| ())?;
        if progress.may_compact(firing, &selected, &statuses)
            && let Some(survivors) = self
                .compact_settled_raid(identity, Some(&progress.seen_dead))
                .await?
        {
            progress
                .seen_dead
                .retain(|identity| identity.firing_id != firing);
            if !survivors {
                progress.completed(firing);
            }
        }
        Ok(())
    }

    /// Polled beside the existing world respawn owner for the node's lifetime.
    pub(super) async fn run_raid_effects(&self, shutdown: &CancellationToken) -> RespawnExit {
        let Some(seed) = self.raid_bundle_digest().filter(|_| self.reset_epoch > 0) else {
            shutdown.cancelled().await;
            return RespawnExit::Shutdown;
        };
        let catalogue = match RaidCatalogue::embedded() {
            Ok(catalogue) => catalogue,
            Err(error) => {
                super::operator_event(&format!("raid_catalogue_refused error={error}"));
                return RespawnExit::StaleOwner;
            }
        };
        let mut cursor: Option<RaidFiringCursor> = None;
        let mut pending_page = VecDeque::new();
        let mut progress = RaidEffectProgress::default();
        loop {
            if self.runtime.lock().await.owner_fence().is_err() {
                return RespawnExit::StaleOwner;
            }
            let now = crate::world_clock::now_utc_ms().and_then(|now| i64::try_from(now).ok());
            if let Some(now_ms) = now {
                if self.publish_raid_activity(now_ms).await.is_err() {
                    super::operator_event("raid_activity_projection_retry");
                }
                let mut attempted = BTreeSet::new();
                for (firing, key) in progress.retry_work() {
                    attempted.insert(firing);
                    let result = super::world_respawns::stopping_first(
                        self.drive_raid_firing(
                            firing,
                            &key,
                            &catalogue,
                            seed,
                            now_ms,
                            &mut progress,
                        ),
                        shutdown,
                    )
                    .await;
                    let Some(result) = result else {
                        return RespawnExit::Shutdown;
                    };
                    if result.is_err() {
                        super::operator_event(&format!("raid_effect_retry key={}", key.as_str()));
                    }
                }
                if attempted.len() < MAX_CANDIDATES
                    && pending_page.is_empty()
                    && progress.retries.len() < MAX_CANDIDATES
                {
                    match self.root.page_raid_firings(self.channel_id, cursor).await {
                        Ok(page) => {
                            cursor = page.next;
                            pending_page.extend(page.firings);
                        }
                        Err(error) => {
                            super::operator_event(&format!("raid_effect_read_retry reason={error}"))
                        }
                    }
                }
                while attempted.len() < MAX_CANDIDATES {
                    let Some((firing, key)) =
                        next_page_firing(&mut pending_page, &attempted, &progress)
                    else {
                        break;
                    };
                    attempted.insert(firing);
                    let result = super::world_respawns::stopping_first(
                        self.drive_raid_firing(
                            firing,
                            &key,
                            &catalogue,
                            seed,
                            now_ms,
                            &mut progress,
                        ),
                        shutdown,
                    )
                    .await;
                    let Some(result) = result else {
                        return RespawnExit::Shutdown;
                    };
                    if result.is_err() {
                        super::operator_event(&format!("raid_effect_retry key={}", key.as_str()));
                    }
                }
            }
            if super::world_respawns::stopping_first(
                tokio::time::sleep(std::time::Duration::from_millis(100)),
                shutdown,
            )
            .await
            .is_none()
            {
                return RespawnExit::Shutdown;
            }
        }
    }
}

#[cfg(test)]
#[path = "raid_live_tests.rs"]
mod tests;
