//! Exact Rune source reservation on the same fenced transaction as caster payment.
//! The instance handle selects no authority: real nested custody/current revisions
//! and the active source-bound ItemCount rule are qualified by the Item owner.
use crate::content::{QualifiedNativeEntryRoom, native_gameplay::NativeGameplayState};
use crate::durability::character_equipment::EquipmentSnapshot;
use crate::durability::spell_item_transaction::{
    self as items, SpellItemAuthority, SpellItemError,
};
use crate::durability::spell_items_abi::{
    DurableInventoryConsumption, DurableRuneUseReservation, InventoryCustody,
};
use crate::foundation::ChannelRuntimeV1;
use crate::spell::world_items_execution::{
    QualifiedItemDefinition, QualifiedRuneDefinition, SpellGroundTarget,
};
use crate::spell::{Carrier, SpellDefinition};
use oteryn_protocol_oteryn::actor_spell_item_v2::ItemSpellCastIntent;
use sqlx::{Postgres, Transaction};

pub(super) async fn reserve(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    runtime: &ChannelRuntimeV1,
    room: &QualifiedNativeEntryRoom,
    content: &NativeGameplayState,
    spell: &SpellDefinition,
    intent: &ItemSpellCastIntent,
) -> Result<DurableRuneUseReservation, SpellItemError> {
    let Carrier::Rune { item, charges, .. } = spell.carrier else {
        return Err(SpellItemError::Rejected("exact Rune carrier required"));
    };
    let policy = content
        .item_policy_for_source_id(item)
        .ok_or(SpellItemError::Rejected(
            "Rune active source policy missing",
        ))?;
    let rune = QualifiedRuneDefinition::from_native_policy(policy, item, charges)
        .map_err(SpellItemError::Rejected)?;
    let (_, position) = runtime
        .positioned_player_for_session(authority.game_session_id())
        .map_err(|_| SpellItemError::Rejected("Rune current caster absent"))?
        .ok_or(SpellItemError::Rejected(
            "Rune current caster position absent",
        ))?;
    let target = SpellGroundTarget::for_native_tile_read(room, runtime, position.position())
        .map_err(SpellItemError::Rejected)?;
    let placement = target.placement(Vec::new());
    items::prepare_rune_instance_consumption_in_transaction(
        tx,
        authority,
        &rune,
        intent.rune_item_instance,
        intent.expected_state_revision,
        &placement,
        &|definition| {
            if definition.family != "Item" {
                return None;
            }
            QualifiedItemDefinition::from_native_policy(
                content.item_policy(&definition.production_key, &definition.revision_ref)?,
            )
            .ok()
        },
    )
    .await
}

/// Preallocate the exact equipped successor before durable COMMIT. Bag Rune
/// consumption keeps the equipment projection unchanged. Only a matched real
/// equipped instance may advance this projection's epoch and quantity/revision.
pub(super) fn equipment_successor(
    before: &EquipmentSnapshot,
    source: &DurableInventoryConsumption,
) -> Result<EquipmentSnapshot, SpellItemError> {
    let mut after = before.clone();
    let InventoryCustody::Equipment {
        slot,
        equipment_revision,
    } = source.custody
    else {
        return Ok(after);
    };
    if before.revision != equipment_revision
        || source.quantity_before == 0
        || source.quantity_after.checked_add(1) != Some(source.quantity_before)
    {
        return Err(SpellItemError::Rejected(
            "Rune equipment predecessor mismatch",
        ));
    }
    let i = after
        .items
        .iter()
        .position(|item| item.slot == slot && item.item_instance_id == source.item_instance_id)
        .ok_or(SpellItemError::Rejected("Rune equipped instance mismatch"))?;
    let item = &after.items[i];
    if item.definition != source.definition.definition
        || item.quantity != source.quantity_before
        || item.state_revision != source.state_revision
    {
        return Err(SpellItemError::Rejected("Rune equipped source changed"));
    }
    after.revision = after
        .revision
        .checked_add(1)
        .ok_or(SpellItemError::Rejected("Rune equipment epoch exhausted"))?;
    if source.quantity_after == 0 {
        after.items.remove(i);
    } else {
        after.items[i].quantity = source.quantity_after;
        after.items[i].state_revision = source
            .state_revision
            .checked_add(1)
            .ok_or(SpellItemError::Rejected("Rune Item revision exhausted"))?;
    }
    Ok(after)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::durability::item_mint::{GroundPlacement, TypedDefinitionRef};
    fn source() -> DurableInventoryConsumption {
        DurableInventoryConsumption {
            item_instance_id: [7; 16],
            definition: QualifiedItemDefinition {
                definition: TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: "source:rune".into(),
                    revision_ref: "r1".into(),
                },
                content_generation_digest: [4; 32],
                materializable: true,
                movable: true,
                stack_maximum: 100,
                container_capacity: None,
                inventory_destination: true,
                ground_destination: true,
                decay: None,
                decay_chain: Vec::new(),
            },
            state_revision: 5,
            quantity_before: 2,
            quantity_after: 1,
            custody: InventoryCustody::Equipment {
                slot: 5,
                equipment_revision: 8,
            },
            caster_ground: GroundPlacement {
                spatial_position: vec![0; 10],
                corpse_ref: Vec::new(),
                map_revision: "map".into(),
                content_revision: "content".into(),
                native_room_placement_context: vec![3; 32],
            },
        }
    }
    fn equipment(source: &DurableInventoryConsumption) -> EquipmentSnapshot {
        EquipmentSnapshot {
            character: [1; 16],
            content_digest: [4; 32],
            character_revision: 7,
            revision: 8,
            combat_mode: None,
            items: vec![crate::durability::character_equipment::EquippedItem {
                slot: 5,
                item_instance_id: source.item_instance_id,
                definition: source.definition.definition.clone(),
                quantity: source.quantity_before,
                state_revision: source.state_revision,
            }],
        }
    }
    #[test]
    fn exact_equipped_count_updates_source_once_and_exhaustion_removes_slot() {
        let mut source = source();
        let before = equipment(&source);
        let next = equipment_successor(&before, &source).unwrap();
        assert_eq!(next.revision, 9);
        assert_eq!(next.items[0].quantity, 1);
        assert_eq!(next.items[0].state_revision, 6);
        assert_eq!(before.revision, 8);
        assert_eq!(before.items[0].quantity, 2);
        source.quantity_before = 1;
        source.quantity_after = 0;
        let before = equipment(&source);
        assert!(
            equipment_successor(&before, &source)
                .unwrap()
                .items
                .is_empty()
        );
    }
    #[test]
    fn each_changed_predecessor_refuses_without_normalizing_inventory() {
        let source = source();
        let before = equipment(&source);
        for change in 0..6 {
            let mut changed = source.clone();
            match change {
                0 => changed.quantity_before += 1,
                1 => changed.quantity_after = 0,
                2 => changed.state_revision += 1,
                3 => changed.item_instance_id[0] += 1,
                4 => changed.definition.definition.revision_ref = "other".into(),
                _ => {
                    changed.custody = InventoryCustody::Equipment {
                        slot: 5,
                        equipment_revision: 9,
                    }
                }
            }
            assert!(
                equipment_successor(&before, &changed).is_err(),
                "mutation{change}"
            );
            assert_eq!(before, equipment(&source));
        }
    }
}
