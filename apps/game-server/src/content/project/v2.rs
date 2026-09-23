//! Versioned editable-source additions. Declarative v2 records never become runtime definitions
//! by appearing in the project: only the existing Reference linker owns executable lowering.

use super::*;

pub const WORLD_PROJECT_V2_SOURCE_PROFILE: &str = "OTERYN_WORLD_PROJECT_SOURCE_PROFILE/v2";
pub const WORLD_PROJECT_V2_ROOT_SCHEMA: &str = "OTERYN_WORLD_PROJECT_ROOT/v2";
pub const WORLD_PROJECT_V2_MANIFEST_SCHEMA: &str = "OTERYN_WORLD_PROJECT_MANIFEST/v2";
pub const WORLD_PROJECT_V2_LOCK_SCHEMA: &str = "OTERYN_WORLD_PROJECT_CONTENT_LOCK/v2";

const DECLARATIONS_SCHEMA: &str = "OTERYN_WORLD_PROJECT_DECLARATIONS/v2";
const WORLDS_SCHEMA: &str = "OTERYN_WORLD_PROJECT_WORLDS/v2";
const PRESENTATIONS_SCHEMA: &str = "OTERYN_WORLD_PROJECT_PRESENTATION_BINDINGS/v2";
const ASSETS_SCHEMA: &str = "OTERYN_WORLD_PROJECT_ASSETS/v2";
const PROVENANCE_SCHEMA: &str = "OTERYN_WORLD_PROJECT_PROVENANCE/v2";
const EDITOR_SCHEMA: &str = "OTERYN_WORLD_PROJECT_EDITOR/v2";

const ROLE_SPECS: [(&str, &str, &str); 8] = [
    (
        "reference-records",
        "definitions/reference.json",
        WORLD_PROJECT_REFERENCE_SCHEMA,
    ),
    (
        "declarative-definitions",
        "definitions/declarations.json",
        DECLARATIONS_SCHEMA,
    ),
    ("world-records", "worlds/world.json", WORLDS_SCHEMA),
    (
        "presentation-bindings",
        "presentations/bindings.json",
        PRESENTATIONS_SCHEMA,
    ),
    ("asset-records", "assets/catalog.json", ASSETS_SCHEMA),
    (
        "import-candidates",
        "provenance/imports.json",
        WORLD_PROJECT_IMPORT_SCHEMA,
    ),
    (
        "provenance-records",
        "provenance/sources.json",
        PROVENANCE_SCHEMA,
    ),
    ("editor-records", "editor/author.json", EDITOR_SCHEMA),
];

/// This authoring vocabulary is intentionally distinct from executable `DefinitionFamily`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ProjectV2Family {
    Terrain,
    Presentation,
    LocalObject,
    WorldObject,
    Item,
    Creature,
    Ability,
    Effect,
    Formula,
    Loot,
    Behavior,
    #[serde(rename = "NPC")]
    Npc,
    Dialogue,
    Service,
    Interaction,
    Quest,
    Transition,
    House,
    Encounter,
}

