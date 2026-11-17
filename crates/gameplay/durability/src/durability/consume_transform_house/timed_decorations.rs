//! Aqueduct animation: Crystal 201189f data/items/items.xml:67147-67173,67218-67243.
//! src/items/tile.cpp:1155-1174 invokes src/items/item.cpp:430-451,
//! which resets a non-stopping form's duration. src/game/game.cpp:12994-13011 and
//! src/items/decay/decay.cpp:29-54 replace the old deadline with that full duration.
use super::*;
use crate::durability::spell_items_abi::QualifiedItemDefinition;

const STAGES: [(u32, u32, u32); 8] = [
    (35949, 200_000, 35950),
    (35950, 1_000, 35951),
    (35951, 200_000, 35952),
    (35952, 1_000, 35949),
    (35969, 200_000, 35970),
    (35970, 1_000, 35971),
    (35971, 200_000, 35972),
    (35972, 1_000, 35969),
];

pub(super) async fn prepare(
    tx: &mut crate::durability::item_mint_audit::Type2Transaction<'_>,
    context: &HouseActionContext,
    request: &ConsumeTransformRequest,
    native: &NativeGameplayState,
    before: &TypedDefinitionRef,
    after: &TypedDefinitionRef,
) -> Result<Option<Value>, DurabilityError> {
    let pair = (
        before.production_key.as_str(),
        after.production_key.as_str(),
    );
    if !matches!(
        pair,
        ("oteryn:item.tibia.i35949", "oteryn:item.tibia.i35969")
            | ("oteryn:item.tibia.i35969", "oteryn:item.tibia.i35949")
            | ("oteryn:item.tibia.i35951", "oteryn:item.tibia.i35971")
            | ("oteryn:item.tibia.i35971", "oteryn:item.tibia.i35951")
    ) || before.family != "Item"
        || after.family != "Item"
        || before.revision_ref != after.revision_ref
    {
        return Ok(None);
    }
    // Validate the complete cycle from the activated policy. A missing intermediate
    // definition cannot become a guessed timer target after the first transition.
    for (id, duration, next) in STAGES {
        let key = format!("oteryn:item.tibia.i{id}");
        let Some(qualified) = native
            .item_policy(&key, &before.revision_ref)
            .and_then(|policy| QualifiedItemDefinition::from_native_policy(policy).ok())
        else {
            return Ok(None);
        };
        let Some(decay) = qualified.decay else {
            return Ok(None);
        };
        let Some(target) = decay.target else {
            return Ok(None);
        };
        if qualified.stack_maximum != 1
            || qualified.container_capacity.is_some()
            || !qualified.ground_destination
            || decay.duration_millis != duration
            || target.family != "Item"
            || target.production_key != format!("oteryn:item.tibia.i{next}")
            || target.revision_ref != before.revision_ref
        {
            return Ok(None);
        }
    }
    let placed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND house_key=$3)")
        .bind(request.source.as_slice()).bind(context.house.world_id().as_bytes().as_slice())
        .bind(context.house.house_key()).fetch_one(&mut ***tx).await?;
    if !placed {
        return Ok(None);
    }
    let previous: Option<Value> = sqlx::query_scalar("SELECT to_jsonb(t) FROM game_item_timed_states t WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(request.source.as_slice()).fetch_optional(&mut ***tx).await?;
    let observed_ms: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&mut ***tx)
            .await?;
    let expired = previous
        .as_ref()
        .and_then(|state| state.get("deadline_at"))
        .and_then(Value::as_i64)
        .is_some_and(|deadline| deadline <= observed_ms.saturating_add(1));
    // Crystal src/game/game.cpp:3118-3128 immediately selects the requested form's
    // decayTo when an already-decaying item has at most one millisecond left.
    let selected_key = if expired {
        let policy = native
            .item_policy(&after.production_key, &after.revision_ref)
            .and_then(|policy| QualifiedItemDefinition::from_native_policy(policy).ok())
            .ok_or(DurabilityError::InvalidStoredState)?;
        policy
            .decay
            .and_then(|decay| decay.target)
            .ok_or(DurabilityError::InvalidStoredState)?
            .production_key
    } else {
        after.production_key.clone()
    };
    Ok(Some(json!({
        "schema":"oteryn-house-timed-decoration-v1", "before":previous,
        "observed_ms":observed_ms, "expired_on_rotation":expired, "selected_key":selected_key,
    })))
}
