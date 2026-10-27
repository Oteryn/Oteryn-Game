//! Quest log client projection: the server remains the quest truth authority.
pub use oteryn_session::quest_log::*;
use oteryn_session::{AppliedDelta, CommandStatus, GatedSnapshot};
mod protocol;
mod state;
pub use protocol::send;
pub use state::{QuestRefusal, QuestState};
#[cfg(test)]
#[path = "tests/reducer.rs"]
mod tests;
