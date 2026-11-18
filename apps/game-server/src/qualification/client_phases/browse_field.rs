//! Browse Field over the real node; reference: `client_e2e/browse_field.rs`.
use super::*;
use oteryn_dev_client::{AppliedDelta, SessionEvent};
use oteryn_protocol_oteryn::container_tree as tree;
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
        floor: -position.floor,
        ..Default::default()
    };
    client.take_events();
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
    let expected = oteryn_protocol_oteryn::item_view::WorldTilePosition {
        x: position.x,
        y: position.y,
        floor: -position.floor,
    };
    let find_view = |views: &tree::ContainerViews| {
        views.views.iter().position(|view| {
            view.browse
                .as_ref()
                .is_some_and(|browse| browse.position == expected)
        })
    };
    let delta = wait_for_views(client, |views| find_view(views).is_some()).await?;
    let view = &delta.value.views[find_view(&delta.value).ok_or("browse view delta missing")?];
    let view_id = view.view_id;
    evidence(&format!(
        "mechanic=browse_field state=opened position={position:?} entries={} view_id={} revision={}",
        view.entries.len(),
        view.view_id,
        delta.new_revision
    ));
    let close = tree::encode_container_view_intent(&tree::ContainerViewIntent::Close { view_id })
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
    wait_for_views(client, |views| {
        views.views.iter().all(|view| view.view_id != view_id)
    })
    .await?;
    Ok(())
}

async fn wait_for_views(
    client: &mut oteryn_dev_client::DevClientSession,
    matches: impl Fn(&tree::ContainerViews) -> bool,
) -> TestResult<AppliedDelta<tree::ContainerViews>> {
    let budget = Duration::from_secs(5);
    tokio::time::timeout(budget, async {
        loop {
            let events = client.session_mut().wait_for_events(budget).await?;
            if let Some(delta) = matching_views(events, &matches) {
                return Ok(delta);
            }
        }
    })
    .await
    .map_err(|_| "browse container view update deadline")?
}

fn matching_views(
    events: Vec<SessionEvent>,
    matches: impl Fn(&tree::ContainerViews) -> bool,
) -> Option<AppliedDelta<tree::ContainerViews>> {
    events.into_iter().find_map(|event| match event {
        SessionEvent::ContainerViews(delta) if matches(&delta.value) => Some(delta),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browse_view_update_can_follow_an_unrelated_delta() {
        let events = vec![
            SessionEvent::Inventory(AppliedDelta {
                server_sequence: 2,
                base_revision: 0,
                new_revision: 1,
                value: Default::default(),
            }),
            SessionEvent::ContainerViews(AppliedDelta {
                server_sequence: 3,
                base_revision: 0,
                new_revision: 1,
                value: Default::default(),
            }),
        ];
        assert_eq!(
            matching_views(events, |_| true).map(|delta| delta.server_sequence),
            Some(3)
        );
    }
}
