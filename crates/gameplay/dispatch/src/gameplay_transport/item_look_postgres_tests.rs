//! LOOK-2 through the real composed owner, PostgreSQL admission and `serve_admitted`.
//! Nested under monk-save tests to share their fresh-admission/recovery fixture.
use super::item_look_fixtures::{large_stack, scenario};

use super::TestResult;

#[test]
fn item_look_postgres_composed_owner_serves_near_far_items_creature_player_and_hidden_target()
-> TestResult {
    large_stack(scenario)
}
