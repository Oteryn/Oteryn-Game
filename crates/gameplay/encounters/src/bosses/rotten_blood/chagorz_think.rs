//! Crystal chagorz.lua:166-367: summons, the 90 s centre teleport and agony stacks.

use super::{AgonyDamage, RoomPosition};
use std::collections::BTreeMap;

pub const CHAGORZ_KEY: &str = "oteryn:creature.chagorz";
pub const PILLAR_KEY: &str = "oteryn:creature.pillar_of_dark_energy";
pub const ELDER_BLOODJAW_KEY: &str = "oteryn:creature.elder_bloodjaw";
pub const CHAGORZ_ROOM_CENTRE: RoomPosition = RoomPosition {
    x: 33_043,
    y: 32_366,
    floor: 15,
};
pub const PILLAR_FLOOR: u8 = 4;
pub const PILLAR_COUNT_RANGE_TILES: u16 = 16;
pub const PILLAR_OFFSET_TILES: u8 = 1;
pub const BLOODJAW_OFFSET_TILES: u8 = 2;
pub const AGONY_RANGE_TILES: u16 = 16;
pub const AGONY_STACK_CAP: u8 = 21;
pub const TELEPORT_CYCLE_MS: u64 = 90_000;
pub const TELEPORT_LIFETIME_MS: u64 = 15_000;
pub const TELEPORT_ITEM_ID: u16 = 37_000;
pub const TELEPORT_ACTION_ID: u16 = 50_001;

