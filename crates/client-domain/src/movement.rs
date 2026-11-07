//! Protocol-neutral movement vocabulary shared by client simulation and intent mapping.

use crate::TileCoord;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepDir {
    North,
    East,
    South,
    West,
}

impl StepDir {
    #[must_use]
    pub const fn from(self, tile: TileCoord) -> TileCoord {
        let (dx, dy) = match self {
            Self::North => (0, -1),
            Self::East => (1, 0),
            Self::South => (0, 1),
            Self::West => (-1, 0),
        };
        TileCoord::new(tile.x.saturating_add(dx), tile.y.saturating_add(dy))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepResult {
    Moved,
    Refused,
}
