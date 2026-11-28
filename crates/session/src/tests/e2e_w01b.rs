//! W01b replay: a rat's kill (damage, removal, corpse) and its authored respawn applied through `Session::apply_state_delta`
//! (the domain-1 type-2 entity decode in `domain_dispatch.rs`) and exposed by
//! `Session::world_entities`, the entity view the world renderer reads.
use super::*;
use oteryn_protocol_oteryn::world_spatial_entities::{
    EntityDetail as Detail, encode_world_spatial_entities_delta,
};
use oteryn_protocol_oteryn::{decode_wire_envelope, encode_state_delta};

fn value<'a>(text: &'a str, key: &str) -> Result<&'a str, BoxError> {
    text.lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected line {key} absent").into())
}

struct Fixture {
    expected: String,
    kill_snapshot: Vec<u8>,
    kill_deltas: Vec<Vec<u8>>,
    respawn_snapshot: Vec<u8>,
}

fn state_text(entity: Option<&WorldSpatialEntity>) -> String {
    let Some(entity) = entity else {
        return "absent".into();
    };
    let health = match entity.detail {
        Detail::Actor { health_percent, .. } | Detail::ActorWithIcons { health_percent, .. } => {
            health_percent.to_string()
        }
        _ => "-".into(),
    };
    format!(
        "{},{},{}|{health}",
        entity.position.x, entity.position.y, entity.position.floor
    )
}

fn parse_target(text: &str) -> Result<EntityRef, BoxError> {
    let (identity, generation) = text.split_once(':').ok_or("target lacks generation")?;
    if identity.len() != 32 || !identity.is_ascii() {
        return Err("target identity must contain 32 hex digits".into());
    }
    let mut bytes = [0_u8; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&identity[index * 2..index * 2 + 2], 16)?;
    }
    Ok(EntityRef {
        identity: bytes,
        generation: generation.parse()?,
    })
}

fn fixture() -> Result<Fixture, BoxError> {
    // Inline fixture: the rat at (14, 10) drops to 40%, dies and leaves a corpse; a replacement
    // generation stands at the authored point after the respawn interval.
    let header = ActorPosition {
        x: 10,
        y: 10,
        floor: 0,
    };
    let at_x = |x| ActorPosition { x, y: 10, floor: 0 };
    let rat = |generation, health| {
        let mut entity = actor(EntityKind::Creature, 8, generation, at_x(14));
        entity.detail = Detail::Actor {
            direction: StepDirection::West,
            appearance_ref: 1,
            health_percent: health,
        };
        entity
    };
    let snapshot = |others: Vec<WorldSpatialEntity>| {
        let mut entities = vec![own(header)];
        entities.extend(others);
        encode_world_spatial_entities_snapshot(&WorldSpatialEntitiesSnapshot {
            content_generation: CONTENT_GENERATION,
            actor_position: header,
            own_identity: OWN,
            entities,
        })
        .map_err(|error| format!("{error:?}"))
    };
    let delta = |enter, update, leave| {
        encode_world_spatial_entities_delta(&entities_delta(header, enter, update, leave))
            .map_err(|error| format!("{error:?}"))
    };
    let dead = rat(1, 100).entity;
    let corpse_entity = corpse(9, at_x(14));
    let expected = format!(
        "kill.target={}:1\nkill.initial=14,10,0|100\nkill.trace=14,10,0|40;absent\n\
         corpse={}:0\ncorpse.position=14,10,0\nspawn=14,10,0\n\
         replacement={}:2\nreplacement.position=14,10,0\nkilled.absent=true\n",
        "08".repeat(16),
        "09".repeat(16),
        "08".repeat(16)
    );
    Ok(Fixture {
        kill_snapshot: snapshot(vec![rat(1, 100)])?,
        kill_deltas: vec![
            delta(vec![], vec![rat(1, 40)], vec![])?,
            delta(vec![corpse_entity], vec![], vec![dead])?,
        ],
        respawn_snapshot: snapshot(vec![corpse_entity, rat(2, 100)])?,
        expected,
    })
}

fn at_text(text: &str) -> Result<ActorPosition, BoxError> {
    let mut parts = text.split(',');
    let mut next = || parts.next().ok_or("position field absent");
    Ok(ActorPosition {
        x: next()?.parse()?,
        y: next()?.parse()?,
        floor: next()?.parse()?,
    })
}

