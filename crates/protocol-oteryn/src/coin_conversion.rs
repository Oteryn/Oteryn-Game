//! Capability marker for native carried-coin conversion through `USE_INTENT`.
//!
//! The capability adds no wire fields: it selects the server-side semantics for an item handle
//! already encoded by `ITEM_USE_V1`. Keeping the marker separate lets clients decline the
//! mechanic while preserving the established command and result codecs.

pub const CAPABILITY_COIN_CONVERSION_V1: u32 = 171;
