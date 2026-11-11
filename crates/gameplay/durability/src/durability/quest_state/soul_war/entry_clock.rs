//! Historical lever-use clock DATA, read under the actual current entrant fence.
use super::{DurabilityError, DurabilityRoot};
use crate::durability::admission_journal::party_target_binding::{
    CurrentPartyActorObservation, current_colocated_party_character_for_observation_in_transaction,
    observe_current_party_actor_for_source_read,
};
use crate::durability::spell_item_transaction::{SpellItemScopeAuthority, check_scope_transaction};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, RuntimeScopeRefV1};
use sqlx::{Postgres, Row, Transaction};

/// This does not grant entry or callback mutation authority. Consumers retain
/// their independently current physical Instance owner and source qualification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrentSoulWarEntryClock {
    pub opened_at_unix_ms: i64,
    pub observed_at_unix_ms: i64,
    pub elapsed_ms: u64,
}

#[allow(clippy::too_many_arguments)]
pub async fn read_current_entry_clock_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    scope: &SpellItemScopeAuthority,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    expected_handoff: [u8; 16],
) -> Result<Option<CurrentSoulWarEntryClock>, DurabilityError> {
    let Some(observed) = observe_current_party_actor_for_source_read(runtime, actor, session)
    else {
        return Ok(None);
    };
    let result = read_current_entry_clock_for_observation_in_transaction(
        tx,
        root,
        scope,
        &observed,
        expected_handoff,
    )
    .await?;
    if !observed.matches(runtime) {
        return Err(DurabilityError::Unavailable);
    }
    Ok(result)
}

/// Detached DATA input permits the handoff owner to release its Runtime guard
/// across SQL. The caller must revalidate observed.matches(actualRuntime) before
/// consuming the returned historical clock; this function grants no callbacks.
pub async fn read_current_entry_clock_for_observation_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    scope: &SpellItemScopeAuthority,
    observed: &CurrentPartyActorObservation,
    expected_handoff: [u8; 16],
) -> Result<Option<CurrentSoulWarEntryClock>, DurabilityError> {
    let RuntimeScopeRefV1::Instance {
        world_id,
        instance_id,
    } = observed.runtime_scope()
    else {
        return Ok(None);
    };
    if expected_handoff == [0; 16]
        || scope.runtime_scope() != observed.runtime_scope()
        || scope.generation() != observed.generation()
    {
        return Err(DurabilityError::Unavailable);
    }
    check_scope_transaction(tx, scope)
        .await
        .map_err(|_| DurabilityError::Unavailable)?;
    let Some(character) =
        current_colocated_party_character_for_observation_in_transaction(tx, root, scope, observed)
            .await?
    else {
        return Ok(None);
    };
    let row = sqlx::query(
        "WITH observed AS MATERIALIZED (SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint AS ms) \
         SELECT a.opened_at,observed.ms FROM game_boss_room_activities a \
         JOIN game_activity_handoff_groups g ON g.handoff_id=a.admission \
         JOIN game_activity_runtime_entries e ON e.handoff_id=g.handoff_id \
         JOIN game_activity_handoff_participants p ON p.handoff_id=g.handoff_id \
         CROSS JOIN observed WHERE a.admission=encode($1,'hex')::uuid \
         AND a.world_id=encode($2,'hex')::uuid AND a.instance_id=encode($3,'hex')::uuid \
         AND (g.world_id,g.instance_id)=(a.world_id,a.instance_id) \
         AND g.ownership_generation=$4::text::numeric AND g.state=2 AND a.state=1 \
         AND p.character_id=encode($5,'hex')::uuid \
         AND e.destination_server_artifact_digest=$6 AND e.destination_map_revision_digest=$7 \
         AND e.destination_coordinate_frame_digest=$8 AND a.deadline-a.opened_at=1200000 \
         AND observed.ms BETWEEN a.opened_at AND a.deadline \
         AND (a.boss_key,a.boss_creature_key) IN ( \
          ('oteryn:boss.goshnar_s_malice','oteryn:creature.goshnar_s_malice'), \
          ('oteryn:boss.goshnar_s_spite','oteryn:creature.goshnar_s_spite'), \
          ('oteryn:boss.goshnar_s_greed','oteryn:creature.goshnar_s_greed'), \
          ('oteryn:boss.goshnar_s_hatred','oteryn:creature.goshnar_s_hatred'), \
          ('oteryn:boss.goshnar_s_cruelty','oteryn:creature.goshnar_s_cruelty'), \
          ('oteryn:boss.goshnar_s_megalomania','oteryn:creature.goshnar_s_megalomania_purple'))"
    )
    .bind(expected_handoff.as_slice())
    .bind(world_id.as_bytes().as_slice())
    .bind(instance_id.as_slice())
    .bind(observed.generation().to_string())
    .bind(character.as_slice())
    .bind(observed.server_digest().as_slice())
    .bind(observed.map_digest().as_slice())
    .bind(observed.frame_digest().as_slice())
    .fetch_optional(&mut **tx).await?;
    check_scope_transaction(tx, scope)
        .await
        .map_err(|_| DurabilityError::Unavailable)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let opened_at_unix_ms: i64 = row.try_get("opened_at")?;
    let observed_at_unix_ms: i64 = row.try_get("ms")?;
    let elapsed_ms = observed_at_unix_ms
        .checked_sub(opened_at_unix_ms)
        .and_then(|elapsed| u64::try_from(elapsed).ok())
        .ok_or(DurabilityError::InvalidStoredState)?;
    Ok(Some(CurrentSoulWarEntryClock {
        opened_at_unix_ms,
        observed_at_unix_ms,
        elapsed_ms,
    }))
}
