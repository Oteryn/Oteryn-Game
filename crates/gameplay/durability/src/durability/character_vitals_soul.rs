//! Soul R5 pure policy: cap, regeneration interval and the typed kill-receipt participant.
//! D1251: cap reads the current PremiumGate; fast regeneration also needs stored promotion.
//! The kill writer must create the participant from its actual applied XP inside its own
//! fenced transaction. This pure proposal neither commits a receipt nor proves authority.

pub use oteryn_gameplay_characters::soul::*;
