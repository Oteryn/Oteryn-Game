#![allow(clippy::expect_used)]
use super::tests::offer;
use super::*;

// Each row corresponds to MARKET-1.md's Reference parity table. These immutable
// reference files, rather than today's changing manual, pin this 2% ruleset:
// https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/src/game/game.cpp
// https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/config.lua.dist
// https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/src/io/iomarket.cpp
#[test]
fn market_reference_numeric_rules() {
    for (rule, actual, expected) in [
        ("open offers per character", MAX_OPEN_OFFERS as u64, 100),
        ("units per offer", u64::from(MAX_AMOUNT), 64_000),
        (
            "piece price and accepted total cap",
            MAX_TOTAL,
            999_999_999_999,
        ),
        ("minimum fee", placement_fee(1, 1).expect("fee"), 20),
        ("2 percent", placement_fee(3, 500).expect("fee"), 30),
        (
            "floor fractional gold",
            placement_fee(1, 1099).expect("fee"),
            21,
        ),
        (
            "maximum fee",
            placement_fee(1, MAX_TOTAL).expect("fee"),
            1_000_000,
        ),
        (
            "30 days in microseconds",
            LIFETIME_US as u64,
            2_592_000_000_000,
        ),
    ] {
        assert_eq!(actual, expected, "{rule}");
    }
    for (amount, price, valid) in [
        (1, 1, true),
        (64_000, 1, true),
        (0, 1, false),
        (64_001, 1, false),
        (1, 0, false),
        (1, MAX_TOTAL, true),
        (1, MAX_TOTAL + 1, false),
        (2, MAX_TOTAL, false),
    ] {
        assert_eq!(
            offer_total(amount, price).is_ok(),
            valid,
            "{amount} × {price}"
        );
    }
}

#[test]
fn market_reference_sides_anonymity_and_expiry() {
    // Both sides may be anonymous; persisted ownership is retained. Public
    // name redaction requires MARKET-WIRE-1, absent on this main.
    for side in [Side::Buy, Side::Sell] {
        for anonymous in [false, true] {
            let mut row = offer(60, side, 40, 100);
            row.anonymous = anonymous;
            assert_eq!(row.validate(), Ok(()));
            assert_eq!(row.anonymous, anonymous);
            assert_eq!(
                end_plan(&row, EndReason::Expire, row.expires_at_us - 1),
                Err(PlanError::NotExpired)
            );
            let ended = end_plan(&row, EndReason::Expire, row.expires_at_us).expect("expiry");
            assert_eq!(ended.ended_as, EndState::Expired);
            assert_eq!(
                (ended.return_gold, ended.return_item_units),
                match side {
                    Side::Buy => (1_000, 0),
                    Side::Sell => (0, 10),
                }
            );
        }
    }
}
