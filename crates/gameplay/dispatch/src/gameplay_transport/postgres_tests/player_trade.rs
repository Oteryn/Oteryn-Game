//! TRADE-WIRE-1: two independently admitted sessions, real ingress and atomic native writer.
use super::player_trade_fixtures::{run_scenario, run_scenario_with_offer_contention, run_scenario_with_read_priority};

use super::TestResult;

#[test]
fn player_trade_postgres_two_real_sessions_atomic_replay_and_cancellation() -> TestResult {
    run_scenario(false)
}
#[test]
fn player_trade_postgres_recipient_capacity_refreshes_after_partner_accepts() -> TestResult {
    run_scenario_with_read_priority(false, false, false, false, true)
}
#[test]
fn player_trade_postgres_offer_waits_for_busy_holder() -> TestResult {
    run_scenario_with_offer_contention(false, true)
}
#[test]
fn player_trade_postgres_counter_look_and_accept_retain_foreground_priority() -> TestResult {
    run_scenario_with_read_priority(false, false, true, false, false)
}

#[test]
fn player_trade_postgres_frozen_nested_contents_wire_inspection_and_swap() -> TestResult {
    run_scenario_with_read_priority(false, false, false, true, false)
}

#[test]
fn npc_fish_hand_in_refuses_a_live_player_trade_offer() -> TestResult {
    super::player_trade_fixtures::run_quest_hand_in_trade()
}
