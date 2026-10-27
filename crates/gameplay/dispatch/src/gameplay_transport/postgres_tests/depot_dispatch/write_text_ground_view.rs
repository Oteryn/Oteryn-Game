//! WRITE-1 external-view prerequisite; TEXT remains inactive.
use super::*;
use crate::gameplay_transport::container_view::ContainerObservation;
use crate::gameplay_transport::item_move::{LoopMove, LoopMoveResult, LoopTarget};
use oteryn_protocol_oteryn::container_tree::{
    COMMAND_TYPE_CONTAINER_VIEW_INTENT, ContainerViewIntent, ContainerViewOutcome,
    decode_container_view_result, decode_container_views, encode_container_view_intent,
};
use oteryn_protocol_oteryn::{MessageType, decode_snapshot_chunk, decode_wire_envelope};

pub(super) async fn exercise(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: crate::gameplay_transport::connection::AdmittedSession,
    item: [u8; 16],
    non_container: [u8; 16],
    definition: std::num::NonZeroU32,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let fence = admitted.item_fence.ok_or("fence")?;
    let key = ItemKey::Instance(item);
    let command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: 800,
        item_fence: Some(fence),
    };
    assert!(matches!(
        owner.observe_container(actor, Some(fence), key).await,
        Some(ContainerObservation::Container {
            parent: Some(_),
            ..
        })
    ));
    assert_eq!(
        owner
            .move_loop_item(
                actor,
                command,
                LoopMove {
                    item: key,
                    count: None,
                    target: LoopTarget::Ground(
                        oteryn_protocol_oteryn::item_view::WorldTilePosition {
                            x: 7,
                            y: 0,
                            floor: -7,
                        }
                    ),
                }
            )
            .await,
        LoopMoveResult::Committed
    );
    assert!(
        owner
            .root
            .read_item_loop_container(owner.character, fence.character_id, item)
            .await?
            .is_none()
    );
    assert!(
        matches!(owner.observe_container(actor, Some(fence), key).await,
        Some(ContainerObservation::Container { parent: None, entries, .. }) if entries.is_empty())
    );
    // Each stale placement component independently fails closed in the native read.
    let persisted = owner
        .root
        .read_item_loop_ground(owner.world_id, owner.channel_id)
        .await?
        .into_iter()
        .find(|entry| entry.item.item_instance_id == item)
        .ok_or("persisted Ground root")?
        .item
        .ground;
    let ground = crate::durability::item_loop_move::LoopGround {
        spatial_position: persisted.spatial_position,
        map_revision: persisted.map_revision,
        content_revision: persisted.content_revision,
        placement_context: persisted.native_room_placement_context,
    };
    assert!(
        owner
            .root
            .read_item_loop_ground_container(owner.character, fence, item, ground.clone())
            .await?
            .is_some()
    );
    for component in 0..4 {
        let mut invalid = ground.clone();
        match component {
            0 => invalid.spatial_position[3] ^= 1,
            1 => invalid.map_revision.push('x'),
            2 => invalid.content_revision.push('x'),
            _ => invalid.placement_context[0] ^= 1,
        }
        assert!(
            owner
                .root
                .read_item_loop_ground_container(owner.character, fence, item, invalid)
                .await?
                .is_none()
        );
    }
    let mut stale = fence;
    stale.character_lease_generation += 1;
    assert_eq!(
        owner.observe_container(actor, Some(stale), key).await,
        Some(ContainerObservation::Gone)
    );

    // A Ground consumable must never be reported as a carried non-container:
    // USE consumes that observation as a carried-tree hint before revalidation.
    let non_container = ItemKey::Instance(non_container);
    assert_eq!(
        owner
            .move_loop_item(
                actor,
                UseCommand {
                    command_id: 799,
                    ..command
                },
                LoopMove {
                    item: non_container,
                    count: None,
                    target: LoopTarget::Ground(
                        oteryn_protocol_oteryn::item_view::WorldTilePosition {
                            x: 7,
                            y: 0,
                            floor: -7,
                        }
                    )
                }
            )
            .await,
        LoopMoveResult::Committed
    );
    assert_eq!(
        owner
            .observe_container(actor, Some(fence), non_container)
            .await,
        Some(ContainerObservation::Gone)
    );

    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 12, 14, 18])
            .ok_or("capabilities")?;
    assert!(
        admitted
            .continuity
            .selected_capabilities
            .command_selected(COMMAND_TYPE_CONTAINER_VIEW_INTENT)
    );
    // Obtain the actual advertised map handle from a production-shaped snapshot.
    // Replaying the same initial continuity below issues that same initial table.
    let (_, output) =
        super::super::weather_wire::fixture_access::serve_input(owner, admitted, &[]).await?;
    let mut handle = None;
    for frame in frames(&output)? {
        let envelope = decode_wire_envelope(frame)?;
        if envelope.message_type() == MessageType::SnapshotChunk {
            for domain in decode_snapshot_chunk(envelope.payload())?.1 {
                if domain.domain_id == 17 {
                    let map = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(
                        domain.payload,
                    )
                    .map_err(debug)?;
                    handle =
                        map.tiles
                            .iter()
                            .flat_map(|tile| &tile.items)
                            .find_map(|entry| {
                                if entry.definition
                                    == oteryn_protocol_oteryn::world_map::MapDefinition::Item(
                                        definition,
                                    )
                                    && let oteryn_protocol_oteryn::world_map::MapOrigin::Handle(
                                        handle,
                                    ) = entry.origin
                                {
                                    Some(handle)
                                } else {
                                    None
                                }
                            });
                }
            }
        }
    }
    let payload = encode_container_view_intent(&ContainerViewIntent::Open {
        handle: handle.ok_or("advertised Ground handle")?,
        replace_view: None,
    })
    .map_err(debug)?;
    let frame = oteryn_protocol_oteryn::encode_client_command(
        1,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id: 1,
            command_type: COMMAND_TYPE_CONTAINER_VIEW_INTENT,
            payload: &payload,
        },
    )?;
    let mut input = u32::try_from(frame.len())?.to_be_bytes().to_vec();
    input.extend(frame);
    let (end, output) =
        super::super::weather_wire::fixture_access::serve_input(owner, admitted, &input).await?;
    let mut opened = false;
    let mut shown = false;
    for frame in frames(&output)? {
        let envelope = decode_wire_envelope(frame)?;
        match envelope.message_type() {
            MessageType::CommandResult => {
                let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                assert_eq!(
                    result.status,
                    oteryn_protocol_oteryn::CommandStatus::Accepted
                );
                assert_eq!(
                    decode_container_view_result(result.payload).map_err(debug)?,
                    ContainerViewOutcome::Opened
                );
                opened = true;
            }
            MessageType::StateDelta => {
                let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())?;
                if delta.domain_id == 14 {
                    let views = decode_container_views(delta.payload).map_err(debug)?;
                    assert_eq!(views.views.len(), 1);
                    assert_eq!(views.views[0].container_handle, handle.ok_or("handle")?);
                    assert!(views.views[0].entries.is_empty());
                    shown = true;
                }
            }
            _ => {}
        }
    }
    assert!(
        opened && shown,
        "real dispatch must open and advertise the Ground view: opened={opened}, shown={shown}, {end}"
    );
    {
        let mut runtime = owner.runtime.lock().await;
        let mut movement = runtime.borrow_movement_position();
        for x in [6, 5] {
            let snapshot = movement.read(actor).map_err(debug)?;
            movement
                .commit_cardinal(snapshot, MovementLocalPosition { x, y: 0, floor: 7 })
                .map_err(debug)?;
        }
    }
    assert_eq!(
        owner.observe_container(actor, Some(fence), key).await,
        Some(ContainerObservation::TooFar)
    );
    {
        let mut runtime = owner.runtime.lock().await;
        let mut movement = runtime.borrow_movement_position();
        for x in [6, 7] {
            let snapshot = movement.read(actor).map_err(debug)?;
            movement
                .commit_cardinal(snapshot, MovementLocalPosition { x, y: 0, floor: 7 })
                .map_err(debug)?;
        }
    }
    assert_eq!(
        owner
            .move_loop_item(
                actor,
                UseCommand {
                    command_id: 802,
                    ..command
                },
                LoopMove {
                    item: key,
                    count: None,
                    target: LoopTarget::MainBackpack
                }
            )
            .await,
        LoopMoveResult::Committed
    );
    // The old Ground observation cannot survive a real custody change.
    assert!(
        owner
            .root
            .read_item_loop_ground_container(owner.character, fence, item, ground)
            .await?
            .is_none()
    );
    assert!(matches!(
        owner.observe_container(actor, Some(fence), key).await,
        Some(ContainerObservation::Container {
            parent: Some(_),
            ..
        })
    ));
    Ok(())
}

fn frames(mut bytes: &[u8]) -> TestResult<Vec<&[u8]>> {
    let mut result = Vec::new();
    while !bytes.is_empty() {
        let length = u32::from_be_bytes(bytes.get(..4).ok_or("frame length")?.try_into()?) as usize;
        result.push(bytes.get(4..4 + length).ok_or("frame body")?);
        bytes = &bytes[4 + length..];
    }
    Ok(result)
}
