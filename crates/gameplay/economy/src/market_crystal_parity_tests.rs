#![allow(clippy::expect_used)]
use super::tests::{offer, price};
use super::*;

// Crystal 201189f871d23597b82caf88fd611454a2ddefb8 (checked against Canary
// 47dfd51f45280a59a1d3e50ba7edd573d7234446) read as reference data only.

// iomarket.cpp checkExpiredOffers: `created <= now - marketOfferDuration`
// (30 days), so an offer expires exactly at created + duration. Expired sell
// offers return the remaining units to the Inbox; expired buy offers credit
// `price * amount` of the remaining amount to the bank (processExpiredOffers).
#[test]
fn crystal_expiry_returns_remaining_units_or_escrow() {
    for (side, remaining, expected) in [
        (Side::Sell, 10, (0, 10)),
        (Side::Sell, 4, (0, 4)),
        (Side::Buy, 10, (1_000, 0)),
        (Side::Buy, 4, (400, 0)),
    ] {
        let mut row = offer(70, side, 40, 1_000);
        row.remaining = remaining;
        price(&mut row, 100);
        assert_eq!(row.expires_at_us - row.created_at_us, LIFETIME_US);
        assert_eq!(
            end_plan(&row, EndReason::Expire, row.expires_at_us - 1),
            Err(PlanError::NotExpired)
        );
        for reason in [EndReason::Expire, EndReason::Cancel] {
            let ended = end_plan(&row, reason, row.expires_at_us).expect("end");
            assert_eq!(
                (ended.return_gold, ended.return_item_units),
                expected,
                "{side:?} {remaining} {reason:?}"
            );
        }
    }
}

// game.cpp:10502-10544: the placement fee is charged on top of the buy escrow
// and is never part of what expiry or cancellation returns.
#[test]
fn crystal_fee_is_burned_outside_escrow() {
    let plan = plan_placement(Side::Buy, 10, 100).expect("plan");
    assert_eq!((plan.total, plan.escrow, plan.fee), (1_000, 1_000, 25));
    let row = offer(71, Side::Buy, 40, 0);
    let ended = end_plan(&row, EndReason::Cancel, 1).expect("cancel");
    assert_eq!(ended.return_gold, plan.escrow);
    assert_eq!(plan_placement(Side::Sell, 10, 100).expect("plan").escrow, 0);
}

// Recorded divergences (Oteryn production values pinned; Crystal values are
// the reference row, not an expectation):
// - fee: Crystal game.cpp:10502-10504 2 % capped at 1,000,000; Oteryn follows
//   tibia.com news 8996 at 2.5 % capped at 2,500,000;
// - price cap: Crystal game.cpp:10430 caps the piece price at
//   999,999,999,999; Oteryn caps the checked total;
// - amount: Crystal game.cpp:10446 limits non-stackable offers to 2,000
//   units; Oteryn applies 64,000 to every Item.
#[test]
fn crystal_divergences_are_pinned() {
    for (total, crystal, oteryn) in [
        (1_000, 20, 25),
        (10_000, 200, 250),
        (50_000_000, 1_000_000, 1_250_000),
        (500_000_000, 1_000_000, 2_500_000),
    ] {
        let fee = placement_fee(1, total).expect("fee");
        assert_eq!(fee, oteryn, "Oteryn fee for {total}");
        assert_ne!(fee, crystal, "Crystal fee for {total}");
    }
    assert_eq!(
        offer_total(2, 999_999_999_999),
        Err(PlanError::InvalidAmount)
    );
    assert_eq!(offer_total(2_001, 1), Ok(2_001));
}
