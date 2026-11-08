mod input;
mod state;
mod view;
pub use input::open_key;
pub use state::{Intent, Panel};
pub use view::{panel_text, title_rows};
mod protocol;
pub use protocol::request;

#[cfg(test)]
mod tests;

#[path = "state/titles.rs"]
mod titles;
pub use input::titles_key;
pub use titles::{OwnTitles, TitleChoice, TitleFilter, apply_title_reply};
