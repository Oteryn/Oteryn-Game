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
use oteryn_world_bundle::{bundle, sector};
use oteryn_world_bundle_compiler::b3;
use std::{collections::HashSet, path::PathBuf};

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

fn newhaven() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let shard = content("world/areas/islands/islands-00000-00059.json")?;
    shard
        .get("records")
        .and_then(serde_json::Value::as_array)
        .ok_or("missing records")?
        .iter()
        .filter_map(|record| record.get("declaration"))
        .find(|declaration| {
            declaration.pointer("/identity/key")
                == Some(&serde_json::json!("oteryn:area.island.newhaven"))
        })
        .cloned()
        .ok_or_else(|| "missing oteryn:area.island.newhaven".into())
}

/// The island converter classifies the first ground item as land unless it is water or lava;
/// absent tiles are void. This includes every walkable land tile, even behind blocking items.
fn committed_land(
    bounds: TutorialArea,
) -> Result<HashSet<TutorialCell>, Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let classes: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        root.join("tools/content-schema/world-authoring/island-ground-classes.json"),
    )?)?;
    let mut non_land = HashSet::new();
    for kind in ["water", "lava"] {
        for ranges in classes[kind]
            .as_object()
            .ok_or("missing ground classes")?
            .values()
        {
            for range in ranges.as_str().ok_or("missing ground range")?.split(',') {
                let (low, high) = range.split_once('-').unwrap_or((range, range));
                non_land.extend(low.parse::<i64>()?..=high.parse::<i64>()?);
            }
        }
    }
    let index = content("world/placements/index.json")?;
    let palette = index["palette"].as_array().ok_or("missing palette")?;
    let mut land = HashSet::new();
    let mut budget = sector::Budget {
        tiles: 4 * 256 * 256,
        entries: 4 * 256 * 256 * 256,
    };
    // Include the boundary neighbours, so growth beyond the recorded footprint fails below.
    for rx in (bounds.min_x - 1) / 256..=(bounds.max_x + 1) / 256 {
        for ry in (bounds.min_y - 1) / 256..=(bounds.max_y + 1) / 256 {
            let path = root.join(format!(
                "content/world/placements/region-z{:02}-x{rx:03}-y{ry:03}.b3",
                bounds.floor
            ));
            let region =
                b3::decode_region(&std::fs::read(path)?, bundle::TILE_LIMITS, &mut budget)?;
            assert_eq!(i16::from(region.z), bounds.floor);
            for (_, _, tiles) in region.sectors {
                for tile in tiles {
                    let is_land = match tile.items.first() {
                        Some(ground) => !non_land.contains(&int(
                            palette
                                .get(usize::try_from(ground.palette)?)
                                .ok_or("missing ground")?,
                            "source_item_id",
                        )?),
                        None => true,
                    };
                    if is_land {
                        land.insert(TutorialCell::new(
                            tile.x.into(),
                            tile.y.into(),
                            bounds.floor,
                        ));
                    }
                }
            }
        }
    }
    Ok(land)
}

#[test]
fn a_different_land_component_inside_the_newhaven_bounds_is_inert() {
    let other = TutorialCell::new(32604, 32406, 7);
    assert!(NEWHAVEN_ISLAND.contains(other));
    assert_eq!(TutorialGate::for_start(other), TutorialGate::Inert);
}

/// Independently flood-fill the committed B3 ground from each content anchor. Check every
/// component tile and the entire bounding box, including other land components, water and void.
#[test]
fn the_gate_matches_exactly_the_two_committed_newhaven_land_components() -> TestResult {
    let declaration = newhaven()?;
    let land = committed_land(NEWHAVEN_ISLAND)?;
    let mut newhaven_land = HashSet::new();
    let mut components = vec![&declaration];
    components.extend(
        declaration["additional_components"]
            .as_array()
            .ok_or("missing components")?,
    );
    assert_eq!(components.len(), 2);
    for component in components {
        let anchor = &component["anchor"];
        let start = TutorialCell::new(
            i32::try_from(int(anchor, "x")?)?,
            i32::try_from(int(anchor, "y")?)?,
            i16::try_from(int(anchor, "floor")?)?,
        );
        assert!(land.contains(&start), "{start:?} must anchor land");
        let footprint = &component["footprint"];
        let bounds = area(footprint)?;
        let mut found = HashSet::from([start]);
        let mut pending = vec![start];
        while let Some(cell) = pending.pop() {
            assert!(
                bounds.contains(cell),
                "component grew beyond {bounds:?}: {cell:?}"
            );
            assert_eq!(
                TutorialGate::for_start(cell),
                TutorialGate::Newhaven,
                "{cell:?}"
            );
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let neighbour = TutorialCell::new(cell.x + dx, cell.y + dy, cell.floor);
                if land.contains(&neighbour) && found.insert(neighbour) {
                    pending.push(neighbour);
                }
            }
        }
        assert_eq!(found.len(), usize::try_from(int(footprint, "tile_count")?)?);
        assert!(newhaven_land.is_disjoint(&found));
        newhaven_land.extend(found);
    }
    let other = TutorialCell::new(32604, 32406, 7);
    assert!(land.contains(&other));
    assert!(!newhaven_land.contains(&other));
    let mut other_land_count = 0;
    for y in NEWHAVEN_ISLAND.min_y..=NEWHAVEN_ISLAND.max_y {
        for x in NEWHAVEN_ISLAND.min_x..=NEWHAVEN_ISLAND.max_x {
            let cell = TutorialCell::new(x, y, NEWHAVEN_ISLAND.floor);
            let expected = if newhaven_land.contains(&cell) {
                TutorialGate::Newhaven
            } else {
                other_land_count += usize::from(land.contains(&cell));
                TutorialGate::Inert
            };
            assert_eq!(TutorialGate::for_start(cell), expected, "{cell:?}");
        }
    }
    assert!(other_land_count > 0, "must exercise other land components");
    for cell in [
        ISLET_TEMPLE,
        ISLET_HOLE,
        ISLET_FORCEFIELD,
        ISLAND_ARRIVAL,
        HUNTING_TILES[0],
        HUNTING_TILES[1],
    ] {
        assert!(
            newhaven_land.contains(&cell),
            "scripted cell {cell:?} must be on Newhaven"
        );
    }
    Ok(())
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

/// The documented bounds match the content record `oteryn:area.island.newhaven`: its footprint
/// and its additional component (the islet), and every scripted cell is inside the bounds.
#[test]
fn the_gate_island_is_the_newhaven_content_record() -> TestResult {
    let declaration = newhaven()?;
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
