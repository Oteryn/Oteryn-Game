#![forbid(unsafe_code)]

mod artifact;
mod compiler;
mod digest;
mod fixture;
mod model;

pub use artifact::{
    ActivationSlot, ActiveContent, ArtifactExpectation, ProjectionClass, StagedArtifact,
    StagedContentPair,
};
pub use compiler::{CompiledContent, compile};
pub use fixture::synthetic_vsl_fixture;
pub use model::*;

#[cfg(test)]
mod tests;
