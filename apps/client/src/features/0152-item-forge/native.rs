//! Named Forge controls with borrowed callbacks and verified item previews.
#[cfg(test)]
#[path = "legacy_native.rs"]
mod legacy;
#[cfg(test)]
pub(in crate::play) use legacy::*;
#[cfg(windows)]
#[path = "window.rs"]
mod window;
#[cfg(windows)]
pub(in crate::play) use window::open_window;

#[cfg(any(windows, test))]
#[path = "lifecycle.rs"]
mod lifecycle;

#[cfg(any(windows, test))]
#[path = "preview_grid.rs"]
mod preview_grid;
