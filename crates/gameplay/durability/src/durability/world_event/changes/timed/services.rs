//! Event-owned service closures join the caller's ordinary Character transaction.
use super::*;

/// Called after the operation's Character roots are locked and before value
/// mutation. A shared state-row lock serializes reopening/closing with recovery
/// and scheduler transitions without reversing the Character/World lock order.
pub(crate) async fn service_admitted(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    npc: &str,
    service: &str,
) -> Result<bool, DurabilityError> {
    let catalogue: serde_json::Value =
        serde_json::from_str(CATALOGUE_JSON).map_err(|_| DurabilityError::InvalidStoredState)?;
    let entries = catalogue["entries"]
        .as_array()
        .filter(|entries| entries.len() <= 256)
        .ok_or(DurabilityError::InvalidStoredState)?;
    for entry in entries {
        let Some(kind) = entry["key"].as_str().and_then(EventKind::from_key) else {
            continue;
        };
        if !qualified(kind) {
            continue;
        }
        let Some(rules) = entry["binding"]["services"].as_array() else {
            continue;
        };
        if rules.len() > 16 {
            return Err(DurabilityError::InvalidStoredState);
        }
        if !rules
            .iter()
            .any(|rule| rule["npc"] == npc && rule["service"] == service)
        {
            continue;
        }
        // Daily closures are scoped to the active reset. A previous day's bag
        // or unattended robbery cannot disable a banker after the next save.
        let raw: Option<serde_json::Value> = sqlx::query_scalar(
            "SELECT e.state FROM game_world_timed_events e \
             JOIN game_world_reset_active a ON a.world_id=e.world_id AND a.reset_epoch=e.cycle_id \
             WHERE e.world_id=encode($1,'hex')::uuid AND e.event_key=$2 FOR SHARE OF e",
        )
        .bind(world.as_bytes().as_slice())
        .bind(kind.key())
        .fetch_optional(&mut **tx)
        .await?;
        if let Some(raw) = raw {
            let state: EventState =
                serde_json::from_value(raw).map_err(|_| DurabilityError::InvalidStoredState)?;
            if state.phase == oteryn_gameplay_world_change::timed::Phase::Active {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
