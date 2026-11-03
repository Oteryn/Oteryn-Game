//! Item Forge over the real node; reference: `client_e2e/item_forge.rs`.
use super::*;
pub(super) async fn item_forge(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime, client_ui};
    use oteryn_protocol_oteryn::item_forge::CAPABILITY_ITEM_FORGE_V1;
    if !client
        .selected_capabilities()
        .contains(&CAPABILITY_ITEM_FORGE_V1)
    {
        // Even QueryHistory needs the active native generation and its pinned Item index.
        evidence("mechanic=item_forge skipped=entry_room_has_no_native_item_content");
        return Ok(());
    }
    let mut panel = client_ui::item_forge::ItemForgePanel {
        selected: true,
        ..Default::default()
    };
    let command =
        client_ui::item_forge::control_request(&mut panel).ok_or("item forge history request")?;
    let response = client_runtime::raw_feature::send(
        client.session_mut(),
        &client_runtime::raw_feature::FeatureCommand {
            capability: command.capability,
            kind: command.command_type,
            payload: command.payload,
        },
    )
    .await?;
    client_ui::item_forge::apply_result(&mut panel, &response);
    if panel.response.is_empty() {
        return Err("item forge history response was empty".into());
    }
    evidence(
        "mechanic=item_forge transport=tls operation=query_history gap=entry_room_has_no_fusion_pair",
    );
    Ok(())
}
//! Item Forge over the real node; reference: `client_e2e/item_forge.rs`.
use super::*;
pub(super) async fn item_forge(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime, client_ui};
    use oteryn_protocol_oteryn::item_forge::CAPABILITY_ITEM_FORGE_V1;
    if !client
        .selected_capabilities()
        .contains(&CAPABILITY_ITEM_FORGE_V1)
    {
        // Even QueryHistory needs the active native generation and its pinned Item index.
        evidence("mechanic=item_forge skipped=entry_room_has_no_native_item_content");
        return Ok(());
    }
    let mut panel = client_ui::item_forge::ItemForgePanel {
        selected: true,
        ..Default::default()
    };
    let command =
        client_ui::item_forge::control_request(&mut panel).ok_or("item forge history request")?;
    let response = client_runtime::raw_feature::send(
        client.session_mut(),
        &client_runtime::raw_feature::FeatureCommand {
            capability: command.capability,
            kind: command.command_type,
            payload: command.payload,
        },
    )
    .await?;
    client_ui::item_forge::apply_result(&mut panel, &response);
    if panel.response.is_empty() {
        return Err("item forge history response was empty".into());
    }
    let result: serde_json::Value = serde_json::from_slice(&panel.response)?;
    if result.as_array().is_none_or(|history| !history.is_empty()) {
        return Err(format!("new character forge history: {result}").into());
    }
    evidence("mechanic=item_forge state=history_empty entries=0");
    evidence(
        "mechanic=item_forge skipped=production_starter_has_no_fusion_pair_or_materials_and_no_network_seed_api",
    );
    Ok(())
}
