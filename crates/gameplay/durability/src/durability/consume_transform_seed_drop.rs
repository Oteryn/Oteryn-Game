//! A materialized pot uses the ordinary consume/transform receipt and absolute timers.
use super::*;

#[allow(
    clippy::too_many_arguments,
    reason = "exact source and destination snapshots from the same native move transaction"
)]
pub(crate) async fn plant_seed_after_move_in_transaction(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    intent: Vec<u8>,
    native: &NativeGameplayState,
    pot: [u8; 16],
    pot_before: Value,
    pot_ground: Value,
    seed: [u8; 16],
    seed_before: Value,
    seed_ground: Value,
    movement: [u8; 16],
    position: &[u8],
) -> Result<(), DurabilityError> {
    let seed_id = seed_before["definition_production_key"]
        .as_str()
        .and_then(|key| key.strip_prefix("oteryn:item.tibia.i"))
        .and_then(|id| id.parse::<u32>().ok())
        .ok_or(DurabilityError::InvalidStoredState)?;
    use oteryn_gameplay_items::event_planting::{self as rules, SeedAction, SeedTrigger};
    let Some(SeedAction::Plant { target }) =
        rules::seed_action(seed_id, 306, SeedTrigger::AddedToPot)
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    if native
        .item_policy(
            "oteryn:item.tibia.i306",
            pot_before["definition_revision_ref"]
                .as_str()
                .ok_or(DurabilityError::InvalidStoredState)?,
        )
        .is_none()
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let policy = native
        .item_policy_for_source_id(target)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let qualified =
        super::super::spell_items_abi::QualifiedItemDefinition::from_native_policy(policy)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
    if !qualified.ground_destination
        || qualified.stack_maximum != 1
        || qualified.container_capacity.is_some()
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let record = policy.record();
    let output = TypedDefinitionRef {
        family: record.production_definition.family.clone(),
        production_key: record.production_definition.production_key.clone(),
        revision_ref: record.production_definition.revision_ref.clone(),
    };
    let cell = super::super::spell_items_abi::decode_ground_cell(position)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let mut effects =
        rules::seed_effects(Some(oteryn_gameplay_items::novelty_items::GroundEffect {
            x: cell.x,
            y: cell.y,
            floor: i16::try_from(cell.z)
                .ok()
                .and_then(i16::checked_neg)
                .ok_or(DurabilityError::InvalidStoredState)?,
        }));
    if seed_id == 647 {
        effects["source_item_id"] = json!(647);
        effects["donor"] = json!(
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data/scripts/movements/seeds.lua"
        );
    }
    let plan = ActionPlan {
        recipe: if seed_id == 647 {
            "oteryn:planting.seed-drop"
        } else {
            "oteryn:planting.ice-flower"
        }
        .into(),
        outputs: vec![(output.clone(), 1)],
        consume_source: true,
        source_count: 1,
        effects,
        extra_inputs: vec![ExtraInput {
            item: seed,
            before: seed_before,
            parent: None,
            ordinal: seed_ground["stack_ordinal"]
                .as_i64()
                .ok_or(DurabilityError::InvalidStoredState)?
                .to_string(),
            count: 1,
            source_kind: 3,
            drop: Some(json!({"move": uuid_text(movement), "ground": seed_ground})),
        }],
    };
    let item: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
        .fetch_one(&mut ***tx)
        .await?;
    let ordinal = pot_ground["stack_ordinal"]
        .as_i64()
        .ok_or(DurabilityError::InvalidStoredState)?
        .to_string();
    let mut output = json!({"item": item, "key": output.production_key, "revision": output.revision_ref,
        "count": 1, "ordinal": ordinal, "decay": null});
    super::super::spell_item_temporal::event_items::decorate_output(native, &mut output)?;
    let request = ConsumeTransformRequest {
        command,
        source: pot,
        target: None,
        intent,
    };
    let result = persist_action(
        tx,
        fence,
        &request,
        native,
        &plan,
        &pot_before,
        None,
        None,
        3,
        &ordinal,
        20,
        &[output],
        &pot_ground,
        None,
    )
    .await?;
    if result.disposition != UseDisposition::Committed {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}