fn recorded_sessions(tape: &[u8]) -> Result<Vec<&[u8]>, BoxError> {
    let mut starts = Vec::new();
    let mut offset = 0;
    while offset < tape.len() {
        let prefix = tape
            .get(offset..offset + 4)
            .ok_or("truncated frame prefix")?;
        let length = FrameLength::from_prefix(prefix)?.get() as usize;
        let frame = tape
            .get(offset + 4..offset + 4 + length)
            .ok_or("truncated frame")?;
        if decode_wire_envelope(frame)?.message_type() == MessageType::ServerAccepted {
            starts.push(offset);
        }
        offset += 4 + length;
    }
    assert_eq!(
        starts.len(),
        3,
        "expected staging, hunt and relog admissions"
    );
    assert_eq!(starts[0], 0);
    starts.push(tape.len());
    Ok(starts
        .windows(2)
        .map(|pair| &tape[pair[0]..pair[1]])
        .collect())
}

struct ReplayStream<'a>(&'a [u8]);

impl tokio::io::AsyncRead for ReplayStream<'_> {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
        buffer: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        let length = buffer.remaining().min(self.0.len());
        buffer.put_slice(&self.0[..length]);
        self.0 = &self.0[length..];
        std::task::Poll::Ready(Ok(()))
    }
}

impl tokio::io::AsyncWrite for ReplayStream<'_> {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
        bytes: &[u8],
    ) -> std::task::Poll<io::Result<usize>> {
        std::task::Poll::Ready(Ok(bytes.len()))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }
}

