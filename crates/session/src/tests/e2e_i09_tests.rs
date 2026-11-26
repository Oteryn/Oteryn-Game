//! CLIENT-I09 replay for furniture rotation.
//!
//! The owning client path is `crates/session/src/map_transport.rs::apply`, which validates and
//! decodes domain 17, advances its revision, and emits `SessionEvent::WorldMap`; callers observe
//! that client-visible value through `crates/session/src/session_access.rs::Session::take_events`.
//! No consumer exists under `crates/client-ui/src/features/` or
//! `crates/client-runtime/src/features/`; `tools/dev-client/src/lib.rs::take_events` delegates to
//! this session owner.
//!
//! The live replay consumes the original received frames from the first admission through the
//! chair update. Node Boot supplies the session, command and reply/delta sequences from the live
//! log. The transcript must agree with that binding, the exact chair, and the original map
//! revision chain. The raw predecessor payload and metadata verify the replay's derived state:
//! - `i09-original-frames.bin`: original length-prefixed Server-to-Client frames.
//! - `i09-binding.txt`: session UUID, USE command ID, reply sequence and rotation delta sequence.
//! - `i09-map-before.bin`: raw predecessor map snapshot payload.
//! - `i09-map-before.txt`: predecessor sequence, domain, snapshot type and revision.
//! - `i09-expected.txt`: base/new revisions, delta sequence, definition, appearance,
//!   x/y/floor and base ordinal.

use super::*;
use oteryn_protocol_oteryn::world_map::{
    self, MapDefinition, MapItem, MapOrigin, MapTile, MapViewHeader, WorldMapViewDelta,
    WorldMapViewSnapshot,
};
use std::{num::NonZeroU32, path::Path};

#[test]
fn furniture_rotation_real_payload_replays_and_malformed_delta_is_atomic() -> Result<(), BoxError> {
    if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        return std::thread::scope(|scope| {
            std::thread::Builder::new()
                .stack_size(64 << 20)
                .spawn_scoped(scope, || block_on(replay_original(Path::new(&directory)))?)?
                .join()
                .map_err(|_| "I09 replay thread panicked")?
        });
    }
    run_replay(None)
}

#[test]
fn furniture_rotation_transcript_retains_original_framed_bytes() -> Result<(), BoxError> {
    block_on(async {
        let (mut reader, mut writer) = tokio::io::duplex(64);
        let wire = [0, 0, 0, 3, 0xab, 0xcd, 0xef];
        tokio::io::AsyncWriteExt::write_all(&mut writer, &wire).await?;
        with_qualification_transcript(async {
            assert_eq!(read_frame(&mut reader).await?, wire[4..]);
            assert_eq!(
                take_qualification_transcript().as_deref(),
                Some(wire.as_slice())
            );
            assert!(take_qualification_transcript().is_none());
            Ok::<_, BoxError>(())
        })
        .await?;
        assert!(take_qualification_transcript().is_none());
        Ok::<_, BoxError>(())
    })?
}

