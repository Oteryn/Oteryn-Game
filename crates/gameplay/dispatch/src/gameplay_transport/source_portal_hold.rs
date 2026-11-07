//! W16: a captured source portal must not warp into an empty, unowned boss arena.
//! Crystal201189f world/15.30/werepanther_boss_map_33035_32967_7.otbm
//! SHA256 b2875f6d7f5ed654a5e97b3378d3d24f7f8d99f8507d3066bab582a4d4bf0105.
//! This binding confers no quest, spawn, Instance or reward authority.
use crate::content::{CollisionClass, LogicalCell, NativeEntryMovementCells};
use crate::foundation::MovementPositionSnapshot;
use crate::map::floor::CellBlocker;
use crate::map::overlay::TilePos;
use crate::movement::CardinalStep;

pub(super) struct OriginPortalBinding {
    from: LogicalCell,
    to: LogicalCell,
}
impl OriginPortalBinding {
    /// Full encounter admission additionally proves the captured source exit.
    /// The old hold remains independently fail-closed on its entry attribute.
    pub(super) fn qualify_complete(cells: &NativeEntryMovementCells) -> Option<Self> {
        let binding = Self::qualify(cells)?;
        (crate::map::floor::table_of(cells)?.teleport(TilePos {
            x: 33043,
            y: 32986,
            floor: -8,
        }) == Some(TilePos {
            x: 33040,
            y: 32962,
            floor: -8,
        }))
        .then_some(binding)
    }
    /// The current Bundle owner must independently retain the exact captured
    /// native teleport attribute and enterable origin in this activation.
    pub(super) fn qualify(cells: &NativeEntryMovementCells) -> Option<Self> {
        let from = LogicalCell {
            x: 33039,
            y: 32964,
            z: 8,
        };
        let to = LogicalCell {
            x: 33045,
            y: 32988,
            z: 8,
        };
        if cells.scope().generation_digest == [0; 32]
            || cells.index().lookup(cells.scope(), from).ok() != Some(CollisionClass::Walkable)
            || crate::map::floor::table_of(cells)?.teleport(TilePos {
                x: 33039,
                y: 32964,
                floor: -8,
            }) != Some(TilePos {
                x: 33045,
                y: 32988,
                floor: -8,
            })
        {
            return None;
        }
        Some(Self { from, to })
    }
}

/// An actor-local landing constraint consumed by the existing sealed Bundle
/// step. A refused teleport landing leaves the actor on its real source portal.
pub(super) struct DeferredPortal<'a> {
    ordinary: &'a dyn CellBlocker,
    landing: Option<LogicalCell>,
}
impl<'a> DeferredPortal<'a> {
    pub(super) fn for_step(
        cells: &NativeEntryMovementCells,
        origin: MovementPositionSnapshot,
        direction: CardinalStep,
        ordinary: &'a dyn CellBlocker,
    ) -> Self {
        let (dx, dy) = match direction {
            CardinalStep::North => (0, -1),
            CardinalStep::South => (0, 1),
            CardinalStep::East => (1, 0),
            CardinalStep::West => (-1, 0),
        };
        let p = origin.position();
        let landing = OriginPortalBinding::qualify(cells).and_then(|binding| {
            ((p.x.checked_add(dx), p.y.checked_add(dy), i32::from(p.floor))
                == (Some(binding.from.x), Some(binding.from.y), binding.from.z))
                .then_some(binding.to)
        });
        Self { ordinary, landing }
    }
}
impl CellBlocker for DeferredPortal<'_> {
    fn blocks(&self, cell: LogicalCell) -> bool {
        self.landing == Some(cell) || self.ordinary.blocks(cell)
    }
}
