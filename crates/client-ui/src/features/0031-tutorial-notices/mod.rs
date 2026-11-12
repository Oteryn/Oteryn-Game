//! Tutorial and server-save notices retained for reopening from play.
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::apply;
pub use state::NoticePanel;
pub use view::panel_text;
#[cfg(test)]
mod tests;
