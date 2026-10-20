//! OFFLINE-0 stamina recovery arithmetic for the native settlement owner (bounded checkpoint).
//! The durable food/HP/mana residual writer is held for its migration/guard/history/death
//! integration bundle and is not part of this module yet.

// One pure arithmetic definition is shared by the native owner and the standalone tests.
mod stamina_math;
pub use stamina_math::{
    MAX_OFFLINE_SECONDS, MAX_STAMINA_MINUTES, StaminaOutOfRange, regenerate_stamina_minutes,
};
