//! The chosen three friendships and ten named priests, without quest-state seeds.

use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

use super::questpc7_friends_fixtures::LEGACY_CASE;

#[test]
fn questpc7_friends_resumes_a_saved_definition_with_the_pinned_quentin() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run(LEGACY_CASE, || {
        super::quest_world_callbacks_fixtures::scenario(
            LEGACY_CASE,
            // Crystal 201189f8, data-global/world/world-npc.xml:1987-1989.
            Position {
                x: 32368,
                y: 32240,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {LEGACY_CASE} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn questpc7_friends_full_route_and_repeatable_unlocked_services() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("questpc7-friends", || {
        super::quest_world_callbacks_fixtures::scenario(
            "questpc7-friends",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_FRIENDS POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
