//! C02 client capture: drive the ordinary ATTACK route and retain the inventory payload that the
//! native session consumes after one ranged ammunition debit.

use super::*;
use oteryn_dev_client::SessionEvent;
use oteryn_protocol_oteryn::item_view;

fn write_capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)
        .map_err(|error| verdict::context(format!("C02 capture file={name}"), error))
}

fn observed_ammunition(
    before: Option<item_view::ItemEntry>,
    after: u64,
    events: Vec<SessionEvent>,
) -> Option<item_view::CharacterInventory> {
    let before = before?;
    events.into_iter().find_map(|event| match event {
        SessionEvent::Inventory(delta) if delta.server_sequence > after => {
            let exact_debit = match ammunition(&delta.value) {
                Some(ammo) => ammo.handle == before.handle
                    && ammo.item_definition_ref == before.item_definition_ref
                    && ammo.count.get().checked_add(1) == Some(before.count.get()),
                None => before.count.get() == 1,
            };
            exact_debit.then_some(delta.value)
        }
        _ => None,
    })
}

fn ammunition(inventory: &item_view::CharacterInventory) -> Option<item_view::ItemEntry> {
    inventory.equipment.iter().find(|item| item.slot == item_view::EquipmentSlot::Ammo).map(|item| item.item)
}

async fn equip(
    client: &mut oteryn_dev_client::DevClientSession,
    reference: std::num::NonZeroU32,
    slot: item_view::EquipmentSlot,
) -> TestResult {
    let inventory = client.inventory().ok_or("C02 inventory absent")?;
    if inventory.equipment.iter().any(|entry| entry.slot == slot && entry.item.item_definition_ref == reference) {
        return Ok(());
    }
    let handle = inventory.entries.iter().find(|entry| entry.item_definition_ref == reference)
        .ok_or("C02 purchased item absent")?.handle;
    let command = client.move_item(&item_view::ItemMoveIntent {
        source: handle,
        destination: item_view::ItemMoveDestination::Equipment(slot),
    }).await?;
    if command.status != CommandStatus::Accepted || command.outcome != item_view::ItemMoveOutcome::Moved {
        return Err(format!("C02 failed to equip {slot:?}: {command:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if client.take_events().into_iter().any(|event| matches!(
            event,
            SessionEvent::Inventory(delta)
                if delta.server_sequence > command.result_server_sequence
                    && delta.value.equipment.iter().any(|entry| entry.slot == slot && entry.item.handle == handle)
        )) {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("C02 missing inventory delta for equipped {slot:?}").into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0xc2).await?;
    let definitions = provisioning::item_index()?;
    for (id, quantity, slot) in [
        (3350, 1, item_view::EquipmentSlot::RightHand),
        (3447, 2, item_view::EquipmentSlot::Ammo),
    ] {
        let reference = provisioning::item_reference(&definitions, id)?;
        let inventory = client.inventory().ok_or("C02 inventory absent")?;
        if !inventory.equipment.iter().any(|entry| entry.slot == slot && entry.item.item_definition_ref == reference)
            && !inventory.entries.iter().any(|entry| entry.item_definition_ref == reference)
        {
            let needed = full_gold::price(id, quantity)?;
            full_gold::earn(&mut client, world, &metadata.spawns, "C02_CLIENT", needed).await?;
            full_gold::buy(&mut client, world, &metadata.npcs, id, quantity).await?;
        }
        equip(&mut client, reference, slot).await?;
    }
    travel_to_spawn(
        &mut client,
        world,
        &metadata.spawns,
        "oteryn:creature.rat",
    )
    .await?;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities
                .iter()
                .find(|entity| entity.kind == oteryn_dev_client::EntityKind::Creature)
        })
        .ok_or("C02 attack route found no visible creature")?
        .entity;
    let before = client.inventory().and_then(ammunition).ok_or("C02 no equipped ammunition")?;
    client.take_events();
    let reply = client.session_mut().attack_target(Some(&target)).await?;
    if reply.status != CommandStatus::Accepted || reply.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(format!("C02 ATTACK refused: {reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(12);
    let inventory = loop {
        if let Some(inventory) = observed_ammunition(Some(before), reply.result_server_sequence, client.take_events()) {
            break inventory;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("C02 attack did not produce a one-ammunition debit after sequence {}", reply.result_server_sequence).into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    };
    let stopped = client.session_mut().attack_target(None).await?;
    if stopped.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(format!("C02 stop attack refused: {stopped:?}").into());
    }
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_some() {
        let payload = item_view::encode_character_inventory_with_equip_drop(&inventory, true)
            .map_err(|error| format!("C02 inventory encode: {error:?}"))?;
        write_capture("c02-inventory.bin", &payload)?;
        let expected = inventory
            .equipment
            .iter()
            .filter(|item| item.slot == item_view::EquipmentSlot::Ammo)
            .map(|item| {
                format!(
                    "{} {}\n",
                    item.item.item_definition_ref, item.item.count
                )
            })
            .collect::<String>();
        write_capture("c02-expected.txt", expected.as_bytes())?;
    }
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=C02.client status=CAPTURED attack={:?} ammo={}",
        reply.disposition,
        inventory
            .equipment
            .iter()
            .filter(|item| item.slot == item_view::EquipmentSlot::Ammo)
            .map(|item| u64::from(item.item.count.get()))
            .sum::<u64>()
    ));
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if let Err(error) = verdict::observe(journey(clients)).await {
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C02.client status=FAIL evidence={}",
            error.to_string().replace(' ', "-")
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ammunition_capture_requires_fresh_exact_debit() -> TestResult {
        let before = item_view::ItemEntry {
            appearance_id: 0,
            handle: item_view::ItemHandle::MIN,
            item_definition_ref: std::num::NonZeroU32::MIN,
            count: std::num::NonZeroU32::new(3).ok_or("ammo count")?,
            sub_type: 0,
        };
        let event = |server_sequence, count| -> TestResult<SessionEvent> {
            Ok(SessionEvent::Inventory(oteryn_dev_client::AppliedDelta {
                server_sequence,
                base_revision: 1,
                new_revision: 2,
                value: item_view::CharacterInventory {
                    equipment: vec![item_view::EquippedItem {
                        slot: item_view::EquipmentSlot::Ammo,
                        item: item_view::ItemEntry {
                            count: std::num::NonZeroU32::new(count).ok_or("ammo count")?,
                            ..before
                        },
                    }],
                    ..Default::default()
                },
            }))
        };
        assert!(observed_ammunition(Some(before), 4, vec![event(5, 3)?]).is_none());
        assert!(observed_ammunition(Some(before), 4, vec![event(4, 2)?]).is_none());
        assert!(observed_ammunition(Some(before), 4, vec![event(5, 1)?]).is_none());
        assert!(observed_ammunition(Some(before), 4, vec![event(5, 2)?]).is_some());
        assert!(observed_ammunition(None, 4, vec![event(5, 2)?]).is_none());
        Ok(())
    }
}
