//! I05 native-client replay capture.
//!
//! The real consumer is `crates/session/src/domain_dispatch.rs`,
//! `Session::apply_delta`'s `STATE_DOMAIN_CHARACTER_INVENTORY` arm. It decodes
//! the domain-9 payload, replaces `Session::inventory`, and exposes that state
//! through `crates/session/src/session_access.rs::Session::inventory` (and thus
//! `tools/dev-client/src/lib.rs::DevClientSession::inventory`).
use super::*;
use oteryn_dev_client::client_runtime::inventory::MoveRequest;
use oteryn_protocol_oteryn::item_view as items;

const ENERGY_RING: u32 = 3051;
const ACTIVE_ENERGY_RING: u32 = 3088;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = verdict::observe(capture(clients)).await;
    evidence(&format!(
        "e2e_all scenario=I05.client status={}{}",
        if result.is_ok() { "PASS" } else { "FAIL" },
        result
            .as_ref()
            .err()
            .map_or(String::new(), |error| format!(" evidence={error}"))
    ));
}

async fn capture(clients: &SeamClients<'_>) -> TestResult {
    let identity = character_prep::dedicated_character(clients, "NODE_BOOT_Q_I05_ACCOUNT").await?;
    let clients = &identity.clients(clients);
    let index = provisioning::item_index()?;
    let base = provisioning::item_reference(&index, ENERGY_RING)?;
    let active = provisioning::item_reference(&index, ACTIVE_ENERGY_RING)?;

    character_prep::declare_seeded("I05.client", "one base energy ring", "equipment transform");
    let client = connect_character(clients, 0xb2, 0).await?;
    character_prep::grant_item(&client, ENERGY_RING, 1).await?;
    logout(client).await?;

    let mut client = connect_character(clients, 0xb3, 0).await?;
    let inventory = client.inventory().ok_or("i05-grant-inventory-missing")?;
    let mut rings = inventory
        .entries
        .iter()
        .filter(|item| item.item_definition_ref == base);
    let ring = rings.next().ok_or("i05-base-ring-not-durable")?;
    if ring.count.get() != 1 || rings.next().is_some() || equipped_ring(inventory).is_some() {
        return Err("i05-unexpected-starting-ring-custody".into());
    }
    let handle = ring.handle;
    let slot = items::ItemMoveDestination::Equipment(items::EquipmentSlot::Ring);
    let delta = move_ring(&mut client, handle, slot).await?;
    verify_active_ring(&delta.value, active)?;
    logout(client).await?;

    let mut client = connect_character(clients, 0xb4, 0).await?;
    let handle = verify_active_ring(
        client
            .inventory()
            .ok_or("i05-initial-equipped-inventory-missing")?,
        active,
    )?;
    let removed = move_ring(
        &mut client,
        handle,
        items::ItemMoveDestination::MainBackpack,
    )
    .await?;
    if equipped_ring(&removed.value).is_some()
        || !removed.value.entries.iter().any(|item| {
            item.handle == handle && item.item_definition_ref == base && item.count.get() == 1
        })
    {
        return Err("i05-unequip-did-not-restore-base-ring".into());
    }
    let delta = move_ring(&mut client, handle, slot).await?;
    verify_active_ring(&delta.value, active)?;
    let payload = items::encode_character_inventory_with_equip_drop(&delta.value, true)
        .map_err(|error| format!("i05-capture-encode:{error:?}"))?;
    logout(client).await?;

    let client = connect_character(clients, 0xb5, 0).await?;
    verify_active_ring(
        client
            .inventory()
            .ok_or("i05-final-equipped-inventory-missing")?,
        active,
    )?;
    logout(client).await?;
    write_capture(
        &payload,
        &format!(
            "{} 1 {} {} {}\n",
            active, delta.base_revision, delta.new_revision, delta.server_sequence
        ),
    )?;
    evidence("I05_CLIENT_CHECK PASS observed=base-to-active+active-to-base+active-after-relog");
    Ok(())
}

fn equipped_ring(inventory: &items::CharacterInventory) -> Option<&items::EquippedItem> {
    inventory
        .equipment
        .iter()
        .find(|entry| entry.slot == items::EquipmentSlot::Ring)
}

fn verify_active_ring(
    inventory: &items::CharacterInventory,
    active: std::num::NonZeroU32,
) -> TestResult<items::ItemHandle> {
    let ring = equipped_ring(inventory).ok_or("i05-active-ring-not-durable")?;
    if ring.item.item_definition_ref != active || ring.item.count.get() != 1 {
        return Err("i05-ring-slot-does-not-contain-one-active-ring".into());
    }
    Ok(ring.item.handle)
}

async fn move_ring(
    client: &mut oteryn_dev_client::DevClientSession,
    source: items::ItemHandle,
    destination: items::ItemMoveDestination,
) -> TestResult<oteryn_dev_client::AppliedDelta<items::CharacterInventory>> {
    client.take_events();
    let moved = client
        .session_mut()
        .move_item_counted(MoveRequest {
            source,
            destination,
            count: None,
        })
        .await?;
    if moved.outcome != items::ItemMoveOutcome::Moved {
        return Err(format!("i05-ring-move-refused:{moved:?}").into());
    }
    let result_sequence = client.last_server_sequence();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in client.take_events() {
            if let oteryn_dev_client::SessionEvent::Inventory(delta) = event {
                if delta.server_sequence <= result_sequence {
                    return Err("i05-inventory-delta-precedes-move-result".into());
                }
                return Ok(delta);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("i05-ring-move-inventory-delta-missing".into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}

async fn logout(mut client: oteryn_dev_client::DevClientSession) -> TestResult {
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("i05-client-capture-logout-refused:{:?}", logout.result).into());
    }
    Ok(())
}

fn write_capture(payload: &[u8], expected: &str) -> TestResult {
    use std::io::Write as _;

    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    let path = Path::new(&path);
    let mut inventory_file = std::fs::File::create_new(path.join("i05-inventory.bin"))?;
    let mut expected_file = std::fs::File::create_new(path.join("i05-expected.txt"))?;
    inventory_file.write_all(payload)?;
    expected_file.write_all(expected.as_bytes())?;
    Ok(())
}
