//! Real ClientCommand -> mechanic router -> transient owner -> CHAT delta/replay checks.
use super::*;
use crate::foundation::{ChannelId, MessageType, WorldId, decode_wire_envelope};
use crate::gameplay_transport::actor_spell::tests::runtime_with_capacity;
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use crate::gameplay_transport::chat_intent::ChatRuntime;
use crate::gameplay_transport::connection::{
    AdmissionRefusal, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAttempt,
    FreshAdmissionAuthority, IDLE_LIVENESS, SessionContinuity, serve_admitted,
};
use crate::gameplay_transport::session_ingress::SessionCommands;
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use crate::gameplay_transport::world_spatial::{
    ChannelEntities, ChannelEntity, EntityLights, StepDirection, VisibleKind,
    WorldSpatialObservation,
};
use oteryn_protocol_oteryn::chat::ChatRoomSet;
use oteryn_protocol_oteryn::player_typing::{
    COMMAND_TYPE_PLAYER_TYPING, DELTA_TYPE_PLAYER_TYPING_V1, decode_update, encode_intent,
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, decode_state_delta,
    encode_client_command,
};
use oteryn_simulation_determinism::SemanticTimeMicros;
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::future::{Future, poll_fn};
use std::task::Poll;
use std::time::Duration;
use tokio::io::{AsyncWriteExt, DuplexStream};

struct TypingAuthority {
    runtime: RefCell<ChannelRuntimeV1>,
    chat: ChatRuntime,
    commands: SessionCommands,
    calls: Cell<usize>,
}

impl FreshAdmissionAuthority for TypingAuthority {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Unavailable)
    }

    fn session_commands(&self) -> Option<&SessionCommands> {
        Some(&self.commands)
    }

    async fn observe(&self, actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        let runtime = self.runtime.borrow();
        let position = runtime.read_actor_position(actor).ok()?.position();
        Some(WorldSpatialObservation {
            content_generation: [0x5c; 32],
            actor_position: ActorPosition {
                x: position.x,
                y: position.y,
                floor: position.floor,
            },
        })
    }

    async fn observe_visible_entities(&self, actor: ExactActorRef) -> Option<ChannelEntities> {
        let runtime = self.runtime.borrow();
        Some(ChannelEntities {
            content_generation: [0x5c; 32],
            observer: actor.placement_identity(),
            entities: runtime
                .visible_entities()
                .players
                .iter()
                .map(|entry| ChannelEntity {
                    health_percent: 100,
                    direction: StepDirection::South,
                    kind: VisibleKind::Player,
                    identity: entry.actor.placement_identity(),
                    generation: entry.generation,
                    position: ActorPosition {
                        x: entry.position.x,
                        y: entry.position.y,
                        floor: entry.position.floor,
                    },
                    revision: 0,
                })
                .collect(),
            lights: EntityLights::new(),
            observed_at_ms: 0,
        })
    }

    async fn chat_admit(&self, _: ExactActorRef, session: GameSessionId) -> ChatRoomSet {
        self.chat
            .admit(session, "Aela", SemanticTimeMicros::from_micros(1));
        ChatRoomSet::default()
    }

    async fn player_typing_admit(&self, admitted: AdmittedSession) {
        let Some(actor) = admitted.runtime_actor else {
            return;
        };
        self.chat.typing.admit(
            &self.runtime.borrow(),
            actor,
            admitted.game_session_id,
            admitted.continuity.connection_generation,
            admitted
                .continuity
                .selected_capabilities
                .contains(CAPABILITY_PLAYER_TYPING_V1),
        );
    }

    async fn player_typing(&self, admitted: AdmittedSession, typing: bool) -> bool {
        self.calls.set(self.calls.get() + 1);
        let Some(actor) = admitted.runtime_actor else {
            return false;
        };
        self.chat.typing.broadcast(
            &self.runtime.borrow(),
            actor,
            admitted.game_session_id,
            admitted.continuity.connection_generation,
            typing,
        )
    }

    async fn player_typing_next(&self, admitted: AdmittedSession) -> Option<TypingUpdate> {
        self.chat.typing.next(
            admitted.game_session_id,
            admitted.continuity.connection_generation,
        )
    }
}

