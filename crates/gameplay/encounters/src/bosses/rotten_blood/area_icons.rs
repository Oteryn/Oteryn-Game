//! Crystal `onRottenMonsterThink` (monster_onthink_area_icons.lua:1-207) for the 15 Rotten
//! Blood minions.

use super::RoomPosition;
use oteryn_gameplay_quests::quest::rotten_blood::RottenBloodState;
use std::collections::{BTreeMap, BTreeSet};

pub const SPECTATOR_RANGE_TILES: u16 = 15;
pub const HAZARD_REFRESH_SECONDS: u64 = 8;
pub const PARTY_CACHE_SECONDS: u64 = 10;
pub const AREA_STEP_SECONDS: u64 = 30;
pub const AREA_MAX_COUNT: u8 = 100;
pub const CROSS_STEP_SECONDS: u64 = 1;
pub const CROSS_TRIGGER_COUNT: u8 = 111;
pub const CROSS_DAMAGE_MIN: u32 = 1_300;
pub const CROSS_DAMAGE_MAX: u32 = 1_500;

/// The 15 creature keys whose think runs this helper (sorted).
pub use oteryn_gameplay_quests::quest::rotten_blood::area_icons::MINIONS as ROTTEN_AREA_MINIONS;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum RottenAreaIcon {
    Hazard,
    AreaArrowUp,
    WhiteCross,
}

