//! Named Forge controls with borrowed callbacks and verified item previews.
#[cfg(test)]
#[path = "legacy_native.rs"]
mod legacy;
#[cfg(test)]
pub(in crate::play) use legacy::*;

#[cfg(test)]
#[path = "lifecycle.rs"]
mod lifecycle;

#[cfg(test)]
#[path = "preview_grid.rs"]
mod preview_grid;

