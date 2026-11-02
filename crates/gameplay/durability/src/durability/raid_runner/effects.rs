use super::{
    CommittedRaidAction, CommittedRaidStep, DurabilityError, NodeIncarnationProof, ProductionKey,
    RaidCreatureIdentity, RaidCreaturePlan, RaidPlacementOutcome, RaidStepIdentity, RaidWrite,
};
use crate::durability::runtime_scope_assignment::prove_current_incarnation;
use crate::foundation::{ChannelId, ScopeOwnershipGeneration};
use oteryn_gameplay_encounter_rules::raid::MAX_CREATURES;
use sqlx::{Postgres, Row, Transaction, postgres::PgRow};
use std::collections::BTreeMap;

pub(super) fn small(value: u16) -> Result<i16, DurabilityError> {
    i16::try_from(value).map_err(|_| DurabilityError::InvalidStoredState)
}

pub(super) async fn current(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    firing: [u8; 16],
    channel: ChannelId,
    generation: ScopeOwnershipGeneration,
) -> Result<bool, DurabilityError> {
    if !prove_current_incarnation(tx, node).await? {
        return Ok(false);
    }
    let fact = node.fact();
    let observed: Option<String> = sqlx::query_scalar("SELECT a.ownership_generation::text FROM game_runtime_scope_assignments a JOIN game_world_raid_firings f ON f.world_id=a.world_id JOIN game_durability_admission_runtime_guards g USING(scope_key) WHERE f.firing_id=encode($1,'hex')::uuid AND a.channel_id=encode($2,'hex')::uuid AND a.scope_kind=1 AND a.state=1 AND a.holder_node_id=encode($3,'hex')::uuid AND a.holder_registration_revision=$4::text::numeric AND g.ready AND g.ownership_generation=a.ownership_generation FOR SHARE OF a,g")
        .bind(firing.as_slice()).bind(channel.as_bytes().as_slice())
        .bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string())
        .fetch_optional(&mut **tx).await?;
    Ok(observed.as_deref() == Some(&generation.get().to_string()))
}

pub(super) async fn world_current(
    tx: &mut Transaction<'_, Postgres>,
    firing: [u8; 16],
) -> Result<bool, DurabilityError> {
    // Reset activation owns the admission relations before the active epoch.
    // Preserve that order even for read-only physical-effect transactions.
    crate::durability::db::lock_admission_relations(tx).await?;
    // The reset owner locks its active epoch and rejects RETIRING. The immutable
    // firing supplies the expected epoch/pin, not the independently read truth.
    let current: Option<bool> = sqlx::query_scalar("SELECT game_world_reset_map_epoch_current(f.world_id,f.reset_epoch) AND EXISTS(SELECT 1 FROM game_world_reset_active a JOIN game_world_reset_epochs e USING(world_id,reset_epoch) WHERE a.world_id=f.world_id AND a.reset_epoch=f.reset_epoch AND e.target_digest=f.bundle_digest) FROM game_world_raid_firings f WHERE f.firing_id=encode($1,'hex')::uuid")
        .bind(firing.as_slice()).fetch_optional(&mut **tx).await?;
    Ok(current.unwrap_or(false))
}

