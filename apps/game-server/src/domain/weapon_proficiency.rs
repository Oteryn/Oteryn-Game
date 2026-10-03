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
//!   new credit saturates at Mastery without reducing retained older progress;
//! - a Bestiary creature gives the published points for its difficulty and
//!   influence, a Bosstiary boss the points for its category.
//!
//! Selection/clear planning is pure: it validates supplied committed snapshots
//! without saving or activating a perk. Kill-credit accrual, durable selection
//! application and effects still wait for PROF-1 and their owning runtime paths.

use super::CharacterRevision;

pub const PROFICIENCY_PERK_LEVELS_MAX: u8 = 7;
pub const PROFICIENCY_PERKS_PER_LEVEL_MAX: u8 = 3;
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
    /// A definition level has no perk or more than three.
    InvalidPerksPerLevel,
    /// Stored selections do not match the definition's levels or perk indices.
    InvalidStoredSelections,
    InvalidSelectionLevel,
    InvalidSelectionPerk,
    SelectionNotUnlocked,
    StaleTrackRevision,
    ProtectionZoneRequired,
}

impl std::fmt::Display for ProficiencyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidPerkLevels => "Weapon Proficiency perk levels are not 1..=7",
            Self::InvalidInfluenceStacks => "influence stacks are above 5",
            Self::InvalidPerksPerLevel => "Weapon Proficiency perks per level are not 1..=3",
            Self::InvalidStoredSelections => "stored Weapon Proficiency selections are malformed",
            Self::InvalidSelectionLevel => {
                "Weapon Proficiency selection level is outside the definition"
            }
            Self::InvalidSelectionPerk => "Weapon Proficiency perk index is outside its level",
            Self::SelectionNotUnlocked => "Weapon Proficiency selection level is not unlocked",
            Self::StaleTrackRevision => "Weapon Proficiency track revision is stale",
            Self::ProtectionZoneRequired => {
                "changing a selected proficiency perk requires a protection zone"
            }
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

    /// New credit saturates at Mastery. A compatible lower threshold must not
    /// reduce retained progress: an already mastered track receives no new credit.
    pub fn credited(self, progress: u32, points: u32) -> u32 {
        let cap = self.final_progress();
        if progress >= cap {
            return progress;
        }
        progress.saturating_add(points).min(cap)
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

/// Counts from one resolved definition, in its existing zero-based source order.
/// The caller binds the correct Item/definition revision and compatibility before
/// using this shape; counts alone cannot prove that an incompatible reorder is safe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProficiencySelectionShape {
    track: ProficiencyTrackShape,
    perk_counts: [u8; PROFICIENCY_PERK_LEVELS_MAX as usize],
}

impl ProficiencySelectionShape {
    pub fn new(
        class: ProficiencyThresholdClass,
        perk_counts: &[u8],
    ) -> Result<Self, ProficiencyError> {
        let levels =
            u8::try_from(perk_counts.len()).map_err(|_| ProficiencyError::InvalidPerkLevels)?;
        let track = ProficiencyTrackShape::new(class, levels)?;
        if perk_counts
            .iter()
            .any(|count| !(1..=PROFICIENCY_PERKS_PER_LEVEL_MAX).contains(count))
        {
            return Err(ProficiencyError::InvalidPerksPerLevel);
        }
        let mut counts = [0; PROFICIENCY_PERK_LEVELS_MAX as usize];
        counts[..perk_counts.len()].copy_from_slice(perk_counts);
        Ok(Self {
            track,
            perk_counts: counts,
        })
    }

    pub fn track_shape(self) -> ProficiencyTrackShape {
        self.track
    }

    /// Exactly one entry per definition level; None represents an unfilled slot.
    /// Validate inactive indices too, rather than silently accepting corrupt data.
    pub fn validate_stored_selections(
        self,
        selections: &[Option<u8>],
    ) -> Result<(), ProficiencyError> {
        if selections.len() != usize::from(self.track.perk_levels())
            || selections
                .iter()
                .zip(self.perk_counts)
                .any(|(choice, count)| choice.is_some_and(|index| index >= count))
        {
            return Err(ProficiencyError::InvalidStoredSelections);
        }
        Ok(())
    }

