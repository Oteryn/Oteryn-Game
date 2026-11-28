//! W24 replays captured admission and ambient envelopes, preserving wire metadata.
use super::*;
use crate::tests::{BoxError, join_peer_with};
use oteryn_protocol_oteryn::encode_state_delta;
use oteryn_protocol_oteryn::presentation::{
    PresentationEvent, PresentationEventKind, STATE_DOMAIN_AMBIENT_SOUND,
    decode_presentation_batch, encode_presentation_batch,
};

struct Capture {
    outside: Vec<u8>,
    city: Vec<u8>,
    expected: String,
}

fn fixture() -> Result<Capture, BoxError> {
    let batch = |cue_id| {
        encode_presentation_batch(&[PresentationEvent {
            decision_ordinal: 1,
            emission_ordinal: 1,
            kind: PresentationEventKind::AmbientSound { cue_id },
        }])
        .map_err(|error| BoxError::from(format!("{error:?}")))
    };
    Ok(Capture {
        outside: batch(0)?,
        city: batch(25)?,
        expected: "outside=0\ncity=25\n".into(),
    })
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, BoxError> {
    expected
        .lines()
        .find_map(|line| {
            line.strip_prefix(key)
                .and_then(|rest| rest.strip_prefix('='))
        })
        .ok_or_else(|| format!("w24 expected lacks {key}").into())
}

fn cue_of(event: SessionEvent) -> Result<u32, BoxError> {
    let SessionEvent::Presentation(delta) = event else {
        return Err("ambient delta did not apply as a Presentation event".into());
    };
    match delta.value.as_slice() {
        [
            PresentationEvent {
                kind: PresentationEventKind::AmbientSound { cue_id },
                ..
            },
        ] => Ok(*cue_id),
        other => Err(format!("unexpected ambient batch {other:?}").into()),
    }
}

type Cue = (u32, u64);
type ReplayStream = tokio::io::Join<std::io::Cursor<Vec<u8>>, tokio::io::Sink>;

fn cue_record(line: &str) -> Result<Cue, BoxError> {
    let (cue, sequence) = line.split_once(' ').ok_or("invalid W24 cue record")?;
    Ok((cue.parse()?, sequence.parse()?))
}

fn observed_cue(event: SessionEvent) -> Result<Option<Cue>, BoxError> {
    match &event {
        SessionEvent::Presentation(delta)
            if delta
                .value
                .iter()
                .any(|event| matches!(event.kind, PresentationEventKind::AmbientSound { .. })) =>
        {
            let sequence = delta.server_sequence;
            Ok(Some((cue_of(event)?, sequence)))
        }
        _ => Ok(None),
    }
}

fn drain_cues(session: &mut Session<ReplayStream>) -> Result<Vec<Cue>, BoxError> {
    session
        .take_events()
        .into_iter()
        .try_fold(Vec::new(), |mut cues, event| {
            cues.extend(observed_cue(event)?);
            Ok(cues)
        })
}

async fn read_cue(session: &mut Session<ReplayStream>, expected: Cue) -> Result<(), BoxError> {
    let mut cues = drain_cues(session)?;
    while session.last_server_sequence() < expected.1 {
        let (kind, sequence, payload) = session.read_sequenced_push("W24 ambient").await?;
        assert_eq!(kind, MessageType::StateDelta);
        cues.extend(observed_cue(
            session.apply_state_delta(sequence, &payload)?,
        )?);
    }
    require_cue(&cues, expected)
}

fn require_cue(cues: &[Cue], expected: Cue) -> Result<(), BoxError> {
    if cues == [expected] {
        Ok(())
    } else {
        Err(format!("expected W24 cue {expected:?}, observed {cues:?}").into())
    }
}

fn require_after_crossing(expected: Cue, spatial_sequence: u64) -> Result<(), BoxError> {
    if expected.1 > spatial_sequence {
        Ok(())
    } else {
        Err("W24 cue preceded the boundary spatial delta".into())
    }
}

fn require_no_cues(cues: &[Cue], phase: &str) -> Result<(), BoxError> {
    if cues.is_empty() {
        Ok(())
    } else {
        Err(format!("unexpected W24 cues during {phase}: {cues:?}").into())
    }
}

fn require_ordinary_step(
    from: ActorPosition,
    to: ActorPosition,
    direction: StepDirection,
) -> Result<(), BoxError> {
    let (dx, dy) = direction.delta();
    if from.x.checked_add(dx) == Some(to.x)
        && from.y.checked_add(dy) == Some(to.y)
        && from.floor == to.floor
    {
        Ok(())
    } else {
        Err("W24 step landed beyond the adjacent tile".into())
    }
}

#[test]
fn w24_rejects_false_boundary_evidence() -> Result<(), BoxError> {
    let from = ActorPosition {
        x: 10,
        y: 10,
        floor: 7,
    };
    let adjacent = ActorPosition { x: 11, ..from };
    let teleported = ActorPosition { x: 50, ..from };
    require_ordinary_step(from, adjacent, StepDirection::East)?;
    assert!(require_ordinary_step(from, teleported, StepDirection::East).is_err());

    require_after_crossing((25, 12), 11)?;
    assert!(require_after_crossing((25, 9), 11).is_err());
    assert!(require_after_crossing((25, 11), 11).is_err());

    require_cue(&[(0, 12)], (0, 12))?;
    assert!(require_cue(&[], (0, 12)).is_err());
    assert!(require_cue(&[(25, 12)], (0, 12)).is_err());

    require_cue(&[(28, 2)], (28, 2))?;
    assert!(require_cue(&[], (28, 2)).is_err());
    assert!(require_cue(&[(25, 2)], (28, 2)).is_err());

    require_no_cues(&[], "logout")?;
    assert!(require_no_cues(&[(0, 13)], "logout").is_err());
    Ok(())
}

async fn replay_connection(
    root: &std::path::Path,
    name: &str,
    initial_silent: bool,
    crossing_silent: Option<bool>,
) -> Result<(String, ActorPosition, ActorPosition), BoxError> {
    let metadata = std::fs::read_to_string(root.join(format!("{name}.txt")))?;
    let mut lines = metadata.lines();
    let id = lines.next().ok_or("missing replay session ID")?;
    let initial = cue_record(lines.next().ok_or("missing admission cue")?)?;
    let crossing = lines.next().ok_or("missing crossing record")?;
    assert!(lines.next().is_none());
    assert_eq!(initial.0 == 0, initial_silent);
    let bytes = std::fs::read(root.join(format!("{name}.bin")))?;
    let stream = tokio::io::join(std::io::Cursor::new(bytes), tokio::io::sink());
    let mut session =
        Session::admit(stream, admission_with(CLIENT_SUPPORTED_CAPABILITIES)?).await?;
    assert_eq!(format!("{:?}", session.join_snapshot().game_session_id), id);
    let before = session.world_spatial().actor_position;
    read_cue(&mut session, initial).await?;
    if let Some(silent) = crossing_silent {
        let (path, cue) = crossing.split_once(' ').ok_or("missing walking route")?;
        let expected = cue_record(cue)?;
        assert_eq!(expected.0 == 0, silent);
        let mut directions = path.chars().peekable();
        let mut crossing_sequence = None;
        while let Some(letter) = directions.next() {
            let direction = match letter {
                'N' => StepDirection::North,
                'E' => StepDirection::East,
                'S' => StepDirection::South,
                'W' => StepDirection::West,
                _ => return Err("invalid W24 replay direction".into()),
            };
            let from = session.world_spatial().actor_position;
            let reply = session.step(direction).await?;
            assert_eq!(reply.disposition, StepDisposition::Moved);
            let delta = reply
                .world_spatial_delta
                .ok_or("missing replay spatial delta")?;
            assert!(delta.server_sequence > reply.result_server_sequence);
            require_ordinary_step(from, delta.value.actor_position, direction)?;
            if directions.peek().is_some() {
                assert!(
                    drain_cues(&mut session)?.is_empty(),
                    "ambient before crossing"
                );
            } else {
                crossing_sequence = Some(delta.server_sequence);
            }
        }
        require_after_crossing(expected, crossing_sequence.ok_or("missing boundary STEP")?)?;
        read_cue(&mut session, expected).await?;
    } else {
        assert_eq!(crossing, "-");
    }
    let after = session.world_spatial().actor_position;
    assert_eq!(
        session.logout().await?.result,
        Some(logout::LogoutResult::Accepted)
    );
    assert!(
        drain_cues(&mut session)?.is_empty(),
        "ambient during logout"
    );
    assert_eq!(
        session.stream.read(&mut [0]).await?,
        0,
        "trailing data after logout"
    );
    assert_invalid_ambient_unchanged(&mut session)?;
    Ok((id.to_owned(), before, after))
}

async fn replay_live(path: &std::path::Path) -> Result<(), BoxError> {
    let origin = std::fs::read_to_string(path.join("w24-expected.txt"))?;
    let (entry_id, outside, inside) =
        replay_connection(path, "w24-entry", true, Some(false)).await?;
    match origin.as_str() {
        "exit" => {
            let (exit_id, _, exit_position) =
                replay_connection(path, "w24-exit", false, Some(true)).await?;
            assert_ne!(exit_id, entry_id);
            assert_eq!(exit_position, outside);
        }
        "outside" => {
            assert!(!path.join("w24-exit.bin").exists());
            assert!(!path.join("w24-exit.txt").exists());
        }
        _ => return Err("invalid W24 origin".into()),
    }
    let (relog_id, relog_position, _) = replay_connection(path, "w24-relog", false, None).await?;
    assert_ne!(relog_id, entry_id);
    assert_ne!(outside, inside);
    assert_eq!(relog_position, inside);
    Ok(())
}

fn assert_invalid_ambient_unchanged<S: SessionStream>(
    session: &mut Session<S>,
) -> Result<(), BoxError> {
    let domain = STATE_DOMAIN_AMBIENT_SOUND;
    let revision = session
        .gated_revisions
        .iter()
        .find(|(id, _)| *id == domain)
        .map(|(_, revision)| *revision)
        .ok_or("missing ambient revision")?;
    let batch = fixture()?.city;
    for (base, new, payload) in [
        (revision - 1, revision, batch.as_slice()),
        (revision, revision + 1, &[0xff]),
    ] {
        let sequence = session.last_server_sequence() + 1;
        let frame = encode_state_delta(
            session.connection_generation,
            sequence,
            domain,
            base,
            new,
            1,
            payload,
        )?;
        assert!(
            session
                .apply_state_delta(sequence, decode_wire_envelope(&frame)?.payload())
                .is_err()
        );
        assert_eq!(
            session.gated_revisions.iter().find(|(id, _)| *id == domain),
            Some(&(domain, revision))
        );
        assert!(session.take_events().is_empty());
    }
    Ok(())
}

#[test]
fn e2e_w24_ambient_zone_sound_replay() -> Result<(), BoxError> {
    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        return block_on(replay_live(std::path::Path::new(&path)))?;
    }
    block_on(async {
        let capture = fixture()?;
        let expected = capture.expected.as_str();
        let (client, mut server) = tokio::io::duplex(64 * 1024);
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                &[6, 288],
                &[6, 288],
                &[],
                entity_snapshot(vec![]),
                &[(STATE_DOMAIN_AMBIENT_SOUND, 0, 1, Vec::new())],
            )
            .await?;
            let mut rest = Vec::new();
            let _ = server.read_to_end(&mut rest).await;
            Ok::<_, BoxError>(())
        });
        let mut session = Session::admit(client, admission_with(&[6, 288])?).await?;
        let apply = |session: &mut Session<_>, sequence, base, new, payload: &[u8]| {
            let frame = encode_state_delta(
                1,
                sequence,
                STATE_DOMAIN_AMBIENT_SOUND,
                base,
                new,
                1,
                payload,
            )?;
            session
                .apply_state_delta(sequence, decode_wire_envelope(&frame)?.payload())
                .map_err(BoxError::from)
        };
        let outside = cue_of(apply(&mut session, 41, 0, 1, &capture.outside)?)?;
        assert_eq!(outside.to_string(), value(expected, "outside")?);
        let city = cue_of(apply(&mut session, 42, 1, 2, &capture.city)?)?;
        assert_eq!(city.to_string(), value(expected, "city")?);
        assert_ne!(city, outside);
        assert_eq!(
            decode_presentation_batch(&capture.city)
                .map_err(|e| format!("{e:?}"))?
                .len(),
            1
        );
        assert_invalid_ambient_unchanged(&mut session)?;
        drop(session);
        peer.await??;
        Ok(())
    })?
}
