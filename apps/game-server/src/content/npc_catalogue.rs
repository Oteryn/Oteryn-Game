//! Read-only import of existing NPC authoring data. This catalogue does not create actors,
//! execute dialogue/services, or modify the active world's Content generation.
use super::{
    CW2_B1_FULL_ITEM_FAMILY_COUNT, ProjectError, ProjectEvidenceLimits, ProjectFilesystemError,
    ProjectFilesystemLimits, ProjectV2AuthoringProfile, ProjectV2Declaration,
    ProjectV2DefinitionRef, ProjectV2Family, ProjectV2Source, ProjectV2SourceIdentityBinding,
    WorldProject, capture_world_project,
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsStr,
    path::Path,
};

/// Existing repository qualification budgets, for bounded preproduction data import only.
/// They do not establish production maxima or NPC gameplay readiness.
pub fn npc_catalogue_preproduction_limits() -> ProjectFilesystemLimits {
    ProjectFilesystemLimits {
        project: ProjectEvidenceLimits {
            max_documents: 11,
            max_document_bytes: 64_000_000,
            max_total_bytes: 160_000_000,
            max_json_depth: 24,
            max_decoded_fields: 2_120_000,
            max_string_bytes: 43_000_000,
            max_locator_bytes: 160,
            max_locator_segments: 8,
            max_reference_records: CW2_B1_FULL_ITEM_FAMILY_COUNT + 21_069 + 2_564,
            max_import_records: 24,
            max_reimport_states: 61,
        },
        max_entries_per_directory_scan: 32,
        max_total_directory_entries_scanned: 144 + 56 + 1,
    }
}

/// Bounded opt-in import pinned to an explicitly supplied canonical source-tree digest.
pub fn load_data_only_npc_catalogue(
    project_root: &Path,
    expected_tree_sha256: &str,
) -> Result<NpcDataCatalogue, ProjectFilesystemError> {
    if expected_tree_sha256.len() != 64
        || !expected_tree_sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(ProjectError::InvalidProject(
            "NPC source-tree SHA256 must be lowercase hexadecimal",
        )
        .into());
    }
    let parent = project_root
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let basename = project_root
        .file_name()
        .ok_or(ProjectError::InvalidProject(
            "NPC project root must name one directory",
        ))?;
    let catalogue = NpcDataCatalogue::load(parent, basename, npc_catalogue_preproduction_limits())?;
    if catalogue.source_tree_digest() != expected_tree_sha256 {
        return Err(
            ProjectError::InvalidProject("NPC catalogue source-tree digest mismatch").into(),
        );
    }
    Ok(catalogue)
}

/// Owns the validated source project and immutable indices into it. Authoring fields,
/// uncertain/defaulted classifications and provenance are retained without reinterpretation.
#[derive(Debug)]
pub struct NpcDataCatalogue {
    project: WorldProject,
    declarations: BTreeMap<ProjectV2DefinitionRef, usize>,
    profiles: BTreeMap<ProjectV2DefinitionRef, usize>,
    canonical_tree_sha256: String,
}
impl NpcDataCatalogue {
    pub fn load(
        parent: &Path,
        basename: &OsStr,
        limits: ProjectFilesystemLimits,
    ) -> Result<Self, ProjectFilesystemError> {
        let project = capture_world_project(parent, basename, limits)?;
        Ok(Self::from_project(project, limits.project)?)
    }

