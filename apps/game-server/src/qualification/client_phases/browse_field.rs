//! Browse Field over the real node; reference: `client_e2e/browse_field.rs`.
use super::*;

fn panel_at(
    position: ActorPosition,
) -> oteryn_dev_client::client_ui::browse_field::BrowseFieldPanel {
    oteryn_dev_client::client_ui::browse_field::BrowseFieldPanel {
        selected: true,
        x: position.x,
        y: position.y,
        floor: position.floor,
        ..Default::default()
    }
}

fn check_browsed_tile(
    position: ActorPosition,
    observed: oteryn_protocol_oteryn::item_view::WorldTilePosition,
) -> TestResult {
    if observed.x != position.x || observed.y != position.y || observed.floor != position.floor {
        return Err("browse field returned a different tile".into());
    }
    Ok(())
}

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
    let mut panel = panel_at(position);
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
    let views = selected_container_views(client.selected_capabilities(), &delta.payload)
        .map_err(|e| format!("browse views: {e:?}"))?;
    let view = views
        .views
        .iter()
        .find(|view| view.browse.is_some())
        .ok_or("browse view delta missing")?;
    let browse = view.browse.as_ref().ok_or("browse page missing")?;
    check_browsed_tile(position, browse.position)?;
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
    if !selected_container_views(client.selected_capabilities(), &closed.payload)
        .map_err(|error| format!("closed browse views: {error:?}"))?
        .views
        .is_empty()
    {
        return Err("browse field remained open".into());
    }
    Ok(())
}

#[test]
fn browse_panel_and_readback_keep_the_observed_global_floor() -> TestResult {
    use oteryn_protocol_oteryn::browse_field::{BrowseFieldIntent, decode_browse_field_intent};
    use oteryn_protocol_oteryn::item_view::WorldTilePosition;
    for floor in [0, 7, 15] {
        let position = ActorPosition {
            x: 32369,
            y: 32238,
            floor,
        };
        let expected = WorldTilePosition {
            x: position.x,
            y: position.y,
            floor,
        };
        let mut panel = panel_at(position);
        let command = oteryn_dev_client::client_ui::browse_field::open(&mut panel)
            .ok_or("browse field request refused the observed tile")?;
        assert_eq!(
            decode_browse_field_intent(&command.payload),
            Ok(BrowseFieldIntent::Open { position: expected })
        );
        check_browsed_tile(position, expected)?;
        for wrong in [
            WorldTilePosition {
                x: expected.x + 1,
                ..expected
            },
            WorldTilePosition {
                y: expected.y + 1,
                ..expected
            },
            WorldTilePosition {
                floor: (floor + 1) % 16,
                ..expected
            },
        ] {
            assert!(check_browsed_tile(position, wrong).is_err());
        }
    }
    Ok(())
}
