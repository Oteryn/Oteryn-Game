//! Immutable bundle spell tile data; map supplies the base index and constructs the data.
use crate::content::{LogicalCell, QualifiedSpellTile, SpellTileLookupError};
use oteryn_gameplay_domain::domain::map_position::TilePos;
use std::borrow::Cow;
use std::fmt;
use std::sync::{Arc, OnceLock};

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

/// The Channel map owner supplies current phase facts; None retains the base.
/// An unavailable phase is Unknown, never permission to reuse stale base facts.
pub trait BundleTilePhases: Send + Sync {
    fn lookup(&self, cell: LogicalCell)
    -> Result<Option<QualifiedSpellTile>, SpellTileLookupError>;
}

/// The spell tile facts of every tile of a bundle World, one interned [`QualifiedSpellTile`] per
/// distinct tile. It holds no scope: [`crate::content::QualifiedSpellTiles`] binds it to the
/// Channel's bundle scope before any lookup.
pub struct BundleSpellTiles {
    pub base: Arc<dyn WorldBase>,
    /// Per base tile ([`WorldBase::tile_index`]): an index into `shapes`, or [`UNKNOWN`].
    pub tiles: Vec<u32>,
    pub shapes: Vec<QualifiedSpellTile>,
    pub phases: OnceLock<Arc<dyn BundleTilePhases>>,
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
    /// Bind the actual map owner once, before exposing the Channel's tile data.
    pub fn install_phases(&self, owner: Arc<dyn BundleTilePhases>) -> bool {
        self.phases.set(owner).is_ok()
    }

    /// The facts of `cell` (legacy `z`, native floor `-z`): absent where the bundle holds no
    /// tile, unknown where a fact has no source (see the module documentation).
    pub fn lookup(
        &self,
        cell: LogicalCell,
    ) -> Result<Cow<'_, QualifiedSpellTile>, SpellTileLookupError> {
        let pos = tile_pos(cell).ok_or(SpellTileLookupError::Absent)?;
        if let Some(phases) = self.phases.get()
            && let Some(tile) = phases.lookup(cell)?
        {
            return Ok(Cow::Owned(tile));
        }
        let index = self
            .base
            .tile_index(pos.x, pos.y, pos.floor)
            .ok_or(SpellTileLookupError::Absent)?;
        match self.tiles.get(index).copied() {
            Some(UNKNOWN) | None => Err(SpellTileLookupError::Unknown),
            Some(shape) => self
                .shapes
                .get(shape as usize)
                .map(Cow::Borrowed)
                .ok_or(SpellTileLookupError::Unknown),
        }
    }
}

#[cfg(test)]
mod phase_tests {
    use super::*;
    use crate::content::NativeSpellTileFlags;
    use std::sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    };

    struct Base;
    impl BundleTileBase for Base {
        fn digest(&self) -> [u8; 32] {
            [7; 32]
        }
        fn tile_index(&self, x: u16, y: u16, floor: i8) -> Option<usize> {
            (x == 33430 && y == 31537 && floor == -11).then_some(0)
        }
    }
    #[derive(Default)]
    struct Phase {
        current: Mutex<Option<QualifiedSpellTile>>,
        unavailable: AtomicBool,
    }
    impl BundleTilePhases for Phase {
        fn lookup(
            &self,
            _: LogicalCell,
        ) -> Result<Option<QualifiedSpellTile>, SpellTileLookupError> {
            if self.unavailable.load(Ordering::Relaxed) {
                return Err(SpellTileLookupError::Unknown);
            }
            self.current
                .lock()
                .map(|tile| tile.clone())
                .map_err(|_| SpellTileLookupError::Unknown)
        }
    }
    fn tile(id: u32, blocked: bool) -> QualifiedSpellTile {
        QualifiedSpellTile::from_bundle(
            Some((id, 150)),
            vec![],
            NativeSpellTileFlags {
                block_solid: blocked,
                block_projectile: blocked,
                immovable_block_solid: blocked,
                immovable_block_item: blocked,
                immovable_nonfield_block_item: blocked,
                floor_change: false,
                protection_zone: false,
            },
            Some((false, false)),
        )
    }
    #[test]
    fn qn5_phase_spell_tile_snapshots_replace_base_without_rewriting_prior_reads()
    -> Result<(), Box<dyn std::error::Error>> {
        let map = BundleSpellTiles {
            base: Arc::new(Base),
            tiles: vec![0],
            shapes: vec![tile(13988, true)],
            phases: OnceLock::new(),
        };
        let cell = LogicalCell {
            x: 33430,
            y: 31537,
            z: 11,
        };
        let original = map.lookup(cell).map_err(|e| format!("{e:?}"))?;
        assert!(matches!(original, Cow::Borrowed(_)));
        let phase = Arc::new(Phase::default());
        assert!(map.install_phases(phase.clone()));
        assert!(
            !map.install_phases(phase.clone()),
            "a second owner cannot replace the Channel map owner"
        );
        assert_eq!(
            map.lookup(cell)
                .map_err(|e| format!("{e:?}"))?
                .ground_source_id(),
            Some(13988)
        );
        *phase.current.lock().map_err(|e| e.to_string())? = Some(tile(20324, false));
        let current = map.lookup(cell).map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            current.ground_source_id(),
            Some(20324),
            "phase Ground replaces the original base tile"
        );
        assert!(!current.flags().block_solid);
        assert_eq!(original.ground_source_id(), Some(13988));
        *phase.current.lock().map_err(|e| e.to_string())? = Some(tile(8085, true));
        assert_eq!(
            current.ground_source_id(),
            Some(20324),
            "a retained phase read is an immutable snapshot"
        );
        assert_eq!(
            map.lookup(cell)
                .map_err(|e| format!("{e:?}"))?
                .ground_source_id(),
            Some(8085)
        );
        phase.unavailable.store(true, Ordering::Relaxed);
        assert_eq!(
            map.lookup(cell),
            Err(SpellTileLookupError::Unknown),
            "unavailable phase facts never fall back to old ground"
        );
        Ok(())
    }
}
