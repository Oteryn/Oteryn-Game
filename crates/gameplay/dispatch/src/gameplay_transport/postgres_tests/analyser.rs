//! Real lethal attacks and committed rewards through the production owners.
#[test]
fn analyser_live_kill_emits_committed_loot_xp_and_damage() -> super::TestResult {
    super::kill_reward_e2e::scenario_thread::run("analyser-live-kill", super::kill_reward_e2e::analyser_scenario)
}
