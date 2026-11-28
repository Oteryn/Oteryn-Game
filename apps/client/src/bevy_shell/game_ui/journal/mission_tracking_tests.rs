use super::*;
use crate::play::run_session;
use oteryn_protocol_oteryn::{
    ChannelId, DomainSnapshot, GameSessionId, ServerAcceptedValue, WorldId, decode_wire_envelope,
    encode_command_result, encode_liveness_probe, encode_server_accepted,
    encode_single_chunk_snapshot, encode_state_delta, world_object,
    world_spatial::{self, ActorPosition, WorldSpatialObservation},
};
use oteryn_session::{Admission, CharacterId, CommandStatus, Session, read_frame, write_frame};
use quest_log::{
    CAPABILITY_QUEST_LOG_V1, CAPABILITY_QUEST_MISSION_TRACKING_V1, MAX_FREE_TRACKED_MISSIONS,
    QuestLine, QuestLog, QuestLogView, QuestMission, TrackedMission, decode_quest_mission_track,
    encode_quest_log, encode_quest_log_missions,
};
use std::time::Duration;
use tokio::io::DuplexStream;

type Error = Box<dyn std::error::Error>;
const DEADLINE: Duration = Duration::from_secs(5);

fn id(marker: u8) -> [u8; 16] {
    let mut bytes = [marker; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes
}

async fn admit(
    client: DuplexStream,
    server: &mut DuplexStream,
    caps: &[u32],
    payload: &[u8],
) -> Result<Session<DuplexStream>, Error> {
    let peer = async {
        let _ = read_frame(server).await?;
        write_frame(
            server,
            &encode_server_accepted(&ServerAcceptedValue {
                game_session_id: GameSessionId::decode(&id(2))?,
                world_id: WorldId::decode(&id(3))?,
                channel_id: ChannelId::decode(&id(4))?,
                connection_generation: 1,
                current_server_sequence: 0,
                next_command_id: 1,
                schema_revision: 1,
                selected_capabilities: caps,
            })?,
        )
        .await?;
        let spatial = world_spatial::encode_world_spatial(&WorldSpatialObservation {
            content_generation: [1; 32],
            actor_position: ActorPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
        });
        let overlay = world_object::encode_world_object_overlay_snapshot(&[])
            .map_err(|_| "overlay encode")?;
        for frame in encode_single_chunk_snapshot(
            1,
            1,
            0,
            &[
                DomainSnapshot {
                    domain_id: world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: 0,
                    snapshot_type: world_spatial::SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                    payload: &spatial,
                },
                DomainSnapshot {
                    domain_id: world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                    revision: 0,
                    snapshot_type: world_object::SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                    payload: &overlay,
                },
                DomainSnapshot {
                    domain_id: 16,
                    revision: 0,
                    snapshot_type: 1,
                    payload,
                },
            ],
        )? {
            write_frame(server, &frame).await?;
        }
        Ok::<_, Error>(())
    };
    let (admitted, sent) = tokio::join!(
        Session::admit(
            client,
            Admission {
                appearance_items: None,
                schema_revision: 1,
                character_id: CharacterId::decode(&id(1))?,
                admission_material: b"quest-tracker",
                client_build_id: "quest-tracker-test",
                supported_capabilities: caps,
                deadline: DEADLINE,
            },
        ),
        peer,
    );
    sent?;
    Ok(admitted?)
}

fn panel(host: &HostOwner) -> Result<quest_log::QuestPanel, Error> {
    Ok(host.play.as_ref().ok_or("play")?.link.quest_panel())
}

async fn wait_for(
    host: &HostOwner,
    ready: impl Fn(&quest_log::QuestPanel) -> bool,
) -> Result<(), Error> {
    while !ready(&panel(host)?) {
        tokio::task::yield_now().await;
    }
    Ok(())
}

fn run_case(case: impl AsyncFnOnce() -> Result<(), Error> + Send + 'static) -> Result<(), Error> {
    let thread = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?;
            runtime
                .block_on(async { tokio::time::timeout(DEADLINE, case()).await })
                .map_err(|error| error.to_string())?
                .map_err(|error| error.to_string())
        })?;
    match thread.join() {
        Ok(result) => result.map_err(Into::into),
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

#[test]
fn gap_0577_quest_mission_tracking() -> Result<(), Error> {
    run_case(async || {
        let (mut host, commands, outbox) = super::super::tests::host()?;
        let (client, mut server) = tokio::io::duplex(4096);
        let line = QuestLine {
            quest_index: 73,
            completed: false,
            missions: (1..=3)
                .map(|mission_index| QuestMission {
                    mission_index,
                    name: format!("Mission {mission_index}"),
                    text: format!("Objective {mission_index}"),
                    done: false,
                })
                .collect(),
        };
        let confirmed = |mission: usize| QuestLine {
            missions: vec![line.missions[mission].clone()],
            ..line.clone()
        };
        let mut log = QuestLog {
            view: Some(QuestLogView::Line(line.clone())),
            tracked: vec![confirmed(0)],
            ..default()
        };
        log.tracked
            .extend((1..MAX_FREE_TRACKED_MISSIONS - 1).map(|index| QuestLine {
                quest_index: index as u32,
                ..confirmed(0)
            }));
        let caps = [
            CAPABILITY_QUEST_LOG_V1,
            CAPABILITY_QUEST_MISSION_TRACKING_V1,
        ];
        let snapshot = encode_quest_log_missions(&log).map_err(|_| "snapshot encode")?;
        let session = admit(client, &mut server, &caps, &snapshot).await?;
        let task = tokio::spawn(run_session(session, commands, outbox));
        wait_for(&host, |panel| panel.state.revision == Some(0)).await?;
        host.play.as_ref().ok_or("play")?.link.quest_set_open(true);
        let key = Key {
            generation: host.generation(),
            tab: Tab::Quests,
        };
        let mut state = State {
            key: Some(key),
            ..default()
        };
        let values = Values::default();
        for kind in [Kind::Mission(73, 2), Kind::Track] {
            assert!(perform(
                &mut host,
                &mut state,
                Action { key, kind },
                &values
            ));
        }
        let frame = read_frame(&mut server).await?;
        let command = decode_wire_envelope(&frame)?.client_command(1)?;
        assert_eq!(command.command_type, 6318);
        let requested =
            decode_quest_mission_track(command.payload).map_err(|_| "decode mission tracking")?;
        assert_eq!(requested.len(), MAX_FREE_TRACKED_MISSIONS);
        assert_eq!(
            requested[0],
            TrackedMission {
                quest_index: 73,
                mission_index: 1
            }
        );
        assert_eq!(
            requested[requested.len() - 1],
            TrackedMission {
                quest_index: 73,
                mission_index: 2
            }
        );
        assert_eq!(panel(&host)?.state.log.tracked, log.tracked);
        write_frame(
            &mut server,
            &encode_command_result(1, 1, command.command_id, CommandStatus::Accepted, &[])?,
        )
        .await?;
        // The idle probe is serviced after the command result has reached the mailbox.
        write_frame(&mut server, &encode_liveness_probe(1, 1)?).await?;
        let ack = read_frame(&mut server).await?;
        assert_eq!(decode_wire_envelope(&ack)?.liveness_ack(1)?.probe_id, 1);
        assert_eq!(panel(&host)?.state.log.tracked, log.tracked);
        assert!(
            model::quests(&panel(&host)?, 0, false)
                .status
                .contains("Waiting for server confirmation")
        );
        log.tracked.push(confirmed(1));
        let delta = encode_quest_log_missions(&log).map_err(|_| "delta encode")?;
        write_frame(&mut server, &encode_state_delta(1, 2, 16, 0, 1, 1, &delta)?).await?;
        wait_for(&host, |panel| panel.state.revision == Some(1)).await?;
        let confirmed_panel = panel(&host)?;
        assert_eq!(confirmed_panel.state.log.tracked, log.tracked);
        let view = model::quests(&confirmed_panel, 0, false);
        assert!(view.tracker.contains("Mission 1: Objective 1"));
        assert!(view.tracker.contains("Mission 2: Objective 2"));
        assert_eq!(
            view.missions
                .iter()
                .filter(|row| row.label.ends_with("(tracked)"))
                .count(),
            2
        );
        assert!(view.tracker.contains("10 free, 25 Premium"));

        for kind in [Kind::Mission(73, 3), Kind::Track] {
            assert!(perform(
                &mut host,
                &mut state,
                Action { key, kind },
                &values
            ));
        }
        let frame = read_frame(&mut server).await?;
        let command = decode_wire_envelope(&frame)?.client_command(1)?;
        assert_eq!(command.command_type, 6318);
        assert_eq!(
            decode_quest_mission_track(command.payload)
                .map_err(|_| "decode over-limit request")?
                .len(),
            MAX_FREE_TRACKED_MISSIONS + 1
        );
        write_frame(
            &mut server,
            &encode_command_result(1, 3, command.command_id, CommandStatus::Rejected, &[])?,
        )
        .await?;
        wait_for(&host, |panel| panel.state.refusal.is_some()).await?;
        let confirmed_panel = panel(&host)?;
        assert_eq!(confirmed_panel.state.log.tracked, log.tracked);
        assert!(
            model::quests(&confirmed_panel, 0, false)
                .status
                .contains("Check your tracker limit")
        );

        assert!(perform(
            &mut host,
            &mut state,
            Action {
                key,
                kind: Kind::ClearTracker
            },
            &values
        ));
        let frame = read_frame(&mut server).await?;
        let command = decode_wire_envelope(&frame)?.client_command(1)?;
        assert_eq!(command.command_type, 6318);
        assert!(command.payload.is_empty());
        write_frame(
            &mut server,
            &encode_command_result(1, 4, command.command_id, CommandStatus::Accepted, &[])?,
        )
        .await?;
        log.tracked.clear();
        let delta = encode_quest_log_missions(&log).map_err(|_| "clear delta encode")?;
        write_frame(&mut server, &encode_state_delta(1, 5, 16, 1, 2, 1, &delta)?).await?;
        wait_for(&host, |panel| panel.state.revision == Some(2)).await?;
        assert!(panel(&host)?.state.log.tracked.is_empty());
        drop(host);
        task.await?;
        Ok(())
    })
}

#[test]
fn gap_0577_legacy_quest_tracking_keeps_command_22_bytes() -> Result<(), Error> {
    run_case(async || {
        let (mut host, commands, outbox) = super::super::tests::host()?;
        let (client, mut server) = tokio::io::duplex(4096);
        let log = super::super::tests::quest_state().log;
        let snapshot = encode_quest_log(&log).map_err(|_| "legacy snapshot encode")?;
        let session = admit(client, &mut server, &[CAPABILITY_QUEST_LOG_V1], &snapshot).await?;
        let task = tokio::spawn(run_session(session, commands, outbox));
        wait_for(&host, |panel| panel.state.revision == Some(0)).await?;
        host.play.as_ref().ok_or("play")?.link.quest_set_open(true);
        let key = Key {
            generation: host.generation(),
            tab: Tab::Quests,
        };
        let mut state = State {
            key: Some(key),
            ..default()
        };
        assert!(perform(
            &mut host,
            &mut state,
            Action {
                key,
                kind: Kind::Track
            },
            &Values::default()
        ));
        let frame = read_frame(&mut server).await?;
        let command = decode_wire_envelope(&frame)?.client_command(1)?;
        assert_eq!(command.command_type, 22);
        assert_eq!(command.payload, &[0x1a, 0x02, 0x08, 73]);
        write_frame(
            &mut server,
            &encode_command_result(1, 1, command.command_id, CommandStatus::Accepted, &[])?,
        )
        .await?;
        drop(host);
        task.await?;
        Ok(())
    })
}
