//! Browse Field over the real node; reference: `client_e2e/browse_field.rs`.
use super::*;
pub(super) async fn browse_field(
    client: &mut oteryn_dev_client::DevClientSession,
) -> TestResult<()> {
    use oteryn_dev_client::{client_runtime, client_ui};
    use oteryn_protocol_oteryn::browse_field::CAPABILITY_BROWSE_FIELD_V1;
    if !client
        .selected_capabilities()
        .contains(&CAPABILITY_BROWSE_FIELD_V1)
    {
        // The entry room has no bundle map: browse_field::observe needs its tile overlay.
        evidence("mechanic=browse_field skipped=entry_room_has_no_bundle_map");
        return Ok(());
    }
    let position = client.world_spatial().actor_position;
    let mut panel = client_ui::browse_field::BrowseFieldPanel {
        selected: true,
        x: position.x,
        y: position.y,
        floor: position.floor,
        ..Default::default()
    };
    let command = client_ui::browse_field::open(&mut panel).ok_or("browse field request")?;
    let response = client_runtime::raw_feature::send(
        client.session_mut(),
        &client_runtime::raw_feature::FeatureCommand {
            capability: command.capability,
            kind: command.command_type,
            payload: command.payload,
        },
    )
    .await?;
    client_ui::browse_field::apply_result(&mut panel, &response);
    if panel.response.is_empty() {
        return Err("browse field did not receive a container-view outcome".into());
    }
    evidence("mechanic=browse_field transport=tls result=container_view_received");
    Ok(())
}