const fn uuid(tag: u8) -> [u8; 16] {
    [
        0x01, 0x90, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag,
    ]
}

fn fixture(selected: bool) -> Result<(TypingAuthority, AdmittedSession), Box<dyn Error>> {
    let (mut runtime, actor, game_session_id) = runtime_with_capacity(0x71, 2);
    runtime.initialize_first_entry_position(actor)?;
    let offered = [
        OfferedCapability {
            id: 6,
            requires: &[],
        },
        OfferedCapability {
            id: 7,
            requires: &[],
        },
        OfferedCapability {
            id: CAPABILITY_PLAYER_TYPING_V1,
            requires: &[6, 7],
        },
    ];
    let supported: &[u32] = if selected {
        &[6, 7, CAPABILITY_PLAYER_TYPING_V1]
    } else {
        &[6, 7]
    };
    let admitted = AdmittedSession {
        game_session_id,
        world_id: WorldId::decode(&uuid(0x60))?,
        channel_id: ChannelId::decode(&uuid(0x61))?,
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: None,
        continuity: SessionContinuity {
            selected_capabilities: SelectedCapabilities::select(&offered, supported)
                .ok_or("selection")?,
            ..SessionContinuity::FRESH
        },
        item_fence: None,
    };
    Ok((
        TypingAuthority {
            runtime: RefCell::new(runtime),
            chat: ChatRuntime::default(),
            commands: SessionCommands::default(),
            calls: Cell::new(0),
        },
        admitted,
    ))
}

async fn send(
    client: &mut DuplexStream,
    id: u64,
    command: u32,
    payload: &[u8],
) -> Result<(), Box<dyn Error>> {
    let frame = encode_client_command(
        1,
        &ClientCommandValue {
            command_id: id,
            command_type: command,
            payload,
        },
    )?;
    write_frame(client, &frame).await?;
    Ok(())
}

async fn result(client: &mut DuplexStream, id: u64) -> Result<CommandStatus, Box<dyn Error>> {
    loop {
        let frame = read_frame(client).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload())?;
            assert_eq!(result.command_id, id);
            assert!(result.payload.is_empty());
            return Ok(result.status);
        }
    }
}

async fn update(client: &mut DuplexStream) -> Result<TypingUpdate, Box<dyn Error>> {
    loop {
        let frame = read_frame(client).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::StateDelta {
            let delta = decode_state_delta(envelope.payload())?;
            if delta.domain_id == 12 && delta.delta_type == DELTA_TYPE_PLAYER_TYPING_V1 {
                assert_eq!(delta.new_revision, delta.base_revision + 1);
                return decode_update(delta.payload).map_err(|error| format!("{error:?}").into());
            }
        }
    }
}