    /// A compatible threshold raise may make stored selections inactive. Keep
    /// those choices intact; project only currently unlocked levels as active.
    pub fn active_selections(
        self,
        progress: u32,
        selections: &[Option<u8>],
    ) -> Result<Vec<ProficiencyPerkSelection>, ProficiencyError> {
        self.validate_stored_selections(selections)?;
        Ok(selections
            .iter()
            .copied()
            .zip(0..self.track.unlocked_perk_levels(progress))
            .filter_map(|(choice, level)| {
                choice.map(|perk_index| ProficiencyPerkSelection { level, perk_index })
            })
            .collect())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProficiencyPerkSelection {
    pub level: u8,
    pub perk_index: u8,
}

/// Pure inputs from committed state/runtime validation, not authority or fences.
/// The future writer must independently resolve them under its gameplay fence and
/// Character root lock; a matching snapshot never authorizes a durable mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProficiencySelectionContext {
    pub committed_track_revision: CharacterRevision,
    pub committed_progress: u32,
    pub in_protection_zone: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProficiencySelectionCommand {
    pub expected_track_revision: CharacterRevision,
    /// Zero-based level in the resolved definition.
    pub level: u8,
    /// Zero-based perk index, or None to clear that level's choice.
    pub perk_index: Option<u8>,
}

/// NoChange has no changed entry and cannot justify a perk_selection receipt line.
/// Changed describes precisely one level; progress/definition/revisions are not
/// mutated by this planner, and the result does not activate an effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProficiencySelectionPlan {
    NoChange,
    Changed {
        level: u8,
        before: Option<u8>,
        after: Option<u8>,
    },
}

/// PROFICIENCY0 §4.2–4.3: an unfilled unlocked slot may be set anywhere;
/// changing/clearing a populated slot requires PZ. Stale snapshots fail closed.
/// A locked clear is rejected as NotUnlocked, matching the candidate wire rule.
/// Clearing inactive stored choices is not separately specified; admitting it
/// would need an owning policy instead of silently deleting those choices here.
pub fn plan_proficiency_selection(
    shape: ProficiencySelectionShape,
    selections: &[Option<u8>],
    context: ProficiencySelectionContext,
    command: ProficiencySelectionCommand,
) -> Result<ProficiencySelectionPlan, ProficiencyError> {
    if command.expected_track_revision != context.committed_track_revision {
        return Err(ProficiencyError::StaleTrackRevision);
    }
    shape.validate_stored_selections(selections)?;
    if command.level >= shape.track.perk_levels() {
        return Err(ProficiencyError::InvalidSelectionLevel);
    }
    let level = usize::from(command.level);
    if command
        .perk_index
        .is_some_and(|index| index >= shape.perk_counts[level])
    {
        return Err(ProficiencyError::InvalidSelectionPerk);
    }
    if command.level >= shape.track.unlocked_perk_levels(context.committed_progress) {
        return Err(ProficiencyError::SelectionNotUnlocked);
    }
    let before = *selections
        .get(level)
        .ok_or(ProficiencyError::InvalidStoredSelections)?;
    if before == command.perk_index {
        return Ok(ProficiencySelectionPlan::NoChange);
    }
    if before.is_some() && !context.in_protection_zone {
        return Err(ProficiencyError::ProtectionZoneRequired);
    }
    Ok(ProficiencySelectionPlan::Changed {
        level: command.level,
        before,
        after: command.perk_index,
    })
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
        assert_eq!(shape.credited(u32::MAX, u32::MAX), u32::MAX);
    }

