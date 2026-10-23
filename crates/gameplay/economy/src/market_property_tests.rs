use super::tests::{offer, price};
use super::*;
use oteryn_test_support::property::for_each_case;

#[test]
fn market_property_checked_totals_and_fees() {
    for_each_case("market totals and fees", |rng, _| {
        for (amount, price) in [
            (rng.u64() as u32, rng.u64()),
            (
                1 + rng.below(u64::from(MAX_AMOUNT)) as u32,
                1 + rng.below(MAX_TOTAL),
            ),
            (MAX_AMOUNT, u64::MAX),
            (u32::MAX, u64::MAX),
        ] {
            let wide = u128::from(amount) * u128::from(price);
            let valid =
                (1..=MAX_AMOUNT).contains(&amount) && price > 0 && wide <= u128::from(MAX_TOTAL);
            assert_eq!(offer_total(amount, price).is_ok(), valid);
            assert_eq!(placement_fee(amount, price).is_ok(), valid);
            if let Ok(total) = offer_total(amount, price) {
                assert_eq!(u128::from(total), wide);
                let fee = placement_fee(amount, price).map_err(|e| format!("{e:?}"))?;
                assert_eq!(fee, (total / 50).clamp(MIN_FEE, MAX_FEE));
                assert!((MIN_FEE..=MAX_FEE).contains(&fee));
            }
        }
        Ok(())
    });
}

#[test]
fn market_property_match_conserves_gold_items_and_expiry_remainder() {
    for_each_case("market match conservation", |rng, _| {
        let mut buy = offer(60, Side::Buy, 40, 100);
        let mut sell = offer(61, Side::Sell, 70, 200);
        let units = 1 + rng.below(u64::from(MAX_AMOUNT)) as u32;
        buy.amount = units;
        buy.remaining = units;
        sell.amount = units;
        sell.remaining = units;
        let buy_price = 1 + rng.below(MAX_TOTAL / u64::from(units));
        let sell_price = 1 + rng.below(buy_price);
        price(&mut buy, buy_price);
        price(&mut sell, sell_price);
        if rng.bool() {
            sell.created_at_us = 50;
            sell.expires_at_us = 50 + LIFETIME_US;
        }
        let accepted = 1 + rng.below(u64::from(units)) as u32;
        let terms = match_terms(&buy, &sell, accepted, 300).map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            terms.sale_gold + terms.buyer_refund_gold,
            terms.buy_escrow_debit
        );
        assert_eq!(
            buy.escrow_gold - terms.buy_escrow_debit,
            u64::from(units - accepted) * buy_price
        );
        assert_eq!(accepted + (sell.remaining - accepted), sell.remaining);
        assert!((sell_price..=buy_price).contains(&terms.piece_price));
        for mut remainder in [buy, sell] {
            remainder.remaining -= accepted;
            if remainder.side == Side::Buy {
                remainder.escrow_gold -= terms.buy_escrow_debit;
            }
            if remainder.remaining == 0 {
                remainder.state = OfferState::Ended(EndState::Filled);
                assert_eq!(remainder.validate(), Ok(()));
            } else {
                let plan = end_plan(&remainder, EndReason::Expire, remainder.expires_at_us)
                    .map_err(|e| format!("{e:?}"))?;
                assert_eq!(plan.return_gold, remainder.escrow_gold);
                assert_eq!(
                    plan.return_item_units,
                    if remainder.side == Side::Sell {
                        units - accepted
                    } else {
                        0
                    }
                );
            }
        }
        Ok(())
    });
}
