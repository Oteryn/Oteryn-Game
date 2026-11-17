//! Local touch adapters; no capability, command or server state is added.
mod input;
#[cfg(windows)]
mod native;
pub(crate) mod promoted;
pub(crate) mod state;
pub(crate) mod view;
