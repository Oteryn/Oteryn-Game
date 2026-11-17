use super::*;
use oteryn_gameplay_items::item_craft::CookingConversion;
use oteryn_gameplay_world_change::thornfire::refill_bucket;

/// The shared craft owner supplies the locked source, a current map-station
/// witness and an atomic consume/mint receipt.
pub(super) fn refill(
    native: &NativeGameplayState,
    used: &TypedDefinitionRef,
    station: &TypedDefinitionRef,
) -> Option<CookingConversion> {
    if [used, station]
        .iter()
        .any(|operand| operand.family != "Item" || operand.revision_ref != "definition-r1")
    {
        return None;
    }
    let output = refill_bucket(&used.production_key, &station.production_key)?;
    let policy = native.item_policy(output, "definition-r1")?.record();
    if !policy.admission.materializable {
        return None;
    }
    let definition = &policy.production_definition;
    Some(CookingConversion {
        recipe: "oteryn:quest-craft/thornfire/refill".into(),
        outputs: vec![(
            TypedDefinitionRef {
                family: definition.family.clone(),
                production_key: definition.production_key.clone(),
                revision_ref: definition.revision_ref.clone(),
            },
            1,
        )],
    })
}

pub(super) const EXTINGUISH_RECIPE: &str = "oteryn:quest-craft/thornfire/extinguish";

pub(super) fn extinguishes(
    request: &ItemCraftRequest,
    gathering: &GatheringCraft,
    definition: &TypedDefinitionRef,
) -> bool {
    use oteryn_gameplay_world_change::thornfire::{
        FILLED_BUCKET, PREPARED_BUCKET_ID, is_thornfire,
    };

    let mutation = &gathering.mutation;
    let Ok(placement) = <&[u8; 40]>::try_from(mutation.placement.as_slice()) else {
        return false;
    };
    let Ok(key) = <[u8; 8]>::try_from(&placement[32..]) else {
        return false;
    };
    request.tool.is_none()
        && request.ingredient.is_none()
        && mutation.tool == request.source
        && definition.family == "Item"
        && definition.revision_ref == "definition-r1"
        && definition.production_key == FILLED_BUCKET
        && u16::try_from(mutation.before)
            .is_ok_and(|appearance| is_thornfire(u64::from_be_bytes(key), appearance))
        && mutation.after == 0
        && mutation.decay.is_empty()
        && gathering.attempt.after.is_none()
        && gathering.attempt.yield_item == Some(PREPARED_BUCKET_ID)
        && gathering.attempt.consumes_bait
}

pub(super) async fn credit_firefighter(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterItemFence,
    event: &str,
) -> Result<(), DurabilityError> {
    super::super::account_achievement::add_progress(
        tx,
        fence.character_id,
        "oteryn:achievement/firefighter",
        oteryn_gameplay_world_change::thornfire::FIREFIGHTER_TOTAL,
        "oteryn:thornfire-extinguish",
        event.as_bytes(),
    )
    .await
}
