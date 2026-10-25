//! Process-local SPECTATE1 owner. All transitions require exclusive mutable access;
//! the transport serializes them under its named owner lock. No actor or durable session exists.
mod audit;
mod grant;
mod projection;
mod rate_window;
mod runtime;
mod session;

pub use audit::AuditFact;
pub use projection::WorldProjection;
pub use runtime::{Limits, Reply, Request, SpectateRuntime, TargetFacts};
pub use session::{ObserverCommand, ObserverIdentity};

#[cfg(test)]
mod audit_tests;
#[cfg(test)]
mod grant_tests;
#[cfg(test)]
mod lifecycle_tests;
#[cfg(test)]
mod projection_tests;
#[cfg(test)]
mod session_tests;

#[cfg(any(test, feature = "test-support"))]
pub mod spectate_test_support;

#[cfg(test)]
mod world_projection_tests;
