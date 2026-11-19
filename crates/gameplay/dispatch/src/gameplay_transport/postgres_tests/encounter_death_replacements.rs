//! Crystal 201189f8, forgotten_knowledge/creaturescripts_thorn_knight_death.lua:1-23
//! and the_first_dragon/creaturescripts_death_dragon_essence.lua:3-12.
use super::TestResult;
use super::encounter_death_replacements_fixtures::{
    BOWL, EARTH, ENERGY, ENERGY_RECOVERY, ESSENCE, FALLEN, FINALE_LEVER, LORE, LORE_ROUTE, VICTORY,
    at,
};

#[test]
fn angry_plant_corpse_converts_unbeatable_dragon_and_preserves_encounter_membership() -> TestResult
{
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(BOWL, || {
        super::quest_world_callbacks_fixtures::scenario(BOWL, FINALE_LEVER, false)
    })
}

#[test]
fn thorn_knight_advances_both_phases_and_clears_only_fixed_items_on_the_death_tiles() -> TestResult
{
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(EARTH, || {
        super::quest_world_callbacks_fixtures::scenario(EARTH, at(32657, 32876), false)
    })
}

#[test]
fn fallen_challenger_corpses_route_two_staged_players_into_the_finale() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(FALLEN, || {
        super::quest_world_callbacks_fixtures::scenario(FALLEN, FINALE_LEVER, false)
    })
}

#[test]
fn energy_prism_death_returns_lloyd_only_from_the_rebirth_tile() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(ENERGY, || {
        super::quest_world_callbacks_fixtures::scenario(ENERGY, at(32759, 32867), false)
    })
}

#[test]
fn energy_prism_recovers_before_damage_only_while_the_rebirth_tile_is_empty() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(ENERGY_RECOVERY, || {
        super::quest_world_callbacks_fixtures::scenario(ENERGY_RECOVERY, at(32759, 32867), false)
    })
}

#[test]
fn lore_keeper_lever_composes_both_floors_and_glyph_death_spawns_its_successor() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(LORE, || {
        super::quest_world_callbacks_fixtures::scenario(LORE, at(32019, 32843), false)
    })
}

#[test]
fn bound_astral_power_routes_from_top_damage_across_floors_at_six_but_not_seven() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(LORE_ROUTE, || {
        super::quest_world_callbacks_fixtures::scenario(LORE_ROUTE, at(32019, 32843), false)
    })
}

#[test]
fn first_dragon_death_transfers_and_entitles_two_room_spectators() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(VICTORY, || {
        super::quest_world_callbacks_fixtures::scenario(VICTORY, at(33583, 30992), false)
    })
}

#[test]
fn later_dragon_essence_death_retires_nearby_essences_and_summons_one_first_dragon() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(ESSENCE, || {
        super::quest_world_callbacks_fixtures::scenario(ESSENCE, at(33583, 30992), false)
    })
}
