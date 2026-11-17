use super::{
    ActionPlan, CurrentCharacterItemFence, DurabilityError, NativeGameplayState,
    TypedDefinitionRef, UseDisposition,
};
use crate::durability::item_use::gourmet::special_food_on_cooldown;
use serde::Deserialize;
use std::{collections::BTreeMap, sync::OnceLock};

const RECIPE: &str = "oteryn:novelty-item.sweet-mangonaise";
const COPIES: usize = 10;

#[derive(Deserialize)]
struct Elixir {
    item: String,
    ring_outputs: BTreeMap<u32, u32>,
}

fn elixir() -> Option<&'static Elixir> {
    static ELIXIR: OnceLock<Option<Elixir>> = OnceLock::new();
    ELIXIR
        .get_or_init(|| {
            serde_json::from_str(include_str!(
                "../../../../../../content/quests/hot-cuisine-sweet-mangonaise.json"
            ))
            .ok()
        })
        .as_ref()
}

pub fn is_sweet_mangonaise(key: &str) -> bool {
    elixir().is_some_and(|elixir| elixir.item == key)
}

fn output_id(ring: u32) -> Option<u32> {
    let outputs = &elixir()?.ring_outputs;
    // A timed ring can be stored in its base form or its equipped form.
    outputs
        .get(&ring)
        .copied()
        .or_else(|| outputs.values().copied().find(|&equipped| equipped == ring))
}

pub(super) async fn prepare(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterItemFence,
    native: &NativeGameplayState,
) -> Result<Result<ActionPlan, UseDisposition>, DurabilityError> {
    // The consume/transform owner has already locked the Character root.
    if special_food_on_cooldown(tx, fence).await? {
        return Ok(Err(UseDisposition::Exhausted));
    }
    let world = fence.runtime_scope.world_id();
    let ring: Option<String> = sqlx::query_scalar(
        "SELECT i.definition_production_key
           FROM game_character_equipment_slots s JOIN game_item_instances i USING(item_instance_id)
          WHERE s.character_id=encode($1,'hex')::uuid AND s.slot=$2
            AND s.world_id=encode($3,'hex')::uuid AND i.world_id=s.world_id
            AND i.lifecycle=1 AND i.quantity=1 AND i.definition_family='Item'",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(oteryn_protocol_oteryn::item_view::EquipmentSlot::Ring as i16)
    .bind(world.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(ring) = ring else {
        return Ok(Err(UseDisposition::NoTarget));
    };
    let output = ring
        .strip_prefix("oteryn:item.tibia.i")
        .and_then(|id| id.parse().ok())
        .and_then(output_id)
        .and_then(|id| native.item_policy_for_source_id(id));
    let Some(output) = output else {
        return Ok(Err(UseDisposition::TargetIllegal));
    };
    let output = output.record();
    if !output.admission.materializable {
        return Ok(Err(UseDisposition::NotPossible));
    }
    let definition = &output.production_definition;
    Ok(Ok(ActionPlan {
        recipe: RECIPE.into(),
        outputs: vec![
            (
                TypedDefinitionRef {
                    family: definition.family.clone(),
                    production_key: definition.production_key.clone(),
                    revision_ref: definition.revision_ref.clone(),
                },
                1
            );
            COPIES
        ],
        consume_source: true,
        source_count: 1,
        effects: serde_json::json!({}),
        extra_inputs: Vec::new(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sweet_mangonaise_uses_fixed_donor_output_ids() {
        assert!(is_sweet_mangonaise("oteryn:item.tibia.i11588"));
        assert_eq!(output_id(3053), Some(3090));
        assert_eq!(output_id(3090), Some(3090));
        assert_eq!(output_id(3048), Some(3048));
        assert_eq!(output_id(16114), Some(16264));
        assert_eq!(output_id(3054), None);
        assert_eq!(COPIES, 10);
    }
}