/// Percentages of chagorz.lua:326-340; 0 for count 0.
pub const fn agony_percent(count: u8) -> u32 {
    match count {
        0 => 0,
        1..=14 => 3,
        15..=19 => 7,
        _ => 15,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AgonyPlayer {
    pub actor: u64,
    pub position: RoomPosition,
    pub max_health: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgonyEffect {
    SetStacks { actor: u64, count: u8 },
    Damage(AgonyDamage),
}

/// Shared by Chagorz, Vemiath and Bakragore (one per boss instance).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AgonyStacks {
    players: BTreeMap<u64, (RoomPosition, u8)>,
}

impl AgonyStacks {
    /// Runs one 1000 ms check for players on the centre's floor and within `range` tiles.
    pub fn check(
        &mut self,
        centre: RoomPosition,
        range: u16,
        players: &[AgonyPlayer],
    ) -> Vec<AgonyEffect> {
        let mut effects = Vec::new();
        for player in players {
            if !centre.in_range(player.position, range) {
                continue;
            }

            let first_sight = !self.players.contains_key(&player.actor);
            let (previous_position, count) = self
                .players
                .entry(player.actor)
                .or_insert((player.position, 0));
            if first_sight {
                effects.push(AgonyEffect::SetStacks {
                    actor: player.actor,
                    count: 0,
                });
            }

            if *previous_position == player.position {
                if *count < AGONY_STACK_CAP {
                    *count += 1;
                }
                effects.push(AgonyEffect::SetStacks {
                    actor: player.actor,
                    count: *count,
                });
                let amount = u64::from(player.max_health) * u64::from(agony_percent(*count)) / 100;
                let amount = u32::try_from(amount).unwrap_or(u32::MAX);
                if amount > 0 {
                    effects.push(AgonyEffect::Damage(AgonyDamage {
                        actor: player.actor,
                        amount,
                    }));
                }
            } else {
                *count = count.saturating_sub(1);
                effects.push(AgonyEffect::SetStacks {
                    actor: player.actor,
                    count: *count,
                });
            }
            *previous_position = player.position;
        }
        effects
    }

    /// Forgets every tracked player not listed in `present`.
    pub fn retain_present(&mut self, present: &[u64]) {
        self.players.retain(|actor, _| present.contains(actor));
    }

    #[must_use]
    pub fn stacks(&self, actor: u64) -> Option<u8> {
        self.players.get(&actor).map(|(_, count)| *count)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CentreTeleport {
    pub centre: RoomPosition,
    pub item_id: u16,
    pub action_id: u16,
    pub remove_at_ms: u64,
}

/// Shared 90 s centre-teleport cycle (Chagorz, Vemiath and Bakragore differ only in centre).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TeleportCycle {
    centre: RoomPosition,
    next_at_ms: Option<u64>,
}

impl TeleportCycle {
    #[must_use]
    pub const fn new(centre: RoomPosition) -> Self {
        Self {
            centre,
            next_at_ms: None,
        }
    }

    pub fn start(&mut self, now_ms: u64) {
        self.next_at_ms = Some(now_ms.saturating_add(TELEPORT_CYCLE_MS));
    }

    #[must_use]
    pub const fn next_at_ms(&self) -> Option<u64> {
        self.next_at_ms
    }

    pub fn due(
        &mut self,
        now_ms: u64,
        boss_present: bool,
        paused: bool,
        bloodjaw_present: bool,
    ) -> Option<CentreTeleport> {
        let next_at_ms = self.next_at_ms?;
        if now_ms < next_at_ms {
            return None;
        }
        if !boss_present {
            self.next_at_ms = None;
            return None;
        }
        self.next_at_ms = Some(now_ms.saturating_add(TELEPORT_CYCLE_MS));
        if paused || !bloodjaw_present {
            return None;
        }
        Some(CentreTeleport {
            centre: self.centre,
            item_id: TELEPORT_ITEM_ID,
            action_id: TELEPORT_ACTION_ID,
            remove_at_ms: now_ms.saturating_add(TELEPORT_LIFETIME_MS),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Placement {
    Close { offset_tiles: u8 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChagorzEffect {
    StartTeleportCycle,
    Spawn {
        creature_key: &'static str,
        around: RoomPosition,
        placement: Placement,
    },
    Agony(AgonyEffect),
}

#[derive(Clone, Copy, Debug)]
pub struct ChagorzThinkFacts<'a> {
    pub boss: RoomPosition,
    pub now_ms: u64,
    pub mechanics_disabled: bool,
    pub bloodjaw_spawn_paused: bool,
    pub bloodjaw_target: u8,
    /// Elder Bloodjaws within 15 tiles of the boss.
    pub nearby_bloodjaws: u8,
    /// Pillars within 16 tiles of the boss.
    pub nearby_pillars: u8,
    pub players: &'a [AgonyPlayer],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChagorzThinkOwner {
    initialized: bool,
    teleport: TeleportCycle,
    agony: AgonyStacks,
}

impl Default for ChagorzThinkOwner {
    fn default() -> Self {
        Self {
            initialized: false,
            teleport: TeleportCycle::new(CHAGORZ_ROOM_CENTRE),
            agony: AgonyStacks::default(),
        }
    }
}

impl ChagorzThinkOwner {
    pub fn think(&mut self, facts: &ChagorzThinkFacts<'_>) -> Vec<ChagorzEffect> {
        let mut effects = Vec::new();
        if !self.initialized {
            self.initialized = true;
            self.teleport.start(facts.now_ms);
            effects.push(ChagorzEffect::StartTeleportCycle);
        }
        if facts.mechanics_disabled {
            return effects;
        }
        if !facts.bloodjaw_spawn_paused {
            for _ in 0..facts.bloodjaw_target.saturating_sub(facts.nearby_bloodjaws) {
                effects.push(ChagorzEffect::Spawn {
                    creature_key: ELDER_BLOODJAW_KEY,
                    around: facts.boss,
                    placement: Placement::Close {
                        offset_tiles: BLOODJAW_OFFSET_TILES,
                    },
                });
            }
        }
        for _ in 0..PILLAR_FLOOR.saturating_sub(facts.nearby_pillars) {
            effects.push(ChagorzEffect::Spawn {
                creature_key: PILLAR_KEY,
                around: facts.boss,
                placement: Placement::Close {
                    offset_tiles: PILLAR_OFFSET_TILES,
                },
            });
        }
        effects.extend(
            self.agony
                .check(facts.boss, AGONY_RANGE_TILES, facts.players)
                .into_iter()
                .map(ChagorzEffect::Agony),
        );
        effects
    }

    pub fn teleport_mut(&mut self) -> &mut TeleportCycle {
        &mut self.teleport
    }

    #[must_use]
    pub fn agony(&self) -> &AgonyStacks {
        &self.agony
    }
}

#[cfg(test)]
#[path = "chagorz_think_tests.rs"]
mod tests;
