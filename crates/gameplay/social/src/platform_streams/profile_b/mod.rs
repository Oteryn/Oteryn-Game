//! Accepted Profile-B wire and DENY lease policy. No product is enabled by this module.
pub mod runtime;
pub mod transport;

pub use crate::durability::platform_streams::profile_b_wire::*;
#[cfg(test)]
mod tests;
