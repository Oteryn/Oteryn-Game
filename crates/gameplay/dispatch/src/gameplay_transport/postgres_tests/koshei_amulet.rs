//! KOSHEI: framed production use-with commits the selected amulet and RewardDoor.
use super::TestResult;
use oteryn_gameplay_quests::quest::{koshei_amulet as source, world_callbacks};

#[test]
fn koshei_validator_accepts_definition_route_and_refuses_appearance_only() -> TestResult {
    let callback = source::callback()?;
    assert_eq!(callback.effects.target_magic, Some(14));
    world_callbacks::validate_callbacks(std::slice::from_ref(&callback))?;
    let mut appearance = callback.clone();
    appearance.selector.target_definition = None;
    appearance.selector.appearance = Some(7535);
    assert!(world_callbacks::validate_callbacks(&[appearance]).is_err());
    for invalid in 0..4 {
        let mut bad = callback.clone();
        match invalid {
            0 => bad.selector.target_definition = Some("oteryn:terrain.tibia.i7535".into()),
            1 => bad.trigger = world_callbacks::Trigger::StepIn,
            2 => bad.item_on_target.as_mut().ok_or("tool")?.selected_instance = false,
            _ => bad.selector.appearance = Some(7535),
        }
        assert!(world_callbacks::validate_callbacks(&[bad]).is_err());
    }
    Ok(())
}

#[test]
fn koshei_real_use_with_grants_storage_refuses_other_operands_and_replays() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("koshei-amulet", || {
        super::quest_world_callbacks_fixtures::scenario(
            "koshei-amulet",
            world_callbacks::Position {
                x: 32700,
                y: 31200,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "KOSHEI_PG_EXECUTED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result?;
    assert!(started.elapsed().as_secs_f64() > 0.01);
    Ok(())
}

#[test]
fn koshei_spawned_ground_target_real_use_with_refusals_and_replay() -> TestResult {
    run_ground("koshei-amulet-ground")
}

#[test]
fn koshei_main_save_keeps_fragment_progress_after_amulet_use() -> TestResult {
    run_ground("koshei-amulet-saved-ground")
}

#[test]
fn koshei_runtime_role_kills_shipped_spawn_and_uses_amulet() -> TestResult {
    run_ground(super::koshei_amulet_fixtures::SPAWN_CASE)
}

fn run_ground(name: &'static str) -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        let start = if name == super::koshei_amulet_fixtures::SPAWN_CASE {
            world_callbacks::Position { x: 33271, y: 32405, floor: -12 }
        } else {
            world_callbacks::Position { x: 32700, y: 31200, floor: -7 }
        };
        super::quest_world_callbacks_fixtures::scenario(
            name,
            start,
            false,
        )
    });
    eprintln!(
        "KOSHEI_GROUND_PG_EXECUTED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result?;
    assert!(started.elapsed().as_secs_f64() > 0.01);
    Ok(())
}
