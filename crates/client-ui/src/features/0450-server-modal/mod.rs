//! Server-owned dialog identities with acknowledgement-driven client state.
mod protocol;
mod state;
mod view;
pub use oteryn_client_runtime::server_modal::{Answer, Dialog, Image, Reply};
pub use protocol::{answer, apply_result, hireling_store, query};
pub use state::{Panel, Pending};
pub use view::panel_text;
#[cfg(test)]
mod tests;
