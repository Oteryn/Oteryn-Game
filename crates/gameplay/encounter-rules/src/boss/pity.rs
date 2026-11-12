//! Moonsilver first-drop pity (WU26 T188/T197, capability 492).
//!
//! TibiaWiki Updates/15.30.a30dad: "To reward persistence, your chance of obtaining new
//! Moonsilver equipment increases until you receive one." Winter Update 2026: the tracker
//! "applies only your first drop, after that only the base drop chance applies."
//! No official figures are published; the step and cap below are the owner decision of
//! 2026-11-11 (`docs/contracts/protocol-oteryn/v1/boss-difficulty-v1.md`, *Pity tracker*).

use oteryn_protocol_oteryn::boss_difficulty::{MAX_PITY_BONUS_PERMILLE, boss_maximum};

/// Each boss victory without a pity drop adds 10% of the base chance (owner decision 2026-11-11).
pub const PITY_STEP_PERMILLE: u16 = 100;
/// The bonus stops at +200% of base, a 3x chance, after 20 victories (owner decision 2026-11-11).
pub const PITY_CAP_PERMILLE: u16 = MAX_PITY_BONUS_PERMILLE;
/// The durable counter saturates at the PostgreSQL SMALLINT maximum.
pub const PITY_KILLS_MAX: u16 = 32_767;
/// The probability scale of an `IndependentBernoulliPpm` loot entry.
const PPM_SCALE: u32 = 1_000_000;
/// TibiaWiki item ids of the Moonsilver equipment family (Moonsilver Epee 53207 through
/// Moonsilver Strike Helm 53233). Moonsilver Crystals (54267) are a material, not equipment.
const MOONSILVER_EQUIPMENT_IDS: std::ops::RangeInclusive<u32> = 53_207..=53_233;

/// One character's tracker for one boss. The default is a character with no victory yet.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PityState {
    pub kills_without_drop: u16,
    pub first_drop_received: bool,
}

impl PityState {
    pub const fn tracker_active(self) -> bool {
        !self.first_drop_received
    }

    /// The bonus over the base chance, in permille of base: 0 at no miss, 1000 (2x) after
    /// 10 misses, capped at 2000 (3x) from 20 misses, and 0 once the first drop is received.
    pub fn bonus_permille(self) -> u16 {
        if self.first_drop_received {
            return 0;
        }
        self.kills_without_drop
            .saturating_mul(PITY_STEP_PERMILLE)
            .min(PITY_CAP_PERMILLE)
    }

    /// The state after one boss victory. A drop switches the tracker off for good and keeps
    /// the count it reached; a victory after that changes nothing.
    pub fn after_victory(self, dropped: bool) -> Self {
        if self.first_drop_received {
            return self;
        }
        if dropped {
            return Self {
                first_drop_received: true,
                ..self
            };
        }
        Self {
            kills_without_drop: self
                .kills_without_drop
                .saturating_add(1)
                .min(PITY_KILLS_MAX),
            first_drop_received: false,
        }
    }
}

/// The boosted chance of one pity entry: `base * (1000 + bonus) / 1000`, never above certainty.
pub fn boosted_probability_ppm(base_ppm: u32, bonus_permille: u16) -> u32 {
    let boosted = u64::from(base_ppm) * (1_000 + u64::from(bonus_permille)) / 1_000;
    u32::try_from(boosted.min(u64::from(PPM_SCALE))).unwrap_or(PPM_SCALE)
}

/// The tracker key of a boss under the Boss Difficulty System, or `None` for any other boss.
/// The boss and creature identities of one boss share one tracker (`oteryn:boss.<name>`).
pub fn pity_boss_key(key: &str) -> Option<String> {
    boss_maximum(key)?;
    let local = key
        .strip_prefix("oteryn:boss.")
        .or_else(|| key.strip_prefix("oteryn:creature."))?;
    Some(format!("oteryn:boss.{local}"))
}

/// Whether an Item production key names Moonsilver equipment.
pub fn is_pity_item(production_key: &str) -> bool {
    production_key
        .strip_prefix("oteryn:item.tibia.i")
        .and_then(|id| id.parse::<u32>().ok())
        .is_some_and(|id| MOONSILVER_EQUIPMENT_IDS.contains(&id))
}

#[cfg(test)]
#[path = "pity_tests.rs"]
mod tests;
