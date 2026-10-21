//! `USE-WIRE-V1` typed payload codecs (#162 owner acceptance 5864914163).
//!
//! Moved to `oteryn-protocol-oteryn::world_object` (ADR-0011 §2, #162 A6, task
//! `OTV2-20260928-protocol-oteryn-crate-c1a`): a client also needs these codecs, and that crate
//! has no dependency on this one. This module re-exports the whole original `pub` surface
//! unchanged, so nothing else in this crate needed to change.

pub use oteryn_protocol_oteryn::world_object::*;
