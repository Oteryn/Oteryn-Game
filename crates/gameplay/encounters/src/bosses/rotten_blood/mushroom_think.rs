//! Crystal mushroom.lua:101-172.

use super::{RoomPosition, RoomTarget};
use std::collections::BTreeMap;

pub const MUSHROOM_KEY: &str = "oteryn:creature.mushroom";
/// Bosses whose presence keeps a mushroom (sorted).
pub const MUSHROOM_BOSSES: [&str; 3] = [
    "oteryn:creature.bakragore",
    "oteryn:creature.ichgahal",
    "oteryn:creature.murcion",
];
pub const TRIGGER_RANGE_TILES: u16 = 1;
pub const TRIGGER_DELAY_MS: u64 = 1_000;
pub const FUSE_MS: u64 = 18_000;
pub const BLAST_RANGE_TILES: u16 = 3;
pub const BLAST_DRAIN_MIN: u32 = 2_500;
pub const BLAST_DRAIN_MAX: u32 = 3_000;
pub const BOSS_PRESENCE_RANGE_TILES: u16 = 15;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MushroomEffect {
    /// CONST_ME_MAGIC_GREEN on the mushroom.
    Primed {
        mushroom: u64,
    },
    Despawn {
        mushroom: u64,
    },
    /// CONST_ME_GREEN_RINGS over the 7x7 area centred on the mushroom.
    BlastVisual {
        mushroom: u64,
        centre: RoomPosition,
    },
    LifeDrain {
        mushroom: u64,
        target: u64,
        amount: u32,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MushroomRefusal {
    InvalidDrainDraw,
}

pub fn is_mushroom_boss(creature_key: &str) -> bool {
    MUSHROOM_BOSSES.binary_search(&creature_key).is_ok()
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MushroomThinkOwner {
    fuses: BTreeMap<u64, u64>,
}

impl MushroomThinkOwner {
    /// One think at `now_ms`. `players` and `monsters` are what the caller read around it.
    pub fn think(
        &mut self,
        mushroom: u64,
        own: RoomPosition,
        now_ms: u64,
        players: &[RoomTarget],
        monsters: &[(&str, RoomPosition)],
    ) -> Vec<MushroomEffect> {
        let mut effects = Vec::new();
        if players
            .iter()
            .any(|player| own.in_range(player.position, TRIGGER_RANGE_TILES))
        {
            effects.push(MushroomEffect::Primed { mushroom });
            self.keep_earliest_fuse(mushroom, now_ms.saturating_add(TRIGGER_DELAY_MS));
        }
        self.keep_earliest_fuse(mushroom, now_ms.saturating_add(FUSE_MS));

        let boss_present = monsters.iter().any(|(key, position)| {
            is_mushroom_boss(key) && own.in_range(*position, BOSS_PRESENCE_RANGE_TILES)
        });
        if !boss_present {
            self.fuses.remove(&mushroom);
            effects.push(MushroomEffect::Despawn { mushroom });
        }
        effects
    }

    #[must_use]
    pub fn explode_at(&self, mushroom: u64) -> Option<u64> {
        self.fuses.get(&mushroom).copied()
    }

    /// Fires the explosion if `now_ms >= explode_at`. `targets` are players and monsters the
    /// caller read around the mushroom (filtered here to 3 tiles, excluding the mushroom itself).
    /// One draw per hit target, in slice order. Returns `Ok(vec![])` when not due or untracked.
    pub fn explode_due(
        &mut self,
        mushroom: u64,
        own: RoomPosition,
        now_ms: u64,
        targets: &[RoomTarget],
        mut draw: impl FnMut() -> u32,
    ) -> Result<Vec<MushroomEffect>, MushroomRefusal> {
        let Some(explode_at) = self.explode_at(mushroom) else {
            return Ok(Vec::new());
        };
        if now_ms < explode_at {
            return Ok(Vec::new());
        }

        let mut drains = Vec::new();
        for target in targets.iter().filter(|target| {
            target.actor != mushroom && own.in_range(target.position, BLAST_RANGE_TILES)
        }) {
            let amount = draw();
            if !(BLAST_DRAIN_MIN..=BLAST_DRAIN_MAX).contains(&amount) {
                return Err(MushroomRefusal::InvalidDrainDraw);
            }
            drains.push((target.actor, amount));
        }

        let mut effects = Vec::with_capacity(drains.len() + 2);
        effects.push(MushroomEffect::BlastVisual {
            mushroom,
            centre: own,
        });
        effects.extend(
            drains
                .into_iter()
                .map(|(target, amount)| MushroomEffect::LifeDrain {
                    mushroom,
                    target,
                    amount,
                }),
        );
        effects.push(MushroomEffect::Despawn { mushroom });
        self.fuses.remove(&mushroom);
        Ok(effects)
    }

    pub fn forget(&mut self, mushroom: u64) -> bool {
        self.fuses.remove(&mushroom).is_some()
    }

    fn keep_earliest_fuse(&mut self, mushroom: u64, candidate: u64) {
        self.fuses
            .entry(mushroom)
            .and_modify(|explode_at| *explode_at = (*explode_at).min(candidate))
            .or_insert(candidate);
    }
}

#[cfg(test)]
#[path = "mushroom_think_tests.rs"]
mod tests;