pub(super) async fn insert(
    tx: &mut Transaction<'_, Postgres>,
    id: RaidStepIdentity,
    owner: &str,
    action: &CommittedRaidAction,
    now: i64,
) -> Result<(), DurabilityError> {
    let (kind, text, advance, wave, boss_step, cleanup) = match action {
        CommittedRaidAction::Announcement { text, advance } => {
            (1_i16, Some(text.as_str()), Some(*advance), None, None, None)
        }
        CommittedRaidAction::Wave {
            wave, boss_step, ..
        } => (2, None, None, Some(small(*wave)?), Some(*boss_step), None),
        CommittedRaidAction::Complete { cleanup } => (3, None, None, None, None, Some(*cleanup)),
        CommittedRaidAction::Lost => (4, None, None, None, None, Some(true)),
    };
    let ordinal = i32::try_from(id.ordinal).map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_world_raid_step_receipts(firing_id,channel_id,step_ordinal,action_kind,committed_at_ms) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5)")
        .bind(id.firing_id.as_slice()).bind(id.channel.as_bytes().as_slice())
        .bind(ordinal).bind(kind.min(3)).bind(now).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_world_raid_effects(firing_id,channel_id,step_ordinal,owner_generation,action_kind,announcement_text,announcement_advance,wave,boss_step,cleanup) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4::text::numeric,$5,$6,$7,$8,$9,$10)")
        .bind(id.firing_id.as_slice()).bind(id.channel.as_bytes().as_slice())
        .bind(ordinal).bind(owner).bind(kind).bind(text).bind(advance).bind(wave)
        .bind(boss_step).bind(cleanup).execute(&mut **tx).await?;
    if let CommittedRaidAction::Wave { creatures, .. } = action {
        for creature in creatures {
            sqlx::query("INSERT INTO game_world_raid_creatures(firing_id,channel_id,wave,entry,creature_index,creature_key,anchor_key) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,$6,$7)")
                .bind(id.firing_id.as_slice()).bind(id.channel.as_bytes().as_slice())
                .bind(small(creature.identity.wave)?).bind(small(creature.identity.entry)?).bind(small(creature.identity.index)?)
                .bind(creature.creature.as_str()).bind(creature.area_anchor.as_str()).execute(&mut **tx).await?;
        }
    }
    Ok(())
}

