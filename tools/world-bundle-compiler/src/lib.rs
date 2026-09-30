//! MAP-BUNDLE-1a: the server World Bundle format (`OTERYN_WORLD_BUNDLE/v1`) and the compiler
//! skeleton from B3 World Project regions (ADR-0021; format document
//! `docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md`).

pub mod b3;
pub mod bundle;
pub mod compile;
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
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format(what) => write!(f, "format: {what}"),
            Self::Limit(what) => write!(f, "limit: {what}"),
            Self::Key(what) => write!(f, "key: {what}"),
            Self::Bounds(what) => write!(f, "bounds: {what}"),
        }
    }
}

impl std::error::Error for Error {}
