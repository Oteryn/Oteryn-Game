//! Preparatory WP track values and pending lines (PROFICIENCY0 §4.2, PROF-1).
//!
//! These pure models perform lexical/storage and cause-direction checks only. They do not
//! persist, load, activate effects, or grant current authority. Key prefixes do not prove
//! canonical Item identity, A12 alias resolution or a resolved definition binding: the future
//! owning content resolver must check those independently under the writer's current fence.
//! Definition-shape validation is supplied separately and does not prove that shape's provenance.
//!
//! A migration line remains a candidate: the future writer and reconcile must verify the
//! declared selection mapping from both retained definition revisions. Compatible revision
//! refresh is unresolved in the contract; training/selection never relax "definition unchanged".

#[path = "character_proficiency_codec.rs"]
pub(in crate::durability) mod read;
pub use read::{CommittedProficiencyChange, StoredProficiencyTrack};

use super::character_progression::{CharacterProgressionError, valid_revision};
use crate::domain::weapon_proficiency::{
    PROFICIENCY_PERK_LEVELS_MAX, PROFICIENCY_PERKS_PER_LEVEL_MAX, ProficiencySelectionShape,
};

type Result<T> = std::result::Result<T, CharacterProgressionError>;

/// Nonnegative PostgreSQL BIGINT storage bound, not a Mastery or progression rule.
pub const MAX_PROFICIENCY_PROGRESS: u64 = i64::MAX.unsigned_abs();

/// Checked track values, without committed receipt metadata or admission/authority evidence.
/// No row has progress 0 and unfilled choices; its definition comes from the future resolver.
/// Level/Mastery are derived elsewhere and are never stored here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableProficiencyState {
    item_key: String,
    definition_key: String,
    definition_revision: String,
    progress: u64,
    selections: Vec<Option<u8>>,
}

impl DurableProficiencyState {
    /// Bounds raw reference spelling with existing durability syntax (128 bytes) and family
    /// prefixes. This is neither canonical-key validation nor content admission. A choice is
    /// zero-based; None is an unfilled slot. Actual per-level bounds require `validate_shape`.
    pub fn new(
        item_key: impl Into<String>,
        definition_key: impl Into<String>,
        definition_revision: impl Into<String>,
        progress: u64,
        selections: Vec<Option<u8>>,
    ) -> Result<Self> {
        let item_key = item_key.into();
        let definition_key = definition_key.into();
        let definition_revision = definition_revision.into();
        let family_key = |key: &str, prefix: &str| {
            valid_revision(key)
                && key
                    .strip_prefix(prefix)
                    .is_some_and(|tail| !tail.is_empty())
        };
        if !family_key(&item_key, "oteryn:item.")
            || !family_key(&definition_key, "oteryn:proficiency.")
            || !valid_revision(&definition_revision)
            || progress > MAX_PROFICIENCY_PROGRESS
            || selections.is_empty()
            || selections.len() > usize::from(PROFICIENCY_PERK_LEVELS_MAX)
            || selections
                .iter()
                .any(|choice| choice.is_some_and(|index| index >= PROFICIENCY_PERKS_PER_LEVEL_MAX))
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        Ok(Self {
            item_key,
            definition_key,
            definition_revision,
            progress,
            selections,
        })
    }

    pub fn item_key(&self) -> &str {
        &self.item_key
    }

    pub fn definition_key(&self) -> &str {
        &self.definition_key
    }

    pub fn definition_revision(&self) -> &str {
        &self.definition_revision
    }

    /// Exact retained progress, including above u32::MAX or a subsequently lowered threshold.
    pub const fn progress(&self) -> u64 {
        self.progress
    }

    /// Includes inactive choices; no threshold or level check clears them.
    pub fn selections(&self) -> &[Option<u8>] {
        &self.selections
    }

    /// Checks every choice against independently supplied actual definition shape, including
    /// inactive levels. The caller still has to resolve and bind the shape to this definition.
    pub fn validate_shape(&self, shape: ProficiencySelectionShape) -> Result<()> {
        shape
            .validate_stored_selections(&self.selections)
            .map_err(|_| CharacterProgressionError::InvalidInput)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProficiencyCause {
    Training,
    PerkSelection,
    Migration,
}

impl ProficiencyCause {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Training => "training",
            Self::PerkSelection => "perk_selection",
            Self::Migration => "migration",
        }
    }
}

