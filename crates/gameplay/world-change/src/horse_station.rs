//! Crystal201189f horse-station NPC/creature movement rules; never permanent ownership.
use serde::{Deserialize, Serialize};
pub const PROVENANCE: &str = "crystal-201189f-horse-station-rental-v1";
pub const RENTAL_SECONDS: i64 = 86_400;
pub const WAVE_SECONDS: i64 = 10_800;
pub const RENTAL_GOLD: u64 = 500;
pub const RNG_RENTAL: u16 = 217;
pub const RNG_ESCAPE: u16 = 218;
pub const RNG_CATCH: u16 = 219;
pub const RNG_SPAWN_X: u16 = 220;
pub const RNG_SPAWN_Y: u16 = 221;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Horse {
    Regular,
    Gray,
    Brown,
}
impl Horse {
    pub fn for_key(key: &str) -> Option<Self> {
        match key {
            "oteryn:content.mount.rented_horse" => Some(Self::Regular),
            "oteryn:content.mount.rented_horse_gray" => Some(Self::Gray),
            "oteryn:content.mount.rented_horse_brown" => Some(Self::Brown),
            _ => None,
        }
    }
    pub const fn from_source_id(id: u16) -> Option<Self> {
        match id {
            22 => Some(Self::Regular),
            25 => Some(Self::Gray),
            26 => Some(Self::Brown),
            _ => None,
        }
    }
    pub const fn source_id(self) -> u16 {
        match self {
            Self::Regular => 22,
            Self::Gray => 25,
            Self::Brown => 26,
        }
    }
    pub const fn look(self) -> u16 {
        match self {
            Self::Regular => 421,
            Self::Gray => 437,
            Self::Brown => 438,
        }
    }
    pub const fn premium(self) -> bool {
        !matches!(self, Self::Regular)
    }
    pub const fn key(self) -> &'static str {
        match self {
            Self::Regular => "oteryn:content.mount.rented_horse",
            Self::Gray => "oteryn:content.mount.rented_horse_gray",
            Self::Brown => "oteryn:content.mount.rented_horse_brown",
        }
    }
    pub const fn grant(self) -> &'static str {
        match self {
            Self::Regular => "oteryn:rental.horse.22",
            Self::Gray => "oteryn:rental.horse.25",
            Self::Brown => "oteryn:rental.horse.26",
        }
    }
}
pub fn rental_choice(source_roll: u8) -> Option<Horse> {
    match source_roll {
        1 => Some(Horse::Regular),
        2 => Some(Horse::Gray),
        3 => Some(Horse::Brown),
        _ => None,
    }
}
pub fn supported_npc(key: &str) -> bool {
    matches!(key, "oteryn:npc.appaloosa" | "oteryn:npc.palomino")
}
/// Donor Timer>=now blocks renewal, Timer<now expires: equality remains live.
pub fn contract_live(expires: i64, now: i64) -> bool {
    expires > 0 && expires >= now
}
pub fn escape_trigger(roll: u8) -> Option<bool> {
    (1..=10).contains(&roll).then_some(roll <= 3)
}
pub fn catch_result(roll: u8) -> Option<bool> {
    (1..=30).contains(&roll).then_some(roll == 1)
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StationState {
    pub generation: u64,
    pub started: i64,
    pub expires: i64,
    pub ended: bool,
}
impl StationState {
    pub fn active(self, now: i64) -> bool {
        !self.ended && now < self.expires
    }
    pub fn begin(previous: Option<Self>, now: i64) -> Option<Self> {
        if now <= 0 || previous.is_some_and(|s| s.active(now)) {
            return None;
        }
        Some(Self {
            generation: previous.map_or(Some(1), |s| s.generation.checked_add(1))?,
            started: now,
            expires: now.checked_add(RENTAL_SECONDS)?,
            ended: false,
        })
    }
    pub fn wave(self, now: i64) -> Option<u8> {
        if !self.active(now) || now < self.started {
            return None;
        }
        u8::try_from((now - self.started) / WAVE_SECONDS).ok()
    }
    pub fn catch(self, now: i64, roll: u8) -> Option<Self> {
        if !self.active(now) || !catch_result(roll)? {
            return None;
        }
        Some(Self {
            ended: true,
            ..self
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceArea {
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
}
pub const RAID_AREAS: [SourceArea; 2] = [
    SourceArea {
        min_x: 32456,
        max_x: 32491,
        min_y: 32193,
        max_y: 32261,
    },
    SourceArea {
        min_x: 32431,
        max_x: 32464,
        min_y: 32240,
        max_y: 32280,
    },
];
pub const CLEAR_STABLE_AREAS: [SourceArea; 2] = [
    SourceArea {
        min_x: 32437,
        max_x: 32448,
        min_y: 32230,
        max_y: 32239,
    },
    SourceArea {
        min_x: 32846,
        max_x: 32850,
        min_y: 32114,
        max_y: 32120,
    },
];
pub const STABLE_AREAS: [SourceArea; 2] = [
    SourceArea {
        min_x: 32438,
        max_x: 32442,
        min_y: 32231,
        max_y: 32238,
    },
    SourceArea {
        min_x: 32846,
        max_x: 32850,
        min_y: 32114,
        max_y: 32120,
    },
];
pub const WAVE_CREATURES: [&str; 4] = [
    "oteryn:creature.wild_horse",
    "oteryn:creature.horse",
    "oteryn:creature.grey_horse",
    "oteryn:creature.brown_horse",
];
pub const STABLE_CREATURES: [&[&str]; 2] = [
    &[
        "oteryn:creature.horse",
        "oteryn:creature.horse",
        "oteryn:creature.horse",
        "oteryn:creature.grey_horse",
        "oteryn:creature.brown_horse",
        "oteryn:creature.brown_horse",
    ],
    &[
        "oteryn:creature.horse",
        "oteryn:creature.grey_horse",
        "oteryn:creature.brown_horse",
    ],
];
pub const EXPIRY_TEXT: &str =
    "Your contract with your horse expired and it returned back to the horse station.";
pub const ASK: &str = "Do you want to rent a horse for one day at a price of 500 gold?";
pub const SUCCESS: &str =
    "I'll give you one of our experienced ones. Take care! Look out for low hanging branches.";
pub const LOOSE: &str = "Right now our horses are on the loose. As long as not enough horses are chased back into the barn there are no horses to rent.";
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srv7_horse_rental_source_three_choices_and_inclusive_real_time_expiry() {
        assert_eq!(
            (1..=3)
                .filter_map(rental_choice)
                .map(Horse::source_id)
                .collect::<Vec<_>>(),
            [22, 25, 26]
        );
        assert_eq!(rental_choice(0), None);
        assert_eq!(rental_choice(4), None);
        assert!(contract_live(86401, 86401));
        assert!(!contract_live(86401, 86402));
        assert!(!Horse::Regular.premium());
        assert!(Horse::Gray.premium());
        assert!(Horse::Brown.premium());
    }
    #[test]
    fn srv7_horse_station_source_trigger_catch_wave_and_stable_counts() -> Result<(), &'static str>
    {
        assert_eq!(
            (1..=10).filter_map(escape_trigger).filter(|b| *b).count(),
            3
        );
        assert_eq!((1..=30).filter_map(catch_result).filter(|b| *b).count(), 1);
        assert_eq!(RAID_AREAS.len() * 3 * WAVE_CREATURES.len(), 24);
        assert_eq!(STABLE_CREATURES.iter().map(|c| c.len()).sum::<usize>(), 9);
        let state = StationState::begin(None, 1).ok_or("valid source time")?;
        assert_eq!(state.wave(1), Some(0));
        assert_eq!(state.wave(10800), Some(0));
        assert_eq!(state.wave(10801), Some(1));
        assert_eq!(state.wave(86400), Some(7));
        assert_eq!(state.wave(86401), None);
        assert!(state.catch(2, 2).is_none());
        assert!(state.catch(2, 1).is_some_and(|s| !s.active(2)));
        assert!(StationState::begin(Some(state), 2).is_none());
        Ok(())
    }
}