impl RottenAreaIcon {
    pub const fn donor_id(self) -> &'static str {
        match self {
            Self::Hazard => "rotten-hazard",
            Self::AreaArrowUp => "rotten-area",
            Self::WhiteCross => "white-cross",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RottenParty<'a> {
    pub leader: u64,
    pub taint_levels: &'a [u8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RottenAreaPlayer<'a> {
    pub actor: u64,
    pub position: RoomPosition,
    pub taint_level: u8,
    pub party: Option<RottenParty<'a>>,
    pub alive: bool,
}

/// Read-only facts from the canonical named-icon owner and persistent player KV.
/// Counts are the full named icons, before the native visible-three projection.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RottenAreaCurrentFacts {
    pub actor: u64,
    pub last_hazard: u64,
    pub last_area: u64,
    pub last_cross: u64,
    pub area_count: Option<u8>,
    pub cross_count: Option<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RottenAreaEffect {
    SetIcon {
        actor: u64,
        icon: RottenAreaIcon,
        count: u8,
    },
    RemoveIcon {
        actor: u64,
        icon: RottenAreaIcon,
    },
    EnergyDamage {
        actor: u64,
        amount: u32,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RottenAreaRefusal {
    InvalidDamageDraw,
    InvalidCurrentFacts,
}

pub fn is_rotten_area_minion(creature_key: &str) -> bool {
    ROTTEN_AREA_MINIONS.binary_search(&creature_key).is_ok()
}

pub fn donor_taint_level(state: &RottenBloodState) -> u8 {
    state.regular_taints() + 5 * u8::from(state.final_taint())
}

pub const fn hazard_icon_level(taint_level: u8) -> Option<u8> {
    match taint_level {
        1..=4 => Some(taint_level),
        6..=9 => Some(taint_level - 5),
        _ => None,
    }
}

pub fn in_upper_left_area(position: RoomPosition) -> bool {
    oteryn_gameplay_quests::quest::rotten_blood::area_icons::in_upper_left_source_cell(
        position.x,
        position.y,
        position.floor,
    )
}
pub fn in_lower_left_area(position: RoomPosition) -> bool {
    oteryn_gameplay_quests::quest::rotten_blood::area_icons::in_lower_left_source_cell(
        position.x,
        position.y,
        position.floor,
    )
}

#[derive(Clone, Debug, Default)]
pub struct RottenAreaIconOwner {
    processed: BTreeSet<u64>,
    last_reset: u64,
    party_cache: BTreeMap<u64, (u8, u64)>,
}

impl RottenAreaIconOwner {
    /// Detached aliases do not own Players. The source compositor prunes these
    /// weak-cache identities when their existing10s cache lifetime has elapsed.
    pub fn retain_cached_identities(&mut self, identities: &[u64]) {
        self.processed.retain(|actor| identities.contains(actor));
        self.party_cache
            .retain(|leader, _| identities.contains(leader));
    }
    /// Current-owner entry point. Durable timestamps and canonical named icons are inputs,
    /// never a second per-player counter or timer store owned by this callback.
    pub fn think_current(
        &mut self,
        minion: RoomPosition,
        now: u64,
        spectators: &[RottenAreaPlayer<'_>],
        current: &[RottenAreaCurrentFacts],
        mut draw: impl FnMut() -> u32,
    ) -> Result<Vec<RottenAreaEffect>, RottenAreaRefusal> {
        let mut actors = BTreeSet::new();
        if current.iter().any(|facts| !actors.insert(facts.actor)) {
            return Err(RottenAreaRefusal::InvalidCurrentFacts);
        }
        let mut next = self.clone();
        let effects = next.think_inner(minion, now, spectators, current, &mut draw)?;
        *self = next;
        Ok(effects)
    }

    fn think_inner(
        &mut self,
        minion: RoomPosition,
        now: u64,
        spectators: &[RottenAreaPlayer<'_>],
        current: &[RottenAreaCurrentFacts],
        draw: &mut impl FnMut() -> u32,
    ) -> Result<Vec<RottenAreaEffect>, RottenAreaRefusal> {
        // Source values are weak and a retained value is usable for at most10s.
        // Removing expired detached DATA avoids retaining retired login aliases.
        self.party_cache
            .retain(|_, (_, at)| now.saturating_sub(*at) < PARTY_CACHE_SECONDS);
        if now.saturating_sub(self.last_reset) >= 1 {
            self.processed.clear();
            self.last_reset = now;
        }

        let mut effects = Vec::new();
        for player in spectators {
            if !minion.in_range(player.position, SPECTATOR_RANGE_TILES)
                || !self.processed.insert(player.actor)
            {
                continue;
            }

            let timers = current
                .iter()
                .find(|facts| facts.actor == player.actor)
                .ok_or(RottenAreaRefusal::InvalidCurrentFacts)?;
            if now.saturating_sub(timers.last_hazard) >= HAZARD_REFRESH_SECONDS {
                let taint = match player.party {
                    None => player.taint_level,
                    Some(party) => match self.party_cache.get(&party.leader) {
                        Some(&(level, cached_at))
                            if now.saturating_sub(cached_at) < PARTY_CACHE_SECONDS =>
                        {
                            level
                        }
                        _ => {
                            let level = party.taint_levels.iter().copied().min().unwrap_or(0);
                            self.party_cache.insert(party.leader, (level, now));
                            level
                        }
                    },
                };
                match hazard_icon_level(taint) {
                    Some(count) => effects.push(RottenAreaEffect::SetIcon {
                        actor: player.actor,
                        icon: RottenAreaIcon::Hazard,
                        count,
                    }),
                    None => effects.push(RottenAreaEffect::RemoveIcon {
                        actor: player.actor,
                        icon: RottenAreaIcon::Hazard,
                    }),
                }
            }

            if in_upper_left_area(player.position)
                && now.saturating_sub(timers.last_area) >= AREA_STEP_SECONDS
            {
                let count = timers
                    .area_count
                    .map_or(1, |count| count.saturating_add(1).min(AREA_MAX_COUNT));
                effects.push(RottenAreaEffect::SetIcon {
                    actor: player.actor,
                    icon: RottenAreaIcon::AreaArrowUp,
                    count,
                });
            }

            if in_lower_left_area(player.position)
                && now.saturating_sub(timers.last_cross) >= CROSS_STEP_SECONDS
            {
                match timers.cross_count {
                    None => {
                        effects.push(RottenAreaEffect::SetIcon {
                            actor: player.actor,
                            icon: RottenAreaIcon::WhiteCross,
                            count: 1,
                        });
                    }
                    Some(count) => {
                        let count = count.saturating_add(1).min(CROSS_TRIGGER_COUNT);
                        effects.push(RottenAreaEffect::SetIcon {
                            actor: player.actor,
                            icon: RottenAreaIcon::WhiteCross,
                            count,
                        });
                        if count >= CROSS_TRIGGER_COUNT {
                            let amount = draw();
                            if !(CROSS_DAMAGE_MIN..=CROSS_DAMAGE_MAX).contains(&amount) {
                                return Err(RottenAreaRefusal::InvalidDamageDraw);
                            }
                            effects.push(RottenAreaEffect::SetIcon {
                                actor: player.actor,
                                icon: RottenAreaIcon::WhiteCross,
                                count: 0,
                            });
                            if player.alive {
                                effects.push(RottenAreaEffect::EnergyDamage {
                                    actor: player.actor,
                                    amount,
                                });
                            }
                        }
                    }
                }
            }
        }
        Ok(effects)
    }

    /// rotten_area_login.lua:8-10: remove the three icons and forget icon counts; timers are kept.
    pub fn login(&mut self, actor: u64) -> Vec<RottenAreaEffect> {
        [
            RottenAreaIcon::Hazard,
            RottenAreaIcon::AreaArrowUp,
            RottenAreaIcon::WhiteCross,
        ]
        .into_iter()
        .map(|icon| RottenAreaEffect::RemoveIcon { actor, icon })
        .collect()
    }
}

#[cfg(test)]
#[path = "area_icons_tests.rs"]
mod tests;
