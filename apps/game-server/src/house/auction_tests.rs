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
