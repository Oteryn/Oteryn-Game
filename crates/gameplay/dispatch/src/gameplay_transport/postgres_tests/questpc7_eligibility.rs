//! Reward prerequisites are authoritative and precede every progress/value effect.

use super::TestResult;
use super::questpc7_eligibility_fixtures::{MAKE_BELIEVE_CASE, WAYLAND};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

fn run_case(name: &'static str) -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        super::quest_world_callbacks_fixtures::scenario(
            name,
            if name == MAKE_BELIEVE_CASE {
                WAYLAND
            } else {
                Position { x: 33294, y: 31677, floor: -15 }
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_ELIGIBILITY POSTGRES_CASE_COMPLETED case={name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn questpc7_reward_requirements_refuse_missing_native_premium() -> TestResult {
    run_case("questpc7-eligibility-no-premium")
}
#[test]
fn questpc7_reward_requirements_current_premium_and_low_level() -> TestResult {
    run_case("questpc7-eligibility-current")
}
#[test]
fn questpc7_reward_requirements_high_level_and_readonly_parent() -> TestResult {
    run_case("questpc7-eligibility-high")
}

#[test]
fn questpc7_wayland_outfit_repeats_full_menu_with_one_intro_receipt() -> TestResult {
    run_case("questpc7-eligibility-outfit-high")
}

#[test]
fn w16_make_believe_stops_at_venore_acceptance() -> TestResult {
    run_case(MAKE_BELIEVE_CASE)
}
