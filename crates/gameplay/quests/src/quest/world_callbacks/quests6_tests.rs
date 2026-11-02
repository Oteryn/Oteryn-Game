//! Child of quest::world_callbacks, using its real loader and evaluator.
//! Run RED before adding the per-quest callback records, GREEN afterwards.
//! These tests do not claim missing production UID/AID bindings are present.
use super::*;

fn named(suffix: &str) -> Result<Callback, String> {
    load()?
        .into_iter()
        .find(|c| c.key == format!("oteryn:quest-callback/{suffix}"))
        .ok_or_else(|| format!("missing source callback {suffix}"))
}

#[test]
fn quests6_galthen_literal_exits_do_not_create_a_progress_track() -> Result<(), String> {
    for (key, from, to) in [
        (
            "galthen-iksupan-exit-1",
            (34015, 31892, -8),
            (32728, 32880, -7),
        ),
        (
            "galthen-iksupan-exit-2",
            (34041, 31726, -10),
            (32728, 32880, -7),
        ),
    ] {
        let c = named(key)?;
        assert_eq!(
            c.selector.position,
            Some(Position {
                x: from.0,
                y: from.1,
                floor: from.2
            })
        );
        assert_eq!(
            c.teleport,
            Some(Position {
                x: to.0,
                y: to.1,
                floor: to.2
            })
        );
        assert!(c.storage.is_empty());
        assert!(c.minimum_level.is_none());
    }
    Ok(())
}

