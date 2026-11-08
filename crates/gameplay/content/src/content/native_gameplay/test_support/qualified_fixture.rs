//! Immutable qualification reuse only: every caller still creates and seals its own
//! controller in activation_fixture.rs::try_activate_input. In particular a restart
//! never inherits an active generation, session, admission or durable authority.
use super::super::NativeGameplayInput;
use crate::content::{ProjectError, QualifiedNativeEntryRoom};
use oteryn_protocol_oteryn::WorldId;
use std::sync::Mutex;

type Entry = (WorldId, NativeGameplayInput, QualifiedNativeEntryRoom);
static QUALIFIED: Mutex<Vec<Entry>> = Mutex::new(Vec::new());

pub(super) fn qualify(
    world: WorldId,
    input: &NativeGameplayInput,
) -> Result<QualifiedNativeEntryRoom, ProjectError> {
    let started = std::time::Instant::now();
    let enabled =
        std::env::var_os("OTERYN_TEST_FIXTURE_CACHE").as_deref() != Some(std::ffi::OsStr::new("0"));
    if enabled {
        let retained = {
            let entries = QUALIFIED
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            entries.iter()
                // Derived equality covers all input fields, including actual bytes
                // AND declared hashes, optional sections, revision and map profile.
                .find(|(found_world, found_input, _)| *found_world == world && found_input == input)
                .map(|(_, _, room)| room.clone())
        };
        if let Some(room) = retained {
            trace(started, true);
            return Ok(room);
        }
    }
    let room = crate::content::qualify_selected_native_gameplay_room(world, input)?;
    if enabled {
        let mut entries = QUALIFIED
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if entries.len() == 2 {
            entries.remove(0);
        }
        entries.push((world, input.clone(), room.clone()));
    }
    trace(started, false);
    Ok(room)
}

fn trace(started: std::time::Instant, cached: bool) {
    if std::env::var_os("OTERYN_TEST_TIMINGS").is_some() {
        eprintln!(
            "PGSPEED phase=fixture_qualification cached={cached} elapsed={:?}",
            started.elapsed()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualified_fixture_keeps_world_and_exact_input_binding_and_rejects_invalid_pins()
    -> Result<(), Box<dyn std::error::Error>> {
        let world = crate::content::production::test_source(1)?.world_id;
        let input = super::super::input();
        let expected = crate::content::qualify_selected_native_gameplay_room(world, &input)?;
        assert_eq!(qualify(world, &input)?, expected);
        assert_eq!(qualify(world, &input)?, expected);
        let mut wrong_pin = input.clone();
        wrong_pin.catalog.sha256 = "0".repeat(64);
        assert!(
            qualify(world, &wrong_pin).is_err(),
            "cached bytes cannot bless a changed declared pin"
        );
        let mut wrong_bytes = input.clone();
        wrong_bytes.catalog.bytes.push(0);
        assert!(
            qualify(world, &wrong_bytes).is_err(),
            "a valid declared pin cannot hide changed bytes"
        );
        assert_eq!(
            qualify(world, &input)?,
            expected,
            "invalid inputs cannot poison qualification"
        );
        let mut different = *world.as_bytes();
        different[15] ^= 1;
        let different = WorldId::decode(&different)?;
        let other = qualify(different, &input)?;
        assert_ne!(
            other, expected,
            "qualification is scoped to the supplied World"
        );
        assert_eq!(
            other,
            crate::content::qualify_selected_native_gameplay_room(different, &input)?
        );
        assert_eq!(qualify(world, &input)?, expected);
        Ok(())
    }
}
