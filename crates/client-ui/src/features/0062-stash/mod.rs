//! Native stash panel state and commands.
pub use oteryn_client_runtime::stash::{StashCommand, StashResponse, container_batch, stash};
#[path = "../0601-stash-container/protocol.rs"]
mod batch;
#[path = "../0602-managed-container-move/protocol.rs"]
mod managed;
#[path = "../0591-stash-actions/protocol.rs"]
mod row_actions;
pub use batch::{Batch, BatchKind, request as batch_request};
pub use managed::managed_move;
pub use row_actions::{apply_row_action, row_action};
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::{
    Intent, apply_result, carried_intent, request, request_stow, request_stow_action,
};
pub use state::{Sort, StashPanel};
pub use view::{panel_text, row_label, visible_rows, visible_withdraw};
#[cfg(test)]
#[path = "tests/e2e_e07.rs"]
mod e2e_e07;
#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
