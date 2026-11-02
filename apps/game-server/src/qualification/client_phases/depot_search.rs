//! Depot Search over the real node; reference: `client_e2e/depot_search.rs`.
use super::*;
pub(super) async fn depot_search(
    client: &mut oteryn_dev_client::DevClientSession,
) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime, client_ui};
    use oteryn_protocol_oteryn::depot_search::CAPABILITY_DEPOT_SEARCH_V1;
    if !client
        .selected_capabilities()
        .contains(&CAPABILITY_DEPOT_SEARCH_V1)
    {
        return Err("depot search capability was not selected".into());
    }
    let mut panel = client_ui::depot_search::DepotSearchPanel {
        selected: true,
        item_id: 37109,
        ..Default::default()
    };
    let command = client_ui::depot_search::search(&mut panel).ok_or("depot search request")?;
    let response = client_runtime::raw_feature::send(
        client.session_mut(),
        &client_runtime::raw_feature::FeatureCommand {
            capability: command.capability,
            kind: command.command_type,
            payload: command.payload,
        },
    )
    .await?;
    client_ui::depot_search::apply_result(&mut panel, &response);
    if panel.reply.is_none() {
        return Err("depot search reply was not decoded".into());
    }
    evidence("mechanic=depot_search transport=tls result=decoded fixture_item=37109");
    Ok(())
}
