//! Public native domain regression. The nine source policy tests now live in
//! premium/promotion.rs unit context, alongside the actual SQL owner. No source
//! file is path-compiled into an alternate crate with different native types.
use oteryn_game_server::domain::premium::{
    PremiumRefusal, PremiumStatus, require_promotion_premium,
};

#[test]
fn delivered_promotion_uses_the_actual_native_status_requirement() {
    assert_eq!(require_promotion_premium(&PremiumStatus::Current), Ok(()));
    assert_eq!(
        require_promotion_premium(&PremiumStatus::NotCurrent),
        Err(PremiumRefusal::NotCurrent)
    );
}
