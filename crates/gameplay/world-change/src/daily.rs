//! W05b reset-selected daily world changes applied to a Channel's live map projection.

/// A position in the live world map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub x: u16,
    pub y: u16,
    pub z: u8,
}

impl Position {
    pub const fn new(x: u16, y: u16, z: u8) -> Self {
        Self { x, y, z }
    }
}

// Crystal 201189f8 `data-global/scripts/world_changes/thawing.lua:1,4-19`.
const ICE_FLOWER: u16 = 13_842;
const THAWING_POSITIONS: [Position; 14] = [
    Position::new(32_255, 31_079, 5),
    Position::new(32_255, 31_066, 5),
    Position::new(32_204, 31_054, 5),
    Position::new(32_192, 31_048, 5),
    Position::new(32_163, 31_030, 5),
    Position::new(32_145, 31_036, 5),
    Position::new(32_136, 31_049, 5),
    Position::new(32_122, 31_060, 5),
    Position::new(32_112, 31_071, 5),
    Position::new(32_120, 31_082, 5),
    Position::new(32_149, 31_061, 5),
    Position::new(32_155, 31_085, 5),
    Position::new(32_183, 31_091, 5),
    Position::new(32_217, 31_091, 5),
];

/// The reset owner's immutable map effects for one epoch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyWorldChange {
    reset_epoch: u64,
    thawing: bool,
}

impl DailyWorldChange {
    /// Applies Crystal's inclusive 10% roll
    /// (`data-global/scripts/world_changes/thawing.lua:2,31-35`). `roll` is supplied by the
    /// reset owner in `0..100`.
    pub fn roll(reset_epoch: u64, roll: u8) -> Self {
        Self {
            reset_epoch,
            thawing: roll < 10,
        }
    }

    pub fn reset_epoch(&self) -> u64 {
        self.reset_epoch
    }

    pub fn thawing_active(&self) -> bool {
        self.thawing
    }

    /// Returns the daily overlay item at a live-map position.
    pub fn item_at(&self, position: Position) -> Option<u16> {
        (self.thawing && THAWING_POSITIONS.contains(&position)).then_some(ICE_FLOWER)
    }

    pub fn placed_items(&self) -> impl Iterator<Item = (Position, u16)> + '_ {
        THAWING_POSITIONS
            .iter()
            .copied()
            .filter(|_| self.thawing)
            .map(|position| (position, ICE_FLOWER))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn w05b_thawing_roll_obeys_the_ten_percent_boundary() {
        assert!(DailyWorldChange::roll(7, 0).thawing_active());
        assert!(DailyWorldChange::roll(7, 9).thawing_active());
        assert!(!DailyWorldChange::roll(7, 10).thawing_active());
        assert!(!DailyWorldChange::roll(7, 99).thawing_active());
    }

    #[test]
    fn w05b_thawing_places_ice_flowers_only_on_its_day() {
        let active = DailyWorldChange::roll(7, 9);
        let inactive = DailyWorldChange::roll(8, 10);
        assert_eq!(
            active.item_at(Position::new(32_255, 31_079, 5)),
            Some(13_842)
        );
        assert_eq!(active.placed_items().count(), 14);
        assert_eq!(inactive.item_at(Position::new(32_255, 31_079, 5)), None);
        assert_eq!(inactive.placed_items().count(), 0);
        println!("W05B_MAP_EFFECT_EXECUTED active_items=14 inactive_items=0");
    }
}
