//! Bounded adapter for the protected CW2-B4 Ability/Effect/Formula evidence batch.
//!
//! The adapter admits only the exact protected evidence bytes. It produces candidate-only import
//! data for a caller-owned [`ProjectDraft`](super::ProjectDraft); it never creates native content
//! identities, project identities, executable formulas, or runtime behavior.

use super::{
    CandidateDisposition, CandidateValue, ImportBatch, ImportCandidate, ImportCandidateFamily,
    ImportCandidateOperation, NamedCandidateField, ReimportDecision, ReimportFieldState,
    world_project_sha256,
};
use serde_json::{Map, Value};
use std::fmt::{self, Display, Formatter};

pub const PROTECTED_CW2_B4_EVIDENCE_MAX_BYTES: usize = 7_428;
pub const PROTECTED_CW2_B4_EVIDENCE_SHA256: &str =
    "97fbfe027f93834bfaef365e4271dbb56b479ba29528e3f00a1b046aae0a7491";
pub const PROTECTED_CW2_B4_PRODUCT_SHA256: &str =
    "56cef2d78442a37c00daa4cb737007e8a069e10ae3d4a298c0d38c38976f8289";

const SCHEMA: &str = "OTERYN_CW2_ABILITY_EFFECT_FORMULA_EVIDENCE_BATCH/v1";
const SOURCE_REPOSITORY: &str = "Oteryn/Oteryn-Game";
const SOURCE_REVISION: &str = "03a821edd828e24ccff6e2cb7fc819a776cbd238";
const MAPPER_PROFILE: &str = "OTERYN_CW2_ABILITY_EFFECT_FORMULA_EVIDENCE_MAPPER/v1";
const MAPPER_BLOB: &str = "e6d98aadd352ad36b466970e1f7182e1bf93643b";
const MAPPER_SHA256: &str = "bc68f0f63a5dd6ea5ee7a3c20b708d6ea78c6033a2f5dddb518c49d6f45f8666";
const MAPPER_PATH: &str =
    "tools/reference-world-corridor-census/ability_effect_formula_evidence_catalog.py";

const TOP_LEVEL_FIELDS: &[&str] = &[
    "ability_effect_formula_candidates",
    "admission_main",
    "closure",
    "counts",
    "loss_report",
    "mapper_profile",
    "mapper_revision",
    "non_claims",
    "product_digest_scope",
    "product_digest_sha256",
    "production_authority",
    "reference_boundary",
    "schema",
    "source_snapshot",
    "task",
    "typed_surface",
];

const CANDIDATE_FIELDS: &[&str] = &[
    "ability_to_effect",
    "display_name",
    "effect_to_formula",
    "executable_promotion",
    "incantation_candidate",
    "legal_review",
    "native_ability_identity",
    "parity",
    "reference_case_ids",
    "source_candidate_id",
    "source_provenance",
    "target_evidence",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtectedCw2B4ImportError {
    InputLimitExceeded { actual: usize, limit: usize },
    InvalidJson(String),
    EvidenceMismatch(&'static str),
}

impl Display for ProtectedCw2B4ImportError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputLimitExceeded { actual, limit } => {
                write!(
                    formatter,
                    "CW2-B4 evidence bytes exceed limit: {actual} > {limit}"
                )
            }
            Self::InvalidJson(error) => write!(formatter, "invalid CW2-B4 evidence JSON: {error}"),
            Self::EvidenceMismatch(field) => {
                write!(formatter, "protected CW2-B4 evidence mismatch: {field}")
            }
        }
    }
}

impl std::error::Error for ProtectedCw2B4ImportError {}

