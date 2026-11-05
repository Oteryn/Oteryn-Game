//! C06: wire replacements through the real Session, play task, mailbox and visible scene.
use oteryn_client::input::StepDir;
use oteryn_client::play::{PlayView, play_channel, run_session};
use oteryn_protocol_oteryn::world_map::*;
use oteryn_protocol_oteryn::world_spatial::{encode_step_result, encode_world_spatial};
use oteryn_protocol_oteryn::{
    ChannelId, CharacterId, CommandStatus, DomainSnapshot, GameSessionId, ServerAcceptedValue,
    WorldId, encode_command_result, encode_server_accepted, encode_single_chunk_snapshot,
    encode_state_delta,
};
use oteryn_session::{
    ActorPosition, Admission, Session, StepDisposition, WorldSpatialObservation, read_frame,
    write_frame,
};
use std::time::Duration;
use tokio::io::{AsyncReadExt, DuplexStream};
type Error = Box<dyn std::error::Error + Send + Sync>;

fn uuid(marker: u8) -> [u8; 16] {
    [
        1, 0x93, 0x4f, 0x10, 0x7c, 0, 0x70, marker, 0x80, 0x5b, 0x3b, 0x11, 0x22, 0x33, 0x44,
        marker,
    ]
}

fn map(floor: i16, epoch: u64) -> WorldMapViewSnapshot {
    let origin = ActorPosition {
        x: 100,
        y: 200,
        floor,
    };
    let floors = if floor == 0 {
        vec![0]
    } else {
        vec![-10, -9, -8, -6]
    };
    WorldMapViewSnapshot {
        header: MapViewHeader {
            content_generation: [1; 32],
            bundle_digest: [2; 32],
            reset_epoch: epoch,
            origin,
            first_visible_floor: if floor == 0 { 0 } else { -8 },
        },
        tiles: floors
            .into_iter()
            .map(|floor| {
                let shift = i32::from(floor - origin.floor);
                MapTile {
                    position: ActorPosition {
                        x: origin.x + shift,
                        y: origin.y + shift,
                        floor,
                    },
                    items: vec![MapItem {
                        definition: MapDefinition::Item(std::num::NonZeroU32::MIN),
                        count: 1,
                        sub_type: 0,
                        origin: MapOrigin::DisplayOnly,
                        appearance_id: 0,
                    }],
                    more: false,
                    ground_speed: 0,
                }
            })
            .collect(),
    }
}

async fn snapshot(
    server: &mut DuplexStream,
    sequence: u64,
    value: &WorldMapViewSnapshot,
    join: bool,
) -> Result<(), Error> {
    let payload = encode_world_map_snapshot(value).map_err(|e| format!("{e:?}"))?;
    let spatial = encode_world_spatial(&WorldSpatialObservation {
        content_generation: [1; 32],
        actor_position: value.header.origin,
    });
    let mut domains = vec![DomainSnapshot {
        domain_id: 17,
        revision: value.header.reset_epoch,
        snapshot_type: 1,
        payload: &payload,
    }];
    if join {
        domains.push(DomainSnapshot {
            domain_id: 1,
            revision: 1,
            snapshot_type: 1,
            payload: &spatial,
        });
    }
    for frame in encode_single_chunk_snapshot(1, sequence, sequence, &domains)? {
        write_frame(server, &frame).await?;
    }
    Ok(())
}

async fn peer(mut server: DuplexStream) -> Result<(), Error> {
    read_frame(&mut server).await?;
    write_frame(
        &mut server,
        &encode_server_accepted(&ServerAcceptedValue {
            game_session_id: GameSessionId::decode(&uuid(1))?,
            world_id: WorldId::decode(&uuid(2))?,
            channel_id: ChannelId::decode(&uuid(3))?,
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 7,
            schema_revision: 1,
            selected_capabilities: &[18],
        })?,
    )
    .await?;
    snapshot(&mut server, 40, &map(0, 1), true).await?;
    read_frame(&mut server).await?;
    snapshot(&mut server, 41, &map(-8, 2), false).await?;
    for (command, sequence, revision, floor) in [(7, 42, 1, -8), (8, 44, 2, 0)] {
        if command == 8 {
            read_frame(&mut server).await?;
        }
        write_frame(
            &mut server,
            &encode_command_result(
                1,
                sequence,
                command,
                CommandStatus::Accepted,
                &encode_step_result(StepDisposition::Moved),
            )?,
        )
        .await?;
        let spatial = encode_world_spatial(&WorldSpatialObservation {
            content_generation: [1; 32],
            actor_position: map(floor, 1).header.origin,
        });
        write_frame(
            &mut server,
            &encode_state_delta(1, sequence + 1, 1, revision, revision + 1, 1, &spatial)?,
        )
        .await?;
    }
    snapshot(&mut server, 46, &map(0, 3), false).await?;
    let mut rest = vec![];
    let _ = server.read_to_end(&mut rest).await;
    Ok(())
}

#[test]
fn native_floor_transition_replaces_window_and_keeps_playing() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    let (client, server) = tokio::io::duplex(256 * 1024);
                    let peer = tokio::spawn(peer(server));
                    let session = Session::admit(
                        client,
                        Admission {
                            appearance_items: None,
                            schema_revision: 1,
                            character_id: CharacterId::decode(&uuid(4))?,
                            admission_material: b"synthetic",
                            client_build_id: "c06",
                            supported_capabilities: &[18],
                            deadline: Duration::from_secs(5),
                        },
                    )
                    .await?;
                    let mut view = PlayView::from_join(session.join_snapshot())?;
                    view.set_live_map(&session.world_map_snapshot().ok_or("missing initial map")?)?;
                    let (link, commands, outbox) = play_channel();
                    let task = tokio::spawn(run_session(session, commands, outbox));
                    let mut preferences = view.preferences().clone();
                    preferences
                        .visible_panels
                        .retain(|panel| panel != "minimap");
                    view.set_preferences(preferences)?;
                    for (floor, sprites) in [(-8, 4), (0, 2)] {
                        link.request(StepDir::North);
                        tokio::time::timeout(Duration::from_secs(5), async {
                            loop {
                                view.tick(&link).map_err(|e| format!("{e:?}"))?;
                                if view.map_center() == (100, 200, floor)
                                    && view.scene().sprites().len() == sprites
                                {
                                    break;
                                }
                                tokio::task::yield_now().await;
                            }
                            Ok::<(), Error>(())
                        })
                        .await??;
                    }
                    drop(link);
                    task.abort();
                    let _ = task.await;
                    peer.await??;
                    Ok::<(), Error>(())
                })
        })?
        .join()
        .map_err(|_| "floor scenario thread failed")?
}
