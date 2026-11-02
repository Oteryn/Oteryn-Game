//! Composition of the existing schedule planner with current node, World reset,
//! admission and channel-target facts. It creates no actors or protocol messages.
use super::{
    CalendarRaidScheduleCandidate, LuaRaidScheduleCandidate, ProductionKey, RaidSchedule, WorldId,
    calendar, lua, storage,
};
use crate::durability::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use crate::durability::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use crate::durability::runtime_scope_assignment::{
    NodeIncarnationProof, prove_current_incarnation,
};
use crate::durability::{DurabilityError, DurabilityRoot};
use crate::foundation::{ChannelId, ScopeOwnershipGeneration};
use oteryn_gameplay_encounter_rules::raid::{
    CalendarDay, FiringState, MAX_CANDIDATES, RaidAction, RaidDefinition, RaidPolicy,
};
use sqlx::Row;
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct RaidScheduleCandidate {
    pub key: ProductionKey,
    pub definition: RaidDefinition,
    pub eligible: bool,
    pub calendar: Option<CalendarDay>,
    pub firing_id: [u8; 16],
    /// Crystal raids.cpp:134 compares this margin with the preceding global
    /// raid's end, independently of this raid's own minimum firing gap.
    pub global_margin_ms: u32,
    /// Modern Lua has an independent per-raid gate and persisted adaptive chance.
    pub lua: Option<LuaRaidScheduleCandidate>,
    pub calendar_window: Option<CalendarRaidScheduleCandidate>,
}

#[derive(Debug, Clone)]
pub struct RaidSchedulerCheck {
    pub world: WorldId,
    pub channel: ChannelId,
    pub generation: ScopeOwnershipGeneration,
    pub reset_epoch: u64,
    pub now_ms: i64,
    pub bundle_digest: [u8; 32],
    /// Source order is significant for the legacy global first-winner check.
    pub candidates: Vec<RaidScheduleCandidate>,
}

#[derive(Debug, Clone, Default)]
pub struct RaidSchedulerOutcome {
    pub firings: Vec<([u8; 16], ProductionKey)>,
    /// Only a due Lua predicate awaiting a fresh current-Channel projection.
    /// The caller may retry promptly without changing the source check cadence.
    pub retry_activity: bool,
}

