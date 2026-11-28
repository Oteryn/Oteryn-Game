//! Authoritative depot browser state and whole-stack input.
#[path = "../0627-cross-town-depot/protocol.rs"]
mod cross_town;
pub use cross_town::request_for_selection;
mod input;
pub use input::open_key;
pub use oteryn_client_runtime::depot::{
    ContainerViewOutcome, DepotAction, DepotIntent, DepotOutcome, DepotReply, DepotRequest,
    ItemEntry, ItemHandle, Items,
};

mod protocol;
mod state;
mod view;
pub use protocol::{
    apply_container, apply_result, deposit_items, request, request_container, request_in_mode,
};
pub use state::{DepotPanel, Intent};
pub use view::panel_text;

#[cfg(test)]
mod tests;
