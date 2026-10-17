//! TUTORIAL-1 (CP D1426): the Newhaven tutorial applies only to a Character whose start is on the
//! Newhaven start island; a StartThais Character never enters it. The durable writer and its
//! reconnect/restart cases run on PostgreSQL in
//! `tests/support/tutorial_progress_postgres_cases.rs`.

use oteryn_game_server::domain::tutorial::{
    HUNTING_HINT, HUNTING_TILES, ISLAND_ARRIVAL, ISLAND_HINT, ISLET_FORCEFIELD, ISLET_HOLE,
    ISLET_HOLE_MESSAGE, ISLET_TEMPLE, NEWHAVEN_ISLAND, NEWHAVEN_ISLET, TutorialActor, TutorialArea,
    TutorialCell, TutorialEffect, TutorialEvent, TutorialGate, TutorialProgress, TutorialStep,
    apply,
};
use std::path::PathBuf;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const NEW_PLAYER: TutorialActor = TutorialActor {
    vocationless: true,
    has_access: false,
};
const KNIGHT: TutorialActor = TutorialActor {
    vocationless: false,
    has_access: false,
};
const GAMEMASTER: TutorialActor = TutorialActor {
    vocationless: true,
    has_access: true,
};

fn content(relative: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../content")
        .join(relative);
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}

fn int(value: &serde_json::Value, key: &str) -> Result<i64, Box<dyn std::error::Error>> {
    value
        .get(key)
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| format!("missing integer {key}").into())
}

fn area(footprint: &serde_json::Value) -> Result<TutorialArea, Box<dyn std::error::Error>> {
    Ok(TutorialArea {
        min_x: i32::try_from(int(footprint, "min_x")?)?,
        max_x: i32::try_from(int(footprint, "max_x")?)?,
        min_y: i32::try_from(int(footprint, "min_y")?)?,
        max_y: i32::try_from(int(footprint, "max_y")?)?,
        floor: i16::try_from(int(footprint, "floor")?)?,
    })
}

/// The World's current start: the StartThais pin `entry_start` (native floor).
fn current_start() -> Result<TutorialCell, Box<dyn std::error::Error>> {
    let pin = content("world/pins/oteryn.json")?;
    let start = pin.get("entry_start").ok_or("missing entry_start")?;
    Ok(TutorialCell::from_native(
        i32::try_from(int(start, "x")?)?,
        i32::try_from(int(start, "y")?)?,
        i16::try_from(int(start, "floor")?)?,
    ))
}

fn step_in(progress: Option<TutorialProgress>, to: TutorialCell) -> TutorialEffectAndProgress {
    let transition = apply(
        TutorialGate::Newhaven,
        progress,
        TutorialEvent::StepIn {
            actor: NEW_PLAYER,
            from: TutorialCell::new(to.x + 1, to.y, to.floor),
            to,
        },
    );
    (transition.effect, transition.progress)
}

type TutorialEffectAndProgress = (Option<TutorialEffect>, Option<TutorialProgress>);

