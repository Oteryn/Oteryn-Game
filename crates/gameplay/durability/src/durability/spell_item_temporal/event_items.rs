//! W19 uses the existing absolute Item timer; its original consume receipt binds the chain.
use crate::durability::DurabilityError;
use crate::durability::item_mint_audit::Type2Transaction;
use oteryn_gameplay_content::content::{ReferenceItemField, native_gameplay::NativeGameplayState};
use oteryn_gameplay_items::event_planting::decay_steps;
use serde_json::{Value, json};
use std::fmt::Write;

/// Include the admitted source chain in the immutable conversion output before its audit/receipt.
pub(crate) fn decorate_output(
    native: &NativeGameplayState,
    output: &mut Value,
) -> Result<(), DurabilityError> {
    let id = output["key"]
        .as_str()
        .and_then(|key| key.strip_prefix("oteryn:item.tibia.i"))
        .and_then(|id| id.parse::<u32>().ok());
    let Some(id) = id else {
        return Ok(());
    };
    // This receipt admits only the exact ice-flower planting output. Map pickup
    // preserves other pot chains separately; trumpet uses the single-stage owner.
    if id != 14031 {
        return Ok(());
    }
    let steps = decay_steps(id);
    if steps.is_empty() {
        return Ok(());
    }
    if output["count"].as_u64() != Some(1) {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut current = id;
    let mut stages = Vec::with_capacity(steps.len());
    for step in steps {
        let before = native
            .item_policy_for_source_id(current)
            .ok_or(DurabilityError::InvalidStoredState)?
            .record();
        let after = native
            .item_policy_for_source_id(step.target)
            .ok_or(DurabilityError::InvalidStoredState)?
            .record();
        let ReferenceItemField::Known(physical) = &before.semantics.physical else {
            return Err(DurabilityError::InvalidStoredState);
        };
        let blocks = before
            .attributes
            .blocks_movement
            .ok_or(DurabilityError::InvalidStoredState)?;
        let projectile = before
            .attributes
            .blocks_projectile
            .ok_or(DurabilityError::InvalidStoredState)?;
        let movable = match physical.movable {
            ReferenceItemField::Known(value) => value,
            _ => return Err(DurabilityError::InvalidStoredState),
        };
        let definition = |d: &oteryn_gameplay_content::content::native_gameplay::NativeProductionItemDefinition| {
            json!({"family": d.family, "key": d.production_key, "revision": d.revision_ref})
        };
        stages.push(json!({
            "definition": definition(&before.production_definition),
            "duration_millis": step.after_ms,
            "target": definition(&after.production_definition),
            "blocks_movement": blocks,
            "blocks_projectile": projectile,
            "immovable_block_solid": blocks && !movable,
        }));
        current = step.target;
    }
    let mut digest = String::with_capacity(64);
    for byte in native.source_digest() {
        write!(&mut digest, "{byte:02x}").map_err(|_| DurabilityError::InvalidStoredState)?;
    }
    output["event_decay"] = Value::Array(stages);
    output["event_content_digest"] = Value::String(digest);
    Ok(())
}

/// Called by the already fenced consume/transform transaction after its output is inserted.
/// The deferred source proof compares every schedule field to that same immutable receipt.
pub(crate) async fn schedule_consume_output(
    tx: &mut Type2Transaction<'_>,
    output: &Value,
    transaction: &str,
    source_ordinal: i32,
    occurred: i64,
) -> Result<(), DurabilityError> {
    let Some(stages) = output.get("event_decay").and_then(Value::as_array) else {
        return Ok(());
    };
    let item = output["item"]
        .as_str()
        .ok_or(DurabilityError::InvalidStoredState)?;
    let digest = output["event_content_digest"]
        .as_str()
        .ok_or(DurabilityError::InvalidStoredState)?;
    let mut deadline = occurred;
    for (index, stage) in stages.iter().enumerate() {
        let duration = stage["duration_millis"]
            .as_i64()
            .ok_or(DurabilityError::InvalidStoredState)?;
        deadline = deadline
            .checked_add(duration)
            .ok_or(DurabilityError::InvalidStoredState)?;
        let definition = &stage["definition"];
        let target = &stage["target"];

        sqlx::query("INSERT INTO game_spell_field_temporal_schedules(item_instance_id,source_stage,source_transaction_id,event_source_transaction_id,source_ordinal,duration_millis,expires_at_unix_ms,definition_family,definition_production_key,definition_revision,target_family,target_production_key,target_revision,content_digest,blocks_movement,blocks_projectile,immovable_block_solid) VALUES($1::uuid,$2,NULL,$3::uuid,$4,$5,$6,$7,$8,$9,$10,$11,$12,decode($13,'hex'),$14,$15,$16)")
            .bind(item).bind(i32::try_from(index + 1).map_err(|_| DurabilityError::InvalidStoredState)?)
            .bind(transaction).bind(source_ordinal).bind(duration).bind(deadline)
            .bind(text(&definition["family"])?).bind(text(&definition["key"])?).bind(text(&definition["revision"])?)
            .bind(text(&target["family"])?).bind(text(&target["key"])?).bind(text(&target["revision"])?)
            .bind(digest)
            .bind(stage["blocks_movement"].as_bool().ok_or(DurabilityError::InvalidStoredState)?)
            .bind(stage["blocks_projectile"].as_bool().ok_or(DurabilityError::InvalidStoredState)?)
            .bind(stage["immovable_block_solid"].as_bool().ok_or(DurabilityError::InvalidStoredState)?)
            .execute(&mut ***tx).await?;
    }
    Ok(())
}

fn text(value: &Value) -> Result<&str, DurabilityError> {
    value.as_str().ok_or(DurabilityError::InvalidStoredState)
}