/// A structurally legal pending line, not an admitted write or a verified migration.
/// Unlock, PZ, current authority, content binding, receipt cardinality and lineage are outside
/// this model and must be checked at their owning future writer/runtime/DB boundaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProficiencyLineCandidate {
    cause: ProficiencyCause,
    before: DurableProficiencyState,
    after: DurableProficiencyState,
}

impl ProficiencyLineCandidate {
    pub fn new(
        cause: ProficiencyCause,
        before: DurableProficiencyState,
        after: DurableProficiencyState,
    ) -> Result<Self> {
        if before.item_key != after.item_key || before.definition_key != after.definition_key {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let same_revision = before.definition_revision == after.definition_revision;
        let same_progress = before.progress == after.progress;
        let legal = match cause {
            ProficiencyCause::Training => {
                after.progress > before.progress
                    && same_revision
                    && before.selections == after.selections
            }
            ProficiencyCause::PerkSelection => {
                same_progress
                    && same_revision
                    && before.selections.len() == after.selections.len()
                    && before
                        .selections
                        .iter()
                        .zip(&after.selections)
                        .filter(|(old, new)| old != new)
                        .count()
                        == 1
            }
            ProficiencyCause::Migration => same_progress && !same_revision,
        };
        if !legal {
            return Err(CharacterProgressionError::InvalidInput);
        }
        Ok(Self {
            cause,
            before,
            after,
        })
    }

    pub const fn cause(&self) -> ProficiencyCause {
        self.cause
    }

    pub const fn before(&self) -> &DurableProficiencyState {
        &self.before
    }

    pub const fn after(&self) -> &DurableProficiencyState {
        &self.after
    }

    /// True means the line cannot be admitted until the declared mapping is independently
    /// recomputed and verified. False does not establish any authority or admission evidence.
    pub const fn requires_mapping_verification(&self) -> bool {
        matches!(self.cause, ProficiencyCause::Migration)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::domain::weapon_proficiency::ProficiencyThresholdClass;
    use ProficiencyCause::{Migration, PerkSelection, Training};

    const ITEM: &str = "oteryn:item.tibia.i3295";
    const DEFINITION: &str = "oteryn:proficiency.tibia.p1";

    fn values(
        item: &str,
        definition: &str,
        revision: &str,
        progress: u64,
        choices: &[Option<u8>],
    ) -> Result<DurableProficiencyState> {
        DurableProficiencyState::new(item, definition, revision, progress, choices.to_vec())
    }

    fn line(
        cause: ProficiencyCause,
        before: DurableProficiencyState,
        after: DurableProficiencyState,
    ) -> Result<ProficiencyLineCandidate> {
        ProficiencyLineCandidate::new(cause, before, after)
    }

    fn state(progress: u64, choices: &[Option<u8>]) -> DurableProficiencyState {
        values(ITEM, DEFINITION, "definition-r1", progress, choices).expect("bounded track values")
    }

    fn invalid<T: std::fmt::Debug>(result: Result<T>) {
        assert!(
            matches!(result, Err(CharacterProgressionError::InvalidInput)),
            "{result:?}"
        );
    }

    #[test]
    fn progress_preserves_bigint_width_and_rejects_overflow() {
        for progress in [0, u64::from(u32::MAX) + 1, MAX_PROFICIENCY_PROGRESS] {
            assert_eq!(state(progress, &[None]).progress(), progress);
        }
        for progress in [MAX_PROFICIENCY_PROGRESS + 1, u64::MAX] {
            invalid(values(ITEM, DEFINITION, "r1", progress, &[None]));
        }
    }

    #[test]
    fn references_are_lexically_bounded_without_admission_claims() {
        let good = state(0, &[None]);
        assert_eq!(good.item_key(), "oteryn:item.tibia.i3295");
        assert_eq!(good.definition_key(), "oteryn:proficiency.tibia.p1");
        assert_eq!(good.definition_revision(), "definition-r1");
        for item in ["oteryn:creature.rat", "oteryn:item.", "oteryn:item.bad key"] {
            invalid(values(item, DEFINITION, "r1", 0, &[None]));
        }
        for definition in ["oteryn:item.tibia.i1", "oteryn:proficiency.", "bad"] {
            invalid(values(ITEM, definition, "r1", 0, &[None]));
        }
        for revision in ["".to_owned(), "r 1".to_owned(), "r".repeat(129)] {
            invalid(values(ITEM, DEFINITION, &revision, 0, &[None]));
        }
        // Lexical success intentionally cannot prove canonical numeric spelling or admission.
        assert!(values("oteryn:item.tibia.i01", DEFINITION, "r1", 0, &[None]).is_ok());
    }

    #[test]
    fn selections_require_storage_and_actual_definition_bounds() {
        for choices in [vec![], vec![None; 8], vec![Some(3)]] {
            invalid(values(ITEM, DEFINITION, "r1", 0, &choices));
        }
        let shape = ProficiencySelectionShape::new(ProficiencyThresholdClass::Standard, &[1, 3])
            .expect("shape");
        let inactive = state(0, &[Some(0), Some(2)]);
        assert!(inactive.validate_shape(shape).is_ok());
        assert_eq!(inactive.selections(), &[Some(0), Some(2)]);
        invalid(state(0, &[Some(1), Some(2)]).validate_shape(shape));
        invalid(state(0, &[None]).validate_shape(shape));
        assert!(
            state(0, &[None; 7])
                .validate_shape(
                    ProficiencySelectionShape::new(ProficiencyThresholdClass::Knight, &[3; 7])
                        .expect("shape")
                )
                .is_ok()
        );
    }

    #[test]
    fn training_requires_strict_progress_and_preserves_binding_and_choices() {
        let before = state(u64::from(u32::MAX) + 1, &[Some(2)]);
        let mut after = before.clone();
        after.progress += 1;
        let candidate = line(Training, before.clone(), after.clone()).expect("training");
        assert_eq!(candidate.cause().key(), "training");
        assert_eq!(candidate.before(), &before);
        assert_eq!(candidate.after(), &after);
        assert!(!candidate.requires_mapping_verification());
        invalid(line(Training, before.clone(), before.clone()));
        let mut lower = after.clone();
        lower.progress = before.progress - 1;
        invalid(line(Training, before.clone(), lower));
        let mut changed_choice = after.clone();
        changed_choice.selections[0] = None;
        invalid(line(Training, before.clone(), changed_choice));
        after.definition_revision = "definition-r2".into();
        invalid(line(Training, before, after));
    }

    #[test]
    fn selection_accepts_exactly_one_set_change_or_clear() {
        for (old, new) in [(None, Some(0)), (Some(0), Some(2)), (Some(2), None)] {
            let candidate = line(
                PerkSelection,
                state(100, &[old, None]),
                state(100, &[new, None]),
            )
            .expect("one change");
            assert_eq!(candidate.cause().key(), "perk_selection");
            assert!(!candidate.requires_mapping_verification());
        }
        for choices in [
            vec![None, None],
            vec![Some(0), Some(1)],
            vec![Some(0)],
            vec![Some(0), None, None],
        ] {
            invalid(line(
                PerkSelection,
                state(100, &[None, None]),
                state(100, &choices),
            ));
        }
        invalid(line(
            PerkSelection,
            state(100, &[None]),
            state(101, &[Some(0)]),
        ));
        let mut after = state(100, &[Some(0)]);
        after.definition_revision = "definition-r2".into();
        invalid(line(PerkSelection, state(100, &[None]), after));
    }

    #[test]
    fn migration_remains_unverified_and_preserves_progress() {
        let before = state(MAX_PROFICIENCY_PROGRESS, &[Some(2), None]);
        let mut after = state(MAX_PROFICIENCY_PROGRESS, &[None]);
        after.definition_revision = "definition-r2".into();
        let candidate = line(Migration, before.clone(), after.clone()).expect("pending mapping");
        assert_eq!(candidate.cause().key(), "migration");
        assert!(candidate.requires_mapping_verification());
        // Arbitrary changed choices above are not proof of an accepted remap/clear.
        let mut decreased = after;
        decreased.progress -= 1;
        invalid(line(Migration, before.clone(), decreased));
        invalid(line(Migration, before.clone(), before));
    }

    #[test]
    fn every_cause_rejects_rekeying_the_item_or_definition() {
        for cause in [Training, PerkSelection, Migration] {
            let before = state(100, &[None]);
            let mut after = before.clone();
            match cause {
                Training => after.progress += 1,
                PerkSelection => after.selections[0] = Some(0),
                Migration => after.definition_revision = "r2".into(),
            }
            let mut other_item = after.clone();
            other_item.item_key = "oteryn:item.tibia.i3305".into();
            invalid(line(cause, before.clone(), other_item));
            after.definition_key = "oteryn:proficiency.tibia.p2".into();
            invalid(line(cause, before, after));
        }
    }
}
