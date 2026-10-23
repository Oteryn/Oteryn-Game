//! HUNT-1 storage owner, extending the 0141 Character slots consumed by kill XP.
//! No writer is admitted until PREY-CONTENT-1 supplies the native draw tables.
//! Reads never initialize a missing slot, change a lapsed slot or mint an unlock.

mod decode;

pub(super) use decode::decode_xp_slot;
