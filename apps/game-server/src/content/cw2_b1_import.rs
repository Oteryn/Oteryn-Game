//! Exact adapter for the owner-approved CW2-B1 vase binding.
//!
//! This adapter accepts only the protected B1 catalogue bytes. The source observations remain
//! import provenance and explicit losses; the native Item values are Oteryn-authored and use the
//! existing project Item record and Reference lowering path.

use super::{
    CandidateDisposition, CandidateValue, DefinitionIdentityDocument, ImportBatch, ImportCandidate,
    ImportCandidateFamily, ImportCandidateOperation, ItemStackDocument, NamedCandidateField,
    NativeItemBindingDisposition, NativeItemBindingDocument, ProjectReferenceRecord,
    ProjectionDocument, ReimportDecision, ReimportFieldState, world_project_sha256,
};
use std::fmt::{self, Display, Formatter};

pub const PROTECTED_CW2_B1_EVIDENCE_BYTES: usize = 16_877_870;
pub const PROTECTED_CW2_B1_EVIDENCE_BLOB: &str = "2f0121f3ea6586477b4535840b9a1f1bc28c677c";
pub const PROTECTED_CW2_B1_EVIDENCE_SHA256: &str =
    "7836c78cad130a5c404f648e76e0823f53ae6a34c6952b9b88c8bed2e50d96a7";
pub const PROTECTED_CW2_B1_PRODUCT_SHA256: &str =
    "d773076b576599b6ced7eb53e262cdc6363515d842da3e8518b610e2106db0fc";
pub const PROTECTED_CW2_B1_MAPPER_BLOB: &str = "904d62e1277ceae76434f75bca104686a7303ff2";
pub const PROTECTED_CW2_B1_MAPPER_SHA256: &str =
    "320ce69f516de2a6e3cec669493ec59a6f3103f405e624478cf2a76acd3d01b6";
pub const PROTECTED_CW2_B1_NODE_SHA256: &str =
    "b7c5c457cdccf047b251313e27cf283442a7346ddc556eaece8c2fdc30d655c3";
pub const PROTECTED_CW2_B1_FIELD_PROFILE_SHA256: &str =
    "11da415530a0c8678cdb3d05c3205f21f3cc708f13a7342674538f97b911472e";

pub const CW2_B1_SOURCE_REPOSITORY: &str = "zimbadev/crystalserver";
pub const CW2_B1_SOURCE_REVISION: &str = "ff7ede593c69d4c658b382c97443e8155926924a";
pub const CW2_B1_SOURCE_PATH: &str = "data/items/items.xml";
pub const CW2_B1_SOURCE_BLOB: &str = "0b1dc3ba1a49094d9c83b90ab399bd2a9dd7a17f";
pub const CW2_B1_SOURCE_SHA256: &str =
    "c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb";
pub const CW2_B1_SOURCE_BYTES: i64 = 3_819_874;
pub const CW2_B1_SOURCE_ITEM_ID: u64 = 2_876;

pub const CW2_B1_VASE_KEY: &str = "oteryn:item.decor.vase";
pub const CW2_B1_VASE_REVISION: &str = "definition-r1";
pub const CW2_B1_VASE_B3_ROW: &str = "definition:monster:0108:loot:0006";
pub const CW2_B1_VASE_B3_ROW_SHA256: &str =
    "f5d87a09806776c70a799abb5b9eb657ed66b93346d943053ba3fad6500b2712";

const CATALOG_SCHEMA: &str = "OTERYN_CW2_ITEM_IDENTITY_CATALOG_SOURCE_BATCH/v1";
const MAPPER_PROFILE: &str = "OTERYN_CW2_ITEM_IDENTITY_CATALOG_MAPPER/v1";
const SOURCE_CANDIDATE_ID: &str = "crystal:item:2876";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtectedCw2B1ImportError {
    InputLimitExceeded { actual: usize, limit: usize },
    EvidenceMismatch(&'static str),
}

impl Display for ProtectedCw2B1ImportError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputLimitExceeded { actual, limit } => {
                write!(
                    formatter,
                    "CW2-B1 evidence bytes exceed limit: {actual} > {limit}"
                )
            }
            Self::EvidenceMismatch(field) => {
                write!(formatter, "protected CW2-B1 evidence mismatch: {field}")
            }
        }
    }
}

impl std::error::Error for ProtectedCw2B1ImportError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtectedCw2B1VaseImport {
    pub record: ProjectReferenceRecord,
    pub batch: ImportBatch,
}

