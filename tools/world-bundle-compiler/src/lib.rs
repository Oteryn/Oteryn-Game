//! MAP-BUNDLE-1/2 and SPAWN-CONTENT-1: the server World Bundle format (`OTERYN_WORLD_BUNDLE/v3`) and its compiler
//! from the B3 World Project regions and families (ADR-0021; format document
//! `docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md`).

pub mod b3;
pub mod bundle;
pub mod compile;
pub mod project;
pub mod resolve;

pub use oteryn_world_bundle::{Error, sector, spawn};
