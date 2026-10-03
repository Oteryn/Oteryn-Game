//! WP track models, retained reads and fenced durable writes (PROFICIENCY0 §4.2, PROF-1).
//!
//! Models check lexical/storage bounds and cause direction. The writer resolves canonical Item
//! bindings, actual shapes and active-weapon counts from an independent semantic content source,
//! then checks current gameplay and persistence fences. Key prefixes and caller-supplied shapes
//! do not establish content provenance. The owning content adapter and ordered PROF-2 gameplay
//! hooks supply the source and command legality; this module neither activates effects nor grants
//! controller authority. Committed replay uses immutable evidence before current-policy lookup.
//!
//! Migration writes and historical reads verify explicit maps from retained definition revisions.
//! Without a retained declaration, migration history is unavailable. Compatible revision refresh
//! is unresolved in the contract; training/selection never relax "definition unchanged".

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

    pub(in crate::durability) fn from_key(key: &str) -> Option<Self> {
        Some(match key {
            "training" => Self::Training,
            "perk_selection" => Self::PerkSelection,
            "migration" => Self::Migration,
            "perk_modification" => Self::PerkModification,
            _ => return None,
        })
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
            ProficiencyCause::PerkModification => {
                same_progress && same_revision && before.selections == after.selections
            }
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
    use ProficiencyCause::{Migration, PerkModification, PerkSelection, Training};

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
        for definition in ["oteryn:item.synthetic.i1", "oteryn:proficiency.", "bad"] {
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
    fn modification_track_line_changes_no_track_value() {
        let before = state(100, &[Some(0)]);
        assert!(line(PerkModification, before.clone(), before.clone()).is_ok());
        let mut progressed = before.clone();
        progressed.progress += 1;
        invalid(line(PerkModification, before.clone(), progressed));
        let mut selected = before.clone();
        selected.selections[0] = None;
        invalid(line(PerkModification, before.clone(), selected));
        let mut migrated = before.clone();
        migrated.definition_revision = "r2".into();
        invalid(line(PerkModification, before, migrated));
    }

    #[test]
    fn every_cause_rejects_rekeying_the_item_or_definition() {
        for cause in [Training, PerkSelection, Migration, PerkModification] {
            let before = state(100, &[None]);
            let mut after = before.clone();
            match cause {
                Training => after.progress += 1,
                PerkSelection => after.selections[0] = Some(0),
                Migration => after.definition_revision = "r2".into(),
                PerkModification => {}
            }
            let mut other_item = after.clone();
            other_item.item_key = "oteryn:item.tibia.i3305".into();
            invalid(line(cause, before.clone(), other_item));
            after.definition_key = "oteryn:proficiency.tibia.p2".into();
            invalid(line(cause, before, after));
        }
    }
}

// Preparatory writer intent only: no persistence, content resolver or authority capability.
use crate::domain::{CharacterId, CharacterRevision};
use sha2::{Digest, Sha256};

/// Stable UUIDv7 identity retained until a future commit's outcome is known (§4.3).
/// CommandId derivation belongs to the owning command path, not this constructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProficiencyOccurrence(super::character_progression::ExperienceRewardOccurrence);

impl ProficiencyOccurrence {
    pub fn from_bytes(bytes: [u8; 16]) -> Result<Self> {
        super::character_progression::ExperienceRewardOccurrence::from_bytes(bytes).map(Self)
    }

    pub const fn as_bytes(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }
}

/// Complete pending track intent, without current authority or content admission.
/// The future writer still checks canonical Item resolution, actual shapes, active-content N,
/// committed before values and, for migration, the declared mapping from retained revisions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProficiencyChangeRequest {
    occurrence: ProficiencyOccurrence,
    cause: ProficiencyCause,
    lines: Vec<ProficiencyLineCandidate>,
    expected_track_revision: Option<CharacterRevision>,
    policy_digest: [u8; 32],
}

impl ProficiencyChangeRequest {
    /// Normalize line order for stable retries; duplicate tracks and mixed causes are refused.
    /// PROF-WIRE-0 §2 requires the last changed track revision for selection, independently of
    /// the global CharacterRevision consumed by every writer. Other causes have no such input.
    /// A declared digest binds intent; it proves neither the supplied content nor its mapping.
    pub fn new(
        occurrence: ProficiencyOccurrence,
        cause: ProficiencyCause,
        mut lines: Vec<ProficiencyLineCandidate>,
        expected_track_revision: Option<CharacterRevision>,
        policy_digest: [u8; 32],
    ) -> Result<Self> {
        if lines.is_empty()
            || cause == ProficiencyCause::PerkModification
            || lines.iter().any(|line| line.cause() != cause)
            || (cause == ProficiencyCause::PerkSelection
                && (lines.len() != 1 || expected_track_revision.is_none()))
            || (cause != ProficiencyCause::PerkSelection && expected_track_revision.is_some())
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        lines.sort_unstable_by(|left, right| {
            left.before().item_key().cmp(right.before().item_key())
        });
        if lines
            .windows(2)
            .any(|pair| pair[0].before().item_key() == pair[1].before().item_key())
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        Ok(Self {
            occurrence,
            cause,
            lines,
            expected_track_revision,
            policy_digest,
        })
    }

    pub const fn occurrence(&self) -> ProficiencyOccurrence {
        self.occurrence
    }

    pub const fn cause(&self) -> ProficiencyCause {
        self.cause
    }

    pub fn lines(&self) -> &[ProficiencyLineCandidate] {
        &self.lines
    }

    pub const fn expected_track_revision(&self) -> Option<CharacterRevision> {
        self.expected_track_revision
    }

    pub const fn policy_digest(&self) -> &[u8; 32] {
        &self.policy_digest
    }