async fn replay_recording(expected: &str, tape: &[u8]) -> Result<(), BoxError> {
    fn id_text(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    let parts = recorded_sessions(tape)?;
    let target = parse_target(value(expected, "kill.target")?)?;
    let corpse = parse_target(value(expected, "corpse")?)?;
    let replacement = parse_target(value(expected, "replacement")?)?;
    let before: u64 = value(expected, "before")?.parse()?;
    let killed: u64 = value(expected, "killed")?.parse()?;
    let returned: u64 = value(expected, "returned")?.parse()?;
    let logout: u64 = value(expected, "logout.sequence")?.parse()?;
    assert!(before < killed && killed < returned && returned < logout);
    assert_ne!(replacement, target);
    let spawn = at_text(value(expected, "spawn")?)?;

    let mut session = Session::admit(ReplayStream(parts[1]), entity_admission()?).await?;
    let first_id = session.join_snapshot().game_session_id;
    assert_eq!(
        id_text(first_id.as_bytes()),
        value(expected, "first.session")?
    );
    let entry = at_text(value(expected, "joined.position")?)?;
    let saved = at_text(value(expected, "position")?)?;
    assert_eq!(session.world_spatial().actor_position, entry);
    assert_ne!(saved, entry);
    assert!(session.last_server_sequence() < before);
    let attack: u64 = value(expected, "attack.sequence")?.parse()?;
    assert!(before < attack && attack < killed);
    let mut attack_seen = false;
    let mut trace = Vec::new();
    let mut damaged = false;
    let mut removed = false;
    while session.last_server_sequence() < logout {
        let bytes = session.read_post_admission("W01b recorded frame").await?;
        let frame = decode_wire_envelope(&bytes)?;
        let sequence = session.next_sequence(&frame)?;
        match frame.message_type() {
            MessageType::CommandResult => {
                let result = decode_command_result(frame.payload())?;
                if sequence == attack {
                    use oteryn_protocol_oteryn::attack::{
                        AttackIntentDisposition, decode_attack_intent_result,
                    };
                    assert_eq!(result.status, CommandStatus::Accepted);
                    assert_eq!(
                        decode_attack_intent_result(result.payload)
                            .map_err(SessionError::Attack)?,
                        AttackIntentDisposition::Ok
                    );
                    attack_seen = true;
                }
                if sequence == logout {
                    assert_eq!(result.status, CommandStatus::Accepted);
                    assert_eq!(
                        logout::decode_logout_result(result.payload)
                            .map_err(SessionError::Logout)?,
                        logout::LogoutResult::Accepted
                    );
                }
            }
            MessageType::StateDelta => {
                assert_ne!(sequence, logout);
                let event = session.apply_state_delta(sequence, frame.payload())?;
                if sequence > before
                    && sequence <= killed
                    && let SessionEvent::WorldSpatialEntities(delta) = event
                {
                    let line = if delta.value.leave.contains(&target) {
                        removed = true;
                        "absent".to_owned()
                    } else if let Some(entity) = delta
                        .value
                        .enter
                        .iter()
                        .chain(&delta.value.update)
                        .find(|entity| entity.entity == target)
                    {
                        damaged |= matches!(
                            entity.detail,
                            Detail::Actor {
                                health_percent: 1..=99,
                                ..
                            } | Detail::ActorWithIcons {
                                health_percent: 1..=99,
                                ..
                            }
                        );
                        state_text(Some(entity))
                    } else {
                        "same".to_owned()
                    };
                    trace.push(line);
                }
            }
            unexpected => {
                return Err(format!("unexpected recorded W01b frame: {unexpected:?}").into());
            }
        }
        let view = session
            .world_entities()
            .ok_or("recorded entity view absent")?;
        if sequence == before {
            let initial_rat = view.get(&target).ok_or("original recorded Rat absent")?;
            assert_eq!(initial_rat.kind, EntityKind::Creature);
            assert_eq!(initial_rat.position, spawn);
            assert!(
                initial_rat
                    .display_name
                    .is_some_and(|name| name.as_str() == Ok("Rat"))
            );
            let initial = state_text(Some(initial_rat));
            assert_eq!(initial, value(expected, "kill.initial")?);
            assert!(initial.ends_with("|100"));
            assert!(view.get(&corpse).is_none() && view.get(&replacement).is_none());
        }
        if sequence == killed {
            assert!(damaged && removed && view.get(&target).is_none());
            let entry = view.get(&corpse).ok_or("recorded corpse absent")?;
            assert_eq!(entry.kind, EntityKind::Corpse);
            assert_eq!(
                entry.position,
                at_text(value(expected, "corpse.position")?)?
            );
        }
        if sequence == returned {
            let entry = view
                .get(&replacement)
                .ok_or("recorded replacement absent")?;
            assert_eq!(entry.kind, EntityKind::Creature);
            assert_eq!(entry.position, spawn);
            assert!(
                entry
                    .display_name
                    .is_some_and(|name| name.as_str() == Ok("Rat"))
            );
            assert!(view.get(&target).is_none());
        }
    }
    assert!(attack_seen && session.stream.0.is_empty());
    assert_eq!(session.world_spatial().actor_position, saved);
    assert_eq!(trace.join(";"), value(expected, "kill.trace")?);
    let mut fresh = Session::admit(ReplayStream(parts[2]), entity_admission()?).await?;
    let fresh_id = fresh.join_snapshot().game_session_id;
    assert_ne!(first_id, fresh_id);
    assert_eq!(
        id_text(fresh_id.as_bytes()),
        value(expected, "fresh.session")?
    );
    assert_eq!(fresh.world_spatial().actor_position, saved);
    let fresh_sequence: u64 = value(expected, "fresh.sequence")?.parse()?;
    assert!(fresh_sequence > fresh.last_server_sequence());
    let view = fresh
        .world_entities()
        .ok_or("fresh recorded entity view absent")?;
    assert!(view.get(&target).is_none() && view.get(&replacement).is_none());
    while fresh.last_server_sequence() < fresh_sequence {
        let bytes = fresh
            .read_post_admission("W01b recorded relog walk")
            .await?;
        let frame = decode_wire_envelope(&bytes)?;
        let sequence = fresh.next_sequence(&frame)?;
        match frame.message_type() {
            MessageType::CommandResult => {
                decode_command_result(frame.payload())?;
            }
            MessageType::StateDelta => {
                fresh.apply_state_delta(sequence, frame.payload())?;
            }
            unexpected => return Err(format!("unexpected W01b relog frame: {unexpected:?}").into()),
        }
    }
    assert_eq!(fresh.last_server_sequence(), fresh_sequence);
    while !fresh.stream.0.is_empty() {
        let bytes = read_frame(&mut fresh.stream).await?;
        let frame = decode_wire_envelope(&bytes)?;
        frame.validate(Direction::ServerToClient, true)?;
        fresh.check_generation(&frame)?;
        assert_eq!(frame.message_type(), MessageType::LivenessProbe);
        fresh.answer_probe(frame.payload()).await?;
    }
    let view = fresh
        .world_entities()
        .ok_or("fresh recorded entity view absent")?;
    assert!(view.get(&target).is_none());
    let replacement = view
        .get(&replacement)
        .ok_or("fresh recorded replacement absent")?;
    assert_eq!(replacement.kind, EntityKind::Creature);
    assert!(
        replacement
            .display_name
            .is_some_and(|name| name.as_str() == Ok("Rat"))
    );
    Ok(())
}

#[test]
#[ignore = "requires original frames from the live node-boot row"]
fn e2e_w01b_rat_kill_and_respawn_replay() -> Result<(), BoxError> {
    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("W01b capture directory absent")?;
    let root = std::path::Path::new(&root);
    let expected = std::fs::read_to_string(root.join("w01b-expected.txt"))?;
    let tape = std::fs::read(root.join("w01b-server-frames.bin"))?;
    block_on(replay_recording(&expected, &tape))?
}

#[test]
fn synthetic_w01b_entity_reducer_regression() -> Result<(), BoxError> {
    let fixture = fixture()?;
    let expected = &fixture.expected;
    let target = parse_target(value(expected, "kill.target")?)?;
    let corpse_ref = parse_target(value(expected, "corpse")?)?;
    let replacement = parse_target(value(expected, "replacement")?)?;
    let spawn = at_text(value(expected, "spawn")?)?;
    block_on(async {
        // Kill: the damage delta, then the removal with the corpse.
        let trace: Vec<&str> = value(expected, "kill.trace")?.split(';').collect();
        assert_eq!(trace.len(), fixture.kill_deltas.len());
        let (client, peer) = entity_peer(
            (
                SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                fixture.kill_snapshot.clone(),
            ),
            &[6],
            vec![],
        );
        let mut session = Session::admit(client, entity_admission()?).await?;
        assert_eq!(
            state_text(
                session
                    .world_entities()
                    .ok_or("capability 6 selected")?
                    .get(&target)
            ),
            value(expected, "kill.initial")?
        );
        let mut revision = session.join_snapshot().world_spatial_revision;
        let mut last = value(expected, "kill.initial")?.to_owned();
        for (index, (bytes, line)) in (0_u64..).zip(fixture.kill_deltas.iter().zip(&trace)) {
            let frame = encode_state_delta(
                1,
                41 + index,
                STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision,
                revision + 1,
                DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                bytes,
            )?;
            session.apply_state_delta(41 + index, decode_wire_envelope(&frame)?.payload())?;
            revision += 1;
            if *line != "same" {
                last = (*line).to_owned();
            }
            assert_eq!(
                state_text(
                    session
                        .world_entities()
                        .ok_or("capability 6 selected")?
                        .get(&target)
                ),
                last
            );
        }
        let view = session.world_entities().ok_or("capability 6 selected")?;
        assert!(view.get(&target).is_none());
        let corpse = view
            .get(&corpse_ref)
            .ok_or("corpse absent after the kill")?;
        assert_eq!(corpse.kind, EntityKind::Corpse);
        assert_eq!(
            corpse.position,
            at_text(value(expected, "corpse.position")?)?
        );

        // A garbage entity payload is refused whole: the view stays as applied.
        let before: Vec<_> = view.iter().copied().collect();
        let frame = encode_state_delta(
            1,
            99,
            STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            revision,
            revision + 1,
            DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2,
            &[0xff],
        )?;
        assert!(
            session
                .apply_state_delta(99, decode_wire_envelope(&frame)?.payload())
                .is_err()
        );
        let after: Vec<_> = session
            .world_entities()
            .ok_or("capability 6 selected")?
            .iter()
            .copied()
            .collect();
        assert_eq!(after, before);
        drop(session);
        peer.await??;

        // Respawn: a later admission holds the replacement at the authored point and not the
        // killed generation.
        let (client, peer) = entity_peer(
            (
                SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                fixture.respawn_snapshot.clone(),
            ),
            &[6],
            vec![],
        );
        let session = Session::admit(client, entity_admission()?).await?;
        let view = session.world_entities().ok_or("capability 6 selected")?;
        let fresh = view.get(&replacement).ok_or("replacement absent")?;
        assert_eq!(fresh.kind, EntityKind::Creature);
        assert_eq!(fresh.position, spawn);
        assert_ne!(replacement, target);
        assert_eq!(value(expected, "killed.absent")?, "true");
        assert!(view.get(&target).is_none());
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[cfg(feature = "test-support")]
#[test]
fn w01b_capture_retains_original_frame_prefixes_and_bodies() -> Result<(), BoxError> {
    let wire = [0, 0, 0, 2, 0x91, 0x11, 0, 0, 0, 1, 0x80];
    block_on(async {
        let mut reader = ReplayStream(&wire);
        let (result, capture) = capture_server_frames(async {
            assert_eq!(read_frame(&mut reader).await?, [0x91, 0x11]);
            let byte = reader.read_u8().await?;
            assert_eq!(
                crate::framing::read_frame_after(&mut reader, byte).await?,
                [0x80]
            );
            Ok::<(), SessionError>(())
        })
        .await;
        result?;
        assert_eq!(capture.as_deref(), Some(wire.as_slice()));
        Ok(())
    })?
}
