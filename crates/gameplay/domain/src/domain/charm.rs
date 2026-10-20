//! Pure Charm unlock, assignment and balance rules (CHARM-0 decision packet §4.2, owner answers
//! §7 of 2026-09-29; CHARM-3).
//!
//! Only unlocks and assignments are persisted. Charm Points and Minor Charm Echoes are derived
//! from them, from the completed Bestiary entries and from promotion, and are never stored
//! (owner answer 2a). There is no unassign in this slice (owner answer 3c): it waits for the
//! `GAME-ITEM-01`/`DUR-03` Character and Item boundary and then ships with its gold fee.

use std::collections::BTreeMap;

/// Longest charm or Bestiary race key, in bytes (the revision-class key bound).
pub const MAX_KEY_BYTES: usize = 128;
/// Every charm key is minted as `oteryn:charm.<name>` (CHARM-1).
pub const CHARM_KEY_PREFIX: &str = "oteryn:charm.";
/// A Bestiary race is keyed by the Creature definition key of its entry (§4.1).
pub const BESTIARY_RACE_KEY_PREFIX: &str = "oteryn:creature.";
/// Each charm has exactly three stages.
pub const CHARM_STAGE_COUNT: usize = 3;
/// Minor Charm Echoes earned by unlocking major stage 1, 2 and 3 (TibiaWiki, *Minor Charms*).
pub const MAJOR_STAGE_ECHOES: [u64; CHARM_STAGE_COUNT] = [50, 100, 200];
/// Minor Charm Echoes earned once by a promoted Character (TibiaWiki, *Minor Charms*).
pub const PROMOTION_ECHOES: u64 = 100;
/// A major charm is assigned only to a completed entry (its final stage).
pub const MAJOR_ASSIGN_MINIMUM_STAGE: BestiaryStage = BestiaryStage::FINAL;
/// A minor charm is assigned to an entry at stage 2 or higher.
pub const MINOR_ASSIGN_MINIMUM_STAGE: BestiaryStage = BestiaryStage(2);

/// Assigned charms of both categories count against one slot limit (Canary `iobestiary.cpp`;
/// the TibiaWiki text predates the 14.10 rework). ASSUMPTION until the owner confirms it.
pub const FREE_ASSIGNMENT_SLOTS: usize = 2;
/// The slot limit while Premium is current (Canary).
pub const PREMIUM_ASSIGNMENT_SLOTS: usize = 6;

/// What bounds a Character's number of assigned charms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharmSlotEntitlement {
    Free,
    Premium,
    /// The Store Charm Expansion: no slot limit.
    CharmExpansion,
}

