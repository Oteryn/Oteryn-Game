//! Canonical editable project snapshot for `OTERYN_WORLD_PROJECT_SOURCE_PROFILE/v1`.
//!
//! This module deliberately starts after filesystem capture and ends before filesystem
//! publication. Callers supply immutable logical-locator/owned-byte pairs and receive a complete
//! canonical document set. Filesystem containment, no-follow admission, alias detection, staging,
//! journalling and atomic publication belong to a later boundary.

mod native_entry;
mod v2;
pub use native_entry::*;
pub use v2::*;

use super::{
    CanonicalReferencePlayableContent, ClientProjectionClass, ContentError, ContentLockBinding,
    ContentLockEntry, DefinitionFamily, DefinitionRevisionRef, PackageManifestBinding,
    ProductionAtom, ProductionKey, REFERENCE_PLAYABLE_CAPABILITY_PROFILE,
    REFERENCE_PLAYABLE_CONTENT_PROFILE_ID, ReferenceAbilityDefinition, ReferenceCreatureDefinition,
    ReferenceDefinition, ReferenceDefinitionKind, ReferenceEffectDefinition, ReferenceEffectFamily,
    ReferenceFormulaDefinition, ReferenceItemDefinition, ReferenceItemDestination,
    ReferenceItemPhysicalClass, ReferenceItemSemantics, ReferenceItemStackClass,
    ReferenceLootDefinition, ReferenceLootEntry, ReferenceLootSelectionAlgorithm,
    ReferencePlayableContentSource, Sha256HexDigest, TypedDefinitionRef, link_reference_playable,
};
use crate::foundation::WorldId;
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};
use std::io::{self, Write};

pub const WORLD_PROJECT_SOURCE_PROFILE: &str = "OTERYN_WORLD_PROJECT_SOURCE_PROFILE/v1";
pub const WORLD_PROJECT_ROOT_SCHEMA: &str = "OTERYN_WORLD_PROJECT_ROOT/v1";
pub const WORLD_PROJECT_MANIFEST_SCHEMA: &str = "OTERYN_WORLD_PROJECT_MANIFEST/v1";
pub const WORLD_PROJECT_LOCK_SCHEMA: &str = "OTERYN_WORLD_PROJECT_CONTENT_LOCK/v1";
pub const WORLD_PROJECT_REFERENCE_SCHEMA: &str = "OTERYN_WORLD_PROJECT_REFERENCE_RECORDS/v1";
pub const WORLD_PROJECT_IMPORT_SCHEMA: &str = "OTERYN_WORLD_PROJECT_IMPORT_CANDIDATES/v1";
pub const WORLD_PROJECT_METADATA_SCHEMA: &str = "OTERYN_WORLD_PROJECT_AUTHOR_METADATA/v1";

