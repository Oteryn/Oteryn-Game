//! Deprecated compatibility facade for the composed map and its layer-owned values.
//!
//! New callers use [`crate::map`] for app composition and the owning crates for values.

pub use crate::map::{ASSET_DIR_ENV, World};
pub use oteryn_client_assets::window::{MAX_TILE_DRAWS, PLAYER_LOOK_TYPE, RADIUS};
pub use oteryn_client_domain::world::{Draw, MapTile, START, START_FLOOR};
pub use oteryn_client_simulation::world::SCAN_MARGIN;
pub use oteryn_renderer::world::{BLACK_CELL, MARKER_CELL, TARGET_CELL};
