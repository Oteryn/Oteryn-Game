//! Isolated Morla reward witness; prerequisite quest stage is arranged by the shared fixture.
//! This does not qualify the whole route or claim that travel/prerequisites were played through.
use super::TestResult;

#[test]
fn questpc5_targuna_morla_commits_exact_old_treasure_map_reward_and_replay() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-questpc5-targuna-morla-map")
}
