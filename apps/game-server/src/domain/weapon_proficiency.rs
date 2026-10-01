//! Pure, persistence-neutral Weapon Proficiency progression (PROF-2 rules).
//!
//! The rules live in `rulesets/progression/weapon-proficiency/progression.json`
//! (PROFICIENCY-0; TibiaWiki `Weapon_Proficiency` revid 1192598, owner
//! decisions #162 5905899852 and 2c):
//!
//! - a track is one weapon of one character; only its cumulative progress is
//!   stored, the level is derived from it and the weapon's threshold class;
//! - a definition with `n` perk levels reaches Mastery at level
//!   `n + PROFICIENCY_MASTERY_OFFSET`, which grants no perk, and progress
//!   saturates at the Mastery threshold;
//! - a Bestiary creature gives the published points for its difficulty and
//!   influence, a Bosstiary boss the points for its category.
//!
//! Accrual at kill credit, perk selection and effects wait for PROF-1
//! (Character state) and are not modelled here.

pub const PROFICIENCY_PERK_LEVELS_MAX: u8 = 7;
pub const PROFICIENCY_MASTERY_OFFSET: u8 = 2;
/// The published influence stack columns are 0..=5.
pub const PROFICIENCY_INFLUENCE_STACKS_MAX: u8 = 5;

const STANDARD_THRESHOLDS: [u32; 9] = [
    1_750, 25_000, 100_000, 400_000, 2_000_000, 8_000_000, 30_000_000, 60_000_000, 90_000_000,
];
const KNIGHT_THRESHOLDS: [u32; 9] = [
    1_250, 20_000, 80_000, 300_000, 1_500_000, 6_000_000, 20_000_000, 40_000_000, 60_000_000,
];
const CROSSBOW_THRESHOLDS: [u32; 9] = [
    600, 8_000, 30_000, 150_000, 650_000, 2_500_000, 10_000_000, 20_000_000, 30_000_000,
];

/// Columns: influence stacks 0..=5, then fiendish. Copied as published; the
/// published values are not all `floor(base * (1 + stacks / 10))`.
const CREATURE_POINTS: [[u32; 7]; 6] = [
    [1, 1, 1, 1, 1, 1, 2],
    [30, 33, 36, 39, 42, 45, 75],
    [70, 77, 84, 91, 98, 105, 175],
    [100, 110, 120, 130, 140, 150, 250],
    [165, 181, 198, 214, 230, 247, 412],
    [240, 264, 288, 312, 336, 360, 600],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProficiencyError {
    /// The definition has no perk level or more than seven.
    InvalidPerkLevels,
    /// The influence stack count is above the published columns.
    InvalidInfluenceStacks,
}

impl std::fmt::Display for ProficiencyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidPerkLevels => "Weapon Proficiency perk levels are not 1..=7",
            Self::InvalidInfluenceStacks => "influence stacks are above 5",
        })
    }
}

impl std::error::Error for ProficiencyError {}

/// The progress threshold table of a weapon (D197, D198, D200).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProficiencyThresholdClass {
    Standard,
    Knight,
    Crossbow,
}

impl ProficiencyThresholdClass {
    pub const ALL: [Self; 3] = [Self::Standard, Self::Knight, Self::Crossbow];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Knight => "knight",
            Self::Crossbow => "crossbow",
        }
    }

    /// Cumulative progress for levels 1..=9.
    pub fn thresholds(self) -> &'static [u32; 9] {
        match self {
            Self::Standard => &STANDARD_THRESHOLDS,
            Self::Knight => &KNIGHT_THRESHOLDS,
            Self::Crossbow => &CROSSBOW_THRESHOLDS,
        }
    }
}

/// The progression shape of one weapon: its threshold class and the number
/// of perk levels of its Proficiency definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProficiencyTrackShape {
    class: ProficiencyThresholdClass,
    perk_levels: u8,
}

impl ProficiencyTrackShape {
    pub fn new(
        class: ProficiencyThresholdClass,
        perk_levels: u8,
    ) -> Result<Self, ProficiencyError> {
        if perk_levels == 0 || perk_levels > PROFICIENCY_PERK_LEVELS_MAX {
            return Err(ProficiencyError::InvalidPerkLevels);
        }
        Ok(Self { class, perk_levels })
    }

    pub fn class(self) -> ProficiencyThresholdClass {
        self.class
    }

    pub fn perk_levels(self) -> u8 {
        self.perk_levels
    }

    pub fn mastery_level(self) -> u8 {
        self.perk_levels + PROFICIENCY_MASTERY_OFFSET
    }

