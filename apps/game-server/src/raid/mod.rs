//! BOSS_RAID0 §4 private data-only transition proposals, not a runtime authority entry.
//! Current reset/content/target-channel facts and native World-job fences remain CP seams.
//! Neither a proposal nor a persisted run grants spawn, announcement or actor-removal authority.
#![allow(
    dead_code,
    reason = "Private raid data component; current World facts and production activation remain gated"
)]

use crate::content::ProductionKey;
include!("core.rs");

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
