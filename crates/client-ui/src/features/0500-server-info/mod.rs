//! Server-owned presentation enrichment, shared by the Bevy and native shells.
pub mod protocol;
mod state;
pub use oteryn_client_runtime::server_info::*;
pub use protocol::{apply_result, request};
pub use state::Panel;
#[cfg(test)]
mod tests;

mod view;
pub use view::spell_text;
