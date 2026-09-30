//! MAP-BUNDLE-1: the server World Bundle format (`OTERYN_WORLD_BUNDLE/v1`) and its compiler
//! from the B3 World Project regions and families (ADR-0021; format document
//! `docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md`).

pub mod b3;
pub mod bundle;
pub mod compile;
pub mod project;
pub mod sector;

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Malformed, corrupt or inconsistent bytes or manifest.
    Format(String),
    /// A registered resource limit was exceeded.
    Limit(String),
    /// A palette key did not resolve (ADR-0021 §4.5).
    Key(String),
    /// A position outside the declared World extent or floor range (ADR-0021 §4.3).
    Bounds(String),
    /// A placement disagrees with a World Project family (ADR-0021 §4.5, §4.6).
    Family(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format(what) => write!(f, "format: {what}"),
            Self::Limit(what) => write!(f, "limit: {what}"),
            Self::Key(what) => write!(f, "key: {what}"),
            Self::Bounds(what) => write!(f, "bounds: {what}"),
            Self::Family(what) => write!(f, "family: {what}"),
        }
    }
}

impl std::error::Error for Error {}
