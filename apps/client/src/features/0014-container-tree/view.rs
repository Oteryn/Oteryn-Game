#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_shared;
#[cfg(windows)]
pub(in crate::play) use native::open_window;
