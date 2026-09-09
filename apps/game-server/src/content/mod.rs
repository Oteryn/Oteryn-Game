//! Bounded CONTENT seam: non-production evidence plus FIRST_PRODUCTION_CONTENT_PROFILE/v1.
//! Live deployment authority is deliberately absent; activation consumes sealed Game authority.
//!
//! ```compile_fail
//! use oteryn_game_server::content::ActivationSlot;
//! let _ = ActivationSlot::new();
//! ```

#![forbid(unsafe_code)]

mod activation;
mod artifact;
mod compiler;
mod digest;
mod fixture;
mod model;
mod production;

pub use activation::*;
#[cfg(test)]
pub(crate) use artifact::{ActivationSlot, ActiveContent};
pub use artifact::{ArtifactExpectation, ProjectionClass, StagedArtifact, StagedContentPair};
pub use compiler::{CompiledContent, compile};
pub use fixture::synthetic_vsl_fixture;
pub use model::*;
pub use production::*;

#[cfg(test)]
mod tests;