#[test]
fn furniture_rotation_replays_from_captured_predecessor() -> Result<(), BoxError> {
    let header = MapViewHeader {
        content_generation: [0x44; 32],
        bundle_digest: [0x55; 32],
        ..synthetic_header()
    };
    let (delta, (base, new, _, definition, appearance)) = synthetic_fixture(header)?;
    let mut before = world_map::decode_world_map_delta(&header, &delta)
        .map_err(|error| format!("fixture decode: {error:?}"))?;
    let chair = &mut before.tiles[0].items[1];
    chair.origin = MapOrigin::BaseOrdinal {
        ordinal: 1,
        object_revision: 0,
    };
    chair.appearance_id = 2358;
    let snapshot = world_map::encode_world_map_snapshot(&WorldMapViewSnapshot {
        header,
        tiles: before.tiles,
    })
    .map_err(|error| format!("fixture snapshot: {error:?}"))?;
    let directory = std::env::temp_dir().join(format!(
        "oteryn-i09-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    std::fs::create_dir(&directory)?;
    let result = (|| {
        std::fs::write(directory.join("i09-map-before.bin"), snapshot)?;
        std::fs::write(
            directory.join("i09-map-before.txt"),
            format!(
                "167 {} {} {base}\n",
                world_map::STATE_DOMAIN_WORLD_MAP_VIEW,
                world_map::SNAPSHOT_TYPE_WORLD_MAP_VIEW_SNAPSHOT_V1,
            ),
        )?;
        std::fs::write(directory.join("i09-map-delta.bin"), delta)?;
        std::fs::write(
            directory.join("i09-expected.txt"),
            format!("{base} {new} 168 {definition} {appearance}\n"),
        )?;
        run_replay(Some(&directory))
    })();
    std::fs::remove_dir_all(directory)?;
    result
}

fn run_replay(root: Option<&Path>) -> Result<(), BoxError> {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn_scoped(scope, || block_on(replay(root))?)?
            .join()
            .map_err(|_| "I09 replay thread panicked")?
    })
}

async fn replay(root: Option<&Path>) -> Result<(), BoxError> {
    let ((payload, expected), (predecessor_sequence, map)) = if let Some(root) = root {
        (capture_delta(root)?, capture_predecessor(root)?)
    } else {
        let header = synthetic_header();
        let (payload, expected) = synthetic_fixture(header)?;
        let map = GatedSnapshot {
            domain_id: world_map::STATE_DOMAIN_WORLD_MAP_VIEW,
            snapshot_type: world_map::SNAPSHOT_TYPE_WORLD_MAP_VIEW_SNAPSHOT_V1,
            revision: expected.0,
            payload: world_map::encode_world_map_snapshot(&WorldMapViewSnapshot {
                header,
                tiles: vec![],
            })
            .map_err(|error| format!("map: {error:?}"))?,
        };
        ((payload, expected), (40, map))
    };
    let (client, mut server) =
        tokio::io::duplex(oteryn_protocol_oteryn::MAX_WIRE_FRAME_BYTES as usize + 4);
    let peer = tokio::spawn(async move {
        read_frame(&mut server).await?;
        write_frame(
            &mut server,
            &encode_server_accepted(&ServerAcceptedValue {
                game_session_id: GameSessionId::decode(&uuid_v7(1))?,
                world_id: WorldId::decode(&uuid_v7(2))?,
                channel_id: ChannelId::decode(&uuid_v7(3))?,
                connection_generation: 1,
                current_server_sequence: 0,
                next_command_id: 7,
                schema_revision: 1,
                selected_capabilities: &[world_map::CAPABILITY_WORLD_MAP_VIEW_V1],
            })?,
        )
        .await?;
        // Admission requires a spatial snapshot from the same content generation as the map.
        let header = world_map::decode_world_map_snapshot(&map.payload)
            .map_err(|error| format!("map: {error:?}"))?
            .header;
        let spatial_payload = encode_world_spatial(&WireSpatialObservation {
            content_generation: header.content_generation,
            actor_position: header.origin,
        });
        for frame in oteryn_protocol_oteryn::encode_snapshot(
            1,
            1,
            predecessor_sequence,
            &[
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: 5,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                    payload: &spatial_payload,
                },
                DomainSnapshot {
                    domain_id: map.domain_id,
                    revision: map.revision,
                    snapshot_type: map.snapshot_type,
                    payload: &map.payload,
                },
            ],
        )? {
            write_frame(&mut server, &frame).await?;
        }
        write_frame(
            &mut server,
            &encode_state_delta(
                1,
                expected.2,
                world_map::STATE_DOMAIN_WORLD_MAP_VIEW,
                expected.0,
                expected.1,
                world_map::DELTA_TYPE_WORLD_MAP_VIEW_DELTA_V1,
                &payload,
            )?,
        )
        .await?;
        Ok::<_, BoxError>(server)
    });
    let admission = Admission {
        appearance_items: None,
        schema_revision: 1,
        character_id: CharacterId::decode(&uuid_v7(4))?,
        admission_material: b"grant",
        client_build_id: "e2e-i09-replay",
        supported_capabilities: &[world_map::CAPABILITY_WORLD_MAP_VIEW_V1],
        deadline: DEADLINE,
    };
    let mut session = Session::admit(client, admission).await?;
    assert_eq!(session.last_server_sequence(), predecessor_sequence);
    // The delta fits in the duplex buffer. Keep the peer open while processing the queued frame.
    let _server = peer.await??;
    session.service_liveness(Duration::from_millis(50)).await?;
    let events = session.take_events();
    let update = events
        .iter()
        .find_map(|event| match event {
            SessionEvent::WorldMap(update) => Some(update),
            _ => None,
        })
        .ok_or("missing world-map event")?;
    let item = update.value.tiles.iter().flat_map(|tile| &tile.items).find(|item| matches!(item.origin, MapOrigin::BaseOrdinal { object_revision, .. } if object_revision == 1)).ok_or("missing rotated item")?;
    assert_eq!(
        (
            update.base_revision,
            update.new_revision,
            update.server_sequence
        ),
        (expected.0, expected.1, expected.2)
    );
    assert_eq!(
        item.definition,
        MapDefinition::Item(NonZeroU32::new(expected.3).ok_or("zero expected definition")?)
    );
    assert_eq!(item.appearance_id, expected.4);
    assert!(map_transport::apply(&mut session, expected.2 + 1, &[0xff]).is_err());
    assert!(session.take_events().is_empty());
    Ok(())
}

