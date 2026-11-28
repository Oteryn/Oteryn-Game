//! W20a client replay: the natural World clock snapshot and two raw clock deltas (domain 18) go
//! through `Session::admit` and `Session::apply_state_delta` (`features/0019-world-light/protocol.rs`);
//! the applied `SessionEvent::WorldClock` values are the server-observed minute, light level and
//! colour. A stale replay and a malformed payload are refused and leave the revision unchanged.
use super::*;
use crate::tests::{BoxError, admission_with, join_peer_with};
use oteryn_protocol_oteryn::encode_state_delta;
use oteryn_protocol_oteryn::world_clock::{
    STATE_DOMAIN_WORLD_CLOCK, WorldClock, decode_world_clock, encode_world_clock,
};

struct Capture {
    snapshot: Vec<u8>,
    delta: [Vec<u8>; 2],
    expected: String,
}

fn fixture() -> Result<Capture, BoxError> {
    let clock = |tibian_minute, light_level| {
        encode_world_clock(WorldClock {
            tibian_minute,
            light_level,
            light_color: 215,
        })
        .map_err(|error| BoxError::from(format!("{error:?}")))
    };
    Ok(Capture {
        snapshot: clock(366, 54)?,
        delta: [clock(370, 61)?, clock(374, 68)?],
        expected: "snapshot_revision=4\nsnapshot=366:54:215\ndelta1=370:61:215\ndelta1_sequence=41\n\
                   delta1_base=4\ndelta1_new=5\ndelta2=374:68:215\ndelta2_sequence=42\ndelta2_base=5\n\
                   delta2_new=6\n"
            .into(),
    })
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, BoxError> {
    expected
        .lines()
        .find_map(|line| {
            line.strip_prefix(key)
                .and_then(|rest| rest.strip_prefix('='))
        })
        .ok_or_else(|| format!("w20a expected lacks {key}").into())
}

fn text(clock: WorldClock) -> String {
    format!(
        "{}:{}:{}",
        clock.tibian_minute, clock.light_level, clock.light_color
    )
}

async fn replay_live(directory: &std::path::Path) -> Result<(), BoxError> {
    let transcript = std::fs::read(directory.join("w20a-inbound.bin"))?;
    let expected = std::fs::read_to_string(directory.join("w20a-expected.txt"))?;
    let (idle, _peer) = tokio::io::duplex(1);
    // Keep the recorded connection open after its last frame so the idle reader can
    // return its queued events at the ordinary idle deadline.
    let inbound = tokio::io::AsyncReadExt::chain(transcript.as_slice(), idle);
    let stream = tokio::io::join(inbound, tokio::io::sink());
    let mut session =
        Session::admit(stream, admission_with(CLIENT_SUPPORTED_CAPABILITIES)?).await?;
    let snapshot = session
        .gated_snapshot(STATE_DOMAIN_WORLD_CLOCK)
        .ok_or("clock snapshot absent")?;
    assert_eq!(
        snapshot.revision,
        value(&expected, "snapshot_revision")?.parse::<u64>()?
    );
    assert_eq!(
        text(decode_world_clock(&snapshot.payload).map_err(|error| format!("{error:?}"))?),
        value(&expected, "snapshot")?
    );
    let mut observed = 0;
    while observed < 2 {
        let events = session.wait_for_events(Duration::from_secs(1)).await?;
        for event in events {
            let SessionEvent::WorldClock(delta) = event else {
                continue;
            };
            observed += 1;
            let key = |suffix: &str| format!("delta{observed}{suffix}");
            assert_eq!(text(delta.value), value(&expected, &key(""))?);
            assert_eq!(
                delta.server_sequence,
                value(&expected, &key("_sequence"))?.parse::<u64>()?
            );
            assert_eq!(
                delta.base_revision,
                value(&expected, &key("_base"))?.parse::<u64>()?
            );
            assert_eq!(
                delta.new_revision,
                value(&expected, &key("_new"))?.parse::<u64>()?
            );
        }
    }
    assert_eq!(observed, 2);
    let first_sequence = value(&expected, "delta1_sequence")?.parse::<u64>()?;
    let last_revision = value(&expected, "delta2_new")?.parse::<u64>()?;
    let mut frames = transcript.as_slice();
    let stale = loop {
        let frame = read_frame(&mut frames).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::StateDelta
            && envelope.server_sequence() == first_sequence
        {
            assert_eq!(
                decode_state_delta(envelope.payload())?.domain_id,
                STATE_DOMAIN_WORLD_CLOCK
            );
            break frame;
        }
    };
    let revision = |session: &Session<_>| {
        session
            .gated_revisions
            .iter()
            .find(|(domain, _)| *domain == STATE_DOMAIN_WORLD_CLOCK)
            .map(|(_, revision)| *revision)
    };
    assert_eq!(revision(&session), Some(last_revision));
    assert!(matches!(
        session.apply_state_delta(first_sequence, decode_wire_envelope(&stale)?.payload()),
        Err(SessionError::StateRevisionMismatch { .. })
    ));
    assert_eq!(revision(&session), Some(last_revision));

    // This is a negative probe after replaying the unmodified transcript.
    let next_sequence = session.last_server_sequence() + 1;
    let malformed = encode_state_delta(
        session.connection_generation,
        next_sequence,
        STATE_DOMAIN_WORLD_CLOCK,
        last_revision,
        last_revision + 1,
        1,
        &[0xff],
    )?;
    assert!(matches!(
        session.apply_state_delta(next_sequence, decode_wire_envelope(&malformed)?.payload()),
        Err(SessionError::Protocol(
            FoundationProtocolError::MalformedFrame
        ))
    ));
    assert_eq!(revision(&session), Some(last_revision));
    Ok(())
}

#[test]
fn e2e_w20a_world_clock_replay() -> Result<(), BoxError> {
    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        return block_on(replay_live(std::path::Path::new(&path)))?;
    }
    block_on(async {
        let capture = fixture()?;
        let expected = capture.expected.as_str();
        let revision: u64 = value(expected, "snapshot_revision")?.parse()?;
        let snapshot = capture.snapshot.clone();
        let (client, mut server) = tokio::io::duplex(64 * 1024);
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                &[6, 19],
                &[6, 19],
                &[],
                entity_snapshot(vec![]),
                &[(STATE_DOMAIN_WORLD_CLOCK, revision, 1, snapshot)],
            )
            .await?;
            Ok::<_, BoxError>(())
        });
        let mut session = Session::admit(client, admission_with(&[6, 19])?).await?;
        peer.await??;
        let admitted = session
            .gated_snapshot(STATE_DOMAIN_WORLD_CLOCK)
            .ok_or("clock snapshot absent")?;
        assert_eq!(admitted.revision, revision);
        assert_eq!(
            text(decode_world_clock(&admitted.payload).map_err(|e| format!("{e:?}"))?),
            value(expected, "snapshot")?
        );
        let mut frames = Vec::new();
        for (index, payload) in capture.delta.iter().enumerate() {
            let key = |suffix: &str| format!("delta{}{suffix}", index + 1);
            let sequence: u64 = value(expected, &key("_sequence"))?.parse()?;
            let base: u64 = value(expected, &key("_base"))?.parse()?;
            let new: u64 = value(expected, &key("_new"))?.parse()?;
            let frame =
                encode_state_delta(1, sequence, STATE_DOMAIN_WORLD_CLOCK, base, new, 1, payload)?;
            let event =
                session.apply_state_delta(sequence, decode_wire_envelope(&frame)?.payload())?;
            let SessionEvent::WorldClock(delta) = event else {
                return Err("clock delta did not apply as a WorldClock event".into());
            };
            assert_eq!(
                text(delta.value),
                value(expected, &format!("delta{}", index + 1))?
            );
            assert_eq!((delta.base_revision, delta.new_revision), (base, new));
            frames.push((sequence, frame, new));
        }
        // A stale replay of the first delta and a malformed payload change nothing.
        let (sequence, frame, _) = &frames[0];
        assert!(
            session
                .apply_state_delta(*sequence, decode_wire_envelope(frame)?.payload())
                .is_err()
        );
        let last = frames[1].2;
        let bad = encode_state_delta(
            1,
            frames[1].0 + 1,
            STATE_DOMAIN_WORLD_CLOCK,
            last,
            last + 1,
            1,
            &[0xff],
        )?;
        assert!(
            session
                .apply_state_delta(frames[1].0 + 1, decode_wire_envelope(&bad)?.payload())
                .is_err()
        );
        assert_eq!(
            session
                .gated_revisions
                .iter()
                .find(|(id, _)| *id == STATE_DOMAIN_WORLD_CLOCK)
                .map(|(_, revision)| *revision),
            Some(last)
        );
        Ok(())
    })?
}

#[test]
fn qualification_capture_preserves_original_frame_bytes() -> Result<(), BoxError> {
    block_on(async {
        let mut frame = Vec::new();
        write_frame(&mut frame, &[0x08, 0x13]).await?;
        let mut body_after_first = &frame[1..];
        let mut complete_frame = frame.as_slice();
        let (result, transcript) = capture_inbound_frames(async {
            assert_eq!(read_frame(&mut complete_frame).await?, [0x08, 0x13]);
            assert_eq!(
                read_frame_after(&mut body_after_first, frame[0]).await?,
                [0x08, 0x13]
            );
            Ok::<_, BoxError>(())
        })
        .await;
        result?;
        assert_eq!(transcript, [frame.as_slice(), frame.as_slice()].concat());
        Ok(())
    })?
}
