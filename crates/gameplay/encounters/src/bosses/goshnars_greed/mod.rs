//! Crystal Goshnar's Greed room: soul chain and Greedbeast/Soul Sphere immunity cycle.
//! See packet W01a-2-greed-souls.

use crate::bosses::rotten_blood::RoomPosition;
use std::collections::BTreeMap;

pub const GREEDBEAST: &str = "oteryn:creature.greedbeast";
pub const SOULSNATCHER: &str = "oteryn:creature.soulsnatcher";
pub const WEAK_SOUL: &str = "oteryn:creature.weak_soul";
pub const STRONG_SOUL: &str = "oteryn:creature.strong_soul";
pub const POWERFUL_SOUL: &str = "oteryn:creature.powerful_soul";
pub const SOUL_SPHERE: &str = "oteryn:creature.soul_sphere";
pub const GOSHNARS_GREED: &str = "oteryn:creature.goshnar_s_greed";
pub const TRANSFORM_MS: u64 = 8_000;
pub const SPHERE_STEP_MS: u64 = 3_000;
pub const IMMUNE_RELEASE_MS: u64 = 34_000;
pub const GREEDBEAST_KILLS_FOR_SPHERE: u32 = 5;
pub const BOSS_SPEED: u16 = 200;
pub const REFLECT_PERCENT_PER_ELEMENT: u8 = 10;
pub const SOULSNATCHER_DAMAGE_MIN: u32 = 500;
pub const SOULSNATCHER_DAMAGE_MAX: u32 = 1_000;
pub const BOSS_HOME: RoomPosition = RoomPosition {
    x: 33746,
    y: 31666,
    floor: 14,
};
pub const BOSS_SPHERE_SPOT: RoomPosition = RoomPosition {
    x: 33741,
    y: 31659,
    floor: 14,
};
pub const SPHERE_SPAWN: RoomPosition = RoomPosition {
    x: 33752,
    y: 31659,
    floor: 14,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum GreedMinion {
    Greedbeast,
    Soulsnatcher,
    WeakSoul,
    StrongSoul,
    PowerfulSoul,
}

impl GreedMinion {
    pub fn from_creature_key(key: &str) -> Option<Self> {
        match key {
            GREEDBEAST => Some(Self::Greedbeast),
            SOULSNATCHER => Some(Self::Soulsnatcher),
            WEAK_SOUL => Some(Self::WeakSoul),
            STRONG_SOUL => Some(Self::StrongSoul),
            POWERFUL_SOUL => Some(Self::PowerfulSoul),
            _ => None,
        }
    }

    pub const fn creature_key(self) -> &'static str {
        match self {
            Self::Greedbeast => GREEDBEAST,
            Self::Soulsnatcher => SOULSNATCHER,
            Self::WeakSoul => WEAK_SOUL,
            Self::StrongSoul => STRONG_SOUL,
            Self::PowerfulSoul => POWERFUL_SOUL,
        }
    }

    pub const fn spawn_position(self) -> RoomPosition {
        match self {
            Self::Greedbeast => RoomPosition {
                x: 33744,
                y: 31666,
                floor: 14,
            },
            Self::Soulsnatcher => RoomPosition {
                x: 33747,
                y: 31668,
                floor: 14,
            },
            Self::WeakSoul | Self::StrongSoul | Self::PowerfulSoul => RoomPosition {
                x: 33750,
                y: 31666,
                floor: 14,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GreedEffect {
    TeleportMagic {
        position: RoomPosition,
    },
    SpawnMonster {
        creature_key: &'static str,
        position: RoomPosition,
        forced: bool,
    },
    RemoveMonster {
        instance: u64,
    },
    PlayerHealthLoss {
        actor: u64,
        amount: u32,
    },
    /// Adds 10% reflection to physical, fire, earth, energy, ice, holy, and death.
    BossBuff {
        reflect_percent_per_element: u8,
        defense: i32,
        max_health: i32,
        health: i32,
    },
    BossImmune {
        immune: bool,
    },
    BossTeleport {
        position: RoomPosition,
    },
    BossSpeed {
        speed: u16,
    },
    RemoveSoulSphere,
    SphereMove {
        instance: u64,
        position: RoomPosition,
    },
    BossHealFull,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GreedRefusal {
    InvalidDamageDraw,
}

#[derive(Clone, Debug, Default)]
pub struct GoshnarsGreedRoom {
    scheduled: Vec<(u64, GreedEffect)>,
    minion_ms: BTreeMap<u64, u64>,
    sphere: BTreeMap<u64, (u64, bool)>,
    greedbeast_kills: u32,
    sphere_kills: u32,
    sphere_present: bool,
    immune_cycle: Option<u64>,
}

impl GoshnarsGreedRoom {
    /// Lever `onUseExtra`: schedules the room's three opening greed spawns.
    pub fn lever_pulled(&mut self, now_ms: u64) {
        self.schedule_greed_spawn(GreedMinion::Greedbeast, now_ms);
        self.schedule_greed_spawn(GreedMinion::Soulsnatcher, now_ms);
        self.schedule_greed_spawn(GreedMinion::WeakSoul, now_ms);
    }

    /// Goshnar's Greed spawn callback: resets room counters and makes the boss immune.
    pub fn boss_spawned(&mut self) -> Vec<GreedEffect> {
        self.minion_ms.clear();
        self.sphere.clear();
        self.greedbeast_kills = 0;
        self.sphere_kills = 0;
        self.sphere_present = false;
        self.immune_cycle = None;
        vec![GreedEffect::BossImmune { immune: true }]
    }

    /// Advances one minion callback; `zone_players` are actors inside the boss zone.
    pub fn minion_think(
        &mut self,
        instance: u64,
        minion: GreedMinion,
        interval_ms: u64,
        now_ms: u64,
        zone_players: &[u64],
        mut draw: impl FnMut() -> u32,
    ) -> Result<Vec<GreedEffect>, GreedRefusal> {
        if minion == GreedMinion::Greedbeast {
            self.minion_ms.remove(&instance);
            // Greedbeast has no think callback in the Crystal donor.
            return Ok(vec![]);
        }

        let mut next = self.clone();
        let elapsed = next.minion_ms.get(&instance).copied().unwrap_or(0) + interval_ms;
        if elapsed < TRANSFORM_MS {
            next.minion_ms.insert(instance, elapsed);
            *self = next;
            return Ok(vec![]);
        }
        next.minion_ms.remove(&instance);

        let effects = match minion {
            GreedMinion::WeakSoul => vec![
                GreedEffect::SpawnMonster {
                    creature_key: STRONG_SOUL,
                    position: minion.spawn_position(),
                    forced: false,
                },
                GreedEffect::RemoveMonster { instance },
            ],
            GreedMinion::StrongSoul => vec![
                GreedEffect::SpawnMonster {
                    creature_key: POWERFUL_SOUL,
                    position: minion.spawn_position(),
                    forced: false,
                },
                GreedEffect::RemoveMonster { instance },
            ],
            GreedMinion::PowerfulSoul => {
                next.schedule_greed_spawn(GreedMinion::WeakSoul, now_ms);
                vec![
                    GreedEffect::RemoveMonster { instance },
                    GreedEffect::BossBuff {
                        reflect_percent_per_element: REFLECT_PERCENT_PER_ELEMENT,
                        defense: 10,
                        max_health: 10_000,
                        health: 10_000,
                    },
                ]
            }
            GreedMinion::Soulsnatcher => {
                let mut effects = Vec::with_capacity(zone_players.len() + 1);
                for &actor in zone_players {
                    let amount = draw();
                    if !(SOULSNATCHER_DAMAGE_MIN..=SOULSNATCHER_DAMAGE_MAX).contains(&amount) {
                        return Err(GreedRefusal::InvalidDamageDraw);
                    }
                    effects.push(GreedEffect::PlayerHealthLoss { actor, amount });
                }
                next.schedule_greed_spawn(GreedMinion::Soulsnatcher, now_ms);
                effects.push(GreedEffect::RemoveMonster { instance });
                effects
            }
            GreedMinion::Greedbeast => vec![],
        };
        *self = next;
        Ok(effects)
    }

    pub fn minion_died(&mut self, instance: u64, minion: GreedMinion, now_ms: u64) {
        self.minion_ms.remove(&instance);
        if minion == GreedMinion::Greedbeast {
            self.greedbeast_kills += 1;
        }
        self.schedule_greed_spawn(minion, now_ms);
    }

    pub fn sphere_think(
        &mut self,
        instance: u64,
        interval_ms: u64,
        position: RoomPosition,
        west_tile_has_boss: bool,
    ) -> Vec<GreedEffect> {
        let state = self.sphere.entry(instance).or_insert((0, false));
        if state.1 {
            return vec![];
        }
        state.0 += interval_ms;
        if state.0 < SPHERE_STEP_MS {
            return vec![];
        }
        if west_tile_has_boss {
            state.1 = true;
            return vec![GreedEffect::BossHealFull];
        }
        state.0 = 0;
        vec![GreedEffect::SphereMove {
            instance,
            position: RoomPosition {
                x: position.x - 1,
                ..position
            },
        }]
    }

    pub fn sphere_died(&mut self, instance: u64) {
        self.sphere_kills += 1;
        self.sphere.remove(&instance);
    }

    pub fn boss_think(&mut self, interval_ms: u64) -> Vec<GreedEffect> {
        let mut effects = Vec::new();
        if self.sphere_kills >= 1 {
            self.sphere_kills = 0;
            effects.extend(self.release_boss());
        }
        if self.greedbeast_kills >= GREEDBEAST_KILLS_FOR_SPHERE {
            self.greedbeast_kills = 0;
            self.immune_cycle = Some(0);
            self.sphere_present = true;
            effects.extend([
                GreedEffect::BossImmune { immune: true },
                GreedEffect::BossTeleport {
                    position: BOSS_SPHERE_SPOT,
                },
                GreedEffect::BossSpeed { speed: 0 },
                GreedEffect::SpawnMonster {
                    creature_key: SOUL_SPHERE,
                    position: SPHERE_SPAWN,
                    forced: true,
                },
            ]);
        }
        if let Some(elapsed) = self.immune_cycle {
            let elapsed = elapsed + interval_ms;
            self.immune_cycle = Some(elapsed);
            if self.sphere_present && elapsed >= IMMUNE_RELEASE_MS {
                effects.extend(self.release_boss());
            }
        }
        effects
    }

    /// Releases due greed-spawn effects in due-time and insertion order.
    pub fn advance(&mut self, now_ms: u64) -> Vec<GreedEffect> {
        self.scheduled.sort_by_key(|(due_ms, _)| *due_ms);
        let ready = self
            .scheduled
            .partition_point(|(due_ms, _)| *due_ms <= now_ms);
        self.scheduled
            .drain(..ready)
            .map(|(_, effect)| effect)
            .collect()
    }

    fn schedule_greed_spawn(&mut self, minion: GreedMinion, now_ms: u64) {
        let position = minion.spawn_position();
        for delay in [7_000, 8_000, 9_000] {
            self.scheduled
                .push((now_ms + delay, GreedEffect::TeleportMagic { position }));
        }
        self.scheduled.push((
            now_ms + 10_000,
            GreedEffect::SpawnMonster {
                creature_key: minion.creature_key(),
                position,
                forced: false,
            },
        ));
    }

    fn release_boss(&mut self) -> Vec<GreedEffect> {
        let mut effects = Vec::with_capacity(4);
        if self.sphere_present {
            effects.push(GreedEffect::RemoveSoulSphere);
        }
        effects.extend([
            GreedEffect::BossImmune { immune: false },
            GreedEffect::BossTeleport {
                position: BOSS_HOME,
            },
            GreedEffect::BossSpeed { speed: BOSS_SPEED },
        ]);
        self.sphere_present = false;
        self.immune_cycle = None;
        effects
    }
}

#[cfg(test)]
mod tests;
