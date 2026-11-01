//! Character-owned Soul War progression. Crystal `201189f` supplies the flags and
//! fourteen-day window; accepted Global/Wiki decisions govern final success/failure.

pub const MINIMUM_LEVEL: u32 = 250;
pub const TAINT_DURATION_SECONDS: i64 = 14 * 24 * 60 * 60;
pub const ALL_MINI_BOSSES: u8 = 0b1_1111;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SoulWarBoss {
    Malice,
    Hatred,
    Spite,
    Cruelty,
    Greed,
    Megalomania,
}

impl SoulWarBoss {
    #[must_use]
    pub fn from_creature(key: &str) -> Option<Self> {
        Some(match key {
            "oteryn:creature.goshnar_s_malice" => Self::Malice,
            "oteryn:creature.goshnar_s_hatred" => Self::Hatred,
            "oteryn:creature.goshnar_s_spite" => Self::Spite,
            "oteryn:creature.goshnar_s_cruelty" => Self::Cruelty,
            "oteryn:creature.goshnar_s_greed" => Self::Greed,
            "oteryn:creature.goshnar_s_megalomania_blue"
            | "oteryn:creature.goshnar_s_megalomania_green" => Self::Megalomania,
            _ => return None,
        })
    }

    #[must_use]
    pub const fn bit(self) -> u8 {
        match self {
            Self::Malice => 1,
            Self::Hatred => 2,
            Self::Spite => 4,
            Self::Cruelty => 8,
            Self::Greed => 16,
            Self::Megalomania => 0,
        }
    }
}

