//! HUNT-1 storage owner, extending the 0141 Character slots consumed by kill XP.
//! Slot writes wait for PREY-CONTENT-1 native draw tables. DAILYFX credits the
//! Account + World wildcard resource in `daily_reward_effects` under the claim transaction.
//! Reads never initialize a missing slot, change a lapsed slot or mint an unlock.

mod decode;

pub(super) use decode::decode_xp_slot;
