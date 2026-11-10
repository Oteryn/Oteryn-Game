//! Server-confirmed party membership and invitations.
#[cfg(test)]
#[path = "tests/e2e_s01.rs"]
mod e2e_s01;
mod input;
mod protocol;
mod state;
#[cfg(test)]
mod tests;
mod view;

pub use input::{Action, PartyInput, open_key};
pub use oteryn_client_runtime::party::{EntityRef, PartyCommand, PartyResponse, party};
pub use protocol::{apply_result, push_feedback, request};
pub use state::{Intent, ListAction, ListSort, PartyPanel};
pub use view::{panel_text, panel_text_at};
