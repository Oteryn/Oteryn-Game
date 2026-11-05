//! C15: admitted session -> selected NPC talk -> live replies, keyword/free text and closure.
use oteryn_client::play::{PlayLink, PlayView, play_channel, run_session};
use oteryn_protocol_oteryn::{
    ChannelId, CharacterId, CommandStatus, DomainSnapshot, GameSessionId, ServerAcceptedValue,
    WorldId, decode_wire_envelope, encode_command_result, encode_server_accepted,
    encode_single_chunk_snapshot, encode_state_delta,
    npc_service::{self, NpcConversation, NpcConversationState, NpcIntentDisposition},
    world_spatial::{ActorPosition, StepDirection},
    world_spatial_entities::{
        EntityDetail, EntityKind, EntityRef, WorldSpatialEntitiesDelta,
        WorldSpatialEntitiesSnapshot, WorldSpatialEntity, encode_world_spatial_entities_delta,
        encode_world_spatial_entities_snapshot,
    },
};
use oteryn_renderer::TileCoord;
use oteryn_session::{Admission, CLIENT_SUPPORTED_CAPABILITIES, Session, read_frame, write_frame};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};
use tokio::sync::oneshot;

type Error = Box<dyn std::error::Error + Send + Sync>;
const WAIT: Duration = Duration::from_secs(5);

#[test]
fn native_npc_dialogue_scenario() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(scenario())
        })?
        .join()
        .map_err(|_| std::io::Error::other("dialogue scenario thread panicked"))?
}

async fn scenario() -> Result<(), Error> {
    let (client, server) = tokio::io::duplex(64 * 1024);
    let (first, first_ready) = oneshot::channel();
    let (second, second_ready) = oneshot::channel();
    let peer = tokio::spawn(peer(server, first_ready, second_ready));
    let session = Session::admit(
        client,
        Admission {
            appearance_items: None,
            schema_revision: 1,
            character_id: CharacterId::decode(&identity(4))?,
            admission_material: b"grant",
            client_build_id: "c15-native-scenario",
            supported_capabilities: CLIENT_SUPPORTED_CAPABILITIES,
            deadline: WAIT,
        },
    )
    .await?;
    let mut view = PlayView::from_join(session.join_snapshot())?;
    let (link, commands, outbox) = play_channel();
    let task = tokio::spawn(run_session(session, commands, outbox));
    let initial_sprites = view.scene().sprites().len();
    pump(&mut view, &link, |view, link| {
        link.npc_dialogue_panel().selected && view.scene().sprites().len() > initial_sprites
    })
    .await?;
    view.click(TileCoord::new(101, 200))?;
    assert_eq!(view.selected_npc(), Some(npc().entity));
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    assert!(link.npc_dialogue_input(
        &[NormalizedInputEvent::Key {
            code: KeyCode::new(27)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(Modifier::Control).with(Modifier::Shift),
            repeat: false,
        }],
        &view
    ));
    pump(&mut view, &link, |_, link| {
        link.npc_dialogue_panel().text().contains("{job}")
    })
    .await?;
    let panel = link.npc_dialogue_panel();
    assert_eq!(panel.keywords(), vec!["job"]);
    link.talk_to_npc(npc().entity, panel.keywords()[0]);
    pump(&mut view, &link, |_, link| {
        link.npc_dialogue_panel().text().contains("fisherman")
    })
    .await?;
    link.talk_to_npc(npc().entity, "tell me more");
    pump(&mut view, &link, |_, link| {
        link.npc_dialogue_panel().text().contains("Good fishing")
    })
    .await?;
    first.send(()).map_err(|_| "peer ended before walk-away")?;
    pump(&mut view, &link, |_, link| !link.npc_dialogue_panel().open).await?;
    let final_reply = link.npc_dialogue_panel();
    assert!(final_reply.target.is_none());
    assert!(final_reply.visible());
    assert!(final_reply.text().contains("Farewell"));
    second
        .send(())
        .map_err(|_| "peer ended before session end")?;
    tokio::time::timeout(WAIT, async {
        while link.npc_dialogue_panel().selected {
            tokio::task::yield_now().await;
        }
    })
    .await?;
    assert!(!link.npc_dialogue_panel().open);
    drop(link);
    tokio::time::timeout(WAIT, task).await??;
    tokio::time::timeout(WAIT, peer).await???;
    Ok(())
}

async fn pump(
    view: &mut PlayView,
    link: &PlayLink,
    ready: impl Fn(&PlayView, &PlayLink) -> bool,
) -> Result<(), Error> {
    tokio::time::timeout(WAIT, async {
        loop {
            view.tick(link).map_err(|e| format!("tick: {e:?}"))?;
            if ready(view, link) {
                return Ok::<_, Error>(());
            }
            tokio::task::yield_now().await;
        }
    })
    .await?
}

