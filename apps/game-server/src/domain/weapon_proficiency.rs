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

// PROFICIENCY-1B perk modification (PROF-SHAPE-1a): pure admission, planning, draws and the
// active-perk rule. Persistence, fences and the value ledger live in durability.

/// `PROF1-RL-01`: modification slots per track.
pub const PROFICIENCY_MODIFICATION_SLOTS: u8 = 2;
/// `PROF1-RL-02`: the highest rank of a modified perk.
pub const PROFICIENCY_MODIFICATION_RANK_MAX: u8 = 10;
/// `PROF1B-RL-02`: entries of a pending reshape offer.
pub const PROFICIENCY_RESHAPE_OFFER_SIZE: usize = 3;
/// `PROF1B-RL-03`: modification commands per character per second, charged at reservation.
pub const PROFICIENCY_MODIFICATIONS_PER_SECOND: usize = 2;
/// `PROF1B-RL-04`: pool entries per shaping definition.
pub const PROFICIENCY_SHAPING_POOL_MAX: usize = 64;
/// `PROF1B-RL-05`: ambiguous-commit bound before a reconciliation hold.
pub const PROFICIENCY_AMBIGUOUS_COMMIT_HOLD_MS: u64 = 2_000;
/// Slot 1 unlocks at this derived proficiency level; slot 2 at Mastery.
pub const PROFICIENCY_FIRST_MODIFICATION_LEVEL: u8 = 3;

/// The six commands, plus the decline arm of `RESHAPE_CHOOSE` and the migration clear, as
/// stored in a modification line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProficiencyModificationOperation {
    Modify,
    RankUp,
    OrbRank,
    ReshapeOffer,
    ReshapeChoose,
    ReshapeDecline,
    Clear,
    MigrationClear,
}

impl ProficiencyModificationOperation {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Modify => "MODIFY",
            Self::RankUp => "RANK_UP",
            Self::OrbRank => "ORB_RANK",
            Self::ReshapeOffer => "RESHAPE_OFFER",
            Self::ReshapeChoose => "RESHAPE_CHOOSE",
            Self::ReshapeDecline => "RESHAPE_DECLINE",
            Self::Clear => "CLEAR",
            Self::MigrationClear => "MIGRATION_CLEAR",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Some(match key {
            "MODIFY" => Self::Modify,
            "RANK_UP" => Self::RankUp,
            "ORB_RANK" => Self::OrbRank,
            "RESHAPE_OFFER" => Self::ReshapeOffer,
            "RESHAPE_CHOOSE" => Self::ReshapeChoose,
            "RESHAPE_DECLINE" => Self::ReshapeDecline,
            "CLEAR" => Self::Clear,
            "MIGRATION_CLEAR" => Self::MigrationClear,
            _ => return None,
        })
    }
}

/// One command's operation with its own payload fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProficiencyModificationCommandKind {
    /// Zero-based perk level whose selected perk is modified.
    Modify {
        level: u8,
    },
    RankUp,
    OrbRank,
    ReshapeOffer,
    /// `Some(0..=2)` chooses an offered entry; `None` declines the offer.
    ReshapeChoose {
        choice: Option<u8>,
    },
    Clear,
}

impl ProficiencyModificationCommandKind {
    /// The command's operation; `RESHAPE_CHOOSE` covers both of its arms.
    pub const fn operation(self) -> ProficiencyModificationOperation {
        match self {
            Self::Modify { .. } => ProficiencyModificationOperation::Modify,
            Self::RankUp => ProficiencyModificationOperation::RankUp,
            Self::OrbRank => ProficiencyModificationOperation::OrbRank,
            Self::ReshapeOffer => ProficiencyModificationOperation::ReshapeOffer,
            Self::ReshapeChoose { .. } => ProficiencyModificationOperation::ReshapeChoose,
            Self::Clear => ProficiencyModificationOperation::Clear,
        }
    }
}

/// The closed result enum shared by the six commands (PROFICIENCY-1B §11.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProficiencyModificationResult {
    Accepted,
    NotAdmitted,
    NotUnlocked,
    NotInProtectionZone,
    UnknownTrack,
    StaleRevision,
    RevisionChanged,
    RateLimited,
    Conflict,
    NoSelection,
    SlotOccupied,
    LevelTaken,
    NotModified,
    RankMax,
    OfferPending,
    NoOffer,
    PoolTooSmall,
    ShapingOutdated,
    InsufficientDust,
    NoOrb,
}

/// A non-cleared modification row: its perk is `(shaping revision, entry index)` at `rank`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProficiencyModifiedPerk {
    pub level: u8,
    pub shaping_key: String,
    pub shaping_revision: String,
    pub entry_index: u8,
    pub rank: u8,
    pub pending_offer: Option<[u8; PROFICIENCY_RESHAPE_OFFER_SIZE]>,
}

