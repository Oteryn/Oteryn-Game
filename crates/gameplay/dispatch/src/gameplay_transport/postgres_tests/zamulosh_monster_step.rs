//! Crystal 201189f8, ferumbras_ascension/movements_lovely_monsters.lua:1-28.
use super::TestResult;
use super::zamulosh_monster_step_fixtures::{
    ACCESS, BLOCKED_SPAWN, BLOSSOM, CASE, FLOWER, HABITAT, HABITAT_LEVER, MISSING_BASIN, PORTAL,
    STALE_HABITAT,
};

#[test]
fn zamulosh_habitat_portal_grants_nine_matches_only_after_seven_habitats() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    // The fixture places AID 34314 on a separate test map at the portal approach.
    super::kill_reward_e2e_fixtures::scenario_thread::run(ACCESS, || {
        super::quest_world_callbacks_fixtures::scenario(ACCESS, PORTAL, false)
    })
}

#[test]
fn zamulosh_monster_step_retires_only_the_creature_paired_with_the_portal() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(CASE, || {
        super::quest_world_callbacks_fixtures::scenario(CASE, PORTAL, false)
    })
}

#[test]
fn spirit_of_fertility_replaces_itself_and_consumes_only_the_source_dry_flower() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(BLOSSOM, || {
        super::quest_world_callbacks_fixtures::scenario(BLOSSOM, FLOWER, false)
    })
}

#[test]
fn zamulosh_grass_habitat_changes_basin_at_use_and_terrain_after_two_seconds() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(HABITAT, || {
        super::quest_world_callbacks_fixtures::scenario(HABITAT, HABITAT_LEVER, false)
    })
}

#[test]
fn zamulosh_grass_habitat_refuses_an_unavailable_spawn_without_awarding_credit() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(BLOCKED_SPAWN, || {
        super::quest_world_callbacks_fixtures::scenario(BLOCKED_SPAWN, HABITAT_LEVER, false)
    })
}

#[test]
fn zamulosh_grass_habitat_drops_the_whole_batch_when_one_terrain_revision_changes() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(STALE_HABITAT, || {
        super::quest_world_callbacks_fixtures::scenario(STALE_HABITAT, HABITAT_LEVER, false)
    })
}

#[test]
fn zamulosh_grass_habitat_skips_the_absent_basin_and_its_cue() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(MISSING_BASIN, || {
        super::quest_world_callbacks_fixtures::scenario(MISSING_BASIN, HABITAT_LEVER, false)
    })
}
