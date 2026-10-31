//! PKG6 P17: the owner decision of 2026-10-30 applies privacy and war rules to every world.
use super::TestResult;
use super::parameter_spell::scenario_in_world;

#[test]
fn p17_exiva_rules_optional_pvp_postgres() -> TestResult {
    exercise(1)
}

#[test]
fn p17_exiva_rules_open_pvp_postgres() -> TestResult {
    exercise(2)
}

#[test]
fn p17_exiva_rules_enforced_pvp_postgres() -> TestResult {
    exercise(3)
}

fn exercise(world_type: i16) -> TestResult {
    // All/public, player and guild lists, own guild/party/VIP, explicit denial,
    // and both directions of the active guild-war exception. Each fixture also
    // proves replay and refusal after the permission or active war is removed.
    for case in [7, 8, 9, 10, 11, 12, 13, 14, 16] {
        scenario_in_world(case, Some(world_type))?;
    }
    Ok(())
}
