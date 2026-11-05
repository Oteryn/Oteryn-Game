//! Browse live carried and ground containers. Depot browsing is a separate server task.
pub mod input;
pub mod protocol;
mod state;
pub mod view;
pub use state::ContainerPanel;
