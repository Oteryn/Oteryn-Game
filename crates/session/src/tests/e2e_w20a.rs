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
    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = std::path::Path::new(&path);
        return Ok(Capture {
            snapshot: std::fs::read(path.join("w20a-snapshot.bin"))?,
            delta: [
                std::fs::read(path.join("w20a-delta1.bin"))?,
                std::fs::read(path.join("w20a-delta2.bin"))?,
            ],
            expected: std::fs::read_to_string(path.join("w20a-expected.txt"))?,
        });
    }
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

#[test]
fn e2e_w20a_world_clock_replay() -> Result<(), BoxError> {
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
