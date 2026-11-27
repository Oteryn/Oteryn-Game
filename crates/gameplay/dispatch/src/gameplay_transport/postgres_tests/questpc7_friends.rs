//! The chosen three friendships and ten named priests, without quest-state seeds.

use super::TestResult;

use super::questpc7_friends_fixtures::run;

#[test]
fn questpc7_friends_full_route_and_repeatable_unlocked_services() -> TestResult {
    run("questpc7-friends")
}
