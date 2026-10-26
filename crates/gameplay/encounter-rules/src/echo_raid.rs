//! Global-style Echo Raid scheduling and outcome rules.
//!
//! Echoes are world phenomena, deliberately represented independently of the item system.  The
//! channel owner can therefore keep them across floor-item resets and reject item placement based
//! on their occupied tile.

use std::collections::BTreeMap;

pub const RAID_LIFETIME_MS: u64 = 10 * 60 * 1_000;
pub const RESPAWN_MIN_MS: u64 = 20 * 60 * 1_000;
pub const RESPAWN_SPAN_MS: u64 = 10 * 60 * 1_000;
pub const REGULAR_COUNT: u8 = 8;
pub const INFLUENCED_COUNT: u8 = 4;
pub const WARDEN_REGULAR_COUNT: u8 = 4;
pub const WARDEN_AURA_DAMAGE_PERCENT: u8 = 20;
pub const WARDEN_DUST: u16 = 50;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Tile {
    pub x: u16,
    pub y: u16,
    pub floor: i8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EligibleSpawn {
    pub creature: String,
    pub tile: Tile,
    /// Compiler-proven on-foot reachability from the subarea entry.
    pub reachable: bool,
    pub bestiary_stars: u8,
    pub common_or_uncommon: bool,
}

impl EligibleSpawn {
    pub fn is_eligible(&self) -> bool {
        self.reachable && self.common_or_uncommon && (1..=2).contains(&self.bestiary_stars)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Echo {
    pub creature: String,
    pub tile: Tile,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Trigger {
    StepIn,
    Use,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Regular {
        count: u8,
    },
    Influenced {
        count: u8,
    },
    Warden {
        regular_count: u8,
        aura_damage_percent: u8,
        dust: u16,
        catalyst: bool,
        imbuement_scroll: bool,
    },
}

/// Select the current three-way Global outcome. `roll` is uniform in `0..100`.
///
/// Warden's base probability is ten percent and doubles for the player who completed this
/// creature's Bestiary entry. The remainder is split 3:1 between regular and influenced raids.
pub fn outcome(roll: u8, bestiary_complete: bool) -> Outcome {
    let warden_cutoff = if bestiary_complete { 20 } else { 10 };
    if roll % 100 < warden_cutoff {
        Outcome::Warden {
            regular_count: WARDEN_REGULAR_COUNT,
            aura_damage_percent: WARDEN_AURA_DAMAGE_PERCENT,
            dust: WARDEN_DUST,
            catalyst: true,
            imbuement_scroll: true,
        }
    } else if (roll % 100 - warden_cutoff).is_multiple_of(4) {
        Outcome::Influenced {
            count: INFLUENCED_COUNT,
        }
    } else {
        Outcome::Regular {
            count: REGULAR_COUNT,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SubareaState {
    Armed(Echo),
    Waiting { due_ms: u64 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Raid {
    pub creature: String,
    pub tile: Tile,
    pub outcome: Outcome,
    pub despawn_at_ms: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TriggerContext {
    pub trigger: Trigger,
    pub now_ms: u64,
    pub interval_roll: u64,
    pub outcome_roll: u8,
    pub bestiary_complete: bool,
}

/// Channel-owned state. One entry means exactly one echo or one replacement timer per subarea.
#[derive(Default)]
pub struct Coordinator {
    subareas: BTreeMap<u16, SubareaState>,
}

impl Coordinator {
    pub fn state(&self, subarea: u16) -> Option<&SubareaState> {
        self.subareas.get(&subarea)
    }

    /// Server-save seed, timer expiry, and the watchdog all use this operation. Invalid or
    /// unreachable compiler candidates are ignored rather than creating inaccessible echoes.
    pub fn arm(&mut self, subarea: u16, candidates: &[EligibleSpawn], choice: u64) -> bool {
        let eligible: Vec<_> = candidates
            .iter()
            .filter(|spawn| spawn.is_eligible())
            .collect();
        let Some(spawn) = eligible.get((choice as usize) % eligible.len().max(1)) else {
            return false;
        };
        self.subareas.insert(
            subarea,
            SubareaState::Armed(Echo {
                creature: spawn.creature.clone(),
                tile: spawn.tile,
            }),
        );
        true
    }

    /// Re-arms the September watchdog condition: neither an echo nor a timer exists.
    pub fn watchdog(&mut self, subarea: u16, candidates: &[EligibleSpawn], choice: u64) -> bool {
        !self.subareas.contains_key(&subarea) && self.arm(subarea, candidates, choice)
    }

    pub fn trigger(&mut self, subarea: u16, tile: Tile, context: TriggerContext) -> Option<Raid> {
        let SubareaState::Armed(echo) = self.subareas.get(&subarea)? else {
            return None;
        };
        if echo.tile != tile {
            return None;
        }
        let echo = echo.clone();
        let delay = RESPAWN_MIN_MS + context.interval_roll % (RESPAWN_SPAN_MS + 1);
        self.subareas.insert(
            subarea,
            SubareaState::Waiting {
                due_ms: context.now_ms.saturating_add(delay),
            },
        );
        Some(Raid {
            creature: echo.creature,
            tile,
            outcome: outcome(context.outcome_roll, context.bestiary_complete),
            despawn_at_ms: context.now_ms.saturating_add(RAID_LIFETIME_MS),
        })
    }

    /// Kill-created echoes occupy the corpse tile and never replace a subarea's timed echo.
    pub fn wild_echo(creature: impl Into<String>, tile: Tile, reachable: bool) -> Option<Echo> {
        reachable.then(|| Echo {
            creature: creature.into(),
            tile,
        })
    }

    pub fn blocks_item_placement(&self, tile: Tile) -> bool {
        self.subareas
            .values()
            .any(|state| matches!(state, SubareaState::Armed(echo) if echo.tile == tile))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spawn(reachable: bool) -> EligibleSpawn {
        EligibleSpawn {
            creature: "rat".into(),
            tile: Tile {
                x: 1,
                y: 2,
                floor: 7,
            },
            reachable,
            bestiary_stars: 1,
            common_or_uncommon: true,
        }
    }

    #[test]
    fn watchdog_arms_once_and_echo_is_not_an_item() {
        let mut owner = Coordinator::default();
        assert!(owner.watchdog(42, &[spawn(true)], 0));
        assert!(!owner.watchdog(42, &[spawn(true)], 0));
        assert!(owner.blocks_item_placement(spawn(true).tile));
    }

    #[test]
    fn unreachable_candidates_never_arm_or_make_wild_echoes() {
        let mut owner = Coordinator::default();
        assert!(!owner.arm(42, &[spawn(false)], 0));
        assert!(Coordinator::wild_echo("rat", spawn(false).tile, false).is_none());
    }

    #[test]
    fn use_and_step_share_the_atomic_trigger_and_twenty_to_thirty_minute_timer()
    -> Result<(), &'static str> {
        for trigger in [Trigger::StepIn, Trigger::Use] {
            let mut owner = Coordinator::default();
            assert!(owner.arm(7, &[spawn(true)], 0));
            let raid = owner
                .trigger(
                    7,
                    spawn(true).tile,
                    TriggerContext {
                        trigger,
                        now_ms: 5,
                        interval_roll: RESPAWN_SPAN_MS,
                        outcome_roll: 99,
                        bestiary_complete: false,
                    },
                )
                .ok_or("armed echo")?;
            assert_eq!(raid.outcome, Outcome::Regular { count: 8 });
            assert_eq!(raid.despawn_at_ms, 5 + RAID_LIFETIME_MS);
            assert_eq!(
                owner.state(7),
                Some(&SubareaState::Waiting {
                    due_ms: 5 + RESPAWN_MIN_MS + RESPAWN_SPAN_MS
                })
            );
            assert!(
                owner
                    .trigger(
                        7,
                        spawn(true).tile,
                        TriggerContext {
                            trigger,
                            now_ms: 6,
                            interval_roll: 0,
                            outcome_roll: 0,
                            bestiary_complete: false,
                        },
                    )
                    .is_none()
            );
        }
        Ok(())
    }

    #[test]
    fn completed_bestiary_doubles_warden_window_and_current_rewards_are_exact() {
        assert!(!matches!(outcome(15, false), Outcome::Warden { .. }));
        assert_eq!(
            outcome(15, true),
            Outcome::Warden {
                regular_count: 4,
                aura_damage_percent: 20,
                dust: 50,
                catalyst: true,
                imbuement_scroll: true,
            }
        );
        assert_eq!(outcome(22, false), Outcome::Influenced { count: 4 });
    }
}
