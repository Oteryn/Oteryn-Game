//! Actual map handle -> OPEN21 -> MOVE9 -> OPEN21 on one live House connection.
use super::*;
use oteryn_protocol_oteryn::container_tree::{
    ContainerViewIntent, ContainerViewOutcome, ContainerViews, STATE_DOMAIN_CONTAINER_VIEWS,
    decode_container_view_result, decode_container_views, encode_container_view_intent,
};
use oteryn_protocol_oteryn::item_view::{
    ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome, ItemMoveSelection,
    STATE_DOMAIN_CHARACTER_INVENTORY, decode_item_move_result_for, encode_item_move_intent_for,
};

pub(super) async fn recover(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    reference: std::num::NonZeroU32,
) -> TestResult<AdmittedSession> {
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let peer = async move {
        let mut handle = None;
        loop {
            let raw = super::super::super::item_look::read_frame(&mut client).await?;
            let env = decode_wire_envelope(&raw).map_err(debug)?;
            if env.message_type() == MessageType::SnapshotChunk {
                for domain in oteryn_protocol_oteryn::decode_snapshot_chunk(env.payload())
                    .map_err(debug)?
                    .1
                {
                    if domain.domain_id
                        == oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW
                    {
                        let map = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(
                            domain.payload,
                        )
                        .map_err(debug)?;
                        handle = map
                            .tiles
                            .iter()
                            .flat_map(|tile| tile.items.iter())
                            .find_map(|item| match (item.definition, item.origin) {
                                (MapDefinition::Item(def), MapOrigin::Handle(handle))
                                    if def == reference =>
                                {
                                    Some(handle)
                                }
                                _ => None,
                            });
                    }
                }
            }
            if env.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        let handle = handle
            .ok_or("actual House map must expose receipt-selected death backpack as pickupable")?;
        let open = encode_container_view_intent(&ContainerViewIntent::Open {
            handle,
            replace_view: None,
        })
        .map_err(debug)?;
        send(&mut client, admitted, 1, 21, &open).await?;
        let before = container_reply(&mut client, 1, handle).await?;
        assert!(
            !before
                .views
                .iter()
                .find(|view| view.container_handle == handle)
                .ok_or("Ground backpack view")?
                .entries
                .is_empty(),
            "actual dropped backpack contents must be reachable before pickup"
        );
        let selection = ItemMoveSelection {
            equip_drop: true,
            container_tree: true,
            world_interaction: true,
        };
        let body = encode_item_move_intent_for(
            &ItemMoveIntent {
                source: handle,
                destination: ItemMoveDestination::MainBackpack,
            },
            selection,
        )
        .map_err(debug)?;
        send(&mut client, admitted, 2, 9, &body).await?;
        let mut moved = false;
        let mut inventory = false;
        while !moved || !inventory {
            let raw = super::super::super::item_look::read_frame(&mut client).await?;
            let env = decode_wire_envelope(&raw).map_err(debug)?;
            if env.message_type() == MessageType::CommandResult {
                let reply = decode_command_result(env.payload()).map_err(debug)?;
                assert_eq!(reply.command_id, 2);
                assert_eq!(reply.status, CommandStatus::Accepted);
                assert_eq!(
                    decode_item_move_result_for(reply.payload, selection).map_err(debug)?,
                    ItemMoveOutcome::Moved
                );
                moved = true;
            }
            if env.message_type() == MessageType::StateDelta {
                let delta =
                    oteryn_protocol_oteryn::decode_state_delta(env.payload()).map_err(debug)?;
                if delta.domain_id == STATE_DOMAIN_CHARACTER_INVENTORY {
                    let view=oteryn_protocol_oteryn::item_view::decode_character_inventory_with_equip_drop(delta.payload,true).map_err(debug)?;
                    inventory = view.entries.iter().any(|entry| {
                        entry.handle == handle && entry.item_definition_ref == reference
                    });
                }
            }
        }
        send(&mut client, admitted, 3, 21, &open).await?;
        let after = container_reply(&mut client, 3, handle).await?;
        assert_eq!(
            before
                .views
                .iter()
                .find(|view| view.container_handle == handle)
                .map(|view| &view.entries),
            after
                .views
                .iter()
                .find(|view| view.container_handle == handle)
                .map(|view| &view.entries),
            "same original child handles, definitions and counts survive actual House pickup"
        );
        // Keep the living guest's real transport until its mature HouseExit commits.
        let mut exit = vec![8, 2, 18, u8::try_from(HOUSE.len())?];
        exit.extend(HOUSE.as_bytes());
        send(&mut client, admitted, 4, 102, &exit).await?;
        let mut exited = false;
        loop {
            let raw = super::super::super::item_look::read_frame(&mut client).await?;
            let env = decode_wire_envelope(&raw).map_err(debug)?;
            if env.message_type() == MessageType::CommandResult {
                let result = decode_command_result(env.payload()).map_err(debug)?;
                assert_eq!(result.command_id, 4);
                assert_eq!(
                    result.status,
                    CommandStatus::Accepted,
                    "living authorized guest exits normally after recovery"
                );
                exited = true;
            }
            if exited && env.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        client.shutdown().await?;
        let mut tail = Vec::new();
        client.read_to_end(&mut tail).await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let server_task = async move {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    use std::future::Future;
    use std::task::Poll;
    let mut serving = std::pin::pin!(server_task);
    let mut client_task = std::pin::pin!(peer);
    let (mut end, mut result) = (None, None);
    tokio::time::timeout(
        std::time::Duration::from_secs(15),
        std::future::poll_fn(|cx| {
            if end.is_none()
                && let Poll::Ready(value) = serving.as_mut().poll(cx)
            {
                end = Some(value);
            }
            if result.is_none()
                && let Poll::Ready(value) = client_task.as_mut().poll(cx)
            {
                result = Some(value);
            }
            if end.is_some() && result.is_some() {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }),
    )
    .await?;
    let end = end.ok_or("House recovery server completion")?;
    let result = result.ok_or("House recovery client completion")?;
    result?;
    match end {
        ConnectionEnd::AdmittedThenDisconnected(current)
        | ConnectionEnd::AdmittedThenClosed(current, _) => Ok(current),
        other => Err(format!("House death recovery wire ended {other:?}").into()),
    }
}
async fn send(
    client: &mut tokio::io::DuplexStream,
    admitted: AdmittedSession,
    id: u64,
    kind: u32,
    payload: &[u8],
) -> TestResult {
    let frame = encode_client_command(
        admitted.continuity.connection_generation,
        &ClientCommandValue {
            command_id: id,
            command_type: kind,
            payload,
        },
    )
    .map_err(debug)?;
    client.write_u32(u32::try_from(frame.len())?).await?;
    client.write_all(&frame).await?;
    Ok(())
}
async fn container_reply(
    client: &mut tokio::io::DuplexStream,
    id: u64,
    handle: std::num::NonZeroU64,
) -> TestResult<ContainerViews> {
    let (mut accepted, mut views) = (false, None);
    while !accepted || views.is_none() {
        let raw = super::super::super::item_look::read_frame(client).await?;
        let env = decode_wire_envelope(&raw).map_err(debug)?;
        if env.message_type() == MessageType::CommandResult {
            let reply = decode_command_result(env.payload()).map_err(debug)?;
            assert_eq!(reply.command_id, id);
            assert_eq!(reply.status, CommandStatus::Accepted);
            assert_eq!(
                decode_container_view_result(reply.payload).map_err(debug)?,
                ContainerViewOutcome::Opened
            );
            accepted = true;
        }
        if env.message_type() == MessageType::StateDelta {
            let delta = oteryn_protocol_oteryn::decode_state_delta(env.payload()).map_err(debug)?;
            if delta.domain_id == STATE_DOMAIN_CONTAINER_VIEWS {
                let current = decode_container_views(delta.payload).map_err(debug)?;
                if current
                    .views
                    .iter()
                    .any(|view| view.container_handle == handle)
                {
                    views = Some(current);
                }
            }
        }
    }
    views.ok_or_else(|| "actual opened container delta".into())
}
