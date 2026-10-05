//! MAP-LOAD-1: the `OTERYN_WORLD_BUNDLE/v3` byte layout, its fail-closed reader and its caps,
//! shared by the compiler (`tools/world-bundle-compiler`, the writer) and the game server (the
//! reader). Format document `docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md`; ADR-0021 §4.2;
//! decision MAP-LOAD-PACKET-1 §1.2.

pub mod bundle;
pub mod sector;
pub mod spawn;

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Malformed, corrupt or inconsistent bytes or manifest.
    Format(String),
    /// A registered resource limit was exceeded.
    Limit(String),
    /// A palette key did not resolve (ADR-0021 §4.5). Compiler only.
    Key(String),
    /// A position outside the declared World extent or floor range (ADR-0021 §4.3). Compiler only.
    Bounds(String),
    /// A placement disagrees with a World Project family (ADR-0021 §4.5, §4.6). Compiler only.
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
