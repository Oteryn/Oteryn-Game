//! I05 native-client replay capture.
//!
//! The real consumer is `crates/session/src/domain_dispatch.rs`,
//! `Session::apply_delta`'s `STATE_DOMAIN_CHARACTER_INVENTORY` arm. It decodes
//! the domain-9 payload, replaces `Session::inventory`, and exposes that state
//! through `crates/session/src/session_access.rs::Session::inventory` (and thus
//! `tools/dev-client/src/lib.rs::DevClientSession::inventory`).
use super::*;
use super::{
    row_connect_world as connect_world,
    row_finish as row_logout,
};
use oteryn_protocol_oteryn::item_view as items;

const ACTIVE_ENERGY_RING: u32 = 3088;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = capture(clients).await;
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
    row_complete(async {
    let mut client = connect_world(clients, 0xb2).await?;
    let mut row_needs_logout_0 = true;
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(300);
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        let active =
            provisioning::item_reference(&provisioning::item_index()?, ACTIVE_ENERGY_RING)?;
        let handle = client
            .inventory()
            .and_then(|view| {
                view.equipment.iter().find(|row| {
                    row.slot == items::EquipmentSlot::Ring && row.item.item_definition_ref == active
                })
            })
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
        client.take_events();
        let moved = client
            .move_item(&items::ItemMoveIntent {
                source: handle,
                destination: items::ItemMoveDestination::Equipment(items::EquipmentSlot::Ring),
            })
            .await?;
        if moved.outcome != items::ItemMoveOutcome::Moved {
            return Err(format!("i05-reequip-refused:{moved:?}").into());
        }
        let delta = client
            .take_events()
            .into_iter()
            .find_map(|event| match event {
                oteryn_dev_client::SessionEvent::Inventory(delta) => Some(delta),
                _ => None,
            })
            .ok_or("i05-reequip-inventory-delta-missing")?;
        let equipped = delta
            .value
            .equipment
            .iter()
            .find(|row| row.slot == items::EquipmentSlot::Ring)
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
        )?;
        let logout = client.row_logout().await?;
        if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Err("i05-client-capture-logout-refused".into());
        }
        row_needs_logout_0 = false;
        Ok(())
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    if row_needs_logout_0 {
        tokio::time::timeout(Duration::from_secs(10), row_logout(&mut client)).await??;
    }
    row_result

    }).await?
}

fn write_capture(name: &str, payload: &[u8], expected: &str) -> TestResult {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    let path = Path::new(&path);
    std::fs::write(path.join(name), payload)?;
    std::fs::write(path.join("i05-expected.txt"), expected)?;

    Ok(())
}
