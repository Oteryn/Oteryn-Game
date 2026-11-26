use super::*;
use crate::foundation::{ChannelId, GameSessionId, WorldId};
use crate::gameplay_transport::AdmissionRefusal;
use crate::gameplay_transport::capabilities::{
    PRODUCTION_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAttempt, IDLE_LIVENESS, SessionContinuity, serve_admitted,
};
use crate::gameplay_transport::world_spatial::{ActorPosition, WorldSpatialObservation};
use oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_VIEW_MOVE_V1;
use oteryn_protocol_oteryn::refusal_detail::{
    CAPABILITY_REFUSAL_DETAIL_V1, RefusalDetail, split_refusal_detail,
};
use oteryn_protocol_oteryn::world_interaction::CAPABILITY_WORLD_INTERACTION_V1;
use oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1;
use oteryn_protocol_oteryn::world_object::{
    COMMAND_TYPE_USE_INTENT, MapUseTarget, UseDisposition, WorldObjectTarget, decode_use_result,
    encode_item_use_intent,
};
use oteryn_protocol_oteryn::world_spatial_entities::CAPABILITY_WORLD_SPATIAL_ENTITIES;
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, MessageType, decode_command_result, decode_wire_envelope,
    encode_client_command,
};
use std::cell::Cell;
use std::num::NonZeroU64;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Failure {
    Cube,
    TradeRead,
    MapToolProbe,
    CubeWriteUnknown,
}

struct Authority {
    failure: Failure,
    hits: Cell<u32>,
}

impl FreshAdmissionAuthority for Authority {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(WorldSpatialObservation {
            content_generation: [0x5c; 32],
            actor_position: ActorPosition {
                x: 0,
                y: 0,
                floor: 0,
            },
        })
    }

    async fn player_trade_item_available(
        &self,
        _: ExactActorRef,
        command: UseCommand,
        _: Option<ItemKey>,
    ) -> Option<bool> {
        if self.failure == Failure::TradeRead && command.command_id == 1 {
            self.hits.set(self.hits.get() + 1);
            return None;
        }
        Some(true)
    }

    async fn use_drome_cube(
        &self,
        _: ExactActorRef,
        command: UseCommand,
        _: UseIntent,
        _: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        if matches!(self.failure, Failure::Cube | Failure::CubeWriteUnknown)
            && command.command_id == 1
        {
            self.hits.set(self.hits.get() + 1);
            return Err(());
        }
        Ok(None)
    }

    async fn tame_mount(
        &self,
        _: ExactActorRef,
        command: UseCommand,
        intent: UseIntent,
        _: Option<ItemKey>,
        _: Option<ItemKey>,
        _: bool,
    ) -> Result<Option<UseOutcome>, ()> {
        if self.failure == Failure::MapToolProbe && command.command_id == 1 {
            assert!(matches!(
                intent.use_with_map_item,
                Some(MapUseTarget::Object(_))
            ));
            self.hits.set(self.hits.get() + 1);
            return Err(());
        }
        Ok(None)
    }

    async fn drome_use_unresolved(&self, _: ExactActorRef, command: UseCommand) -> bool {
        self.failure == Failure::CubeWriteUnknown && command.command_id == 1
    }
}