/// The gate's island is the content record `oteryn:area.island.newhaven`: its footprint and its
/// additional component (the islet), and every scripted cell is on one of them.
#[test]
fn the_gate_island_is_the_newhaven_content_record() -> TestResult {
    let shard = content("world/areas/islands/islands-00000-00059.json")?;
    let records = shard
        .get("records")
        .and_then(serde_json::Value::as_array)
        .ok_or("missing records")?;
    let declaration = records
        .iter()
        .filter_map(|record| record.get("declaration"))
        .find(|declaration| {
            declaration.pointer("/identity/key")
                == Some(&serde_json::json!("oteryn:area.island.newhaven"))
        })
        .ok_or("missing oteryn:area.island.newhaven")?;
    let footprint = declaration.get("footprint").ok_or("missing footprint")?;
    assert_eq!(
        footprint.get("coordinate_frame"),
        Some(&serde_json::json!("global-target-2026-09-27"))
    );
    assert_eq!(area(footprint)?, NEWHAVEN_ISLAND);
    let islet = declaration
        .pointer("/additional_components/0/footprint")
        .ok_or("missing islet component")?;
    assert_eq!(area(islet)?, NEWHAVEN_ISLET);

    for cell in [ISLET_TEMPLE, ISLET_HOLE, ISLET_FORCEFIELD] {
        assert!(NEWHAVEN_ISLET.contains(cell), "{cell:?} is on the islet");
    }
    for cell in [ISLAND_ARRIVAL, HUNTING_TILES[0], HUNTING_TILES[1]] {
        assert!(NEWHAVEN_ISLAND.contains(cell), "{cell:?} is on the island");
    }
    assert_eq!(
        TutorialGate::for_start(ISLET_TEMPLE),
        TutorialGate::Newhaven
    );
    assert_eq!(
        TutorialGate::for_start(TutorialCell::new(32534, 32513, 6)),
        TutorialGate::Inert
    );
    Ok(())
}

/// D1426: the current start is StartThais, so the tutorial never starts: no event writes a
/// progress or has an effect, even on the Newhaven cells.
#[test]
fn a_start_thais_character_never_enters_the_tutorial() -> TestResult {
    let start = current_start()?;
    assert_eq!(start, TutorialCell::new(32369, 32238, 7));
    let gate = TutorialGate::for_start(start);
    assert_eq!(gate, TutorialGate::Inert);

    let mut events = vec![
        TutorialEvent::Login(NEW_PLAYER),
        TutorialEvent::Login(KNIGHT),
        TutorialEvent::CorruptorLoot,
        TutorialEvent::Depart,
        TutorialEvent::Death(NEW_PLAYER),
    ];
    for to in [
        ISLET_HOLE,
        ISLET_FORCEFIELD,
        HUNTING_TILES[0],
        HUNTING_TILES[1],
    ] {
        events.push(TutorialEvent::StepIn {
            actor: NEW_PLAYER,
            from: ISLET_TEMPLE,
            to,
        });
    }
    for event in events {
        let transition = apply(gate, None, event);
        assert_eq!(transition.progress, None, "{event:?}");
        assert_eq!(transition.effect, None, "{event:?}");
    }
    Ok(())
}

