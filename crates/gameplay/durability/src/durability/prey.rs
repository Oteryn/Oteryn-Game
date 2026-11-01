//! PREY-LIVE-1: fenced, receipt-first owner of the three typed Prey slots.
mod command;
mod decode;
mod draw;
mod house_read;
mod storage;

pub use command::PreyMutation;
pub(super) use decode::decode_xp_slot;
pub use draw::PreyCatalogue;
