//! Uses the shared one-item retirement, with reservation and terminal write in
//! the same transaction as the arena visit. A lost acknowledgement replays the visit.
use super::*;
use crate::durability::arena_match::pit_reset::{ArenaPitResetItem, ArenaResetVisit};
use crate::durability::item_mint_audit::Type2Transaction;

pub(in crate::durability) async fn retire_top(
    tx: &mut Type2Transaction<'_>,
    permit: &SpellLanePermit,
    node: &NodeIncarnationProof,
    visit: &ArenaResetVisit,
    item: ArenaPitResetItem,
) -> std::result::Result<(), DurabilityError> {
    let scope = visit.fence.runtime_scope;
    let generation = visit.fence.scope_ownership_generation;
    let other_floor = item.other_floor();
    // The Ground owner lock is already held. Include both storage floor
    // conventions and unrenderable rows in the deciding top-of-stack read.
    let row = sqlx::query(
        "SELECT uuid_send(g.item_instance_id) AS item, g.spatial_position, g.stack_ordinal,
                g.runtime_scope_ownership_generation::text AS generation, i.definition_family,
                i.definition_production_key, i.definition_revision_ref
         FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id)
         WHERE g.world_id=encode($1,'hex')::uuid AND g.channel_id=encode($2,'hex')::uuid
           AND g.scope_kind=1 AND g.spatial_position IN ($3,$4) AND i.lifecycle=1
         ORDER BY g.stack_ordinal DESC NULLS FIRST, g.item_instance_id LIMIT 1 FOR UPDATE OF i NOWAIT",
    )
    .bind(scope.world_id().as_bytes().as_slice())
    .bind(scope.channel_id().map(|id| id.as_bytes().to_vec()))
    .bind(item.position.as_slice())
    .bind(other_floor.as_slice())
    .fetch_optional(&mut ***tx)
    .await?;
    let Some(row) = row else {
        return Ok(());
    };
    let stored_position: Vec<u8> = row.try_get("spatial_position")?;
    if row.try_get::<Vec<u8>, _>("item")? != item.identity
        || (stored_position != item.position && stored_position != other_floor)
        || row.try_get::<Option<i64>, _>("stack_ordinal")? != Some(item.ordinal)
        || row.try_get::<String, _>("generation")? != generation.get().to_string()
        || row.try_get::<&str, _>("definition_family")? != "Item"
        || row.try_get::<&str, _>("definition_production_key")? != item.key
        || row.try_get::<&str, _>("definition_revision_ref")? != item.revision
        || load_reservation(tx, item.identity, &generation.get().to_string())
            .await?
            .is_some()
    {
        return Ok(());
    }

    let fence = CorpseDecayFence {
        runtime_scope: scope,
        scope_ownership_generation: generation,
    };
    let step = DecayRetireStep {
        corpse_item_instance_id: item.identity,
        item_instance_id: item.identity,
        cause: RetireCause::ArenaReset(visit.binding),
    };
    let admitted = world_reset_cause::admit_ground(tx, &fence, step)
        .await?
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let ids = sqlx::query(
        "SELECT game_character_uuid_v7()::text AS transaction_id, game_character_uuid_v7()::text AS event_id",
    )
    .fetch_one(&mut ***tx)
    .await?;
    let transaction_id = uuid_text(ids.try_get("transaction_id")?)?;
    let event_id = uuid_text(ids.try_get("event_id")?)?;
    let tuple = tx.tuple();
    let mut message = decay_message(step, &fence, &admitted);
    message.cause = None;
    let cause = audit::OneItemArenaResetV1 {
        world_id: scope.world_id().as_bytes().to_vec(),
        channel_id: scope
            .channel_id()
            .ok_or(DurabilityError::InvalidStoredState)?
            .as_bytes()
            .to_vec(),
        character_id: visit.fence.character_id.as_bytes().to_vec(),
        game_session_id: visit.state.session.to_vec(),
        run_id: visit.state.run.to_vec(),
        item_instance_id: item.identity.to_vec(),
        rank: u32::from(visit.state.rank),
        pit: u32::from(visit.state.pit),
        deadline_unix_ms: u64::try_from(visit.state.deadline_ms)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
    };
    message.arena_reset = Some(cause.clone());
    let envelope = audit::encode_decay_retire_event(
        DecayRetireEventIdentity {
            event_id,
            transaction_id,
            occurred_at_unix_ms: admitted.database_now_unix_ms,
            server_build_id: SERVER_BUILD_ID,
        },
        message,
        tuple.tuple(),
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let fact = node.fact();
    let reservation = Reservation {
        cause: step.cause,
        corpse_item_instance_id: item.identity,
        fence_generation: generation.get(),
        transaction_id,
        event_id,
        occurred_at_unix_ms: admitted.database_now_unix_ms,
        deadline_unix_ms: 0,
        quantity_before: admitted.quantity_before,
        placement_ordinal: None,
        envelope,
        fence_node_id: *fact.node_id().as_bytes(),
        fence_registration_revision: fact.registration_revision(),
        work_units_used: 0,
    };
    retirement_workflow::insert_reservation(tx, step, &admitted, &reservation, tuple).await?;
    world_reset_cause::retire_death_contents(
        tx, permit, &fence, node, step, &admitted, Some(&cause),
        #[cfg(any(test, feature = "test-support"))]
        None,
    ).await?;
    apply_retire(tx, permit, step, &admitted, &reservation, tuple).await?;
    Ok(())
}