/// A Newhaven-start Character goes islet -> island -> departed, with each reference effect once.
#[test]
fn a_newhaven_start_character_goes_through_the_steps() {
    let gate = TutorialGate::for_start(ISLET_TEMPLE);
    assert_eq!(gate, TutorialGate::Newhaven);

    // Login: placed on the islet temple, a citizen on the islet.
    let login = apply(gate, None, TutorialEvent::Login(NEW_PLAYER));
    assert_eq!(login.effect, Some(TutorialEffect::Place(ISLET_TEMPLE)));
    let islet = login.progress;
    assert_eq!(islet, Some(TutorialProgress::new(TutorialStep::Islet)));
    assert!(login.changes(None));

    // A gamemaster or a Character with a vocation is not placed.
    for actor in [GAMEMASTER, KNIGHT] {
        let other = apply(gate, None, TutorialEvent::Login(actor));
        assert_eq!((other.progress, other.effect), (None, None));
    }

    // The islet hole pushes back to where the Character came from.
    let from = TutorialCell::new(32534, 32513, 7);
    let hole = apply(
        gate,
        islet,
        TutorialEvent::StepIn {
            actor: NEW_PLAYER,
            from,
            to: ISLET_HOLE,
        },
    );
    assert_eq!(hole.effect, Some(TutorialEffect::PushBack { to: from }));
    assert_eq!(hole.progress, islet);
    assert!(ISLET_HOLE_MESSAGE.starts_with("I should not climb down again"));

    // The hunting tiles do nothing before the forcefield arms the track.
    assert_eq!(step_in(islet, HUNTING_TILES[0]), (None, islet));

    // The forcefield: the island, both tracks armed, hint 2 for a vocationless Character.
    let (effect, island) = step_in(islet, ISLET_FORCEFIELD);
    assert_eq!(
        effect,
        Some(TutorialEffect::PlaceWithHint {
            cell: ISLAND_ARRIVAL,
            hint: ISLAND_HINT,
        })
    );
    let armed = TutorialProgress {
        step: TutorialStep::Island,
        hunting_hint_pending: true,
        corruptor_loot_pending: true,
    };
    assert_eq!(island, Some(armed));

    // The hunting hint shows once, on either tile.
    let (effect, hunted) = step_in(island, HUNTING_TILES[1]);
    assert_eq!(effect, Some(TutorialEffect::Hint(HUNTING_HINT)));
    assert_eq!(step_in(hunted, HUNTING_TILES[0]), (None, hunted));

    // The Corruptor's vocation loot is added once.
    let looted = apply(gate, hunted, TutorialEvent::CorruptorLoot);
    assert_eq!(looted.effect, Some(TutorialEffect::CorruptorVocationLoot));
    let again = apply(gate, looted.progress, TutorialEvent::CorruptorLoot);
    assert_eq!((again.effect, again.progress), (None, looted.progress));
    assert_eq!(
        looted.progress,
        Some(TutorialProgress::new(TutorialStep::Island))
    );

    // A vocationless death goes back to the islet temple; a Character with a vocation stays.
    let death = apply(gate, island, TutorialEvent::Death(NEW_PLAYER));
    assert_eq!(death.effect, Some(TutorialEffect::Place(ISLET_TEMPLE)));
    assert_eq!(
        death.progress.map(|stored| stored.step),
        Some(TutorialStep::Islet)
    );
    let knight = apply(gate, island, TutorialEvent::Death(KNIGHT));
    assert_eq!((knight.effect, knight.progress), (None, island));

    // Leaving Newhaven clears every track.
    let departed = apply(gate, island, TutorialEvent::Depart);
    assert_eq!(
        departed.progress,
        Some(TutorialProgress::new(TutorialStep::Departed))
    );
    assert_eq!(departed.effect, None);
    for progress in [islet, island, hunted, departed.progress] {
        assert!(progress.is_some_and(TutorialProgress::is_valid));
    }
}

/// Reconnect/recovery: a citizen's login resumes the persisted step without a placement, and the
/// stored step code round-trips.
#[test]
fn a_reconnect_resumes_from_the_persisted_step() {
    let gate = TutorialGate::Newhaven;
    let stored = TutorialProgress {
        step: TutorialStep::Island,
        hunting_hint_pending: true,
        corruptor_loot_pending: false,
    };
    for step in [TutorialStep::Islet, TutorialStep::Island] {
        let persisted = Some(TutorialProgress { step, ..stored });
        let resumed = apply(gate, persisted, TutorialEvent::Login(NEW_PLAYER));
        assert_eq!(resumed.progress, persisted);
        assert_eq!(resumed.effect, None);
        assert!(!resumed.changes(persisted));
    }
    for step in [
        TutorialStep::Islet,
        TutorialStep::Island,
        TutorialStep::Departed,
    ] {
        assert_eq!(TutorialStep::from_code(step.code()), Some(step));
    }
    assert_eq!(TutorialStep::from_code(0), None);
    assert_eq!(TutorialStep::from_code(4), None);

    // The reference re-enters a departed vocationless Character on the islet at login
    // (`newhavenCitizen <= 0`), with every track still cleared.
    let departed = Some(TutorialProgress::new(TutorialStep::Departed));
    let login = apply(gate, departed, TutorialEvent::Login(NEW_PLAYER));
    assert_eq!(login.effect, Some(TutorialEffect::Place(ISLET_TEMPLE)));
    assert_eq!(
        login.progress,
        Some(TutorialProgress::new(TutorialStep::Islet))
    );
}
