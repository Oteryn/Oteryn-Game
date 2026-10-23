//! Permanent promotion eligibility and the login-only display projection.
use oteryn_gameplay_domain::domain::premium::{
    PreDelivery, PremiumStatus, promotion_benefit_current,
};

pub const PROMOTION_PRICE: u64 = 20_000;

pub fn eligible(level: u32, promoted: bool) -> bool {
    level >= 20 && !promoted
}

/// The pre-delivery exception can only be selected while borrowing the gate's witness.
fn before_delivery(_held: &PreDelivery<'_>) -> bool {
    true
}

pub fn purchase_permitted(status: &PremiumStatus<'_>) -> bool {
    match status {
        PremiumStatus::NotActivated(held) => before_delivery(held),
        PremiumStatus::Current => true,
        PremiumStatus::NotCurrent => false,
    }
}

/// Never an authority input for a per-use benefit; this is captured only at admission.
pub fn login_projection(promoted: bool, premium_current: bool) -> bool {
    promotion_benefit_current(promoted, premium_current)
}

#[cfg(test)]
#[path = "promotion_tests.rs"]
mod tests;
