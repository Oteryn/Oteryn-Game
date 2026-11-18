//! Hotkey Equip over the real node; reference: `client_e2e/hotkey_equip.rs`.
use super::*;
pub(super) async fn hotkey_equip(
    client: &mut oteryn_dev_client::DevClientSession,
) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime, client_ui};
    use oteryn_protocol_oteryn::hotkey_equip::CAPABILITY_HOTKEY_EQUIP_V1;
    if !client
        .selected_capabilities()
        .contains(&CAPABILITY_HOTKEY_EQUIP_V1)
    {
        // Only bundle Worlds offer this owner; native_for_move needs pinned Item content.
        evidence("mechanic=hotkey_equip skipped=entry_room_has_no_native_equipment_content");
        return Ok(());
    }
    // The production starter grant installs a backpack, with no carried equipment.
    // Never manufacture a weapon in game tables to turn this prerequisite into a pass.
    if client
        .inventory()
        .is_none_or(|inventory| inventory.entries.is_empty())
    {
        evidence(
            "mechanic=hotkey_equip skipped=production_starter_has_no_carried_equipment_and_no_network_seed_api",
        );
        return Ok(());
    }
    // Classify every carried item from the pinned item profiles, independently of the server:
    // an equippable item must move; only an all-non-equipment carry may end in the skip below.
    let carried: Vec<u32> = client
        .inventory()
        .map(|inventory| {
            inventory
                .entries
                .iter()
                .map(|item| item.item_definition_ref.get())
                .collect()
        })
        .unwrap_or_default();
    let classes = carried
        .iter()
        .map(|definition| Ok((*definition, pinned_equipment_class(*definition)?)))
        .collect::<TestResult<Vec<_>>>()?;
    let equippable = classes
        .iter()
        .find(|(_, class)| *class == EquipmentClass::Equippable)
        .map(|(definition, _)| *definition);
    let definition = equippable
        .or_else(|| carried.first().copied())
        .ok_or("hotkey equip fixture item missing from inventory")?;
    let carried_handles: Vec<_> = client
        .inventory()
        .ok_or("hotkey equip inventory missing")?
        .entries
        .iter()
        .filter(|item| item.item_definition_ref.get() == definition)
        .map(|item| item.handle)
        .collect();
    let mut panel = client_ui::hotkey_equip::HotkeyEquipPanel {
        selected: true,
        tiered: client.selected_capabilities().contains(
            &oteryn_protocol_oteryn::hotkey_equip::CAPABILITY_HOTKEY_EQUIP_UPGRADE_TIER_V1,
        ),
        item_definition: definition,
        ..Default::default()
    };
    let command =
        client_ui::hotkey_equip::control_request(&mut panel).ok_or("hotkey equip request")?;
    let before = client
        .inventory()
        .cloned()
        .ok_or("hotkey equip missing prior inventory")?;
    let response = client_runtime::raw_feature::send(
        client.session_mut(),
        &client_runtime::raw_feature::FeatureCommand {
            capability: command.capability,
            kind: command.command_type,
            payload: command.payload,
        },
    )
    .await?;
    client_ui::hotkey_equip::apply_result(&mut panel, &response);
    if panel.outcome == Some(oteryn_protocol_oteryn::item_view::ItemMoveOutcome::SlotMismatch) {
        // The starter's carried items are not equipment: the server found no equipment
        // policy for this definition (gameplay_transport/hotkey_equip.rs, SlotMismatch).
        // The skip holds only while the character wears nothing in the nine equipment slots.
        let worn = client
            .inventory()
            .map(|inventory| inventory.equipment.clone())
            .unwrap_or_default();
        if equippable.is_some()
            || classes
                .iter()
                .any(|(_, class)| *class != EquipmentClass::NotEquipment)
        {
            return Err(format!(
                "hotkey equip refused SlotMismatch but pinned profiles do not prove the carry is non-equipment: {classes:?}"
            )
            .into());
        }
        if !worn.is_empty() {
            return Err(format!(
                "hotkey equip refused SlotMismatch for definition {definition} while equipment is worn: {worn:?}"
            )
            .into());
        }
        evidence(&format!(
            "mechanic=hotkey_equip skipped=production_starter_has_no_carried_equipment server_outcome=SlotMismatch worn_equipment=0 definition={definition}"
        ));
        return Ok(());
    }
    if panel.outcome != Some(oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved) {
        return Err(format!(
            "hotkey equip did not move equipment: {:?} status={:?} feedback={:?}",
            panel.outcome, response.status, panel.feedback
        )
        .into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(equipped) = client.inventory().and_then(|inventory| {
            equipped_from_carry(inventory, definition, &carried_handles)?;
            moved_to_equipment(&before, inventory, definition, None, None)
        }) {
            evidence(&format!(
                "mechanic=hotkey_equip state=equipped definition={definition} slot={:?}",
                equipped.slot
            ));
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("hotkey equip inventory delta missing".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    Ok(())
}

use oteryn_protocol_oteryn::item_view::{
    CharacterInventory, EquipmentSlot, EquippedItem, ItemHandle,
};

pub(super) fn moved_to_equipment<'a>(
    before: &CharacterInventory,
    after: &'a CharacterInventory,
    definition: u32,
    source: Option<ItemHandle>,
    slot: Option<EquipmentSlot>,
) -> Option<&'a EquippedItem> {
    after.equipment.iter().find(|entry| {
        let moved = entry.item;
        moved.item_definition_ref.get() == definition
            && slot.is_none_or(|slot| slot == entry.slot)
            && source.is_none_or(|source| source == moved.handle)
            && before.entries.contains(&moved)
            && !after
                .entries
                .iter()
                .any(|entry| entry.handle == moved.handle)
    })
}

#[test]
fn equipment_proof_requires_the_carried_handle_in_its_new_slot() -> TestResult {
    use oteryn_protocol_oteryn::item_view::ItemEntry;
    let item = ItemEntry {
        appearance_id: 4,
        handle: std::num::NonZeroU64::MIN,
        item_definition_ref: std::num::NonZeroU32::MIN,
        count: std::num::NonZeroU32::MIN,
        sub_type: 0,
    };
    let worn = ItemEntry {
        handle: std::num::NonZeroU64::new(2).ok_or("equipped handle")?,
        ..item
    };
    let before = CharacterInventory {
        entries: vec![item],
        equipment: vec![EquippedItem {
            slot: EquipmentSlot::Head,
            item: worn,
        }],
        ..Default::default()
    };
    assert!(moved_to_equipment(&before, &before, 1, Some(item.handle), None).is_none());
    let after = CharacterInventory {
        entries: vec![worn],
        equipment: vec![EquippedItem {
            slot: EquipmentSlot::Head,
            item,
        }],
        ..Default::default()
    };
    assert!(
        moved_to_equipment(
            &before,
            &after,
            1,
            Some(item.handle),
            Some(EquipmentSlot::Head)
        )
        .is_some()
    );
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EquipmentClass {
    /// A pattern names one of the eight hotkey slots (not Container or Extra).
    Equippable,
    /// Content says the item has no equipment pattern in those slots.
    NotEquipment,
    /// Unknown or conflicting content: proves nothing either way.
    Unproven,
}

/// The pinned item profile's equipment semantics for a carried definition.
fn pinned_equipment_class(definition: u32) -> TestResult<EquipmentClass> {
    use crate::content::ReferenceEquipmentSlot as Slot;
    use crate::content::ReferenceItemField::{Known, NotApplicable};
    let input = crate::qualification::client_world::native_input()?;
    let index = crate::content::item_ref::ItemDefinitionIndex::decode(
        &input
            .item_keys
            .as_ref()
            .ok_or("qualification Item keys missing")?
            .bytes,
    )?;
    let profiles: crate::content::native_gameplay::ItemProfilesDocument = serde_json::from_slice(
        &input
            .item_profiles
            .as_ref()
            .ok_or("pinned item profiles absent")?
            .bytes,
    )?;
    let record = profiles
        .records
        .iter()
        .find(|record| {
            let key = &record.production_definition.production_key;
            index
                .revision(key)
                .and_then(|revision| index.definition_ref("Item", key, revision))
                .is_some_and(|reference| reference.get() == definition)
        })
        .ok_or_else(|| {
            format!("hotkey equip definition {definition} absent from pinned item profiles")
        })?;
    Ok(match &record.semantics.equipment {
        NotApplicable => EquipmentClass::NotEquipment,
        Known(equipment) => match &equipment.patterns {
            NotApplicable => EquipmentClass::NotEquipment,
            Known(patterns) => {
                if patterns.iter().any(|pattern| {
                    !matches!(pattern.primary_slot, Known(Slot::Container | Slot::Extra))
                }) {
                    EquipmentClass::Equippable
                } else {
                    EquipmentClass::NotEquipment
                }
            }
            _ => EquipmentClass::Unproven,
        },
        _ => EquipmentClass::Unproven,
    })
}

#[test]
fn pinned_profiles_classify_the_sword_as_equipment_and_the_rope_as_not() -> TestResult<()> {
    let input = crate::qualification::client_world::native_input()?;
    let index = crate::content::item_ref::ItemDefinitionIndex::decode(
        &input
            .item_keys
            .as_ref()
            .ok_or("qualification Item keys missing")?
            .bytes,
    )?;
    let reference = |key: &str| -> TestResult<u32> {
        let revision = index.revision(key).ok_or("Item absent")?;
        Ok(index
            .definition_ref("Item", key, revision)
            .ok_or("Item ref missing")?
            .get())
    };
    assert_eq!(
        pinned_equipment_class(reference("oteryn:item.tibia.i3264")?)?,
        EquipmentClass::Equippable
    );
    assert_eq!(
        pinned_equipment_class(reference("oteryn:item.tibia.i3003")?)?,
        EquipmentClass::NotEquipment
    );
    Ok(())
}

fn equipped_from_carry<'a>(
    inventory: &'a CharacterInventory,
    definition: u32,
    carried: &[ItemHandle],
) -> Option<&'a EquippedItem> {
    inventory.equipment.iter().find(|equipped| {
        let item = &equipped.item;
        item.item_definition_ref.get() == definition
            && carried.contains(&item.handle)
            && inventory
                .entries
                .iter()
                .all(|entry| entry.handle != item.handle)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_protocol_oteryn::item_view::{EquipmentSlot, ItemEntry};

    #[test]
    fn hotkey_requires_the_carried_copy_to_move() -> TestResult {
        let stale = ItemEntry {
            appearance_id: 0,
            handle: std::num::NonZeroU64::MIN,
            item_definition_ref: std::num::NonZeroU32::MIN,
            count: std::num::NonZeroU32::MIN,
            sub_type: 0,
        };
        let carried = ItemEntry {
            handle: std::num::NonZeroU64::new(2).ok_or("zero handle")?,
            ..stale
        };
        let mut inventory = CharacterInventory {
            entries: vec![carried],
            equipment: vec![EquippedItem {
                slot: EquipmentSlot::Head,
                item: stale,
            }],
            ..Default::default()
        };
        assert!(equipped_from_carry(&inventory, 1, &[carried.handle]).is_none());
        inventory.entries = vec![stale];
        inventory.equipment[0].item = carried;
        assert!(equipped_from_carry(&inventory, 1, &[carried.handle]).is_some());
        Ok(())
    }
}
