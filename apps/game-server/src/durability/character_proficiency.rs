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
        DurableProficiencyState::new(
            format!("oteryn:item.tibia.i{item}"),
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
                "oteryn:item.tibia.i1"
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
                0x01, 0x16, 0x44, 0xde, 0xef, 0xe4, 0x7d, 0x11, 0xd5, 0x22, 0x74, 0xeb, 0xf2, 0xd9,
                0x1d, 0x11, 0x61, 0x87, 0xc3, 0xaa, 0xb0, 0xc9, 0x7e, 0xe3, 0x77, 0x25, 0xa0, 0x09,
                0x45, 0x33, 0x82, 0xfb, 0x27,
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
        assert!(framed.starts_with(b"oteryn:item.tibia.i1\0oteryn:proficiency.tibia.p1\0r1\0"));
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
