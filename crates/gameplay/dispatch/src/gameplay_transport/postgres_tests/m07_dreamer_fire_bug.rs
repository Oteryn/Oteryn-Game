//! Real USE/STEP ingress for the shared Dreamer entrance; no seeded quest progress.
use super::TestResult;
use oteryn_gameplay_quests::quest::dreamer_fire_bug;

#[test]
fn m07_dreamer_fire_bug_retains_input_and_resets_shared_entrance() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("m07-dreamer-fire-bug", || {
        super::quest_world_callbacks_fixtures::scenario(
            "dreamers-challenge-fire-bug-ignite",
            dreamer_fire_bug::SOURCE,
            false,
        )
    })
}

/// The admitted ignition scenario also proves the bundle Channel frame arm
/// (`bundle_channel_frame_admission`) against the real retained proof.
#[test]
fn dreamer_ignition_accepts_bundle_channel_frame() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("m07-dreamer-bundle-frame", || {
        super::quest_world_callbacks_fixtures::scenario(
            "dreamers-challenge-fire-bug-ignite",
            dreamer_fire_bug::SOURCE,
            false,
        )
    })
}
