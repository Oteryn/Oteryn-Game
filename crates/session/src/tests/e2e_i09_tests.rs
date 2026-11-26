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
//! - `i09-{original,relog,observer}-frames.bin`: original length-prefixed frames.
//! - `i09-envelopes.json` and `i09-{relog,observer}-envelopes.json`: one #3521 envelope per frame.
//! - `i09-binding.txt`: session UUID, pre-USE sequence, USE command and reply/delta sequences,
//!   logout command and reply sequence, and distinct relog and public observer UUIDs.
//! - `i09-map-before.bin`: raw predecessor map snapshot payload.
//! - `i09-map-before.txt`: predecessor sequence, domain, snapshot type and revision.
//! - `i09-expected.txt`: base/new revisions, delta sequence, definition, appearance,
//!   chair x/y/floor, base ordinal, stale command/sequence and saved actor x/y/floor.

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
    assert_malformed_map_is_atomic(&mut session, expected.2 + 1)?;
    Ok(())
}

async fn replay_original(root: &Path) -> Result<(), BoxError> {
    use oteryn_protocol_oteryn::world_object::{UseDisposition, decode_use_result};
    use oteryn_protocol_oteryn::{Direction, decode_command_result, decode_state_delta};

    let binding = std::fs::read_to_string(root.join("i09-binding.txt"))?;
    if binding != std::env::var("OTERYN_I09_EXPECTED_BINDING")? {
        return Err("I09 capture binding did not match the current live log".into());
    }
    let fields: Vec<_> = binding.split_ascii_whitespace().collect();
    let [
        session_id,
        before_use_sequence,
        command,
        reply_sequence,
        delta_sequence,
        logout_command,
        logout_sequence,
        relog_id,
        observer_id,
    ] = fields.as_slice()
    else {
        return Err("invalid I09 capture binding".into());
    };
    let before_use_sequence: u64 = before_use_sequence.parse()?;
    let logout_command: u64 = logout_command.parse()?;
    let logout_sequence: u64 = logout_sequence.parse()?;
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
        stale_command,
        stale_sequence,
        saved_x,
        saved_y,
        saved_floor,
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
    let stale_command: u64 = stale_command.parse()?;
    let stale_sequence: u64 = stale_sequence.parse()?;
    let saved = ActorPosition {
        x: saved_x.parse()?,
        y: saved_y.parse()?,
        floor: saved_floor.parse()?,
    };
    if saved
        != (ActorPosition {
            x: 32356,
            y: 32206,
            floor: 7,
        })
        || *session_id == *observer_id
        || *relog_id == *observer_id
        || *session_id == *relog_id
        || stale_command <= command
        || stale_sequence <= delta_sequence
        || logout_command <= stale_command
        || logout_sequence <= stale_sequence
        || before_use_sequence >= reply_sequence
    {
        return Err("inconsistent I09 session or stale command binding".into());
    }
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
    let frames = validate_envelopes(root, "i09-envelopes.json", &bytes, session_id)?;
    let (mut session, joined) = admit_original(&bytes, &frames, session_id).await?;
    let generation = frames[1].connection_generation();
    let mut last_sequence = session.last_server_sequence();
    let mut committed = false;
    let mut rotated = false;
    let mut stale = false;
    let mut logged_out = false;
    let before_map = world_map::decode_world_map_snapshot(&map.payload)
        .map_err(|error| format!("I09 predecessor map decode: {error:?}"))?;
    let before_item = before_map
        .tiles
        .iter()
        .find(|tile| tile.position == target)
        .and_then(|tile| {
            tile.items.iter().find(|item| {
                item.origin
                    == (MapOrigin::BaseOrdinal {
                        ordinal,
                        object_revision: 0,
                    })
            })
        })
        .filter(|item| {
            item.appearance_id == 2358
                && matches!(item.definition, MapDefinition::Item(id) if id != definition)
        })
        .ok_or("I09 captured predecessor did not contain the original chair")?;
    let rotated_item = world_map::MapItem {
        definition: MapDefinition::Item(definition),
        appearance_id: 2359,
        origin: MapOrigin::BaseOrdinal {
            ordinal,
            object_revision: 1,
        },
        ..*before_item
    };
    if !has_unique_chair(&before_map.tiles, target, ordinal, before_item) {
        return Err("I09 captured predecessor contains an ambiguous chair".into());
    }
    let expected_chair =
        |tiles: &[MapTile]| has_unique_chair(tiles, target, ordinal, &rotated_item);
    for frame in &frames[joined..] {
        frame.validate(Direction::ServerToClient, true)?;
        if frame.connection_generation() != generation {
            return Err("I09 transcript crossed connection generations".into());
        }
        let sequence = frame.server_sequence();
        if sequence != 0 {
            if last_sequence.checked_add(1) != Some(sequence) {
                return Err("I09 original frame sequence is discontinuous".into());
            }
            last_sequence = sequence;
        }
        if frame.message_type() == MessageType::CommandResult {
            let result = decode_command_result(frame.payload())?;
            if sequence == logout_sequence {
                assert_logout_receipt(logout_command, result)?;
                logged_out = true;
            }
            if sequence == reply_sequence {
                if result.command_id != command
                    || result.status != CommandStatus::Accepted
                    || decode_use_result(result.payload) != Ok(UseDisposition::Committed)
                {
                    return Err("I09 original USE result did not match".into());
                }
                committed = true;
            }
            if sequence == stale_sequence {
                if result.command_id != stale_command
                    || result.status != CommandStatus::Accepted
                    || decode_use_result(result.payload) != Ok(UseDisposition::StaleState)
                {
                    return Err("I09 original stale refusal did not match".into());
                }
                stale = true;
            }
        }
        if frame.message_type() != MessageType::StateDelta {
            continue;
        }
        let delta = decode_state_delta(frame.payload())?;
        if delta.domain_id != world_map::STATE_DOMAIN_WORLD_MAP_VIEW {
            continue;
        }
        let is_rotation = sequence == delta_sequence;
        if is_rotation
            && (!committed
                || rotated
                || delta.base_revision != base
                || delta.new_revision != new
                || session.gated_snapshot(map.domain_id) != Some(&map))
        {
            return Err("I09 original rotation did not match the captured predecessor".into());
        }
        let SessionEvent::WorldMap(update) =
            map_transport::apply(&mut session, sequence, frame.payload())?
        else {
            return Err("I09 original rotation did not publish a map update".into());
        };
        let touches_chair = update.value.cleared.contains(&target)
            || update
                .value
                .tiles
                .iter()
                .any(|tile| tile.position == target);
        if !is_rotation {
            if touches_chair
                && sequence > before_use_sequence
                && (!rotated || !expected_chair(&update.value.tiles))
            {
                return Err("I09 unexpected chair update outside the bound rotation".into());
            }
            continue;
        }
        if !expected_chair(&update.value.tiles) {
            return Err("I09 original rotation did not change the exact chair".into());
        }
        rotated = true;
    }
    if !rotated
        || !stale
        || !logged_out
        || !session
            .world_map_snapshot()
            .is_some_and(|map| expected_chair(&map.tiles))
    {
        return Err(
            "I09 original session did not retain the exact chair after stale USE and logout".into(),
        );
    }
    assert_malformed_map_is_atomic(&mut session, last_sequence + 1)?;

    let relog_bytes = std::fs::read(root.join("i09-relog-frames.bin"))?;
    let relog_frames =
        validate_envelopes(root, "i09-relog-envelopes.json", &relog_bytes, relog_id)?;
    let (relogged, _) = admit_original(&relog_bytes, &relog_frames, relog_id).await?;
    if relogged.world_spatial().actor_position != saved
        || !relogged
            .world_map_snapshot()
            .is_some_and(|map| expected_chair(&map.tiles))
    {
        return Err(
            "I09 original relog state differs from the saved position or rotated chair".into(),
        );
    }
    let observer_bytes = std::fs::read(root.join("i09-observer-frames.bin"))?;
    let observer_frames = validate_envelopes(
        root,
        "i09-observer-envelopes.json",
        &observer_bytes,
        observer_id,
    )?;
    let (mut observer, joined) =
        admit_original(&observer_bytes, &observer_frames, observer_id).await?;
    let mut previous_sequence = observer.last_server_sequence();
    for frame in &observer_frames[joined..] {
        frame.validate(Direction::ServerToClient, true)?;
        if frame.connection_generation() != observer_frames[1].connection_generation() {
            return Err("I09 observer crossed connection generations".into());
        }
        let sequence = frame.server_sequence();
        if sequence != 0 {
            if previous_sequence.checked_add(1) != Some(sequence) {
                return Err("I09 observer frame sequence is discontinuous".into());
            }
            previous_sequence = sequence;
        }
        if frame.message_type() == MessageType::StateDelta
            && decode_state_delta(frame.payload())?.domain_id
                == world_map::STATE_DOMAIN_WORLD_MAP_VIEW
        {
            map_transport::apply(&mut observer, sequence, frame.payload())?;
        }
    }
    if !observer
        .world_map_snapshot()
        .is_some_and(|map| expected_chair(&map.tiles))
    {
        return Err("I09 distinct observer did not see the exact public chair rotation".into());
    }
    Ok(())
}

