//! STAGPOS: a bounded STEP area selector, one rectangle per floor minus
//! explicitly excluded tiles (Crystal registers one MoveEvent per position).
use super::Position;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SelectorArea {
    pub rectangles: Vec<AreaRectangle>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excluded: Vec<Position>,
}

/// Inclusive `[low, high]` bounds on one native floor.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AreaRectangle {
    pub floor: i8,
    pub x: [u16; 2],
    pub y: [u16; 2],
}

/// At most one rectangle per floor.
pub const MAX_AREA_RECTANGLES: usize = 4;
/// Selected tiles after exclusion; Stag's mines use 238.
pub const MAX_AREA_TILES: u64 = 256;
pub const MAX_AREA_EXCLUDED: usize = 8;

impl AreaRectangle {
    fn contains(&self, position: Position) -> bool {
        position.floor == self.floor
            && (self.x[0]..=self.x[1]).contains(&position.x)
            && (self.y[0]..=self.y[1]).contains(&position.y)
    }

    fn tiles(&self) -> u64 {
        (u64::from(self.x[1]) - u64::from(self.x[0]) + 1)
            * (u64::from(self.y[1]) - u64::from(self.y[0]) + 1)
    }
}

impl SelectorArea {
    #[must_use]
    pub fn contains(&self, position: Position) -> bool {
        self.rectangles.len() <= MAX_AREA_RECTANGLES
            && self.excluded.len() <= MAX_AREA_EXCLUDED
            && self.rectangles.iter().any(|r| r.contains(position))
            && !self.excluded.contains(&position)
    }

    /// Closed bounds: distinct floors, ordered corners, excluded tiles inside
    /// and distinct, and a bounded non-empty selection.
    #[must_use]
    pub fn valid(&self) -> bool {
        let floors: std::collections::BTreeSet<_> =
            self.rectangles.iter().map(|r| r.floor).collect();
        let excluded: std::collections::BTreeSet<_> = self.excluded.iter().collect();
        if self.rectangles.is_empty()
            || self.rectangles.len() > MAX_AREA_RECTANGLES
            || floors.len() != self.rectangles.len()
            || self.excluded.len() > MAX_AREA_EXCLUDED
            || excluded.len() != self.excluded.len()
            || self
                .rectangles
                .iter()
                .any(|r| !(-15..=0).contains(&r.floor) || r.x[0] > r.x[1] || r.y[0] > r.y[1])
            || self
                .excluded
                .iter()
                .any(|p| !self.rectangles.iter().any(|r| r.contains(*p)))
        {
            return false;
        }
        // Four rectangles of at most 2^32 tiles each cannot overflow u64.
        self.rectangles
            .iter()
            .map(AreaRectangle::tiles)
            .sum::<u64>()
            .checked_sub(u64::try_from(self.excluded.len()).unwrap_or(u64::MAX))
            .is_some_and(|tiles| (1..=MAX_AREA_TILES).contains(&tiles))
    }
}