async fn replay_original(root: &Path) -> Result<(), BoxError> {
    use oteryn_protocol_oteryn::world_object::{UseDisposition, decode_use_result};
    use oteryn_protocol_oteryn::{
        Direction, FrameLength, decode_command_result, decode_server_accepted, decode_state_delta,
    };

    let binding = std::fs::read_to_string(root.join("i09-binding.txt"))?;
    if binding != std::env::var("OTERYN_I09_EXPECTED_BINDING")? {
        return Err("I09 capture binding did not match the current live log".into());
    }
    let fields: Vec<_> = binding.split_ascii_whitespace().collect();
    let [session_id, command, reply_sequence, delta_sequence] = fields.as_slice() else {
        return Err("invalid I09 capture binding".into());
    };
    let command: u64 = command.parse()?;
    let reply_sequence: u64 = reply_sequence.parse()?;
    let delta_sequence: u64 = delta_sequence.parse()?;
    let expected_text = std::fs::read_to_string(root.join("i09-expected.txt"))?;
    let expected: Vec<_> = expected_text.split_ascii_whitespace().collect();
    let [
        base,
        new,
        sequence,
        definition,
        appearance,
        x,
        y,
        floor,
        ordinal,
    ] = expected.as_slice()
    else {
        return Err("invalid I09 original-frame expectations".into());
    };
    let base: u64 = base.parse()?;
    let new: u64 = new.parse()?;
    let definition = NonZeroU32::new(definition.parse()?).ok_or("zero I09 item definition")?;
    let target = ActorPosition {
        x: x.parse()?,
        y: y.parse()?,
        floor: floor.parse()?,
    };
    let ordinal: u8 = ordinal.parse()?;
    let (predecessor, map) = capture_predecessor(root)?;
    if sequence.parse::<u64>()? != delta_sequence
        || delta_sequence <= reply_sequence
        || predecessor.checked_add(1) != Some(delta_sequence)
        || base != map.revision
        || map.domain_id != world_map::STATE_DOMAIN_WORLD_MAP_VIEW
        || map.snapshot_type != world_map::SNAPSHOT_TYPE_WORLD_MAP_VIEW_SNAPSHOT_V1
        || *appearance != "2359"
        || target
            != (ActorPosition {
                x: 32355,
                y: 32206,
                floor: 7,
            })
    {
        return Err("inconsistent I09 replay expectations".into());
    }

    let bytes = std::fs::read(root.join("i09-original-frames.bin"))?;
    let mut remaining = bytes.as_slice();
    let mut frames = Vec::new();
    while !remaining.is_empty() {
        let prefix = remaining.get(..4).ok_or("truncated I09 frame prefix")?;
        let length = FrameLength::from_prefix(prefix)?.get() as usize;
        let frame = remaining
            .get(..4 + length)
            .ok_or("truncated I09 frame body")?;
        frames.push((frame, decode_wire_envelope(&frame[4..])?));
        remaining = &remaining[frame.len()..];
    }
    let (_, accepted_frame) = frames.first().ok_or("I09 transcript is empty")?;
    accepted_frame.validate(Direction::ServerToClient, false)?;
    if accepted_frame.message_type() != MessageType::ServerAccepted {
        return Err("I09 transcript did not start with ServerAccepted".into());
    }
    let accepted = decode_server_accepted(accepted_frame.payload())?;
    let id = accepted.game_session_id.as_bytes();
    let actual_id = id
        .iter()
        .enumerate()
        .fold(String::new(), |mut out, (index, byte)| {
            if matches!(index, 4 | 6 | 8 | 10) {
                out.push('-');
            }
            out.push_str(&format!("{byte:02x}"));
            out
        });
    if actual_id != *session_id || command < accepted.next_command_id {
        return Err("I09 original session or command did not match".into());
    }
    let joined = frames
        .iter()
        .position(|(_, frame)| frame.message_type() == MessageType::SnapshotCommit)
        .ok_or("I09 original admission snapshot incomplete")?;
    let original_join: Vec<u8> = frames[..=joined]
        .iter()
        .flat_map(|(bytes, _)| bytes.iter().copied())
        .collect();
    let (client, mut server) = tokio::io::duplex(64 * 1024);
    let peer = tokio::spawn(async move {
        read_frame(&mut server).await?;
        tokio::io::AsyncWriteExt::write_all(&mut server, &original_join).await?;
        Ok::<_, BoxError>(server)
    });
    let mut session = Session::admit(
        client,
        Admission {
            appearance_items: None,
            schema_revision: accepted.schema_revision,
            character_id: CharacterId::decode(&uuid_v7(4))?,
            admission_material: b"i09-offline-replay",
            client_build_id: "e2e-i09-replay",
            supported_capabilities: &accepted.selected_capabilities,
            deadline: DEADLINE,
        },
    )
    .await?;
    let _server = peer.await??;
    let mut last_sequence = session.last_server_sequence();
    let mut committed = false;
    for (_, frame) in &frames[joined + 1..] {
        frame.validate(Direction::ServerToClient, true)?;
        if frame.connection_generation() != accepted.connection_generation {
            return Err("I09 transcript crossed connection generations".into());
        }
        let sequence = frame.server_sequence();
        if sequence != 0 {
            if last_sequence.checked_add(1) != Some(sequence) {
                return Err("I09 original frame sequence is discontinuous".into());
            }
            last_sequence = sequence;
        }
        if sequence == reply_sequence {
            if frame.message_type() != MessageType::CommandResult {
                return Err("I09 expected a command result at the recorded sequence".into());
            }
            let result = decode_command_result(frame.payload())?;
            if result.command_id != command
                || result.status != CommandStatus::Accepted
                || decode_use_result(result.payload) != Ok(UseDisposition::Committed)
            {
                return Err("I09 original USE result did not match".into());
            }
            committed = true;
        }
        if frame.message_type() != MessageType::StateDelta {
            continue;
        }
        let delta = decode_state_delta(frame.payload())?;
        if delta.domain_id != world_map::STATE_DOMAIN_WORLD_MAP_VIEW {
            continue;
        }
        if sequence < delta_sequence {
            map_transport::apply(&mut session, sequence, frame.payload())?;
            continue;
        }
        if !committed
            || sequence != delta_sequence
            || delta.base_revision != base
            || delta.new_revision != new
            || session.gated_snapshot(map.domain_id) != Some(&map)
        {
            return Err("I09 original rotation did not match the captured predecessor".into());
        }
        let SessionEvent::WorldMap(update) =
            map_transport::apply(&mut session, sequence, frame.payload())?
        else {
            return Err("I09 original rotation did not publish a map update".into());
        };
        let origin = MapOrigin::BaseOrdinal {
            ordinal,
            object_revision: 1,
        };
        if !update.value.tiles.iter().any(|tile| {
            tile.position == target
                && tile.items.iter().any(|item| {
                    item.origin == origin
                        && item.definition == MapDefinition::Item(definition)
                        && item.appearance_id == 2359
                })
        }) {
            return Err("I09 original rotation did not change the exact chair".into());
        }
        assert!(map_transport::apply(&mut session, sequence + 1, &[0xff]).is_err());
        assert!(session.take_events().is_empty());
        return Ok(());
    }
    Err("I09 transcript omitted its exact rotation frame".into())
}

