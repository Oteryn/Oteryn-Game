use super::*;
use super::super::m01b_explore_fixtures::followup_entries::*;

#[test]
fn m01b_followup_dragon_corpse_crosses_native_fire_after_first_visit_and_use() -> TestResult {
    run(ROUTES[0])
}

#[test]
fn m01b_followup_dragon_tower_requires_first_visit_and_ten_kills() -> TestResult {
    run(ROUTES[1])
}

#[test]
fn m01b_followup_edron_throne_requires_first_visit_ten_kills_and_premium() -> TestResult {
    run(ROUTES[2])
}