impl ProficiencyModifiedPerk {
    /// The row CHECK of migration 0055, so a malformed value never reaches storage.
    pub fn is_well_formed(&self) -> bool {
        let index = |value: u8| usize::from(value) < PROFICIENCY_SHAPING_POOL_MAX;
        self.level < PROFICIENCY_PERK_LEVELS_MAX
            && index(self.entry_index)
            && (1..=PROFICIENCY_MODIFICATION_RANK_MAX).contains(&self.rank)
            && self.pending_offer.is_none_or(|offer| {
                offer
                    .iter()
                    .all(|entry| index(*entry) && *entry != self.entry_index)
                    && offer[0] != offer[1]
                    && offer[0] != offer[2]
                    && offer[1] != offer[2]
            })
    }
}

/// One pool entry of a shaping revision. `None` and `false` are unevidenced cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProficiencyShapingEntry {
    /// Canonical non-value perk identity, comparable with a selected perk's identity.
    pub perk_identity: String,
    pub weight: Option<u32>,
    /// Whether the entry's values at ranks 1..=10 are evidenced.
    pub rank_values: [bool; PROFICIENCY_MODIFICATION_RANK_MAX as usize],
}

/// One `ProficiencyShaping` revision as resolved from content (PROFICIENCY-1B §3.1). Every
/// `None` is an unevidenced cell; OTS-classed cells never reach here (content validation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProficiencyShapingRevision {
    pub shaping_key: String,
    pub revision: String,
    pub pool: Option<Vec<ProficiencyShapingEntry>>,
    pub modify_cost: [Option<u64>; PROFICIENCY_MODIFICATION_SLOTS as usize],
    /// Dust for the steps 1→2 … 9→10.
    pub rank_step_cost: [Option<u64>; PROFICIENCY_MODIFICATION_RANK_MAX as usize - 1],
    pub reshape_offer_cost: Option<u64>,
    pub clear_cost: Option<u64>,
    /// Orbs ORB_RANK burns: 1 if consumed, 0 if not.
    pub orb_count: Option<u8>,
}

/// The cost an admitted operation binds (PROFICIENCY-1B §7.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProficiencyModificationCost {
    pub dust: u64,
    pub orbs: u8,
}

impl ProficiencyShapingRevision {
    /// Shape rules the runtime relies on: a bounded pool of distinct identities with positive
    /// weights, and an orb count of 0 or 1.
    pub fn is_well_formed(&self) -> bool {
        self.orb_count.is_none_or(|count| count <= 1)
            && self.pool.as_ref().is_none_or(|pool| {
                let mut identities = pool.iter().map(|e| &e.perk_identity).collect::<Vec<_>>();
                identities.sort_unstable();
                identities.dedup();
                pool.len() <= PROFICIENCY_SHAPING_POOL_MAX
                    && identities.len() == pool.len()
                    && pool.iter().all(|entry| entry.weight != Some(0))
            })
    }

    fn drawable(&self) -> Option<&[ProficiencyShapingEntry]> {
        self.pool
            .as_deref()
            .filter(|pool| pool.iter().all(|entry| entry.weight.is_some()))
    }

    fn values_known(&self, rank: u8) -> bool {
        self.pool.as_ref().is_some_and(|pool| {
            pool.iter()
                .all(|entry| entry.rank_values[usize::from(rank) - 1])
        })
    }

    fn entry_known(&self, entry: u8, rank: u8) -> bool {
        self.pool
            .as_ref()
            .and_then(|pool| pool.get(usize::from(entry)))
            .is_some_and(|entry| entry.rank_values[usize::from(rank) - 1])
    }

