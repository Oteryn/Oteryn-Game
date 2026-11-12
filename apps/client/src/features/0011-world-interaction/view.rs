#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub use native::open_context as open_context_menu;
#[cfg(windows)]
pub use native::open_menu as open_world_context_menu;
