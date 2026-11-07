//! S06 finite Stampor source acquisition pipeline, all helpers are test-only.
use super::*;
mod collect_batch;
mod combat;
mod decay;
mod fresh_after_reset;
mod historical;
mod replace_assignment;
mod restore_owner;
mod scenario;
mod source_frame;
mod trade;
mod walk;
pub(super) use scenario::{historical_scenario, scenario};
