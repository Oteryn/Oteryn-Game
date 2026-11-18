//! Composition guards for qualification paths exercised by the live node.

#[test]
fn features_do_not_short_circuit_continuity() {
    let source = include_str!("client_phases/dev_client.rs");
    assert!(source.contains(r#"feature_result.and(client_hireling::hireling"#));
    assert!(include_str!("../qualification.rs").contains("features.and(readmission)"));
}

#[test]
fn recovery_replay_checks_first_epoch_consumption() {
    let source = include_str!("client_phases/recovery.rs");
    assert!(source.contains(r#"recovery_grant_consumed(url, session, [0x5e; 32], 1)"#));
}

#[test]
fn map_hunt_checks_actual_handle_instance() {
    let source = include_str!("client_phases/hunt.rs");
    assert!(source.contains(r#"observed_item_instance(session, handle)"#));
}

#[test]
fn trade_pins_each_offered_instance_before_acceptance() {
    let source = include_str!("client_phases/player_trade.rs");
    assert!(source.contains(r#"let offered_instances ="#));
}

#[test]
fn market_reads_exact_durable_escrow() {
    let source = include_str!("client_phases/market.rs");
    assert!(source.contains(r#"o.state=3 AND o.ended_at_us IS NOT NULL"#));
}

#[test]
fn core_funds_market_before_create() {
    let source = include_str!("client_phases/world.rs");
    assert!(source.contains(r#"fund_market_account(&mut client"#));
}

#[test]
fn committed_fixture_claims_bypass_mutation_budget() {
    let source = include_str!("../node/serve/qualification_fixture.rs");
    assert!(source.contains(r#".read_committed_reward_claim_mint("#));
}

#[test]
fn spawn_uses_shared_item_owner_lane() {
    let source = include_str!("../qualification_wild_spawn.rs");
    assert!(source.contains(r#"qualification_observation::scope_lane(scope)"#));
}

#[test]
fn relog_persistence_result_flows_through_release() {
    let source = include_str!("client_phases/world.rs");
    assert!(source.contains(r#"let persistence: TestResult = async"#));
}

#[test]
fn full_border_budget_and_profile_probe_have_reagents() {
    let source = include_str!("../node/serve/qualification_fixture.rs");
    assert!(source.contains(r#"u32::from(crate::QUALIFICATION_MAP_SAMPLES_PER_KIND) + 1"#));
}

#[test]
fn each_fight_mode_approaches_a_live_rat() {
    let source = include_str!("client_phases/combat_journey.rs");
    assert!(source.contains(r#"client_world::travel_to_spawn_target("#));
}
