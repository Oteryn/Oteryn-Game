//! Bounded, test-only reuse of an immutable, successfully decoded catalogue.
//!
//! The compiler validates an envelope and activation stages those same bytes again
//! (native_gameplay.rs::compile_native_gameplay; production.rs::StagedGeneration::stage).
//! Keep the real decoder on every miss. Exact whole-envelope equality includes the
//! baseline World binding, every section header/hash, version and trailing bytes.
//! No activation controller, live actor, authorization or mutable session is cached.
use super::super::{ContentError, DecodedNativeGameplay, MAGIC, NativeGameplayState};
use std::sync::Mutex;

// Two immutable entries cover the initial/successor pair exercised by cutovers.
// Sharing spans fixture worker/restart threads, without retaining runtime owners.
static LAST: Mutex<Vec<(usize, NativeGameplayState)>> = Mutex::new(Vec::new());

pub(in crate::content::native_gameplay) fn decode(
    bytes: &[u8],
) -> Result<DecodedNativeGameplay<'_>, ContentError> {
    let started = std::time::Instant::now();
    let enabled =
        std::env::var_os("OTERYN_TEST_FIXTURE_CACHE").as_deref() != Some(std::ffi::OsStr::new("0"));
    if enabled {
        let retained = {
            let last = LAST.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            last.iter()
                .find(|(_, state)| state.encoded.as_ref() == bytes)
                .map(|(length, state)| (*length, state.clone()))
        };
        if let Some((length, state)) = retained {
            let start = MAGIC.len() + 36;
            let baseline = bytes
                .get(start..start + length)
                .ok_or(ContentError::InvalidSectionBounds)?;
            trace(started, true, bytes.len());
            return Ok(DecodedNativeGameplay { baseline, state });
        }
    }
    let decoded = super::super::decode_uncached(bytes)?;
    if enabled {
        let mut last = LAST.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if last.len() == 2 {
            last.remove(0);
        }
        last.push((decoded.baseline.len(), decoded.state.clone()));
    }
    trace(started, false, bytes.len());
    Ok(decoded)
}

fn trace(started: std::time::Instant, cached: bool, bytes: usize) {
    if std::env::var_os("OTERYN_TEST_TIMINGS").is_some() {
        eprintln!(
            "PGSPEED phase=native_decode cached={cached} bytes={bytes} elapsed={:?}",
            started.elapsed()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_envelope_cache_preserves_decode_and_rejects_mutation_and_trailing_bytes()
    -> Result<(), Box<dyn std::error::Error>> {
        let world = crate::content::production::test_source(1)?.world_id;
        let baseline = crate::content::qualify_native_entry_room(world)?;
        let input = super::super::input();
        let artifact = super::super::super::compile_native_gameplay(baseline.compiled(), &input)?;
        let bytes = &artifact.server_artifact;
        let expected = super::super::super::decode_uncached(bytes)?;
        for _ in 0..2 {
            let actual = decode(bytes)?;
            assert_eq!(actual.baseline, expected.baseline);
            assert_eq!(actual.state, expected.state);
        }
        let mut corrupted = bytes.clone();
        *corrupted.last_mut().ok_or("empty fixture")? ^= 1;
        assert!(matches!(
            decode(&corrupted),
            Err(ContentError::RevisionMismatch(_))
        ));
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(matches!(
            decode(&trailing),
            Err(ContentError::InvalidArtifact(
                "native gameplay trailing or nested bytes"
            ))
        ));
        let actual = decode(bytes)?;
        assert_eq!(
            actual.state, expected.state,
            "failed decodes cannot poison the cache"
        );
        let mut second_world = *world.as_bytes();
        second_world[15] ^= 1;
        let second_world = oteryn_protocol_oteryn::WorldId::decode(&second_world)?;
        let second_baseline = crate::content::qualify_native_entry_room(second_world)?;
        let second =
            super::super::super::compile_native_gameplay(second_baseline.compiled(), &input)?;
        assert_ne!(
            bytes, &second.server_artifact,
            "the baseline World binding is part of the cache identity"
        );
        let expected_second = super::super::super::decode_uncached(&second.server_artifact)?;
        let actual_second = decode(&second.server_artifact)?;
        assert_eq!(actual_second.baseline, expected_second.baseline);
        assert_eq!(actual_second.state, expected_second.state);
        assert_eq!(
            decode(bytes)?.state,
            expected.state,
            "switching Worlds cannot retain another World's catalogue"
        );
        Ok(())
    }
}
