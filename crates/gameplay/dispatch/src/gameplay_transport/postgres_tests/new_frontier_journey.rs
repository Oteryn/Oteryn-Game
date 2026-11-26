use super::TestResult;
use super::new_frontier_journey_fixtures::{
    BAIT_RECOVERY, NAME, POLISHER_RETRY, farmine_world_spawn, start,
};
use super::quest_world_callbacks_fixtures::scenario;

#[test]
fn farmine_mine_leader_is_a_scheduled_world_spawn() -> TestResult {
    let spawn = farmine_world_spawn()?;
    assert_eq!(
        spawn.policy,
        oteryn_gameplay_content::content::BossPolicy::Respawn {
            min_s: 90,
            max_s: 90
        }
    );
    Ok(())
}

#[test]
fn new_frontier_full_settlement_journey_earns_rewards() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, || scenario(NAME, start(), false))
}

#[test]
fn new_frontier_king_tibianus_requires_a_crown_polisher_on_retry() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(POLISHER_RETRY, || {
        scenario(POLISHER_RETRY, start(), false)
    })
}

#[test]
fn new_frontier_melfar_replaces_bait_consumed_at_the_southern_tree() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run(BAIT_RECOVERY, || {
        scenario(BAIT_RECOVERY, start(), false)
    })
}
