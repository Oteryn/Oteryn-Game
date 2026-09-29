//! The location phrase of Find Person and Find Fiend, `locate_message`
//! (`docs/architecture/OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md` part B.3 P7, owner S27).
//!
//! A pure rule over the offset from the target to the caster. The spells that use it are not
//! admitted: Find Person needs the player-name parameter (no V1 cast-wire field) and Find Fiend the
//! Forge fiendish-monster registry (no owner). Bands follow F/TibiaMaps (5, 101, 251 tiles; QP2).

/// Distance band (`bands_tiles [5, 101, 251]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocateDistance {
    Beside,
    Close,
    Far,
    VeryFar,
}

/// The target's floor relative to the caster's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocateLevel {
    Higher,
    Same,
    Lower,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Compass {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

impl Compass {
    fn text(self) -> &'static str {
        match self {
            Self::North => "north",
            Self::NorthEast => "north-east",
            Self::East => "east",
            Self::SouthEast => "south-east",
            Self::South => "south",
            Self::SouthWest => "south-west",
            Self::West => "west",
            Self::NorthWest => "north-west",
        }
    }
}

/// Where the target is, seen from the caster.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Located {
    pub(crate) distance: LocateDistance,
    pub(crate) level: LocateLevel,
    /// `None` when the target is beside the caster.
    pub(crate) direction: Option<Compass>,
}

/// `direction_tangents [0.4142, 2.4142]`, as exact ten-thousandths.
const TANGENT_SCALE: i64 = 10_000;
const TANGENT_NARROW: i64 = 4_142;
const TANGENT_WIDE: i64 = 24_142;

/// Locate a target from `dx, dy, dz` = caster position minus target position (P7). `x` grows to the
/// east, `y` to the south and `z` downwards, so `dz > 0` means the target is higher.
pub(crate) fn locate(dx: i32, dy: i32, dz: i32) -> Located {
    let (dx, dy) = (i64::from(dx), i64::from(dy));
    let distance = match dx.abs().max(dy.abs()) {
        0..=4 => LocateDistance::Beside,
        5..=100 => LocateDistance::Close,
        101..=250 => LocateDistance::Far,
        _ => LocateDistance::VeryFar,
    };
    let level = match dz {
        1.. => LocateLevel::Higher,
        0 => LocateLevel::Same,
        _ => LocateLevel::Lower,
    };
    let direction = (distance != LocateDistance::Beside).then(|| {
        // t = dy / dx, and 10 when dx = 0; |t| is compared in exact integers.
        let (numerator, denominator) = if dx == 0 {
            (10, 1)
        } else {
            (dy.abs(), dx.abs())
        };
        if numerator * TANGENT_SCALE < TANGENT_NARROW * denominator {
            if dx > 0 { Compass::West } else { Compass::East }
        } else if numerator * TANGENT_SCALE < TANGENT_WIDE * denominator {
            // Here dx and dy are both non-zero; t > 0 when they share a sign.
            match (dx > 0, dy > 0) {
                (true, true) => Compass::NorthWest,
                (false, false) => Compass::SouthEast,
                (true, false) => Compass::SouthWest,
                (false, true) => Compass::NorthEast,
            }
        } else if dy > 0 {
            Compass::North
        } else {
            Compass::South
        }
    });
    Located {
        distance,
        level,
        direction,
    }
}

impl Located {
    /// The phrase after the target's name, without the final period (P7, F wording).
    pub(crate) fn phrase(&self) -> String {
        let direction = self.direction.map_or("", Compass::text);
        match (self.distance, self.level) {
            (LocateDistance::Beside, LocateLevel::Higher) => "is above you".to_owned(),
            (LocateDistance::Beside, LocateLevel::Same) => "is standing next to you".to_owned(),
            (LocateDistance::Beside, LocateLevel::Lower) => "is below you".to_owned(),
            (LocateDistance::Close, LocateLevel::Higher) => {
                format!("is on a higher level to the {direction}")
            }
            (LocateDistance::Close, LocateLevel::Same) => format!("is to the {direction}"),
            (LocateDistance::Close, LocateLevel::Lower) => {
                format!("is on a lower level to the {direction}")
            }
            (LocateDistance::Far, _) => format!("is far to the {direction}"),
            (LocateDistance::VeryFar, _) => format!("is very far to the {direction}"),
        }
    }
}