    /// PROFICIENCY-1B §3.3: `Some(cost)` when every cell the operation reads is evidenced.
    /// `row` is the slot's current perk when it is on this revision; cells that depend on a
    /// row are only read when there is one (a missing row is refused later, as §5's checks).
    pub fn admitted(
        &self,
        command: ProficiencyModificationCommandKind,
        slot: u8,
        row: Option<&ProficiencyModifiedPerk>,
    ) -> Option<ProficiencyModificationCost> {
        let dust = |dust: u64| ProficiencyModificationCost { dust, orbs: 0 };
        match command {
            ProficiencyModificationCommandKind::Modify { .. } => {
                let cost = (*self.modify_cost.get(usize::from(slot).checked_sub(1)?)?)?;
                (self.drawable().is_some() && self.values_known(1)).then_some(dust(cost))
            }
            ProficiencyModificationCommandKind::RankUp => match row {
                None => Some(dust(0)),
                Some(row) if row.rank >= PROFICIENCY_MODIFICATION_RANK_MAX => Some(dust(0)),
                Some(row) => {
                    let cost = self.rank_step_cost[usize::from(row.rank) - 1]?;
                    self.entry_known(row.entry_index, row.rank + 1)
                        .then_some(dust(cost))
                }
            },
            ProficiencyModificationCommandKind::OrbRank => {
                let orbs = self.orb_count?;
                row.is_none_or(|row| {
                    self.entry_known(row.entry_index, PROFICIENCY_MODIFICATION_RANK_MAX)
                })
                .then_some(ProficiencyModificationCost { dust: 0, orbs })
            }
            ProficiencyModificationCommandKind::ReshapeOffer => {
                let cost = self.reshape_offer_cost?;
                self.drawable()?;
                row.is_none_or(|row| self.values_known(row.rank))
                    .then_some(dust(cost))
            }
            ProficiencyModificationCommandKind::ReshapeChoose { .. } => Some(dust(0)),
            ProficiencyModificationCommandKind::Clear => self.clear_cost.map(dust),
        }
    }
}

/// Slot 1 is unlocked while the derived level is at least 3, slot 2 while the track has
/// Mastery (PROFICIENCY-1 §3).
pub fn proficiency_modification_slot_unlocked(
    track: ProficiencyTrackShape,
    progress: u32,
    slot: u8,
) -> bool {
    match slot {
        1 => track.level(progress) >= PROFICIENCY_FIRST_MODIFICATION_LEVEL,
        2 => track.is_mastered(progress),
        _ => false,
    }
}

/// An unlocked, non-cleared row whose level still has a selection is active: its perk
/// replaces that level's selected perk for PROF-EFFECT-0. Any other row is stored and inactive.
pub fn proficiency_modification_active(
    shape: ProficiencySelectionShape,
    progress: u32,
    selections: &[Option<u8>],
    slot: u8,
    row: &ProficiencyModifiedPerk,
) -> bool {
    proficiency_modification_slot_unlocked(shape.track_shape(), progress, slot)
        && row.level < shape.track_shape().perk_levels()
        && selections
            .get(usize::from(row.level))
            .is_some_and(Option::is_some)
}

/// SIM-DETERMINISM-01 purpose `proficiency_shaping`: seed material for one occurrence, from a
/// server seed the client cannot derive and the bound revisions. Same inputs, same draws.
pub fn proficiency_shaping_seed(
    server_seed: &[u8; 32],
    simulation_revision: &str,
    shaping_revision: &str,
    occurrence: &[u8; 16],
) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"oteryn:rng:proficiency_shaping\0");
    for text in [simulation_revision, shaping_revision] {
        hasher.update(text.as_bytes());
        hasher.update([0]);
    }
    hasher.update(server_seed);
    hasher.update(occurrence);
    hasher.finalize().into()
}

/// Draw `count` distinct entries by weight, without replacement, skipping `excluded`, in draw
/// order. `None` when fewer than `count` entries remain.
fn draw_by_weight(
    pool: &[ProficiencyShapingEntry],
    excluded: impl Fn(usize, &ProficiencyShapingEntry) -> bool,
    count: usize,
    seed: &[u8; 32],
) -> Option<Vec<u8>> {
    use sha2::{Digest, Sha256};
    let mut open = pool
        .iter()
        .enumerate()
        .filter(|(index, entry)| !excluded(*index, entry))
        .map(|(index, entry)| Some((u8::try_from(index).ok()?, u64::from(entry.weight?))))
        .collect::<Option<Vec<_>>>()?;
    let mut drawn = Vec::with_capacity(count);
    for round in 0..count {
        let total = open.iter().map(|(_, weight)| *weight).sum::<u64>();
        if total == 0 {
            return None;
        }
        let digest = Sha256::new()
            .chain_update(seed)
            .chain_update(u32::try_from(round).ok()?.to_be_bytes())
            .finalize();
        let word = u64::from_be_bytes(digest[..8].try_into().ok()?);
        // Unbiased enough for weights below 2^32 per entry: the high word of word × total.
        let mut target = ((u128::from(word) * u128::from(total)) >> 64) as u64;
        let position = open.iter().position(|(_, weight)| {
            if target < *weight {
                true
            } else {
                target -= *weight;
                false
            }
        })?;
        drawn.push(open.remove(position).0);
    }
    Some(drawn)
}

/// Committed facts of the command's track, read under the writer's locks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProficiencyModificationTrack<'a> {
    pub shape: ProficiencySelectionShape,
    pub progress: u32,
    pub selections: &'a [Option<u8>],
    pub committed_track_revision: CharacterRevision,
    /// The track's two slots; `None` is an absent or cleared row.
    pub slots: [Option<&'a ProficiencyModifiedPerk>; PROFICIENCY_MODIFICATION_SLOTS as usize],
}