fn has_unique_chair(
    tiles: &[MapTile],
    target: ActorPosition,
    ordinal: u8,
    expected: &world_map::MapItem,
) -> bool {
    let mut chairs = tiles.iter().filter(|tile| tile.position == target)
        .flat_map(|tile| &tile.items)
        .filter(|item| matches!(item.appearance_id, 2358 | 2359)
            || matches!(item.origin, MapOrigin::BaseOrdinal { ordinal: actual, .. } if actual == ordinal));
    chairs.next() == Some(expected) && chairs.next().is_none()
}

#[test]
fn furniture_rotation_rejects_duplicate_and_character_local_views() {
    let target = ActorPosition {
        x: 32355,
        y: 32206,
        floor: 7,
    };
    let rotated = world_map::MapItem {
        count: 1,
        sub_type: 0,
        origin: MapOrigin::BaseOrdinal {
            ordinal: 1,
            object_revision: 1,
        },
        definition: MapDefinition::Item(NonZeroU32::MIN),
        appearance_id: 2359,
    };
    let original = world_map::MapItem {
        origin: MapOrigin::BaseOrdinal {
            ordinal: 1,
            object_revision: 0,
        },
        appearance_id: 2358,
        ..rotated
    };
    let mut tile = MapTile {
        position: target,
        items: vec![rotated],
        more: false,
        ground_speed: 100,
    };
    assert!(has_unique_chair(
        std::slice::from_ref(&tile),
        target,
        1,
        &rotated
    ));
    tile.items.push(original);
    assert!(!has_unique_chair(
        std::slice::from_ref(&tile),
        target,
        1,
        &rotated
    ));
    tile.items.remove(0);
    assert!(!has_unique_chair(&[tile], target, 1, &rotated));
}

