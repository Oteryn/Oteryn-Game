//! Capability 503 decoding for the existing self-only blessings panel.
use oteryn_client_runtime::cyclopedia_character::blessings_dialog::Snapshot;
pub(crate) fn decode(bytes: &[u8]) -> Option<Snapshot> {
    Snapshot::decode(bytes)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