/// Convert the exact protected CW2-B4 evidence bytes into one candidate-only import batch.
///
/// Callers retain ownership of the project envelope and can append the returned batch to
/// [`ProjectDraft::imports`](super::ProjectDraft::imports) before invoking the existing canonical
/// project writer. Input size is bounded before JSON decoding allocates.
pub fn protected_cw2_b4_import_batch(
    evidence_bytes: &[u8],
) -> Result<ImportBatch, ProtectedCw2B4ImportError> {
    if evidence_bytes.len() > PROTECTED_CW2_B4_EVIDENCE_MAX_BYTES {
        return Err(ProtectedCw2B4ImportError::InputLimitExceeded {
            actual: evidence_bytes.len(),
            limit: PROTECTED_CW2_B4_EVIDENCE_MAX_BYTES,
        });
    }

    let evidence: Value = serde_json::from_slice(evidence_bytes)
        .map_err(|error| ProtectedCw2B4ImportError::InvalidJson(error.to_string()))?;
    let root = exact_object(&evidence, TOP_LEVEL_FIELDS, "top-level fields")?;

    exact_string(root, "schema", SCHEMA, "schema")?;
    exact_string(
        root,
        "task",
        "CW2-B4 ABILITY_EFFECT_FORMULA_EVIDENCE_504",
        "task",
    )?;
    exact_string(root, "admission_main", SOURCE_REVISION, "admission main")?;
    exact_string(root, "closure", "CANDIDATE_ONLY", "closure")?;
    exact_string(root, "production_authority", "NONE", "production authority")?;
    exact_string(
        root,
        "product_digest_scope",
        "canonical JSON excluding digest fields",
        "product digest scope",
    )?;
    exact_string(
        root,
        "product_digest_sha256",
        PROTECTED_CW2_B4_PRODUCT_SHA256,
        "product digest",
    )?;

    let source = object_field(root, "source_snapshot", "source snapshot")?;
    exact_string(source, "repository", SOURCE_REPOSITORY, "source repository")?;
    exact_string(source, "revision", SOURCE_REVISION, "source revision")?;
    let protected_inputs = array_field(source, "protected_inputs", "protected inputs")?;
    if protected_inputs.len() != 6 {
        return Err(ProtectedCw2B4ImportError::EvidenceMismatch(
            "protected input count",
        ));
    }

    exact_string(root, "mapper_profile", MAPPER_PROFILE, "mapper profile")?;
    let mapper = object_field(root, "mapper_revision", "mapper revision")?;
    exact_string(mapper, "profile", MAPPER_PROFILE, "mapper revision profile")?;
    exact_string(mapper, "path", MAPPER_PATH, "mapper path")?;
    exact_string(mapper, "git_blob", MAPPER_BLOB, "mapper blob")?;
    exact_string(mapper, "canonical_sha256", MAPPER_SHA256, "mapper digest")?;
    exact_u64(mapper, "canonical_size", 16_013, "mapper size")?;
    exact_string(
        mapper,
        "canonicalization",
        "repository text bytes; CRLF normalized to LF; lone CR rejected",
        "mapper canonicalization",
    )?;

    let candidates = array_field(root, "ability_effect_formula_candidates", "candidate array")?;
    if candidates.len() != 2 {
        return Err(ProtectedCw2B4ImportError::EvidenceMismatch(
            "candidate count",
        ));
    }

    let mut imported = Vec::with_capacity(candidates.len());
    let mut reimport_states = Vec::with_capacity(candidates.len() * 5);
    for candidate in candidates {
        let candidate = exact_object(candidate, CANDIDATE_FIELDS, "candidate fields")?;
        let source_candidate_id = required_string(candidate, "source_candidate_id")?;
        let (expected_label, expected_incantation, expected_family, expected_shape, operation) =
            match source_candidate_id {
                "reference-source:ability:ice_strike" => (
                    "Ice Strike",
                    "exori frigo",
                    "DAMAGE",
                    "TARGETED_ICE_DAMAGE",
                    ImportCandidateOperation::Damage,
                ),
                "reference-source:ability:light_healing" => (
                    "Light Healing",
                    "exura",
                    "HEAL",
                    "SELF_HEAL",
                    ImportCandidateOperation::Heal,
                ),
                _ => {
                    return Err(ProtectedCw2B4ImportError::EvidenceMismatch(
                        "candidate identity",
                    ));
                }
            };
        exact_string(candidate, "display_name", expected_label, "candidate label")?;
        exact_string(
            candidate,
            "incantation_candidate",
            expected_incantation,
            "candidate incantation",
        )?;
        for (field, expected, mismatch) in [
            ("target_evidence", "UNKNOWN", "target evidence"),
            ("source_provenance", "PENDING", "source provenance"),
            ("legal_review", "PENDING", "legal review"),
            ("parity", "PARITY_PENDING_EVIDENCE", "parity"),
        ] {
            exact_string(candidate, field, expected, mismatch)?;
        }

        let ability_effect = object_field(candidate, "ability_to_effect", "ability/effect")?;
        exact_string(
            ability_effect,
            "candidate_family",
            expected_family,
            "candidate family",
        )?;
        exact_string(
            ability_effect,
            "candidate_shape",
            expected_shape,
            "candidate shape",
        )?;
        exact_string(
            ability_effect,
            "disposition",
            "CANDIDATE_ONLY",
            "candidate disposition",
        )?;

        let formula = object_field(candidate, "effect_to_formula", "effect/formula")?;
        exact_string(formula, "disposition", "UNRESOLVED", "formula disposition")?;
        exact_string(formula, "formula_state", "UNKNOWN", "formula state")?;
        if formula.get("quantitative_formula") != Some(&Value::Null) {
            return Err(ProtectedCw2B4ImportError::EvidenceMismatch(
                "quantitative formula",
            ));
        }
        let unknown_fields = string_array(formula, "unknown_fields", "formula losses")?;
        if unknown_fields.is_empty() {
            return Err(ProtectedCw2B4ImportError::EvidenceMismatch(
                "formula losses",
            ));
        }

        let native = object_field(candidate, "native_ability_identity", "native identity")?;
        if native.get("content_key") != Some(&Value::Null) {
            return Err(ProtectedCw2B4ImportError::EvidenceMismatch(
                "native content key",
            ));
        }
        exact_string(
            native,
            "disposition",
            "UNRESOLVED",
            "native identity disposition",
        )?;

        let promotion = object_field(candidate, "executable_promotion", "promotion")?;
        exact_string(promotion, "disposition", "BLOCKED", "promotion disposition")?;
        let reason_codes = string_array(promotion, "reason_codes", "promotion reasons")?;
        let expected_reasons = [
            "TARGET_EVIDENCE_UNKNOWN",
            "SOURCE_PROVENANCE_PENDING",
            "LEGAL_REVIEW_PENDING",
            "NATIVE_IDENTITY_UNRESOLVED",
            "EXACT_FORMULA_UNKNOWN",
        ];
        if reason_codes.as_slice() != expected_reasons {
            return Err(ProtectedCw2B4ImportError::EvidenceMismatch(
                "promotion reasons",
            ));
        }

        let reference_case_ids = string_array(candidate, "reference_case_ids", "reference cases")?;
        if reference_case_ids.len() != 2 {
            return Err(ProtectedCw2B4ImportError::EvidenceMismatch(
                "reference case count",
            ));
        }

        let normalized_fields = vec![
            text_field("candidate.formula-state", "UNKNOWN"),
            text_field(
                "candidate.formula-unknown-fields",
                &unknown_fields.join(","),
            ),
            text_field("candidate.target-evidence", "UNKNOWN"),
            text_field("evidence.legal-review", "PENDING"),
            text_field("evidence.native-identity", "UNRESOLVED"),
            text_field("evidence.parity", "PARITY_PENDING_EVIDENCE"),
            text_field(
                "evidence.product-digest-sha256",
                PROTECTED_CW2_B4_PRODUCT_SHA256,
            ),
            text_field("evidence.reference-case-ids", &reference_case_ids.join(",")),
            text_field("evidence.source-provenance", "PENDING"),
        ];
        for field in [
            "candidate.formula-state",
            "candidate.target-evidence",
            "evidence.legal-review",
            "evidence.native-identity",
            "evidence.source-provenance",
        ] {
            let value = normalized_fields
                .iter()
                .find(|entry| entry.field_path == field)
                .map(|entry| entry.value.clone())
                .ok_or(ProtectedCw2B4ImportError::EvidenceMismatch(
                    "normalized field",
                ))?;
            reimport_states.push(ReimportFieldState {
                stable_identity: source_candidate_id.to_owned(),
                field_path: field.to_owned(),
                baseline: Some(value.clone()),
                upstream: Some(value.clone()),
                local: Some(value),
                decision: ReimportDecision::Unchanged,
            });
        }

        imported.push(ImportCandidate {
            source_candidate_id: source_candidate_id.to_owned(),
            source_label: expected_label.to_owned(),
            source_numeric_id: None,
            candidate_family: ImportCandidateFamily::AbilityEffectFormula,
            candidate_operation: operation,
            candidate_target: expected_shape.to_owned(),
            candidate_formula: "UNKNOWN".to_owned(),
            evidence_class: "UNKNOWN".to_owned(),
            closure_disposition: CandidateDisposition::Blocked,
            disposition_reason: reason_codes.join(","),
            normalized_fields,
        });
    }

    imported.sort_by(|left, right| left.source_candidate_id.cmp(&right.source_candidate_id));
    if imported[0].source_candidate_id == imported[1].source_candidate_id {
        return Err(ProtectedCw2B4ImportError::EvidenceMismatch(
            "duplicate candidate identity",
        ));
    }
    reimport_states.sort_by(|left, right| {
        left.stable_identity
            .cmp(&right.stable_identity)
            .then_with(|| left.field_path.cmp(&right.field_path))
    });

    if world_project_sha256(evidence_bytes) != PROTECTED_CW2_B4_EVIDENCE_SHA256 {
        return Err(ProtectedCw2B4ImportError::EvidenceMismatch(
            "evidence byte digest",
        ));
    }

    Ok(ImportBatch {
        batch_id: "cw2-b4-ability-effect-formula".to_owned(),
        source_repository: SOURCE_REPOSITORY.to_owned(),
        source_revision: SOURCE_REVISION.to_owned(),
        source_artifact_sha256: PROTECTED_CW2_B4_EVIDENCE_SHA256.to_owned(),
        access_disposition: "PENDING".to_owned(),
        source_generation_profile: SCHEMA.to_owned(),
        importer: "repository-protected-cw2-b4-evidence".to_owned(),
        mapper: MAPPER_PROFILE.to_owned(),
        mapper_revision: MAPPER_BLOB.to_owned(),
        mapper_sha256: MAPPER_SHA256.to_owned(),
        candidates: imported,
        reimport_states,
    })
}

