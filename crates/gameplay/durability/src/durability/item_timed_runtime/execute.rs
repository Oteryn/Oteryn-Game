use super::*;
use crate::domain::timed_item::{LaneStep, LaneWriteKind, TimedItemLane};
use crate::durability::item_mint::TypedDefinitionRef;
pub(super) async fn apply(
    tx: &mut crate::durability::item_mint_audit::Type2Transaction<'_>,
    fence: &CurrentCharacterGameplayFence,
    request: &TimedRequest,
) -> Result<TimedItemResult> {
    if let TimedOperation::Repair(coins) = &request.operation {
        return repair::apply(tx, fence, request, coins).await;
    }
    // Lock the item after the fenced Character root. Immutable custody rows cannot move
    // concurrently: every owner changing them holds that same root/item lock.
    let row = sqlx::query("SELECT i.definition_family,i.definition_production_key,i.definition_revision_ref,i.lifecycle,i.world_id::text, EXISTS(SELECT 1 FROM game_character_equipment_slots s WHERE s.item_instance_id=i.item_instance_id AND s.character_id=encode($2,'hex')::uuid) AS equipped, EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id AND e.character_id=encode($2,'hex')::uuid) OR EXISTS(SELECT 1 FROM game_character_equipment_slots e WHERE e.item_instance_id=i.item_instance_id AND e.character_id=encode($2,'hex')::uuid) AS held FROM game_item_instances i WHERE i.item_instance_id=encode($1,'hex')::uuid FOR UPDATE OF i")
        .bind(request.item.as_slice()).bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut ***tx).await?;
    let Some(row) = row else {
        return Err(TimedWriteError::NotHeld);
    };
    let definition = TypedDefinitionRef {
        family: row.try_get("definition_family")?,
        production_key: row.try_get("definition_production_key")?,
        revision_ref: row.try_get("definition_revision_ref")?,
    };
    if !row.try_get::<bool, _>("held")?
        || row.try_get::<i16, _>("lifecycle")? != 1
        || uuid_text(&row.try_get::<String, _>("world_id")?)?
            != *fence.runtime_scope.world_id().as_bytes()
        || definition != request.definition.definition
    {
        return Err(TimedWriteError::NotHeld);
    }
    if matches!(
        request.operation,
        TimedOperation::Advance(_) | TimedOperation::Use { elapsed_ms: 1.. }
    ) && !row.try_get::<bool, _>("equipped")?
    {
        return Err(TimedWriteError::NotHeld);
    }
    let current = timed::load_timed_state(tx, &request.item).await?;
    let revision = current.map_or(0, |row| row.revision);
    if !matches!(request.operation, TimedOperation::Inspect)
        && revision != request.expected_revision
    {
        return Ok(TimedItemResult::refused(TimedDisposition::Stale));
    }
    let values = current.map_or(Some(request.definition.full), |row| row.values);
    let mut result = TimedItemResult {
        disposition: TimedDisposition::Ok,
        revision,
        charges: values.and_then(|v| v.charges()),
        remaining_ms: values.and_then(|v| v.remaining_ms()),
        retired: false,
        fee_gold: 0,
    };
    if matches!(request.operation, TimedOperation::Inspect) {
        return Ok(result);
    }
    if current.is_some_and(|row| row.deadline_at.is_some()) {
        return Err(TimedWriteError::NotAStoreableChange);
    }
    let values = values.ok_or(TimedWriteError::NotAStoreableChange)?;
    let mut lane = TimedItemLane::live_from_row(revision, values, 0);
    match request.operation {
        TimedOperation::Use { elapsed_ms } => {
            lane.advance(elapsed_ms);
            if lane.is_active() && !lane.spend_charge() {
                return Err(TimedWriteError::NotAStoreableChange);
            }
        }
        TimedOperation::Advance(elapsed) => lane.advance(elapsed),
        TimedOperation::Inspect | TimedOperation::Repair(_) => {
            return Err(TimedWriteError::InvalidInput);
        }
    }
    lane.stop();
    let write = match lane.next_step(1, request.transaction_id) {
        LaneStep::Write(write) => write,
        LaneStep::Stopped if matches!(request.operation, TimedOperation::Advance(_)) => {
            return Ok(result);
        }
        _ => return Err(TimedWriteError::NotAStoreableChange),
    };
    match write.kind {
        LaneWriteKind::Checkpoint { values } => {
            let checkpoint = TimedCheckpointRequest {
                item_instance_id: request.item,
                expected_revision: revision,
                transaction_id: request.transaction_id,
                definition: request.definition.clone(),
                values,
            };
            timed::validate_checkpoint(&checkpoint)?;
            timed::write_checkpoint(tx, fence.character_id, &checkpoint).await?;
            result.revision += 1;
            result.charges = values.charges();
            result.remaining_ms = values.remaining_ms();
        }
        LaneWriteKind::Expire { reason } => {
            let expiry = TimedExpiryRequest {
                item_instance_id: request.item,
                expected_revision: revision,
                transaction_id: request.transaction_id,
                event_id: request.event_id,
                reason,
                definition: request.definition.clone(),
                target: request.target.clone(),
                occurred_at_unix_ms: request.occurred_at_unix_ms,
                server_build_id: "oteryn-game-server".into(),
            };
            timed::validate_expiry(&expiry)?;
            let tuple = tx.tuple();
            timed::write_expiry(tx, fence, &expiry, tuple).await?;
            result.retired = request.target.is_none();
            result.revision += u64::from(!result.retired);
            result.charges = request
                .target
                .as_ref()
                .and_then(|t| t.full)
                .and_then(|v| v.charges());
            result.remaining_ms = request
                .target
                .as_ref()
                .and_then(|t| t.full)
                .and_then(|v| v.remaining_ms());
        }
    }
    Ok(result)
}
