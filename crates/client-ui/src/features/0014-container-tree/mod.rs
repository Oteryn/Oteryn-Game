//! Browse live carried and ground containers. Depot browsing is a separate server task.
#[path = "state/bulk.rs"]
mod bulk;
pub use bulk::BulkMove;
pub mod input;
pub mod protocol;
mod state;
pub mod view;
pub use state::ContainerPanel;
#[cfg(test)]
mod tests;
