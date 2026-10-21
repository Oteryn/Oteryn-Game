//! Immutable bundle spell tile data; map supplies the base index and constructs the data.
use crate::content::{LogicalCell, QualifiedSpellTile, SpellTileLookupError};
use oteryn_gameplay_domain::domain::map_position::TilePos;
use std::fmt;
use std::sync::Arc;

/// The immutable tile index retained by the bundle's qualified tile data.
pub trait BundleTileBase: Send + Sync {
    fn digest(&self) -> [u8; 32];
    fn tile_index(&self, x: u16, y: u16, floor: i8) -> Option<usize>;
}
use self::BundleTileBase as WorldBase;

/// A tile whose facts are unknown.
pub const UNKNOWN: u32 = u32::MAX;

/// The native tile of a movement cell: `z` is legacy `z` in `0..=15`, native floor `-z`.
pub fn tile_pos(cell: LogicalCell) -> Option<TilePos> {
    let z = i8::try_from(cell.z).ok().filter(|z| (0..=15).contains(z))?;
    Some(TilePos {
        x: u16::try_from(cell.x).ok()?,
        y: u16::try_from(cell.y).ok()?,
        floor: -z,
    })
}

/// The spell tile facts of every tile of a bundle World, one interned [`QualifiedSpellTile`] per
/// distinct tile. It holds no scope: [`crate::content::QualifiedSpellTiles`] binds it to the
/// Channel's bundle scope before any lookup.
pub struct BundleSpellTiles {
    pub base: Arc<dyn WorldBase>,
    /// Per base tile ([`WorldBase::tile_index`]): an index into `shapes`, or [`UNKNOWN`].
    pub tiles: Vec<u32>,
    pub shapes: Vec<QualifiedSpellTile>,
}

impl fmt::Debug for BundleSpellTiles {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BundleSpellTiles")
            .field("digest", &self.base.digest())
            .field("tiles", &self.tiles.len())
            .field("shapes", &self.shapes.len())
            .finish()
    }
}

impl PartialEq for BundleSpellTiles {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
            || (self.base.digest() == other.base.digest()
                && self.tiles == other.tiles
                && self.shapes == other.shapes)
    }
}

impl Eq for BundleSpellTiles {}

impl BundleSpellTiles {
    /// The facts of `cell` (legacy `z`, native floor `-z`): absent where the bundle holds no
    /// tile, unknown where a fact has no source (see the module documentation).
    pub fn lookup(&self, cell: LogicalCell) -> Result<&QualifiedSpellTile, SpellTileLookupError> {
        let pos = tile_pos(cell).ok_or(SpellTileLookupError::Absent)?;
        let index = self
            .base
            .tile_index(pos.x, pos.y, pos.floor)
            .ok_or(SpellTileLookupError::Absent)?;
        match self.tiles.get(index).copied() {
            Some(UNKNOWN) | None => Err(SpellTileLookupError::Unknown),
            Some(shape) => self
                .shapes
                .get(shape as usize)
                .ok_or(SpellTileLookupError::Unknown),
        }
    }
}
