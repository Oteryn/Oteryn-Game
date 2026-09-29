//! `FIRST-CONTROL-WIRE-V1` typed payload codecs (#642 owner acceptance 5853424280).
//!
//! Moved to `oteryn-protocol-oteryn::world_spatial` (ADR-0011 §2, #162 A6, task
//! `OTV2-20260928-protocol-oteryn-crate-c1a`): a client also needs these codecs, and that crate
//! has no dependency on this one. This module re-exports the whole original `pub(crate)` surface
//! unchanged, so nothing else in this crate needed to change.

pub(crate) use oteryn_protocol_oteryn::world_spatial::*;
