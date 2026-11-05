//! Crystal `Monster:tryTeleportToPlayer` (lib/quests/soul_war.lua:1286-1351) for six Soul War monsters.

use crate::bosses::rotten_blood::RoomPosition;
use oteryn_gameplay_quests::quest::soul_war::SoulWarState;
use std::collections::BTreeMap;

pub const TELEPORT_CHANCE_PERCENT: u32 = 10;
pub const SEARCH_RANGE_TILES: u32 = 30;
pub const TELEPORT_DELAY_MS: u64 = 2_000;
pub const COOLDOWN_MS: u64 = 10_000;

/// (creature key, line), sorted by key.
pub const TAINT_TELEPORT_MONSTERS: [(&str, &str); 6] = [
    ("oteryn:creature.bony_sea_devil", "Get out the way!"),
    ("oteryn:creature.brachiodemon", "Burn in hell!"),
    (
        "oteryn:creature.branchy_crawler",
        "My growth is your death!",
    ),
    ("oteryn:creature.cloak_of_terror", "I am your terror!"),
    ("oteryn:creature.many_faces", "Hands off my comrades!"),
    (
        "oteryn:creature.spiteful_spitter",
        "You have been chosen for a harvest!",
    ),
];

#[must_use]
pub fn teleport_line(creature_key: &str) -> Option<&'static str> {
    TAINT_TELEPORT_MONSTERS
        .binary_search_by_key(&creature_key, |(key, _)| key)
        .ok()
        .map(|index| TAINT_TELEPORT_MONSTERS[index].1)
}

/// PARTIAL wiring entry point for the later live think-loop packet.
#[must_use]
pub fn has_teleport_taint(state: SoulWarState, now_seconds: i64) -> bool {
    state.taint_count(now_seconds) >= 1
}

const fn in_box(position: RoomPosition, from: RoomPosition, to: RoomPosition) -> bool {
    position.x >= from.x
        && position.x <= to.x
        && position.y >= from.y
        && position.y <= to.y
        && position.floor >= from.floor
        && position.floor <= to.floor
}

#[must_use]
pub fn in_soul_war_hunting_zone(position: RoomPosition) -> bool {
    let hunting_zone = in_box(position, pos(33_982, 30_981, 9), pos(34_051, 31_110, 11))
        || in_box(position, pos(33_873, 30_994, 8), pos(33_968, 31_150, 9))
        || in_box(position, pos(33_814, 31_819, 3), pos(33_907, 31_920, 7))
        // Crystal's reversed x bounds produce only one column, despite world spawns to its west.
        // Normalize them so the intended rotten-wasteland box contains those admitted spawns.
        || in_box(position, pos(33_901, 30_986, 11), pos(33_980, 31_105, 12))
        || in_box(position, pos(33_877, 31_164, 9), pos(33_991, 31_241, 13));
    let safe_area = in_box(position, pos(34_002, 31_008, 9), pos(34_019, 31_019, 9))
        || in_box(position, pos(33_887, 31_015, 8), pos(33_920, 31_024, 8))
        || in_box(position, pos(33_854, 31_828, 3), pos(33_869, 31_834, 3))
        || in_box(position, pos(33_967, 31_037, 11), pos(33_977, 31_051, 11))
        || in_box(position, pos(33_884, 31_181, 10), pos(33_892, 31_198, 10));
    hunting_zone && !safe_area
}

