//! Source-bounded raid placement. The caller supplies current map/occupancy facts;
//! a candidate is never a placement grant.
use super::RaidError;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};

/// Crystal 201189f src/lua/creature/raids.hpp:35-38.
pub const RAID_PLACEMENT_ATTEMPTS: u8 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RaidPosition {
    pub x: i32,
    pub y: i32,
    pub floor: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RaidArea {
    pub from: RaidPosition,
    pub to: RaidPosition,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RaidTileFacts {
    pub blocked: bool,
    pub protection_zone: bool,
    pub house: bool,
    pub stairs: bool,
    pub occupied: bool,
}

impl RaidTileFacts {
    pub fn admits(self) -> bool {
        !self.blocked && !self.protection_zone && !self.house && !self.stairs && !self.occupied
    }
}

/// Every draw is bound to the same persisted firing. Re-reading a step does
/// not choose another variant, amount or candidate sequence.
pub struct RaidRandom {
    root: GameplayDecisionRoot,
    firing: DecisionOccurrenceId,
}

impl RaidRandom {
    pub fn new(seed: [u8; 32], firing: [u8; 16]) -> Self {
        Self {
            root: GameplayDecisionRoot::from_bytes(seed),
            firing: DecisionOccurrenceId::from_bytes(firing),
        }
    }

    fn below(&self, purpose: &str, index: u64, bound: u64) -> Result<u64, RaidError> {
        if bound == 0 {
            return Err(RaidError::InvalidDefinition);
        }
        // Reject the short prefix instead of introducing modulo bias. The
        // rejection domain does not consume another candidate's draw identity.
        let threshold = bound.wrapping_neg() % bound;
        let mut rejection = 0_u64;
        loop {
            let draw = deterministic_decision_u64(
                &self.root,
                self.firing,
                &format!("raid-{purpose}-{rejection}"),
                index,
            )
            .map_err(|_| RaidError::InvalidState)?;
            if draw >= threshold {
                return Ok(draw % bound);
            }
            rejection = rejection.checked_add(1).ok_or(RaidError::Exhausted)?;
        }
    }

    pub fn variant(&self, count: usize) -> Result<usize, RaidError> {
        usize::try_from(self.below(
            "variant",
            0,
            u64::try_from(count).map_err(|_| RaidError::Exhausted)?,
        )?)
        .map_err(|_| RaidError::Exhausted)
    }

    /// Crystal 201189f src/lua/creature/raids.cpp:521-547,552-555:
    /// each entry selects its requested amount once, inclusively.
    pub fn amount(&self, wave: u16, entry: u16, min: u16, max: u16) -> Result<u16, RaidError> {
        if min > max {
            return Err(RaidError::InvalidDefinition);
        }
        let index = (u64::from(wave) << 16) | u64::from(entry);
        let draw = self.below("amount", index, u64::from(max) - u64::from(min) + 1)?;
        u16::try_from(u64::from(min) + draw).map_err(|_| RaidError::Exhausted)
    }

    /// Crystal 201189f data/libs/systems/zones.lua:19-36: choose one
    /// position from the current map owner's already filtered zone positions.
    /// The owner enumerates the source rectangle once (Sugar Mommy:110 tiles)
    /// and revalidates the selected position at physical admission.
    pub fn zone_position(
        &self,
        wave: u16,
        entry: u16,
        spawn_index: u16,
        valid_positions: &[RaidPosition],
    ) -> Result<Option<RaidPosition>, RaidError> {
        if valid_positions.is_empty() {
            return Ok(None);
        }
        let index = (u64::from(wave) << 32) | (u64::from(entry) << 16) | u64::from(spawn_index);
        let selected = self.below(
            "zone-position",
            index,
            u64::try_from(valid_positions.len()).map_err(|_| RaidError::Exhausted)?,
        )?;
        Ok(valid_positions
            .get(usize::try_from(selected).map_err(|_| RaidError::Exhausted)?)
            .copied())
    }
}

impl RaidArea {
    pub fn validate(self) -> Result<(), RaidError> {
        if self.from.x > self.to.x || self.from.y > self.to.y || self.from.floor > self.to.floor {
            return Err(RaidError::InvalidDefinition);
        }
        Ok(())
    }

    pub fn contains(self, at: RaidPosition) -> bool {
        (self.from.x..=self.to.x).contains(&at.x)
            && (self.from.y..=self.to.y).contains(&at.y)
            && (self.from.floor..=self.to.floor).contains(&at.floor)
    }

    /// Crystal 201189f src/lua/creature/raids.cpp:391-405: a single
    /// spawn attempts its declared tile once. The native map owner still
    /// applies the placement restrictions required by RAIDS-GLOBAL.
    pub fn place_single(
        self,
        facts: impl FnOnce(RaidPosition) -> Option<RaidTileFacts>,
    ) -> Result<Option<RaidPosition>, RaidError> {
        self.validate()?;
        if self.from != self.to {
            return Err(RaidError::InvalidDefinition);
        }
        Ok(facts(self.from)
            .is_some_and(RaidTileFacts::admits)
            .then_some(self.from))
    }

    pub fn candidate(
        self,
        random: &RaidRandom,
        wave: u16,
        entry: u16,
        spawn_index: u16,
        attempt: u8,
    ) -> Result<RaidPosition, RaidError> {
        self.validate()?;
        if attempt >= RAID_PLACEMENT_ATTEMPTS {
            return Err(RaidError::InvalidState);
        }
        let index = (u64::from(wave) << 48)
            | (u64::from(entry) << 32)
            | (u64::from(spawn_index) << 16)
            | u64::from(attempt);
        let coordinate = |purpose, from: i64, to: i64| -> Result<i64, RaidError> {
            let bound = u64::try_from(to - from + 1).map_err(|_| RaidError::Exhausted)?;
            let offset = random.below(purpose, index, bound)?;
            from.checked_add(i64::try_from(offset).map_err(|_| RaidError::Exhausted)?)
                .ok_or(RaidError::Exhausted)
        };
        Ok(RaidPosition {
            x: i32::try_from(coordinate(
                "x",
                i64::from(self.from.x),
                i64::from(self.to.x),
            )?)
            .map_err(|_| RaidError::Exhausted)?,
            y: i32::try_from(coordinate(
                "y",
                i64::from(self.from.y),
                i64::from(self.to.y),
            )?)
            .map_err(|_| RaidError::Exhausted)?,
            floor: i16::try_from(coordinate(
                "floor",
                i64::from(self.from.floor),
                i64::from(self.to.floor),
            )?)
            .map_err(|_| RaidError::Exhausted)?,
        })
    }

    /// Crystal 201189f src/lua/creature/raids.cpp:562-574. Failed
    /// placement is terminal for this requested monster, not a future timer.
    pub fn place(
        self,
        random: &RaidRandom,
        wave: u16,
        entry: u16,
        spawn_index: u16,
        mut facts: impl FnMut(RaidPosition) -> Option<RaidTileFacts>,
    ) -> Result<Option<RaidPosition>, RaidError> {
        for attempt in 0..RAID_PLACEMENT_ATTEMPTS {
            let at = self.candidate(random, wave, entry, spawn_index, attempt)?;
            if facts(at).is_some_and(RaidTileFacts::admits) {
                return Ok(Some(at));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> RaidArea {
        RaidArea {
            from: RaidPosition {
                x: 10,
                y: 20,
                floor: 6,
            },
            to: RaidPosition {
                x: 30,
                y: 40,
                floor: 8,
            },
        }
    }

    #[test]
    fn raid_placement_skips_blocked_pz_house_stairs_occupied_and_absent_tiles()
    -> Result<(), RaidError> {
        let random = RaidRandom::new([7; 32], [8; 16]);
        let rejected = [
            None,
            Some(RaidTileFacts {
                blocked: true,
                ..RaidTileFacts::default()
            }),
            Some(RaidTileFacts {
                protection_zone: true,
                ..RaidTileFacts::default()
            }),
            Some(RaidTileFacts {
                house: true,
                ..RaidTileFacts::default()
            }),
            Some(RaidTileFacts {
                stairs: true,
                ..RaidTileFacts::default()
            }),
            Some(RaidTileFacts {
                occupied: true,
                ..RaidTileFacts::default()
            }),
        ];
        let mut reads = 0;
        let placed = area().place(&random, 1, 2, 3, |at| {
            assert!(area().contains(at));
            let facts = rejected
                .get(reads)
                .copied()
                .unwrap_or(Some(RaidTileFacts::default()));
            reads += 1;
            facts
        })?;
        assert!(placed.is_some());
        assert_eq!(reads, 7);
        Ok(())
    }

    #[test]
    fn raid_full_area_counts_one_failure_after_ten_attempts() -> Result<(), RaidError> {
        let random = RaidRandom::new([7; 32], [8; 16]);
        let mut failures = 0;
        let mut reads = 0;
        for spawn_index in 0..4 {
            if area()
                .place(&random, 0, 0, spawn_index, |_| {
                    reads += 1;
                    Some(RaidTileFacts {
                        occupied: true,
                        ..RaidTileFacts::default()
                    })
                })?
                .is_none()
            {
                failures += 1;
            }
        }
        assert_eq!(failures, 4);
        assert_eq!(reads, 4 * usize::from(RAID_PLACEMENT_ATTEMPTS));
        Ok(())
    }

    #[test]
    fn raid_single_spawn_attempts_only_its_fixed_tile_once() -> Result<(), RaidError> {
        let fixed = RaidArea {
            from: area().from,
            to: area().from,
        };
        let mut attempts = 0;
        assert_eq!(
            fixed.place_single(|_| {
                attempts += 1;
                Some(RaidTileFacts {
                    blocked: true,
                    ..RaidTileFacts::default()
                })
            })?,
            None
        );
        assert_eq!(attempts, 1);
        assert_eq!(
            area().place_single(|_| Some(RaidTileFacts::default())),
            Err(RaidError::InvalidDefinition)
        );
        Ok(())
    }

    #[test]
    fn raid_seed_replays_one_variant_amount_and_position_sequence() -> Result<(), RaidError> {
        let first = RaidRandom::new([7; 32], [8; 16]);
        let replay = RaidRandom::new([7; 32], [8; 16]);
        assert_eq!(first.variant(3)?, replay.variant(3)?);
        assert!(first.variant(3)? < 3);
        assert_eq!(first.variant(0), Err(RaidError::InvalidDefinition));
        assert_eq!(first.amount(1, 2, 4, 9)?, replay.amount(1, 2, 4, 9)?);
        assert_eq!(first.amount(1, 2, 9, 9)?, 9);
        assert_eq!(first.amount(1, 2, 0, 0)?, 0);
        assert_eq!(first.amount(1, 2, 9, 4), Err(RaidError::InvalidDefinition));
        for attempt in 0..RAID_PLACEMENT_ATTEMPTS {
            assert_eq!(
                area().candidate(&first, 1, 2, 3, attempt)?,
                area().candidate(&replay, 1, 2, 3, attempt)?
            );
        }
        Ok(())
    }

    #[test]
    fn raid_zone_draw_is_from_valid_list_and_empty_zone_is_one_failed_request()
    -> Result<(), RaidError> {
        let random = RaidRandom::new([7; 32], [8; 16]);
        assert_eq!(random.zone_position(0, 0, 0, &[])?, None);
        assert_eq!(
            random.zone_position(0, 0, 0, &[area().from])?,
            Some(area().from)
        );
        let available = [area().from, area().to];
        let first = random
            .zone_position(0, 0, 0, &available)?
            .ok_or(RaidError::InvalidState)?;
        assert!(available.contains(&first));
        assert_eq!(random.zone_position(0, 0, 0, &available)?, Some(first));
        Ok(())
    }
}
