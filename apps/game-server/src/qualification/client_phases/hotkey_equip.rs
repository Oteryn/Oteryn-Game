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
    let definition = client
        .inventory()
        .and_then(|inventory| inventory.entries.first())
        .map(|item| item.item_definition_ref.get())
        .ok_or("hotkey equip fixture item missing from inventory")?;
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
    if panel.outcome != Some(oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved) {
        return Err(format!("hotkey equip did not move equipment: {:?}", panel.outcome).into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(equipped) = client.inventory().and_then(|inventory| {
            inventory
                .equipment
                .iter()
                .find(|equipped| equipped.item.item_definition_ref.get() == definition)
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
