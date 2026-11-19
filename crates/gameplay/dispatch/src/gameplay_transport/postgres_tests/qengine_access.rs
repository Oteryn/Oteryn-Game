//! QENGINE-1 source access through the literal production command dispatcher.
use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

fn scenario(name: &str, trigger: Position) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let task = name.to_owned();
    let result = (|| -> TestResult {
        std::thread::Builder::new()
            .name(task.clone())
            .stack_size(256 << 20)
            .spawn(move || {
                super::quest_world_callbacks_fixtures::scenario(&task, trigger, false)
                    .map_err(|error| error.to_string())
            })?
            .join()
            .map_err(|_| "quest access scenario panicked")??;
        Ok(())
    })();
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qengine_thais_plate_step_out_restores_stairs() -> TestResult {
    scenario(
        "thais-lighthouse-plate",
        Position {
            x: 32225,
            y: 32268,
            floor: -9,
        },
    )
}

#[test]
fn qengine_thais_portal_pair_teleports_and_removes_atomically() -> TestResult {
    scenario(
        "thais-lighthouse-portals",
        Position {
            x: 32225,
            y: 32285,
            floor: -10,
        },
    )
}

#[test]
fn qengine_triangle_tower_asymmetric_item_cycle() -> TestResult {
    scenario(
        "triangle-tower-lever",
        Position {
            x: 32573,
            y: 32121,
            floor: -7,
        },
    )
}

#[test]
fn qengine_thais_portal_pair_expires_atomically() -> TestResult {
    scenario(
        "thais-lighthouse-portals-expiry",
        Position {
            x: 32225,
            y: 32285,
            floor: -10,
        },
    )
}

#[test]
fn qengine_waterfall_facing_and_splash() -> TestResult {
    for (name, x, y, floor) in [
        ("waterfall-cliff", 32966, 32626, -5),
        ("waterfall-splash", 32968, 32630, -7),
        ("waterfall-cave-entry", 32968, 32629, -7),
        ("waterfall-cave-exit", 32967, 32630, -8),
    ] {
        scenario(name, Position { x, y, floor })?;
    }
    Ok(())
}

#[test]
fn qengine_annihilator_party_encounter_fail_closed() -> TestResult {
    scenario(
        "annihilator-entry",
        Position {
            x: 33226,
            y: 31671,
            floor: -13,
        },
    )
}

#[test]
fn qengine_l1_world_warmaster_base_wardrobe() -> TestResult {
    scenario(
        "l1-warmaster-wardrobe",
        Position {
            x: 33053,
            y: 31020,
            floor: -7,
        },
    )
}

#[test]
fn qengine_l1_world_wayfarer_base_wardrobe() -> TestResult {
    scenario(
        "l1-wayfarer-wardrobe",
        Position {
            x: 33073,
            y: 31173,
            floor: -8,
        },
    )
}