    #[test]
    fn credit_preserves_progress_after_a_compatible_lower_mastery_cap() {
        let original =
            ProficiencyTrackShape::new(ProficiencyThresholdClass::Standard, 1).expect("valid");
        let lowered =
            ProficiencyTrackShape::new(ProficiencyThresholdClass::Knight, 1).expect("valid");
        let preserved = original.final_progress();
        assert!(preserved > lowered.final_progress());
        for points in [0, 165, u32::MAX] {
            assert_eq!(lowered.credited(preserved, points), preserved);
        }
        assert_eq!(lowered.credited(79_999, u32::MAX), 80_000);
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

    fn selection_shape() -> ProficiencySelectionShape {
        ProficiencySelectionShape::new(ProficiencyThresholdClass::Standard, &[3, 1, 2])
            .expect("valid definition counts")
    }

    fn selection_context(progress: u32, pz: bool) -> ProficiencySelectionContext {
        ProficiencySelectionContext {
            committed_track_revision: CharacterRevision::new(7).expect("revision"),
            committed_progress: progress,
            in_protection_zone: pz,
        }
    }

    fn selection_command(level: u8, perk_index: Option<u8>) -> ProficiencySelectionCommand {
        ProficiencySelectionCommand {
            expected_track_revision: CharacterRevision::new(7).expect("revision"),
            level,
            perk_index,
        }
    }

    #[test]
    fn selection_shape_bounds_actual_definition_counts() {
        for length in 1..=7 {
            let counts: Vec<u8> = (0..length).map(|index| index % 3 + 1).collect();
            let shape =
                ProficiencySelectionShape::new(ProficiencyThresholdClass::Standard, &counts)
                    .expect("one to seven levels with one to three perks");
            assert_eq!(shape.track_shape().perk_levels(), length);
            assert_eq!(
                shape.validate_stored_selections(&vec![None; usize::from(length)]),
                Ok(())
            );
        }
        for counts in [vec![], vec![1; 8], vec![1; 256]] {
            assert_eq!(
                ProficiencySelectionShape::new(ProficiencyThresholdClass::Standard, &counts),
                Err(ProficiencyError::InvalidPerkLevels)
            );
        }
        for count in [0, 4, u8::MAX] {
            assert_eq!(
                ProficiencySelectionShape::new(ProficiencyThresholdClass::Standard, &[1, count]),
                Err(ProficiencyError::InvalidPerksPerLevel)
            );
        }
    }

    #[test]
    fn malformed_stored_lengths_and_inactive_indices_fail_closed() {
        let shape = selection_shape();
        for stored in [
            vec![],
            vec![None; 2],
            vec![None; 4],
            vec![None; 8],
            vec![Some(3), None, None],
            vec![None, Some(1), None],
            vec![None, None, Some(2)],
        ] {
            assert_eq!(
                shape.validate_stored_selections(&stored),
                Err(ProficiencyError::InvalidStoredSelections)
            );
            // Progress zero keeps every selection inactive, but cannot hide corruption.
            assert_eq!(
                shape.active_selections(0, &stored),
                Err(ProficiencyError::InvalidStoredSelections)
            );
            assert_eq!(
                plan_proficiency_selection(
                    shape,
                    &stored,
                    selection_context(100_000, true),
                    selection_command(0, Some(0))
                ),
                Err(ProficiencyError::InvalidStoredSelections)
            );
        }
    }

    #[test]
    fn active_projection_preserves_choices_after_compatible_threshold_raise() {
        // The owner must separately prove compatibility; these shapes model a
        // raised threshold while level count and perk order remain unchanged.
        let old = ProficiencySelectionShape::new(ProficiencyThresholdClass::Crossbow, &[3, 1, 2])
            .expect("old");
        let raised = selection_shape();
        let stored = [Some(2), Some(0), Some(1)];
        let before = stored;
        assert_eq!(
            old.active_selections(8_000, &stored).expect("valid"),
            vec![
                ProficiencyPerkSelection {
                    level: 0,
                    perk_index: 2
                },
                ProficiencyPerkSelection {
                    level: 1,
                    perk_index: 0
                },
            ]
        );
        assert_eq!(
            raised.active_selections(8_000, &stored).expect("valid"),
            vec![ProficiencyPerkSelection {
                level: 0,
                perk_index: 2
            },]
        );
        assert_eq!(
            raised
                .active_selections(100_000, &stored)
                .expect("valid")
                .len(),
            3
        );
        assert!(
            raised
                .active_selections(0, &stored)
                .expect("valid")
                .is_empty()
        );
        assert_eq!(stored, before);
    }

    #[test]
    fn commands_reject_levels_and_indices_outside_the_resolved_definition() {
        let shape = selection_shape();
        for level in [3, 7, u8::MAX] {
            assert_eq!(
                plan_proficiency_selection(
                    shape,
                    &[None; 3],
                    selection_context(100_000, true),
                    selection_command(level, Some(0))
                ),
                Err(ProficiencyError::InvalidSelectionLevel)
            );
        }
        for (level, index) in [(0, 3), (1, 1), (2, 2), (0, u8::MAX)] {
            assert_eq!(
                plan_proficiency_selection(
                    shape,
                    &[None; 3],
                    selection_context(100_000, true),
                    selection_command(level, Some(index))
                ),
                Err(ProficiencyError::InvalidSelectionPerk)
            );
        }
    }

    #[test]
    fn unlocked_boundary_applies_to_set_and_clear_even_in_a_protection_zone() {
        let shape = selection_shape();
        for desired in [Some(0), None] {
            assert_eq!(
                plan_proficiency_selection(
                    shape,
                    &[Some(1), Some(0), None],
                    selection_context(24_999, true),
                    selection_command(1, desired)
                ),
                Err(ProficiencyError::SelectionNotUnlocked)
            );
        }
        assert_eq!(
            plan_proficiency_selection(
                shape,
                &[None; 3],
                selection_context(25_000, false),
                selection_command(1, Some(0))
            ),
            Ok(ProficiencySelectionPlan::Changed {
                level: 1,
                before: None,
                after: Some(0)
            })
        );
        assert_eq!(
            plan_proficiency_selection(
                shape,
                &[None; 3],
                selection_context(0, true),
                selection_command(0, None)
            ),
            Err(ProficiencyError::SelectionNotUnlocked)
        );
    }

    #[test]
    fn stale_revision_rejects_changed_and_no_change_commands() {
        let shape = selection_shape();
        for desired in [Some(0), Some(1), None] {
            let mut command = selection_command(0, desired);
            command.expected_track_revision = CharacterRevision::new(6).expect("stale");
            assert_eq!(
                plan_proficiency_selection(
                    shape,
                    &[Some(0), None, None],
                    selection_context(100_000, true),
                    command
                ),
                Err(ProficiencyError::StaleTrackRevision)
            );
        }
    }

    #[test]
    fn unfilled_set_anywhere_but_populated_change_or_clear_requires_pz() {
        let shape = selection_shape();
        let stored = [Some(1), None, Some(0)];
        assert_eq!(
            plan_proficiency_selection(
                shape,
                &stored,
                selection_context(100_000, false),
                selection_command(1, Some(0))
            ),
            Ok(ProficiencySelectionPlan::Changed {
                level: 1,
                before: None,
                after: Some(0)
            })
        );
        for desired in [Some(0), None] {
            assert_eq!(
                plan_proficiency_selection(
                    shape,
                    &stored,
                    selection_context(100_000, false),
                    selection_command(0, desired)
                ),
                Err(ProficiencyError::ProtectionZoneRequired)
            );
            assert_eq!(
                plan_proficiency_selection(
                    shape,
                    &stored,
                    selection_context(100_000, true),
                    selection_command(0, desired)
                ),
                Ok(ProficiencySelectionPlan::Changed {
                    level: 0,
                    before: Some(1),
                    after: desired
                })
            );
        }
        assert_eq!(stored, [Some(1), None, Some(0)]);
    }

    #[test]
    fn unchanged_choice_and_empty_clear_never_plan_a_changed_receipt_line() {
        let shape = selection_shape();
        for pz in [false, true] {
            assert_eq!(
                plan_proficiency_selection(
                    shape,
                    &[Some(2), None, None],
                    selection_context(100_000, pz),
                    selection_command(0, Some(2))
                ),
                Ok(ProficiencySelectionPlan::NoChange)
            );
            assert_eq!(
                plan_proficiency_selection(
                    shape,
                    &[Some(2), None, None],
                    selection_context(100_000, pz),
                    selection_command(1, None)
                ),
                Ok(ProficiencySelectionPlan::NoChange)
            );
        }
    }
}
