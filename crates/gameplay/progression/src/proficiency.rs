//! Pure proficiency track bounds and change direction rules.
use crate::revision::valid_revision;
use oteryn_gameplay_domain::domain::weapon_proficiency::{
    PROFICIENCY_PERK_LEVELS_MAX, PROFICIENCY_PERKS_PER_LEVEL_MAX, ProficiencySelectionShape,
};
use sha2::{Digest, Sha256};

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

/// Proficiency values exceed lexical, storage or definition shape bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProficiencyInputError;

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
    ) -> Result<Self, ProficiencyInputError> {
        let item_key = item_key.into();
        let definition_key = definition_key.into();
        let definition_revision = definition_revision.into();
        if !valid_state(
            &item_key,
            &definition_key,
            &definition_revision,
            progress,
            &selections,
        ) {
            return Err(ProficiencyInputError);
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
    pub fn validate_shape(
        &self,
        shape: ProficiencySelectionShape,
    ) -> Result<(), ProficiencyInputError> {
        shape
            .validate_stored_selections(&self.selections)
            .map_err(|_| ProficiencyInputError)
    }
}

/// Stable UUIDv7 identity retained until a future commit's outcome is known (§4.3).
/// CommandId derivation belongs to the owning command path, not this constructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProficiencyOccurrence([u8; 16]);

impl ProficiencyOccurrence {
    pub fn from_bytes(bytes: [u8; 16]) -> Result<Self, ProficiencyInputError> {
        if bytes[6] >> 4 != 7 || bytes[8] & 0xc0 != 0x80 {
            return Err(ProficiencyInputError);
        }
        Ok(Self(bytes))
    }

    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

/// Normalize line order for stable retries; duplicate tracks and mixed causes are refused.
/// The adapter supplies only each line's cause and item key; values and storage stay with it.
pub fn normalize_request_lines<T>(
    cause: ProficiencyCause,
    mut lines: Vec<T>,
    has_expected_track_revision: bool,
    line_values: impl Fn(&T) -> (ProficiencyCause, &str),
) -> Result<Vec<T>, ProficiencyInputError> {
    if lines.is_empty()
        || cause == ProficiencyCause::PerkModification
        || lines.iter().any(|line| line_values(line).0 != cause)
        || (cause == ProficiencyCause::PerkSelection
            && (lines.len() != 1 || !has_expected_track_revision))
        || (cause != ProficiencyCause::PerkSelection && has_expected_track_revision)
    {
        return Err(ProficiencyInputError);
    }
    lines.sort_unstable_by(|left, right| line_values(left).1.cmp(line_values(right).1));
    if lines
        .windows(2)
        .any(|pair| line_values(&pair[0]).1 == line_values(&pair[1]).1)
    {
        return Err(ProficiencyInputError);
    }
    Ok(lines)
}

/// Versioned intent binding, as in `commit_character_build`, for replay-or-conflict.
/// Session, connection, lease and scope generations are intentionally not inputs. Replacing
/// a global revision after losing a write requires a new occurrence (§4.3 Losing writer).
/// This digest does not authorize a commit or verify a receipt's provenance.
pub fn command_binding<'a>(
    occurrence: &[u8; 16],
    character_id: &[u8; 16],
    expected_character_revision: u64,
    cause: ProficiencyCause,
    expected_track_revision: Option<u64>,
    lines: impl ExactSizeIterator<Item = (TrackValues<'a>, TrackValues<'a>)>,
    policy_digest: &[u8; 32],
) -> [u8; 33] {
    const VERSION: u8 = 1;
    let mut semantic = b"oteryn:character-proficiency:command-binding\0".to_vec();
    semantic.push(VERSION);
    semantic.extend_from_slice(occurrence);
    semantic.extend_from_slice(character_id);
    semantic.extend_from_slice(&expected_character_revision.to_be_bytes());
    semantic.extend_from_slice(cause.key().as_bytes());
    semantic.push(0);
    match expected_track_revision {
        None => semantic.push(0),
        Some(revision) => {
            semantic.push(1);
            semantic.extend_from_slice(&revision.to_be_bytes());
        }
    }
    semantic.extend_from_slice(&u64::try_from(lines.len()).unwrap_or(u64::MAX).to_be_bytes());
    for (before, after) in lines {
        encode_proficiency_intent_state(before, &mut semantic);
        encode_proficiency_intent_state(after, &mut semantic);
    }
    semantic.extend_from_slice(policy_digest);
    let mut binding = [VERSION; 33];
    binding[1..].copy_from_slice(&Sha256::digest(&semantic));
    binding
}

/// Frame one checked track with full-width progress and explicit empty-choice tags.
pub fn encode_proficiency_intent_state(state: TrackValues<'_>, semantic: &mut Vec<u8>) {
    // Checked reference spellings cannot contain NUL; delimiters frame all three strings.
    for text in [state.0, state.1, state.2] {
        semantic.extend_from_slice(text.as_bytes());
        semantic.push(0);
    }
    semantic.extend_from_slice(&state.3.to_be_bytes());
    semantic.push(u8::try_from(state.4.len()).unwrap_or(u8::MAX));
    for choice in state.4 {
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
mod tests {
    use super::*;
    use oteryn_gameplay_domain::domain::weapon_proficiency::ProficiencyThresholdClass;
    const ITEM: &str = "oteryn:item.tibia.i3295";
    const DEFINITION: &str = "oteryn:proficiency.tibia.p1";
    fn values(
        item: &str,
        definition: &str,
        revision: &str,
        progress: u64,
        choices: &[Option<u8>],
    ) -> Result<DurableProficiencyState, ProficiencyInputError> {
        DurableProficiencyState::new(item, definition, revision, progress, choices.to_vec())
    }
    fn state(progress: u64, choices: &[Option<u8>]) -> DurableProficiencyState {
        values(ITEM, DEFINITION, "definition-r1", progress, choices).expect("bounded track values")
    }
    fn invalid<T: std::fmt::Debug>(result: Result<T, ProficiencyInputError>) {
        assert!(matches!(result, Err(ProficiencyInputError)), "{result:?}");
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
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod occurrence_tests {
    use super::*;
    fn id(tag: u8) -> [u8; 16] {
        [0, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag]
    }
    fn occurrence(tag: u8) -> ProficiencyOccurrence {
        ProficiencyOccurrence::from_bytes(id(tag)).expect("UUIDv7")
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
}
