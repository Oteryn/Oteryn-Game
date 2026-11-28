//! Tibia Global's writing window opens with the item's current text (capability 511
//! `ITEM_INSCRIPTION_READ_V1`). Without the server's text a rewrite could erase it, so a Write
//! is admitted only for a text the client has read, or that is not empty.
pub mod protocol;
pub use oteryn_client_runtime::item_inscription::{
    CAPABILITY_ITEM_INSCRIPTION_ATTRIBUTION_V1, CAPABILITY_ITEM_INSCRIPTION_READ_V1,
    COMMAND_TYPE_ITEM_INSCRIPTION_ATTRIBUTION, COMMAND_TYPE_ITEM_INSCRIPTION_READ,
    LegacyReply as Reply, MAX_REPLY_BYTES, MAX_REQUEST_BYTES, MAX_TEXT_LENGTH, Request,
};
pub use protocol::{Known, WriteCheck, apply_result, request, write_check};
mod view;
pub use view::attribution_text;

/// The writing window item's text as the session knows it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Inscription {
    /// The session negotiated capability 511.
    pub selected: bool,
    /// The session also negotiated capability 596.
    pub attribution_selected: bool,
    pub known: Known,
}

#[cfg(test)]
mod tests;
