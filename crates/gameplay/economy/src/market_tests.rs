#![allow(clippy::expect_used)]
use super::*;

fn id(seed: u8) -> [u8; 16] {
    let mut id = [0; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id[15] = seed;
    id
}
fn offer(seed: u8, side: Side, account: u8, created: i64) -> Offer<'static> {
    // The wrapper's production identity checks stay in durability; these tests own economic rules.
    static DEFINITION: std::sync::OnceLock<TypedDefinitionRef> = std::sync::OnceLock::new();
    Offer {
        id: id(seed),
        world: WorldId::from_bytes(id(1)).expect("world"),
        definition: DEFINITION.get_or_init(|| TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:content.item.vase".into(),
            revision_ref: "r1".into(),
        }),
        side,
        account: AccountId::from_bytes(id(account)).expect("account"),
        character: CharacterId::from_bytes(id(account + 1)).expect("character"),
        piece_price: 100,
        amount: 10,
        remaining: 10,
        anonymous: false,
        created_at_us: created,
        expires_at_us: created + LIFETIME_US,
        state: OfferState::Open,
        escrow_gold: if side == Side::Buy { 1000 } else { 0 },
        held_credit_gold: 0,
        matching_pending: false,
    }
}
fn price(o: &mut Offer<'_>, p: u64) {
    o.piece_price = p;
    o.escrow_gold = if o.side == Side::Buy && o.state == OfferState::Open {
        u64::from(o.remaining) * p
    } else {
        0
    };
}
#[test]
fn market_checked_total_and_fee_limits() {
    assert_eq!(offer_total(1, MAX_TOTAL), Ok(MAX_TOTAL));
    for (amount, price) in [
        (0, 1),
        (64001, 1),
        (1, 0),
        (1, u64::MAX),
        (64000, MAX_TOTAL),
    ] {
        assert!(offer_total(amount, price).is_err());
    }
    assert_eq!(placement_fee(1, 1), Ok(20));
    assert_eq!(placement_fee(1, 1049), Ok(20));
    assert_eq!(placement_fee(1, 1050), Ok(21));
    assert_eq!(placement_fee(1, MAX_TOTAL), Ok(1_000_000));
}
#[test]
fn market_state_custody_guards_fail_closed() {
    let good = offer(60, Side::Buy, 40, 100);
    assert_eq!(good.validate(), Ok(()));
    let mut bad = good.clone();
    bad.escrow_gold -= 1;
    assert!(bad.validate().is_err());
    let mut bad = good.clone();
    bad.state = OfferState::CreditHeld(EndState::Cancelled);
    bad.held_credit_gold = 1;
    assert!(bad.validate().is_err());
    let mut held = good.clone();
    held.state = OfferState::CreditHeld(EndState::Cancelled);
    held.held_credit_gold = 1000;
    held.escrow_gold = 0;
    assert!(held.validate().is_ok());
    held.matching_pending = true;
    assert!(held.validate().is_err());
    let mut bad = good.clone();
    bad.expires_at_us -= 1;
    assert!(bad.validate().is_err());
    let mut bad = good;
    bad.remaining = 0;
    bad.escrow_gold = 0;
    assert!(bad.validate().is_err());
    bad.state = OfferState::Ended(EndState::Filled);
    assert!(bad.validate().is_ok());
}
#[test]
fn market_match_maker_price_and_exact_refund_without_compatibility_claim() {
    let mut buy = offer(60, Side::Buy, 40, 200);
    let sell = offer(61, Side::Sell, 70, 100);
    price(&mut buy, 120);
    let terms = match_terms(&buy, &sell, 4, 300).expect("terms");
    assert_eq!(
        (
            terms.maker,
            terms.sale_gold,
            terms.buy_escrow_debit,
            terms.buyer_refund_gold
        ),
        (sell.id, 400, 480, 80)
    );
    buy.created_at_us = 50;
    buy.expires_at_us = 50 + LIFETIME_US;
    assert_eq!(
        match_terms(&buy, &sell, 4, 300)
            .expect("buy maker")
            .buyer_refund_gold,
        0
    );
    let mut other = sell.clone();
    other.account = buy.account;
    assert_eq!(
        match_terms(&buy, &other, 1, 300),
        Err(PlanError::SameAccount)
    );
    let mut other = sell.clone();
    price(&mut other, 121);
    assert_eq!(
        match_terms(&buy, &other, 1, 300),
        Err(PlanError::NotCrossed)
    );
    for amount in [0, 11, u32::MAX] {
        assert!(match_terms(&buy, &sell, amount, 300).is_err());
    }
    assert!(match_terms(&buy, &sell, 1, buy.expires_at_us).is_err());
}
#[test]
fn market_cancel_expiry_and_held_credit_are_noncommitting_exact_requirements() {
    let buy = offer(60, Side::Buy, 40, 100);
    assert_eq!(
        end_plan(&buy, EndReason::Expire, 200),
        Err(PlanError::NotExpired)
    );
    let plan = end_plan(&buy, EndReason::Expire, buy.expires_at_us).expect("expire");
    assert_eq!(
        (plan.return_gold, plan.return_item_units, plan.ended_as),
        (1000, 0, EndState::Expired)
    );
    let sell = offer(61, Side::Sell, 70, 100);
    assert_eq!(
        end_plan(&sell, EndReason::Cancel, 200)
            .expect("cancel")
            .return_item_units,
        10
    );
    let mut held = buy;
    held.state = OfferState::CreditHeld(EndState::Cancelled);
    held.held_credit_gold = 1100;
    held.escrow_gold = 0;
    assert_eq!(
        held_credit_release(&held, HARD_CREDIT_CEILING - 1100),
        Ok(Some(1100))
    );
    assert_eq!(
        held_credit_release(&held, HARD_CREDIT_CEILING - 1099),
        Ok(None)
    );
    assert!(end_plan(&held, EndReason::Cancel, 200).is_err());
}
