//! Existing admitted-session transport -> PvP rules -> PostgreSQL -> result bytes and reload.
use super::pvp_live_fixtures::{scenario};

use super::TestResult;

#[test]
fn pvp_live_wire_melee_spell_death_replay_and_reload() -> TestResult {
    super::item_look_fixtures::large_stack(|| scenario(2))
}
#[test]
fn pvp_live_optional_world_refuses_player_harm_without_durable_writes() -> TestResult {
    super::item_look_fixtures::large_stack(|| scenario(1))
}

#[test]
fn pvp_live_protection_zone_refuses_player_harm() -> TestResult {
    super::item_look_fixtures::large_stack(|| scenario(4))
}
#[test]
fn pvp_live_no_pvp_tile_refuses_player_harm() -> TestResult {
    super::item_look_fixtures::large_stack(|| scenario(5))
}

#[test]
fn pvp_live_bundle_melee_spell_death_replay_and_reload() -> TestResult {
    super::item_look_fixtures::large_stack(|| scenario(6))
}

#[test]
fn pvp_expert_wire_modes_reach_melee_and_spell_checker() -> TestResult {
    super::item_look_fixtures::large_stack(|| scenario(7))
}
#[test]
fn pvp_expert_hardcore_locks_red_fist() -> TestResult {
    super::item_look_fixtures::large_stack(|| scenario(8))
}
#[test]
fn pvp_expert_unselected_capability_refuses_command_and_sends_no_domain() -> TestResult {
    super::item_look_fixtures::large_stack(|| scenario(9))
}
#[test]
fn pvp_expert_black_skull_switching_and_session_fences() -> TestResult {
    super::item_look_fixtures::large_stack(|| scenario(10))
}
#[test]
fn pvp_expert_wire_modes_reach_distance_checker() -> TestResult {
    super::item_look_fixtures::large_stack(|| scenario(11))
}
#[test]
fn pvp_expert_optional_world_ignores_expert_choice() -> TestResult {
    super::item_look_fixtures::large_stack(|| scenario(12))
}

#[test]
fn pvp_expert_white_hand_reads_party_and_guild_aggressors() -> TestResult {
    super::item_look_fixtures::large_stack(|| scenario(13))
}