/// One specialized projection owned by the Quest durability module, never by an instance.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SoulWarState {
    pub teleport_access: bool,
    pub defeated_bosses: u8,
    pub first_taint_at: Option<i64>,
    pub megalomania_completed: bool,
    pub final_reward_claimed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SoulWarTransition {
    /// Only the confirmed Flickering Soul dialogue consumer issues this action.
    AcceptTask,
    /// Only a captured, source-qualified creature death issues this action.
    BossDefeated(SoulWarBoss),
    /// The NPC's free "taints" / "penalties" action.
    Cleanse,
    /// Lazy expiry is also run at admission and before every state-changing action.
    Refresh,
    /// W02 alone proves the terminal failed attempt and retains the shared pair.
    FailedMegalomania { removed_bosses: u8 },
    /// The transactional reward owner must deliver the item before this commits.
    ClaimReward,
    /// Confirmed NPC completion dialogue grants the existing Revenant entitlement.
    ReportCompletion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SoulWarRefusal {
    InvalidState,
    InvalidTime,
    AccessRequired,
    BossChainRequired,
    InvalidFailurePair,
    CompletionRequired,
    RewardAlreadyClaimed,
}

impl SoulWarState {
    #[must_use]
    pub const fn access(self) -> bool {
        self.teleport_access
    }

    #[must_use]
    pub const fn chain_ready(self) -> bool {
        self.teleport_access && self.defeated_bosses == ALL_MINI_BOSSES
    }

    #[must_use]
    pub const fn completed(self) -> bool {
        self.megalomania_completed
    }

    #[must_use]
    pub const fn reward_claimed(self) -> bool {
        self.final_reward_claimed
    }

    pub fn validate(self) -> Result<(), SoulWarRefusal> {
        if self.defeated_bosses & !ALL_MINI_BOSSES != 0
            || (self.defeated_bosses == 0) != self.first_taint_at.is_none()
            || self.first_taint_at.is_some_and(|at| at < 0)
            || (!self.teleport_access && self.defeated_bosses != 0)
            || (self.final_reward_claimed && !self.megalomania_completed)
        {
            Err(SoulWarRefusal::InvalidState)
        } else {
            Ok(())
        }
    }

    /// The first mini-boss starts the window; later kills never extend it.
    #[must_use]
    pub fn taint_count(self, now_seconds: i64) -> u8 {
        if self.first_taint_at.is_some_and(|at| {
            i128::from(now_seconds) - i128::from(at) >= i128::from(TAINT_DURATION_SECONDS)
        }) {
            0
        } else {
            self.defeated_bosses.count_ones() as u8
        }
    }

    #[must_use]
    pub fn expired(self, now_seconds: i64) -> Self {
        if self.first_taint_at.is_some() && self.taint_count(now_seconds) == 0 {
            Self {
                defeated_bosses: 0,
                first_taint_at: None,
                ..self
            }
        } else {
            self
        }
    }

    pub fn apply(
        self,
        transition: SoulWarTransition,
        now_seconds: i64,
    ) -> Result<Self, SoulWarRefusal> {
        self.validate()?;
        if now_seconds < 0 {
            return Err(SoulWarRefusal::InvalidTime);
        }
        let mut next = self.expired(now_seconds);
        match transition {
            SoulWarTransition::AcceptTask => {
                // Flickering Soul accepts "yes" at any level. The entrance, not
                // the dialogue, enforces level 250 (Crystal NPC:122-138).
                next.teleport_access = true;
            }
            SoulWarTransition::BossDefeated(boss) => {
                if !next.teleport_access {
                    return Err(SoulWarRefusal::AccessRequired);
                }
                if boss == SoulWarBoss::Megalomania {
                    // Crystal soul_war_mechanics.lua:1051-1060 credits the
                    // verified native killers. The five active flags are checked
                    // at entry, not again after the admitted fight's clock expires
                    // (PR2784 ANSWER6548123594).
                    next.megalomania_completed = true;
                    next.defeated_bosses = 0;
                    next.first_taint_at = None;
                } else if next.defeated_bosses & boss.bit() == 0 {
                    next.defeated_bosses |= boss.bit();
                    next.first_taint_at.get_or_insert(now_seconds);
                }
            }
            SoulWarTransition::Cleanse => {
                next.defeated_bosses = 0;
                next.first_taint_at = None;
            }
            SoulWarTransition::Refresh => {}
            SoulWarTransition::FailedMegalomania { removed_bosses } => {
                // PR #2784 ANSWER 6547488765: one source-retained distinct pair,
                // shared across the failed group's participants; reconnect is not failure.
                if removed_bosses.count_ones() != 2 || removed_bosses & !ALL_MINI_BOSSES != 0 {
                    return Err(SoulWarRefusal::InvalidFailurePair);
                }
                if !next.chain_ready() {
                    return Err(SoulWarRefusal::BossChainRequired);
                }
                next.defeated_bosses &= !removed_bosses;
            }
            SoulWarTransition::ClaimReward => {
                if !next.megalomania_completed {
                    return Err(SoulWarRefusal::CompletionRequired);
                }
                if next.final_reward_claimed {
                    return Err(SoulWarRefusal::RewardAlreadyClaimed);
                }
                next.final_reward_claimed = true;
            }
            SoulWarTransition::ReportCompletion => {
                if !next.megalomania_completed {
                    return Err(SoulWarRefusal::CompletionRequired);
                }
            }
        }
        next.validate()?;
        Ok(next)
    }
}

/// Crystal `data-global/lib/quests/soul_war.lua:40-67`; ordinary outside creatures
/// and the invulnerable Megalomania phase do not receive a Soul War XP bonus.
#[must_use]
pub fn qualifies_for_taint_experience(creature: &str) -> bool {
    matches!(
        creature.strip_prefix("oteryn:creature."),
        Some(
            "bony_sea_devil"
                | "brachiodemon"
                | "branchy_crawler"
                | "capricious_phantom"
                | "cloak_of_terror"
                | "courage_leech"
                | "distorted_phantom"
                | "druid_s_apparition"
                | "infernal_demon"
                | "infernal_phantom"
                | "knight_s_apparition"
                | "monk_s_apparition"
                | "many_faces"
                | "mould_phantom"
                | "paladin_s_apparition"
                | "rotten_golem"
                | "sorcerer_s_apparition"
                | "turbulent_elemental"
                | "vibrant_phantom"
                | "hazardous_phantom"
                | "goshnar_s_cruelty"
                | "goshnar_s_spite"
                | "goshnar_s_malice"
                | "goshnar_s_hatred"
                | "goshnar_s_greed"
                | "goshnar_s_megalomania"
                | "goshnar_s_megalomania_blue"
                | "goshnar_s_megalomania_green"
        )
    )
}

/// Exact 1.045^n, with one integer floor at the XP boundary. W11's accepted
/// Wiki rule supersedes Crystal's rounded percentage map at soul_war.lua:25-30.
pub fn taint_experience(base: i64, taints: u8) -> Result<i64, SoulWarRefusal> {
    if base < 0 || taints > 5 {
        return Err(SoulWarRefusal::InvalidState);
    }
    let numerator = 209_u128.pow(u32::from(taints));
    let denominator = 200_u128.pow(u32::from(taints));
    let result = (base as u128) * numerator / denominator;
    i64::try_from(result).map_err(|_| SoulWarRefusal::InvalidState)
}

/// Deterministic selection among the ten distinct pairs. The shared encounter owner
/// persists this pair with its failed-attempt occurrence before replaying recipients.
#[must_use]
pub fn failure_pair(draw: u64) -> u8 {
    const PAIRS: [u8; 10] = [3, 5, 9, 17, 6, 10, 18, 12, 20, 24];
    PAIRS[(draw % 10) as usize]
}

/// Crystal `data-global/lib/quests/soul_war.lua:84-103`, one uniform once-only item.
pub const FINAL_REWARD_ITEM_IDS: [u32; 18] = [
    34082, 34083, 34084, 34085, 34086, 34087, 34088, 34089, 34090, 34091, 34092, 34093, 34094,
    34095, 34096, 34097, 34098, 34099,
];

#[cfg(test)]
#[path = "soul_war_tests.rs"]
mod tests;
