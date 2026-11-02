//! Fenced durable execution of a channel raid run and its acknowledged live effects.
mod effects;
mod end;
mod live_guard;
mod pending;
pub use live_guard::raid_effect_pending_in_transaction;

use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::raid_schedule::RaidWrite;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::{ChannelId, ScopeOwnershipGeneration};
use oteryn_gameplay_content::content::ProductionKey;
use oteryn_gameplay_encounter_rules::raid::{
    MAX_CANDIDATES, RaidDefinition, RaidRunAction, RaidRunCursor,
};
use sqlx::Row;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RaidStepIdentity {
    pub firing_id: [u8; 16],
    pub channel: ChannelId,
    /// Independently checked current generation. LOST cleanup may settle an older run.
    pub generation: ScopeOwnershipGeneration,
    pub ordinal: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RaidCreatureIdentity {
    pub wave: u16,
    pub entry: u16,
    pub index: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaidCreaturePlan {
    pub identity: RaidCreatureIdentity,
    pub creature: ProductionKey,
    pub area_anchor: ProductionKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RaidPlacementOutcome {
    pub identity: RaidCreatureIdentity,
    pub placed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommittedRaidAction {
    Announcement {
        text: String,
        advance: bool,
    },
    Wave {
        wave: u16,
        inserted: u32,
        creatures: Vec<RaidCreaturePlan>,
        boss_step: bool,
    },
    Complete {
        cleanup: bool,
    },
    Lost,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedRaidStep {
    pub identity: RaidStepIdentity,
    pub action: CommittedRaidAction,
    pub replayed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RaidFiringCursor {
    pub fired_at_ms: i64,
    pub firing_id: [u8; 16],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaidFiringPage {
    pub firings: Vec<([u8; 16], ProductionKey)>,
    pub next: Option<RaidFiringCursor>,
}

/// Called only inside the scheduler's independently fenced World transaction.
/// Page stale or missing targets; current live runs retain their due-step queue.
pub(super) async fn reconcile_deadlines(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    world: crate::foundation::WorldId,
    now_ms: i64,
) -> Result<(), DurabilityError> {
    let firings = sqlx::query("SELECT uuid_send(firing_id) FROM game_world_raid_firings WHERE world_id=encode($1,'hex')::uuid AND state=1 AND deadline_ms<=$2 ORDER BY fired_at_ms,firing_id LIMIT 32 FOR UPDATE")
        .bind(world.as_bytes().as_slice()).bind(now_ms).fetch_all(&mut **tx).await?;
    for firing in firings {
        let firing: [u8; 16] = firing
            .try_get::<Vec<u8>, _>(0)?
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let targets = sqlx::query("SELECT uuid_send(t.channel_id),t.owner_generation::text,COALESCE(r.committed_steps,0) AS steps FROM game_world_raid_targets t LEFT JOIN game_world_raid_channel_runs r USING(firing_id,channel_id) WHERE t.firing_id=encode($1,'hex')::uuid AND (r.state IS NULL OR (r.state=1 AND NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments a JOIN game_durability_admission_runtime_guards g USING(scope_key) JOIN game_node_registrations n ON n.node_id=a.holder_node_id AND n.registration_revision=a.holder_registration_revision WHERE a.world_id=encode($2,'hex')::uuid AND a.channel_id=t.channel_id AND a.scope_kind=1 AND a.state=1 AND a.ownership_generation=t.owner_generation AND g.ownership_generation=a.ownership_generation AND g.ready AND n.state=1))) ORDER BY t.channel_id LIMIT 32")
            .bind(firing.as_slice()).bind(world.as_bytes().as_slice()).fetch_all(&mut **tx).await?;
        for target in targets {
            let channel_bytes: [u8; 16] = target
                .try_get::<Vec<u8>, _>(0)?
                .try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            let channel = ChannelId::decode(&channel_bytes)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            let owner: String = target.try_get(1)?;
            let generation = ScopeOwnershipGeneration::new(
                owner
                    .parse()
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
            )
            .map_err(|_| DurabilityError::InvalidStoredState)?;
            let ordinal = u32::try_from(target.try_get::<i32, _>("steps")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            sqlx::query("INSERT INTO game_world_raid_channel_runs(firing_id,channel_id,owner_generation,updated_at_ms) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4) ON CONFLICT DO NOTHING")
                .bind(firing.as_slice()).bind(channel.as_bytes().as_slice()).bind(&owner).bind(now_ms).execute(&mut **tx).await?;
            let identity = RaidStepIdentity {
                firing_id: firing,
                channel,
                generation,
                ordinal,
            };
            effects::lose(tx, identity, 1, &owner, now_ms).await?;
        }
        // Offline target cleanup is retained for its next owner. It cannot hold
        // every other live channel's scheduler open forever after the deadline.
        let closed = sqlx::query("UPDATE game_world_raid_firings f SET state=2 WHERE firing_id=encode($1,'hex')::uuid AND state=1 AND EXISTS(SELECT 1 FROM game_world_raid_targets t WHERE t.firing_id=f.firing_id) AND NOT EXISTS(SELECT 1 FROM game_world_raid_targets t LEFT JOIN game_world_raid_channel_runs r USING(firing_id,channel_id) WHERE t.firing_id=f.firing_id AND (r.state IS NULL OR r.state=1)) AND NOT EXISTS(SELECT 1 FROM game_world_raid_effects e WHERE e.firing_id=f.firing_id AND e.state=1 AND e.action_kind<>4)")
            .bind(firing.as_slice()).execute(&mut **tx).await?;
        if closed.rows_affected() != 0 {
            sqlx::query("UPDATE game_world_raid_scheduler_clocks SET last_end_at_ms=GREATEST(last_end_at_ms,$2) WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=(SELECT reset_epoch FROM game_world_raid_firings WHERE firing_id=encode($3,'hex')::uuid) AND NOT EXISTS(SELECT 1 FROM game_world_raid_firings f JOIN game_world_raid_schedule_sources s USING(world_id,raid_key) WHERE f.firing_id=encode($3,'hex')::uuid AND s.source_kind<>1)")
                .bind(world.as_bytes().as_slice()).bind(now_ms).bind(firing.as_slice()).execute(&mut **tx).await?;
        }
    }
    Ok(())
}

impl DurabilityRoot {
    /// An obsolete firing can be retired without reconstructing a removed or
    /// now-inactive catalogue definition. A current same-generation run is
    /// never abandoned by this metadata-free path.
    pub async fn claim_lost_raid_run(
        &self,
        node: &NodeIncarnationProof,
        firing_id: [u8; 16],
        channel: ChannelId,
        generation: ScopeOwnershipGeneration,
        now_ms: i64,
    ) -> Result<Option<CommittedRaidStep>, DurabilityError> {
        if now_ms < 0 {
            return Err(DurabilityError::InvalidStoredState);
        }
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let active_world = effects::world_current(&mut tx, firing_id).await?;
            if !effects::current(&mut tx, &node, firing_id, channel, generation).await? {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(None);
            }
            let firing_state: i16 = sqlx::query_scalar("SELECT state FROM game_world_raid_firings WHERE firing_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(firing_id.as_slice()).fetch_one(&mut *tx).await?;
            let owner: Option<String> = sqlx::query_scalar("SELECT owner_generation::text FROM game_world_raid_targets WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid")
                .bind(firing_id.as_slice()).bind(channel.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let Some(owner) = owner else {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(None);
            };
            let previously_lost: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_world_raid_channel_runs WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND state=3)")
                .bind(firing_id.as_slice()).bind(channel.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            if active_world && owner == generation.get().to_string() && firing_state != 3 && !previously_lost {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(None);
            }
            sqlx::query("INSERT INTO game_world_raid_channel_runs(firing_id,channel_id,owner_generation,updated_at_ms) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4) ON CONFLICT DO NOTHING")
                .bind(firing_id.as_slice()).bind(channel.as_bytes().as_slice()).bind(&owner).bind(now_ms).execute(&mut *tx).await?;
            let run = sqlx::query("SELECT committed_steps,state,updated_at_ms FROM game_world_raid_channel_runs WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid FOR UPDATE")
                .bind(firing_id.as_slice()).bind(channel.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            if now_ms < run.try_get::<i64, _>("updated_at_ms")? { return Err(DurabilityError::InvalidStoredState); }
            let ordinal = u32::try_from(run.try_get::<i32, _>("committed_steps")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            let identity = RaidStepIdentity { firing_id, channel, generation, ordinal };
            let result = effects::lose(&mut tx, identity, run.try_get("state")?, &owner, now_ms).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(result)
        })).await
    }

    pub async fn query_raid_schedule(
        &self,
        channel: ChannelId,
    ) -> Result<oteryn_protocol_oteryn::raid_schedule::RaidScheduleState, DurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let row = sqlx::query("SELECT f.raid_key,r.state,r.committed_steps,(SELECT count(*) FROM game_world_raid_creatures c JOIN game_world_raid_creature_placements p USING(firing_id,channel_id,wave,entry,creature_index) WHERE c.firing_id=r.firing_id AND c.channel_id=r.channel_id AND c.state=1 AND p.placed)::text FROM game_world_raid_channel_runs r JOIN game_world_raid_firings f USING(firing_id) JOIN game_runtime_scope_assignments a ON a.world_id=f.world_id AND a.channel_id=r.channel_id AND a.scope_kind=1 AND a.state=1 WHERE r.channel_id=encode($1,'hex')::uuid ORDER BY f.fired_at_ms DESC,f.firing_id DESC LIMIT 1")
                .bind(channel.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let state = if let Some(row) = row {
                oteryn_protocol_oteryn::raid_schedule::RaidScheduleState {
                    raid_key: row.try_get(0)?, running: row.try_get::<i16, _>(1)? == 1,
                    committed_steps: u32::try_from(row.try_get::<i32, _>(2)?)
                        .map_err(|_| DurabilityError::InvalidStoredState)?,
                    active_creatures: row.try_get::<String, _>(3)?.parse()
                        .map_err(|_| DurabilityError::InvalidStoredState)?,
                }
            } else {
                oteryn_protocol_oteryn::raid_schedule::RaidScheduleState {
                    raid_key: String::new(), running: false, committed_steps: 0, active_creatures: 0,
                }
            };
            commit_semantic_transaction(tx, deadline).await?;
            Ok(state)
        })).await
    }

    /// A pending step is returned verbatim until acknowledged. The live owner
    /// deduplicates the identity before touching its actor map or queuing messages;
    /// SQL commit alone does not establish delivery or successful placement.
    pub async fn advance_raid_run(
        &self,
        node: &NodeIncarnationProof,
        firing_id: [u8; 16],
        channel: ChannelId,
        generation: ScopeOwnershipGeneration,
        definition: RaidDefinition,
        now_ms: i64,
    ) -> Result<Option<CommittedRaidStep>, DurabilityError> {
        if now_ms < 0 {
            return Err(DurabilityError::InvalidStoredState);
        }
        definition
            .validate()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let world_current = effects::world_current(&mut tx, firing_id).await?;
            if !effects::current(&mut tx, &node, firing_id, channel, generation).await? {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(None);
            }
            let firing = sqlx::query("SELECT fired_at_ms,state FROM game_world_raid_firings WHERE firing_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(firing_id.as_slice()).fetch_one(&mut *tx).await?;
            let target: Option<String> = sqlx::query_scalar("SELECT owner_generation::text FROM game_world_raid_targets WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid")
                .bind(firing_id.as_slice()).bind(channel.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let Some(target_generation) = target else {
                // A channel activated after the firing does not join it.
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(None);
            };
            sqlx::query("INSERT INTO game_world_raid_channel_runs(firing_id,channel_id,owner_generation,updated_at_ms) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4) ON CONFLICT DO NOTHING")
                .bind(firing_id.as_slice()).bind(channel.as_bytes().as_slice()).bind(&target_generation).bind(now_ms).execute(&mut *tx).await?;
            let row = sqlx::query("SELECT owner_generation::text,committed_steps,state,updated_at_ms FROM game_world_raid_channel_runs WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid FOR UPDATE")
                .bind(firing_id.as_slice()).bind(channel.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            let owner: String = row.try_get("owner_generation")?;
            let steps = u32::try_from(row.try_get::<i32, _>("committed_steps")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            let state: i16 = row.try_get("state")?;
            if now_ms < row.try_get::<i64, _>("updated_at_ms")? {
                return Err(DurabilityError::InvalidStoredState);
            }
            let identity = RaidStepIdentity { firing_id, channel, generation, ordinal: steps };
            if !world_current || owner != generation.get().to_string() || firing.try_get::<i16, _>("state")? == 3 || state == 3 {
                let effect = effects::lose(&mut tx, identity, state, &owner, now_ms).await?;
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(effect);
            }
            if let Some(pending) = effects::pending(&mut tx, identity).await? {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(Some(pending));
            }
            if state != 1 || firing.try_get::<i16, _>("state")? != 1 {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(None);
            }
            let cursor = RaidRunCursor { committed_steps: steps };
            let Some(step) = cursor.next_due(&definition, firing.try_get("fired_at_ms")?, now_ms)
                .map_err(|_| DurabilityError::InvalidStoredState)? else {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(None);
            };
            let action = match &step.action {
                RaidRunAction::Announce { text, kind } => CommittedRaidAction::Announcement {
                    text: (*text).to_owned(),
                    advance: matches!(kind, oteryn_gameplay_encounter_rules::raid::AnnouncementKind::Advance),
                },
                RaidRunAction::SpawnWave { wave, entries, boss_step } => {
                    let mut creatures = Vec::new();
                    for (entry, value) in entries.iter().enumerate() {
                        let entry = u16::try_from(entry).map_err(|_| DurabilityError::InvalidStoredState)?;
                        for index in 0..value.count {
                            creatures.push(RaidCreaturePlan {
                                identity: RaidCreatureIdentity { wave: *wave, entry, index },
                                creature: value.creature.clone(), area_anchor: value.area_anchor.clone(),
                            });
                        }
                    }
                    CommittedRaidAction::Wave {
                        wave: *wave,
                        inserted: u32::try_from(creatures.len()).map_err(|_| DurabilityError::InvalidStoredState)?,
                        creatures, boss_step: *boss_step,
                    }
                }
                RaidRunAction::Complete => CommittedRaidAction::Complete {
                    // BOSS-RAID-0 §4.1/4.4: persistent creatures survive firing close.
                    cleanup: definition.despawn_after_ms.is_some(),
                },
            };
            effects::insert(&mut tx, identity, &owner, &action, now_ms).await?;
            let next = cursor.commit(&step).map_err(|_| DurabilityError::InvalidStoredState)?;
            sqlx::query("UPDATE game_world_raid_channel_runs SET committed_steps=$3,updated_at_ms=$4 WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid")
                .bind(firing_id.as_slice()).bind(channel.as_bytes().as_slice())
                .bind(i32::try_from(next.committed_steps).map_err(|_| DurabilityError::InvalidStoredState)?)
                .bind(now_ms).execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Some(CommittedRaidStep { identity, action, replayed: false }))
        })).await
    }

    /// Persist the placement outcomes retained by the existing live owner. A
    /// replay must match every outcome; failures are counted once, never requeued.
    pub async fn acknowledge_raid_effect(
        &self,
        node: &NodeIncarnationProof,
        identity: RaidStepIdentity,
        outcomes: Vec<RaidPlacementOutcome>,
        now_ms: i64,
    ) -> Result<RaidWrite, DurabilityError> {
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result =
                        effects::acknowledge(&mut tx, &node, identity, &outcomes, now_ms).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(result)
                })
            })
            .await
    }

    /// Death evidence comes from the channel's monster owner. Only identities
    /// already acknowledged as successfully placed can be retired here.
    pub async fn record_raid_creature_deaths(
        &self,
        node: &NodeIncarnationProof,
        identity: RaidStepIdentity,
        deaths: Vec<RaidCreatureIdentity>,
    ) -> Result<RaidWrite, DurabilityError> {
        if deaths.len() > usize::from(oteryn_gameplay_encounter_rules::raid::MAX_CREATURES) {
            return Err(DurabilityError::InvalidStoredState);
        }
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            if !effects::world_current(&mut tx, identity.firing_id).await?
                || !effects::current(&mut tx, &node, identity.firing_id, identity.channel, identity.generation).await? {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(RaidWrite::CompareFailed);
            }
            let owner: Option<String> = sqlx::query_scalar("SELECT owner_generation::text FROM game_world_raid_channel_runs WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid FOR UPDATE")
                .bind(identity.firing_id.as_slice()).bind(identity.channel.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            if owner.as_deref() != Some(&identity.generation.get().to_string()) {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(RaidWrite::CompareFailed);
            }
            // Validate the complete batch before changing any row. Missing or
            // failed placement is not a replayed death: its wave still needs
            // an ACK, or no live actor ever existed for that identity.
            let mut unique = std::collections::BTreeSet::new();
            for death in &deaths {
                if !unique.insert(*death) { return Err(DurabilityError::InvalidStoredState); }
                let placed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_world_raid_creatures c JOIN game_world_raid_creature_placements p USING(firing_id,channel_id,wave,entry,creature_index) JOIN game_world_raid_effects e ON e.firing_id=c.firing_id AND e.channel_id=c.channel_id AND e.wave=c.wave AND e.action_kind=2 WHERE c.firing_id=encode($1,'hex')::uuid AND c.channel_id=encode($2,'hex')::uuid AND c.wave=$3 AND c.entry=$4 AND c.creature_index=$5 AND p.placed AND e.state=2 AND e.owner_generation=$6::text::numeric)")
                    .bind(identity.firing_id.as_slice()).bind(identity.channel.as_bytes().as_slice())
                    .bind(effects::small(death.wave)?).bind(effects::small(death.entry)?).bind(effects::small(death.index)?)
                    .bind(identity.generation.get().to_string()).fetch_one(&mut *tx).await?;
                if !placed {
                    commit_semantic_transaction(tx, deadline).await?;
                    return Ok(RaidWrite::CompareFailed);
                }
            }
            let mut changed = false;
            for death in deaths {
                let result = sqlx::query("UPDATE game_world_raid_creatures c SET state=2 WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND wave=$3 AND entry=$4 AND creature_index=$5 AND state=1 AND EXISTS(SELECT 1 FROM game_world_raid_creature_placements p WHERE (p.firing_id,p.channel_id,p.wave,p.entry,p.creature_index)=(c.firing_id,c.channel_id,c.wave,c.entry,c.creature_index) AND p.placed)")
                    .bind(identity.firing_id.as_slice()).bind(identity.channel.as_bytes().as_slice())
                    .bind(effects::small(death.wave)?).bind(effects::small(death.entry)?).bind(effects::small(death.index)?)
                    .execute(&mut *tx).await?;
                changed |= result.rows_affected() != 0;
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(if changed { RaidWrite::Applied } else { RaidWrite::Replayed })
        })).await
    }

    /// Ends a boss raid after the live owner reports the admitted boss identities
    /// dead. All supplied bosses must have a successful placement and a durable
    /// death; an empty or entirely failed boss wave is not death evidence.
    pub async fn finish_raid_after_boss_death(
        &self,
        node: &NodeIncarnationProof,
        mut identity: RaidStepIdentity,
        bosses: Vec<RaidCreatureIdentity>,
        now_ms: i64,
    ) -> Result<Option<CommittedRaidStep>, DurabilityError> {
        if now_ms < 0
            || bosses.is_empty()
            || bosses.len() > usize::from(oteryn_gameplay_encounter_rules::raid::MAX_CREATURES)
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            if !effects::world_current(&mut tx, identity.firing_id).await?
                || !effects::current(&mut tx, &node, identity.firing_id, identity.channel, identity.generation).await? {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(None);
            }
            sqlx::query("SELECT firing_id FROM game_world_raid_firings WHERE firing_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(identity.firing_id.as_slice()).fetch_one(&mut *tx).await?;
            let run = sqlx::query("SELECT committed_steps,updated_at_ms FROM game_world_raid_channel_runs WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND owner_generation=$3::text::numeric AND state IN (1,2) FOR UPDATE")
                .bind(identity.firing_id.as_slice()).bind(identity.channel.as_bytes().as_slice())
                .bind(identity.generation.get().to_string()).fetch_optional(&mut *tx).await?;
            let Some(run) = run else {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(None);
            };
            if now_ms < run.try_get::<i64, _>("updated_at_ms")? {
                return Err(DurabilityError::InvalidStoredState);
            }
            identity.ordinal = u32::try_from(run.try_get::<i32, _>("committed_steps")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            if let Some(pending) = effects::pending(&mut tx, identity).await? {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(matches!(pending.action, CommittedRaidAction::Complete { .. }).then_some(pending));
            }
            let already_cleaned: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_world_raid_effects WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND action_kind=3 AND cleanup AND state=2)")
                .bind(identity.firing_id.as_slice()).bind(identity.channel.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            if already_cleaned {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(None);
            }
            for boss in bosses {
                let dead: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_world_raid_creatures c JOIN game_world_raid_creature_placements p USING(firing_id,channel_id,wave,entry,creature_index) WHERE c.firing_id=encode($1,'hex')::uuid AND c.channel_id=encode($2,'hex')::uuid AND c.wave=$3 AND c.entry=$4 AND c.creature_index=$5 AND c.state=2 AND p.placed)")
                    .bind(identity.firing_id.as_slice()).bind(identity.channel.as_bytes().as_slice())
                    .bind(effects::small(boss.wave)?).bind(effects::small(boss.entry)?).bind(effects::small(boss.index)?)
                    .fetch_one(&mut *tx).await?;
                if !dead {
                    commit_semantic_transaction(tx, deadline).await?;
                    return Ok(None);
                }
            }
            let action = CommittedRaidAction::Complete { cleanup: true };
            effects::insert(&mut tx, identity, &identity.generation.get().to_string(), &action, now_ms).await?;
            let next = identity.ordinal.checked_add(1).and_then(|n| i32::try_from(n).ok())
                .ok_or(DurabilityError::InvalidStoredState)?;
            sqlx::query("UPDATE game_world_raid_channel_runs SET committed_steps=$3,updated_at_ms=$4 WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND state=1")
                .bind(identity.firing_id.as_slice()).bind(identity.channel.as_bytes().as_slice())
                .bind(next).bind(now_ms).execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Some(CommittedRaidStep { identity, action, replayed: false }))
        })).await
    }

    pub async fn due_raid_firings(
        &self,
        channel: ChannelId,
    ) -> Result<Vec<([u8; 16], ProductionKey)>, DurabilityError> {
        Ok(self.page_raid_firings(channel, None).await?.firings)
    }

    /// Terminal persistent runs remain visible until their actual actor deaths
    /// are acknowledged. A stable cursor prevents older survivors starving new runs.
    pub async fn page_raid_firings(
        &self,
        channel: ChannelId,
        after: Option<RaidFiringCursor>,
    ) -> Result<RaidFiringPage, DurabilityError> {
        if after.is_some_and(|cursor| {
            cursor.fired_at_ms < 0 || crate::foundation::WorldId::decode(&cursor.firing_id).is_err()
        }) {
            return Err(DurabilityError::InvalidStoredState);
        }
        let after = after.unwrap_or(RaidFiringCursor {
            fired_at_ms: -1,
            firing_id: [0; 16],
        });
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let mut rows = sqlx::query("SELECT uuid_send(f.firing_id),f.raid_key,f.fired_at_ms FROM game_world_raid_targets t JOIN game_world_raid_firings f USING(firing_id) JOIN game_runtime_scope_assignments a ON a.world_id=f.world_id AND a.channel_id=t.channel_id AND a.scope_kind=1 AND a.state=1 WHERE t.channel_id=encode($1,'hex')::uuid AND (f.fired_at_ms,f.firing_id)>($2,encode($3,'hex')::uuid) AND (f.state=1 OR EXISTS(SELECT 1 FROM game_world_raid_channel_runs r WHERE r.firing_id=f.firing_id AND r.channel_id=t.channel_id AND r.state=1 AND (f.state=3 OR r.owner_generation<>a.ownership_generation)) OR EXISTS(SELECT 1 FROM game_world_raid_effects e WHERE e.firing_id=f.firing_id AND e.channel_id=t.channel_id AND e.state=1) OR EXISTS(SELECT 1 FROM game_world_raid_creatures c WHERE c.firing_id=f.firing_id AND c.channel_id=t.channel_id AND c.state=1)) ORDER BY f.fired_at_ms,f.firing_id LIMIT $4")
                .bind(channel.as_bytes().as_slice()).bind(after.fired_at_ms).bind(after.firing_id.as_slice())
                .bind(i64::try_from(MAX_CANDIDATES + 1).map_err(|_| DurabilityError::InvalidStoredState)?)
                .fetch_all(&mut *tx).await?;
            let more = rows.len() > MAX_CANDIDATES;
            rows.truncate(MAX_CANDIDATES);
            let mut out = Vec::with_capacity(rows.len());
            let mut last = None;
            for row in rows {
                let id: Vec<u8> = row.try_get(0)?;
                let bytes = id.try_into().map_err(|_| DurabilityError::InvalidStoredState)?;
                let raw: String = row.try_get(1)?;
                let key = ProductionKey::new(&raw).map_err(|_| DurabilityError::InvalidStoredState)?;
                last = Some(RaidFiringCursor { firing_id: bytes, fired_at_ms: row.try_get(2)? });
                out.push((bytes, key));
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(RaidFiringPage { firings: out, next: if more { last } else { None } })
        })).await
    }
}