    /// Accept only a WorldProject already admitted by the existing parser. Canonical bytes
    /// are hashed from that same validated value, avoiding a second filesystem read/race.
    /// The digest identifies the canonical source tree, not arbitrary original whitespace.
    pub fn from_project(
        project: WorldProject,
        limits: ProjectEvidenceLimits,
    ) -> Result<Self, ProjectError> {
        let state = project.v2().ok_or(ProjectError::InvalidProject(
            "NPC catalogue requires WorldProject/v2",
        ))?;
        let mut declarations = BTreeMap::new();
        let mut referenced_profiles = BTreeSet::new();
        let mut referenced_records = BTreeSet::new();
        for declaration in &state.declarations {
            if let ProjectV2Declaration::Npc {
                dialogue, services, ..
            } = declaration
            {
                referenced_records.extend(dialogue.iter().cloned());
                referenced_records.extend(services.iter().cloned());
            }
        }
        for (index, declaration) in state.declarations.iter().enumerate() {
            let (identity, family) = match declaration {
                ProjectV2Declaration::Npc {
                    identity,
                    presentation,
                    behavior,
                    ..
                } => {
                    referenced_profiles.extend(presentation.iter().cloned());
                    referenced_profiles.extend(behavior.iter().cloned());
                    (identity, ProjectV2Family::Npc)
                }
                ProjectV2Declaration::Dialogue { identity, .. } => {
                    (identity, ProjectV2Family::Dialogue)
                }
                ProjectV2Declaration::Service { identity, .. } => {
                    (identity, ProjectV2Family::Service)
                }
                _ => continue,
            };
            let reference = ProjectV2DefinitionRef {
                family,
                key: identity.key.clone(),
                revision: identity.revision.clone(),
            };
            if family == ProjectV2Family::Npc || referenced_records.contains(&reference) {
                declarations.insert(reference, index);
            }
        }
        let profiles = state
            .authoring_profiles
            .iter()
            .enumerate()
            .filter(|(_, profile)| referenced_profiles.contains(&profile.target))
            .map(|(index, profile)| (profile.target.clone(), index))
            .collect();
        let documents = project.canonical_documents(limits)?;
        let mut digest = Sha256::new();
        for (locator, bytes) in documents.documents() {
            digest.update((locator.len() as u64).to_be_bytes());
            digest.update(locator.as_bytes());
            digest.update((bytes.len() as u64).to_be_bytes());
            digest.update(bytes);
        }
        let canonical_tree_sha256 = digest
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        Ok(Self {
            project,
            declarations,
            profiles,
            canonical_tree_sha256,
        })
    }
    pub fn project_revision(&self) -> &str {
        self.project.project_revision()
    }
    pub fn source_tree_digest(&self) -> &str {
        &self.canonical_tree_sha256
    }
    pub fn declaration(&self, reference: &ProjectV2DefinitionRef) -> Option<&ProjectV2Declaration> {
        self.project
            .v2()?
            .declarations
            .get(*self.declarations.get(reference)?)
    }
    pub fn profile(
        &self,
        reference: &ProjectV2DefinitionRef,
    ) -> Option<&ProjectV2AuthoringProfile> {
        self.project
            .v2()?
            .authoring_profiles
            .get(*self.profiles.get(reference)?)
    }
    pub fn records(
        &self,
    ) -> impl Iterator<Item = (&ProjectV2DefinitionRef, &ProjectV2Declaration)> {
        self.declarations.iter().filter_map(|(reference, _)| {
            self.declaration(reference)
                .map(|declaration| (reference, declaration))
        })
    }
    pub fn npc_count(&self) -> usize {
        self.count(ProjectV2Family::Npc)
    }
    pub fn dialogue_count(&self) -> usize {
        self.count(ProjectV2Family::Dialogue)
    }
    pub fn service_count(&self) -> usize {
        self.count(ProjectV2Family::Service)
    }
    pub fn profile_count(&self) -> usize {
        self.profiles.len()
    }
    fn count(&self, family: ProjectV2Family) -> usize {
        self.declarations
            .keys()
            .filter(|reference| reference.family == family)
            .count()
    }
    pub fn sources(&self) -> &[ProjectV2Source] {
        self.project
            .v2()
            .map_or(&[], |state| state.sources.as_slice())
    }
    pub fn source_identity_bindings(&self) -> &[ProjectV2SourceIdentityBinding] {
        self.project
            .v2()
            .map_or(&[], |state| state.source_identity_bindings.as_slice())
    }
}