type Fixture = (Vec<u8>, (u64, u64, u64, u32, u16));

fn capture_delta(root: &Path) -> Result<Fixture, BoxError> {
    let payload = std::fs::read(root.join("i09-map-delta.bin"))?;
    let values = std::fs::read_to_string(root.join("i09-expected.txt"))?
        .split_whitespace()
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()?;
    let [base, new, sequence, definition, appearance] = values.as_slice() else {
        return Err("invalid I09 expected capture".into());
    };
    Ok((
        payload,
        (
            *base,
            *new,
            *sequence,
            u32::try_from(*definition)?,
            u16::try_from(*appearance)?,
        ),
    ))
}

fn capture_predecessor(root: &Path) -> Result<(u64, GatedSnapshot), BoxError> {
    let values = std::fs::read_to_string(root.join("i09-map-before.txt"))?
        .split_whitespace()
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()?;
    let [sequence, domain, snapshot_type, revision] = values.as_slice() else {
        return Err("invalid I09 predecessor capture".into());
    };
    Ok((
        *sequence,
        GatedSnapshot {
            domain_id: u32::try_from(*domain)?,
            snapshot_type: u32::try_from(*snapshot_type)?,
            revision: *revision,
            payload: std::fs::read(root.join("i09-map-before.bin"))?,
        },
    ))
}