async fn scenario(selected: bool, flood: bool) -> Result<(), Box<dyn Error>> {
    let (authority, admitted) = fixture(selected)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = serve_admitted(&mut server, admitted, &authority, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let client_authority = &authority;
    let playing = async move {
        loop {
            let frame = read_frame(&mut client).await?;
            if decode_wire_envelope(&frame)?.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        if !selected {
            send(
                &mut client,
                1,
                COMMAND_TYPE_PLAYER_TYPING,
                &encode_intent(true),
            )
            .await?;
            assert_eq!(result(&mut client, 1).await?, CommandStatus::Rejected);
            assert_eq!(client_authority.calls.get(), 0);
        } else if flood {
            // Same existing SEC-1 aggregate budget as every other transient command.
            for id in 1..=100 {
                send(&mut client, id, 0x7fff, &[]).await?;
                assert_eq!(result(&mut client, id).await?, CommandStatus::Rejected);
            }
            let before = tokio::time::Instant::now();
            send(
                &mut client,
                101,
                COMMAND_TYPE_PLAYER_TYPING,
                &encode_intent(true),
            )
            .await?;
            assert_eq!(result(&mut client, 101).await?, CommandStatus::Rejected);
            assert!(before.elapsed() >= Duration::from_secs(1));
            assert_eq!(client_authority.calls.get(), 0);
            send(
                &mut client,
                102,
                COMMAND_TYPE_PLAYER_TYPING,
                &encode_intent(false),
            )
            .await?;
            assert_eq!(result(&mut client, 102).await?, CommandStatus::Accepted);
            assert!(!update(&mut client).await?.typing);
            assert_eq!(client_authority.calls.get(), 1);
        } else {
            send(
                &mut client,
                1,
                COMMAND_TYPE_PLAYER_TYPING,
                &encode_intent(true),
            )
            .await?;
            assert_eq!(result(&mut client, 1).await?, CommandStatus::Accepted);
            assert!(update(&mut client).await?.typing);
            assert_eq!(client_authority.calls.get(), 1);
            // Replacement bytes must not change the already-recorded intent.
            send(
                &mut client,
                1,
                COMMAND_TYPE_PLAYER_TYPING,
                &encode_intent(false),
            )
            .await?;
            assert_eq!(
                result(&mut client, 1).await?,
                CommandStatus::DuplicateReplay
            );
            assert_eq!(client_authority.calls.get(), 1);
            assert_eq!(
                client_authority
                    .chat
                    .typing
                    .next(admitted.game_session_id, 1),
                None
            );
            send(&mut client, 2, COMMAND_TYPE_PLAYER_TYPING, &[8, 2]).await?;
            assert_eq!(result(&mut client, 2).await?, CommandStatus::Rejected);
            assert_eq!(client_authority.calls.get(), 1);
            send(
                &mut client,
                3,
                COMMAND_TYPE_PLAYER_TYPING,
                &encode_intent(false),
            )
            .await?;
            assert_eq!(result(&mut client, 3).await?, CommandStatus::Accepted);
            assert!(!update(&mut client).await?.typing);
            assert_eq!(client_authority.calls.get(), 2);
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn Error>>(())
    };
    let (mut serving, mut playing) = (std::pin::pin!(serving), std::pin::pin!(playing));
    let (mut ended, mut played) = (None, None);
    tokio::time::timeout(
        Duration::from_secs(10),
        poll_fn(|context| {
            if ended.is_none()
                && let Poll::Ready(end) = serving.as_mut().poll(context)
            {
                ended = Some(end);
            }
            if played.is_none()
                && let Poll::Ready(result) = playing.as_mut().poll(context)
            {
                played = Some(result);
            }
            if ended.is_some() && played.is_some() {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }),
    )
    .await?;
    played.ok_or("client did not finish")??;
    assert!(matches!(
        ended.ok_or("server did not finish")?,
        ConnectionEnd::AdmittedThenDisconnected(_)
    ));
    Ok(())
}

fn run_case(selected: bool, flood: bool) -> Result<(), String> {
    // The real connection future must run on the repository's qualified large stack.
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .start_paused(true)
                .build()
                .map_err(|error| error.to_string())?;
            runtime
                .block_on(scenario(selected, flood))
                .map_err(|error| error.to_string())
        })
        .map_err(|error| error.to_string())?
        .join()
        .map_err(|_| "typing connection test thread failed".to_owned())?
}

#[test]
fn player_typing_real_connection_broadcasts_on_off_and_replays_without_reexecution()
-> Result<(), String> {
    run_case(true, false)
}

#[test]
fn player_typing_real_connection_requires_negotiated_capability() -> Result<(), String> {
    run_case(false, false)
}

#[test]
fn player_typing_real_connection_reuses_transient_command_rate_limit() -> Result<(), String> {
    run_case(true, true)
}