const fn pos(x: i32, y: i32, floor: i16) -> RoomPosition {
    RoomPosition { x, y, floor }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TeleportCandidate {
    pub actor: u64,
    pub position: RoomPosition,
    pub has_teleport_taint: bool,
    pub in_boss_zone: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaintTeleportMagic {
    MortArea,
    Teleport,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaintTeleportEffect {
    Magic {
        position: RoomPosition,
        effect: TaintTeleportMagic,
    },
    Say {
        monster: u64,
        text: &'static str,
    },
    Teleport {
        monster: u64,
        destination: RoomPosition,
    },
}

/// Facts the caller reads when a pending teleport falls due. None means the monster or player no longer exists.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DueFacts {
    pub monster_position: RoomPosition,
    pub player_position: RoomPosition,
    pub destination_allows_teleport: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TaintTeleportRefusal {
    InvalidRoll,
}

#[derive(Clone, Debug)]
struct Pending {
    monster: u64,
    player: u64,
    text: &'static str,
    due_ms: u64,
}

#[derive(Clone, Debug, Default)]
pub struct TaintTeleportOwner {
    cooldowns: BTreeMap<u64, u64>,
    pending: Vec<Pending>,
}

impl TaintTeleportOwner {
    pub fn think(
        &mut self,
        monster: u64,
        creature_key: &str,
        position: RoomPosition,
        now_ms: u64,
        candidates: &[TeleportCandidate],
        mut roll: impl FnMut() -> u32,
    ) -> Result<Vec<TaintTeleportEffect>, TaintTeleportRefusal> {
        let Some(text) = teleport_line(creature_key) else {
            return Ok(Vec::new());
        };
        let rolled = roll();
        if !(1..=100).contains(&rolled) {
            return Err(TaintTeleportRefusal::InvalidRoll);
        }
        if rolled > TELEPORT_CHANCE_PERCENT {
            return Ok(Vec::new());
        }

        let mut farthest = 0_u32;
        let mut picked = None;
        for candidate in candidates {
            let distance = position
                .x
                .abs_diff(candidate.position.x)
                .max(position.y.abs_diff(candidate.position.y));
            if candidate.position.floor == position.floor
                && distance <= SEARCH_RANGE_TILES
                && candidate.has_teleport_taint
                && (in_soul_war_hunting_zone(candidate.position) || candidate.in_boss_zone)
                && distance > farthest
            {
                farthest = distance;
                picked = Some(candidate);
            }
        }
        let Some(candidate) = picked else {
            return Ok(Vec::new());
        };
        if self.cooldowns.contains_key(&candidate.actor) {
            return Ok(Vec::new());
        }

        self.cooldowns
            .insert(candidate.actor, now_ms.saturating_add(COOLDOWN_MS));
        self.pending.push(Pending {
            monster,
            player: candidate.actor,
            text,
            due_ms: now_ms.saturating_add(TELEPORT_DELAY_MS),
        });
        Ok(vec![
            TaintTeleportEffect::Magic {
                position,
                effect: TaintTeleportMagic::MortArea,
            },
            TaintTeleportEffect::Magic {
                position: candidate.position,
                effect: TaintTeleportMagic::MortArea,
            },
        ])
    }

    #[must_use]
    pub fn advance(
        &mut self,
        now_ms: u64,
        mut facts: impl FnMut(u64, u64) -> Option<DueFacts>,
        mut player_online: impl FnMut(u64) -> bool,
    ) -> Vec<TaintTeleportEffect> {
        let mut effects = Vec::new();
        let mut index = 0;
        while index < self.pending.len() {
            if self.pending[index].due_ms > now_ms {
                index += 1;
                continue;
            }
            let pending = self.pending.remove(index);
            if let Some(facts) = facts(pending.monster, pending.player)
                && facts.destination_allows_teleport
            {
                effects.extend([
                    TaintTeleportEffect::Say {
                        monster: pending.monster,
                        text: pending.text,
                    },
                    TaintTeleportEffect::Magic {
                        position: facts.monster_position,
                        effect: TaintTeleportMagic::Teleport,
                    },
                    TaintTeleportEffect::Teleport {
                        monster: pending.monster,
                        destination: facts.player_position,
                    },
                    TaintTeleportEffect::Magic {
                        position: facts.player_position,
                        effect: TaintTeleportMagic::Teleport,
                    },
                ]);
            }
        }
        self.cooldowns
            .retain(|player, clear_at| *clear_at > now_ms || !player_online(*player));
        effects
    }

    #[must_use]
    pub fn cooldown_active(&self, player: u64) -> bool {
        self.cooldowns.contains_key(&player)
    }
}

#[cfg(test)]
#[path = "soul_war_taint_teleport_tests.rs"]
mod tests;
