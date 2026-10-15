//! Detached policy checks over the actual native Premium domain and unregistered PREM modules.
//! These logical status fixtures do not qualify a producer, a database read or live authority.

use oteryn_game_server::domain;

#[path = "../src/premium/promotion.rs"]
pub(crate) mod promotion;
mod premium {
    pub(crate) use crate::promotion;
}
#[path = "../src/interaction/npc_promotion.rs"]
mod npc_promotion;

use domain::premium::{PremiumGate, PremiumRefusal, PremiumStatus, promotion_benefit_current};
use npc_promotion::{
    PremiumBlessingService, check_premium_blessing_purchase, promotion_coin_fee_quote,
};
use promotion::{PromotionRefusal, check_promotion_eligibility, effective_promotion_at_login};

fn current() -> PremiumGate {
    PremiumStatus::Current.gate()
}

#[test]
fn promotion_starts_at_level_twenty_without_a_manufactured_upper_level_limit()
-> Result<(), PromotionRefusal> {
    for level in [0, 1, 19] {
        assert_eq!(
            check_promotion_eligibility(level, false, current()),
            Err(PromotionRefusal::LevelTooLow)
        );
    }
    for level in [20, 21, u32::MAX] {
        check_promotion_eligibility(level, false, current())?;
    }
    Ok(())
}

#[test]
fn absent_or_noncurrent_native_gates_refuse_even_with_valid_character_values() {
    for closed in [
        PremiumGate::CLOSED,
        PremiumGate::default(),
        PremiumStatus::NotCurrent.gate(),
    ] {
        assert_eq!(
            check_promotion_eligibility(20, false, closed),
            Err(PromotionRefusal::Premium(PremiumRefusal::NotCurrent))
        );
        assert!(!effective_promotion_at_login(true, closed));
    }
}

#[test]
fn a_durable_promotion_cannot_be_purchased_again_after_premium_renewal() {
    assert_eq!(
        check_promotion_eligibility(20, true, current()),
        Err(PromotionRefusal::AlreadyPromoted)
    );
    assert!(!effective_promotion_at_login(true, PremiumGate::CLOSED));
    assert!(effective_promotion_at_login(true, current()));
    assert_eq!(
        promotion_coin_fee_quote(20, true, current()),
        Err(PromotionRefusal::AlreadyPromoted)
    );
}

#[test]
fn npc_quote_is_exactly_twenty_thousand_gold_at_every_eligible_level()
-> Result<(), PromotionRefusal> {
    for level in [20, 21, 100, u32::MAX] {
        assert_eq!(promotion_coin_fee_quote(level, false, current())?, 20_000);
    }
    Ok(())
}

#[test]
fn refused_character_or_premium_inputs_never_yield_a_fee_quote() {
    assert_eq!(
        promotion_coin_fee_quote(19, false, current()),
        Err(PromotionRefusal::LevelTooLow)
    );
    assert_eq!(
        promotion_coin_fee_quote(20, true, current()),
        Err(PromotionRefusal::AlreadyPromoted)
    );
    assert_eq!(
        promotion_coin_fee_quote(20, false, PremiumGate::CLOSED),
        Err(PromotionRefusal::Premium(PremiumRefusal::NotCurrent))
    );
}

#[test]
fn each_quote_uses_the_supplied_current_gate_and_keeps_no_previous_approval()
-> Result<(), PromotionRefusal> {
    assert_eq!(promotion_coin_fee_quote(20, false, current())?, 20_000);
    assert_eq!(
        promotion_coin_fee_quote(20, false, PremiumStatus::NotCurrent.gate()),
        Err(PromotionRefusal::Premium(PremiumRefusal::NotCurrent))
    );
    assert_eq!(promotion_coin_fee_quote(20, false, current())?, 20_000);
    Ok(())
}

#[test]
fn benefits_stop_on_lapse_while_the_login_projection_is_only_a_display_snapshot() {
    let displayed_at_login = effective_promotion_at_login(true, current());
    assert!(displayed_at_login);
    let expired = PremiumStatus::NotCurrent.gate();
    assert!(!promotion_benefit_current(true, expired.premium_current()));
    assert!(displayed_at_login);
    assert!(!effective_promotion_at_login(true, expired));
    assert!(promotion_benefit_current(true, current().premium_current()));
    assert!(effective_promotion_at_login(true, current()));
    assert!(!effective_promotion_at_login(false, current()));
}

#[test]
fn the_existing_native_soul_rule_is_not_replaced_by_effective_promotion() {
    assert!(!effective_promotion_at_login(false, current()));
    assert_eq!(
        domain::premium::soul_maximum(current().premium_current()),
        200
    );
    assert_eq!(domain::premium::apply_account_transition(200, false), 100);
}

#[test]
fn all_three_premium_blessing_purchases_use_the_native_gate() -> Result<(), PremiumRefusal> {
    for service in [
        PremiumBlessingService::WisdomOfSolitude,
        PremiumBlessingService::BloodOfTheMountain,
        PremiumBlessingService::HeartOfTheMountain,
    ] {
        assert_eq!(
            check_premium_blessing_purchase(service, PremiumGate::CLOSED),
            Err(PremiumRefusal::NotCurrent)
        );
        check_premium_blessing_purchase(service, current())?;
    }
    Ok(())
}
