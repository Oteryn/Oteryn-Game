use super::{DoorBook, DoorUse};
use crate::map::view::EntryFacts;
use oteryn_protocol_oteryn::world_map::MapDefinition;
use std::sync::Arc;

fn book() -> DoorBook {
    let facts = EntryFacts {
        definition: MapDefinition::Item(std::num::NonZeroU32::MIN),
        terrain_kind: None,
        appearance_id: 1630,
        blocks_projectile: false,
        pickupable: false,
        bound: false,
        count: 1,
        sub_type: 0,
    };
    let mut doors = DoorBook::default();
    doors.push(7, true, false, facts, true, Some(3520), None, None);
    doors.set_key_forms(Some([facts; 2]));
    doors.finish();
    doors
}

#[test]
fn key_use_observation_emits_only_for_a_matching_key_that_opens() -> Result<(), String> {
    let doors = book();
    for action in [0, 3519, 3521] {
        let (outcome, capture) = doors.use_key_observed(7, 0, action);
        assert_eq!(outcome, Some(DoorUse::Locked));
        assert!(capture.is_none());
        assert_eq!(doors.revision(7), 0);
    }
    let (outcome, capture) = doors.use_key_observed(7, 0, 3520);
    assert_eq!(
        outcome,
        Some(DoorUse::Toggled {
            revision: 1,
            open: true
        })
    );
    let capture = capture.ok_or("successful opening")?;
    assert_eq!(
        (
            capture.placement_key(),
            capture.key_number(),
            capture.revision()
        ),
        (7, 3520, 1)
    );
    let (outcome, capture) = doors.use_key_observed(7, 0, 3520);
    assert_eq!(outcome, Some(DoorUse::Stale));
    assert!(capture.is_none());
    let (outcome, capture) = doors.use_key_observed(7, 1, 3520);
    assert_eq!(
        outcome,
        Some(DoorUse::Toggled {
            revision: 2,
            open: false
        })
    );
    assert!(capture.is_none(), "locking must not count as entering");
    assert!(doors.use_key_observed(8, 0, 3520).1.is_none());
    Ok(())
}

#[test]
fn key_use_observation_concurrent_attempts_capture_one_native_transition() -> Result<(), String> {
    let doors = Arc::new(book());
    let attempts: Vec<_> = (0..2)
        .map(|_| {
            let doors = Arc::clone(&doors);
            std::thread::spawn(move || doors.use_key_observed(7, 0, 3520))
        })
        .collect();
    let mut captures = 0;
    for attempt in attempts {
        let (outcome, capture) = attempt.join().map_err(|_| "door attempt")?;
        if capture.is_some() {
            assert_eq!(
                outcome,
                Some(DoorUse::Toggled {
                    revision: 1,
                    open: true
                })
            );
            captures += 1;
        } else {
            assert_eq!(outcome, Some(DoorUse::Stale));
        }
    }
    assert_eq!(captures, 1);
    assert_eq!(doors.revision(7), 1);
    Ok(())
}
