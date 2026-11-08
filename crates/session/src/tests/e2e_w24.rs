//! W24 client replay: the private ambient-sound domain (101, capability 288) goes through
//! `Session::apply_state_delta` (`presentation.rs`). The outside cue 0 and the city replacement cue
//! are session-owned presentation batches; the row re-encodes them with the production
//! `encode_presentation_batch` (the C17c practice). A malformed batch and a stale replay are
//! refused and leave the revision and the queue unchanged.
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
    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = std::path::Path::new(&path);
        return Ok(Capture {
            outside: std::fs::read(path.join("w24-outside.bin"))?,
            city: std::fs::read(path.join("w24-city.bin"))?,
            expected: std::fs::read_to_string(path.join("w24-expected.txt"))?,
        });
    }
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

#[test]
fn e2e_w24_ambient_zone_sound_replay() -> Result<(), BoxError> {
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
        // A stale replay and a malformed batch change nothing.
        assert!(apply(&mut session, 43, 1, 2, &capture.city).is_err());
        assert!(apply(&mut session, 44, 2, 3, &[0xff]).is_err());
        assert_eq!(
            session
                .gated_revisions
                .iter()
                .find(|(id, _)| *id == STATE_DOMAIN_AMBIENT_SOUND)
                .map(|(_, revision)| *revision),
            Some(2)
        );
        assert!(session.take_events().is_empty());
        drop(session);
        peer.await??;
        Ok(())
    })?
}
