//! Negotiated seasonal observation in the existing world-event window.
mod protocol;
mod view;
pub use protocol::apply_result;
pub use view::panel_text;
#[cfg(test)]
mod tests;
