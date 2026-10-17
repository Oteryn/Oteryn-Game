#![allow(clippy::unwrap_used)]

use super::*;

fn bid(tag: u16, maximum_gold: u64, maximum_reached_order: u64) -> Bid {
    let mut bytes = [0_u8; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[14..].copy_from_slice(&tag.to_be_bytes());
    Bid {
        account: AccountId::from_bytes(bytes).unwrap(),
        maximum_gold,
        maximum_reached_order,
    }
}

#[test]
fn empty_single_and_zero_maximum_have_no_reserve_price() {
    assert_eq!(proxy_quote(&[]), Ok(None));
    for maximum in [0, 1, GOLD_CEILING] {
        let only = bid(1, maximum, 0);
        assert_eq!(
            proxy_quote(&[only]),
            Ok(Some(ProxyQuote {
                leader: only.account,
                price_gold: 0,
            }))
        );
    }
}

#[test]
fn proxy_price_uses_second_maximum_plus_one_and_caps_at_leader() {
    for (maxima, price) in [([20, 10], 11), ([20, 19], 20), ([20, 20], 20), ([0, 0], 0)] {
        let first = bid(1, maxima[0], 0);
        let second = bid(2, maxima[1], 1);
        for set in [[first, second], [second, first]] {
            assert_eq!(
                proxy_quote(&set),
                Ok(Some(ProxyQuote {
                    leader: first.account,
                    price_gold: price,
                }))
            );
        }
    }
    let high = bid(3, GOLD_CEILING, 0);
    assert_eq!(
        proxy_quote(&[high, bid(4, GOLD_CEILING, 1)])
            .unwrap()
            .unwrap()
            .price_gold,
        GOLD_CEILING
    );
}

#[test]
fn equal_maxima_follow_serialized_maximum_changes_in_every_input_order() {
    let old_leader = bid(1, 100, 1);
    let later_equal = bid(2, 100, 2);
    let third = bid(3, 80, 3);
    for set in [
        [old_leader, later_equal, third],
        [old_leader, third, later_equal],
        [later_equal, old_leader, third],
        [later_equal, third, old_leader],
        [third, old_leader, later_equal],
        [third, later_equal, old_leader],
    ] {
        assert_eq!(
            proxy_quote(&set).unwrap().unwrap().leader,
            old_leader.account
        );
    }
    // A raise that reaches the other bidder's maximum later cannot steal the tie. These are
    // two serialized input orders, not a claim that a database lock or durable writer exists.
    let raised = Bid {
        maximum_reached_order: 4,
        ..old_leader
    };
    assert_eq!(
        proxy_quote(&[raised, later_equal]).unwrap().unwrap().leader,
        later_equal.account
    );
}

#[test]
fn malformed_or_ambiguous_bid_inputs_refuse() {
    let first = bid(1, 10, 1);
    assert_eq!(
        proxy_quote(&[
            first,
            Bid {
                maximum_gold: 20,
                ..first
            }
        ]),
        Err(AuctionRefusal::InvalidBids)
    );
    assert_eq!(
        proxy_quote(&[first, bid(2, 10, 1)]),
        Err(AuctionRefusal::InvalidBids)
    );
    assert_eq!(
        proxy_quote(&[bid(1, GOLD_CEILING + 1, 1)]),
        Err(AuctionRefusal::InvalidBids)
    );
}

#[test]
fn exactly_256_bidders_fit_and_the_257th_refuses() {
    let mut bids = (0..BIDDERS_MAX)
        .map(|ordinal| bid(u16::try_from(ordinal).unwrap(), 1, ordinal as u64))
        .collect::<Vec<_>>();
    assert!(proxy_quote(&bids).unwrap().is_some());
    bids.push(bid(256, 1, 256));
    assert_eq!(proxy_quote(&bids), Err(AuctionRefusal::AuctionFull));
}

#[test]
fn bid_raise_lower_and_repeated_inputs_conserve_reserved_value() {
    for (previous, requested, expected) in [
        (None, 100, (120, 120, 0)),
        (Some(100), 150, (170, 50, 0)),
        (Some(150), 100, (120, 0, 50)),
        (Some(100), 100, (120, 0, 0)),
    ] {
        let plan = bid_amounts(previous, requested, 100, 20).unwrap();
        assert_eq!(
            (plan.escrow_gold, plan.reserve_gold, plan.release_gold),
            expected
        );
        let old = previous.map_or(0, |maximum| maximum + 20);
        assert_eq!(
            old + plan.reserve_gold,
            plan.escrow_gold + plan.release_gold
        );
        assert_eq!(bid_amounts(previous, requested, 100, 20), Ok(plan));
    }
    assert_eq!(
        bid_amounts(Some(150), 99, 100, 20),
        Err(AuctionRefusal::TooLow)
    );
    assert_eq!(bid_amounts(None, 0, 0, 0).unwrap().escrow_gold, 0);
    assert_eq!(bid_amounts(None, 0, 100, 20).unwrap().escrow_gold, 20);
    assert_eq!(bid_amounts(Some(5), 8, 100, 20).unwrap().reserve_gold, 3);
}

#[test]
fn escrow_arithmetic_refuses_ceiling_and_integer_overflow() {
    assert_eq!(
        bid_amounts(None, GOLD_CEILING - 1, 0, 1)
            .unwrap()
            .escrow_gold,
        GOLD_CEILING
    );
    for (previous, requested, rent) in [
        (None, GOLD_CEILING, 1),
        (None, u64::MAX, 1),
        (Some(u64::MAX), 1, 1),
        (None, 0, GOLD_CEILING + 1),
    ] {
        assert_eq!(
            bid_amounts(previous, requested, 0, rent),
            Err(AuctionRefusal::BalanceLimit)
        );
    }
    assert_eq!(
        bid_amounts(None, GOLD_CEILING, GOLD_CEILING + 1, 0),
        Err(AuctionRefusal::InvalidBids)
    );
}

#[test]
fn held_escrow_keeps_room_for_its_full_return() {
    assert_eq!(credit_fits(GOLD_CEILING - 100, 100, 0), Ok(()));
    assert_eq!(
        credit_fits(GOLD_CEILING - 100, 100, 1),
        Err(AuctionRefusal::BalanceLimit)
    );
    assert_eq!(credit_fits(GOLD_CEILING - 101, 100, 1), Ok(()));
    assert_eq!(
        credit_fits(GOLD_CEILING, 0, 1),
        Err(AuctionRefusal::BalanceLimit)
    );
    assert_eq!(
        credit_fits(u64::MAX, 1, 0),
        Err(AuctionRefusal::BalanceLimit)
    );
    assert_eq!(
        credit_fits(0, 0, u64::MAX),
        Err(AuctionRefusal::BalanceLimit)
    );
    // A reserve/release only redistributes this same sum; no credit headroom is created.
    assert_eq!(
        credit_fits(GOLD_CEILING - 100, 100, 0),
        credit_fits(GOLD_CEILING - 50, 50, 0)
    );
}

#[test]
fn the_first_bid_starts_seven_days_and_late_bids_extend_from_the_locked_clock() {
    let start = 1_000_000;
    let end = first_bid_deadline(start).unwrap();
    assert_eq!(end - start, 604_800_000);
    for now in [start, end - ANTI_SNIPING_MS - 1, end - ANTI_SNIPING_MS] {
        assert_eq!(admitted_bid_deadline(now, end), Ok(end));
    }
    assert_eq!(
        admitted_bid_deadline(end - 1, end),
        Ok(end - 1 + ANTI_SNIPING_MS)
    );
    let extended = admitted_bid_deadline(end - 1, end).unwrap();
    assert_eq!(
        admitted_bid_deadline(end, extended),
        Ok(end + ANTI_SNIPING_MS)
    );
    assert_eq!(
        admitted_bid_deadline(end, end),
        Err(AuctionRefusal::AuctionClosed)
    );
    assert_eq!(
        admitted_bid_deadline(end + 1, end),
        Err(AuctionRefusal::AuctionClosed)
    );
}

#[test]
fn unrepresentable_deadlines_refuse_without_wrapping() {
    assert_eq!(
        first_bid_deadline(i64::MAX - AUCTION_LENGTH_MS),
        Ok(i64::MAX)
    );
    assert_eq!(
        first_bid_deadline(i64::MAX - AUCTION_LENGTH_MS + 1),
        Err(AuctionRefusal::TimeOverflow)
    );
    assert_eq!(
        admitted_bid_deadline(i64::MAX - 1, i64::MAX),
        Err(AuctionRefusal::TimeOverflow)
    );
    assert_eq!(admitted_bid_deadline(i64::MIN, i64::MAX), Ok(i64::MAX));
}

#[test]
fn d1375_uses_the_native_bank_cap_without_a_separate_house_ceiling() {
    assert_eq!(
        GOLD_CEILING,
        crate::durability::bank_audit::BANK_BALANCE_MAX
    );
    assert_eq!(GOLD_CEILING, 999_999_999_999);
    assert_eq!(
        bid_amounts(None, 9_000_000_000_000_000, 0, 0),
        Err(AuctionRefusal::BalanceLimit)
    );
    assert_eq!(
        bid_amounts(None, GOLD_CEILING - 1, 0, 1)
            .unwrap()
            .escrow_gold,
        GOLD_CEILING
    );
    assert_eq!(
        bid_amounts(None, GOLD_CEILING, 0, 1),
        Err(AuctionRefusal::BalanceLimit)
    );
}

#[test]
fn native_ledger_plan_reserves_only_the_delta_and_returns_only_a_lowering() {
    let first = plan_bid_value(1_000, None, 300, 0, 25).unwrap();
    assert_eq!((first.balance_after, first.escrow_after), (675, 325));
    assert_eq!(first.entries[0].kind, HOUSE_BID_RESERVE);
    assert_eq!(first.entries[0].amount, 325);
    assert_eq!(first.value_lines(), 2);

    let raise = plan_bid_value(675, Some(300), 400, 301, 25).unwrap();
    assert_eq!((raise.balance_after, raise.escrow_after), (575, 425));
    assert_eq!(raise.entries[0].amount, 100);
    let lower = plan_bid_value(575, Some(400), 301, 301, 25).unwrap();
    assert_eq!((lower.balance_after, lower.escrow_after), (674, 326));
    assert_eq!(lower.entries[0].kind, HOUSE_BID_RELEASE);
    assert_eq!(lower.entries[0].amount, 99);
    assert_eq!(
        plan_bid_value(575, Some(400), 300, 301, 25),
        Err(AuctionRefusal::TooLow)
    );

    let same = plan_bid_value(674, Some(301), 301, 301, 25).unwrap();
    assert_eq!((same.balance_after, same.escrow_after), (674, 326));
    assert!(same.entries.is_empty());
    assert_eq!(same.value_lines(), 0);
}

#[test]
fn winning_price_and_first_rent_burn_the_captured_escrow_without_a_second_debit() {
    let plan = plan_winner_value(675, 300, 25, 201).unwrap();
    assert_eq!((plan.escrow_before, plan.escrow_after), (325, 0));
    assert_eq!((plan.balance_before, plan.balance_after), (675, 774));
    assert_eq!((plan.price_burn, plan.rent_burn), (201, 25));
    assert_eq!(
        plan.entries,
        vec![
            HouseLedgerAmount {
                kind: HOUSE_BID_RELEASE,
                amount: 99,
                balance_before: 675,
                balance_after: 774,
            },
            HouseLedgerAmount {
                kind: HOUSE_PRICE,
                amount: 201,
                balance_before: 774,
                balance_after: 774,
            },
            HouseLedgerAmount {
                kind: HOUSE_RENT,
                amount: 25,
                balance_before: 774,
                balance_after: 774,
            },
        ]
    );
    assert_eq!(plan.value_lines(), 4);
    assert_eq!(
        plan_winner_value(675, 300, 25, 301),
        Err(AuctionRefusal::InvalidBids)
    );
}

#[test]
fn excluded_bid_returns_the_snapshotted_rent_and_full_maximum_at_the_native_cap() {
    let plan = plan_release_value(GOLD_CEILING - 325, 300, 25).unwrap();
    assert_eq!(plan.balance_after, GOLD_CEILING);
    assert_eq!(plan.escrow_after, 0);
    assert_eq!((plan.price_burn, plan.rent_burn), (0, 0));
    assert_eq!(plan.entries[0].kind, HOUSE_BID_RELEASE);
    assert_eq!(plan.entries[0].amount, 325);
    assert_eq!(plan.value_lines(), 2);
    assert_eq!(
        plan_release_value(GOLD_CEILING - 324, 300, 25),
        Err(AuctionRefusal::BalanceLimit)
    );
    // These are arithmetic refusal checks. The native shared headroom producer is
    // responsible for preventing the second, impossible HELD state from arising.
}

#[test]
fn zero_value_house_outcomes_manufacture_no_bank_entry_or_value_line() {
    for plan in [
        plan_bid_value(0, None, 0, 0, 0).unwrap(),
        plan_winner_value(0, 0, 0, 0).unwrap(),
        plan_release_value(0, 0, 0).unwrap(),
        plan_ongoing_rent_value(0, 0).unwrap(),
    ] {
        assert!(plan.entries.is_empty());
        assert_eq!(plan.value_lines(), 0);
        assert_eq!(plan.balance_after, 0);
        assert_eq!(plan.escrow_after, 0);
    }
    let single = plan_winner_value(0, 300, 25, 0).unwrap();
    assert_eq!(single.balance_after, 300);
    assert_eq!(single.price_burn, 0);
    assert_eq!(single.rent_burn, 25);
    assert_eq!(single.entries.len(), 2);
    assert!(!single.entries.iter().any(|entry| entry.kind == HOUSE_PRICE));
}

#[test]
fn later_rent_uses_the_same_kind_with_an_actual_balance_debit() {
    let initial = plan_winner_value(0, 0, 25, 0).unwrap();
    let later = plan_ongoing_rent_value(25, 25).unwrap();
    assert_eq!(initial.entries[0].kind, HOUSE_RENT);
    assert_eq!(later.entries[0].kind, HOUSE_RENT);
    assert_eq!(initial.escrow_before, 25);
    assert_eq!(later.escrow_before, 0);
    assert_eq!(initial.entries[0].balance_before, 0);
    assert_eq!(later.entries[0].balance_before, 25);
    assert_eq!(initial.balance_after, 0);
    assert_eq!(later.balance_after, 0);
    assert_eq!(initial.value_lines(), 2);
    assert_eq!(later.value_lines(), 1);
    assert_eq!(
        plan_ongoing_rent_value(24, 25),
        Err(AuctionRefusal::InsufficientBalance)
    );
}

#[test]
fn every_value_plan_refuses_unfunded_or_out_of_range_inputs() {
    assert_eq!(
        plan_bid_value(324, None, 300, 0, 25),
        Err(AuctionRefusal::InsufficientBalance)
    );
    assert_eq!(
        plan_bid_value(GOLD_CEILING + 1, None, 0, 0, 0),
        Err(AuctionRefusal::BalanceLimit)
    );
    assert_eq!(
        plan_winner_value(0, GOLD_CEILING, 1, 0),
        Err(AuctionRefusal::BalanceLimit)
    );
    assert_eq!(
        plan_release_value(u64::MAX, 1, 0),
        Err(AuctionRefusal::BalanceLimit)
    );
    assert_eq!(
        plan_ongoing_rent_value(u64::MAX, 0),
        Err(AuctionRefusal::BalanceLimit)
    );
    assert_eq!(
        plan_ongoing_rent_value(0, u64::MAX),
        Err(AuctionRefusal::BalanceLimit)
    );
}