    /// The progress at which the track is mastered and stops growing.
    pub fn final_progress(self) -> u32 {
        self.class.thresholds()[usize::from(self.mastery_level()) - 1]
    }

    /// Progress after `points` are credited, saturated at Mastery.
    pub fn credited(self, progress: u32, points: u32) -> u32 {
        progress.saturating_add(points).min(self.final_progress())
    }

    /// The level reached, 0..=mastery_level.
    pub fn level(self, progress: u32) -> u8 {
        let reached = self
            .class
            .thresholds()
            .iter()
            .take(usize::from(self.mastery_level()))
            .take_while(|threshold| **threshold <= progress)
            .count();
        // At most mastery_level <= 9.
        reached as u8
    }

    /// Perk levels whose perk may be selected, 0..=perk_levels.
    pub fn unlocked_perk_levels(self, progress: u32) -> u8 {
        self.level(progress).min(self.perk_levels)
    }

    pub fn is_mastered(self, progress: u32) -> bool {
        progress >= self.final_progress()
    }
}

/// Bestiary difficulty of a creature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureDifficulty {
    Harmless,
    Trivial,
    Easy,
    Medium,
    Hard,
    Challenging,
}

impl CreatureDifficulty {
    pub const ALL: [Self; 6] = [
        Self::Harmless,
        Self::Trivial,
        Self::Easy,
        Self::Medium,
        Self::Hard,
        Self::Challenging,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Harmless => "harmless",
            Self::Trivial => "trivial",
            Self::Easy => "easy",
            Self::Medium => "medium",
            Self::Hard => "hard",
            Self::Challenging => "challenging",
        }
    }
}

/// Influence of the killed creature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureInfluence {
    /// Influence stacks, 0 for an ordinary creature.
    Stacks(u8),
    Fiendish,
}

/// Bosstiary category of a boss.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BossCategory {
    Bane,
    Archfoe,
    Nemesis,
}

impl BossCategory {
    pub const ALL: [Self; 3] = [Self::Bane, Self::Archfoe, Self::Nemesis];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bane => "bane",
            Self::Archfoe => "archfoe",
            Self::Nemesis => "nemesis",
        }
    }
}

/// Progress points for killing a Bestiary creature.
pub fn creature_points(
    difficulty: CreatureDifficulty,
    influence: CreatureInfluence,
) -> Result<u32, ProficiencyError> {
    let column = match influence {
        CreatureInfluence::Stacks(stacks) if stacks <= PROFICIENCY_INFLUENCE_STACKS_MAX => {
            usize::from(stacks)
        }
        CreatureInfluence::Stacks(_) => return Err(ProficiencyError::InvalidInfluenceStacks),
        CreatureInfluence::Fiendish => 6,
    };
    Ok(CREATURE_POINTS[difficulty as usize][column])
}

