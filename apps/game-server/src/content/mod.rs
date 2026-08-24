//! Non-production VSL evidence seam. Runtime activation is deliberately not public.
//!
//! ```compile_fail
//! use oteryn_game_server::content::ActivationSlot;
//! let _ = ActivationSlot::new();
//! ```

#![forbid(unsafe_code)]

mod artifact;
mod compiler;
mod digest;
mod fixture;
mod model;

#[cfg(test)]
pub(crate) use artifact::{ActivationSlot, ActiveContent};
pub use artifact::{ArtifactExpectation, ProjectionClass, StagedArtifact, StagedContentPair};
pub use compiler::{CompiledContent, compile};
pub use fixture::synthetic_vsl_fixture;
pub use model::*;

#[cfg(test)]
mod tests;
