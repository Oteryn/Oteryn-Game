//! C24: actual admitted Session -> native input/task/mailbox -> rendered actor and camera.
use oteryn_client::play::{PlayLink, PlayView, play_channel, run_session};
use oteryn_protocol_oteryn::{
    ChannelId, CharacterId, CommandStatus, DomainSnapshot, GameSessionId, ServerAcceptedValue,
    WorldId, decode_wire_envelope, encode_command_result, encode_server_accepted,
    encode_single_chunk_snapshot, encode_state_delta,
    target_move::{TargetMoveIntent, decode_target_move_intent},
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
use tokio::io::{AsyncReadExt, DuplexStream};
use tokio::sync::oneshot;

type Error = Box<dyn std::error::Error + Send + Sync>;
const WAIT: Duration = Duration::from_secs(5);

#[test]
fn native_server_target_walk_scenario() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(scenario())
        })?
        .join()
        .map_err(|_| std::io::Error::other("target walk scenario thread panicked"))?
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
            client_build_id: "c24-native-scenario",
            supported_capabilities: CLIENT_SUPPORTED_CAPABILITIES,
            deadline: WAIT,
        },
    )
    .await?;
    let mut view = PlayView::from_join(session.join_snapshot())?;
    let (link, commands, outbox) = play_channel();
    let task = tokio::spawn(run_session(session, commands, outbox));
    pump(&mut view, &link, |_, link| {
        link.target_walk_state().selected
    })
    .await?;

    for feedback in [
        "Walk request refused",
        "No path to that tile",
        "Walk request accepted",
    ] {
        view.click(TileCoord::new(102, 200))?;
        view.tick(&link).map_err(|e| format!("tick: {e:?}"))?;
        pump(&mut view, &link, |view, _| {
            view.hud_status().contains(feedback)
        })
        .await?;
        assert_eq!(view.own(), TileCoord::new(100, 200));
        assert_eq!(view.next_step(), None);
    }
    first
        .send(())
        .map_err(|_| "peer ended before first movement")?;
    pump(&mut view, &link, |view, _| {
        view.own() == TileCoord::new(101, 200)
    })
    .await?;
    assert_eq!(view.scene().view().origin(), TileCoord::new(94, 195));
    second
        .send(())
        .map_err(|_| "peer ended before second movement")?;
    pump(&mut view, &link, |view, _| {
        view.own() == TileCoord::new(102, 200)
    })
    .await?;
    assert_eq!(view.scene().view().origin(), TileCoord::new(95, 195));
    assert_eq!(view.next_step(), None);
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
            .contains(&64)
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
            selected_capabilities: &[6, 13, 64],
        })?,
    )
    .await?;
    let spatial = encode_world_spatial_entities_snapshot(&WorldSpatialEntitiesSnapshot {
        content_generation: [1; 32],
        actor_position: position(100),
        own_identity: [7; 16],
        entities: vec![actor(100)],
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
                domain_id: 2,
                revision: 2,
                snapshot_type: 1,
                payload: &[],
            },
        ],
    )? {
        write_frame(&mut stream, &frame).await?;
    }
    for (command_id, status, payload) in [
        (7, CommandStatus::Rejected, vec![8, 3]),
        (8, CommandStatus::Accepted, vec![8, 2]),
        (9, CommandStatus::Accepted, vec![8, 1]),
    ] {
        let bytes = read_frame(&mut stream).await?;
        let command = decode_wire_envelope(&bytes)?.client_command(1)?;
        assert_eq!(command.command_id, command_id);
        assert_eq!(command.command_type, 81);
        assert_eq!(
            decode_target_move_intent(command.payload).map_err(|e| format!("intent: {e:?}"))?,
            TargetMoveIntent::Walk(position(102))
        );
        write_frame(
            &mut stream,
            &encode_command_result(1, 34 + command_id, command_id, status, &payload)?,
        )
        .await?;
    }
    first.await?;
    push(&mut stream, 44, 5, 101).await?;
    second.await?;
    push(&mut stream, 45, 6, 102).await?;
    // A click must send exactly one target request, never subsequent local STEP commands.
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