/// Everything §5 checks 5-8 read. `selected_identity` is the perk identity selected at the
/// MODIFY level, when it has one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProficiencyModificationInput<'a> {
    pub slot: u8,
    pub command: ProficiencyModificationCommandKind,
    pub expected_track_revision: CharacterRevision,
    pub in_protection_zone: bool,
    pub active: &'a ProficiencyShapingRevision,
    pub track: Option<ProficiencyModificationTrack<'a>>,
    pub selected_identity: Option<&'a str>,
    pub seed: [u8; 32],
}

/// The one-slot change an accepted command makes, with its bound cost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProficiencyModificationPlan {
    pub operation: ProficiencyModificationOperation,
    pub slot: u8,
    pub before: Option<ProficiencyModifiedPerk>,
    pub after: Option<ProficiencyModifiedPerk>,
    pub cost: ProficiencyModificationCost,
}

/// PROFICIENCY-1B §5 checks 5-8, in order: admission, the track and its revision, the
/// protection zone, then the operation's own checks. Check 9 (dust balance and orb stack)
/// belongs to the value writer. The draw is a pure function of the seed, so a retry of the
/// same occurrence draws the same entries.
pub fn plan_proficiency_modification(
    input: &ProficiencyModificationInput<'_>,
) -> Result<ProficiencyModificationPlan, ProficiencyModificationResult> {
    use ProficiencyModificationCommandKind as Command;
    use ProficiencyModificationResult as R;
    if !(1..=PROFICIENCY_MODIFICATION_SLOTS).contains(&input.slot) {
        return Err(R::NotUnlocked);
    }
    let row = input
        .track
        .as_ref()
        .and_then(|track| track.slots[usize::from(input.slot) - 1]);
    let current = row.filter(|row| {
        row.shaping_key == input.active.shaping_key && row.shaping_revision == input.active.revision
    });
    let cost = input
        .active
        .admitted(input.command, input.slot, current)
        .ok_or(R::NotAdmitted)?;
    let track = input.track.as_ref().ok_or(R::UnknownTrack)?;
    if track.committed_track_revision != input.expected_track_revision {
        return Err(R::StaleRevision);
    }
    if !input.in_protection_zone {
        return Err(R::NotInProtectionZone);
    }
    let outdated = |row: &ProficiencyModifiedPerk| {
        row.shaping_key != input.active.shaping_key || row.shaping_revision != input.active.revision
    };
    let plan = |operation, after: Option<ProficiencyModifiedPerk>| {
        Ok(ProficiencyModificationPlan {
            operation,
            slot: input.slot,
            before: row.cloned(),
            after,
            cost,
        })
    };
    let ranked = |row: Option<&ProficiencyModifiedPerk>| {
        let row = row.ok_or(R::NotModified)?;
        if row.rank >= PROFICIENCY_MODIFICATION_RANK_MAX {
            return Err(R::RankMax);
        }
        if row.pending_offer.is_some() {
            return Err(R::OfferPending);
        }
        if outdated(row) {
            return Err(R::ShapingOutdated);
        }
        Ok(row.clone())
    };
    match input.command {
        Command::Modify { level } => {
            if !proficiency_modification_slot_unlocked(
                track.shape.track_shape(),
                track.progress,
                input.slot,
            ) {
                return Err(R::NotUnlocked);
            }
            if row.is_some() {
                return Err(R::SlotOccupied);
            }
            let selected = input.selected_identity.filter(|_| {
                level < track.shape.track_shape().perk_levels()
                    && track
                        .selections
                        .get(usize::from(level))
                        .is_some_and(Option::is_some)
            });
            let Some(selected) = selected else {
                return Err(R::NoSelection);
            };
            if track
                .slots
                .iter()
                .flatten()
                .any(|other| other.level == level)
            {
                return Err(R::LevelTaken);
            }
            let pool = input.active.drawable().ok_or(R::NotAdmitted)?;
            let drawn = draw_by_weight(
                pool,
                |_, entry| entry.perk_identity == selected,
                1,
                &input.seed,
            )
            .ok_or(R::PoolTooSmall)?;
            plan(
                ProficiencyModificationOperation::Modify,
                Some(ProficiencyModifiedPerk {
                    level,
                    shaping_key: input.active.shaping_key.clone(),
                    shaping_revision: input.active.revision.clone(),
                    entry_index: drawn[0],
                    rank: 1,
                    pending_offer: None,
                }),
            )
        }
        Command::RankUp => {
            let mut after = ranked(row)?;
            after.rank += 1;
            plan(ProficiencyModificationOperation::RankUp, Some(after))
        }
        Command::OrbRank => {
            let mut after = ranked(row)?;
            after.rank = PROFICIENCY_MODIFICATION_RANK_MAX;
            plan(ProficiencyModificationOperation::OrbRank, Some(after))
        }
        Command::ReshapeOffer => {
            let row = row.ok_or(R::NotModified)?;
            if row.pending_offer.is_some() {
                return Err(R::OfferPending);
            }
            if outdated(row) {
                return Err(R::ShapingOutdated);
            }
            let pool = input.active.drawable().ok_or(R::NotAdmitted)?;
            let drawn = draw_by_weight(
                pool,
                |index, _| index == usize::from(row.entry_index),
                PROFICIENCY_RESHAPE_OFFER_SIZE,
                &input.seed,
            )
            .ok_or(R::PoolTooSmall)?;
            let mut after = row.clone();
            after.pending_offer = Some([drawn[0], drawn[1], drawn[2]]);
            plan(ProficiencyModificationOperation::ReshapeOffer, Some(after))
        }
        Command::ReshapeChoose { choice } => {
            let row = row.ok_or(R::NoOffer)?;
            let offer = row.pending_offer.ok_or(R::NoOffer)?;
            let mut after = row.clone();
            after.pending_offer = None;
            match choice {
                None => plan(
                    ProficiencyModificationOperation::ReshapeDecline,
                    Some(after),
                ),
                Some(choice) => {
                    after.entry_index = *offer.get(usize::from(choice)).ok_or(R::NoOffer)?;
                    plan(ProficiencyModificationOperation::ReshapeChoose, Some(after))
                }
            }
        }
        Command::Clear => {
            row.ok_or(R::NotModified)?;
            plan(ProficiencyModificationOperation::Clear, None)
        }
    }
}

