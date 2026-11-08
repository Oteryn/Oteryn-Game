//! Protocol-neutral movement vocabulary shared by client simulation and intent mapping.

use crate::TileCoord;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepDir {
    North,
    East,
    South,
    West,
    NorthEast,
    SouthEast,
    SouthWest,
    NorthWest,
}

impl StepDir {
    #[must_use]
    pub const fn from(self, tile: TileCoord) -> TileCoord {
        let (dx, dy) = match self {
            Self::North => (0, -1),
            Self::East => (1, 0),
            Self::South => (0, 1),
            Self::West => (-1, 0),
            Self::NorthEast => (1, -1),
            Self::SouthEast => (1, 1),
            Self::SouthWest => (-1, 1),
            Self::NorthWest => (-1, -1),
        };
        match (tile.x.checked_add(dx), tile.y.checked_add(dy)) {
            (Some(x), Some(y)) => TileCoord::new(x, y),
            _ => tile,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepResult {
    Moved,
    Refused,
}

#[cfg(test)]
#[test]
fn qn6_diagonal_client_offsets_and_overflow_are_atomic() {
    let origin = TileCoord::new(4, 5);
    for (direction, x, y) in [
        (StepDir::NorthEast, 5, 4),
        (StepDir::SouthEast, 5, 6),
        (StepDir::SouthWest, 3, 6),
        (StepDir::NorthWest, 3, 4),
    ] {
        assert_eq!(direction.from(origin), TileCoord::new(x, y));
    }
    let edge = TileCoord::new(i32::MAX, 5);
    assert_eq!(StepDir::NorthEast.from(edge), edge);
}