fn synthetic_header() -> MapViewHeader {
    MapViewHeader {
        content_generation: CONTENT_GENERATION,
        bundle_digest: [0x22; 32],
        reset_epoch: 0,
        origin: ActorPosition {
            x: 0,
            y: 0,
            floor: 0,
        },
        first_visible_floor: 0,
    }
}

fn synthetic_fixture(header: MapViewHeader) -> Result<Fixture, BoxError> {
    let definition = NonZeroU32::new(42).ok_or("fixture definition")?;
    let delta = WorldMapViewDelta {
        header,
        tiles: vec![MapTile {
            position: ActorPosition {
                x: 1,
                y: 0,
                floor: 0,
            },
            items: vec![
                MapItem {
                    definition: MapDefinition::Terrain(
                        NonZeroU32::new(1).ok_or("fixture terrain")?,
                    ),
                    count: 1,
                    sub_type: 0,
                    origin: MapOrigin::BaseOrdinal {
                        ordinal: 0,
                        object_revision: 0,
                    },
                    appearance_id: 100,
                },
                MapItem {
                    definition: MapDefinition::Item(definition),
                    count: 1,
                    sub_type: 0,
                    origin: MapOrigin::BaseOrdinal {
                        ordinal: 1,
                        object_revision: 1,
                    },
                    appearance_id: 2359,
                },
            ],
            more: false,
            ground_speed: 100,
        }],
        cleared: vec![],
    };
    Ok((
        world_map::encode_world_map_delta(&header, &delta)
            .map_err(|error| format!("fixture encode: {error:?}"))?,
        (1, 2, 41, definition.get(), 2359),
    ))
}