pub(super) const PROJECT_LOCATOR: &str = "project.json";
pub(super) const MANIFEST_LOCATOR: &str = "manifest.json";
pub(super) const LOCK_LOCATOR: &str = "content.lock.json";
const RECORDS_LOCATOR: &str = "records/reference.json";
const IMPORTS_LOCATOR: &str = "imports/candidates.json";
const METADATA_LOCATOR: &str = "metadata/author.json";
const CANONICAL_DOCUMENT_COUNT: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectError {
    InvalidLimit(&'static str),
    LimitExceeded {
        resource: &'static str,
        actual: usize,
        limit: usize,
    },
    InvalidLocator(String),
    DuplicateLocator(String),
    MissingDocument(String),
    UnexpectedDocument(String),
    DuplicateJsonMember(String),
    InvalidJson(String),
    InvalidProject(&'static str),
    DigestMismatch(String),
    Content(ContentError),
}

impl Display for ProjectError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimit(name) => write!(formatter, "invalid zero project limit: {name}"),
            Self::LimitExceeded {
                resource,
                actual,
                limit,
            } => {
                write!(
                    formatter,
                    "{resource} exceeds project limit: {actual} > {limit}"
                )
            }
            Self::InvalidLocator(value) => write!(formatter, "invalid project locator: {value}"),
            Self::DuplicateLocator(value) => {
                write!(formatter, "duplicate project locator: {value}")
            }
            Self::MissingDocument(value) => write!(formatter, "missing project document: {value}"),
            Self::UnexpectedDocument(value) => {
                write!(formatter, "unexpected project document: {value}")
            }
            Self::DuplicateJsonMember(value) => write!(formatter, "duplicate JSON member: {value}"),
            Self::InvalidJson(value) => write!(formatter, "invalid project JSON: {value}"),
            Self::InvalidProject(value) => write!(formatter, "invalid project: {value}"),
            Self::DigestMismatch(value) => write!(formatter, "project digest mismatch: {value}"),
            Self::Content(error) => Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for ProjectError {}

impl From<ContentError> for ProjectError {
    fn from(value: ContentError) -> Self {
        Self::Content(value)
    }
}

/// Finite, caller-selected limits for the first non-production evidence corpus.
///
/// These values are intentionally not production or full-world maxima.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectEvidenceLimits {
    pub max_documents: usize,
    pub max_document_bytes: usize,
    pub max_total_bytes: usize,
    pub max_json_depth: usize,
    pub max_decoded_fields: usize,
    pub max_string_bytes: usize,
    pub max_locator_bytes: usize,
    pub max_locator_segments: usize,
    pub max_reference_records: usize,
    pub max_import_records: usize,
    pub max_reimport_states: usize,
}

impl ProjectEvidenceLimits {
    pub fn validate(self) -> Result<Self, ProjectError> {
        for (name, value) in [
            ("documents", self.max_documents),
            ("document bytes", self.max_document_bytes),
            ("total bytes", self.max_total_bytes),
            ("JSON depth", self.max_json_depth),
            ("decoded fields", self.max_decoded_fields),
            ("string bytes", self.max_string_bytes),
            ("locator bytes", self.max_locator_bytes),
            ("locator segments", self.max_locator_segments),
            ("reference records", self.max_reference_records),
            ("import records", self.max_import_records),
            ("reimport states", self.max_reimport_states),
        ] {
            if value == 0 {
                return Err(ProjectError::InvalidLimit(name));
            }
        }
        Ok(self)
    }

    fn check(
        self,
        resource: &'static str,
        actual: usize,
        limit: usize,
    ) -> Result<(), ProjectError> {
        if actual > limit {
            return Err(ProjectError::LimitExceeded {
                resource,
                actual,
                limit,
            });
        }
        Ok(())
    }
}

fn checked_limit_sum(
    resource: &'static str,
    current: usize,
    increment: usize,
    limit: usize,
) -> Result<usize, ProjectError> {
    let actual = current
        .checked_add(increment)
        .ok_or(ProjectError::LimitExceeded {
            resource,
            actual: usize::MAX,
            limit,
        })?;
    if actual > limit {
        return Err(ProjectError::LimitExceeded {
            resource,
            actual,
            limit,
        });
    }
    Ok(actual)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSnapshot {
    documents: BTreeMap<String, Vec<u8>>,
}

impl ProjectSnapshot {
    pub fn new(
        documents: impl IntoIterator<Item = (String, Vec<u8>)>,
        limits: ProjectEvidenceLimits,
    ) -> Result<Self, ProjectError> {
        let limits = limits.validate()?;
        let mut admitted = BTreeMap::new();
        let mut total = 0_usize;
        for (locator, bytes) in documents {
            validate_locator(&locator, limits)?;
            let document_count =
                admitted
                    .len()
                    .checked_add(1)
                    .ok_or(ProjectError::LimitExceeded {
                        resource: "project documents",
                        actual: usize::MAX,
                        limit: limits.max_documents,
                    })?;
            limits.check("project documents", document_count, limits.max_documents)?;
            limits.check(
                "project document bytes",
                bytes.len(),
                limits.max_document_bytes,
            )?;
            total = checked_limit_sum(
                "project total bytes",
                total,
                bytes.len(),
                limits.max_total_bytes,
            )?;
            match admitted.entry(locator) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(bytes);
                }
                std::collections::btree_map::Entry::Occupied(entry) => {
                    return Err(ProjectError::DuplicateLocator(entry.key().clone()));
                }
            }
        }
        Ok(Self {
            documents: admitted,
        })
    }

    pub fn documents(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.documents
    }

    pub fn parse(&self, limits: ProjectEvidenceLimits) -> Result<WorldProject, ProjectError> {
        parse_snapshot(self, limits.validate()?, ProjectAdmission::Ordinary)
            .map(|(project, _)| project)
    }

    /// Admit, parse and qualify one native entry-room project (`NATIVE_ENTRY_SOURCE_QUALIFICATION_V1`).
    ///
    /// The native variant is selected here, before any control document is parsed, and is always
    /// parsed under the fixed `native_entry_first_slice_limits()` (#940 §4). Ordinary
    /// [`Self::parse`] refuses it.
    pub fn parse_native_entry(&self) -> Result<NativeEntryProject, ProjectError> {
        let limits = native_entry_first_slice_limits().project.validate()?;
        let (project, overlay) = parse_snapshot(self, limits, ProjectAdmission::NativeEntry)?;
        let overlay =
            overlay.ok_or(ProjectError::InvalidProject("native entry overlay missing"))?;
        NativeEntryProject::qualify(project, overlay)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalProjectDocuments {
    documents: BTreeMap<String, Vec<u8>>,
}

impl CanonicalProjectDocuments {
    pub fn from_draft(
        mut draft: ProjectDraft,
        limits: ProjectEvidenceLimits,
    ) -> Result<Self, ProjectError> {
        let limits = limits.validate()?;
        draft.records = sorted_records(draft.records);
        draft.imports = sorted_imports(draft.imports);
        draft.metadata = sorted_metadata(draft.metadata);
        draft.validate(limits)?;
        limits.check(
            "project documents",
            CANONICAL_DOCUMENT_COUNT,
            limits.max_documents,
        )?;
        for locator in [
            PROJECT_LOCATOR,
            MANIFEST_LOCATOR,
            LOCK_LOCATOR,
            RECORDS_LOCATOR,
            IMPORTS_LOCATOR,
            METADATA_LOCATOR,
        ] {
            validate_locator(locator, limits)?;
        }
        let mut write_budget = CanonicalWriteBudget::new(limits);
        let record_document = ReferenceDocument {
            schema: WORLD_PROJECT_REFERENCE_SCHEMA.to_owned(),
            world_id: draft.world_id,
            coordinate_frame: draft.coordinate_frame,
            records: draft.records,
        };
        let import_document = ImportDocument {
            schema: WORLD_PROJECT_IMPORT_SCHEMA.to_owned(),
            batches: draft.imports,
        };
        let metadata_document = MetadataDocument {
            schema: WORLD_PROJECT_METADATA_SCHEMA.to_owned(),
            entries: draft.metadata,
        };

        let mut managed = BTreeMap::new();
        managed.insert(
            RECORDS_LOCATOR.to_owned(),
            write_budget.encode(&record_document)?,
        );
        managed.insert(
            IMPORTS_LOCATOR.to_owned(),
            write_budget.encode(&import_document)?,
        );
        managed.insert(
            METADATA_LOCATOR.to_owned(),
            write_budget.encode(&metadata_document)?,
        );

        let roles = [
            (
                RECORDS_LOCATOR,
                "reference-records",
                WORLD_PROJECT_REFERENCE_SCHEMA,
            ),
            (
                IMPORTS_LOCATOR,
                "import-candidates",
                WORLD_PROJECT_IMPORT_SCHEMA,
            ),
            (
                METADATA_LOCATOR,
                "author-metadata",
                WORLD_PROJECT_METADATA_SCHEMA,
            ),
        ];
        let mut inventory: Vec<_> = roles
            .into_iter()
            .map(|(locator, role, schema)| {
                let bytes = managed.get(locator).ok_or(ProjectError::InvalidProject(
                    "canonical managed document missing",
                ))?;
                Ok(ManifestDocument {
                    role: role.to_owned(),
                    schema: schema.to_owned(),
                    locator: locator.to_owned(),
                    byte_length: bytes.len(),
                    sha256: digest_hex(bytes),
                })
            })
            .collect::<Result<_, ProjectError>>()?;
        inventory.sort_by(|left, right| left.locator.cmp(&right.locator));
        let manifest = ManifestDocumentRoot {
            schema: WORLD_PROJECT_MANIFEST_SCHEMA.to_owned(),
            package_key: draft.package_key,
            package_revision: draft.project_revision.clone(),
            semantic_schema_version: draft.semantic_schema_version,
            licensing_metadata: draft.licensing_metadata,
            required_features: Vec::new(),
            optional_features: Vec::new(),
            documents: inventory,
        };
        let manifest_bytes = write_budget.encode(&manifest)?;
        let package = package_binding(&manifest, &manifest_bytes)?;
        let lock = LockDocument {
            schema: WORLD_PROJECT_LOCK_SCHEMA.to_owned(),
            project_revision: draft.project_revision.clone(),
            revision_digest_token: format!("lock:{}", draft.project_revision),
            entries: vec![LockEntryDocument {
                package_key: manifest.package_key.clone(),
                package_revision: manifest.package_revision.clone(),
                package_provenance_digest: package.package_provenance_digest()?.as_str().to_owned(),
                floating: false,
                dependency: false,
            }],
        };
        let lock_bytes = write_budget.encode(&lock)?;
        let root = RootDocument {
            schema: WORLD_PROJECT_ROOT_SCHEMA.to_owned(),
            source_profile: WORLD_PROJECT_SOURCE_PROFILE.to_owned(),
            project_revision: draft.project_revision,
            manifest_locator: MANIFEST_LOCATOR.to_owned(),
            manifest_sha256: digest_hex(&manifest_bytes),
            content_lock_locator: LOCK_LOCATOR.to_owned(),
            content_lock_sha256: digest_hex(&lock_bytes),
        };

        let mut documents = managed;
        documents.insert(MANIFEST_LOCATOR.to_owned(), manifest_bytes);
        documents.insert(LOCK_LOCATOR.to_owned(), lock_bytes);
        documents.insert(PROJECT_LOCATOR.to_owned(), write_budget.encode(&root)?);
        let snapshot = ProjectSnapshot::new(documents.into_iter(), limits)?;
        snapshot.parse(limits)?;
        Ok(Self {
            documents: snapshot.documents,
        })
    }

    pub fn documents(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.documents
    }

    pub fn into_snapshot(
        self,
        limits: ProjectEvidenceLimits,
    ) -> Result<ProjectSnapshot, ProjectError> {
        ProjectSnapshot::new(self.documents, limits)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldProject {
    root: RootDocument,
    manifest: ManifestDocumentRoot,
    lock: LockDocument,
    reference: ReferenceDocument,
    imports: ImportDocument,
    metadata: MetadataDocument,
    manifest_bytes: Vec<u8>,
    v2: Option<ProjectV2State>,
}

impl WorldProject {
    pub fn project_revision(&self) -> &str {
        &self.root.project_revision
    }

    pub fn imports(&self) -> &[ImportBatch] {
        &self.imports.batches
    }

    pub fn author_metadata(&self) -> &[AuthorMetadataEntry] {
        &self.metadata.entries
    }

    pub fn v2(&self) -> Option<&ProjectV2State> {
        self.v2.as_ref()
    }

    pub fn migrate_to_v2(&self) -> ProjectV2Draft {
        ProjectV2Draft::from_project(self)
    }

    pub fn lower_reference_source(&self) -> Result<ReferencePlayableContentSource, ProjectError> {
        let package_manifest = package_binding(&self.manifest, &self.manifest_bytes)?;
        let content_lock = content_lock_binding(&self.lock)?;
        let mut definitions = Vec::with_capacity(self.reference.records.len());
        for record in &self.reference.records {
            definitions.push(record.lower()?);
        }
        Ok(ReferencePlayableContentSource {
            profile_revision: ProductionAtom::new(
                "reference profile revision",
                REFERENCE_PLAYABLE_CONTENT_PROFILE_ID,
            )?,
            capability_profile: ProductionAtom::new(
                "reference capability profile",
                REFERENCE_PLAYABLE_CAPABILITY_PROFILE,
            )?,
            package_manifest,
            content_lock,
            world_id: decode_world_id(&self.reference.world_id)?,
            coordinate_frame: super::CoordinateFrameRef::new(&self.reference.coordinate_frame)?,
            definitions,
            placements: Vec::new(),
            ordered_placements: Vec::new(),
            transitions: Vec::new(),
        })
    }

    pub fn link(&self) -> Result<CanonicalReferencePlayableContent, ProjectError> {
        Ok(link_reference_playable(self.lower_reference_source()?)?)
    }

    pub fn canonical_documents(
        &self,
        limits: ProjectEvidenceLimits,
    ) -> Result<CanonicalProjectDocuments, ProjectError> {
        if self.v2.is_some() {
            return CanonicalProjectDocuments::from_v2_draft(
                ProjectV2Draft::from_project(self),
                limits,
            );
        }
        CanonicalProjectDocuments::from_draft(
            ProjectDraft {
                project_revision: self.root.project_revision.clone(),
                package_key: self.manifest.package_key.clone(),
                semantic_schema_version: self.manifest.semantic_schema_version.clone(),
                licensing_metadata: self.manifest.licensing_metadata.clone(),
                world_id: self.reference.world_id.clone(),
                coordinate_frame: self.reference.coordinate_frame.clone(),
                records: self.reference.records.clone(),
                imports: self.imports.batches.clone(),
                metadata: self.metadata.entries.clone(),
            },
            limits,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDraft {
    pub project_revision: String,
    pub package_key: String,
    pub semantic_schema_version: String,
    pub licensing_metadata: String,
    pub world_id: String,
    pub coordinate_frame: String,
    pub records: Vec<ProjectReferenceRecord>,
    pub imports: Vec<ImportBatch>,
    pub metadata: Vec<AuthorMetadataEntry>,
}

impl ProjectDraft {
    fn validate(&self, limits: ProjectEvidenceLimits) -> Result<(), ProjectError> {
        ProductionAtom::new("project revision", &self.project_revision)?;
        ProductionKey::new(&self.package_key)?;
        ProductionAtom::new("project semantic schema", &self.semantic_schema_version)?;
        ProductionAtom::new("project licensing metadata", &self.licensing_metadata)?;
        decode_world_id(&self.world_id)?;
        super::CoordinateFrameRef::new(&self.coordinate_frame)?;
        limits.check(
            "project reference records",
            self.records.len(),
            limits.max_reference_records,
        )?;
        let import_records = self.imports.iter().try_fold(0_usize, |count, batch| {
            count
                .checked_add(batch.candidates.len())
                .ok_or(ProjectError::LimitExceeded {
                    resource: "project import records",
                    actual: usize::MAX,
                    limit: limits.max_import_records,
                })
        })?;
        limits.check(
            "project import records",
            import_records,
            limits.max_import_records,
        )?;
        let reimports = self.imports.iter().try_fold(0_usize, |count, batch| {
            count
                .checked_add(batch.reimport_states.len())
                .ok_or(ProjectError::LimitExceeded {
                    resource: "project reimport states",
                    actual: usize::MAX,
                    limit: limits.max_reimport_states,
                })
        })?;
        limits.check(
            "project reimport states",
            reimports,
            limits.max_reimport_states,
        )?;
        validate_reference_records(&self.records)?;
        validate_imports(&self.imports, &self.records)?;
        validate_native_item_licensing(&self.imports, &self.licensing_metadata)?;
        validate_metadata(&self.metadata)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
// This is a bounded source-document carrier, not a hot runtime representation. Keeping the
// existing Reference creature fields inline preserves the direct, typed JSON schema.
#[allow(clippy::large_enum_variant)]
pub enum ProjectReferenceRecord {
    Ability {
        identity: DefinitionIdentityDocument,
        effects: Vec<DefinitionReferenceDocument>,
    },
    Effect {
        identity: DefinitionIdentityDocument,
        client_projection: ProjectionDocument,
        effect_family: EffectFamilyDocument,
        formula: DefinitionReferenceDocument,
    },
    Formula {
        identity: DefinitionIdentityDocument,
    },
    Item {
        identity: DefinitionIdentityDocument,
        client_projection: ProjectionDocument,
        materializable: bool,
        stack_class: ItemStackDocument,
        #[serde(
            default,
            skip_serializing_if = "ReferenceItemSemantics::is_all_unknown"
        )]
        semantics: ReferenceItemSemantics,
    },
    Generic {
        identity: DefinitionIdentityDocument,
        client_projection: ProjectionDocument,
    },
    Creature {
        identity: DefinitionIdentityDocument,
        client_projection: ProjectionDocument,
        presentation: DefinitionReferenceDocument,
        behavior: DefinitionReferenceDocument,
        loot: Option<DefinitionReferenceDocument>,
    },
    Loot {
        identity: DefinitionIdentityDocument,
        algorithm: LootAlgorithmDocument,
        entries: Vec<LootEntryDocument>,
    },
    LocalObject {
        identity: DefinitionIdentityDocument,
        client_projection: ProjectionDocument,
        states: Vec<String>,
    },
}

impl ProjectReferenceRecord {
    fn identity(&self) -> &DefinitionIdentityDocument {
        match self {
            Self::Ability { identity, .. }
            | Self::Effect { identity, .. }
            | Self::Formula { identity, .. }
            | Self::Item { identity, .. }
            | Self::Generic { identity, .. }
            | Self::Creature { identity, .. }
            | Self::Loot { identity, .. }
            | Self::LocalObject { identity, .. } => identity,
        }
    }

    fn lower(&self) -> Result<ReferenceDefinition, ProjectError> {
        match self {
            Self::Ability { identity, effects } => {
                require_family(&identity.family, DefinitionFamily::Ability)?;
                for effect in effects {
                    require_family(&effect.family, DefinitionFamily::Effect)?;
                }
                Ok(ReferenceDefinition {
                    definition: identity.lower()?,
                    kind: ReferenceDefinitionKind::Ability(ReferenceAbilityDefinition {
                        effects: effects
                            .iter()
                            .map(DefinitionReferenceDocument::lower)
                            .collect::<Result<_, ProjectError>>()?,
                    }),
                    client_projection: ClientProjectionClass::ServerOnly,
                })
            }
            Self::Effect {
                identity,
                client_projection,
                effect_family,
                formula,
            } => {
                require_family(&identity.family, DefinitionFamily::Effect)?;
                require_family(&formula.family, DefinitionFamily::Formula)?;
                Ok(ReferenceDefinition {
                    definition: identity.lower()?,
                    kind: ReferenceDefinitionKind::Effect(ReferenceEffectDefinition {
                        family: effect_family.lower(),
                        formula: formula.lower()?,
                    }),
                    client_projection: client_projection.lower(),
                })
            }
            Self::Formula { identity } => {
                require_family(&identity.family, DefinitionFamily::Formula)?;
                Ok(ReferenceDefinition {
                    definition: identity.lower()?,
                    kind: ReferenceDefinitionKind::Formula(ReferenceFormulaDefinition),
                    client_projection: ClientProjectionClass::ServerOnly,
                })
            }
            Self::Item {
                identity,
                client_projection,
                materializable,
                stack_class,
                semantics,
            } => {
                require_family(&identity.family, DefinitionFamily::Item)?;
                let (physical_class, stack_class, legal_destinations) = match stack_class {
                    ItemStackDocument::Unknown => {
                        if *materializable {
                            return Err(ProjectError::InvalidProject(
                                "identity-only Item cannot be materializable",
                            ));
                        }
                        (
                            ReferenceItemPhysicalClass::Unknown,
                            ReferenceItemStackClass::Unknown,
                            Vec::new(),
                        )
                    }
                    ItemStackDocument::NonStackable => (
                        ReferenceItemPhysicalClass::Physical,
                        ReferenceItemStackClass::NonStackable,
                        if *materializable {
                            vec![ReferenceItemDestination::CharacterInventory]
                        } else {
                            Vec::new()
                        },
                    ),
                    ItemStackDocument::StackCapable => (
                        ReferenceItemPhysicalClass::Physical,
                        ReferenceItemStackClass::StackCapable,
                        if *materializable {
                            vec![ReferenceItemDestination::CharacterInventory]
                        } else {
                            Vec::new()
                        },
                    ),
                };
                Ok(ReferenceDefinition {
                    definition: identity.lower()?,
                    kind: ReferenceDefinitionKind::Item(ReferenceItemDefinition {
                        physical_class,
                        materializable: *materializable,
                        stack_class,
                        legal_destinations,
                        semantics: semantics.clone(),
                    }),
                    client_projection: client_projection.lower(),
                })
            }
            Self::Generic {
                identity,
                client_projection,
            } => {
                let family = parse_family(&identity.family)?;
                if !matches!(
                    family,
                    DefinitionFamily::Terrain
                        | DefinitionFamily::Presentation
                        | DefinitionFamily::Behavior
                ) {
                    return Err(ProjectError::InvalidProject(
                        "unsupported generic Reference family",
                    ));
                }
                Ok(ReferenceDefinition {
                    definition: identity.lower()?,
                    kind: ReferenceDefinitionKind::Generic,
                    client_projection: client_projection.lower(),
                })
            }
            Self::Creature {
                identity,
                client_projection,
                presentation,
                behavior,
                loot,
            } => {
                require_family(&identity.family, DefinitionFamily::Creature)?;
                require_family(&presentation.family, DefinitionFamily::Presentation)?;
                require_family(&behavior.family, DefinitionFamily::Behavior)?;
                if let Some(loot) = loot {
                    require_family(&loot.family, DefinitionFamily::Loot)?;
                }
                Ok(ReferenceDefinition {
                    definition: identity.lower()?,
                    kind: ReferenceDefinitionKind::Creature(ReferenceCreatureDefinition {
                        presentation: presentation.lower()?,
                        behavior: behavior.lower()?,
                        loot: loot
                            .as_ref()
                            .map(DefinitionReferenceDocument::lower)
                            .transpose()?,
                    }),
                    client_projection: client_projection.lower(),
                })
            }
            Self::Loot {
                identity,
                algorithm,
                entries,
            } => {
                require_family(&identity.family, DefinitionFamily::Loot)?;
                Ok(ReferenceDefinition {
                    definition: identity.lower()?,
                    kind: ReferenceDefinitionKind::Loot(ReferenceLootDefinition {
                        algorithm: algorithm.lower(),
                        entries: entries
                            .iter()
                            .map(LootEntryDocument::lower)
                            .collect::<Result<_, _>>()?,
                    }),
                    client_projection: ClientProjectionClass::ServerOnly,
                })
            }
            Self::LocalObject {
                identity,
                client_projection,
                states,
            } => {
                require_family(&identity.family, DefinitionFamily::LocalObject)?;
                Ok(ReferenceDefinition {
                    definition: identity.lower()?,
                    kind: ReferenceDefinitionKind::LocalObjectStates(
                        states
                            .iter()
                            .map(|state| ProductionKey::new(state).map_err(ProjectError::from))
                            .collect::<Result<_, _>>()?,
                    ),
                    client_projection: client_projection.lower(),
                })
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LootAlgorithmDocument {
    IndependentBernoulliPpm,
    WeightedSingleSelection,
    GuaranteedEntries,
    NestedGroups,
}

impl LootAlgorithmDocument {
    fn lower(self) -> ReferenceLootSelectionAlgorithm {
        match self {
            Self::IndependentBernoulliPpm => {
                ReferenceLootSelectionAlgorithm::IndependentBernoulliPpm
            }
            Self::WeightedSingleSelection => {
                ReferenceLootSelectionAlgorithm::WeightedSingleSelection
            }
            Self::GuaranteedEntries => ReferenceLootSelectionAlgorithm::GuaranteedEntries,
            Self::NestedGroups => ReferenceLootSelectionAlgorithm::NestedGroups,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LootEntryDocument {
    pub item: DefinitionReferenceDocument,
    pub min_count: u32,
    pub max_count: u32,
    pub probability_ppm: Option<u32>,
}

impl LootEntryDocument {
    fn lower(&self) -> Result<ReferenceLootEntry, ProjectError> {
        require_family(&self.item.family, DefinitionFamily::Item)?;
        Ok(ReferenceLootEntry {
            item: self.item.lower()?,
            min_count: self.min_count,
            max_count: self.max_count,
            probability_ppm: self.probability_ppm,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefinitionIdentityDocument {
    pub family: String,
    pub key: String,
    pub revision: String,
}

impl DefinitionIdentityDocument {
    fn lower(&self) -> Result<TypedDefinitionRef, ProjectError> {
        Ok(TypedDefinitionRef::new(
            parse_family(&self.family)?,
            ProductionKey::new(&self.key)?,
            DefinitionRevisionRef::new(&self.revision)?,
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefinitionReferenceDocument {
    pub family: String,
    pub key: String,
    pub revision: String,
}

impl DefinitionReferenceDocument {
    fn lower(&self) -> Result<TypedDefinitionRef, ProjectError> {
        DefinitionIdentityDocument {
            family: self.family.clone(),
            key: self.key.clone(),
            revision: self.revision.clone(),
        }
        .lower()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectionDocument {
    ServerOnly,
    ClientSafe,
}

impl ProjectionDocument {
    fn lower(self) -> ClientProjectionClass {
        match self {
            Self::ServerOnly => ClientProjectionClass::ServerOnly,
            Self::ClientSafe => ClientProjectionClass::ClientSafe,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemStackDocument {
    Unknown,
    NonStackable,
    StackCapable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectFamilyDocument {
    Damage,
    Heal,
}

impl EffectFamilyDocument {
    fn lower(self) -> ReferenceEffectFamily {
        match self {
            Self::Damage => ReferenceEffectFamily::Damage,
            Self::Heal => ReferenceEffectFamily::Heal,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportBatch {
    pub batch_id: String,
    pub source_repository: String,
    pub source_revision: String,
    pub source_artifact_sha256: String,
    pub access_disposition: String,
    pub source_generation_profile: String,
    pub importer: String,
    pub mapper: String,
    pub mapper_revision: String,
    pub mapper_sha256: String,
    pub candidates: Vec<ImportCandidate>,
    pub reimport_states: Vec<ReimportFieldState>,
}

impl ImportBatch {
    fn has_native_item_binding(&self) -> bool {
        self.candidates
            .iter()
            .any(ImportCandidate::native_item_binding)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportCandidate {
    pub source_candidate_id: String,
    pub source_label: String,
    pub source_numeric_id: Option<u64>,
    pub candidate_family: ImportCandidateFamily,
    pub candidate_operation: ImportCandidateOperation,
    pub candidate_target: String,
    pub candidate_formula: String,
    pub evidence_class: String,
    pub closure_disposition: CandidateDisposition,
    pub disposition_reason: String,
    pub normalized_fields: Vec<NamedCandidateField>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportCandidateFamily {
    AbilityEffectFormula,
    Item,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportCandidateOperation {
    Damage,
    Heal,
    BindNativeItem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CandidateDisposition {
    CandidateOnly,
    Blocked,
    LocalNonProduction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamedCandidateField {
    pub field_path: String,
    pub value: CandidateValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", deny_unknown_fields)]
pub enum CandidateValue {
    Text(String),
    Integer(i64),
    Boolean(bool),
    SourceId(u64),
    NativeItemBinding(NativeItemBindingDocument),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeItemBindingDocument {
    pub identity: DefinitionIdentityDocument,
    pub disposition: NativeItemBindingDisposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NativeItemBindingDisposition {
    LocalNonProduction,
}

impl ImportCandidate {
    fn native_item_binding(&self) -> bool {
        self.normalized_fields
            .iter()
            .any(|field| matches!(&field.value, CandidateValue::NativeItemBinding(_)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReimportDecision {
    Unchanged,
    AdoptUpstream,
    RetainLocal,
    Converged,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReimportFieldState {
    pub stable_identity: String,
    pub field_path: String,
    pub baseline: Option<CandidateValue>,
    pub upstream: Option<CandidateValue>,
    pub local: Option<CandidateValue>,
    pub decision: ReimportDecision,
}

pub fn decide_reimport(
    baseline: &Option<CandidateValue>,
    upstream: &Option<CandidateValue>,
    local: &Option<CandidateValue>,
) -> ReimportDecision {
    if upstream == baseline && local == baseline {
        ReimportDecision::Unchanged
    } else if upstream != baseline && local == baseline {
        ReimportDecision::AdoptUpstream
    } else if upstream == baseline && local != baseline {
        ReimportDecision::RetainLocal
    } else if upstream == local {
        ReimportDecision::Converged
    } else {
        ReimportDecision::Conflict
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorMetadataEntry {
    pub stable_identity: String,
    pub display_name: String,
    pub description: String,
    pub categories: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RootDocument {
    schema: String,
    source_profile: String,
    project_revision: String,
    manifest_locator: String,
    manifest_sha256: String,
    content_lock_locator: String,
    content_lock_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestDocumentRoot {
    schema: String,
    package_key: String,
    package_revision: String,
    semantic_schema_version: String,
    licensing_metadata: String,
    required_features: Vec<String>,
    optional_features: Vec<String>,
    documents: Vec<ManifestDocument>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestDocument {
    role: String,
    schema: String,
    locator: String,
    byte_length: usize,
    sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LockDocument {
    schema: String,
    project_revision: String,
    revision_digest_token: String,
    entries: Vec<LockEntryDocument>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LockEntryDocument {
    package_key: String,
    package_revision: String,
    package_provenance_digest: String,
    floating: bool,
    dependency: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReferenceDocument {
    schema: String,
    world_id: String,
    coordinate_frame: String,
    records: Vec<ProjectReferenceRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportDocument {
    schema: String,
    batches: Vec<ImportBatch>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MetadataDocument {
    schema: String,
    entries: Vec<AuthorMetadataEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProjectCaptureDocument {
    pub(super) locator: String,
    pub(super) byte_length: usize,
    pub(super) sha256: String,
}

/// Which source variant a capture may admit. The native entry variant is selected explicitly by
/// its own API before any parse; ordinary capture never accepts it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProjectAdmission {
    Ordinary,
    NativeEntry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProjectCapturePlan {
    admission: ProjectAdmission,
    root: RootDocument,
    manifest: ManifestDocumentRoot,
    lock: LockDocument,
    documents: Vec<ProjectCaptureDocument>,
}

impl ProjectCapturePlan {
    pub(super) fn from_control_documents(
        root_bytes: &[u8],
        manifest_bytes: &[u8],
        lock_bytes: &[u8],
        limits: ProjectEvidenceLimits,
    ) -> Result<Self, ProjectError> {
        Self::from_control_documents_for(
            root_bytes,
            manifest_bytes,
            lock_bytes,
            limits,
            ProjectAdmission::Ordinary,
        )
    }

    pub(super) fn from_control_documents_for(
        root_bytes: &[u8],
        manifest_bytes: &[u8],
        lock_bytes: &[u8],
        limits: ProjectEvidenceLimits,
        admission: ProjectAdmission,
    ) -> Result<Self, ProjectError> {
        let limits = limits.validate()?;
        for bytes in [root_bytes, manifest_bytes, lock_bytes] {
            limits.check(
                "project document bytes",
                bytes.len(),
                limits.max_document_bytes,
            )?;
        }

        let root: RootDocument = parse_strict(root_bytes, limits)?;
        let is_v2 = match admission {
            ProjectAdmission::Ordinary => {
                root.schema == WORLD_PROJECT_V2_ROOT_SCHEMA
                    && root.source_profile == WORLD_PROJECT_V2_SOURCE_PROFILE
            }
            ProjectAdmission::NativeEntry => {
                if root.schema != WORLD_PROJECT_V2_ROOT_SCHEMA
                    || root.source_profile != NATIVE_ENTRY_SOURCE_PROFILE
                {
                    return Err(ProjectError::InvalidProject(
                        "native entry admission requires the native source profile",
                    ));
                }
                true
            }
        };
        if !is_v2
            && (root.schema != WORLD_PROJECT_ROOT_SCHEMA
                || root.source_profile != WORLD_PROJECT_SOURCE_PROFILE)
        {
            return Err(ProjectError::InvalidProject(
                "unsupported project root profile",
            ));
        }
        if root.manifest_locator != MANIFEST_LOCATOR || root.content_lock_locator != LOCK_LOCATOR {
            return Err(ProjectError::InvalidProject("v1 control locator mismatch"));
        }
        require_digest(MANIFEST_LOCATOR, manifest_bytes, &root.manifest_sha256)?;
        require_digest(LOCK_LOCATOR, lock_bytes, &root.content_lock_sha256)?;

        let manifest: ManifestDocumentRoot = parse_strict(manifest_bytes, limits)?;
        let lock: LockDocument = parse_strict(lock_bytes, limits)?;
        if manifest.schema
            != if is_v2 {
                WORLD_PROJECT_V2_MANIFEST_SCHEMA
            } else {
                WORLD_PROJECT_MANIFEST_SCHEMA
            }
            || lock.schema
                != if is_v2 {
                    WORLD_PROJECT_V2_LOCK_SCHEMA
                } else {
                    WORLD_PROJECT_LOCK_SCHEMA
                }
        {
            return Err(ProjectError::InvalidProject(
                "unsupported control document schema",
            ));
        }
        if root.project_revision != manifest.package_revision
            || root.project_revision != lock.project_revision
        {
            return Err(ProjectError::InvalidProject("mixed project revision"));
        }
        if !manifest.required_features.is_empty() || !manifest.optional_features.is_empty() {
            return Err(ProjectError::InvalidProject(
                "unsupported project feature declaration",
            ));
        }
        let package = package_binding(&manifest, manifest_bytes)?;
        let content_lock = content_lock_binding(&lock)?;
        if content_lock.entries.len() != 1 {
            return Err(ProjectError::InvalidProject(
                "v1 Content Lock must contain exactly one root entry",
            ));
        }
        let expected_provenance = package.package_provenance_digest()?;
        let entry = content_lock
            .entries
            .first()
            .ok_or(ProjectError::InvalidProject(
                "Content Lock root package missing",
            ))?;
        if entry.package_key != package.package_key
            || entry.package_revision != package.package_revision
            || entry.package_provenance_digest != expected_provenance
            || entry.floating
            || entry.dependency
        {
            return Err(ProjectError::InvalidProject(
                "Content Lock root provenance mismatch",
            ));
        }

        let document_count =
            manifest
                .documents
                .len()
                .checked_add(3)
                .ok_or(ProjectError::LimitExceeded {
                    resource: "project documents",
                    actual: usize::MAX,
                    limit: limits.max_documents,
                })?;
        limits.check("project documents", document_count, limits.max_documents)?;
        let mut total_bytes = 0_usize;
        for bytes in [root_bytes, manifest_bytes, lock_bytes] {
            total_bytes = checked_limit_sum(
                "project total bytes",
                total_bytes,
                bytes.len(),
                limits.max_total_bytes,
            )?;
        }

        let mut expected = BTreeSet::from([
            PROJECT_LOCATOR.to_owned(),
            MANIFEST_LOCATOR.to_owned(),
            LOCK_LOCATOR.to_owned(),
        ]);
        let mut by_role: BTreeMap<String, Vec<&ManifestDocument>> = BTreeMap::new();
        let mut previous_locator: Option<&str> = None;
        let mut documents = Vec::new();
        documents
            .try_reserve_exact(manifest.documents.len())
            .map_err(|_| ProjectError::LimitExceeded {
                resource: "project documents",
                actual: manifest.documents.len(),
                limit: limits.max_documents,
            })?;
        for document in &manifest.documents {
            validate_locator(&document.locator, limits)?;
            if matches!(
                document.locator.as_str(),
                PROJECT_LOCATOR | MANIFEST_LOCATOR | LOCK_LOCATOR
            ) {
                return Err(ProjectError::InvalidProject(
                    "control document appears in manifest inventory",
                ));
            }
            if previous_locator.is_some_and(|previous| previous >= document.locator.as_str()) {
                return Err(ProjectError::InvalidProject(
                    "manifest inventory is not identity sorted",
                ));
            }
            previous_locator = Some(&document.locator);
            if !expected.insert(document.locator.clone()) {
                return Err(ProjectError::DuplicateLocator(document.locator.clone()));
            }
            limits.check(
                "project document bytes",
                document.byte_length,
                limits.max_document_bytes,
            )?;
            total_bytes = checked_limit_sum(
                "project total bytes",
                total_bytes,
                document.byte_length,
                limits.max_total_bytes,
            )?;
            Sha256HexDigest::new(&document.sha256)?;
            by_role
                .entry(document.role.clone())
                .or_default()
                .push(document);
            documents.push(ProjectCaptureDocument {
                locator: document.locator.clone(),
                byte_length: document.byte_length,
                sha256: document.sha256.clone(),
            });
        }
        if is_v2 {
            validate_v2_roles(&by_role, admission)?;
        } else {
            require_role(
                &by_role,
                "reference-records",
                "records/",
                WORLD_PROJECT_REFERENCE_SCHEMA,
            )?;
            require_role(
                &by_role,
                "import-candidates",
                "imports/",
                WORLD_PROJECT_IMPORT_SCHEMA,
            )?;
            require_role(
                &by_role,
                "author-metadata",
                "metadata/",
                WORLD_PROJECT_METADATA_SCHEMA,
            )?;
            if by_role.len() != 3 {
                return Err(ProjectError::InvalidProject(
                    "unsupported manifest document role",
                ));
            }
        }

        Ok(Self {
            admission,
            root,
            manifest,
            lock,
            documents,
        })
    }

    pub(super) fn documents(&self) -> &[ProjectCaptureDocument] {
        &self.documents
    }
}

fn parse_snapshot(
    snapshot: &ProjectSnapshot,
    limits: ProjectEvidenceLimits,
    admission: ProjectAdmission,
) -> Result<(WorldProject, Option<NativeFirstEntryDocument>), ProjectError> {
    let root_bytes = required(snapshot, PROJECT_LOCATOR)?;
    let manifest_bytes = required(snapshot, MANIFEST_LOCATOR)?;
    let lock_bytes = required(snapshot, LOCK_LOCATOR)?;
    let plan = ProjectCapturePlan::from_control_documents_for(
        root_bytes,
        manifest_bytes,
        lock_bytes,
        limits,
        admission,
    )?;

    let mut expected = BTreeSet::from([
        PROJECT_LOCATOR.to_owned(),
        MANIFEST_LOCATOR.to_owned(),
        LOCK_LOCATOR.to_owned(),
    ]);
    let mut by_role: BTreeMap<String, Vec<&ManifestDocument>> = BTreeMap::new();
    for document in &plan.manifest.documents {
        expected.insert(document.locator.clone());
        by_role
            .entry(document.role.clone())
            .or_default()
            .push(document);
        let bytes = required(snapshot, &document.locator)?;
        if bytes.len() != document.byte_length {
            return Err(ProjectError::InvalidProject(
                "manifest byte length mismatch",
            ));
        }
        require_digest(&document.locator, bytes, &document.sha256)?;
    }
    for locator in snapshot.documents.keys() {
        if !expected.contains(locator) {
            return Err(ProjectError::UnexpectedDocument(locator.clone()));
        }
    }
    if snapshot.documents.len() != expected.len() {
        return Err(ProjectError::InvalidProject(
            "manifest document set mismatch",
        ));
    }
    if plan.root.schema == WORLD_PROJECT_V2_ROOT_SCHEMA {
        return parse_v2_snapshot(snapshot, limits, plan, manifest_bytes);
    }
    let reference_infos = require_role(
        &by_role,
        "reference-records",
        "records/",
        WORLD_PROJECT_REFERENCE_SCHEMA,
    )?;
    let import_infos = require_role(
        &by_role,
        "import-candidates",
        "imports/",
        WORLD_PROJECT_IMPORT_SCHEMA,
    )?;
    let metadata_infos = require_role(
        &by_role,
        "author-metadata",
        "metadata/",
        WORLD_PROJECT_METADATA_SCHEMA,
    )?;
    debug_assert_eq!(by_role.len(), 3);
    let mut reference: Option<ReferenceDocument> = None;
    for info in reference_infos {
        let parsed: ReferenceDocument = parse_strict(required(snapshot, &info.locator)?, limits)?;
        if parsed.schema != info.schema {
            return Err(ProjectError::InvalidProject(
                "managed document schema mismatch",
            ));
        }
        match &mut reference {
            Some(combined) => {
                if combined.world_id != parsed.world_id
                    || combined.coordinate_frame != parsed.coordinate_frame
                {
                    return Err(ProjectError::InvalidProject(
                        "regrouped Reference documents disagree on world binding",
                    ));
                }
                combined.records.extend(parsed.records);
            }
            None => reference = Some(parsed),
        }
    }
    let mut reference = reference.ok_or(ProjectError::InvalidProject(
        "required Reference document missing",
    ))?;
    reference.records = sorted_records(reference.records);
    let mut imports = ImportDocument {
        schema: WORLD_PROJECT_IMPORT_SCHEMA.to_owned(),
        batches: Vec::new(),
    };
    for info in import_infos {
        let parsed: ImportDocument = parse_strict(required(snapshot, &info.locator)?, limits)?;
        if parsed.schema != info.schema {
            return Err(ProjectError::InvalidProject(
                "managed document schema mismatch",
            ));
        }
        imports.batches.extend(parsed.batches);
    }
    imports.batches = sorted_imports(imports.batches);
    let mut metadata = MetadataDocument {
        schema: WORLD_PROJECT_METADATA_SCHEMA.to_owned(),
        entries: Vec::new(),
    };
    for info in metadata_infos {
        let parsed: MetadataDocument = parse_strict(required(snapshot, &info.locator)?, limits)?;
        if parsed.schema != info.schema {
            return Err(ProjectError::InvalidProject(
                "managed document schema mismatch",
            ));
        }
        metadata.entries.extend(parsed.entries);
    }
    metadata.entries = sorted_metadata(metadata.entries);
    limits.check(
        "project reference records",
        reference.records.len(),
        limits.max_reference_records,
    )?;
    let imported = imports.batches.iter().try_fold(0_usize, |total, batch| {
        total
            .checked_add(batch.candidates.len())
            .ok_or(ProjectError::LimitExceeded {
                resource: "project import records",
                actual: usize::MAX,
                limit: limits.max_import_records,
            })
    })?;
    limits.check(
        "project import records",
        imported,
        limits.max_import_records,
    )?;
    let states = imports.batches.iter().try_fold(0_usize, |total, batch| {
        total
            .checked_add(batch.reimport_states.len())
            .ok_or(ProjectError::LimitExceeded {
                resource: "project reimport states",
                actual: usize::MAX,
                limit: limits.max_reimport_states,
            })
    })?;
    limits.check(
        "project reimport states",
        states,
        limits.max_reimport_states,
    )?;
    validate_reference_records(&reference.records)?;
    validate_imports(&imports.batches, &reference.records)?;
    validate_native_item_licensing(&imports.batches, &plan.manifest.licensing_metadata)?;
    validate_metadata(&metadata.entries)?;
    Ok((
        WorldProject {
            root: plan.root,
            manifest: plan.manifest,
            lock: plan.lock,
            reference,
            imports,
            metadata,
            manifest_bytes: manifest_bytes.to_vec(),
            v2: None,
        },
        None,
    ))
}

fn required<'a>(snapshot: &'a ProjectSnapshot, locator: &str) -> Result<&'a [u8], ProjectError> {
    snapshot
        .documents
        .get(locator)
        .map(Vec::as_slice)
        .ok_or_else(|| ProjectError::MissingDocument(locator.to_owned()))
}

fn require_role<'a>(
    roles: &BTreeMap<String, Vec<&'a ManifestDocument>>,
    role: &str,
    locator_prefix: &str,
    schema: &str,
) -> Result<Vec<&'a ManifestDocument>, ProjectError> {
    let documents = roles
        .get(role)
        .cloned()
        .ok_or(ProjectError::InvalidProject(
            "required manifest role missing",
        ))?;
    if documents.is_empty()
        || documents.iter().any(|document| {
            !document.locator.starts_with(locator_prefix) || document.schema != schema
        })
    {
        return Err(ProjectError::InvalidProject(
            "v1 manifest role binding mismatch",
        ));
    }
    Ok(documents)
}

fn package_binding(
    manifest: &ManifestDocumentRoot,
    manifest_bytes: &[u8],
) -> Result<PackageManifestBinding, ProjectError> {
    Ok(PackageManifestBinding::new(
        ProductionKey::new(&manifest.package_key)?,
        ProductionAtom::new("project package revision", &manifest.package_revision)?,
        ProductionAtom::new("project semantic schema", &manifest.semantic_schema_version)?,
        ProductionAtom::new("project licensing metadata", &manifest.licensing_metadata)?,
        Sha256HexDigest::new(&digest_hex(manifest_bytes))?,
    ))
}

fn content_lock_binding(lock: &LockDocument) -> Result<ContentLockBinding, ProjectError> {
    let entries = lock
        .entries
        .iter()
        .map(|entry| {
            Ok(ContentLockEntry {
                package_key: ProductionKey::new(&entry.package_key)?,
                package_revision: ProductionAtom::new(
                    "project lock package revision",
                    &entry.package_revision,
                )?,
                package_provenance_digest: Sha256HexDigest::new(&entry.package_provenance_digest)?,
                floating: entry.floating,
                dependency: entry.dependency,
            })
        })
        .collect::<Result<Vec<_>, ProjectError>>()?;
    Ok(ContentLockBinding {
        revision_digest_token: ProductionAtom::new(
            "project Content Lock revision",
            &lock.revision_digest_token,
        )?,
        entries,
    })
}

fn validate_reference_records(records: &[ProjectReferenceRecord]) -> Result<(), ProjectError> {
    let mut identities = BTreeSet::new();
    let mut previous: Option<(&str, &str)> = None;
    for record in records {
        let identity = record.identity();
        parse_family(&identity.family)?;
        identity.lower()?;
        let current = (identity.family.as_str(), identity.key.as_str());
        if previous.is_some_and(|prior| prior >= current) {
            return Err(ProjectError::InvalidProject(
                "reference records are not identity sorted",
            ));
        }
        previous = Some(current);
        if !identities.insert((identity.family.clone(), identity.key.clone())) {
            return Err(ProjectError::InvalidProject(
                "duplicate reference record identity",
            ));
        }
        record.lower()?;
    }
    Ok(())
}

fn validate_native_item_licensing(
    imports: &[ImportBatch],
    licensing_metadata: &str,
) -> Result<(), ProjectError> {
    if imports.iter().any(ImportBatch::has_native_item_binding) && licensing_metadata != "PENDING" {
        return Err(ProjectError::InvalidProject(
            "local native item proof requires PENDING licensing metadata",
        ));
    }
    Ok(())
}

fn validate_imports(
    imports: &[ImportBatch],
    records: &[ProjectReferenceRecord],
) -> Result<(), ProjectError> {
    let item_record_identities = records
        .iter()
        .filter_map(|record| match record {
            ProjectReferenceRecord::Item { identity, .. } => Some((
                identity.family.as_str(),
                identity.key.as_str(),
                identity.revision.as_str(),
            )),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let mut previous_batch: Option<&str> = None;
    let mut native_item_bindings = BTreeSet::new();
    for batch in imports {
        if previous_batch.is_some_and(|prior| prior >= batch.batch_id.as_str()) {
            return Err(ProjectError::InvalidProject(
                "import batches are not identity sorted",
            ));
        }
        previous_batch = Some(&batch.batch_id);
        Sha256HexDigest::new(&batch.source_artifact_sha256)?;
        Sha256HexDigest::new(&batch.mapper_sha256)?;
        let has_native_item_binding = batch.has_native_item_binding();
        if has_native_item_binding && batch.access_disposition != "PENDING" {
            return Err(ProjectError::InvalidProject(
                "local native item proof requires PENDING access disposition",
            ));
        }
        let mut previous_candidate: Option<&str> = None;
        for candidate in &batch.candidates {
            if previous_candidate
                .is_some_and(|prior| prior >= candidate.source_candidate_id.as_str())
            {
                return Err(ProjectError::InvalidProject(
                    "import candidates are not identity sorted",
                ));
            }
            previous_candidate = Some(&candidate.source_candidate_id);
            let mut paths = BTreeSet::new();
            let mut native_binding: Option<&NativeItemBindingDocument> = None;
            for field in &candidate.normalized_fields {
                if !paths.insert(&field.field_path) {
                    return Err(ProjectError::InvalidProject(
                        "duplicate normalized import field",
                    ));
                }
                if let CandidateValue::NativeItemBinding(binding) = &field.value {
                    if field.field_path != "binding.native-item" || native_binding.is_some() {
                        return Err(ProjectError::InvalidProject(
                            "native item import requires one typed binding field",
                        ));
                    }
                    native_binding = Some(binding);
                }
            }
            match candidate.candidate_family {
                ImportCandidateFamily::AbilityEffectFormula => {
                    if native_binding.is_some()
                        || !matches!(
                            candidate.candidate_operation,
                            ImportCandidateOperation::Damage | ImportCandidateOperation::Heal
                        )
                        || candidate.closure_disposition == CandidateDisposition::LocalNonProduction
                    {
                        return Err(ProjectError::InvalidProject(
                            "ability import cannot carry a native item binding",
                        ));
                    }
                }
                ImportCandidateFamily::Item => {
                    let binding = native_binding.ok_or(ProjectError::InvalidProject(
                        "item import is missing its typed native binding",
                    ))?;
                    require_family(&binding.identity.family, DefinitionFamily::Item)?;
                    binding.identity.lower()?;
                    if candidate.source_numeric_id.is_none()
                        || candidate.candidate_operation != ImportCandidateOperation::BindNativeItem
                        || candidate.candidate_formula != "NOT_APPLICABLE"
                        || candidate.evidence_class != "OTS_HYPOTHESIS_ONLY"
                        || candidate.closure_disposition != CandidateDisposition::LocalNonProduction
                        || candidate.candidate_target
                            != format!("{}@{}", binding.identity.key, binding.identity.revision)
                    {
                        return Err(ProjectError::InvalidProject(
                            "native item import shape is inconsistent",
                        ));
                    }
                    if !item_record_identities.contains(&(
                        binding.identity.family.as_str(),
                        binding.identity.key.as_str(),
                        binding.identity.revision.as_str(),
                    )) {
                        return Err(ProjectError::InvalidProject(
                            "native item binding target is missing",
                        ));
                    }
                    if !native_item_bindings.insert((
                        binding.identity.family.clone(),
                        binding.identity.key.clone(),
                        binding.identity.revision.clone(),
                    )) {
                        return Err(ProjectError::InvalidProject(
                            "duplicate native item binding target",
                        ));
                    }
                }
            }
        }
        let mut previous_state: Option<(&str, &str)> = None;
        for state in &batch.reimport_states {
            let current = (state.stable_identity.as_str(), state.field_path.as_str());
            if previous_state.is_some_and(|prior| prior >= current) {
                return Err(ProjectError::InvalidProject(
                    "reimport states are not identity sorted",
                ));
            }
            previous_state = Some(current);
            if state.decision != decide_reimport(&state.baseline, &state.upstream, &state.local) {
                return Err(ProjectError::InvalidProject(
                    "stored reimport decision is inconsistent",
                ));
            }
            if has_native_item_binding && state.decision == ReimportDecision::Conflict {
                return Err(ProjectError::InvalidProject(
                    "local native item proof has an unresolved import conflict",
                ));
            }
        }
    }
    Ok(())
}

fn validate_metadata(metadata: &[AuthorMetadataEntry]) -> Result<(), ProjectError> {
    let mut previous: Option<&str> = None;
    for entry in metadata {
        if previous.is_some_and(|prior| prior >= entry.stable_identity.as_str()) {
            return Err(ProjectError::InvalidProject(
                "author metadata is not identity sorted",
            ));
        }
        previous = Some(&entry.stable_identity);
    }
    Ok(())
}

fn sorted_records(mut records: Vec<ProjectReferenceRecord>) -> Vec<ProjectReferenceRecord> {
    records.sort_by(|left, right| {
        left.identity()
            .family
            .cmp(&right.identity().family)
            .then_with(|| left.identity().key.cmp(&right.identity().key))
    });
    records
}

fn sorted_imports(mut imports: Vec<ImportBatch>) -> Vec<ImportBatch> {
    for batch in &mut imports {
        batch
            .candidates
            .sort_by(|left, right| left.source_candidate_id.cmp(&right.source_candidate_id));
        for candidate in &mut batch.candidates {
            candidate
                .normalized_fields
                .sort_by(|left, right| left.field_path.cmp(&right.field_path));
        }
        batch.reimport_states.sort_by(|left, right| {
            left.stable_identity
                .cmp(&right.stable_identity)
                .then_with(|| left.field_path.cmp(&right.field_path))
        });
    }
    imports.sort_by(|left, right| left.batch_id.cmp(&right.batch_id));
    imports
}

fn sorted_metadata(mut metadata: Vec<AuthorMetadataEntry>) -> Vec<AuthorMetadataEntry> {
    metadata.sort_by(|left, right| left.stable_identity.cmp(&right.stable_identity));
    metadata
}

fn parse_family(value: &str) -> Result<DefinitionFamily, ProjectError> {
    match value {
        "Terrain" => Ok(DefinitionFamily::Terrain),
        "Presentation" => Ok(DefinitionFamily::Presentation),
        "LocalObject" => Ok(DefinitionFamily::LocalObject),
        "Behavior" => Ok(DefinitionFamily::Behavior),
        "Creature" => Ok(DefinitionFamily::Creature),
        "Item" => Ok(DefinitionFamily::Item),
        "Loot" => Ok(DefinitionFamily::Loot),
        "Ability" => Ok(DefinitionFamily::Ability),
        "Effect" => Ok(DefinitionFamily::Effect),
        "Formula" => Ok(DefinitionFamily::Formula),
        _ => Err(ProjectError::InvalidProject(
            "unsupported Reference definition family",
        )),
    }
}

fn require_family(value: &str, expected: DefinitionFamily) -> Result<(), ProjectError> {
    if parse_family(value)? != expected {
        return Err(ProjectError::InvalidProject(
            "Reference record family does not match typed shape",
        ));
    }
    Ok(())
}

fn validate_locator(locator: &str, limits: ProjectEvidenceLimits) -> Result<(), ProjectError> {
    limits.check(
        "project locator bytes",
        locator.len(),
        limits.max_locator_bytes,
    )?;
    if locator.is_empty()
        || !locator.is_ascii()
        || locator.contains(['\\', ':', '\0'])
        || locator.starts_with('/')
        || locator.ends_with('/')
        || locator.contains('%')
    {
        return Err(ProjectError::InvalidLocator(locator.to_owned()));
    }
    let segments: Vec<&str> = locator.split('/').collect();
    limits.check(
        "project locator segments",
        segments.len(),
        limits.max_locator_segments,
    )?;
    for segment in segments {
        if segment.is_empty()
            || segment == "."
            || segment == ".."
            || segment.ends_with(['.', ' '])
            || !valid_segment(segment)
            || is_device_name(segment)
        {
            return Err(ProjectError::InvalidLocator(locator.to_owned()));
        }
    }
    Ok(())
}

fn valid_segment(segment: &str) -> bool {
    let components: Vec<&str> = segment.split('.').collect();
    !components.is_empty()
        && components.iter().all(|component| {
            !component.is_empty()
                && component.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'_' | b'-')
                })
                && component
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphanumeric)
                && component
                    .as_bytes()
                    .last()
                    .is_some_and(u8::is_ascii_alphanumeric)
        })
}

fn is_device_name(segment: &str) -> bool {
    let base = segment.split('.').next().unwrap_or(segment);
    matches!(base, "con" | "prn" | "aux" | "nul")
        || (base.len() == 4
            && (base.starts_with("com") || base.starts_with("lpt"))
            && matches!(base.as_bytes()[3], b'1'..=b'9'))
}

fn require_digest(locator: &str, bytes: &[u8], expected: &str) -> Result<(), ProjectError> {
    Sha256HexDigest::new(expected)?;
    if digest_hex(bytes) != expected {
        return Err(ProjectError::DigestMismatch(locator.to_owned()));
    }
    Ok(())
}

/// SHA-256 encoding used by snapshot manifests and roots. Filesystem adapters use this after
/// capturing immutable bytes; this function performs no filesystem access.
pub fn world_project_sha256(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = super::digest::sha256(bytes);
    let mut value = String::with_capacity(64);
    for byte in digest {
        value.push(char::from(HEX[usize::from(byte >> 4)]));
        value.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    value
}

fn digest_hex(bytes: &[u8]) -> String {
    world_project_sha256(bytes)
}

struct CanonicalWriteBudget {
    limits: ProjectEvidenceLimits,
    total_bytes: usize,
}

impl CanonicalWriteBudget {
    fn new(limits: ProjectEvidenceLimits) -> Self {
        Self {
            limits,
            total_bytes: 0,
        }
    }

    fn encode<T: Serialize>(&mut self, value: &T) -> Result<Vec<u8>, ProjectError> {
        let mut writer = BoundedDocumentWriter {
            bytes: Vec::new(),
            limits: self.limits,
            prior_total: self.total_bytes,
            failure: None,
        };
        if let Err(error) = serde_json::to_writer(&mut writer, value) {
            return Err(writer
                .failure
                .unwrap_or_else(|| ProjectError::InvalidJson(error.to_string())));
        }
        if let Err(error) = writer.write_all(b"\n") {
            return Err(writer
                .failure
                .unwrap_or_else(|| ProjectError::InvalidJson(error.to_string())));
        }
        self.total_bytes = checked_limit_sum(
            "project total bytes",
            self.total_bytes,
            writer.bytes.len(),
            self.limits.max_total_bytes,
        )?;
        Ok(writer.bytes)
    }
}

struct BoundedDocumentWriter {
    bytes: Vec<u8>,
    limits: ProjectEvidenceLimits,
    prior_total: usize,
    failure: Option<ProjectError>,
}

impl Write for BoundedDocumentWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let document_bytes = self.bytes.len().checked_add(buffer.len()).ok_or_else(|| {
            self.failure = Some(ProjectError::LimitExceeded {
                resource: "project document bytes",
                actual: usize::MAX,
                limit: self.limits.max_document_bytes,
            });
            io::Error::other("project document byte count overflow")
        })?;
        if document_bytes > self.limits.max_document_bytes {
            self.failure = Some(ProjectError::LimitExceeded {
                resource: "project document bytes",
                actual: document_bytes,
                limit: self.limits.max_document_bytes,
            });
            return Err(io::Error::other(
                "project document bytes exceed evidence limit",
            ));
        }
        let total_bytes = self
            .prior_total
            .checked_add(document_bytes)
            .ok_or_else(|| {
                self.failure = Some(ProjectError::LimitExceeded {
                    resource: "project total bytes",
                    actual: usize::MAX,
                    limit: self.limits.max_total_bytes,
                });
                io::Error::other("project total byte count overflow")
            })?;
        if total_bytes > self.limits.max_total_bytes {
            self.failure = Some(ProjectError::LimitExceeded {
                resource: "project total bytes",
                actual: total_bytes,
                limit: self.limits.max_total_bytes,
            });
            return Err(io::Error::other(
                "project total bytes exceed evidence limit",
            ));
        }
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn decode_world_id(value: &str) -> Result<WorldId, ProjectError> {
    if value.len() != 32
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(ProjectError::InvalidProject(
            "WorldId must be 32 lowercase hexadecimal UUIDv7 bytes",
        ));
    }
    let mut bytes = [0_u8; 16];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let hex = |byte| match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            _ => None,
        };
        let high = hex(pair[0]).ok_or(ProjectError::InvalidProject(
            "WorldId must be lowercase hexadecimal",
        ))?;
        let low = hex(pair[1]).ok_or(ProjectError::InvalidProject(
            "WorldId must be lowercase hexadecimal",
        ))?;
        bytes[index] = (high << 4) | low;
    }
    WorldId::decode(&bytes).map_err(|_| ProjectError::InvalidProject("WorldId must be UUIDv7"))
}

#[derive(Debug, Clone)]
enum StrictJsonValue {
    Null,
    Bool(bool),
    I64(i64),
    U64(u64),
    String(String),
    Array(Vec<StrictJsonValue>),
    Object(BTreeMap<String, StrictJsonValue>),
}

struct JsonBudget {
    limits: ProjectEvidenceLimits,
    values: usize,
    string_bytes: usize,
}

impl JsonBudget {
    fn new(limits: ProjectEvidenceLimits) -> Self {
        Self {
            limits,
            values: 0,
            string_bytes: 0,
        }
    }

    fn admit_value<E: de::Error>(&mut self, depth: usize) -> Result<(), E> {
        if depth > self.limits.max_json_depth {
            return Err(E::custom("project JSON depth exceeds evidence limit"));
        }
        self.values = self
            .values
            .checked_add(1)
            .ok_or_else(|| E::custom("project decoded field count overflow"))?;
        if self.values > self.limits.max_decoded_fields {
            return Err(E::custom("project decoded fields exceed evidence limit"));
        }
        Ok(())
    }

    fn admit_string<E: de::Error>(&mut self, bytes: usize) -> Result<(), E> {
        self.string_bytes = self
            .string_bytes
            .checked_add(bytes)
            .ok_or_else(|| E::custom("project JSON string byte count overflow"))?;
        if self.string_bytes > self.limits.max_string_bytes {
            return Err(E::custom("project JSON string bytes exceed evidence limit"));
        }
        Ok(())
    }
}

struct BudgetedValueSeed<'a> {
    budget: &'a mut JsonBudget,
    depth: usize,
}

impl<'de> DeserializeSeed<'de> for BudgetedValueSeed<'_> {
    type Value = StrictJsonValue;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        self.budget.admit_value(self.depth)?;
        deserializer.deserialize_any(BudgetedValueVisitor {
            budget: self.budget,
            depth: self.depth,
        })
    }
}

struct BudgetedValueVisitor<'a> {
    budget: &'a mut JsonBudget,
    depth: usize,
}

impl<'de> Visitor<'de> for BudgetedValueVisitor<'_> {
    type Value = StrictJsonValue;

    fn expecting(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("strict bounded JSON value")
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
        Ok(StrictJsonValue::Bool(value))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
        Ok(StrictJsonValue::I64(value))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
        Ok(StrictJsonValue::U64(value))
    }

    fn visit_f64<E: de::Error>(self, _value: f64) -> Result<Self::Value, E> {
        Err(E::custom("floating point numbers are unsupported"))
    }

    fn visit_borrowed_str<E: de::Error>(self, value: &'de str) -> Result<Self::Value, E> {
        self.budget.admit_string(value.len())?;
        Ok(StrictJsonValue::String(value.to_owned()))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        self.budget.admit_string(value.len())?;
        Ok(StrictJsonValue::String(value.to_owned()))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
        self.budget.admit_string(value.len())?;
        Ok(StrictJsonValue::String(value))
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(StrictJsonValue::Null)
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(StrictJsonValue::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let child_depth = self
            .depth
            .checked_add(1)
            .ok_or_else(|| de::Error::custom("project JSON depth overflow"))?;
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element_seed(BudgetedValueSeed {
            budget: &mut *self.budget,
            depth: child_depth,
        })? {
            values.push(value);
        }
        Ok(StrictJsonValue::Array(values))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let child_depth = self
            .depth
            .checked_add(1)
            .ok_or_else(|| de::Error::custom("project JSON depth overflow"))?;
        let mut values = BTreeMap::new();
        while let Some(key) = map.next_key_seed(BudgetedKeySeed {
            budget: &mut *self.budget,
        })? {
            match values.entry(key) {
                std::collections::btree_map::Entry::Occupied(entry) => {
                    return Err(de::Error::custom(format!(
                        "duplicate JSON member:{}",
                        entry.key()
                    )));
                }
                std::collections::btree_map::Entry::Vacant(entry) => {
                    let value = map.next_value_seed(BudgetedValueSeed {
                        budget: &mut *self.budget,
                        depth: child_depth,
                    })?;
                    entry.insert(value);
                }
            }
        }
        Ok(StrictJsonValue::Object(values))
    }
}

struct BudgetedKeySeed<'a> {
    budget: &'a mut JsonBudget,
}

impl<'de> DeserializeSeed<'de> for BudgetedKeySeed<'_> {
    type Value = String;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_string(BudgetedKeyVisitor {
            budget: self.budget,
        })
    }
}

struct BudgetedKeyVisitor<'a> {
    budget: &'a mut JsonBudget,
}

impl<'de> Visitor<'de> for BudgetedKeyVisitor<'_> {
    type Value = String;

    fn expecting(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("bounded JSON object member")
    }

    fn visit_borrowed_str<E: de::Error>(self, value: &'de str) -> Result<Self::Value, E> {
        self.budget.admit_string(value.len())?;
        Ok(value.to_owned())
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        self.budget.admit_string(value.len())?;
        Ok(value.to_owned())
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
        self.budget.admit_string(value.len())?;
        Ok(value)
    }
}

fn parse_strict<T: for<'de> Deserialize<'de>>(
    bytes: &[u8],
    limits: ProjectEvidenceLimits,
) -> Result<T, ProjectError> {
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        return Err(ProjectError::InvalidJson(
            "UTF-8 BOM is forbidden".to_owned(),
        ));
    }
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let mut budget = JsonBudget::new(limits);
    let strict = BudgetedValueSeed {
        budget: &mut budget,
        depth: 1,
    }
    .deserialize(&mut deserializer)
    .map_err(map_json_error)?;
    deserializer.end().map_err(map_json_error)?;
    let value = strict.into_serde_value();
    serde_json::from_value(value).map_err(|error| ProjectError::InvalidJson(error.to_string()))
}

fn map_json_error(error: serde_json::Error) -> ProjectError {
    let message = error.to_string();
    if let Some((_, member)) = message.split_once("duplicate JSON member:") {
        ProjectError::DuplicateJsonMember(
            member.split(" at line").next().unwrap_or(member).to_owned(),
        )
    } else {
        ProjectError::InvalidJson(message)
    }
}

impl StrictJsonValue {
    fn into_serde_value(self) -> serde_json::Value {
        match self {
            Self::Null => serde_json::Value::Null,
            Self::Bool(value) => serde_json::Value::Bool(value),
            Self::I64(value) => serde_json::Value::Number(value.into()),
            Self::U64(value) => serde_json::Value::Number(value.into()),
            Self::String(value) => serde_json::Value::String(value),
            Self::Array(values) => {
                serde_json::Value::Array(values.into_iter().map(Self::into_serde_value).collect())
            }
            Self::Object(values) => serde_json::Value::Object(
                values
                    .into_iter()
                    .map(|(key, value)| (key, value.into_serde_value()))
                    .collect(),
            ),
        }
    }
}

#[cfg(test)]
mod project_resource_tests {
    use super::*;

    #[test]
    fn checked_resource_accumulation_rejects_machine_overflow() {
        assert!(matches!(
            checked_limit_sum("overflow probe", usize::MAX, 1, usize::MAX),
            Err(ProjectError::LimitExceeded {
                resource: "overflow probe",
                actual: usize::MAX,
                limit: usize::MAX,
            })
        ));
    }
}
