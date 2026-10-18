//! MARKET0 production Content identities mapped onto the owned durable Market model.
//! This conversion preserves typed definition identity, not marketability or admission.
#![allow(
    dead_code,
    reason = "private Market owner awaits atomic CP integration"
)]

use crate::content::{DefinitionFamily, TypedDefinitionRef};
use crate::durability::DurabilityError;
use crate::durability::market::StoredItemDefinition;

pub(crate) fn definition_for_storage(
    value: &TypedDefinitionRef,
) -> Result<StoredItemDefinition, DurabilityError> {
    if value.family() != DefinitionFamily::Item {
        return Err(DurabilityError::InvalidStoredState);
    }
    StoredItemDefinition::new(crate::durability::item_mint::TypedDefinitionRef {
        family: "Item".to_owned(),
        production_key: value.key().as_str().to_owned(),
        revision_ref: value.revision().as_str().to_owned(),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use crate::bestiary_postgres_harness::{WORLD, id};
    use crate::domain::{AccountId, CharacterId, WorldId};
    use crate::durability::market::*;
    fn world() -> WorldId {
        WorldId::from_bytes(id(WORLD)).expect("world")
    }
    fn offer(seed: u8, side: Side, account: u8, created: i64) -> Offer {
        Offer {
            id: MarketOfferId::new(id(seed)).expect("offer"),
            world: world(),
            definition: definition("oteryn:content.item.vase", "r1").expect("definition"),
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
    fn definition(
        key: &str,
        revision: &str,
    ) -> Result<StoredItemDefinition, crate::durability::DurabilityError> {
        StoredItemDefinition::new(crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".to_owned(),
            production_key: key.to_owned(),
            revision_ref: revision.to_owned(),
        })
    }
    fn price(o: &mut Offer, p: u64) {
        o.piece_price = p;
        o.escrow_gold = if o.side == Side::Buy && o.state == OfferState::Open {
            u64::from(o.remaining) * p
        } else {
            0
        };
    }
    #[test]
    fn market_production_definition_mapping_preserves_identity_and_rejects_other_families() {
        use crate::content::{
            DefinitionFamily, DefinitionRevisionRef, ProductionKey, TypedDefinitionRef,
        };
        let key = ProductionKey::new("n:ware").expect("key");
        let revision = DefinitionRevisionRef::new("r+1").expect("revision");
        let content =
            TypedDefinitionRef::new(DefinitionFamily::Item, key.clone(), revision.clone());
        let stored = super::definition_for_storage(&content).expect("mapping");
        assert_eq!(
            (stored.family(), stored.key(), stored.revision()),
            ("Item", "n:ware", "r+1")
        );
        let other = TypedDefinitionRef::new(DefinitionFamily::Creature, key, revision);
        assert!(super::definition_for_storage(&other).is_err());
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
}
