//! Tibia Global's writing window opens with the item's current text (capability 511
//! `ITEM_INSCRIPTION_READ_V1`). Without the server's text a rewrite could erase it, so a Write
//! is admitted only for a text the client has read, or that is not empty.
pub mod protocol;
pub use oteryn_client_runtime::item_inscription::*;
pub use protocol::{Known, WriteCheck, apply_result, request, write_check};

/// The writing window item's text as the session knows it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Inscription {
    /// The session negotiated capability 511.
    pub selected: bool,
    pub known: Known,
}

#[cfg(test)]
mod tests;