impl CharmSlotEntitlement {
    /// `None` means unlimited.
    #[must_use]
    pub const fn assignment_slots(self) -> Option<usize> {
        match self {
            Self::Free => Some(FREE_ASSIGNMENT_SLOTS),
            Self::Premium => Some(PREMIUM_ASSIGNMENT_SLOTS),
            Self::CharmExpansion => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharmRuleError {
    InvalidCharmKey,
    InvalidRaceKey,
    InvalidStage,
    InvalidCatalogue,
    UnknownCharm,
    FinalStageReached,
    InsufficientBalance,
    CharmLocked,
    CharmAlreadyAssigned,
    BestiaryStageTooLow,
    RaceCapacityReached,
    AssignmentSlotsFull,
    Overflow,
}

fn valid_key(value: &str, prefix: &str) -> bool {
    value.len() <= MAX_KEY_BYTES
        && value.strip_prefix(prefix).is_some_and(|name| {
            !name.is_empty()
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        })
}

/// A validated `oteryn:charm.<name>` key.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CharmKey(String);

impl CharmKey {
    pub fn new(value: impl Into<String>) -> Result<Self, CharmRuleError> {
        let value = value.into();
        if !valid_key(&value, CHARM_KEY_PREFIX) {
            return Err(CharmRuleError::InvalidCharmKey);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A validated Bestiary race key, the `oteryn:creature.<name>` key of the entry's definition.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BestiaryRaceKey(String);

impl BestiaryRaceKey {
    pub fn new(value: impl Into<String>) -> Result<Self, CharmRuleError> {
        let value = value.into();
        if !valid_key(&value, BESTIARY_RACE_KEY_PREFIX) {
            return Err(CharmRuleError::InvalidRaceKey);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CharmCategory {
    Major,
    Minor,
}

impl CharmCategory {
    /// Major stages are paid with Charm Points, minor stages with Minor Charm Echoes.
    #[must_use]
    pub const fn currency(self) -> CharmCurrency {
        match self {
            Self::Major => CharmCurrency::CharmPoints,
            Self::Minor => CharmCurrency::MinorCharmEchoes,
        }
    }

    #[must_use]
    pub const fn assign_minimum_stage(self) -> BestiaryStage {
        match self {
            Self::Major => MAJOR_ASSIGN_MINIMUM_STAGE,
            Self::Minor => MINOR_ASSIGN_MINIMUM_STAGE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharmCurrency {
    CharmPoints,
    MinorCharmEchoes,
}

/// The completed Bestiary stage of one race for one Character: 0 (none) to 3 (entry completed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BestiaryStage(u8);

impl BestiaryStage {
    pub const NONE: Self = Self(0);
    pub const FINAL: Self = Self(3);

    pub const fn new(value: u8) -> Result<Self, CharmRuleError> {
        if value > Self::FINAL.0 {
            return Err(CharmRuleError::InvalidStage);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// An unlocked charm stage, 1 to 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CharmStage(u8);

impl CharmStage {
    pub const FIRST: Self = Self(1);
    pub const FINAL: Self = Self(3);

    pub const fn new(value: u8) -> Result<Self, CharmRuleError> {
        if value == 0 || value > Self::FINAL.0 {
            return Err(CharmRuleError::InvalidStage);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// One charm of the static catalogue: category and the cost of each of its three stages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharmDefinition {
    pub key: CharmKey,
    pub category: CharmCategory,
    pub stage_costs: [u32; CHARM_STAGE_COUNT],
}

/// The static charm catalogue, keyed by charm key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharmCatalogue(BTreeMap<CharmKey, CharmDefinition>);

impl CharmCatalogue {
    /// Rejects an empty catalogue, a duplicate key and a zero stage cost.
    pub fn new(
        definitions: impl IntoIterator<Item = CharmDefinition>,
    ) -> Result<Self, CharmRuleError> {
        let mut map = BTreeMap::new();
        for definition in definitions {
            if definition.stage_costs.contains(&0) {
                return Err(CharmRuleError::InvalidCatalogue);
            }
            if map.insert(definition.key.clone(), definition).is_some() {
                return Err(CharmRuleError::InvalidCatalogue);
            }
        }
        if map.is_empty() {
            return Err(CharmRuleError::InvalidCatalogue);
        }
        Ok(Self(map))
    }

    pub fn get(&self, key: &CharmKey) -> Result<&CharmDefinition, CharmRuleError> {
        self.0.get(key).ok_or(CharmRuleError::UnknownCharm)
    }

    /// Definitions in key order.
    pub fn definitions(&self) -> impl Iterator<Item = &CharmDefinition> {
        self.0.values()
    }
}

/// Derived Charm Points and Minor Charm Echoes. Spent can exceed earned only after a Bestiary
/// definition revision raised a threshold; `available` is then negative and nothing is affordable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharmBalance {
    pub points_earned: u64,
    pub points_spent: u64,
    pub echoes_earned: u64,
    pub echoes_spent: u64,
}

impl CharmBalance {
    #[must_use]
    pub fn available(&self, currency: CharmCurrency) -> i128 {
        let (earned, spent) = match currency {
            CharmCurrency::CharmPoints => (self.points_earned, self.points_spent),
            CharmCurrency::MinorCharmEchoes => (self.echoes_earned, self.echoes_spent),
        };
        i128::from(earned) - i128::from(spent)
    }

    #[must_use]
    pub fn affords(&self, currency: CharmCurrency, cost: u32) -> bool {
        self.available(currency) >= i128::from(cost)
    }
}

fn checked_sum(values: impl IntoIterator<Item = u64>) -> Result<u64, CharmRuleError> {
    values
        .into_iter()
        .try_fold(0_u64, u64::checked_add)
        .ok_or(CharmRuleError::Overflow)
}

/// Owner answer 2a:
/// - Charm Points earned is the sum of `charm_points` over the completed Bestiary entries;
/// - Echoes earned is 50/100/200 per major stage unlocked, plus 100 for a promoted Character;
/// - spent is the sum of the unlocked stage costs, per currency.
///
/// An unlocked charm that is missing from the catalogue fails closed.
pub fn derive_balance(
    catalogue: &CharmCatalogue,
    unlocks: &BTreeMap<CharmKey, CharmStage>,
    completed_entry_charm_points: impl IntoIterator<Item = u32>,
    promoted: bool,
) -> Result<CharmBalance, CharmRuleError> {
    let points_earned = checked_sum(completed_entry_charm_points.into_iter().map(u64::from))?;
    let mut points_spent = 0_u64;
    let mut echoes_spent = 0_u64;
    let mut echoes_earned = if promoted { PROMOTION_ECHOES } else { 0 };
    for (key, stage) in unlocks {
        let definition = catalogue.get(key)?;
        let reached = usize::from(stage.get());
        let cost = checked_sum(
            definition.stage_costs[..reached]
                .iter()
                .copied()
                .map(u64::from),
        )?;
        match definition.category {
            CharmCategory::Major => {
                points_spent = points_spent
                    .checked_add(cost)
                    .ok_or(CharmRuleError::Overflow)?;
                let echoes = checked_sum(MAJOR_STAGE_ECHOES[..reached].iter().copied())?;
                echoes_earned = echoes_earned
                    .checked_add(echoes)
                    .ok_or(CharmRuleError::Overflow)?;
            }
            CharmCategory::Minor => {
                echoes_spent = echoes_spent
                    .checked_add(cost)
                    .ok_or(CharmRuleError::Overflow)?;
            }
        }
    }
    Ok(CharmBalance {
        points_earned,
        points_spent,
        echoes_earned,
        echoes_spent,
    })
}

/// The next stage of one charm and what it costs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnlockPlan {
    pub charm: CharmKey,
    pub category: CharmCategory,
    /// 0 when the charm was locked.
    pub stage_before: u8,
    pub stage_after: CharmStage,
    pub cost: u32,
}

/// Unlock the next stage of `charm` if the derived balance of its currency affords it.
pub fn plan_unlock(
    catalogue: &CharmCatalogue,
    unlocks: &BTreeMap<CharmKey, CharmStage>,
    balance: &CharmBalance,
    charm: &CharmKey,
) -> Result<UnlockPlan, CharmRuleError> {
    let definition = catalogue.get(charm)?;
    let stage_before = unlocks.get(charm).map_or(0, |stage| stage.get());
    if stage_before >= CharmStage::FINAL.get() {
        return Err(CharmRuleError::FinalStageReached);
    }
    let stage_after = CharmStage::new(stage_before + 1)?;
    let cost = definition.stage_costs[usize::from(stage_before)];
    if !balance.affords(definition.category.currency(), cost) {
        return Err(CharmRuleError::InsufficientBalance);
    }
    Ok(UnlockPlan {
        charm: charm.clone(),
        category: definition.category,
        stage_before,
        stage_after,
        cost,
    })
}

/// Since 14.10 a Bestiary race holds at most one major and one minor charm at the same time
/// (TibiaWiki *Updates/14.10*; Canary `iobestiary.cpp`): it admits `category` unless one of the
/// `held` charms has that category.
#[must_use]
pub fn race_admits(held: &[CharmCategory], category: CharmCategory) -> bool {
    !held.contains(&category)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignPlan {
    pub charm: CharmKey,
    pub category: CharmCategory,
    pub race: BestiaryRaceKey,
}

/// Assign an unlocked, unassigned charm to a race whose completed Bestiary stage meets the
/// category's minimum, if the race has room for the category and the Character a free slot.
pub fn plan_assign(
    catalogue: &CharmCatalogue,
    unlocks: &BTreeMap<CharmKey, CharmStage>,
    assignments: &BTreeMap<CharmKey, BestiaryRaceKey>,
    charm: &CharmKey,
    race: &BestiaryRaceKey,
    race_stage: BestiaryStage,
    entitlement: CharmSlotEntitlement,
) -> Result<AssignPlan, CharmRuleError> {
    let definition = catalogue.get(charm)?;
    if !unlocks.contains_key(charm) {
        return Err(CharmRuleError::CharmLocked);
    }
    if assignments.contains_key(charm) {
        return Err(CharmRuleError::CharmAlreadyAssigned);
    }
    if race_stage < definition.category.assign_minimum_stage() {
        return Err(CharmRuleError::BestiaryStageTooLow);
    }
    let held = assignments
        .iter()
        .filter(|(_, assigned)| *assigned == race)
        .map(|(key, _)| catalogue.get(key).map(|held| held.category))
        .collect::<Result<Vec<_>, _>>()?;
    if !race_admits(&held, definition.category) {
        return Err(CharmRuleError::RaceCapacityReached);
    }
    if entitlement
        .assignment_slots()
        .is_some_and(|slots| assignments.len() >= slots)
    {
        return Err(CharmRuleError::AssignmentSlotsFull);
    }
    Ok(AssignPlan {
        charm: charm.clone(),
        category: definition.category,
        race: race.clone(),
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn charm(name: &str) -> CharmKey {
        CharmKey::new(format!("oteryn:charm.{name}")).expect("charm key")
    }

    fn race(name: &str) -> BestiaryRaceKey {
        BestiaryRaceKey::new(format!("oteryn:creature.{name}")).expect("race key")
    }

    fn stage(value: u8) -> CharmStage {
        CharmStage::new(value).expect("stage")
    }

    /// Two majors and two minors with the candidate catalogue's costs.
    fn catalogue() -> CharmCatalogue {
        CharmCatalogue::new([
            CharmDefinition {
                key: charm("wound"),
                category: CharmCategory::Major,
                stage_costs: [240, 360, 1200],
            },
            CharmDefinition {
                key: charm("zap"),
                category: CharmCategory::Major,
                stage_costs: [320, 480, 1600],
            },
            CharmDefinition {
                key: charm("gut"),
                category: CharmCategory::Minor,
                stage_costs: [100, 150, 225],
            },
            CharmDefinition {
                key: charm("bless"),
                category: CharmCategory::Minor,
                stage_costs: [100, 150, 225],
            },
        ])
        .expect("catalogue")
    }

    #[test]
    fn keys_are_bounded_prefixed_lowercase_names() {
        assert!(CharmKey::new("oteryn:charm.low_blow").is_ok());
        assert!(BestiaryRaceKey::new("oteryn:creature.dragon_lord").is_ok());
        for bad in [
            "",
            "oteryn:charm.",
            "oteryn:charm.Low",
            "oteryn:charm.low-blow",
            "oteryn:creature.rat",
            "charm.low_blow",
        ] {
            assert_eq!(
                CharmKey::new(bad),
                Err(CharmRuleError::InvalidCharmKey),
                "{bad}"
            );
        }
        assert_eq!(
            BestiaryRaceKey::new("oteryn:charm.rat"),
            Err(CharmRuleError::InvalidRaceKey)
        );
        let long = format!("oteryn:creature.{}", "a".repeat(MAX_KEY_BYTES));
        assert_eq!(
            BestiaryRaceKey::new(long),
            Err(CharmRuleError::InvalidRaceKey)
        );
        let longest = format!("oteryn:creature.{}", "a".repeat(MAX_KEY_BYTES - 16));
        assert!(BestiaryRaceKey::new(longest).is_ok());
    }

    #[test]
    fn stages_are_bounded() {
        assert_eq!(CharmStage::new(0), Err(CharmRuleError::InvalidStage));
        assert_eq!(CharmStage::new(4), Err(CharmRuleError::InvalidStage));
        assert_eq!(BestiaryStage::new(4), Err(CharmRuleError::InvalidStage));
        assert_eq!(BestiaryStage::new(0), Ok(BestiaryStage::NONE));
        assert_eq!(BestiaryStage::new(3), Ok(BestiaryStage::FINAL));
    }

    #[test]
    fn catalogue_rejects_empty_duplicate_and_zero_cost() {
        assert_eq!(
            CharmCatalogue::new([]),
            Err(CharmRuleError::InvalidCatalogue)
        );
        let definition = CharmDefinition {
            key: charm("wound"),
            category: CharmCategory::Major,
            stage_costs: [240, 360, 1200],
        };
        assert_eq!(
            CharmCatalogue::new([definition.clone(), definition.clone()]),
            Err(CharmRuleError::InvalidCatalogue)
        );
        let mut free = definition;
        free.stage_costs[2] = 0;
        assert_eq!(
            CharmCatalogue::new([free]),
            Err(CharmRuleError::InvalidCatalogue)
        );
    }

    #[test]
    fn balance_is_derived_from_entries_unlocks_and_promotion() {
        let catalogue = catalogue();
        let none = BTreeMap::new();
        let empty = derive_balance(&catalogue, &none, [], false).expect("empty");
        assert_eq!(
            empty,
            CharmBalance {
                points_earned: 0,
                points_spent: 0,
                echoes_earned: 0,
                echoes_spent: 0,
            }
        );
        let promoted = derive_balance(&catalogue, &none, [], true).expect("promoted");
        assert_eq!(promoted.echoes_earned, PROMOTION_ECHOES);

        let unlocks = BTreeMap::from([
            (charm("wound"), stage(3)),
            (charm("zap"), stage(1)),
            (charm("gut"), stage(2)),
        ]);
        let balance = derive_balance(&catalogue, &unlocks, [15, 25, 1000], true).expect("balance");
        assert_eq!(balance.points_earned, 1040);
        assert_eq!(balance.points_spent, 240 + 360 + 1200 + 320);
        assert_eq!(balance.echoes_earned, 100 + (50 + 100 + 200) + 50);
        assert_eq!(balance.echoes_spent, 100 + 150);
        assert_eq!(
            balance.available(CharmCurrency::CharmPoints),
            1040 - (240 + 360 + 1200 + 320)
        );
        assert_eq!(
            balance.available(CharmCurrency::MinorCharmEchoes),
            500 - 250
        );
    }

    #[test]
    fn an_unlocked_charm_missing_from_the_catalogue_fails_closed() {
        let unlocks = BTreeMap::from([(charm("unknown"), stage(1))]);
        assert_eq!(
            derive_balance(&catalogue(), &unlocks, [], false),
            Err(CharmRuleError::UnknownCharm)
        );
    }

    #[test]
    fn sums_fail_closed_on_overflow() {
        assert_eq!(checked_sum([u64::MAX, 1]), Err(CharmRuleError::Overflow));
        assert_eq!(checked_sum([u64::MAX - 1, 1]), Ok(u64::MAX));
    }

    #[test]
    fn unlock_takes_the_next_stage_and_needs_its_exact_cost() {
        let catalogue = catalogue();
        let mut unlocks = BTreeMap::new();
        let exact = derive_balance(&catalogue, &unlocks, [240], false).expect("balance");
        let plan = plan_unlock(&catalogue, &unlocks, &exact, &charm("wound")).expect("unlock");
        assert_eq!(plan.stage_before, 0);
        assert_eq!(plan.stage_after, CharmStage::FIRST);
        assert_eq!(plan.cost, 240);
        assert_eq!(plan.category, CharmCategory::Major);

        let short = derive_balance(&catalogue, &unlocks, [239], false).expect("balance");
        assert_eq!(
            plan_unlock(&catalogue, &unlocks, &short, &charm("wound")),
            Err(CharmRuleError::InsufficientBalance)
        );

        unlocks.insert(charm("wound"), stage(1));
        let after = derive_balance(&catalogue, &unlocks, [240 + 360], false).expect("balance");
        let second = plan_unlock(&catalogue, &unlocks, &after, &charm("wound")).expect("second");
        assert_eq!(
            (second.stage_before, second.stage_after.get(), second.cost),
            (1, 2, 360)
        );

        unlocks.insert(charm("wound"), stage(3));
        let rich = derive_balance(&catalogue, &unlocks, [u32::MAX], false).expect("balance");
        assert_eq!(
            plan_unlock(&catalogue, &unlocks, &rich, &charm("wound")),
            Err(CharmRuleError::FinalStageReached)
        );
        assert_eq!(
            plan_unlock(&catalogue, &unlocks, &rich, &charm("unknown")),
            Err(CharmRuleError::UnknownCharm)
        );
    }

    #[test]
    fn minor_stages_are_paid_with_echoes_only() {
        let catalogue = catalogue();
        let none = BTreeMap::new();
        // Charm Points never buy a minor stage.
        let points_only = derive_balance(&catalogue, &none, [10_000], false).expect("balance");
        assert_eq!(
            plan_unlock(&catalogue, &none, &points_only, &charm("gut")),
            Err(CharmRuleError::InsufficientBalance)
        );
        // Promotion echoes buy exactly the first minor stage.
        let promoted = derive_balance(&catalogue, &none, [], true).expect("balance");
        let plan = plan_unlock(&catalogue, &none, &promoted, &charm("gut")).expect("gut");
        assert_eq!(plan.cost, 100);
        // Echoes never buy a major stage.
        assert_eq!(
            plan_unlock(&catalogue, &none, &promoted, &charm("wound")),
            Err(CharmRuleError::InsufficientBalance)
        );
    }

    #[test]
    fn a_deficit_after_a_threshold_revision_affords_nothing() {
        let catalogue = catalogue();
        let unlocks = BTreeMap::from([(charm("wound"), stage(1))]);
        let balance = derive_balance(&catalogue, &unlocks, [], false).expect("balance");
        assert_eq!(balance.available(CharmCurrency::CharmPoints), -240);
        assert!(!balance.affords(CharmCurrency::CharmPoints, 1));
        // The 50 echoes earned by the unlocked major stage stay available.
        assert!(balance.affords(CharmCurrency::MinorCharmEchoes, 50));
    }

    fn assign(
        catalogue: &CharmCatalogue,
        unlocks: &BTreeMap<CharmKey, CharmStage>,
        assignments: &BTreeMap<CharmKey, BestiaryRaceKey>,
        key: &str,
        to: &str,
        stage: u8,
        entitlement: CharmSlotEntitlement,
    ) -> Result<AssignPlan, CharmRuleError> {
        plan_assign(
            catalogue,
            unlocks,
            assignments,
            &charm(key),
            &race(to),
            BestiaryStage::new(stage).expect("stage"),
            entitlement,
        )
    }

    #[test]
    fn assign_requires_an_unlocked_unassigned_charm_and_the_category_stage() {
        use CharmSlotEntitlement::CharmExpansion;
        let catalogue = catalogue();
        let unlocks = BTreeMap::from([(charm("wound"), stage(1)), (charm("gut"), stage(1))]);
        let none = BTreeMap::new();

        let plan = assign(
            &catalogue,
            &unlocks,
            &none,
            "wound",
            "rat",
            3,
            CharmExpansion,
        )
        .expect("major on a completed entry");
        assert_eq!(plan.category, CharmCategory::Major);
        assert_eq!(
            assign(
                &catalogue,
                &unlocks,
                &none,
                "wound",
                "rat",
                2,
                CharmExpansion
            ),
            Err(CharmRuleError::BestiaryStageTooLow)
        );
        assert!(assign(&catalogue, &unlocks, &none, "gut", "rat", 2, CharmExpansion).is_ok());
        assert_eq!(
            assign(&catalogue, &unlocks, &none, "gut", "rat", 1, CharmExpansion),
            Err(CharmRuleError::BestiaryStageTooLow)
        );
        assert_eq!(
            assign(&catalogue, &unlocks, &none, "zap", "rat", 3, CharmExpansion),
            Err(CharmRuleError::CharmLocked)
        );
        assert_eq!(
            assign(
                &catalogue,
                &unlocks,
                &none,
                "unknown",
                "rat",
                3,
                CharmExpansion
            ),
            Err(CharmRuleError::UnknownCharm)
        );

        // No unassign and no re-assign in this slice (owner answer 3c).
        let assigned = BTreeMap::from([(charm("wound"), race("wolf"))]);
        for to in ["rat", "wolf"] {
            assert_eq!(
                assign(
                    &catalogue,
                    &unlocks,
                    &assigned,
                    "wound",
                    to,
                    3,
                    CharmExpansion
                ),
                Err(CharmRuleError::CharmAlreadyAssigned)
            );
        }
    }

    #[test]
    fn a_race_holds_one_major_and_one_minor_charm() {
        use CharmCategory::{Major, Minor};
        use CharmSlotEntitlement::CharmExpansion;
        assert!(race_admits(&[], Major));
        assert!(race_admits(&[Major], Minor));
        assert!(race_admits(&[Minor], Major));
        assert!(!race_admits(&[Major], Major));
        assert!(!race_admits(&[Minor], Minor));
        assert!(!race_admits(&[Major, Minor], Minor));

        let catalogue = catalogue();
        let unlocks = BTreeMap::from([
            (charm("wound"), stage(1)),
            (charm("zap"), stage(1)),
            (charm("gut"), stage(1)),
            (charm("bless"), stage(1)),
        ]);
        let assigned = BTreeMap::from([(charm("wound"), race("rat"))]);
        assert_eq!(
            assign(
                &catalogue,
                &unlocks,
                &assigned,
                "zap",
                "rat",
                3,
                CharmExpansion
            ),
            Err(CharmRuleError::RaceCapacityReached)
        );
        assert!(
            assign(
                &catalogue,
                &unlocks,
                &assigned,
                "gut",
                "rat",
                3,
                CharmExpansion
            )
            .is_ok()
        );
        // Another race is unaffected.
        assert!(
            assign(
                &catalogue,
                &unlocks,
                &assigned,
                "zap",
                "wolf",
                3,
                CharmExpansion
            )
            .is_ok()
        );

        let both = BTreeMap::from([(charm("wound"), race("rat")), (charm("gut"), race("rat"))]);
        assert_eq!(
            assign(
                &catalogue,
                &unlocks,
                &both,
                "bless",
                "rat",
                3,
                CharmExpansion
            ),
            Err(CharmRuleError::RaceCapacityReached)
        );
    }

    #[test]
    fn assigned_charms_of_both_categories_share_the_slot_limit() {
        use CharmSlotEntitlement::{CharmExpansion, Free, Premium};
        assert_eq!(Free.assignment_slots(), Some(2));
        assert_eq!(Premium.assignment_slots(), Some(6));
        assert_eq!(CharmExpansion.assignment_slots(), None);

        let catalogue = catalogue();
        let unlocks = BTreeMap::from([
            (charm("wound"), stage(1)),
            (charm("zap"), stage(1)),
            (charm("gut"), stage(1)),
        ]);
        let one = BTreeMap::from([(charm("wound"), race("rat"))]);
        assert!(assign(&catalogue, &unlocks, &one, "gut", "wolf", 3, Free).is_ok());
        // One major and one minor fill both free slots.
        let two = BTreeMap::from([(charm("wound"), race("rat")), (charm("gut"), race("wolf"))]);
        assert_eq!(
            assign(&catalogue, &unlocks, &two, "zap", "bear", 3, Free),
            Err(CharmRuleError::AssignmentSlotsFull)
        );
        assert!(assign(&catalogue, &unlocks, &two, "zap", "bear", 3, Premium).is_ok());
        assert!(assign(&catalogue, &unlocks, &two, "zap", "bear", 3, CharmExpansion).is_ok());
    }
}