    /// Versioned intent binding, as in `commit_character_build`, for replay-or-conflict.
    /// Session, connection, lease and scope generations are intentionally not inputs. Replacing
    /// a global revision after losing a write requires a new occurrence (§4.3 Losing writer).
    /// This digest does not authorize a commit or verify a receipt's provenance.
    pub fn command_binding(
        &self,
        character_id: CharacterId,
        expected_character_revision: CharacterRevision,
    ) -> [u8; 33] {
        const VERSION: u8 = 1;
        let mut semantic = b"oteryn:character-proficiency:command-binding\0".to_vec();
        semantic.push(VERSION);
        semantic.extend_from_slice(self.occurrence.as_bytes());
        semantic.extend_from_slice(character_id.as_bytes());
        semantic.extend_from_slice(&expected_character_revision.get().to_be_bytes());
        semantic.extend_from_slice(self.cause.key().as_bytes());
        semantic.push(0);
        match self.expected_track_revision {
            None => semantic.push(0),
            Some(revision) => {
                semantic.push(1);
                semantic.extend_from_slice(&revision.get().to_be_bytes());
            }
        }
        semantic.extend_from_slice(
            &u64::try_from(self.lines.len())
                .unwrap_or(u64::MAX)
                .to_be_bytes(),
        );
        for line in &self.lines {
            encode_proficiency_intent_state(line.before(), &mut semantic);
            encode_proficiency_intent_state(line.after(), &mut semantic);
        }
        semantic.extend_from_slice(&self.policy_digest);
        let mut binding = [VERSION; 33];
        binding[1..].copy_from_slice(&Sha256::digest(&semantic));
        binding
    }
}

