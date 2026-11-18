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

#[test]
fn checkout_clock_preserves_the_current_window_and_player_reserve() {
    let mut budget = ChannelPathBudget::default();
    for _ in 0..48 {
        assert!(budget.admits(50_000, false, true));
        budget.charge(false, 1024);
    }
    let charged = budget.charged_work();
    let now = budget.checkout_time(0);
    assert_eq!(now, 50_000);
    assert!(!budget.admits(now, false, true));
    assert!(budget.admits(now, true, true));
    assert_eq!(budget.charged_work(), charged);
    assert_eq!(budget.checkout_time(100_001), 100_001);
    assert_eq!(
        budget.charged_work(),
        charged,
        "reading a future clock must not refill the ledger"
    );
    assert!(
        !budget.admits(0, false, false),
        "raw backwards admissions remain refused"
    );
}