/// Produce the one approved native Item record and its bound import provenance.
///
/// The exact protected catalogue digest is checked before any catalogue decoding. No broad
/// catalogue family is imported, and none of the source pickup, weight, name, primary-type or B3
/// observations select native gameplay semantics.
pub fn protected_cw2_b1_vase_import(
    evidence_bytes: &[u8],
) -> Result<ProtectedCw2B1VaseImport, ProtectedCw2B1ImportError> {
    if evidence_bytes.len() > PROTECTED_CW2_B1_EVIDENCE_BYTES {
        return Err(ProtectedCw2B1ImportError::InputLimitExceeded {
            actual: evidence_bytes.len(),
            limit: PROTECTED_CW2_B1_EVIDENCE_BYTES,
        });
    }
    if evidence_bytes.len() != PROTECTED_CW2_B1_EVIDENCE_BYTES {
        return Err(ProtectedCw2B1ImportError::EvidenceMismatch(
            "evidence byte length",
        ));
    }
    if world_project_sha256(evidence_bytes) != PROTECTED_CW2_B1_EVIDENCE_SHA256 {
        return Err(ProtectedCw2B1ImportError::EvidenceMismatch(
            "evidence byte digest",
        ));
    }

    let identity = DefinitionIdentityDocument {
        family: "Item".to_owned(),
        key: CW2_B1_VASE_KEY.to_owned(),
        revision: CW2_B1_VASE_REVISION.to_owned(),
    };
    let record = ProjectReferenceRecord::Item {
        identity: identity.clone(),
        client_projection: ProjectionDocument::ClientSafe,
        materializable: true,
        stack_class: ItemStackDocument::NonStackable,
    };

    let normalized_fields = vec![
        field(
            "binding.native-item",
            CandidateValue::NativeItemBinding(NativeItemBindingDocument {
                identity,
                disposition: NativeItemBindingDisposition::LocalNonProduction,
            }),
        ),
        text_field("evidence.catalog-blob", PROTECTED_CW2_B1_EVIDENCE_BLOB),
        text_field(
            "evidence.catalog-product-sha256",
            PROTECTED_CW2_B1_PRODUCT_SHA256,
        ),
        text_field("evidence.catalog-sha256", PROTECTED_CW2_B1_EVIDENCE_SHA256),
        text_field(
            "evidence.field-profile-sha256",
            PROTECTED_CW2_B1_FIELD_PROFILE_SHA256,
        ),
        text_field("evidence.mapper-blob", PROTECTED_CW2_B1_MAPPER_BLOB),
        text_field("evidence.mapper-sha256", PROTECTED_CW2_B1_MAPPER_SHA256),
        text_field("evidence.node-sha256", PROTECTED_CW2_B1_NODE_SHA256),
        text_field("loss.b3-loot-semantics", "NOT_PROMOTED"),
        text_field("loss.redistribution-grant", "NONE"),
        text_field("loss.weight-native-field", "UNREPRESENTED"),
        text_field("loss.weight-unit", "UNKNOWN"),
        text_field("source.b3-loot-row-digest", CW2_B1_VASE_B3_ROW_SHA256),
        text_field("source.b3-loot-row-identity", CW2_B1_VASE_B3_ROW),
        field(
            "source.item-id",
            CandidateValue::SourceId(CW2_B1_SOURCE_ITEM_ID),
        ),
        text_field("source.items-xml-blob", CW2_B1_SOURCE_BLOB),
        field(
            "source.items-xml-byte-length",
            CandidateValue::Integer(CW2_B1_SOURCE_BYTES),
        ),
        text_field("source.items-xml-path", CW2_B1_SOURCE_PATH),
        text_field("source.items-xml-sha256", CW2_B1_SOURCE_SHA256),
        text_field("source.license-locator", "LICENSE"),
        text_field("source.license-observation", "GNU GPL v2"),
        field("source.pickup-eligibility", CandidateValue::Integer(1)),
        text_field("source.primary-type-disposition", "PROVENANCE_ONLY"),
        field("source.weight-raw", CandidateValue::Integer(940)),
    ];

    let reimport_states = [
        ("source.pickup-eligibility", CandidateValue::Integer(1)),
        ("source.weight-raw", CandidateValue::Integer(940)),
    ]
    .into_iter()
    .map(|(field_path, value)| ReimportFieldState {
        stable_identity: CW2_B1_VASE_KEY.to_owned(),
        field_path: field_path.to_owned(),
        baseline: Some(value.clone()),
        upstream: Some(value.clone()),
        local: Some(value),
        decision: ReimportDecision::Unchanged,
    })
    .collect();

    Ok(ProtectedCw2B1VaseImport {
        record,
        batch: ImportBatch {
            batch_id: "cw2-b1-vase-native-item".to_owned(),
            source_repository: CW2_B1_SOURCE_REPOSITORY.to_owned(),
            source_revision: CW2_B1_SOURCE_REVISION.to_owned(),
            source_artifact_sha256: PROTECTED_CW2_B1_EVIDENCE_SHA256.to_owned(),
            access_disposition: "PENDING".to_owned(),
            source_generation_profile: CATALOG_SCHEMA.to_owned(),
            importer: "repository-protected-cw2-b1-evidence".to_owned(),
            mapper: MAPPER_PROFILE.to_owned(),
            mapper_revision: PROTECTED_CW2_B1_MAPPER_BLOB.to_owned(),
            mapper_sha256: PROTECTED_CW2_B1_MAPPER_SHA256.to_owned(),
            candidates: vec![ImportCandidate {
                source_candidate_id: SOURCE_CANDIDATE_ID.to_owned(),
                source_label: "vase".to_owned(),
                source_numeric_id: Some(CW2_B1_SOURCE_ITEM_ID),
                candidate_family: ImportCandidateFamily::Item,
                candidate_operation: ImportCandidateOperation::BindNativeItem,
                candidate_target: format!("{CW2_B1_VASE_KEY}@{CW2_B1_VASE_REVISION}"),
                candidate_formula: "NOT_APPLICABLE".to_owned(),
                evidence_class: "OTS_HYPOTHESIS_ONLY".to_owned(),
                closure_disposition: CandidateDisposition::LocalNonProduction,
                disposition_reason: "OWNER_APPROVED_LOCAL_NON_PRODUCTION_BINDING".to_owned(),
                normalized_fields,
            }],
            reimport_states,
        },
    })
}

fn text_field(path: &str, value: &str) -> NamedCandidateField {
    field(path, CandidateValue::Text(value.to_owned()))
}

fn field(path: &str, value: CandidateValue) -> NamedCandidateField {
    NamedCandidateField {
        field_path: path.to_owned(),
        value,
    }
}
