//! I05 native-client replay capture.
//!
//! The real consumer is `crates/session/src/domain_dispatch.rs`,
//! `Session::apply_delta`'s `STATE_DOMAIN_CHARACTER_INVENTORY` arm. It decodes
//! the domain-9 payload, replaces `Session::inventory`, and exposes that state
//! through `crates/session/src/session_access.rs::Session::inventory` (and thus
//! `tools/dev-client/src/lib.rs::DevClientSession::inventory`).
use super::*;
use oteryn_protocol_oteryn::item_view as items;

const ACTIVE_ENERGY_RING: u32 = 3088;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = verdict::observe(capture(clients)).await;
    evidence(&format!(
        "e2e_all scenario=I05.client status={}{}",
        if result.is_ok() { "PASS" } else { "FAIL" },
        result.as_ref().err().map_or(String::new(), |error| format!(" evidence={error}"))
    ));
}

fn reequipped_ring(
    delta: &oteryn_dev_client::AppliedDelta<items::CharacterInventory>,
    after: u64,
    handle: items::ItemHandle,
    active: std::num::NonZeroU32,
) -> Option<&items::EquippedItem> {
    if delta.server_sequence <= after {
        return None;
    }
    delta.value.equipment.iter().find(|row| {
        row.slot == items::EquipmentSlot::Ring && row.item.handle == handle && row.item.item_definition_ref == active
    })
}

async fn capture(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0xb2).await?;
    let active = provisioning::item_reference(&provisioning::item_index()?, ACTIVE_ENERGY_RING)?;
    let handle = client
        .inventory()
        .and_then(|view| view.equipment.iter().find(|row| {
            row.slot == items::EquipmentSlot::Ring && row.item.item_definition_ref == active
        }))
        .map(|row| row.item.handle)
        .ok_or("i05-active-ring-not-durable")?;

    client.take_events();
    let moved = client
        .move_item(&items::ItemMoveIntent {
            source: handle,
            destination: items::ItemMoveDestination::MainBackpack,
        })
        .await?;
    if moved.outcome != items::ItemMoveOutcome::Moved {
        return Err(format!("i05-unequip-refused:{moved:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if client.take_events().into_iter().any(|event| matches!(
            event,
            oteryn_dev_client::SessionEvent::Inventory(delta)
                if delta.server_sequence > moved.result_server_sequence
                    && delta.value.entries.iter().any(|item| item.handle == handle)
                    && !delta.value.equipment.iter().any(|row| row.slot == items::EquipmentSlot::Ring)
        )) {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("i05-unequip-inventory-delta-missing".into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
    let moved = client
        .move_item(&items::ItemMoveIntent {
            source: handle,
            destination: items::ItemMoveDestination::Equipment(items::EquipmentSlot::Ring),
        })
        .await?;
    if moved.outcome != items::ItemMoveOutcome::Moved {
        return Err(format!("i05-reequip-refused:{moved:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let delta = loop {
        if let Some(delta) = client.take_events().into_iter().find_map(|event| match event {
            oteryn_dev_client::SessionEvent::Inventory(delta)
                if reequipped_ring(&delta, moved.result_server_sequence, handle, active).is_some() => Some(delta),
            _ => None,
        }) {
            break delta;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("i05-reequip-original-active-ring-delta-missing".into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    };
    let equipped = reequipped_ring(&delta, moved.result_server_sequence, handle, active)
        .ok_or("i05-ring-slot-empty-after-reequip")?;
    let payload = items::encode_character_inventory_with_equip_drop(&delta.value, true)
        .map_err(|error| format!("i05-capture-encode:{error:?}"))?;
    write_capture(
        "i05-inventory.bin",
        &payload,
        &format!(
            "{} {} {} {} {}\n",
            equipped.item.item_definition_ref,
            equipped.item.count,
            delta.base_revision,
            delta.new_revision,
            delta.server_sequence
        ),
    );
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err("i05-client-capture-logout-refused".into());
    }
    Ok(())
}

fn write_capture(name: &str, payload: &[u8], expected: &str) {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    let path = Path::new(&path);
    if let Err(error) = std::fs::write(path.join(name), payload) {
        evidence(&format!("I05_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
    if let Err(error) = std::fs::write(path.join("i05-expected.txt"), expected) {
        evidence(&format!(
            "I05_CLIENT_CAPTURE_WRITE_ERROR file=i05-expected.txt error={error}"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reequipped_ring_requires_fresh_frame_and_original_instance() -> TestResult {
        let handle = items::ItemHandle::new(42).ok_or("handle")?;
        let active = std::num::NonZeroU32::new(ACTIVE_ENERGY_RING).ok_or("definition")?;
        let mut delta = oteryn_dev_client::AppliedDelta {
            server_sequence: 5,
            base_revision: 1,
            new_revision: 2,
            value: items::CharacterInventory {
                equipment: vec![items::EquippedItem {
                    slot: items::EquipmentSlot::Ring,
                    item: items::ItemEntry {
                        appearance_id: 0,
                        handle,
                        item_definition_ref: active,
                        count: std::num::NonZeroU32::MIN,
                        sub_type: 0,
                    },
                }],
                ..Default::default()
            },
        };
        assert!(reequipped_ring(&delta, 6, handle, active).is_none());
        delta.server_sequence = 7;
        assert!(reequipped_ring(&delta, 6, items::ItemHandle::MIN, active).is_none());
        assert!(reequipped_ring(&delta, 6, handle, std::num::NonZeroU32::MIN).is_none());
        assert!(reequipped_ring(&delta, 6, handle, active).is_some());
        Ok(())
    }
}
