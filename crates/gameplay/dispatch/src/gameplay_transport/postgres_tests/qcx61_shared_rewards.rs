//! Crystal 201189f8 chest.lua:2307-2419, one entitlement per rank and chest group.
use super::TestResult;

#[test]
fn arena_chest_and_command_share_the_weapon_claim() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(
        "qcx61_shared_rewards",
        super::arena_match_fixtures::chest_and_wire_share_rewards,
    )
}

#[test]
fn arena_chest_can_claim_an_earned_reward_after_leaving_the_reward_room() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(
        "qcx61-earned-chest-after-leaving",
        super::arena_match_fixtures::earned_chest_after_leaving,
    )
}
