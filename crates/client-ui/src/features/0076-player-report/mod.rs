mod input;
mod state;
mod view;
pub use input::open_key;
pub use state::{BugDraft, Intent, Panel};
pub use view::panel_text;
mod protocol;
pub use protocol::request;

#[cfg(test)]
mod tests;