fn encode_proficiency_intent_state(state: &DurableProficiencyState, semantic: &mut Vec<u8>) {
    // Checked reference spellings cannot contain NUL; delimiters frame all three strings.
    for text in [
        state.item_key(),
        state.definition_key(),
        state.definition_revision(),
    ] {
        semantic.extend_from_slice(text.as_bytes());
        semantic.push(0);
    }
    semantic.extend_from_slice(&state.progress().to_be_bytes());
    semantic.push(u8::try_from(state.selections().len()).unwrap_or(u8::MAX));
    for choice in state.selections() {
        match choice {
            None => semantic.push(0),
            Some(index) => {
                semantic.push(1);
                semantic.push(*index);
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod request_tests {
    use super::*;
    use ProficiencyCause::{Migration, PerkSelection, Training};

    fn id(tag: u8) -> [u8; 16] {
        [0, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag]
    }

    fn revision(value: u64) -> CharacterRevision {
        CharacterRevision::new(value).expect("nonzero revision")
    }

    fn occurrence(tag: u8) -> ProficiencyOccurrence {
        ProficiencyOccurrence::from_bytes(id(tag)).expect("UUIDv7")
    }

    fn state(
        item: u8,
        revision: &str,
        progress: u64,
        choices: &[Option<u8>],
    ) -> DurableProficiencyState {
        // Codec fixtures are synthetic and do not claim an admitted Tibia identity.
        DurableProficiencyState::new(
            format!("oteryn:item.synthetic.i{item}"),
            "oteryn:proficiency.tibia.p1",
            revision,
            progress,
            choices.to_vec(),
        )
        .expect("bounded state")
    }

    fn line(item: u8, cause: ProficiencyCause) -> ProficiencyLineCandidate {
        let before = state(item, "r1", 10, &[None, Some(0)]);
        let after = match cause {
            Training => state(item, "r1", 20, &[None, Some(0)]),
            PerkSelection => state(item, "r1", 10, &[Some(0), Some(0)]),
            Migration => state(item, "r2", 10, &[None, Some(1)]),
            ProficiencyCause::PerkModification => state(item, "r1", 10, &[None, Some(0)]),
        };
        ProficiencyLineCandidate::new(cause, before, after).expect("candidate direction")
    }

    fn request(
        cause: ProficiencyCause,
        lines: Vec<ProficiencyLineCandidate>,
    ) -> ProficiencyChangeRequest {
        ProficiencyChangeRequest::new(
            occurrence(1),
            cause,
            lines,
            (cause == PerkSelection).then(|| revision(4)),
            [2; 32],
        )
        .expect("inert intent")
    }

    fn binding(request: &ProficiencyChangeRequest) -> [u8; 33] {
        request.command_binding(
            CharacterId::from_bytes(id(2)).expect("character"),
            revision(9),
        )
    }

    #[test]
    fn occurrence_reuses_uuidv7_validation() {
        assert_eq!(occurrence(1).as_bytes(), &id(1));
        for bytes in [
            [0; 16],
            [1; 16],
            {
                let mut bytes = id(1);
                bytes[6] = 0x40;
                bytes
            },
            {
                let mut bytes = id(1);
                bytes[8] = 0x40;
                bytes
            },
        ] {
            assert!(ProficiencyOccurrence::from_bytes(bytes).is_err());
        }
    }

    #[test]
    fn request_rejects_empty_duplicate_mixed_and_wrong_selection_cardinality() {
        for (cause, lines, expected) in [
            (Training, vec![], None),
            (Training, vec![line(1, Training), line(1, Training)], None),
            (Training, vec![line(1, Training), line(2, Migration)], None),
            (Training, vec![line(1, Training)], Some(revision(4))),
            (Migration, vec![line(1, Migration)], Some(revision(4))),
            (PerkSelection, vec![line(1, PerkSelection)], None),
            (
                PerkSelection,
                vec![line(1, PerkSelection), line(2, PerkSelection)],
                Some(revision(4)),
            ),
        ] {
            assert!(matches!(
                ProficiencyChangeRequest::new(occurrence(1), cause, lines, expected, [2; 32]),
                Err(CharacterProgressionError::InvalidInput)
            ));
        }
    }

    #[test]
    fn retries_normalize_track_order_and_retain_exact_intent() {
        let _ = crate::durability::DurabilityRoot::commit_character_proficiency;
        let _ = crate::durability::DurabilityRoot::reconcile_character_proficiency;
        let _ = std::mem::size_of::<super::ProficiencyCommitOutcome>();
        let _ = std::mem::size_of::<super::ResolvedProficiencyDefinition>();
        let _ = std::mem::size_of::<super::DeclaredProficiencyMigration>();
        let _ = super::ProficiencyLevelMigration::Clear;
        let _ = super::ProficiencyWriteRefusal::PolicyMismatch;
        let _: Option<&dyn super::ProficiencyDefinitions> = None;
        for cause in [Training, Migration] {
            let forward = request(cause, vec![line(1, cause), line(2, cause)]);
            let reverse = request(cause, vec![line(2, cause), line(1, cause)]);
            assert_eq!(forward, reverse);
            assert_eq!(binding(&forward), binding(&reverse));
            assert_eq!(forward.occurrence(), occurrence(1));
            assert_eq!(forward.cause(), cause);
            assert_eq!(forward.expected_track_revision(), None);
            assert_eq!(forward.policy_digest(), &[2; 32]);
            assert_eq!(
                forward.lines()[0].before().item_key(),
                "oteryn:item.synthetic.i1"
            );
        }
        assert_eq!(
            request(PerkSelection, vec![line(1, PerkSelection)]).expected_track_revision(),
            Some(revision(4))
        );
    }

    #[test]
    fn binding_covers_header_and_independent_track_revision() {
        let original = request(PerkSelection, vec![line(1, PerkSelection)]);
        let expected = binding(&original);
        assert_eq!(expected[0], 1);
        // Independent SHA-256 vector pins v1 framing, widths, NULL tags and field order.
        assert_eq!(
            expected,
            [
                0x01, 0xbb, 0x7e, 0xce, 0x48, 0xc0, 0x95, 0x01, 0xa0, 0x35, 0x71, 0x42, 0xe5, 0xce,
                0x25, 0xb2, 0x5b, 0x50, 0x32, 0xcc, 0x8b, 0x4d, 0xf8, 0xed, 0xc5, 0x92, 0x98, 0x5c,
                0x71, 0x21, 0xd8, 0x0e, 0x4c,
            ]
        );
        for mutate in [
            |r: &mut ProficiencyChangeRequest| r.occurrence = occurrence(3),
            |r: &mut ProficiencyChangeRequest| r.expected_track_revision = Some(revision(5)),
            |r: &mut ProficiencyChangeRequest| r.policy_digest[0] ^= 1,
        ] {
            let mut changed = original.clone();
            mutate(&mut changed);
            assert_ne!(binding(&changed), expected);
        }
        assert_ne!(
            original.command_binding(
                CharacterId::from_bytes(id(3)).expect("character"),
                revision(9)
            ),
            expected
        );
        assert_ne!(
            original.command_binding(
                CharacterId::from_bytes(id(2)).expect("character"),
                revision(10)
            ),
            expected
        );
        let mut framed = Vec::new();
        encode_proficiency_intent_state(original.lines()[0].before(), &mut framed);
        assert!(framed.starts_with(b"oteryn:item.synthetic.i1\0oteryn:proficiency.tibia.p1\0r1\0"));
    }

    #[test]
    fn binding_covers_every_exact_line_field_without_narrowing_progress() {
        let original = request(Migration, vec![line(1, Migration), line(2, Migration)]);
        let expected = binding(&original);
        // Field sensitivity of the codec is independent of cause/admission validation.
        for mutate in [
            |r: &mut ProficiencyChangeRequest| r.cause = Training,
            |r: &mut ProficiencyChangeRequest| {
                r.lines.pop();
            },
            |r: &mut ProficiencyChangeRequest| r.lines[0].before.item_key.push('1'),
            |r: &mut ProficiencyChangeRequest| r.lines[0].after.item_key.push('1'),
            |r: &mut ProficiencyChangeRequest| r.lines[0].before.definition_key.push('1'),
            |r: &mut ProficiencyChangeRequest| r.lines[0].after.definition_key.push('1'),
            |r: &mut ProficiencyChangeRequest| r.lines[0].before.definition_revision.push('1'),
            |r: &mut ProficiencyChangeRequest| r.lines[0].after.definition_revision.push('1'),
            |r: &mut ProficiencyChangeRequest| r.lines[0].before.progress += 1,
            |r: &mut ProficiencyChangeRequest| r.lines[0].after.progress += 1,
            |r: &mut ProficiencyChangeRequest| r.lines[0].before.selections[0] = Some(0),
            |r: &mut ProficiencyChangeRequest| r.lines[0].after.selections[0] = Some(0),
            |r: &mut ProficiencyChangeRequest| r.lines[0].before.selections[1] = Some(1),
            |r: &mut ProficiencyChangeRequest| r.lines[0].after.selections[1] = Some(2),
            |r: &mut ProficiencyChangeRequest| r.lines[0].before.selections.push(None),
            |r: &mut ProficiencyChangeRequest| r.lines[0].after.selections.push(None),
            |r: &mut ProficiencyChangeRequest| r.lines[1].after.progress += 1,
        ] {
            let mut changed = original.clone();
            mutate(&mut changed);
            assert_ne!(binding(&changed), expected);
        }
        let mut digests = std::collections::BTreeSet::new();
        for progress in [
            0,
            u64::from(u32::MAX),
            u64::from(u32::MAX) + 1,
            MAX_PROFICIENCY_PROGRESS,
        ] {
            let candidate = ProficiencyLineCandidate::new(
                Migration,
                state(1, "r1", progress, &[None]),
                state(1, "r2", progress, &[None]),
            )
            .expect("full-width migration candidate");
            assert!(digests.insert(binding(&request(Migration, vec![candidate]))));
        }
    }
}

mod writer {
    //! Source-only PROF-1 candidate. Requires 0032; explicit migration requires retained declarations; stale compatible refresh is refused.
    //! PZ and command legality are checked by the owning ordered PROF-2 runtime before submission.
    use super::{
        CommittedProficiencyChange, ProficiencyCause, ProficiencyChangeRequest,
        ProficiencyLineCandidate, ProficiencyOccurrence, read,
    };
    use crate::domain::weapon_proficiency::ProficiencySelectionShape;
    use crate::domain::{CharacterId, CharacterRevision};
    use crate::durability::character_authority::{
        ReconciledCharacterAuthority, assert_recovery_fence,
        verify_character_proficiency_history_with_definitions,
    };
    use crate::durability::character_proficiency_modification::{
        clear_migrated_modifications, level_has_active_modification,
    };
    use crate::durability::character_progression::{
        CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence,
        numeric_u64, state_matches_root, uuid_text,
    };
    use crate::durability::db::{
        begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
    };
    use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
    use crate::durability::{DurabilityError, DurabilityRoot};
    use crate::foundation::RuntimeScopeRefV1;
    use sqlx::Row;
    type Result<T> = std::result::Result<T, CharacterProgressionError>;
    type StoredResult<T> = std::result::Result<T, DurabilityError>;

    /// Independently resolved active Item binding and actual perk counts/threshold class.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ResolvedProficiencyDefinition {
        pub canonical_item_key: String,
        pub definition_key: String,
        pub definition_revision: String,
        pub shape: ProficiencySelectionShape,
    }

    /// Semantic content policy, as BuildFormula; this is not a session authority capability.
    /// The owning content path supplies one immutable generation, resolves A12 aliases, includes
    /// only promoted bindings, and derives N by enumerating that generation's active weapons.
    /// Lookups query this independent source; they must never echo a request's supplied values.
    pub trait ProficiencyDefinitions: Send + Sync {
        fn content_revision(&self) -> &str;
        fn digest(&self) -> [u8; 32];
        fn active_weapon_count(&self) -> usize;
        fn resolve_weapon(&self, item_key: &str) -> Option<ResolvedProficiencyDefinition>;
        /// Lookup by immutable receipt context and both retained revisions. Return independent
        /// authoring data for an explicitly incompatible migration, never an after-state or boolean.
        fn retained_migration(
            &self,
            _content_revision: &str,
            _digest: &[u8; 32],
            _item_key: &str,
            _definition_key: &str,
            _old_revision: &str,
            _new_revision: &str,
        ) -> Option<DeclaredProficiencyMigration> {
            None
        }
        /// Item class plus actual counts from this retained definition, not caller-supplied shape.
        fn retained_definition(
            &self,
            item_key: &str,
            definition_key: &str,
            revision: &str,
        ) -> Option<ProficiencySelectionShape>;
    }

    /// Every old/new level is explicitly accounted for; added/removed levels require Clear.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum ProficiencyLevelMigration {
        Keep,
        Clear,
        Remap(Vec<Option<u8>>),
    }
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct DeclaredProficiencyMigration {
        pub old_shape: ProficiencySelectionShape,
        pub new_shape: ProficiencySelectionShape,
        pub levels: Vec<ProficiencyLevelMigration>,
    }
    impl DeclaredProficiencyMigration {
        /// Recompute from retained choices, validating every declared source/destination index,
        /// including currently unselected perks. Never narrow the persisted progress value.
        pub fn apply(&self, choices: &[Option<u8>]) -> StoredResult<Vec<Option<u8>>> {
            let invalid = || DurabilityError::InvalidStoredState;
            self.old_shape
                .validate_stored_selections(choices)
                .map_err(|_| invalid())?;
            let old = usize::from(self.old_shape.track_shape().perk_levels());
            let new = usize::from(self.new_shape.track_shape().perk_levels());
            if self.levels.len() != old.max(new) {
                return Err(invalid());
            }
            let valid = |shape: ProficiencySelectionShape, level, index| {
                let mut probe = vec![None; usize::from(shape.track_shape().perk_levels())];
                probe[level] = Some(index);
                shape.validate_stored_selections(&probe).is_ok()
            };
            let mut result = vec![None; new];
            for (level, rule) in self.levels.iter().enumerate() {
                if level >= old || level >= new {
                    if !matches!(rule, ProficiencyLevelMigration::Clear) {
                        return Err(invalid());
                    }
                    continue;
                }
                let count = (0..3)
                    .take_while(|index| valid(self.old_shape, level, *index))
                    .count();
                let choice = match rule {
                    ProficiencyLevelMigration::Clear => None,
                    ProficiencyLevelMigration::Keep => {
                        if (0..count).any(|index| !valid(self.new_shape, level, index as u8)) {
                            return Err(invalid());
                        }
                        choices[level]
                    }
                    ProficiencyLevelMigration::Remap(map) => {
                        if map.len() != count
                            || map
                                .iter()
                                .flatten()
                                .any(|index| !valid(self.new_shape, level, *index))
                        {
                            return Err(invalid());
                        }
                        choices[level].and_then(|index| map[usize::from(index)])
                    }
                };
                result[level] = choice;
            }
            self.new_shape
                .validate_stored_selections(&result)
                .map_err(|_| invalid())?;
            Ok(result)
        }
    }
    pub(in crate::durability) fn verify_migration(
        source: Option<&dyn ProficiencyDefinitions>,
        content_revision: &str,
        digest: &[u8; 32],
        line: &ProficiencyLineCandidate,
    ) -> StoredResult<DeclaredProficiencyMigration> {
        let declaration = source
            .and_then(|source| {
                source.retained_migration(
                    content_revision,
                    digest,
                    line.before().item_key(),
                    line.before().definition_key(),
                    line.before().definition_revision(),
                    line.after().definition_revision(),
                )
            })
            .ok_or(DurabilityError::Unavailable)?;
        if declaration.apply(line.before().selections())? != line.after().selections() {
            return Err(DurabilityError::InvalidStoredState);
        }
        line.before()
            .validate_shape(declaration.old_shape)
            .and_then(|()| line.after().validate_shape(declaration.new_shape))
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        Ok(declaration)
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ProficiencyWriteRefusal {
        MigrationDeclarationUnavailable,
        DefinitionRevisionMismatch,
        UnknownTrackDefinition,
        TrackStateMismatch,
        StaleTrackRevision,
        PolicyMismatch,
        LineCountExceeded,
        /// PROFICIENCY-1B §10: the level's selected perk is replaced by an active modification.
        ModifiedLevel,
    }
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum ProficiencyCommitOutcome {
        Committed(CommittedProficiencyChange),
        AlreadyCommitted(CommittedProficiencyChange),
        Refused(ProficiencyWriteRefusal),
    }

    impl DurabilityRoot {
        /// One global revision for every changed track, or nothing. The owner serializes every
        /// revision writer in one session queue; a losing training writer retains its deltas and
        /// retries against newly committed values with a new occurrence (§4.3).
        /// This candidate refuses stale compatible definition revisions. It is not the
        /// complete 0032 gate. Runtime PZ checks are PROF-2's responsibility; no caller bool is used.
        /// Arc keeps the immutable semantic source alive through the asynchronous pass, while
        /// permitting all source queries to occur after exact replay lookup. It grants no authority.
        /// Runtime callers reach it only through a
        /// [`RevisionSlot`](crate::durability::character_revision_sequencer::RevisionSlot)
        /// (CHAR-REV-SEQ-1).
        pub async fn commit_character_proficiency(
            &self,
            authority: &ReconciledCharacterAuthority<'_, '_>,
            node: &NodeIncarnationProof,
            fence: CurrentCharacterGameplayFence,
            request: ProficiencyChangeRequest,
            policy: std::sync::Arc<dyn ProficiencyDefinitions>,
        ) -> Result<ProficiencyCommitOutcome> {
            if fence.character_lease_generation == 0
                || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
            {
                return Err(CharacterProgressionError::InvalidInput);
            }
            let binding =
                request.command_binding(fence.character_id, fence.expected_character_revision);
            let recovery = authority
                .record_for(self)
                .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
            let node = node.clone();
            self.try_issue_semantic_pass()?.run(move |holder, deadline| { Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(\
                'oteryn:character-proficiency:' || encode($1,'hex'),0))")
                .bind(request.occurrence().as_bytes().as_slice()).execute(&mut *tx).await?;
            if let Some(row) = load_receipt(&mut tx, request.occurrence()).await? {
                if row.try_get::<Vec<u8>,_>("command_binding")? != binding {
                    return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
                }
                let committed = decode_receipt(&mut tx, &row, &request, Some(policy.as_ref())).await?;
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(Ok(ProficiencyCommitOutcome::AlreadyCommitted(committed)));
            }
            let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                Ok(root) => root, Err(error) => return Ok(Err(error)),
            };
            let character = fence.character_id.as_bytes().as_slice();
            let state = sqlx::query("SELECT character_revision::text,profile_revision,ruleset_revision,\
                content_revision FROM game_character_progression_state \
                WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(character).fetch_optional(&mut *tx).await?;
            let Some(state) = state else { return Ok(Err(CharacterProgressionError::MissingProgressionState)); };
            if numeric_u64(&state,"character_revision")? != root.revision {
                return Err(DurabilityError::InvalidStoredState);
            }
            if !state_matches_root(&state,&root) {
                return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
            }
            // Current semantic source is consulted only for a previously unseen occurrence.
            let digest = policy.digest();
            if policy.content_revision() != state.try_get::<String,_>("content_revision")? || request.policy_digest() != &digest {
                return Ok(Ok(ProficiencyCommitOutcome::Refused(ProficiencyWriteRefusal::PolicyMismatch)));
            }
            let pending: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_pending_respawns \
                WHERE character_id=encode($1,'hex')::uuid)").bind(character).fetch_one(&mut *tx).await?;
            if pending { return Ok(Err(CharacterProgressionError::RespawnPending)); }
            if request.lines().len() > policy.active_weapon_count() {
                return Ok(Ok(ProficiencyCommitOutcome::Refused(ProficiencyWriteRefusal::LineCountExceeded)));
            }
            for line in request.lines() {
                let target = if request.cause() == ProficiencyCause::Migration { line.after() } else { line.before() };
                let current = policy.resolve_weapon(target.item_key());
                let migration = if request.cause() == ProficiencyCause::Migration {
                    match verify_migration(Some(policy.as_ref()), policy.content_revision(), &digest, line) {
                        Ok(declaration) => Some(declaration),
                        Err(DurabilityError::Unavailable) => return Ok(Ok(ProficiencyCommitOutcome::Refused(
                            ProficiencyWriteRefusal::MigrationDeclarationUnavailable))),
                        Err(DurabilityError::InvalidStoredState) => return Ok(Err(CharacterProgressionError::InvalidInput)),
                        Err(error) => return Err(error),
                    }
                } else { None };
                let retained = migration.as_ref().map(|declaration| declaration.old_shape).or_else(||
                    policy.retained_definition(line.before().item_key(), line.before().definition_key(),
                        line.before().definition_revision()));
                let (Some(current),Some(retained)) = (current,retained) else {
                    return Ok(Ok(ProficiencyCommitOutcome::Refused(ProficiencyWriteRefusal::UnknownTrackDefinition)));
                };
                if current.canonical_item_key != target.item_key()
                    || current.definition_key != target.definition_key()
                    || current.definition_revision != target.definition_revision() {
                    return Ok(Ok(ProficiencyCommitOutcome::Refused(ProficiencyWriteRefusal::DefinitionRevisionMismatch)));
                }
                if current.shape != migration.as_ref().map_or(retained, |declaration| declaration.new_shape) { return Ok(Err(CharacterProgressionError::ProgressionContextMismatch)); }
                if let Err(error) = line.before().validate_shape(retained)
                    .and_then(|()| line.after().validate_shape(current.shape)) {
                    return Ok(Err(error));
                }
                let stored = sqlx::query("SELECT *, committed_character_revision::text AS track_revision,\
                    coalesce(array_ndims(selections)=1 AND array_lower(selections,1)=1,false) AS canonical \
                    FROM game_character_proficiency WHERE character_id=encode($1,'hex')::uuid \
                    AND item_key=$2 FOR UPDATE").bind(character).bind(line.before().item_key())
                    .fetch_optional(&mut *tx).await?;
                if let Some(row) = &stored {
                    if read::decode_state(row,"")? != *line.before() {
                        return Ok(Ok(ProficiencyCommitOutcome::Refused(ProficiencyWriteRefusal::TrackStateMismatch)));
                    }
                } else if line.before().progress() != 0 || line.before().selections().iter().any(Option::is_some) {
                    return Ok(Ok(ProficiencyCommitOutcome::Refused(ProficiencyWriteRefusal::TrackStateMismatch)));
                }
                match request.cause() {
                    ProficiencyCause::Training => {
                        let cap = u64::from(current.shape.track_shape().final_progress());
                        if line.after().progress() > line.before().progress().max(cap) {
                            return Ok(Err(CharacterProgressionError::InvalidInput));
                        }
                    }
                    ProficiencyCause::PerkSelection => {
                        let Some(row) = stored else {
                            return Ok(Ok(ProficiencyCommitOutcome::Refused(ProficiencyWriteRefusal::UnknownTrackDefinition)));
                        };
                        let expected = request.expected_track_revision().ok_or(DurabilityError::InvalidStoredState)?;
                        if numeric_u64(&row,"track_revision")? != expected.get() {
                            return Ok(Ok(ProficiencyCommitOutcome::Refused(ProficiencyWriteRefusal::StaleTrackRevision)));
                        }
                        let changed = line.before().selections().iter().zip(line.after().selections())
                            .position(|(before,after)| before != after).ok_or(DurabilityError::InvalidStoredState)?;
                        let threshold = current.shape.track_shape().class().thresholds()[changed];
                        if line.before().progress() < u64::from(threshold) {
                            return Ok(Err(CharacterProgressionError::InvalidInput));
                        }
                        if level_has_active_modification(&mut tx, fence.character_id, line.before(),
                            current.shape, changed).await? {
                            return Ok(Ok(ProficiencyCommitOutcome::Refused(ProficiencyWriteRefusal::ModifiedLevel)));
                        }
                    }
                    ProficiencyCause::Migration => {},
                    ProficiencyCause::PerkModification => return Ok(Err(CharacterProgressionError::InvalidInput)),
                }
            }
            let next = root.revision.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?;
            let (original,committed) = (root.revision.to_string(),next.to_string());
            for sql in [
                "UPDATE game_character_roots SET character_revision=$2::text::numeric(20,0) \
                 WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric(20,0)",
                "UPDATE game_character_progression_state SET character_revision=$2::text::numeric(20,0) \
                 WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric(20,0)",
            ] {
                if sqlx::query(sql).bind(character).bind(&committed).bind(&original)
                    .execute(&mut *tx).await?.rows_affected() != 1 { return Err(DurabilityError::InvalidStoredState); }
            }
            insert_header(&mut tx,fence.character_id,&request,&binding,&original,&committed).await?;
            for line in request.lines() {
                insert_track(&mut tx,fence.character_id,request.occurrence(),&committed,line).await?;
                if request.cause() == ProficiencyCause::Migration {
                    clear_migrated_modifications(&mut tx,fence.character_id,request.occurrence().as_bytes(),
                        &committed,line).await?;
                }
            }
            let result = CommittedProficiencyChange { character_id:fence.character_id,
                occurrence_id:*request.occurrence().as_bytes(), original_character_revision:fence.expected_character_revision,
                committed_character_revision:CharacterRevision::new(next).map_err(|_|DurabilityError::InvalidStoredState)?,
                cause:request.cause(),command_binding:binding.to_vec(),policy_digest:digest,lines:request.lines().to_vec() };
            commit_semantic_transaction(tx,deadline).await?;
            Ok(Ok(ProficiencyCommitOutcome::Committed(result)))
        }) }).await?
        }

        /// Reconcile one supplied pending intent, including its original global/track revision and
        /// digest. Historical evidence never requires a current gameplay session or current policy,
        /// and never restores controller authority. This wrapper supplies no retained source and
        /// therefore returns Unavailable for migration history; use the source-aware sibling below.
        pub async fn reconcile_character_proficiency(
            &self,
            authority: &ReconciledCharacterAuthority<'_, '_>,
            character: CharacterId,
            expected_revision: CharacterRevision,
            request: ProficiencyChangeRequest,
        ) -> Result<Option<CommittedProficiencyChange>> {
            self.reconcile_character_proficiency_with_definitions(
                authority,
                character,
                expected_revision,
                request,
                None,
            )
            .await
        }
        /// Historical migrations require an independently retained declaration for every line.
        /// Its immutable receipt context selects the declaration without querying current policy.
        pub async fn reconcile_character_proficiency_with_definitions(
            &self,
            authority: &ReconciledCharacterAuthority<'_, '_>,
            character: CharacterId,
            expected_revision: CharacterRevision,
            request: ProficiencyChangeRequest,
            definitions: Option<std::sync::Arc<dyn ProficiencyDefinitions>>,
        ) -> Result<Option<CommittedProficiencyChange>> {
            let binding = request.command_binding(character, expected_revision);
            let recovery = authority
                .record_for(self)
                .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
            self.try_issue_semantic_pass()?
                .run(move |holder, deadline| {
                    Box::pin(async move {
                        let mut tx = begin_semantic_transaction(holder, deadline).await?;
                        assert_recovery_fence(&mut tx, &recovery).await?;
                        let result = if let Some(row) =
                            load_receipt(&mut tx, request.occurrence()).await?
                        {
                            if row.try_get::<Vec<u8>, _>("command_binding")? != binding {
                                return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
                            }
                            Some(
                                decode_receipt(&mut tx, &row, &request, definitions.as_deref())
                                    .await?,
                            )
                        } else {
                            None
                        };
                        commit_semantic_transaction(tx, deadline).await?;
                        Ok(Ok(result))
                    })
                })
                .await?
        }
    }

    async fn insert_header(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        character: CharacterId,
        request: &ProficiencyChangeRequest,
        binding: &[u8; 33],
        original: &str,
        committed: &str,
    ) -> StoredResult<()> {
        let inserted=sqlx::query("INSERT INTO game_character_proficiency_receipts(\
        proficiency_occurrence_id,command_binding,policy_digest,character_id,original_character_revision,\
        committed_character_revision,cause,level_before,level_after,experience_before,experience_after,\
        profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,\
        declaration_revision,policy_revision,reward_revision,committed_at) \
        SELECT encode($1,'hex')::uuid,$2,$3,character_id,$5::text::numeric(20,0),$6::text::numeric(20,0),\
        $7,level,level,total_experience,total_experience,profile_revision,ruleset_revision,content_revision,\
        simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,\
        floor(extract(epoch FROM statement_timestamp())*1000)::bigint \
        FROM game_character_progression_state WHERE character_id=encode($4,'hex')::uuid")
        .bind(request.occurrence().as_bytes().as_slice()).bind(binding.as_slice()).bind(request.policy_digest().as_slice())
        .bind(character.as_bytes().as_slice()).bind(original).bind(committed).bind(request.cause().key())
        .execute(&mut **tx).await?;
        if inserted.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(())
    }

    async fn insert_track(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        character: CharacterId,
        occurrence: ProficiencyOccurrence,
        revision: &str,
        line: &ProficiencyLineCandidate,
    ) -> StoredResult<()> {
        let before = line.before();
        let after = line.after();
        let choices = |state: &super::DurableProficiencyState| {
            state
                .selections()
                .iter()
                .map(|choice| choice.map(i16::from))
                .collect::<Vec<_>>()
        };
        let progress =
            |value| i64::try_from(value).map_err(|_| DurabilityError::InvalidStoredState);
        let inserted=sqlx::query("INSERT INTO game_character_proficiency_receipt_lines(\
        proficiency_occurrence_id,character_id,committed_character_revision,cause,item_key,\
        definition_key_before,definition_revision_before,progress_before,selections_before,\
        definition_key_after,definition_revision_after,progress_after,selections_after) \
        VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric(20,0),$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)")
        .bind(occurrence.as_bytes().as_slice()).bind(character.as_bytes().as_slice()).bind(revision)
        .bind(line.cause().key()).bind(before.item_key()).bind(before.definition_key()).bind(before.definition_revision())
        .bind(progress(before.progress())?).bind(choices(before)).bind(after.definition_key()).bind(after.definition_revision())
        .bind(progress(after.progress())?).bind(choices(after)).execute(&mut **tx).await?;
        if inserted.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
        let updated=sqlx::query("INSERT INTO game_character_proficiency(character_id,item_key,definition_key,\
        definition_revision,progress,selections,committed_character_revision,last_proficiency_occurrence_id) \
        VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5,$6,$7::text::numeric(20,0),encode($8,'hex')::uuid) \
        ON CONFLICT(character_id,item_key) DO UPDATE SET (definition_key,definition_revision,progress,selections,\
        committed_character_revision,last_proficiency_occurrence_id)=(EXCLUDED.definition_key,EXCLUDED.definition_revision,\
        EXCLUDED.progress,EXCLUDED.selections,EXCLUDED.committed_character_revision,EXCLUDED.last_proficiency_occurrence_id)")
        .bind(character.as_bytes().as_slice()).bind(after.item_key()).bind(after.definition_key()).bind(after.definition_revision())
        .bind(progress(after.progress())?).bind(choices(after)).bind(revision).bind(occurrence.as_bytes().as_slice())
        .execute(&mut **tx).await?;
        if updated.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(())
    }

    async fn load_receipt(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        occurrence: ProficiencyOccurrence,
    ) -> StoredResult<Option<sqlx::postgres::PgRow>> {
        Ok(sqlx::query("SELECT character_id::text,proficiency_occurrence_id::text,original_character_revision::text,\
        committed_character_revision::text,cause,command_binding,policy_digest FROM game_character_proficiency_receipts \
        WHERE proficiency_occurrence_id=encode($1,'hex')::uuid")
        .bind(occurrence.as_bytes().as_slice()).fetch_optional(&mut **tx).await?)
    }

    async fn decode_receipt(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        row: &sqlx::postgres::PgRow,
        pending: &ProficiencyChangeRequest,
        definitions: Option<&dyn ProficiencyDefinitions>,
    ) -> StoredResult<CommittedProficiencyChange> {
        let character = CharacterId::from_bytes(uuid_text(row.try_get("character_id")?)?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let occurrence = ProficiencyOccurrence::from_bytes(uuid_text(
            row.try_get("proficiency_occurrence_id")?,
        )?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
        let original = CharacterRevision::new(numeric_u64(row, "original_character_revision")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let committed = CharacterRevision::new(numeric_u64(row, "committed_character_revision")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        if original.get().checked_add(1) != Some(committed.get()) {
            return Err(DurabilityError::InvalidStoredState);
        }
        // Stable multi-relation retained read; never use immutable receipt identities as current authority.
        if sqlx::query(
        "SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR SHARE",
    )
    .bind(character.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?
    .is_none()
    {
        return Err(DurabilityError::InvalidStoredState);
    }
        verify_character_proficiency_history_with_definitions(tx, character, definitions).await?;
        let rows=sqlx::query("SELECT *,coalesce(array_ndims(selections_before)=1 AND array_lower(selections_before,1)=1,false) AS canonical_before,\
        coalesce(array_ndims(selections_after)=1 AND array_lower(selections_after,1)=1,false) AS canonical_after \
        FROM game_character_proficiency_receipt_lines WHERE proficiency_occurrence_id=encode($1,'hex')::uuid ORDER BY item_key")
        .bind(occurrence.as_bytes().as_slice()).fetch_all(&mut **tx).await?;
        let lines = rows
            .iter()
            .map(read::decode_line)
            .collect::<StoredResult<Vec<_>>>()?;
        let cause = match row.try_get::<&str, _>("cause")? {
            "training" => ProficiencyCause::Training,
            "perk_selection" => ProficiencyCause::PerkSelection,
            "migration" => ProficiencyCause::Migration,
            // The modification writer owns its receipts and their replay.
            _ => return Err(DurabilityError::InvalidStoredState),
        };
        let digest: [u8; 32] = row
            .try_get::<Vec<u8>, _>("policy_digest")?
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let binding: Vec<u8> = row.try_get("command_binding")?;
        let retained = ProficiencyChangeRequest::new(
            occurrence,
            cause,
            lines.clone(),
            pending.expected_track_revision(),
            digest,
        )
        .map_err(|_| DurabilityError::InvalidStoredState)?;
        if retained.command_binding(character, original).as_slice() != binding {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(CommittedProficiencyChange {
            character_id: character,
            occurrence_id: *occurrence.as_bytes(),
            original_character_revision: original,
            committed_character_revision: committed,
            cause,
            command_binding: binding,
            policy_digest: digest,
            lines,
        })
    }

    #[cfg(test)]
    #[allow(clippy::expect_used)]
    mod migration_tests {
        use super::*;
        use crate::domain::weapon_proficiency::ProficiencyThresholdClass::Standard;
        fn declaration(
            old: &[u8],
            new: &[u8],
            levels: Vec<ProficiencyLevelMigration>,
        ) -> DeclaredProficiencyMigration {
            DeclaredProficiencyMigration {
                old_shape: ProficiencySelectionShape::new(Standard, old)
                    .expect("valid mapping fixture"),
                new_shape: ProficiencySelectionShape::new(Standard, new)
                    .expect("valid mapping fixture"),
                levels,
            }
        }
        #[test]
        fn declared_choices_keep_clear_remap_without_unlock_projection() {
            use ProficiencyLevelMigration::{Clear, Keep, Remap};
            let migration = declaration(
                &[3, 2, 1],
                &[3, 2, 1],
                vec![Remap(vec![Some(2), None, Some(0)]), Keep, Clear],
            );
            assert_eq!(
                migration
                    .apply(&[Some(0), Some(1), Some(0)])
                    .expect("valid mapping fixture"),
                vec![Some(2), Some(1), None]
            );
            assert_eq!(
                migration
                    .apply(&[None, None, Some(0)])
                    .expect("valid mapping fixture"),
                vec![None, None, None]
            );
            assert_eq!(
                migration
                    .apply(&[Some(1), Some(0), None])
                    .expect("valid mapping fixture"),
                vec![None, Some(0), None]
            );
            // Keep/remap retain inactive choices; migration does not project against progress.
            let root = DurabilityRoot::reconcile_character_recovery_with_proficiency_definitions;
            let _ = (
                root,
                DurabilityRoot::open_character_authority_with_proficiency_definitions,
                DurabilityRoot::read_character_proficiency_with_definitions,
                DurabilityRoot::reconcile_character_proficiency_with_definitions,
            );
        }
        #[test]
        fn every_added_removed_level_requires_explicit_clear() {
            use ProficiencyLevelMigration::{Clear, Keep};
            assert_eq!(
                declaration(&[2, 1], &[2], vec![Keep, Clear])
                    .apply(&[Some(1), Some(0)])
                    .expect("valid mapping fixture"),
                vec![Some(1)]
            );
            assert_eq!(
                declaration(&[2], &[2, 1], vec![Keep, Clear])
                    .apply(&[Some(1)])
                    .expect("valid mapping fixture"),
                vec![Some(1), None]
            );
            for migration in [
                declaration(&[2], &[2, 1], vec![Keep]),
                declaration(&[2], &[2, 1], vec![Keep, Keep]),
                declaration(&[2, 1], &[2], vec![Keep, Keep]),
            ] {
                let choices =
                    vec![None; usize::from(migration.old_shape.track_shape().perk_levels())];
                assert!(matches!(
                    migration.apply(&choices),
                    Err(DurabilityError::InvalidStoredState)
                ));
            }
        }
        #[test]
        fn invalid_unselected_map_entries_and_keep_are_rejected() {
            use ProficiencyLevelMigration::{Keep, Remap};
            for rule in [
                Keep,
                Remap(vec![Some(0)]),
                Remap(vec![Some(0), Some(1)]),
                Remap(vec![Some(0), None, None]),
            ] {
                assert!(matches!(
                    declaration(&[2], &[1], vec![rule]).apply(&[None]),
                    Err(DurabilityError::InvalidStoredState)
                ));
            }
            assert!(matches!(
                declaration(&[2], &[2], vec![Keep]).apply(&[Some(2)]),
                Err(DurabilityError::InvalidStoredState)
            ));
        }
    }
}
pub use writer::{
    DeclaredProficiencyMigration, ProficiencyCommitOutcome, ProficiencyDefinitions,
    ProficiencyLevelMigration, ProficiencyWriteRefusal, ResolvedProficiencyDefinition,
};