/// `PROF1B-RL-05`: a commit whose outcome is still unknown after this bound puts its occurrence
/// on a reconciliation hold; it stays refused until reconciliation reads its receipt or terminal
/// record (PROFICIENCY-1B §6.3).
pub fn proficiency_modification_needs_reconciliation(started_at_ms: u64, now_ms: u64) -> bool {
    now_ms.saturating_sub(started_at_ms) > PROFICIENCY_AMBIGUOUS_COMMIT_HOLD_MS
}

/// `PROF1B-RL-03`: at most two new modification occurrences per character in any second,
/// charged when an occurrence is reserved. A replay is answered before the cap is charged.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProficiencyModificationRateCap {
    reserved_at_ms: std::collections::VecDeque<u64>,
}

impl ProficiencyModificationRateCap {
    /// Charge one reservation at `now_ms`; `false` means `RATE_LIMITED` and charges nothing.
    pub fn charge(&mut self, now_ms: u64) -> bool {
        while self
            .reserved_at_ms
            .front()
            .is_some_and(|at| now_ms.saturating_sub(*at) >= 1_000)
        {
            self.reserved_at_ms.pop_front();
        }
        if self.reserved_at_ms.len() >= PROFICIENCY_MODIFICATIONS_PER_SECOND {
            return false;
        }
        self.reserved_at_ms.push_back(now_ms);
        true
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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod modification_tests {
    use super::*;
    use ProficiencyModificationCommandKind as Command;
    use ProficiencyModificationResult as R;

    const KEY: &str = "oteryn:proficiency-shaping.tibia.p1";

    fn entry(identity: &str) -> ProficiencyShapingEntry {
        ProficiencyShapingEntry {
            perk_identity: identity.into(),
            weight: Some(1),
            rank_values: [true; 10],
        }
    }

    fn shaping(revision: &str, entries: usize) -> ProficiencyShapingRevision {
        ProficiencyShapingRevision {
            shaping_key: KEY.into(),
            revision: revision.into(),
            pool: Some((0..entries).map(|i| entry(&format!("perk-{i}"))).collect()),
            modify_cost: [Some(0); 2],
            rank_step_cost: [Some(0); 9],
            reshape_offer_cost: Some(0),
            clear_cost: Some(0),
            orb_count: Some(0),
        }
    }

    fn perk(revision: &str, level: u8, entry_index: u8, rank: u8) -> ProficiencyModifiedPerk {
        ProficiencyModifiedPerk {
            level,
            shaping_key: KEY.into(),
            shaping_revision: revision.into(),
            entry_index,
            rank,
            pending_offer: None,
        }
    }

    fn shape() -> ProficiencySelectionShape {
        ProficiencySelectionShape::new(ProficiencyThresholdClass::Standard, &[3, 2, 1])
            .expect("shape")
    }

    fn revision(value: u64) -> CharacterRevision {
        CharacterRevision::new(value).expect("revision")
    }

    const LEVEL_THREE: u32 = 100_000;
    const SELECTIONS: [Option<u8>; 3] = [Some(0), Some(1), None];

    fn track<'a>(
        progress: u32,
        slots: [Option<&'a ProficiencyModifiedPerk>; 2],
    ) -> ProficiencyModificationTrack<'a> {
        ProficiencyModificationTrack {
            shape: shape(),
            progress,
            selections: &SELECTIONS,
            committed_track_revision: revision(5),
            slots,
        }
    }

    fn input<'a>(
        command: Command,
        active: &'a ProficiencyShapingRevision,
        track: Option<ProficiencyModificationTrack<'a>>,
    ) -> ProficiencyModificationInput<'a> {
        ProficiencyModificationInput {
            slot: 1,
            command,
            expected_track_revision: revision(5),
            in_protection_zone: true,
            active,
            track,
            selected_identity: Some("perk-0"),
            seed: [7; 32],
        }
    }

    #[test]
    fn admission_reads_only_evidenced_cells() {
        let full = shaping("r1", 4);
        let row = perk("r1", 0, 1, 2);
        for command in [
            Command::Modify { level: 0 },
            Command::RankUp,
            Command::OrbRank,
            Command::ReshapeOffer,
            Command::ReshapeChoose { choice: Some(0) },
            Command::Clear,
        ] {
            assert!(
                full.admitted(command, 1, Some(&row)).is_some(),
                "{command:?}"
            );
        }
        let mut cost = full.clone();
        cost.modify_cost[1] = None;
        assert!(
            cost.admitted(Command::Modify { level: 0 }, 1, None)
                .is_some()
        );
        assert!(
            cost.admitted(Command::Modify { level: 0 }, 2, None)
                .is_none()
        );
        let mut weight = full.clone();
        weight.pool.as_mut().expect("pool")[3].weight = None;
        assert!(
            weight
                .admitted(Command::Modify { level: 0 }, 1, None)
                .is_none()
        );
        assert!(
            weight
                .admitted(Command::ReshapeOffer, 1, Some(&row))
                .is_none()
        );
        // RESHAPE_OFFER reads every entry's values at the row's rank, not only its own.
        let mut values = full.clone();
        values.pool.as_mut().expect("pool")[3].rank_values[1] = false;
        assert!(
            values
                .admitted(Command::ReshapeOffer, 1, Some(&row))
                .is_none()
        );
        assert!(values.admitted(Command::RankUp, 1, Some(&row)).is_some());
        assert!(
            values
                .admitted(Command::Modify { level: 0 }, 1, None)
                .is_some()
        );
        let mut next = full.clone();
        next.pool.as_mut().expect("pool")[1].rank_values[2] = false;
        assert!(next.admitted(Command::RankUp, 1, Some(&row)).is_none());
        let mut orb = full.clone();
        orb.orb_count = None;
        assert!(orb.admitted(Command::OrbRank, 1, Some(&row)).is_none());
        let mut clear = full.clone();
        clear.clear_cost = None;
        assert!(clear.admitted(Command::Clear, 1, Some(&row)).is_none());
        // RESHAPE_CHOOSE follows the pending offer, whatever the active cells.
        let unknown = ProficiencyShapingRevision {
            pool: None,
            ..clear
        };
        assert!(
            unknown
                .admitted(Command::ReshapeChoose { choice: None }, 1, Some(&row))
                .is_some()
        );
        assert!(
            unknown
                .admitted(Command::Modify { level: 0 }, 1, None)
                .is_none()
        );
    }

    #[test]
    fn checks_run_in_the_decision_order() {
        let mut closed = shaping("r1", 4);
        closed.clear_cost = None;
        assert_eq!(
            plan_proficiency_modification(&input(Command::Clear, &closed, None)),
            Err(R::NotAdmitted)
        );
        let active = shaping("r1", 4);
        assert_eq!(
            plan_proficiency_modification(&input(Command::Clear, &active, None)),
            Err(R::UnknownTrack)
        );
        let mut stale = input(Command::Clear, &active, Some(track(LEVEL_THREE, [None; 2])));
        stale.expected_track_revision = revision(4);
        assert_eq!(plan_proficiency_modification(&stale), Err(R::StaleRevision));
        let mut outside = input(Command::Clear, &active, Some(track(LEVEL_THREE, [None; 2])));
        outside.in_protection_zone = false;
        assert_eq!(
            plan_proficiency_modification(&outside),
            Err(R::NotInProtectionZone)
        );
        assert_eq!(
            plan_proficiency_modification(&input(
                Command::Clear,
                &active,
                Some(track(LEVEL_THREE, [None; 2]))
            )),
            Err(R::NotModified)
        );
    }

    #[test]
    fn modify_checks_unlock_occupancy_selection_level_and_pool() {
        let active = shaping("r1", 4);
        let modify = |level, slots, progress| {
            plan_proficiency_modification(&input(
                Command::Modify { level },
                &active,
                Some(track(progress, slots)),
            ))
        };
        assert_eq!(modify(0, [None; 2], 99_999), Err(R::NotUnlocked));
        let held = perk("r1", 1, 2, 1);
        assert_eq!(
            modify(0, [Some(&held), None], LEVEL_THREE),
            Err(R::SlotOccupied)
        );
        assert_eq!(modify(2, [None; 2], LEVEL_THREE), Err(R::NoSelection));
        assert_eq!(modify(6, [None; 2], LEVEL_THREE), Err(R::NoSelection));
        let mut second = input(
            Command::Modify { level: 1 },
            &active,
            Some(track(u32::MAX, [None, Some(&held)])),
        );
        assert_eq!(plan_proficiency_modification(&second), Err(R::LevelTaken));
        second.slot = 2;
        assert_eq!(plan_proficiency_modification(&second), Err(R::SlotOccupied));
        let lonely = ProficiencyShapingRevision {
            pool: Some(vec![entry("perk-0")]),
            ..active.clone()
        };
        assert_eq!(
            plan_proficiency_modification(&input(
                Command::Modify { level: 0 },
                &lonely,
                Some(track(LEVEL_THREE, [None; 2]))
            )),
            Err(R::PoolTooSmall)
        );
        let plan = modify(0, [None; 2], LEVEL_THREE).expect("plan");
        assert_eq!(plan.operation, ProficiencyModificationOperation::Modify);
        let after = plan.after.clone().expect("modified");
        assert_ne!(after.entry_index, 0, "the selected perk is never drawn");
        assert_eq!((after.level, after.rank, after.pending_offer), (0, 1, None));
        assert!(after.is_well_formed());
        assert_eq!(plan, modify(0, [None; 2], LEVEL_THREE).expect("same draw"));
    }

    #[test]
    fn rank_operations_refuse_max_offer_and_outdated_revisions() {
        let active = shaping("r2", 4);
        let rank = |command, row: &ProficiencyModifiedPerk| {
            plan_proficiency_modification(&input(
                command,
                &active,
                Some(track(LEVEL_THREE, [Some(row), None])),
            ))
        };
        assert_eq!(
            plan_proficiency_modification(&input(
                Command::RankUp,
                &active,
                Some(track(LEVEL_THREE, [None; 2]))
            )),
            Err(R::NotModified)
        );
        assert_eq!(
            rank(Command::RankUp, &perk("r2", 0, 1, 10)),
            Err(R::RankMax)
        );
        let mut offered = perk("r2", 0, 1, 3);
        offered.pending_offer = Some([0, 2, 3]);
        assert_eq!(rank(Command::RankUp, &offered), Err(R::OfferPending));
        assert_eq!(rank(Command::OrbRank, &offered), Err(R::OfferPending));
        assert_eq!(rank(Command::ReshapeOffer, &offered), Err(R::OfferPending));
        let old = perk("r1", 0, 1, 3);
        assert_eq!(rank(Command::RankUp, &old), Err(R::ShapingOutdated));
        assert_eq!(rank(Command::ReshapeOffer, &old), Err(R::ShapingOutdated));
        assert_eq!(
            rank(Command::Clear, &old).map(|plan| plan.after),
            Ok(None),
            "CLEAR stays open on an older revision"
        );
        let up = rank(Command::RankUp, &perk("r2", 0, 1, 3)).expect("rank up");
        assert_eq!(up.after.map(|row| row.rank), Some(4));
        let orb = rank(Command::OrbRank, &perk("r2", 0, 1, 3)).expect("orb");
        assert_eq!(orb.after.map(|row| row.rank), Some(10));
    }

    #[test]
    fn offers_are_three_distinct_others_and_choices_keep_the_rank() {
        let small = shaping("r1", 3);
        let row = perk("r1", 0, 1, 4);
        assert_eq!(
            plan_proficiency_modification(&input(
                Command::ReshapeOffer,
                &small,
                Some(track(LEVEL_THREE, [Some(&row), None]))
            )),
            Err(R::PoolTooSmall)
        );
        let active = shaping("r1", 4);
        let plan = plan_proficiency_modification(&input(
            Command::ReshapeOffer,
            &active,
            Some(track(LEVEL_THREE, [Some(&row), None])),
        ))
        .expect("offer");
        let offered = plan.after.expect("row");
        let offer = offered.pending_offer.expect("offer");
        assert!(offered.is_well_formed());
        assert!(!offer.contains(&1));
        let choose = |choice, row: &ProficiencyModifiedPerk| {
            plan_proficiency_modification(&input(
                Command::ReshapeChoose { choice },
                &active,
                Some(track(LEVEL_THREE, [Some(row), None])),
            ))
        };
        assert_eq!(choose(Some(0), &row), Err(R::NoOffer));
        let chosen = choose(Some(2), &offered).expect("choose");
        assert_eq!(
            chosen.operation,
            ProficiencyModificationOperation::ReshapeChoose
        );
        let after = chosen.after.expect("row");
        assert_eq!((after.entry_index, after.rank), (offer[2], 4));
        assert_eq!(after.pending_offer, None);
        let declined = choose(None, &offered).expect("decline");
        assert_eq!(
            declined.operation,
            ProficiencyModificationOperation::ReshapeDecline
        );
        assert_eq!(declined.after, Some(row.clone()));
        // A paid offer stays choosable after a newer revision becomes active.
        let newer = shaping("r9", 4);
        assert!(
            plan_proficiency_modification(&input(
                Command::ReshapeChoose { choice: Some(1) },
                &newer,
                Some(track(LEVEL_THREE, [Some(&offered), None])),
            ))
            .is_ok()
        );
    }

    #[test]
    fn draws_depend_on_seed_material_only() {
        let a = proficiency_shaping_seed(&[1; 32], "sim-1", "r1", &[2; 16]);
        assert_eq!(
            a,
            proficiency_shaping_seed(&[1; 32], "sim-1", "r1", &[2; 16])
        );
        assert_ne!(
            a,
            proficiency_shaping_seed(&[1; 32], "sim-1", "r1", &[3; 16])
        );
        assert_ne!(
            a,
            proficiency_shaping_seed(&[9; 32], "sim-1", "r1", &[2; 16])
        );
        assert_ne!(
            a,
            proficiency_shaping_seed(&[1; 32], "sim-2", "r1", &[2; 16])
        );
        let mut pool = shaping("r1", 4).pool.expect("pool");
        pool[2].weight = Some(u32::MAX);
        let drawn = draw_by_weight(&pool, |_, _| false, 1, &a).expect("draw");
        assert_eq!(drawn, vec![2]);
        let all = draw_by_weight(&pool, |_, _| false, 4, &a).expect("draw all");
        let mut sorted = all.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, vec![0, 1, 2, 3]);
        assert!(draw_by_weight(&pool, |_, _| false, 5, &a).is_none());
    }

    #[test]
    fn activity_follows_unlock_and_selection() {
        let row = perk("r1", 0, 1, 1);
        assert!(!proficiency_modification_active(
            shape(),
            99_999,
            &SELECTIONS,
            1,
            &row
        ));
        assert!(proficiency_modification_active(
            shape(),
            LEVEL_THREE,
            &SELECTIONS,
            1,
            &row
        ));
        assert!(
            !proficiency_modification_active(shape(), LEVEL_THREE, &SELECTIONS, 2, &row),
            "slot 2 above the unlocked level is stored and inactive"
        );
        let mastery = shape().track_shape().final_progress();
        assert!(proficiency_modification_active(
            shape(),
            mastery,
            &SELECTIONS,
            2,
            &row
        ));
        let unselected = perk("r1", 2, 1, 1);
        assert!(!proficiency_modification_active(
            shape(),
            mastery,
            &SELECTIONS,
            1,
            &unselected
        ));
    }

    #[test]
    fn limits_hold_at_max_and_refuse_max_plus_one() {
        assert!(!proficiency_modification_needs_reconciliation(1_000, 3_000));
        assert!(proficiency_modification_needs_reconciliation(1_000, 3_001));
        let mut cap = ProficiencyModificationRateCap::default();
        assert!(cap.charge(1_000));
        assert!(cap.charge(1_500));
        assert!(!cap.charge(1_999), "a third reservation in one second");
        assert!(cap.charge(2_000));
        let mut pool = shaping("r1", PROFICIENCY_SHAPING_POOL_MAX);
        assert!(pool.is_well_formed());
        pool.pool.as_mut().expect("pool").push(entry("perk-extra"));
        assert!(!pool.is_well_formed());
        let mut row = perk("r1", 0, 63, PROFICIENCY_MODIFICATION_RANK_MAX);
        assert!(row.is_well_formed());
        row.rank += 1;
        assert!(!row.is_well_formed());
        row.rank = 1;
        row.entry_index = 64;
        assert!(!row.is_well_formed());
        row.entry_index = 1;
        row.pending_offer = Some([2, 3, 1]);
        assert!(!row.is_well_formed(), "the current entry is never offered");
        let mut orb = shaping("r1", 4);
        orb.orb_count = Some(2);
        assert!(!orb.is_well_formed());
        let mut weight = shaping("r1", 4);
        weight.pool.as_mut().expect("pool")[0].weight = Some(0);
        assert!(!weight.is_well_formed());
    }
}