fn assert_logout_receipt(
    expected_command: u64,
    result: oteryn_protocol_oteryn::CommandResultView<'_>,
) -> Result<(), BoxError> {
    use oteryn_protocol_oteryn::logout::{LogoutResult, decode_logout_result};
    if result.command_id != expected_command
        || result.status != CommandStatus::Accepted
        || decode_logout_result(result.payload) != Ok(LogoutResult::Accepted)
    {
        return Err("I09 original logout receipt did not match the live logout command".into());
    }
    Ok(())
}

fn assert_malformed_map_is_atomic<S: SessionStream>(
    session: &mut Session<S>,
    sequence: u64,
) -> Result<(), BoxError> {
    let domain = world_map::STATE_DOMAIN_WORLD_MAP_VIEW;
    let snapshot = session
        .gated_snapshot(domain)
        .ok_or("I09 map absent")?
        .clone();
    let header = session.map_header;
    let revisions = session.gated_revisions.clone();
    let framed = encode_state_delta(
        1,
        sequence,
        domain,
        snapshot.revision,
        snapshot.revision + 1,
        world_map::DELTA_TYPE_WORLD_MAP_VIEW_DELTA_V1,
        &[0xff],
    )?;
    for malformed in [&[0xff][..], decode_wire_envelope(&framed)?.payload()] {
        assert!(map_transport::apply(session, sequence, malformed).is_err());
        assert_eq!(session.gated_snapshot(domain), Some(&snapshot));
        assert_eq!(session.map_header, header);
        assert_eq!(session.gated_revisions, revisions);
        assert!(session.take_events().is_empty());
    }
    Ok(())
}

