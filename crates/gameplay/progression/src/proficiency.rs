//! Pure proficiency track bounds and change direction rules.
use crate::revision::valid_revision;
use oteryn_gameplay_domain::domain::weapon_proficiency::{
    PROFICIENCY_PERK_LEVELS_MAX, PROFICIENCY_PERKS_PER_LEVEL_MAX,
};

/// Nonnegative PostgreSQL BIGINT storage bound, not a Mastery or progression rule.
pub const MAX_PROFICIENCY_PROGRESS: u64 = i64::MAX.unsigned_abs();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProficiencyCause {
    Training,
    PerkSelection,
    Migration,
    /// PROFICIENCY-1B §4.3: the track line only advances the track's committed revision; the
    /// change is the receipt's one modification line. Only the modification writer uses it.
    PerkModification,
}

impl ProficiencyCause {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Training => "training",
            Self::PerkSelection => "perk_selection",
            Self::Migration => "migration",
            Self::PerkModification => "perk_modification",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Some(match key {
            "training" => Self::Training,
            "perk_selection" => Self::PerkSelection,
            "migration" => Self::Migration,
            "perk_modification" => Self::PerkModification,
            _ => return None,
        })
    }
}

/// Validate plain track values before a durability adapter constructs a checked state.
pub fn valid_state(
    item_key: &str,
    definition_key: &str,
    definition_revision: &str,
    progress: u64,
    selections: &[Option<u8>],
) -> bool {
    let family_key = |key: &str, prefix: &str| {
        valid_revision(key)
            && key
                .strip_prefix(prefix)
                .is_some_and(|tail| !tail.is_empty())
    };
    !(!family_key(item_key, "oteryn:item.")
        || !family_key(definition_key, "oteryn:proficiency.")
        || !valid_revision(definition_revision)
        || progress > MAX_PROFICIENCY_PROGRESS
        || selections.is_empty()
        || selections.len() > usize::from(PROFICIENCY_PERK_LEVELS_MAX)
        || selections
            .iter()
            .any(|choice| choice.is_some_and(|index| index >= PROFICIENCY_PERKS_PER_LEVEL_MAX)))
}

/// Borrowed track inputs in item, definition, revision, progress and selection order.
pub type TrackValues<'a> = (&'a str, &'a str, &'a str, u64, &'a [Option<u8>]);

/// Check cause direction independently of storage, authority and receipts.
pub fn legal_change(
    cause: ProficiencyCause,
    before: TrackValues<'_>,
    after: TrackValues<'_>,
) -> bool {
    if before.0 != after.0 || before.1 != after.1 {
        return false;
    }
    let same_revision = before.2 == after.2;
    let same_progress = before.3 == after.3;
    match cause {
        ProficiencyCause::Training => after.3 > before.3 && same_revision && before.4 == after.4,
        ProficiencyCause::PerkSelection => {
            same_progress
                && same_revision
                && before.4.len() == after.4.len()
                && before
                    .4
                    .iter()
                    .zip(after.4)
                    .filter(|(old, new)| old != new)
                    .count()
                    == 1
        }
        ProficiencyCause::Migration => same_progress && !same_revision,
        ProficiencyCause::PerkModification => same_progress && same_revision && before.4 == after.4,
    }
}
