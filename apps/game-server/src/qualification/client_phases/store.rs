//! Platform Store over the real node; reference: `client_e2e/store.rs`.
use super::*;
pub(super) async fn store(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime, client_ui};
    use oteryn_protocol_oteryn::store::CAPABILITY_STORE_V1;
    if !client
        .selected_capabilities()
        .contains(&CAPABILITY_STORE_V1)
    {
        evidence(
            "mechanic=store capability=154_not_offered TODO=compose_Platform_Store_client_in_node",
        );
        return Ok(());
    }
    let mut panel = client_ui::store::StorePanel {
        selected: true,
        category: "featured".into(),
        ..Default::default()
    };
    let command = client_ui::store::catalogue(&mut panel).ok_or("store catalogue request")?;
    let response = client_runtime::raw_feature::send(
        client.session_mut(),
        &client_runtime::raw_feature::FeatureCommand {
            capability: command.capability,
            kind: command.command_type,
            payload: command.payload,
        },
    )
    .await?;
    client_ui::store::apply_result(&mut panel, &response);
    if panel.catalogue.is_none() {
        return Err("store catalogue was not decoded".into());
    }
    evidence(
        "mechanic=store capability=154_offered catalogue=decoded purchase=not_attempted_no_test_currency_contract",
    );
    Ok(())
}
