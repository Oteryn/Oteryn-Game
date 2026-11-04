//! Crystal "pillar of dark energy.lua":100-134.

use super::RoomPosition;
use std::collections::BTreeMap;

pub const PILLAR_KEY: &str = "oteryn:creature.pillar_of_dark_energy";
/// Bosses whose presence keeps a pillar (sorted).
pub const PILLAR_BOSSES: [&str; 3] = [
    "oteryn:creature.bakragore",
    "oteryn:creature.chagorz",
    "oteryn:creature.vemiath",
];
pub const PILLAR_LIFETIME_THINKS: u8 = 10;
pub const BOSS_PRESENCE_RANGE_TILES: u16 = 15;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PillarThink {
    Stay,
    Expired,
    BossGone,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PillarThinkOwner {
    thinks: BTreeMap<u64, u8>,
}

impl PillarThinkOwner {
    /// One think of pillar `pillar` at `own`; `monsters` are (creature key, position) pairs the
    /// caller read around it (any range; filtered here).
    pub fn think(
        &mut self,
        pillar: u64,
        own: RoomPosition,
        monsters: &[(&str, RoomPosition)],
    ) -> PillarThink {
        let thinks = self.thinks.entry(pillar).or_default();
        *thinks += 1;

        if *thinks >= PILLAR_LIFETIME_THINKS {
            self.thinks.remove(&pillar);
            return PillarThink::Expired;
        }

        let boss_alive = monsters.iter().any(|(creature_key, position)| {
            is_pillar_boss(creature_key) && own.in_range(*position, BOSS_PRESENCE_RANGE_TILES)
        });
        if !boss_alive {
            self.thinks.remove(&pillar);
            return PillarThink::BossGone;
        }

        PillarThink::Stay
    }

    /// A pillar that died or was removed by anything else; returns whether it was tracked.
    pub fn forget(&mut self, pillar: u64) -> bool {
        self.thinks.remove(&pillar).is_some()
    }

    #[must_use]
    pub fn thinks(&self, pillar: u64) -> Option<u8> {
        self.thinks.get(&pillar).copied()
    }
}

pub fn is_pillar_boss(creature_key: &str) -> bool {
    PILLAR_BOSSES.binary_search(&creature_key).is_ok()
}

#[cfg(test)]
#[path = "pillar_of_dark_energy_think_tests.rs"]
mod tests;
