//! INSPECT-1 through PostgreSQL admission, native map snapshots and real dispatch.
use super::{TestResult, item_look};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, IDLE_LIVENESS, serve_admitted},
};
use oteryn_protocol_oteryn::{
    self as protocol,
    inspection::{self as wire, Request},
    world_spatial_entities::EntityRef,
};
use std::{future::Future, task::Poll};
use tokio::io::AsyncWriteExt;

#[test]
fn inspection_postgres_inventory_ground_catalogue_and_private_player_dispatch() -> TestResult {
    item_look::large_stack(|| item_look::scenario_with_inspection(Some(true)))
}
#[test]
fn inspection_postgres_unselected_capability_rejects_command() -> TestResult {
    item_look::large_stack(|| item_look::scenario_with_inspection(Some(false)))
}
#[test]
fn inspection_postgres_trade_counterpart_requires_current_offer_and_leaf_index() -> TestResult {
    super::player_trade::run_scenario(true)
}
pub(super) async fn serve_inspections(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    own: EntityRef,
    enabled: bool,
) -> TestResult {
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
