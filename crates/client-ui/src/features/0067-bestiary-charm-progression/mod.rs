#[path = "state/browser.rs"]
mod browser;
#[path = "state/selector.rs"]
mod selector;
pub use selector::Selector;
#[path = "state/catalogue.rs"]
pub mod catalogue;
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::request;
pub use state::{Intent, Panel, RaceFilter};
pub use view::{bestiary_detail_text, bestiary_text, charms_text, panel_text};
#[cfg(test)]
mod tests;