impl DurabilityRoot {
    pub async fn check_raid_schedules(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        request: RaidSchedulerCheck,
    ) -> Result<RaidSchedulerOutcome, DurabilityError> {
        if request.now_ms < 0
            || request.reset_epoch == 0
            || request.candidates.len() > MAX_CANDIDATES
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let mut keys = BTreeSet::new();
        for candidate in &request.candidates {
            candidate
                .definition
                .validate()
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            if !keys.insert(candidate.key.as_str())
                || (candidate.lua.is_some() && candidate.calendar_window.is_some())
                || ((candidate.lua.is_some() || candidate.calendar_window.is_some())
                    != (candidate.definition.policy == RaidPolicy::SourceScheduled))
            {
                return Err(DurabilityError::InvalidStoredState);
            }
            if let Some(calendar) = &candidate.calendar_window {
                calendar
                    .window
                    .validate()
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                if calendar.interval_ms == 0 {
                    return Err(DurabilityError::InvalidStoredState);
                }
            }
        }
        drop(keys);
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if !prove_current_incarnation(&mut tx, &node).await? {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(RaidSchedulerOutcome::default());
            }
            let reset_current: bool = sqlx::query_scalar("SELECT game_world_reset_map_epoch_current(encode($1,'hex')::uuid,$2::text::numeric)")
                .bind(request.world.as_bytes().as_slice()).bind(request.reset_epoch.to_string())
                .fetch_one(&mut *tx).await?;
            let active = sqlx::query("SELECT a.reset_epoch::text,e.target_digest FROM game_world_reset_active a JOIN game_world_reset_epochs e USING(world_id,reset_epoch) WHERE a.world_id=encode($1,'hex')::uuid")
                .bind(request.world.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let active_matches = active.as_ref().is_some_and(|row| {
                row.try_get::<String, _>("reset_epoch").ok().as_deref() == Some(&request.reset_epoch.to_string())
                    && row.try_get::<Vec<u8>, _>("target_digest").ok().as_deref() == Some(request.bundle_digest.as_slice())
            });
            let fact = node.fact();
            let generation: Option<String> = sqlx::query_scalar("SELECT a.ownership_generation::text FROM game_runtime_scope_assignments a JOIN game_durability_admission_runtime_guards g USING(scope_key) WHERE a.world_id=encode($1,'hex')::uuid AND a.channel_id=encode($2,'hex')::uuid AND a.scope_kind=1 AND a.state=1 AND a.holder_node_id=encode($3,'hex')::uuid AND a.holder_registration_revision=$4::text::numeric AND g.ready AND g.ownership_generation=a.ownership_generation FOR SHARE OF a,g")
                .bind(request.world.as_bytes().as_slice()).bind(request.channel.as_bytes().as_slice())
                .bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string())
                .fetch_optional(&mut *tx).await?;
            if !reset_current || !active_matches || generation.as_deref() != Some(&request.generation.get().to_string()) {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(RaidSchedulerOutcome::default());
            }
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:raid-world:' || encode($1,'hex'),0))")
                .bind(request.world.as_bytes().as_slice()).execute(&mut *tx).await?;
            // Retire previous-epoch firings before checking a new admitted epoch.
            // Their channel effects become LOST cleanup on the normal runner path.
            sqlx::query("UPDATE game_world_raid_firings SET state=3 WHERE world_id=encode($1,'hex')::uuid AND reset_epoch<>$2::text::numeric AND state=1")
                .bind(request.world.as_bytes().as_slice()).bind(request.reset_epoch.to_string()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_world_raid_scheduler_clocks(world_id,reset_epoch,last_end_at_ms) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3) ON CONFLICT(world_id) DO UPDATE SET reset_epoch=EXCLUDED.reset_epoch,last_end_at_ms=EXCLUDED.last_end_at_ms WHERE game_world_raid_scheduler_clocks.reset_epoch<EXCLUDED.reset_epoch")
                .bind(request.world.as_bytes().as_slice()).bind(request.reset_epoch.to_string()).bind(request.now_ms).execute(&mut *tx).await?;
            crate::durability::raid_runner::reconcile_deadlines(&mut tx, request.world, request.now_ms).await?;
            let last_end: i64 = sqlx::query_scalar("SELECT last_end_at_ms FROM game_world_raid_scheduler_clocks WHERE world_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(request.world.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            let mut legacy_running: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_world_raid_firings f WHERE f.world_id=encode($1,'hex')::uuid AND f.state=1 AND NOT EXISTS(SELECT 1 FROM game_world_raid_schedule_sources s WHERE s.world_id=f.world_id AND s.raid_key=f.raid_key AND s.source_kind<>1))")
                .bind(request.world.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            let mut fired = Vec::new();
            let mut retry_activity = false;
            for candidate in request.candidates {
                let source_kind: i16 = if candidate.lua.is_some() { 2 }
                    else if candidate.calendar_window.is_some() { 3 } else { 1 };
                if (source_kind == 1 && legacy_running)
                    || (source_kind != 1 && !candidate.eligible)
                { continue; }
                super::lock(&mut tx, &recovery, request.world, &candidate.key).await
                    .map_err(|_| DurabilityError::Unavailable)?;
                let mut before = match storage::load(&mut tx, request.world, &candidate.key).await
                    .map_err(|_| DurabilityError::InvalidStoredState)? {
                    Some(before) => before,
                    None => {
                        let initial = RaidSchedule::initial(request.reset_epoch, request.now_ms)
                            .map_err(|_| DurabilityError::InvalidStoredState)?;
                        storage::insert_schedule(&mut tx, request.world, &candidate.key, &initial).await
                            .map_err(|_| DurabilityError::Unavailable)?;
                        initial
                    }
                };
                sqlx::query("INSERT INTO game_world_raid_schedule_sources(world_id,raid_key,source_kind) VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT DO NOTHING")
                    .bind(request.world.as_bytes().as_slice()).bind(candidate.key.as_str()).bind(source_kind).execute(&mut *tx).await?;
                let recorded_source: i16 = sqlx::query_scalar("SELECT source_kind FROM game_world_raid_schedule_sources WHERE world_id=encode($1,'hex')::uuid AND raid_key=$2")
                    .bind(request.world.as_bytes().as_slice()).bind(candidate.key.as_str()).fetch_one(&mut *tx).await?;
                if recorded_source != source_kind { return Err(DurabilityError::InvalidStoredState); }
                if before.reset_epoch != request.reset_epoch {
                    let reset = before.plan(request.world, &candidate.key, &candidate.definition,
                        RaidAction::Reset { reset_epoch: request.reset_epoch, now_ms: request.now_ms })
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    storage::write_transition(&mut tx, request.world, &candidate.key, &reset).await
                        .map_err(|_| DurabilityError::Unavailable)?;
                    before = reset.after().clone();
                }
                if before.retired { continue; }
                let action = if let Some(lua_candidate) = &candidate.lua {
                    let running = before.latest_firing.as_ref()
                        .is_some_and(|firing| firing.state == FiringState::Running);
                    let selected = match lua::selected(&mut tx, request.world, &candidate.key,
                        lua_candidate, request.now_ms, before.next_check_at_ms, running).await? {
                        lua::LuaSelection::NotDue => continue,
                        lua::LuaSelection::AwaitActivity => {
                            retry_activity = true;
                            continue;
                        }
                        lua::LuaSelection::Selected(selected) => selected,
                    };
                    RaidAction::ResolvedCheck {
                        now_ms: request.now_ms, selected,
                        firing_id: candidate.firing_id, bundle_digest: request.bundle_digest,
                    }
                } else if let Some(window) = &candidate.calendar_window {
                    let clock = calendar::clock(&mut tx, request.world, request.now_ms).await?;
                    let in_window = window.window.contains(clock.month, clock.month_day)
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    let interval_due = before.last_fired_at_ms.is_none_or(|last|
                        request.now_ms.saturating_sub(last) >= i64::from(window.interval_ms));
                    let persistent_fired = candidate.definition.despawn_after_ms.is_none()
                        && before.latest_firing.as_ref().is_some_and(|f| f.reset_epoch == request.reset_epoch);
                    RaidAction::ResolvedCheck {
                        now_ms: request.now_ms, selected: in_window && interval_due && !persistent_fired,
                        firing_id: candidate.firing_id, bundle_digest: request.bundle_digest,
                    }
                } else {
                    let margin_end = last_end.checked_add(i64::from(candidate.global_margin_ms))
                        .ok_or(DurabilityError::InvalidStoredState)?;
                    if request.now_ms < margin_end { continue; }
                    RaidAction::Check {
                        now_ms: request.now_ms, eligible: candidate.eligible, calendar: candidate.calendar,
                        firing_id: candidate.firing_id, bundle_digest: request.bundle_digest,
                    }
                };
                let planned = before.plan(request.world, &candidate.key, &candidate.definition, action)
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                if planned.after() == planned.before() { continue; }
                storage::write_transition(&mut tx, request.world, &candidate.key, &planned).await
                    .map_err(|_| DurabilityError::Unavailable)?;
                if planned.after().next_firing_ordinal != planned.before().next_firing_ordinal {
                    let targets = sqlx::query("INSERT INTO game_world_raid_targets(firing_id,channel_id,owner_generation) SELECT encode($1,'hex')::uuid,a.channel_id,a.ownership_generation FROM game_runtime_scope_assignments a JOIN game_durability_admission_runtime_guards g USING(scope_key) WHERE a.world_id=encode($2,'hex')::uuid AND a.scope_kind=1 AND a.state=1 AND g.ready AND g.ownership_generation=a.ownership_generation AND game_node_lock_current_registration(a.holder_node_id,a.holder_registration_revision) ORDER BY a.channel_id FOR SHARE OF a,g")
                        .bind(candidate.firing_id.as_slice()).bind(request.world.as_bytes().as_slice())
                        .execute(&mut *tx).await?;
                    if targets.rows_affected() == 0 { return Err(DurabilityError::InvalidStoredState); }
                    if source_kind == 1 { legacy_running = true; }
                    fired.push((candidate.firing_id, candidate.key));
                }
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(RaidSchedulerOutcome { firings: fired, retry_activity })
        })).await
    }
}
