//! Hash-pinned 15.30 appearance and sprite sheet pipeline for the Oteryn client (MAP-SPRITE-1).
//!
//! Every file is read through an [`AssetStore`], which checks it against the `sha256` of its
//! manifest entry, found by file name. A missing file, a missing manifest entry or a hash
//! mismatch is an error for that file; nothing panics and the caller draws the placeholder cell.
//! Decoding happens in memory, on demand; no derived cache is written to disk.

mod appearance;
mod catalog;
mod error;
mod sheets;
mod store;

pub use appearance::{
    AppearanceIndex, DrawCell, MAX_APPEARANCE_ID, MAX_ENTRY_CELLS, Placement, ResolvedEntry,
};
pub use catalog::{Catalog, SpriteLayout, SpriteSheetDescriptor};
pub use error::AssetError;
pub use sheets::{CELL_PX, CELL_RGBA_BYTES, MAX_RESIDENT_SHEETS, SpriteSheets};
pub use store::{
    AssetStore, MAX_APPEARANCES_BYTES, MAX_CATALOG_BYTES, MAX_COMPRESSED_SHEET_BYTES,
    MAX_DECOMPRESSED_SHEET_BYTES, MAX_MANIFEST_BYTES,
};