#[test]
fn furniture_rotation_does_not_treat_a_use_result_as_the_logout_receipt() -> Result<(), BoxError> {
    use oteryn_protocol_oteryn::{CommandResultView, logout, world_object};
    let use_payload = world_object::encode_use_result(world_object::UseDisposition::Committed)
        .map_err(|error| format!("{error:?}"))?;
    let logout_payload = logout::encode_logout_result(logout::LogoutResult::Accepted)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(use_payload, logout_payload);
    let receipt = |command_id| CommandResultView {
        command_id,
        status: CommandStatus::Accepted,
        error_code: 0,
        payload: &logout_payload,
    };
    assert!(assert_logout_receipt(8, receipt(7)).is_err());
    assert_logout_receipt(8, receipt(8))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CaptureEnvelope {
    command_id: u64,
    game_session_id: String,
    payload: Vec<u8>,
    server_sequence: u64,
    status: u32,
}

fn validate_envelopes<'a>(
    root: &Path,
    name: &str,
    mut transcript: &'a [u8],
    session: &str,
) -> Result<Vec<oteryn_protocol_oteryn::WireEnvelopeView<'a>>, BoxError> {
    use oteryn_protocol_oteryn::{FrameLength, decode_command_result};

    let envelopes: Vec<CaptureEnvelope> = serde_json::from_slice(&std::fs::read(root.join(name))?)?;
    let mut frames = Vec::new();
    for envelope in envelopes {
        let prefix = transcript.get(..4).ok_or("truncated I09 frame prefix")?;
        let length = FrameLength::from_prefix(prefix)?.get() as usize;
        let frame = decode_wire_envelope(
            transcript
                .get(4..4 + length)
                .ok_or("truncated I09 frame body")?,
        )?;
        transcript = &transcript[4 + length..];
        let sequence = frame.server_sequence();
        let (command, status) = if frame.message_type() == MessageType::CommandResult {
            let reply = decode_command_result(frame.payload())?;
            (reply.command_id, reply.status as u32)
        } else {
            (0, 0)
        };
        if envelope.game_session_id != session
            || envelope.command_id != command
            || envelope.server_sequence != sequence
            || envelope.status != status
            || envelope.payload != frame.payload()
        {
            return Err("I09 #3521 envelope does not match its original received frame".into());
        }
        frames.push(frame);
    }
    if frames.is_empty() || !transcript.is_empty() {
        return Err("I09 #3521 envelopes do not cover the original received frames".into());
    }
    Ok(frames)
}

#[test]
fn furniture_rotation_rejects_modified_3521_envelopes() -> Result<(), BoxError> {
    let body = encode_state_delta(1, 42, 17, 1, 2, 1, &[0xab])?;
    let payload = decode_wire_envelope(&body)?.payload();
    let mut transcript = oteryn_protocol_oteryn::FrameLength::new(u32::try_from(body.len())?)?
        .to_prefix()
        .to_vec();
    transcript.extend_from_slice(&body);
    let session = "01234567-89ab-7cde-8f01-23456789abcd";
    let valid = serde_json::json!({
        "command_id": 0,
        "game_session_id": session,
        "payload": payload,
        "server_sequence": 42,
        "status": 0,
    });
    let root = std::env::temp_dir().join(format!(
        "oteryn-i09-envelope-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos(),
    ));
    std::fs::create_dir(&root)?;
    let name = "envelopes.json";
    let result = (|| {
        std::fs::write(root.join(name), serde_json::to_vec(&[&valid])?)?;
        assert_eq!(
            validate_envelopes(&root, name, &transcript, session)?.len(),
            1
        );
        for (field, wrong) in [
            ("command_id", serde_json::json!(8)),
            ("game_session_id", serde_json::json!("another session")),
            ("payload", serde_json::json!([0])),
            ("server_sequence", serde_json::json!(43)),
            ("status", serde_json::json!(CommandStatus::Accepted as u32)),
            ("extra", serde_json::json!(null)),
        ] {
            let mut changed = valid.clone();
            changed[field] = wrong;
            std::fs::write(root.join(name), serde_json::to_vec(&[changed])?)?;
            assert!(validate_envelopes(&root, name, &transcript, session).is_err());
        }
        Ok(())
    })();
    std::fs::remove_dir_all(root)?;
    result
}

async fn admit_original(
    bytes: &[u8],
    frames: &[oteryn_protocol_oteryn::WireEnvelopeView<'_>],
    session_id: &str,
) -> Result<(Session<tokio::io::DuplexStream>, usize), BoxError> {
    use oteryn_protocol_oteryn::{Direction, decode_server_accepted};

    let accepted_frame = frames.first().ok_or("I09 transcript is empty")?;
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
    if actual_id != session_id {
        return Err("I09 original admitted session did not match".into());
    }
    let joined = frames
        .iter()
        .position(|frame| frame.message_type() == MessageType::SnapshotCommit)
        .ok_or("I09 original admission snapshot incomplete")?
        + 1;
    let mut remaining = bytes;
    for _ in 0..joined {
        let length =
            oteryn_protocol_oteryn::FrameLength::from_prefix(&remaining[..4])?.get() as usize;
        remaining = &remaining[4 + length..];
    }
    let original_join = bytes[..bytes.len() - remaining.len()].to_vec();
    let (client, mut server) = tokio::io::duplex(64 * 1024);
    let peer = tokio::spawn(async move {
        read_frame(&mut server).await?;
        tokio::io::AsyncWriteExt::write_all(&mut server, &original_join).await?;
        Ok::<_, BoxError>(server)
    });
    let session = Session::admit(
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
    Ok((session, joined))
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
