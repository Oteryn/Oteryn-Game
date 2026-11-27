//! Mechanic-specific composition for `ComposedFreshAdmission`.

use super::*;

#[cfg(test)]
tokio::task_local! {
    pub(super) static FRESH_SPELL_ADMISSION_BARRIER:
        std::sync::Arc<(tokio::sync::Notify, tokio::sync::Notify)>;
    pub(super) static RELOG_CONDITIONS_BARRIER:
        std::sync::Arc<(tokio::sync::Notify, tokio::sync::Notify)>;
}

#[cfg(test)]
#[derive(Default)]
pub(super) struct FamiliarLoginCheckoutProbe {
    pub(super) seed_barrier: std::sync::Arc<(tokio::sync::Notify, tokio::sync::Notify)>,
    pub(super) seed_started: std::cell::Cell<bool>,
    pub(super) backpack: std::cell::Cell<u32>,
    pub(super) kit: std::cell::Cell<u32>,
    pub(super) placement: std::cell::Cell<u32>,
    pub(super) hold_first_write: bool,
    pub(super) writes: std::cell::Cell<u32>,
    pub(super) checkout_misses: std::cell::Cell<u32>,
}

#[cfg(test)]
tokio::task_local! {
    pub(super) static FAMILIAR_LOGIN_CHECKOUT: FamiliarLoginCheckoutProbe;
}

mod achievements;
mod admission;
mod combat;
mod control_loss;
mod death;
mod familiar;
mod first_entry;
mod grace_expiry;
#[cfg(test)]
pub(super) use grace_expiry::GRACE_RELEASE_CHECKOUT;
mod items;
mod npc;
mod objects;
mod premium;
mod progression;
mod quest;
mod release;
#[cfg(test)]
pub(super) use release::{
    CONDITION_RELEASE_PROBE, FAMILIAR_READ_CHECKOUT_PROBE, FAMILIAR_WRITE_CHECKOUT_PROBE,
    HolderCheckoutProbe,
};
mod release_deferred;
mod resume;
mod spells;
mod training;
mod vitals;
mod wheel;
mod wheel_gift;
mod world_map;
