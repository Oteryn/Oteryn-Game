//! Negotiated command coverage; the existing Wheel panel owns presentation and state.
use oteryn_client_runtime::wheel_allocation::atelier::resources::{self, ResourceBalances};

pub fn decode_reply(bytes: &[u8]) -> Option<ResourceBalances> {
    resources::decode_reply(bytes).ok()
}
