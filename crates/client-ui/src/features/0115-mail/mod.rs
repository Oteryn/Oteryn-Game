mod input;
mod state;
mod view;
pub use input::open_key;
pub use state::{Intent, Panel};
pub use view::panel_text;
mod protocol;
pub use protocol::request;
#[path = "input/context.rs"]
pub mod context;

#[cfg(test)]
mod tests;
