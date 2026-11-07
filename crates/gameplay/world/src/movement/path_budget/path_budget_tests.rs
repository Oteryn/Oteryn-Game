use super::*;

#[test]
fn sixty_five_full_searches_run_sixty_four_then_wait_for_the_next_window() {
    let mut budget = ChannelPathBudget::default();
    budget.declare_creatures(65);
    for _ in 0..64 {
        assert!(budget.admits(0, false, false));
        budget.charge(false, 1024);
    }
    assert!(!budget.admits(49_999, false, false));
    assert!(budget.admits(50_000, false, false));
    assert!(!budget.admits(0, false, false));
}

#[test]
fn waiting_players_reserve_sixteen_searches_and_unused_reserve_returns() {
    let mut budget = ChannelPathBudget::default();
    budget.declare_creatures(65);
    for _ in 0..48 {
        assert!(budget.admits(0, false, true));
        budget.charge(false, 1024);
    }
    assert!(!budget.admits(0, false, true));
    assert!(budget.admits(0, true, true));
    budget.charge(true, 5);
    for _ in 0..15 {
        assert!(budget.admits(0, false, false));
        budget.charge(false, 1024);
    }
    assert!(!budget.admits(0, false, false));
}

#[test]
fn actual_expansions_are_charged_and_declared_creature_work_is_protected() {
    let mut budget = ChannelPathBudget::default();
    budget.declare_creatures(64);
    for _ in 0..16 {
        assert!(budget.admits(0, true, true));
        budget.charge(true, 1024);
    }
    assert!(!budget.admits(0, true, true));
    assert!(budget.admits(0, false, true));
    budget.charge(false, 1);
    assert!(budget.admits(0, false, true));
}
