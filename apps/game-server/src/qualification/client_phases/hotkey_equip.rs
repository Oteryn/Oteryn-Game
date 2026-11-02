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
    let definition = client
        .inventory()
        .and_then(|inventory| inventory.entries.first())
        .map(|item| item.item_definition_ref.get())
        .ok_or("hotkey equip fixture item missing from inventory")?;
    let mut panel = client_ui::hotkey_equip::HotkeyEquipPanel {
        selected: true,
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
    if panel.outcome.is_none() {
        return Err("hotkey equip outcome was not decoded".into());
    }
    evidence("mechanic=hotkey_equip transport=tls result=decoded");
    Ok(())
}
