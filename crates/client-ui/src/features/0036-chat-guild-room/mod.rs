mod input;
mod protocol;
mod state;
mod view;
pub use input::request;
pub use protocol::{available, replace_state};
pub use view::panel_text;
#[cfg(test)]
mod tests;
