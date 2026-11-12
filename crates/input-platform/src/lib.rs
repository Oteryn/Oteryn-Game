//! Bounded winit physical-input adapter for native desktop hosts.
//!
//! Windows exposes the existing winit ingress by default. Other desktop hosts
//! opt in through `native-winit`; Linux enables the existing X11 backend.
//!
//! This crate maps supported platform events into the merged framework-neutral
//! `oteryn-input-actions` physical-event contract. It owns no product keymap,
//! semantic gameplay binding, UI action, game command, global hook, background
//! input capture or application composition.

mod adapter;
mod error;

#[cfg(any(windows, feature = "native-winit"))]
mod winit_adapter;

pub use adapter::InputPlatformAdapter;
pub use error::InputPlatformError;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod audit_regressions;
