use tokio::io::AsyncWriteExt;
use super::{TestResult, item_look_fixtures as item_look};

use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted},
    item_view::SessionItemView,
};
use crate::durability::DurabilityRoot;

use oteryn_protocol_oteryn::{
    self as protocol,
    inspection::{self as wire, Request},
    world_spatial_entities::EntityRef,
};

use std::{future::Future, task::Poll};

async fn inspect_backpack_when_holder_busy(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("inspection actor")?;
    let inventory = owner
        .observe_character_inventory(actor, admitted.game_session_id)
        .await
        .ok_or("inspection inventory")?;
    let mut view = SessionItemView::resume(admitted.continuity.item_view);
    let [items, _] = view.snapshot(inventory, None).map_err(super::debug)?;
    let handle = protocol::item_view::decode_character_inventory(&items.payload)
        .map_err(super::debug)?
        .main_backpack
        .ok_or("inspection backpack")?
        .handle;
    let held = owner.root.try_issue_background_semantic_pass()?;
    let mut observation = std::pin::pin!(DurabilityRoot::with_holder_owner(owner.inspection_command(
        admitted, admitted.continuity.next_command_id, Request::Item(handle), Some(&view),
    )));
    let first = std::future::poll_fn(|cx| Poll::Ready(observation.as_mut().poll(cx))).await;
    held.run(|_, _| Box::pin(async { Ok(()) })).await?;
    assert!(first.is_pending(), "backpack inspection must wait for the holder: {first:?}");
    let reply = observation.await;
    assert_eq!(reply.status, 1, "{reply:?}");
    assert_eq!(reply.items.len(), 1);
    assert!(reply.items[0].attributes.iter().any(|(label, _)| label == "Total Weight"));
    Ok(())
}

pub(super) async fn serve_inspections(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    own: EntityRef,
    enabled: bool,
) -> TestResult {
    if enabled {
        inspect_backpack_when_holder_busy(owner, admitted).await?;
    }
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serve = async {
        serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
    };
    let inspect = async {
        let mut bag = None;
        let mut ground = None;
        let mut catalogue = None;
        loop {
            let frame = item_look::read_frame(&mut client).await?;
            let envelope = protocol::decode_wire_envelope(&frame).map_err(super::debug)?;
            if envelope.message_type() == protocol::MessageType::SnapshotChunk {
                let (_, domains) =
                    protocol::decode_snapshot_chunk(envelope.payload()).map_err(super::debug)?;
                for domain in domains {
                    if domain.domain_id == 9 {
                        bag = protocol::item_view::decode_character_inventory(domain.payload)
                            .map_err(super::debug)?
                            .main_backpack
                            .map(|i| {
                                catalogue = Some(i.item_definition_ref.get());
                                i.handle
                            });
                    } else if domain.domain_id == 17 {
                        let map = protocol::world_map::decode_world_map_snapshot(domain.payload)
                            .map_err(super::debug)?;
                        ground =
                            map.tiles
                                .iter()
                                .flat_map(|t| &t.items)
                                .find_map(|i| match i.origin {
                                    protocol::world_map::MapOrigin::Handle(h) => Some(h),
                                    _ => None,
                                });
                    }
                }
            }
            if envelope.message_type() == protocol::MessageType::SnapshotCommit {
                break;
            }
        }
        let requests = [
            Request::Item(bag.ok_or("backpack handle")?),
            Request::Item(ground.ok_or("ground handle")?),
            Request::Catalogue(catalogue.ok_or("catalogue reference")?),
            Request::Player(EntityRef {
                identity: [99; 16],
                generation: 1,
            }),
            Request::Player(own),
            Request::Trade {
                counter: true,
                index: 0,
            },
        ];
        for (index, request) in requests.into_iter().enumerate() {
            let bytes = protocol::encode_client_command(
                admitted.continuity.connection_generation,
                &protocol::ClientCommandValue {
                    command_id: (index + 1) as u64,
                    command_type: wire::COMMAND_TYPE_INSPECTION,
                    payload: &wire::encode_request(request).map_err(super::debug)?,
                },
            )
            .map_err(super::debug)?;
            client.write_u32(u32::try_from(bytes.len())?).await?;
            client.write_all(&bytes).await?;
            loop {
                let frame = item_look::read_frame(&mut client).await?;
                let envelope = protocol::decode_wire_envelope(&frame).map_err(super::debug)?;
                if envelope.message_type() != protocol::MessageType::CommandResult {
                    continue;
                }
                let result =
                    protocol::decode_command_result(envelope.payload()).map_err(super::debug)?;
                if !enabled {
                    assert_eq!(result.status, protocol::CommandStatus::Rejected);
                    assert!(result.payload.is_empty());
                    break;
                }
                assert_eq!(result.status, protocol::CommandStatus::Accepted);
                let reply = wire::decode_reply(result.payload).map_err(super::debug)?;
                match index {
                    0..=2 => {
                        assert_eq!(reply.status, 1, "{request:?}: {reply:?}");
                        assert!(!reply.items[0].name.is_empty());
                        assert!(
                            reply.items[0]
                                .attributes
                                .iter()
                                .any(|(label, _)| label == "Weight" || label == "Total Weight")
                        );
                    }
                    3 => assert_eq!(reply.status, 3),
                    4 => {
                        assert_eq!(reply.status, 1);
                        assert_eq!(reply.items.len(), 1);
                        assert_eq!(reply.items[0].slot, 10);
                    }
                    5 => assert_eq!(reply.status, 2),
                    _ => unreachable!(),
                }
                break;
            }
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let mut serve = std::pin::pin!(serve);
        let mut inspect = std::pin::pin!(inspect);
        let mut served = false;
        let mut result = None;
        std::future::poll_fn(|cx| {
            if !served && serve.as_mut().poll(cx).is_ready() {
                served = true;
            }
            if result.is_none()
                && let Poll::Ready(value) = inspect.as_mut().poll(cx)
            {
                result = Some(value);
            }
            if result.is_some() {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
        .await;
        result.ok_or("inspection client did not finish")?
    })
    .await??;
    Ok(())
}