fn exact_object<'a>(
    value: &'a Value,
    fields: &[&str],
    mismatch: &'static str,
) -> Result<&'a Map<String, Value>, ProtectedCw2B4ImportError> {
    let object = value
        .as_object()
        .ok_or(ProtectedCw2B4ImportError::EvidenceMismatch(mismatch))?;
    if object.len() != fields.len() || fields.iter().any(|field| !object.contains_key(*field)) {
        return Err(ProtectedCw2B4ImportError::EvidenceMismatch(mismatch));
    }
    Ok(object)
}

fn object_field<'a>(
    object: &'a Map<String, Value>,
    field: &str,
    mismatch: &'static str,
) -> Result<&'a Map<String, Value>, ProtectedCw2B4ImportError> {
    object
        .get(field)
        .and_then(Value::as_object)
        .ok_or(ProtectedCw2B4ImportError::EvidenceMismatch(mismatch))
}

fn array_field<'a>(
    object: &'a Map<String, Value>,
    field: &str,
    mismatch: &'static str,
) -> Result<&'a [Value], ProtectedCw2B4ImportError> {
    object
        .get(field)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or(ProtectedCw2B4ImportError::EvidenceMismatch(mismatch))
}

fn required_string<'a>(
    object: &'a Map<String, Value>,
    field: &str,
) -> Result<&'a str, ProtectedCw2B4ImportError> {
    object
        .get(field)
        .and_then(Value::as_str)
        .ok_or(ProtectedCw2B4ImportError::EvidenceMismatch(
            "required text field",
        ))
}

