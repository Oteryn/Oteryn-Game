//! Bounded Windows/winit physical-input adapter.
//!
//! This crate maps supported platform events into the merged framework-neutral
//! `oteryn-input-actions` physical-event contract. It owns no product keymap,
//! semantic gameplay binding, UI action, game command, global hook, background
//! input capture or application composition.

mod adapter;
mod error;

#[cfg(windows)]
mod winit_adapter;

pub use adapter::InputPlatformAdapter;
pub use error::InputPlatformError;

#[cfg(test)]
mod tests;
