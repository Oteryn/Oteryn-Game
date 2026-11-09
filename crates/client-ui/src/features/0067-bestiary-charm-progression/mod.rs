#[path = "state/catalogue.rs"]
pub mod catalogue;
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::request;
pub use state::{Intent, Panel, RaceFilter};
pub use view::{bestiary_text, charms_text, panel_text};
#[cfg(test)]
mod tests;