/// Progress points for killing a Bosstiary boss; influence gives no bonus.
pub fn boss_points(category: BossCategory) -> u32 {
    match category {
        BossCategory::Bane => 500,
        BossCategory::Archfoe => 5_000,
        BossCategory::Nemesis => 15_000,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn ruleset() -> serde_json::Value {
        serde_json::from_str(include_str!(
            "../../../../rulesets/progression/weapon-proficiency/progression.json"
        ))
        .expect("ruleset json")
    }

    fn numbers(value: &serde_json::Value) -> Vec<u64> {
        value
            .as_array()
            .expect("array")
            .iter()
            .map(|n| n.as_u64().expect("number"))
            .collect()
    }

    #[test]
    fn ruleset_file_matches_the_implemented_rules() {
        let ruleset = ruleset();
        assert_eq!(
            ruleset["schema"].as_str(),
            Some("OTERYN_GAME_WEAPON_PROFICIENCY_PROGRESSION/v1")
        );
        let levels = &ruleset["levels"];
        assert_eq!(
            levels["perk_levels_max"].as_u64(),
            Some(u64::from(PROFICIENCY_PERK_LEVELS_MAX))
        );
        assert_eq!(
            levels["mastery_offset"].as_u64(),
            Some(u64::from(PROFICIENCY_MASTERY_OFFSET))
        );
        let tables = levels["progress_thresholds"].as_object().expect("tables");
        assert_eq!(tables.len(), ProficiencyThresholdClass::ALL.len());
        for class in ProficiencyThresholdClass::ALL {
            let expected: Vec<u64> = class.thresholds().iter().map(|t| u64::from(*t)).collect();
            assert_eq!(numbers(&tables[class.as_str()]), expected, "{class:?}");
        }
        let points = &ruleset["points"];
        assert_eq!(points["creature_columns"].as_array().map(Vec::len), Some(7));
        let creatures = points["creature_points"].as_object().expect("creatures");
        assert_eq!(creatures.len(), CreatureDifficulty::ALL.len());
        for difficulty in CreatureDifficulty::ALL {
            let mut expected: Vec<u64> = (0..=PROFICIENCY_INFLUENCE_STACKS_MAX)
                .map(|stacks| {
                    creature_points(difficulty, CreatureInfluence::Stacks(stacks)).expect("valid")
                })
                .map(u64::from)
                .collect();
            expected.push(u64::from(
                creature_points(difficulty, CreatureInfluence::Fiendish).expect("valid"),
            ));
            assert_eq!(
                numbers(&creatures[difficulty.as_str()]),
                expected,
                "{difficulty:?}"
            );
        }
        let bosses = points["boss_points"].as_object().expect("bosses");
        assert_eq!(bosses.len(), BossCategory::ALL.len());
        for category in BossCategory::ALL {
            assert_eq!(
                bosses[category.as_str()].as_u64(),
                Some(u64::from(boss_points(category)))
            );
        }
    }

    #[test]
    fn thresholds_strictly_increase() {
        for class in ProficiencyThresholdClass::ALL {
            assert!(
                class.thresholds().windows(2).all(|w| w[0] < w[1]),
                "{class:?}"
            );
        }
    }

    #[test]
    fn a_track_needs_one_to_seven_perk_levels() {
        for perk_levels in [0, 8, u8::MAX] {
            assert_eq!(
                ProficiencyTrackShape::new(ProficiencyThresholdClass::Standard, perk_levels),
                Err(ProficiencyError::InvalidPerkLevels)
            );
        }
        let shape =
            ProficiencyTrackShape::new(ProficiencyThresholdClass::Knight, 7).expect("valid");
        assert_eq!(shape.mastery_level(), 9);
        assert_eq!(shape.final_progress(), 60_000_000);
    }

    #[test]
    fn levels_are_derived_from_progress_and_the_class() {
        let shape =
            ProficiencyTrackShape::new(ProficiencyThresholdClass::Standard, 3).expect("valid");
        assert_eq!(shape.level(0), 0);
        assert_eq!(shape.level(1_749), 0);
        assert_eq!(shape.level(1_750), 1);
        assert_eq!(shape.level(99_999), 2);
        assert_eq!(shape.unlocked_perk_levels(100_000), 3);
        // Levels 4 and 5 grant no perk; 5 is Mastery for a 3-level tree.
        assert_eq!(shape.level(400_000), 4);
        assert_eq!(shape.unlocked_perk_levels(400_000), 3);
        assert!(!shape.is_mastered(1_999_999));
        assert!(shape.is_mastered(2_000_000));
        assert_eq!(shape.level(u32::MAX), 5);

        let crossbow =
            ProficiencyTrackShape::new(ProficiencyThresholdClass::Crossbow, 3).expect("valid");
        assert_eq!(crossbow.level(600), 1);
        assert_eq!(crossbow.final_progress(), 650_000);
    }

    #[test]
    fn progress_saturates_at_mastery() {
        let shape =
            ProficiencyTrackShape::new(ProficiencyThresholdClass::Standard, 1).expect("valid");
        assert_eq!(shape.final_progress(), 100_000);
        assert_eq!(shape.credited(0, 165), 165);
        assert_eq!(shape.credited(99_990, 165), 100_000);
        assert_eq!(shape.credited(100_000, 15_000), 100_000);
        assert_eq!(shape.credited(u32::MAX, u32::MAX), 100_000);
    }

    #[test]
    fn points_follow_the_published_table() {
        use CreatureDifficulty as D;
        use CreatureInfluence as I;
        assert_eq!(creature_points(D::Harmless, I::Stacks(5)), Ok(1));
        assert_eq!(creature_points(D::Harmless, I::Fiendish), Ok(2));
        assert_eq!(creature_points(D::Medium, I::Stacks(0)), Ok(100));
        // Published as 230, not 165 * 1.4 = 231.
        assert_eq!(creature_points(D::Hard, I::Stacks(4)), Ok(230));
        assert_eq!(creature_points(D::Challenging, I::Fiendish), Ok(600));
        assert_eq!(
            creature_points(D::Easy, I::Stacks(6)),
            Err(ProficiencyError::InvalidInfluenceStacks)
        );
        assert_eq!(boss_points(BossCategory::Bane), 500);
        assert_eq!(boss_points(BossCategory::Nemesis), 15_000);
    }
}
