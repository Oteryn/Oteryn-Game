//! Pure Harmony multiplier arithmetic for monk spell plans.

use oteryn_gameplay_combat_rules::chain::ChainCreature;

/// Most Harmony a monk holds (§8.2: 0..5).
pub const MAX_HARMONY: u8 = 5;

/// Denominator of a [`HarmonyMultiplier`]: percent (100) x the 0.005-per-level step (1000) x the
/// half steps of the virtue factor (2).
pub const MULTIPLIER_DENOMINATOR: u64 = 200_000;

/// The Harmony multiplier `m` of §A.2 step 2 as the exact fraction
/// `numerator / MULTIPLIER_DENOMINATOR`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HarmonyMultiplier {
    numerator: u64,
}

impl HarmonyMultiplier {
    pub const ONE: Self = Self {
        numerator: MULTIPLIER_DENOMINATOR,
    };

    /// `m = 1 + B * 2^(c - 1) / 100` for c = `charges` > 0, else 1, with
    /// `B = (7 + 0.005 * level) * V` percent and `V` = 1 without Virtue of Harmony, 1.5 with it,
    /// 2 with it while Serene (news 8944). `None` above [`MAX_HARMONY`] charges.
    pub fn new(level: u32, charges: u8, virtue_of_harmony: bool, serene: bool) -> Option<Self> {
        if charges > MAX_HARMONY {
            return None;
        }
        if charges == 0 {
            return Some(Self::ONE);
        }
        let half_steps = match (virtue_of_harmony, serene) {
            (false, _) => 2,
            (true, false) => 3,
            (true, true) => 4,
        };
        // B in thousandths of a percent, doubled: (7000 + 5 * level) * 2V.
        let bonus = (7_000 + 5 * u64::from(level)) * half_steps;
        Some(Self {
            numerator: MULTIPLIER_DENOMINATOR + bonus * (1 << (charges - 1)),
        })
    }

    pub fn numerator(self) -> u64 {
        self.numerator
    }

    /// `bound * m`, truncated toward zero (§A.2 step 3b), in exact integer arithmetic.
    pub fn apply(self, bound: i64) -> i64 {
        let scaled =
            i128::from(bound) * i128::from(self.numerator) / i128::from(MULTIPLIER_DENOMINATOR);
        i64::try_from(scaled).unwrap_or(if scaled < 0 { i64::MIN } else { i64::MAX })
    }
}

/// Time between two Serene evaluations (Canary player think, §8.2).
pub const SERENE_EVALUATION_MICROS: u64 = 1_000_000;

/// How long Focus Serenity forces Serene (Canary `focus_serenity.lua`, §8.2).
pub const FOCUS_SERENITY_MICROS: u64 = 7_000_000;

/// Adjacent creatures that end Serene when a party member is visible (§A.2 step 6, news 8944).
pub const SERENE_CROWD: u32 = 8;

/// World facts the Serene evaluation reads.
pub trait SereneWorld {
    fn monk(&self) -> &ChainCreature;
    /// The monk's party (the monk included); `None` without a party.
    fn party(&self) -> Option<&[ChainCreature]>;
    /// Creatures adjacent to the monk that count against Serene (which ones is §A.2 Q6).
    fn adjacent_creatures(&self) -> u32;
}

/// The §A.2 step 6 rule without a forced state: Serene unless another party member is visible
/// (same floor, dx in [-8, 9], dy in [-6, 7]) and [`SERENE_CROWD`] or more creatures are adjacent.
pub fn serene_by_rule(world: &dyn SereneWorld) -> bool {
    let Some(party) = world.party() else {
        return true;
    };
    let monk = world.monk();
    let visible = party.iter().any(|member| {
        let dx = i64::from(member.position.x) - i64::from(monk.position.x);
        let dy = i64::from(member.position.y) - i64::from(monk.position.y);
        member.id != monk.id
            && member.position.floor == monk.position.floor
            && (-8..=9).contains(&dx)
            && (-6..=7).contains(&dy)
    });
    !(visible && world.adjacent_creatures() >= SERENE_CROWD)
}