async fn peer(
    mut stream: DuplexStream,
    first: oneshot::Receiver<()>,
    second: oneshot::Receiver<()>,
) -> Result<(), Error> {
    let bootstrap = read_frame(&mut stream).await?;
    assert!(
        decode_wire_envelope(&bootstrap)?
            .client_bootstrap()?
            .supported_capabilities
            .contains(&3)
    );
    write_frame(
        &mut stream,
        &encode_server_accepted(&ServerAcceptedValue {
            game_session_id: GameSessionId::decode(&identity(1))?,
            world_id: WorldId::decode(&identity(2))?,
            channel_id: ChannelId::decode(&identity(3))?,
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 7,
            schema_revision: 1,
            selected_capabilities: &[3, 6, 13],
        })?,
    )
    .await?;
    let spatial = encode_world_spatial_entities_snapshot(&WorldSpatialEntitiesSnapshot {
        content_generation: [1; 32],
        actor_position: position(100),
        own_identity: [7; 16],
        entities: vec![actor(100), npc()],
    })
    .map_err(|e| format!("snapshot: {e:?}"))?;
    for frame in encode_single_chunk_snapshot(
        1,
        1,
        40,
        &[
            DomainSnapshot {
                domain_id: 1,
                revision: 5,
                snapshot_type: 2,
                payload: &spatial,
            },
            DomainSnapshot {
                domain_id: 7,
                revision: 0,
                snapshot_type: 1,
                payload: &[],
            },
            DomainSnapshot {
                domain_id: 8,
                revision: 0,
                snapshot_type: 1,
                payload: &[],
            },
            DomainSnapshot {
                domain_id: 2,
                revision: 2,
                snapshot_type: 1,
                payload: &[],
            },
        ],
    )? {
        write_frame(&mut stream, &frame).await?;
    }
    for (index, (text, reply)) in [
        ("hi", "Ask about my {job}."),
        ("job", "I am a fisherman."),
        ("tell me more", "Good fishing!"),
    ]
    .into_iter()
    .enumerate()
    {
        let bytes = read_frame(&mut stream).await?;
        let command = decode_wire_envelope(&bytes)?.client_command(1)?;
        assert_eq!(command.command_type, 7);
        let intent = npc_service::decode_npc_talk_intent(command.payload)
            .map_err(|e| format!("intent: {e:?}"))?;
        assert_eq!(intent.npc_actor, npc().entity);
        assert_eq!(intent.text, text);
        let sequence = 41 + index as u64 * 2;
        write_frame(
            &mut stream,
            &encode_command_result(
                1,
                sequence,
                command.command_id,
                CommandStatus::Accepted,
                &npc_service::encode_npc_intent_result(NpcIntentDisposition::Ok),
            )?,
        )
        .await?;
        let conversation = NpcConversation {
            open: Some((npc().entity, NpcConversationState::Open)),
            lines: vec![reply.into()],
        };
        conversation_push(&mut stream, sequence + 1, index as u64, &conversation).await?;
    }
    first.await?;
    push(&mut stream, 47, 5, 110).await?;
    conversation_push(
        &mut stream,
        48,
        3,
        &NpcConversation {
            open: None,
            lines: vec!["Farewell".into()],
        },
    )
    .await?;
    second.await?;
    // End an open panel as well as closing it on a server walk-away event.
    conversation_push(
        &mut stream,
        49,
        4,
        &NpcConversation {
            open: Some((npc().entity, NpcConversationState::Open)),
            lines: vec!["Goodbye".into()],
        },
    )
    .await?;
    stream.shutdown().await?;
    // No trade commands are sent by dialogue controls.
    let mut extra = [0];
    assert_eq!(stream.read(&mut extra).await?, 0);
    Ok(())
}

async fn push(stream: &mut DuplexStream, sequence: u64, base: u64, x: i32) -> Result<(), Error> {
    let payload = encode_world_spatial_entities_delta(&WorldSpatialEntitiesDelta {
        content_generation: [1; 32],
        actor_position: position(x),
        enter: vec![],
        update: vec![actor(x)],
        leave: vec![],
    })
    .map_err(|e| format!("delta: {e:?}"))?;
    write_frame(
        stream,
        &encode_state_delta(1, sequence, 1, base, base + 1, 2, &payload)?,
    )
    .await?;
    Ok(())
}
fn position(x: i32) -> ActorPosition {
    ActorPosition {
        x,
        y: 200,
        floor: -2,
    }
}
fn actor(x: i32) -> WorldSpatialEntity {
    WorldSpatialEntity {
        display_name: None,
        kind: EntityKind::Player,
        entity: EntityRef {
            identity: [7; 16],
            generation: 1,
        },
        position: position(x),
        detail: EntityDetail::Actor {
            direction: StepDirection::East,
            appearance_ref: 1,
            health_percent: 100,
        },
    }
}
fn identity(marker: u8) -> [u8; 16] {
    [
        1, 0x93, 0x4f, 0x10, 0x7c, 0, 0x70, marker, 0x80, 0x5b, 0x3b, 0x11, 0x22, 0x33, 0x44,
        marker,
    ]
}

async fn conversation_push(
    stream: &mut DuplexStream,
    sequence: u64,
    base: u64,
    value: &NpcConversation,
) -> Result<(), Error> {
    let payload =
        npc_service::encode_npc_conversation(value).map_err(|e| format!("conversation: {e:?}"))?;
    write_frame(
        stream,
        &encode_state_delta(1, sequence, 7, base, base + 1, 1, &payload)?,
    )
    .await?;
    Ok(())
}
fn npc() -> WorldSpatialEntity {
    WorldSpatialEntity {
        kind: EntityKind::Npc,
        entity: EntityRef {
            identity: [9; 16],
            generation: 2,
        },
        ..actor(101)
    }
}
