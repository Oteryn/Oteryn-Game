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
    let outcome =
        oteryn_protocol_oteryn::container_tree::decode_container_view_result(&panel.response)
            .map_err(|e| format!("browse field outcome: {e:?}"))?;
    if outcome != oteryn_protocol_oteryn::container_tree::ContainerViewOutcome::Opened {
        return Err("browse field did not receive a container-view outcome".into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(
            oteryn_protocol_oteryn::container_tree::STATE_DOMAIN_CONTAINER_VIEWS,
            oteryn_protocol_oteryn::container_tree::DELTA_TYPE_CONTAINER_VIEWS_V1,
        )
        .await?;
    let views =
        oteryn_protocol_oteryn::container_tree::decode_container_views_for(&delta.payload, true)
            .map_err(|e| format!("browse views: {e:?}"))?;
    let view = views
        .views
        .iter()
        .find(|view| view.browse.is_some())
        .ok_or("browse view delta missing")?;
    let browse = view.browse.as_ref().ok_or("browse page missing")?;
    if browse.position.x != position.x
        || browse.position.y != position.y
        || browse.position.floor != position.floor
    {
        return Err("browse field returned a different tile".into());
    }
    evidence(&format!(
        "mechanic=browse_field state=opened position={position:?} entries={} view_id={} revision={}",
        view.entries.len(),
        view.view_id,
        delta.new_revision
    ));
    use oteryn_protocol_oteryn::container_tree as tree;
    let close = tree::encode_container_view_intent(&tree::ContainerViewIntent::Close {
        view_id: view.view_id,
    })
    .map_err(|error| format!("browse close encoding: {error:?}"))?;
    let closed = client
        .session_mut()
        .gated_command(
            tree::CAPABILITY_CONTAINER_TREE_V1,
            tree::COMMAND_TYPE_CONTAINER_VIEW_INTENT,
            &close,
        )
        .await?;
    if tree::decode_container_view_result(&closed.payload)
        .map_err(|error| format!("browse close: {error:?}"))?
        != tree::ContainerViewOutcome::Closed
    {
        return Err("browse close refused".into());
    }
    let closed = client
        .session_mut()
        .read_gated_delta(
            tree::STATE_DOMAIN_CONTAINER_VIEWS,
            tree::DELTA_TYPE_CONTAINER_VIEWS_V1,
        )
        .await?;
    if !tree::decode_container_views_for(&closed.payload, true)
        .map_err(|error| format!("closed browse views: {error:?}"))?
        .views
        .is_empty()
    {
        return Err("browse field remained open".into());
    }
    Ok(())
}
