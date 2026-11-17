//! Account achievements earned by the admitted account.
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::{Intent, apply_result, catalog_next_request, request};
pub use state::AchievementsPanel;
pub use view::panel_text;
#[cfg(test)]
mod tests;

#[path = "state/catalog.rs"]
mod catalog;
pub use catalog::{AchievementFilter, AchievementSort, CatalogState, catalog_rows};