fn exact_string(
    object: &Map<String, Value>,
    field: &str,
    expected: &str,
    mismatch: &'static str,
) -> Result<(), ProtectedCw2B4ImportError> {
    if object.get(field).and_then(Value::as_str) != Some(expected) {
        return Err(ProtectedCw2B4ImportError::EvidenceMismatch(mismatch));
    }
    Ok(())
}

fn exact_u64(
    object: &Map<String, Value>,
    field: &str,
    expected: u64,
    mismatch: &'static str,
) -> Result<(), ProtectedCw2B4ImportError> {
    if object.get(field).and_then(Value::as_u64) != Some(expected) {
        return Err(ProtectedCw2B4ImportError::EvidenceMismatch(mismatch));
    }
    Ok(())
}

fn string_array(
    object: &Map<String, Value>,
    field: &str,
    mismatch: &'static str,
) -> Result<Vec<String>, ProtectedCw2B4ImportError> {
    object
        .get(field)
        .and_then(Value::as_array)
        .ok_or(ProtectedCw2B4ImportError::EvidenceMismatch(mismatch))?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or(ProtectedCw2B4ImportError::EvidenceMismatch(mismatch))
        })
        .collect()
}

fn text_field(path: &str, value: &str) -> NamedCandidateField {
    NamedCandidateField {
        field_path: path.to_owned(),
        value: CandidateValue::Text(value.to_owned()),
    }
}