fn id(tag: u8) -> [u8; 16] {
    let mut id = [tag; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id
}

fn rejects_and_serves_next(
    failure: Failure,
    target: Option<WorldObjectTarget>,
) -> Result<(), String> {
    let run = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?
                .block_on(async {
                    let authority = Authority {
                        failure,
                        hits: Cell::new(0),
                    };
                    let world_id = WorldId::decode(&id(1)).map_err(|error| format!("{error:?}"))?;
                    let channel_id =
                        ChannelId::decode(&id(2)).map_err(|error| format!("{error:?}"))?;
                    let admitted = AdmittedSession {
                        game_session_id: GameSessionId::decode(&id(3))
                            .map_err(|error| format!("{error:?}"))?,
                        world_id,
                        channel_id,
                        runtime_actor: Some(ExactActorRef::transport_fixture(world_id, channel_id)),
                        first_entry: FirstEntryOutcome::Positioned,
                        controller: None,
                        continuity: SessionContinuity {
                            selected_capabilities: SelectedCapabilities::select(
                                PRODUCTION_OFFERED_CAPABILITIES,
                                &[
                                    CAPABILITY_ITEM_VIEW_MOVE_V1,
                                    CAPABILITY_ITEM_USE_V1,
                                    CAPABILITY_WORLD_INTERACTION_V1,
                                    CAPABILITY_WORLD_MAP_VIEW_V1,
                                    CAPABILITY_WORLD_SPATIAL_ENTITIES,
                                    CAPABILITY_REFUSAL_DETAIL_V1,
                                ],
                            )
                            .ok_or("capability selection")?,
                            ..SessionContinuity::FRESH
                        },
                        item_fence: None,
                    };
                    let payload = encode_item_use_intent(&UseIntent {
                        target: UseTarget::Item(NonZeroU64::MIN),
                        use_with: None,
                        use_with_map_item: target.map(MapUseTarget::Object),
                        use_with_position: None,
                    })
                    .map_err(|error| format!("{error:?}"))?;
                    let (mut server, mut client) = tokio::io::duplex(1 << 20);
                    for command_id in [1, 2] {
                        let frame = encode_client_command(
                            1,
                            &ClientCommandValue {
                                command_id,
                                command_type: COMMAND_TYPE_USE_INTENT,
                                payload: if command_id == 1 { &payload } else { &[] },
                            },
                        )
                        .map_err(|error| error.to_string())?;
                        client
                            .write_u32(
                                u32::try_from(frame.len()).map_err(|error| error.to_string())?,
                            )
                            .await
                            .map_err(|error| error.to_string())?;
                        client
                            .write_all(&frame)
                            .await
                            .map_err(|error| error.to_string())?;
                    }
                    client.shutdown().await.map_err(|error| error.to_string())?;
                    serve_admitted(&mut server, admitted, &authority, IDLE_LIVENESS).await;
                    drop(server);
                    assert_eq!(authority.hits.get(), 1, "fault injection must be reached");
                    let mut results = Vec::new();
                    while let Ok(len) = client.read_u32().await {
                        let mut frame = vec![0; len as usize];
                        client
                            .read_exact(&mut frame)
                            .await
                            .map_err(|error| error.to_string())?;
                        let envelope =
                            decode_wire_envelope(&frame).map_err(|error| error.to_string())?;
                        if envelope.message_type() == MessageType::CommandResult {
                            let reply = decode_command_result(envelope.payload())
                                .map_err(|error| error.to_string())?;
                            let (body, detail) = split_refusal_detail(reply.payload)
                                .map_err(|error| format!("{error:?}"))?;
                            results.push((
                                reply.command_id,
                                reply.status,
                                decode_use_result(body),
                                detail,
                            ));
                        }
                    }
                    if failure == Failure::CubeWriteUnknown {
                        assert!(
                            results.is_empty(),
                            "a sent write without an ACK must stay pending"
                        );
                    } else {
                        assert_eq!(
                            results,
                            [1, 2].map(|id| (
                                id,
                                CommandStatus::Rejected,
                                Ok(UseDisposition::Rejected),
                                Some(RefusalDetail::NotPossible),
                            ))
                        );
                    }
                    Ok(())
                })
        })
        .map_err(|error| error.to_string())?
        .join();
    match run {
        Ok(result) => result,
        Err(failure) => std::panic::resume_unwind(failure),
    }
}

#[test]
fn cube_authority_failure_keeps_connection_open() -> Result<(), String> {
    rejects_and_serves_next(Failure::Cube, None)
}

#[test]
fn generic_use_authority_failure_keeps_connection_open() -> Result<(), String> {
    rejects_and_serves_next(Failure::TradeRead, None)
}

#[test]
fn dig_use_authority_failure_keeps_connection_open() -> Result<(), String> {
    rejects_and_serves_next(
        Failure::MapToolProbe,
        Some(WorldObjectTarget {
            placement: b"dig-target".to_vec(),
            expected_revision: 0,
        }),
    )
}

#[test]
fn fishing_use_authority_failure_keeps_connection_open() -> Result<(), String> {
    rejects_and_serves_next(
        Failure::MapToolProbe,
        Some(WorldObjectTarget {
            placement: b"fishing-target".to_vec(),
            expected_revision: 0,
        }),
    )
}

#[test]
fn use_rejection_frame_round_trips() -> Result<(), String> {
    rejects_and_serves_next(Failure::TradeRead, None)
}

#[test]
fn cube_unknown_write_closes_without_acknowledging_use() -> Result<(), String> {
    rejects_and_serves_next(Failure::CubeWriteUnknown, None)
}
