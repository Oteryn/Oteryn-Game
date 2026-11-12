//! Authoritative depot browser state and whole-stack input.
mod input;
pub use input::open_key;
pub use oteryn_client_runtime::depot::{
    ContainerViewOutcome, DepotAction, DepotIntent, DepotOutcome, DepotReply, ItemEntry,
    ItemHandle, Items,
};

mod protocol;
mod state;
mod view;
pub use protocol::{apply_container, apply_result, deposit_items, request, request_container};
pub use state::{DepotPanel, Intent};
pub use view::panel_text;

#[cfg(test)]
mod tests;
