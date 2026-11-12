//! Mechanic-specific composition for `ComposedFreshAdmission`.

use super::*;

#[cfg(test)]
tokio::task_local! {
    pub(super) static FRESH_SPELL_ADMISSION_BARRIER:
        std::sync::Arc<(tokio::sync::Notify, tokio::sync::Notify)>;
}

mod achievements;
mod admission;
mod combat;
mod control_loss;
mod death;
mod familiar;
mod first_entry;
mod grace_expiry;
mod items;
mod npc;
mod objects;
mod premium;
mod progression;
mod quest;
mod release;
mod release_deferred;
mod resume;
mod spells;
mod training;
mod vitals;
mod wheel;
mod wheel_gift;
mod world_map;