impl ProjectV2Family {
    fn from_reference(value: &str) -> Result<Self, ProjectError> {
        Ok(match parse_family(value)? {
            DefinitionFamily::Terrain => Self::Terrain,
            DefinitionFamily::Presentation => Self::Presentation,
            DefinitionFamily::LocalObject => Self::LocalObject,
            DefinitionFamily::Item => Self::Item,
            DefinitionFamily::Creature => Self::Creature,
            DefinitionFamily::Ability => Self::Ability,
            DefinitionFamily::Effect => Self::Effect,
            DefinitionFamily::Formula => Self::Formula,
            DefinitionFamily::Loot => Self::Loot,
            DefinitionFamily::Behavior => Self::Behavior,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2DefinitionRef {
    pub family: ProjectV2Family,
    pub key: String,
    pub revision: String,
}

impl ProjectV2DefinitionRef {
    fn validate(&self) -> Result<(), ProjectError> {
        ProductionKey::new(&self.key)?;
        DefinitionRevisionRef::new(&self.revision)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2Identity {
    pub key: String,
    pub revision: String,
}

impl ProjectV2Identity {
    fn validate(&self) -> Result<(), ProjectError> {
        ProductionKey::new(&self.key)?;
        DefinitionRevisionRef::new(&self.revision)?;
        Ok(())
    }
}

/// Candidate-only structural declarations. No enum variant has an executable lowering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum ProjectV2Declaration {
    WorldObject {
        identity: ProjectV2Identity,
        presentation: Option<ProjectV2DefinitionRef>,
    },
    #[serde(rename = "NPC")]
    Npc {
        identity: ProjectV2Identity,
        presentation: Option<ProjectV2DefinitionRef>,
        behavior: Option<ProjectV2DefinitionRef>,
        dialogue: Option<ProjectV2DefinitionRef>,
        services: Vec<ProjectV2DefinitionRef>,
    },
    Dialogue {
        identity: ProjectV2Identity,
    },
    Service {
        identity: ProjectV2Identity,
    },
    Interaction {
        identity: ProjectV2Identity,
    },
    Quest {
        identity: ProjectV2Identity,
    },
    Transition {
        identity: ProjectV2Identity,
    },
    House {
        identity: ProjectV2Identity,
    },
    Encounter {
        identity: ProjectV2Identity,
    },
}

impl ProjectV2Declaration {
    fn family(&self) -> ProjectV2Family {
        match self {
            Self::WorldObject { .. } => ProjectV2Family::WorldObject,
            Self::Npc { .. } => ProjectV2Family::Npc,
            Self::Dialogue { .. } => ProjectV2Family::Dialogue,
            Self::Service { .. } => ProjectV2Family::Service,
            Self::Interaction { .. } => ProjectV2Family::Interaction,
            Self::Quest { .. } => ProjectV2Family::Quest,
            Self::Transition { .. } => ProjectV2Family::Transition,
            Self::House { .. } => ProjectV2Family::House,
            Self::Encounter { .. } => ProjectV2Family::Encounter,
        }
    }

    fn identity(&self) -> &ProjectV2Identity {
        match self {
            Self::WorldObject { identity, .. }
            | Self::Npc { identity, .. }
            | Self::Dialogue { identity }
            | Self::Service { identity }
            | Self::Interaction { identity }
            | Self::Quest { identity }
            | Self::Transition { identity }
            | Self::House { identity }
            | Self::Encounter { identity } => identity,
        }
    }

    fn references(&self) -> Vec<(ProjectV2Family, &ProjectV2DefinitionRef)> {
        match self {
            Self::WorldObject { presentation, .. } => presentation
                .iter()
                .map(|reference| (ProjectV2Family::Presentation, reference))
                .collect(),
            Self::Npc {
                presentation,
                behavior,
                dialogue,
                services,
                ..
            } => presentation
                .iter()
                .map(|reference| (ProjectV2Family::Presentation, reference))
                .chain(
                    behavior
                        .iter()
                        .map(|reference| (ProjectV2Family::Behavior, reference)),
                )
                .chain(
                    dialogue
                        .iter()
                        .map(|reference| (ProjectV2Family::Dialogue, reference)),
                )
                .chain(
                    services
                        .iter()
                        .map(|reference| (ProjectV2Family::Service, reference)),
                )
                .collect(),
            _ => Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2World {
    pub key: String,
    pub world_id: String,
    pub coordinate_frame: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectV2Disposition {
    CandidateOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2Placement {
    pub key: String,
    pub world: String,
    pub map_revision: String,
    pub definition: ProjectV2DefinitionRef,
    pub coordinate_frame: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub disposition: ProjectV2Disposition,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2AssetRef {
    pub key: String,
    pub revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2Asset {
    pub identity: ProjectV2AssetRef,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2AppearanceBinding {
    pub presentation: ProjectV2DefinitionRef,
    pub asset: ProjectV2AssetRef,
    pub disposition: ProjectV2Disposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectV2EvidenceClass {
    Proven,
    Derived,
    Unknown,
    Conflict,
    OtsHypothesisOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2Source {
    pub key: String,
    pub revision: String,
    pub sha256: String,
    pub evidence: ProjectV2EvidenceClass,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2EditorEntry {
    pub target: ProjectV2DefinitionRef,
    pub display_name: String,
    pub description: String,
    pub categories: Vec<String>,
    pub notes: Vec<String>,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectV2State {
    pub declarations: Vec<ProjectV2Declaration>,
    pub worlds: Vec<ProjectV2World>,
    pub placements: Vec<ProjectV2Placement>,
    pub appearance_bindings: Vec<ProjectV2AppearanceBinding>,
    pub assets: Vec<ProjectV2Asset>,
    pub sources: Vec<ProjectV2Source>,
    pub editor: Vec<ProjectV2EditorEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectV2Draft {
    pub core: ProjectDraft,
    pub state: ProjectV2State,
}

impl ProjectV2Draft {
    pub fn from_project(project: &WorldProject) -> Self {
        Self {
            core: ProjectDraft {
                project_revision: project.root.project_revision.clone(),
                package_key: project.manifest.package_key.clone(),
                semantic_schema_version: project.manifest.semantic_schema_version.clone(),
                licensing_metadata: project.manifest.licensing_metadata.clone(),
                world_id: project.reference.world_id.clone(),
                coordinate_frame: project.reference.coordinate_frame.clone(),
                records: project.reference.records.clone(),
                imports: project.imports.batches.clone(),
                metadata: project.metadata.entries.clone(),
            },
            state: project.v2.clone().unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeclarationsDocument {
    schema: String,
    records: Vec<ProjectV2Declaration>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorldsDocument {
    schema: String,
    worlds: Vec<ProjectV2World>,
    placements: Vec<ProjectV2Placement>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BindingsDocument {
    schema: String,
    bindings: Vec<ProjectV2AppearanceBinding>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AssetsDocument {
    schema: String,
    assets: Vec<ProjectV2Asset>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourcesDocument {
    schema: String,
    sources: Vec<ProjectV2Source>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EditorDocument {
    schema: String,
    legacy_entries: Vec<AuthorMetadataEntry>,
    entries: Vec<ProjectV2EditorEntry>,
}

pub(super) fn validate_v2_roles(
    roles: &BTreeMap<String, Vec<&ManifestDocument>>,
) -> Result<(), ProjectError> {
    if roles.len() != ROLE_SPECS.len() {
        return Err(ProjectError::InvalidProject(
            "unsupported v2 manifest role set",
        ));
    }
    for (role, locator, schema) in ROLE_SPECS {
        let prefix = locator
            .split_once('/')
            .ok_or(ProjectError::InvalidProject(
                "v2 role locator missing directory",
            ))?
            .0;
        let prefix = format!("{prefix}/");
        let found = require_role(roles, role, &prefix, schema)?;
        if found.len() != 1 {
            return Err(ProjectError::InvalidProject(
                "v2 role requires exactly one document",
            ));
        }
    }
    Ok(())
}

fn role_bytes<'a>(
    snapshot: &'a ProjectSnapshot,
    plan: &ProjectCapturePlan,
    role: &str,
) -> Result<&'a [u8], ProjectError> {
    let info = plan
        .manifest
        .documents
        .iter()
        .find(|entry| entry.role == role)
        .ok_or(ProjectError::InvalidProject("v2 role missing"))?;
    required(snapshot, &info.locator)
}

pub(super) fn parse_v2_snapshot(
    snapshot: &ProjectSnapshot,
    limits: ProjectEvidenceLimits,
    plan: ProjectCapturePlan,
    manifest_bytes: &[u8],
) -> Result<WorldProject, ProjectError> {
    let reference: ReferenceDocument =
        parse_strict(role_bytes(snapshot, &plan, "reference-records")?, limits)?;
    let imports: ImportDocument =
        parse_strict(role_bytes(snapshot, &plan, "import-candidates")?, limits)?;
    let declarations: DeclarationsDocument = parse_strict(
        role_bytes(snapshot, &plan, "declarative-definitions")?,
        limits,
    )?;
    let worlds: WorldsDocument =
        parse_strict(role_bytes(snapshot, &plan, "world-records")?, limits)?;
    let bindings: BindingsDocument = parse_strict(
        role_bytes(snapshot, &plan, "presentation-bindings")?,
        limits,
    )?;
    let assets: AssetsDocument =
        parse_strict(role_bytes(snapshot, &plan, "asset-records")?, limits)?;
    let sources: SourcesDocument =
        parse_strict(role_bytes(snapshot, &plan, "provenance-records")?, limits)?;
    let editor: EditorDocument =
        parse_strict(role_bytes(snapshot, &plan, "editor-records")?, limits)?;
    for (actual, expected) in [
        (&reference.schema, WORLD_PROJECT_REFERENCE_SCHEMA),
        (&imports.schema, WORLD_PROJECT_IMPORT_SCHEMA),
        (&declarations.schema, DECLARATIONS_SCHEMA),
        (&worlds.schema, WORLDS_SCHEMA),
        (&bindings.schema, PRESENTATIONS_SCHEMA),
        (&assets.schema, ASSETS_SCHEMA),
        (&sources.schema, PROVENANCE_SCHEMA),
        (&editor.schema, EDITOR_SCHEMA),
    ] {
        if actual != expected {
            return Err(ProjectError::InvalidProject(
                "v2 managed document schema mismatch",
            ));
        }
    }
    ProjectDraft {
        project_revision: plan.root.project_revision.clone(),
        package_key: plan.manifest.package_key.clone(),
        semantic_schema_version: plan.manifest.semantic_schema_version.clone(),
        licensing_metadata: plan.manifest.licensing_metadata.clone(),
        world_id: reference.world_id.clone(),
        coordinate_frame: reference.coordinate_frame.clone(),
        records: reference.records.clone(),
        imports: imports.batches.clone(),
        metadata: editor.legacy_entries.clone(),
    }
    .validate(limits)?;
    let state = ProjectV2State {
        declarations: declarations.records,
        worlds: worlds.worlds,
        placements: worlds.placements,
        appearance_bindings: bindings.bindings,
        assets: assets.assets,
        sources: sources.sources,
        editor: editor.entries,
    };
    validate_v2_state(&state, &reference.records, limits)?;
    Ok(WorldProject {
        root: plan.root,
        manifest: plan.manifest,
        lock: plan.lock,
        reference,
        imports,
        metadata: MetadataDocument {
            schema: WORLD_PROJECT_METADATA_SCHEMA.to_owned(),
            entries: editor.legacy_entries,
        },
        manifest_bytes: manifest_bytes.to_vec(),
        v2: Some(state),
    })
}

fn validate_v2_state(
    state: &ProjectV2State,
    records: &[ProjectReferenceRecord],
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    let mut identities = BTreeSet::new();
    for record in records {
        let identity = record.identity();
        identities.insert(ProjectV2DefinitionRef {
            family: ProjectV2Family::from_reference(&identity.family)?,
            key: identity.key.clone(),
            revision: identity.revision.clone(),
        });
    }
    limits.check(
        "v2 declarations",
        state.declarations.len(),
        limits.max_reference_records,
    )?;
    let mut previous: Option<(ProjectV2Family, &str)> = None;
    for declaration in &state.declarations {
        let identity = declaration.identity();
        identity.validate()?;
        let key = (declaration.family(), identity.key.as_str());
        if previous.is_some_and(|prior| prior >= key) {
            return Err(ProjectError::InvalidProject(
                "v2 declarations are not identity sorted",
            ));
        }
        previous = Some(key);
        if !identities.insert(ProjectV2DefinitionRef {
            family: key.0,
            key: identity.key.clone(),
            revision: identity.revision.clone(),
        }) {
            return Err(ProjectError::InvalidProject(
                "duplicate v2 definition identity",
            ));
        }
    }
    let require_ref = |reference: &ProjectV2DefinitionRef| -> Result<(), ProjectError> {
        reference.validate()?;
        if !identities.contains(reference) {
            return Err(ProjectError::InvalidProject(
                "unresolved v2 typed definition reference",
            ));
        }
        Ok(())
    };
    for declaration in &state.declarations {
        for (family, reference) in declaration.references() {
            if reference.family != family {
                return Err(ProjectError::InvalidProject(
                    "v2 definition reference family mismatch",
                ));
            }
            require_ref(reference)?;
        }
    }
    limits.check(
        "v2 worlds",
        state.worlds.len(),
        limits.max_reference_records,
    )?;
    let mut worlds = BTreeMap::new();
    for world in &state.worlds {
        ProductionKey::new(&world.key)?;
        decode_world_id(&world.world_id)?;
        super::super::CoordinateFrameRef::new(&world.coordinate_frame)?;
        if worlds.insert(&world.key, &world.coordinate_frame).is_some() {
            return Err(ProjectError::InvalidProject("duplicate v2 world identity"));
        }
    }
    if state
        .worlds
        .windows(2)
        .any(|pair| pair[0].key >= pair[1].key)
    {
        return Err(ProjectError::InvalidProject(
            "v2 worlds are not identity sorted",
        ));
    }
    limits.check(
        "v2 placements",
        state.placements.len(),
        limits.max_reference_records,
    )?;
    let mut keys = BTreeSet::new();
    for placement in &state.placements {
        ProductionKey::new(&placement.key)?;
        ProductionAtom::new("v2 map revision", &placement.map_revision)?;
        super::super::CoordinateFrameRef::new(&placement.coordinate_frame)?;
        require_ref(&placement.definition)?;
        if worlds.get(&placement.world).map(|frame| frame.as_str())
            != Some(placement.coordinate_frame.as_str())
        {
            return Err(ProjectError::InvalidProject(
                "v2 placement world/frame mismatch",
            ));
        }
        if !keys.insert(&placement.key) {
            return Err(ProjectError::InvalidProject(
                "duplicate v2 placement identity",
            ));
        }
    }
    if state
        .placements
        .windows(2)
        .any(|pair| pair[0].key >= pair[1].key)
    {
        return Err(ProjectError::InvalidProject(
            "v2 placements are not identity sorted",
        ));
    }
    limits.check(
        "v2 assets",
        state.assets.len(),
        limits.max_reference_records,
    )?;
    let mut assets = BTreeSet::new();
    for asset in &state.assets {
        ProductionKey::new(&asset.identity.key)?;
        DefinitionRevisionRef::new(&asset.identity.revision)?;
        Sha256HexDigest::new(&asset.sha256)?;
        if !assets.insert(&asset.identity) {
            return Err(ProjectError::InvalidProject("duplicate v2 asset"));
        }
    }
    if state
        .assets
        .windows(2)
        .any(|pair| pair[0].identity >= pair[1].identity)
    {
        return Err(ProjectError::InvalidProject(
            "v2 assets are not identity sorted",
        ));
    }
    limits.check(
        "v2 appearance bindings",
        state.appearance_bindings.len(),
        limits.max_reference_records,
    )?;
    let mut bound = BTreeSet::new();
    for binding in &state.appearance_bindings {
        if binding.presentation.family != ProjectV2Family::Presentation {
            return Err(ProjectError::InvalidProject(
                "appearance binding requires Presentation",
            ));
        }
        require_ref(&binding.presentation)?;
        if !assets.contains(&binding.asset) {
            return Err(ProjectError::InvalidProject("appearance asset is missing"));
        }
        if !bound.insert(&binding.presentation) {
            return Err(ProjectError::InvalidProject("duplicate appearance binding"));
        }
    }
    if state
        .appearance_bindings
        .windows(2)
        .any(|pair| pair[0].presentation >= pair[1].presentation)
    {
        return Err(ProjectError::InvalidProject(
            "appearance bindings are not identity sorted",
        ));
    }
    limits.check("v2 sources", state.sources.len(), limits.max_import_records)?;
    let mut source_keys = BTreeSet::new();
    for source in &state.sources {
        ProductionKey::new(&source.key)?;
        ProductionAtom::new("v2 source revision", &source.revision)?;
        Sha256HexDigest::new(&source.sha256)?;
        if !source_keys.insert((&source.key, &source.revision)) {
            return Err(ProjectError::InvalidProject("duplicate v2 source"));
        }
    }
    if state
        .sources
        .windows(2)
        .any(|pair| (&pair[0].key, &pair[0].revision) >= (&pair[1].key, &pair[1].revision))
    {
        return Err(ProjectError::InvalidProject(
            "v2 sources are not identity sorted",
        ));
    }
    limits.check(
        "v2 editor entries",
        state.editor.len(),
        limits.max_reference_records,
    )?;
    let mut alias_keys = BTreeSet::new();
    let mut targets = BTreeSet::new();
    for entry in &state.editor {
        require_ref(&entry.target)?;
        if !targets.insert(&entry.target) {
            return Err(ProjectError::InvalidProject("duplicate v2 editor target"));
        }
        if entry.aliases.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(ProjectError::InvalidProject(
                "v2 editor aliases are not sorted and unique",
            ));
        }
        for alias in &entry.aliases {
            if alias.trim() != alias
                || alias.is_empty()
                || alias.chars().any(char::is_control)
                || !alias_keys.insert((entry.target.family, alias))
            {
                return Err(ProjectError::InvalidProject(
                    "invalid or duplicate v2 alias",
                ));
            }
        }
        for tag in &entry.tags {
            ProductionKey::new(tag)?;
        }
        if entry.tags.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(ProjectError::InvalidProject(
                "v2 editor tags are not sorted and unique",
            ));
        }
    }
    if state
        .editor
        .windows(2)
        .any(|pair| pair[0].target >= pair[1].target)
    {
        return Err(ProjectError::InvalidProject(
            "v2 editor entries are not identity sorted",
        ));
    }
    Ok(())
}

impl CanonicalProjectDocuments {
    pub fn from_v2_draft(
        mut draft: ProjectV2Draft,
        limits: ProjectEvidenceLimits,
    ) -> Result<Self, ProjectError> {
        let limits = limits.validate()?;
        draft.core.records = sorted_records(draft.core.records);
        draft.core.imports = sorted_imports(draft.core.imports);
        draft.core.metadata = sorted_metadata(draft.core.metadata);
        draft
            .state
            .declarations
            .sort_by(|a, b| (a.family(), &a.identity().key).cmp(&(b.family(), &b.identity().key)));
        draft.state.worlds.sort_by(|a, b| a.key.cmp(&b.key));
        draft.state.placements.sort_by(|a, b| a.key.cmp(&b.key));
        draft
            .state
            .assets
            .sort_by(|a, b| a.identity.cmp(&b.identity));
        draft
            .state
            .appearance_bindings
            .sort_by(|a, b| a.presentation.cmp(&b.presentation));
        draft
            .state
            .sources
            .sort_by(|a, b| (&a.key, &a.revision).cmp(&(&b.key, &b.revision)));
        draft.state.editor.sort_by(|a, b| a.target.cmp(&b.target));
        for entry in &mut draft.state.editor {
            entry.aliases.sort();
            entry.tags.sort();
        }
        draft.core.validate(limits)?;
        validate_v2_state(&draft.state, &draft.core.records, limits)?;
        limits.check(
            "project documents",
            3 + ROLE_SPECS.len(),
            limits.max_documents,
        )?;
        let mut budget = CanonicalWriteBudget::new(limits);
        let mut managed = BTreeMap::new();
        managed.insert(
            ROLE_SPECS[0].1.to_owned(),
            budget.encode(&ReferenceDocument {
                schema: WORLD_PROJECT_REFERENCE_SCHEMA.to_owned(),
                world_id: draft.core.world_id.clone(),
                coordinate_frame: draft.core.coordinate_frame.clone(),
                records: draft.core.records,
            })?,
        );
        managed.insert(
            ROLE_SPECS[1].1.to_owned(),
            budget.encode(&DeclarationsDocument {
                schema: DECLARATIONS_SCHEMA.to_owned(),
                records: draft.state.declarations,
            })?,
        );
        managed.insert(
            ROLE_SPECS[2].1.to_owned(),
            budget.encode(&WorldsDocument {
                schema: WORLDS_SCHEMA.to_owned(),
                worlds: draft.state.worlds,
                placements: draft.state.placements,
            })?,
        );
        managed.insert(
            ROLE_SPECS[3].1.to_owned(),
            budget.encode(&BindingsDocument {
                schema: PRESENTATIONS_SCHEMA.to_owned(),
                bindings: draft.state.appearance_bindings,
            })?,
        );
        managed.insert(
            ROLE_SPECS[4].1.to_owned(),
            budget.encode(&AssetsDocument {
                schema: ASSETS_SCHEMA.to_owned(),
                assets: draft.state.assets,
            })?,
        );
        managed.insert(
            ROLE_SPECS[5].1.to_owned(),
            budget.encode(&ImportDocument {
                schema: WORLD_PROJECT_IMPORT_SCHEMA.to_owned(),
                batches: draft.core.imports,
            })?,
        );
        managed.insert(
            ROLE_SPECS[6].1.to_owned(),
            budget.encode(&SourcesDocument {
                schema: PROVENANCE_SCHEMA.to_owned(),
                sources: draft.state.sources,
            })?,
        );
        managed.insert(
            ROLE_SPECS[7].1.to_owned(),
            budget.encode(&EditorDocument {
                schema: EDITOR_SCHEMA.to_owned(),
                legacy_entries: draft.core.metadata,
                entries: draft.state.editor,
            })?,
        );
        let mut inventory = Vec::new();
        for (role, locator, schema) in ROLE_SPECS {
            validate_locator(locator, limits)?;
            let bytes = managed
                .get(locator)
                .ok_or(ProjectError::InvalidProject("missing v2 canonical role"))?;
            inventory.push(ManifestDocument {
                role: role.to_owned(),
                schema: schema.to_owned(),
                locator: locator.to_owned(),
                byte_length: bytes.len(),
                sha256: digest_hex(bytes),
            });
        }
        inventory.sort_by(|a, b| a.locator.cmp(&b.locator));
        let manifest = ManifestDocumentRoot {
            schema: WORLD_PROJECT_V2_MANIFEST_SCHEMA.to_owned(),
            package_key: draft.core.package_key,
            package_revision: draft.core.project_revision.clone(),
            semantic_schema_version: draft.core.semantic_schema_version,
            licensing_metadata: draft.core.licensing_metadata,
            required_features: Vec::new(),
            optional_features: Vec::new(),
            documents: inventory,
        };
        let manifest_bytes = budget.encode(&manifest)?;
        let package = package_binding(&manifest, &manifest_bytes)?;
        let lock = LockDocument {
            schema: WORLD_PROJECT_V2_LOCK_SCHEMA.to_owned(),
            project_revision: draft.core.project_revision.clone(),
            revision_digest_token: format!("lock:{}", draft.core.project_revision),
            entries: vec![LockEntryDocument {
                package_key: manifest.package_key.clone(),
                package_revision: manifest.package_revision.clone(),
                package_provenance_digest: package.package_provenance_digest()?.as_str().to_owned(),
                floating: false,
                dependency: false,
            }],
        };
        let lock_bytes = budget.encode(&lock)?;
        let root = RootDocument {
            schema: WORLD_PROJECT_V2_ROOT_SCHEMA.to_owned(),
            source_profile: WORLD_PROJECT_V2_SOURCE_PROFILE.to_owned(),
            project_revision: draft.core.project_revision,
            manifest_locator: MANIFEST_LOCATOR.to_owned(),
            manifest_sha256: digest_hex(&manifest_bytes),
            content_lock_locator: LOCK_LOCATOR.to_owned(),
            content_lock_sha256: digest_hex(&lock_bytes),
        };
        managed.insert(MANIFEST_LOCATOR.to_owned(), manifest_bytes);
        managed.insert(LOCK_LOCATOR.to_owned(), lock_bytes);
        managed.insert(PROJECT_LOCATOR.to_owned(), budget.encode(&root)?);
        let snapshot = ProjectSnapshot::new(managed.into_iter(), limits)?;
        snapshot.parse(limits)?;
        Ok(Self {
            documents: snapshot.documents,
        })
    }
}