pub(super) async fn decode(
    tx: &mut Transaction<'_, Postgres>,
    mut id: RaidStepIdentity,
    row: &PgRow,
) -> Result<CommittedRaidStep, DurabilityError> {
    id.ordinal = u32::try_from(row.try_get::<i32, _>("step_ordinal")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let action = match row.try_get::<i16, _>("action_kind")? {
        1 => CommittedRaidAction::Announcement {
            text: row.try_get("announcement_text")?,
            advance: row.try_get("announcement_advance")?,
        },
        2 => {
            let wave = u16::try_from(row.try_get::<i16, _>("wave")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            let rows = sqlx::query("SELECT entry,creature_index,creature_key,anchor_key FROM game_world_raid_creatures WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND wave=$3 ORDER BY entry,creature_index LIMIT $4")
                .bind(id.firing_id.as_slice()).bind(id.channel.as_bytes().as_slice())
                .bind(small(wave)?).bind(i64::from(MAX_CREATURES) + 1).fetch_all(&mut **tx).await?;
            if rows.is_empty() || rows.len() > usize::from(MAX_CREATURES) {
                return Err(DurabilityError::InvalidStoredState);
            }
            let mut creatures = Vec::with_capacity(rows.len());
            for creature in rows {
                let identity = RaidCreatureIdentity {
                    wave,
                    entry: u16::try_from(creature.try_get::<i16, _>("entry")?)
                        .map_err(|_| DurabilityError::InvalidStoredState)?,
                    index: u16::try_from(creature.try_get::<i16, _>("creature_index")?)
                        .map_err(|_| DurabilityError::InvalidStoredState)?,
                };
                let key: String = creature.try_get("creature_key")?;
                let anchor: String = creature.try_get("anchor_key")?;
                creatures.push(RaidCreaturePlan {
                    identity,
                    creature: ProductionKey::new(&key)
                        .map_err(|_| DurabilityError::InvalidStoredState)?,
                    area_anchor: ProductionKey::new(&anchor)
                        .map_err(|_| DurabilityError::InvalidStoredState)?,
                });
            }
            CommittedRaidAction::Wave {
                wave,
                inserted: u32::try_from(creatures.len())
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                creatures,
                boss_step: row.try_get("boss_step")?,
            }
        }
        3 => CommittedRaidAction::Complete {
            cleanup: row.try_get("cleanup")?,
        },
        4 => CommittedRaidAction::Lost,
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    Ok(CommittedRaidStep {
        identity: id,
        action,
        replayed: true,
    })
}

pub(super) async fn pending(
    tx: &mut Transaction<'_, Postgres>,
    id: RaidStepIdentity,
) -> Result<Option<CommittedRaidStep>, DurabilityError> {
    let row = sqlx::query("SELECT * FROM game_world_raid_effects WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND state=1 ORDER BY step_ordinal LIMIT 1")
        .bind(id.firing_id.as_slice()).bind(id.channel.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    match row {
        Some(row) => Ok(Some(decode(tx, id, &row).await?)),
        None => Ok(None),
    }
}

pub(super) async fn lose(
    tx: &mut Transaction<'_, Postgres>,
    mut id: RaidStepIdentity,
    state: i16,
    owner: &str,
    now: i64,
) -> Result<Option<CommittedRaidStep>, DurabilityError> {
    let lost: Option<i16> = sqlx::query_scalar("SELECT state FROM game_world_raid_effects WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND action_kind=4 ORDER BY step_ordinal LIMIT 1")
        .bind(id.firing_id.as_slice()).bind(id.channel.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    if let Some(state) = lost {
        return if state == 1 {
            pending(tx, id).await
        } else {
            Ok(None)
        };
    }
    // A persistent DONE run may already have a separate boss-cleanup receipt
    // beyond its immutable schedule cursor. LOST always appends after it.
    let previous: Option<i32> = sqlx::query_scalar("SELECT max(step_ordinal) FROM game_world_raid_step_receipts WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid")
        .bind(id.firing_id.as_slice()).bind(id.channel.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    if let Some(previous) = previous {
        id.ordinal = id.ordinal.max(
            u32::try_from(previous)
                .ok()
                .and_then(|n| n.checked_add(1))
                .ok_or(DurabilityError::InvalidStoredState)?,
        );
    }
    sqlx::query("UPDATE game_world_raid_effects SET state=3,settled_at_ms=$3 WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND state=1")
        .bind(id.firing_id.as_slice()).bind(id.channel.as_bytes().as_slice()).bind(now).execute(&mut **tx).await?;
    if state == 1 {
        sqlx::query("UPDATE game_world_raid_channel_runs SET state=3,updated_at_ms=$3 WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND state=1")
            .bind(id.firing_id.as_slice()).bind(id.channel.as_bytes().as_slice()).bind(now).execute(&mut **tx).await?;
    }
    insert(tx, id, owner, &CommittedRaidAction::Lost, now).await?;
    Ok(Some(CommittedRaidStep {
        identity: id,
        action: CommittedRaidAction::Lost,
        replayed: false,
    }))
}

fn placement_map(
    outcomes: &[RaidPlacementOutcome],
) -> Result<BTreeMap<RaidCreatureIdentity, bool>, DurabilityError> {
    if outcomes.len() > usize::from(MAX_CREATURES) {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut result = BTreeMap::new();
    for outcome in outcomes {
        if result.insert(outcome.identity, outcome.placed).is_some() {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    Ok(result)
}

pub(super) async fn acknowledge(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    id: RaidStepIdentity,
    outcomes: &[RaidPlacementOutcome],
    now: i64,
) -> Result<RaidWrite, DurabilityError> {
    if now < 0 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let outcomes = placement_map(outcomes)?;
    let active_world = world_current(tx, id.firing_id).await?;
    if !current(tx, node, id.firing_id, id.channel, id.generation).await? {
        return Ok(RaidWrite::CompareFailed);
    }
    // Scheduler startup/reconciliation owns this clock before firing rows.
    // Only terminal ACKs update it; their immutable effect kind can be read
    // without taking the later firing lock and creating a lock-order cycle.
    sqlx::query("SELECT c.world_id FROM game_world_raid_scheduler_clocks c JOIN game_world_raid_firings f USING(world_id) WHERE f.firing_id=encode($1,'hex')::uuid AND EXISTS(SELECT 1 FROM game_world_raid_effects e WHERE e.firing_id=f.firing_id AND e.channel_id=encode($2,'hex')::uuid AND e.step_ordinal=$3 AND e.action_kind IN(3,4)) FOR UPDATE OF c")
        .bind(id.firing_id.as_slice()).bind(id.channel.as_bytes().as_slice())
        .bind(i32::try_from(id.ordinal).map_err(|_| DurabilityError::InvalidStoredState)?)
        .execute(&mut **tx).await?;
    sqlx::query("SELECT firing_id FROM game_world_raid_firings WHERE firing_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(id.firing_id.as_slice()).fetch_one(&mut **tx).await?;
    let owner: Option<String> = sqlx::query_scalar("SELECT owner_generation::text FROM game_world_raid_channel_runs WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid FOR UPDATE")
        .bind(id.firing_id.as_slice()).bind(id.channel.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let row = sqlx::query("SELECT e.*,r.committed_at_ms FROM game_world_raid_effects e JOIN game_world_raid_step_receipts r USING(firing_id,channel_id,step_ordinal) WHERE e.firing_id=encode($1,'hex')::uuid AND e.channel_id=encode($2,'hex')::uuid AND e.step_ordinal=$3 FOR UPDATE OF e")
        .bind(id.firing_id.as_slice()).bind(id.channel.as_bytes().as_slice())
        .bind(i32::try_from(id.ordinal).map_err(|_| DurabilityError::InvalidStoredState)?)
        .fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(RaidWrite::CompareFailed);
    };
    let state: i16 = row.try_get("state")?;
    let effect = decode(tx, id, &row).await?;
    if state == 3
        || (owner.as_deref() != Some(&id.generation.get().to_string())
            && effect.action != CommittedRaidAction::Lost)
    {
        return Ok(RaidWrite::CompareFailed);
    }
    if effect.action != CommittedRaidAction::Lost && !active_world {
        return Ok(RaidWrite::CompareFailed);
    }
    if now < row.try_get::<i64, _>("committed_at_ms")? {
        return Err(DurabilityError::InvalidStoredState);
    }
    if let CommittedRaidAction::Wave { creatures, .. } = &effect.action {
        if creatures.len() != outcomes.len() {
            return Err(DurabilityError::InvalidStoredState);
        }
        for creature in creatures {
            let placed = outcomes
                .get(&creature.identity)
                .ok_or(DurabilityError::InvalidStoredState)?;
            let old: Option<bool> = sqlx::query_scalar("SELECT placed FROM game_world_raid_creature_placements WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND wave=$3 AND entry=$4 AND creature_index=$5")
                .bind(id.firing_id.as_slice()).bind(id.channel.as_bytes().as_slice())
                .bind(small(creature.identity.wave)?).bind(small(creature.identity.entry)?).bind(small(creature.identity.index)?)
                .fetch_optional(&mut **tx).await?;
            if let Some(old) = old {
                if old != *placed {
                    return Err(DurabilityError::InvalidStoredState);
                }
            } else {
                if state != 1 {
                    return Err(DurabilityError::InvalidStoredState);
                }
                sqlx::query("INSERT INTO game_world_raid_creature_placements(firing_id,channel_id,wave,entry,creature_index,placed,recorded_at_ms) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,$6,$7)")
                    .bind(id.firing_id.as_slice()).bind(id.channel.as_bytes().as_slice())
                    .bind(small(creature.identity.wave)?).bind(small(creature.identity.entry)?).bind(small(creature.identity.index)?)
                    .bind(*placed).bind(now).execute(&mut **tx).await?;
                if !*placed {
                    sqlx::query("UPDATE game_world_raid_creatures SET state=2 WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND wave=$3 AND entry=$4 AND creature_index=$5")
                        .bind(id.firing_id.as_slice()).bind(id.channel.as_bytes().as_slice())
                        .bind(small(creature.identity.wave)?).bind(small(creature.identity.entry)?).bind(small(creature.identity.index)?)
                        .execute(&mut **tx).await?;
                }
            }
        }
    } else if !outcomes.is_empty() {
        return Err(DurabilityError::InvalidStoredState);
    }
    if state == 2 {
        return Ok(RaidWrite::Replayed);
    }
    sqlx::query("UPDATE game_world_raid_effects SET state=2,settled_at_ms=$4 WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND step_ordinal=$3 AND state=1")
        .bind(id.firing_id.as_slice()).bind(id.channel.as_bytes().as_slice())
        .bind(i32::try_from(id.ordinal).map_err(|_| DurabilityError::InvalidStoredState)?)
        .bind(now).execute(&mut **tx).await?;
    if matches!(effect.action, CommittedRaidAction::Complete { .. }) {
        sqlx::query("UPDATE game_world_raid_channel_runs SET state=2,updated_at_ms=GREATEST(updated_at_ms,$3) WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND state=1")
            .bind(id.firing_id.as_slice()).bind(id.channel.as_bytes().as_slice()).bind(now).execute(&mut **tx).await?;
    }
    if matches!(
        effect.action,
        CommittedRaidAction::Complete { cleanup: true } | CommittedRaidAction::Lost
    ) {
        sqlx::query("UPDATE game_world_raid_creatures SET state=2 WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND state=1")
            .bind(id.firing_id.as_slice()).bind(id.channel.as_bytes().as_slice()).execute(&mut **tx).await?;
    }
    if matches!(
        effect.action,
        CommittedRaidAction::Complete { .. } | CommittedRaidAction::Lost
    ) {
        let finished = sqlx::query("UPDATE game_world_raid_firings f SET state=2 WHERE firing_id=encode($1,'hex')::uuid AND state=1 AND EXISTS(SELECT 1 FROM game_world_raid_targets t WHERE t.firing_id=f.firing_id) AND NOT EXISTS(SELECT 1 FROM game_world_raid_targets t LEFT JOIN game_world_raid_channel_runs r USING(firing_id,channel_id) WHERE t.firing_id=f.firing_id AND (r.state IS NULL OR r.state=1)) AND NOT EXISTS(SELECT 1 FROM game_world_raid_effects e WHERE e.firing_id=f.firing_id AND e.state=1)")
            .bind(id.firing_id.as_slice()).execute(&mut **tx).await?;
        if finished.rows_affected() != 0 {
            sqlx::query("UPDATE game_world_raid_scheduler_clocks SET last_end_at_ms=GREATEST(last_end_at_ms,$2) WHERE reset_epoch=(SELECT reset_epoch FROM game_world_raid_firings WHERE firing_id=encode($1,'hex')::uuid) AND world_id=(SELECT f.world_id FROM game_world_raid_firings f WHERE f.firing_id=encode($1,'hex')::uuid AND NOT EXISTS(SELECT 1 FROM game_world_raid_schedule_sources s WHERE s.world_id=f.world_id AND s.raid_key=f.raid_key AND s.source_kind<>1))")
                .bind(id.firing_id.as_slice()).bind(now).execute(&mut **tx).await?;
        }
    }
    Ok(RaidWrite::Applied)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raids_global_duplicate_placement_identity_is_rejected() {
        let identity = RaidCreatureIdentity {
            wave: 0,
            entry: 0,
            index: 0,
        };
        assert!(
            placement_map(&[
                RaidPlacementOutcome {
                    identity,
                    placed: true
                },
                RaidPlacementOutcome {
                    identity,
                    placed: false
                },
            ])
            .is_err()
        );
    }

    #[test]
    fn raids_global_placement_failure_is_a_retained_outcome() -> Result<(), DurabilityError> {
        let identity = RaidCreatureIdentity {
            wave: 2,
            entry: 1,
            index: 3,
        };
        let outcomes = placement_map(&[RaidPlacementOutcome {
            identity,
            placed: false,
        }])?;
        assert_eq!(outcomes.get(&identity), Some(&false));
        Ok(())
    }
}
