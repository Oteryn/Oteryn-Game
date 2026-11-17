//! The character's own sanction record (capability 497): kind, scope, active or revoked,
//! reason, start and end of every mute, jail, ban, namelock and notation Platform recorded.
mod protocol;
mod state;
mod view;
pub use protocol::{apply_result, request};
pub use state::SanctionsPanel;
pub use view::panel_text;

#[cfg(test)]
mod tests;
