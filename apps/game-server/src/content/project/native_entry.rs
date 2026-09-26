//! Native entry-room source consumer (`NATIVE_ENTRY_SOURCE_QUALIFICATION_V1`, #937) with the
//! owner-accepted product bindings and first-slice limits of `NATIVE-ENTRY-ROOM-PRODUCT-BINDINGS-V1`
//! (#940).
//!
//! The variant is admitted only through its explicit API; ordinary v1/v2 capture refuses it. The
//! closed `native_first_entry` overlay plus the exact typed records of the same admitted bytes lower
//! to one [`FirstProductionContentSource`]. Nothing here activates Content, issues a WorldId or
//! grants position/control authority.

use super::*;
use crate::content::{
    CollisionClass, DurableMigrationClass, EffectFamily, EligibilityScope,
    FIRST_PRODUCTION_CAPABILITY_PROFILE, FIRST_PRODUCTION_PROFILE_ID, FirstProductionAbility,
    FirstProductionArea, FirstProductionBehavior, FirstProductionCell,
    FirstProductionContentSource, FirstProductionCreature, FirstProductionEffect,
    FirstProductionFormulaProfile, FirstProductionItem, FirstProductionLootEntry,
    FirstProductionLootTable, FirstProductionPresentation, FirstProductionRegion,
    FirstProductionRelocation, FirstProductionRevisionSet, FirstProductionRngContext,
    FirstProductionSpawn, FirstProductionTerrain, FirstProductionXpDefinition, MultiplicityClass,
    ProjectFilesystemLimits, SpawnRecoveryClass,
};

pub const NATIVE_ENTRY_SOURCE_PROFILE: &str =
    "OTERYN_WORLD_PROJECT_SOURCE_PROFILE/v2-native-entry-qualification-1";
pub const NATIVE_ENTRY_DECLARATIONS_SCHEMA: &str =
    "OTERYN_WORLD_PROJECT_DECLARATIONS/v2-native-entry-qualification-1";
/// Licensing metadata required by #940 §3.
pub const NATIVE_ENTRY_LICENSING: &str = "oteryn-original-preproduction";
pub const NATIVE_ENTRY_COORDINATE_PROFILE: &str = "oteryn-world-spatial-v1";
pub const NATIVE_ENTRY_CONTRACT_REVISION: u32 = 1;
/// The one entry-room has exactly these three cells (#935, #937 §4).
pub const NATIVE_ENTRY_CELLS: usize = 3;
pub const NATIVE_ENTRY_PRESENTATIONS: usize = 3;

/// `PREPRODUCTION_FIRST_SLICE` source-pipeline limits accepted in #940 §4. These are finite
/// fail-closed bounds for the native entry-room variant only, never a production default.
pub const fn native_entry_first_slice_limits() -> ProjectFilesystemLimits {
    ProjectFilesystemLimits {
        project: ProjectEvidenceLimits {
            max_documents: 11,
            max_document_bytes: 64 * 1024,
            max_total_bytes: 256 * 1024,
            max_json_depth: 16,
            max_decoded_fields: 4096,
            max_string_bytes: 16 * 1024,
            max_locator_bytes: 128,
            max_locator_segments: 4,
            max_reference_records: 32,
            // Smallest accepted limits; the qualifier separately requires zero.
            max_import_records: 1,
            max_reimport_states: 1,
        },
        max_entries_per_directory_scan: 32,
        max_total_directory_entries_scanned: 192,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeFirstEntryDocument {
    pub world_key: String,
    pub frame: NativeEntryFrame,
    pub revisions: NativeEntryRevisions,
    pub region: NativeEntryKey,
    pub cells: Vec<NativeEntryCell>,
    pub relocation: NativeEntryRelocation,
    pub behavior: NativeEntryPolicyBinding,
    pub presentations: Vec<NativeEntryPresentation>,
    pub creature: NativeEntryPolicyBinding,
    pub spawn: NativeEntrySpawn,
    pub ability: NativeEntryPresented,
    pub item: NativeEntryPresented,
    pub loot_table: NativeEntryLootTable,
    pub xp: NativeEntryXp,
    pub rng: NativeEntryRng,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEntryFrame {
    pub coordinate_profile: String,
    pub contract_revision: u32,
    pub coordinate_frame: String,
    pub origin: NativeEntryOrigin,
    pub x_direction: NativeEntryXDirection,
    pub y_direction: NativeEntryYDirection,
    pub higher_floor: NativeEntryHigherFloor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEntryOrigin {
    pub x: i32,
    pub y: i32,
    pub floor: i16,
}

/// Canonical directions only (#937 §4); every other axis refuses at parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NativeEntryXDirection {
    East,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NativeEntryYDirection {
    South,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NativeEntryHigherFloor {
    Up,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEntryRevisions {
    pub content: String,
    pub map: String,
    pub ruleset: String,
    pub world_policy: String,
    pub compiler: String,
    pub canonicalization: String,
    pub sim_profile: String,
    pub profile_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEntryKey {
    pub key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NativeEntryCollision {
    Walkable,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEntryCell {
    pub placement_key: String,
    pub region_key: String,
    pub collision: NativeEntryCollision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEntryRelocation {
    pub key: String,
    pub from_cell: String,
    pub to_cell: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEntryPolicyBinding {
    pub definition: ProjectV2DefinitionRef,
    pub policy_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEntryPresentation {
    pub definition: ProjectV2DefinitionRef,
    pub metadata_token: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NativeEntryRecovery {
    EphemeralScopeReset,
    CheckpointedRuntimeContinuity,
    DurableEventOccurrence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NativeEntryMultiplicity {
    ChannelLocalRepeatable,
    ChannelLocalSharedEligibility,
    WorldScopedUnique,
    ExplicitEventPolicyRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NativeEntryEligibility {
    CharacterWorld,
    AccountWorld,
    World,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEntrySpawn {
    pub key: String,
    pub creature: ProjectV2DefinitionRef,
    pub behavior: ProjectV2DefinitionRef,
    pub cell_key: String,
    pub population_limit: u16,
    pub recovery: NativeEntryRecovery,
    pub multiplicity: NativeEntryMultiplicity,
    pub eligibility_scope: NativeEntryEligibility,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEntryPresented {
    pub definition: ProjectV2DefinitionRef,
    pub presentation: ProjectV2DefinitionRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEntryLootEntry {
    pub key: String,
    pub item: ProjectV2DefinitionRef,
    pub rng_purpose_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEntryLootTable {
    pub key: String,
    pub entry: NativeEntryLootEntry,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEntryXp {
    pub key: String,
    pub formula: ProjectV2DefinitionRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEntryRng {
    pub profile_revision: String,
    pub purpose_key: String,
}

/// One admitted native entry-room project and its qualified FirstProduction source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeEntryProject {
    project: WorldProject,
    source: FirstProductionContentSource,
}

impl NativeEntryProject {
    pub fn project(&self) -> &WorldProject {
        &self.project
    }

    pub fn source(&self) -> &FirstProductionContentSource {
        &self.source
    }

    pub(super) fn qualify(
        project: WorldProject,
        overlay: NativeFirstEntryDocument,
    ) -> Result<Self, ProjectError> {
        let source = lower(&project, &overlay)?;
        Ok(Self { project, source })
    }
}

fn refuse<T>(reason: &'static str) -> Result<T, ProjectError> {
    Err(ProjectError::InvalidProject(reason))
}

fn same(reference: &DefinitionIdentityDocument, expected: &ProjectV2DefinitionRef) -> bool {
    ProjectV2Family::from_reference(&reference.family).ok() == Some(expected.family)
        && reference.key == expected.key
        && reference.revision == expected.revision
}

fn find_record<'a>(
    records: &'a [ProjectReferenceRecord],
    expected: &ProjectV2DefinitionRef,
) -> Result<&'a ProjectReferenceRecord, ProjectError> {
    let mut found = records
        .iter()
        .filter(|record| same(record.identity(), expected));
    match (found.next(), found.next()) {
        (Some(record), None) => Ok(record),
        _ => refuse("native entry definition reference does not resolve exactly"),
    }
}

fn require_family(
    reference: &ProjectV2DefinitionRef,
    family: ProjectV2Family,
) -> Result<(), ProjectError> {
    if reference.family != family {
        return refuse("native entry definition reference has the wrong family");
    }
    Ok(())
}

fn require_generic(
    records: &[ProjectReferenceRecord],
    reference: &ProjectV2DefinitionRef,
    family: ProjectV2Family,
) -> Result<ProductionKey, ProjectError> {
    require_family(reference, family)?;
    match find_record(records, reference)? {
        ProjectReferenceRecord::Generic { .. } => Ok(ProductionKey::new(&reference.key)?),
        _ => refuse("native entry definition has the wrong record kind"),
    }
}

fn reference_matches(
    reference: &DefinitionReferenceDocument,
    expected: &ProjectV2DefinitionRef,
) -> bool {
    same(
        &DefinitionIdentityDocument {
            family: reference.family.clone(),
            key: reference.key.clone(),
            revision: reference.revision.clone(),
        },
        expected,
    )
}

fn atom(field: &'static str, value: &str) -> Result<ProductionAtom, ProjectError> {
    Ok(ProductionAtom::new(field, value)?)
}

fn in_bounds(bounds: &ProjectV2Bounds, floors: &[i16], x: i32, y: i32, floor: i16) -> bool {
    let (x, y) = (i64::from(x), i64::from(y));
    x >= bounds.min_x
        && x < bounds.max_x_exclusive
        && y >= bounds.min_y
        && y < bounds.max_y_exclusive
        && floors.contains(&floor)
}

#[allow(clippy::too_many_lines)]
fn lower(
    project: &WorldProject,
    overlay: &NativeFirstEntryDocument,
) -> Result<FirstProductionContentSource, ProjectError> {
    let records = &project.reference.records;
    let state = project.v2.as_ref().ok_or(ProjectError::InvalidProject(
        "native entry requires v2 state",
    ))?;

    // Package, licensing and import custody.
    if project.manifest.licensing_metadata != NATIVE_ENTRY_LICENSING {
        return refuse("native entry licensing metadata mismatch");
    }
    if project
        .imports
        .batches
        .iter()
        .any(|batch| !batch.candidates.is_empty() || !batch.reimport_states.is_empty())
        || !project.imports.batches.is_empty()
    {
        return refuse("native entry admits no import candidates or reimport states");
    }
    if !state.item_authoring.is_empty() || !state.authoring_profiles.is_empty() {
        return refuse("native entry admits no authoring overlays");
    }

    // World and frame (#937 §4).
    let [world] = state.worlds.as_slice() else {
        return refuse("native entry requires exactly one World");
    };
    let frame = &overlay.frame;
    if world.key != overlay.world_key
        || world.world_id != project.reference.world_id
        || world.coordinate_frame != project.reference.coordinate_frame
        || world.coordinate_frame != frame.coordinate_frame
    {
        return refuse("native entry World or frame binding mismatch");
    }
    if frame.coordinate_profile != NATIVE_ENTRY_COORDINATE_PROFILE
        || frame.contract_revision != NATIVE_ENTRY_CONTRACT_REVISION
    {
        return refuse("native entry coordinate contract mismatch");
    }
    let origin = frame.origin;
    if !in_bounds(
        &world.bounds,
        &world.floors,
        origin.x,
        origin.y,
        origin.floor,
    ) {
        return refuse("native entry origin outside the selected World");
    }

    // Region and three cells bijective with three Terrain placements.
    let region = ProductionKey::new(&overlay.region.key)?;
    if overlay.cells.len() != NATIVE_ENTRY_CELLS || state.placements.len() != NATIVE_ENTRY_CELLS {
        return refuse("native entry requires exactly three cells and placements");
    }
    let [area_declaration] = state.declarations.as_slice() else {
        return refuse("native entry requires exactly one Area declaration");
    };
    let ProjectV2Declaration::Area {
        identity: area_identity,
        parent: None,
        ..
    } = area_declaration
    else {
        return refuse("native entry declaration must be one parentless Area");
    };
    let terrain_ref = &state.placements[0].definition;
    require_family(terrain_ref, ProjectV2Family::Terrain)?;
    let terrain = require_generic(records, terrain_ref, ProjectV2Family::Terrain)?;
    let area = ProductionKey::new(&area_identity.key)?;
    let mut cells = Vec::with_capacity(NATIVE_ENTRY_CELLS);
    let mut coordinates = BTreeSet::new();
    let mut cell_keys = BTreeSet::new();
    for cell in &overlay.cells {
        let mut matching = state
            .placements
            .iter()
            .filter(|placement| placement.key == cell.placement_key);
        let (Some(placement), None) = (matching.next(), matching.next()) else {
            return refuse("native entry cell does not match exactly one placement");
        };
        let area_ok = placement.area.as_ref().is_some_and(|area_ref| {
            area_ref.family == ProjectV2Family::Area
                && area_ref.key == area_identity.key
                && area_ref.revision == area_identity.revision
        });
        if placement.world != world.key
            || placement.coordinate_frame != frame.coordinate_frame
            || placement.map_revision != overlay.revisions.map
            || placement.definition != *terrain_ref
            || !area_ok
            || placement.document.is_some()
            || placement.parent_placement.is_some()
        {
            return refuse("native entry placement binding mismatch");
        }
        if !in_bounds(
            &world.bounds,
            &world.floors,
            placement.x,
            placement.y,
            placement.floor,
        ) || !coordinates.insert((placement.x, placement.y, placement.floor))
        {
            return refuse("native entry cell coordinate outside the World or duplicated");
        }
        if cell.region_key != overlay.region.key || !cell_keys.insert(cell.placement_key.clone()) {
            return refuse("native entry cell region mismatch or duplicate cell");
        }
        cells.push(FirstProductionCell {
            key: ProductionKey::new(&cell.placement_key)?,
            region_key: region.clone(),
            area_key: area.clone(),
            terrain_key: terrain.clone(),
            x: placement.x,
            y: placement.y,
            z: placement.floor,
            collision: match cell.collision {
                NativeEntryCollision::Walkable => CollisionClass::Walkable,
                NativeEntryCollision::Blocked => CollisionClass::Blocked,
            },
        });
    }

    let relocation = &overlay.relocation;
    if relocation.from_cell == relocation.to_cell
        || !cell_keys.contains(&relocation.from_cell)
        || !cell_keys.contains(&relocation.to_cell)
    {
        return refuse("native entry relocation endpoints must be two of the three cells");
    }

    // Behavior, presentations and Creature.
    let behavior_ref = &overlay.behavior.definition;
    let behavior = require_generic(records, behavior_ref, ProjectV2Family::Behavior)?;
    if overlay.presentations.len() != NATIVE_ENTRY_PRESENTATIONS {
        return refuse("native entry requires exactly three presentations");
    }
    let mut presentations = Vec::with_capacity(NATIVE_ENTRY_PRESENTATIONS);
    for presentation in &overlay.presentations {
        presentations.push(FirstProductionPresentation {
            key: require_generic(
                records,
                &presentation.definition,
                ProjectV2Family::Presentation,
            )?,
            metadata_token: atom("presentation metadata", &presentation.metadata_token)?,
        });
    }
    let presentation_refs: Vec<&ProjectV2DefinitionRef> = overlay
        .presentations
        .iter()
        .map(|presentation| &presentation.definition)
        .collect();

    let creature_ref = &overlay.creature.definition;
    require_family(creature_ref, ProjectV2Family::Creature)?;
    let ProjectReferenceRecord::Creature {
        presentation: creature_presentation,
        behavior: creature_behavior,
        loot: None,
        ..
    } = find_record(records, creature_ref)?
    else {
        return refuse("native entry Creature must be a Creature record without Reference loot");
    };
    if !reference_matches(creature_behavior, behavior_ref) {
        return refuse("native entry Creature behavior mismatch");
    }
    let Some(creature_presentation_ref) = presentation_refs
        .iter()
        .find(|candidate| reference_matches(creature_presentation, candidate))
    else {
        return refuse("native entry Creature presentation is not a selected presentation");
    };

    // Spawn.
    let spawn = &overlay.spawn;
    if spawn.creature != *creature_ref
        || spawn.behavior != *behavior_ref
        || !cell_keys.contains(&spawn.cell_key)
    {
        return refuse("native entry spawn binding mismatch");
    }

    // Ability -> exactly one Damage Effect -> Formula; XP shares that Formula.
    let ability_ref = &overlay.ability.definition;
    require_family(ability_ref, ProjectV2Family::Ability)?;
    let ProjectReferenceRecord::Ability { effects, .. } = find_record(records, ability_ref)? else {
        return refuse("native entry Ability must be an Ability record");
    };
    let [effect_reference] = effects.as_slice() else {
        return refuse("native entry Ability must have exactly one Effect");
    };
    let effect_record = records
        .iter()
        .find(|record| {
            let identity = record.identity();
            identity.family == effect_reference.family
                && identity.key == effect_reference.key
                && identity.revision == effect_reference.revision
        })
        .ok_or(ProjectError::InvalidProject(
            "native entry Effect does not resolve",
        ))?;
    let ProjectReferenceRecord::Effect {
        identity: effect_identity,
        effect_family: EffectFamilyDocument::Damage,
        formula: formula_reference,
        ..
    } = effect_record
    else {
        return refuse("native entry Effect must be a Damage Effect");
    };
    if !reference_matches(formula_reference, &overlay.xp.formula) {
        return refuse("native entry XP must use the single Effect formula profile");
    }
    require_family(&overlay.xp.formula, ProjectV2Family::Formula)?;
    let ProjectReferenceRecord::Formula { .. } = find_record(records, &overlay.xp.formula)? else {
        return refuse("native entry formula must be a Formula record");
    };
    if !presentation_refs.contains(&&overlay.ability.presentation) {
        return refuse("native entry Ability presentation is not a selected presentation");
    }

    // Item and loot.
    let item_ref = &overlay.item.definition;
    require_family(item_ref, ProjectV2Family::Item)?;
    let ProjectReferenceRecord::Item {
        materializable: true,
        ..
    } = find_record(records, item_ref)?
    else {
        return refuse("native entry Item must be a materializable Item record");
    };
    if !presentation_refs.contains(&&overlay.item.presentation) {
        return refuse("native entry Item presentation is not a selected presentation");
    }
    let loot_entry = &overlay.loot_table.entry;
    if loot_entry.item != *item_ref || loot_entry.rng_purpose_key != overlay.rng.purpose_key {
        return refuse("native entry loot entry binding mismatch");
    }

    // Every selected presentation is used exactly by Creature, Ability or Item.
    let used: BTreeSet<&ProjectV2DefinitionRef> = [
        *creature_presentation_ref,
        &overlay.ability.presentation,
        &overlay.item.presentation,
    ]
    .into_iter()
    .collect();
    if used.len() != NATIVE_ENTRY_PRESENTATIONS {
        return refuse(
            "native entry presentations must be distinct per Creature, Ability and Item",
        );
    }

    // No Reference record outside the selected graph.
    let graph_refs: Vec<&ProjectV2DefinitionRef> = [
        terrain_ref,
        behavior_ref,
        creature_ref,
        ability_ref,
        &overlay.xp.formula,
        item_ref,
    ]
    .into_iter()
    .chain(presentation_refs.iter().copied())
    .collect();
    for record in records {
        let identity = record.identity();
        let in_graph = graph_refs.iter().any(|expected| same(identity, expected))
            || (identity.family == effect_identity.family
                && identity.key == effect_identity.key
                && identity.revision == effect_identity.revision);
        if !in_graph {
            return refuse("native entry Reference record outside the selected graph");
        }
    }

    let revisions = &overlay.revisions;
    if revisions.profile_revision != FIRST_PRODUCTION_PROFILE_ID {
        return refuse("native entry profile revision must be the FirstProduction profile");
    }
    let package_manifest = package_binding(&project.manifest, &project.manifest_bytes)?;
    let content_lock = content_lock_binding(&project.lock)?;
    let formula = ProductionKey::new(&overlay.xp.formula.key)?;
    let effect = ProductionKey::new(&effect_identity.key)?;
    let rng_purpose = ProductionKey::new(&overlay.rng.purpose_key)?;
    let creature = ProductionKey::new(&creature_ref.key)?;
    let item = ProductionKey::new(&item_ref.key)?;

    Ok(FirstProductionContentSource {
        package_manifest,
        content_lock,
        world_id: decode_world_id(&project.reference.world_id)?,
        revisions: FirstProductionRevisionSet {
            content: atom("content revision", &revisions.content)?,
            map: atom("map revision", &revisions.map)?,
            ruleset: atom("ruleset revision", &revisions.ruleset)?,
            world_policy: atom("world policy revision", &revisions.world_policy)?,
            compiler: atom("compiler revision", &revisions.compiler)?,
            canonicalization: atom("canonicalization revision", &revisions.canonicalization)?,
            sim_profile: atom("sim profile", &revisions.sim_profile)?,
            profile_revision: atom(
                "first-production profile revision",
                &revisions.profile_revision,
            )?,
        },
        capability_profile: atom("capability profile", FIRST_PRODUCTION_CAPABILITY_PROFILE)?,
        migration_class: DurableMigrationClass::CompatibleNoMigration,
        regions: vec![FirstProductionRegion { key: region }],
        areas: vec![FirstProductionArea { key: area }],
        terrains: vec![FirstProductionTerrain { key: terrain }],
        cells,
        relocations: vec![FirstProductionRelocation {
            key: ProductionKey::new(&relocation.key)?,
            from_cell: ProductionKey::new(&relocation.from_cell)?,
            to_cell: ProductionKey::new(&relocation.to_cell)?,
        }],
        behaviors: vec![FirstProductionBehavior {
            key: behavior.clone(),
            policy_revision: atom("behavior policy", &overlay.behavior.policy_revision)?,
        }],
        presentations,
        creatures: vec![FirstProductionCreature {
            key: creature.clone(),
            behavior_key: behavior.clone(),
            presentation_key: ProductionKey::new(&creature_presentation_ref.key)?,
            policy_revision: atom("creature policy", &overlay.creature.policy_revision)?,
        }],
        spawns: vec![FirstProductionSpawn {
            key: ProductionKey::new(&spawn.key)?,
            creature_key: creature,
            behavior_key: behavior,
            cell_key: ProductionKey::new(&spawn.cell_key)?,
            population_limit: spawn.population_limit,
            recovery: match spawn.recovery {
                NativeEntryRecovery::EphemeralScopeReset => SpawnRecoveryClass::EphemeralScopeReset,
                NativeEntryRecovery::CheckpointedRuntimeContinuity => {
                    SpawnRecoveryClass::CheckpointedRuntimeContinuity
                }
                NativeEntryRecovery::DurableEventOccurrence => {
                    SpawnRecoveryClass::DurableEventOccurrence
                }
            },
            multiplicity: match spawn.multiplicity {
                NativeEntryMultiplicity::ChannelLocalRepeatable => {
                    MultiplicityClass::ChannelLocalRepeatable
                }
                NativeEntryMultiplicity::ChannelLocalSharedEligibility => {
                    MultiplicityClass::ChannelLocalSharedEligibility
                }
                NativeEntryMultiplicity::WorldScopedUnique => MultiplicityClass::WorldScopedUnique,
                NativeEntryMultiplicity::ExplicitEventPolicyRequired => {
                    MultiplicityClass::ExplicitEventPolicyRequired
                }
            },
            eligibility_scope: match spawn.eligibility_scope {
                NativeEntryEligibility::CharacterWorld => EligibilityScope::CharacterWorld,
                NativeEntryEligibility::AccountWorld => EligibilityScope::AccountWorld,
                NativeEntryEligibility::World => EligibilityScope::World,
            },
        }],
        formula_profiles: vec![FirstProductionFormulaProfile {
            key: formula.clone(),
        }],
        effects: vec![FirstProductionEffect {
            key: effect.clone(),
            family: EffectFamily::Damage,
            formula_profile_key: formula.clone(),
        }],
        abilities: vec![FirstProductionAbility {
            key: ProductionKey::new(&ability_ref.key)?,
            effect_key: effect,
            presentation_key: ProductionKey::new(&overlay.ability.presentation.key)?,
        }],
        items: vec![FirstProductionItem {
            key: item.clone(),
            presentation_key: ProductionKey::new(&overlay.item.presentation.key)?,
            materializable: true,
        }],
        loot_tables: vec![FirstProductionLootTable {
            key: ProductionKey::new(&overlay.loot_table.key)?,
            entries: vec![FirstProductionLootEntry {
                key: ProductionKey::new(&loot_entry.key)?,
                item_key: item,
                rng_purpose_key: rng_purpose.clone(),
            }],
        }],
        xp_definitions: vec![FirstProductionXpDefinition {
            key: ProductionKey::new(&overlay.xp.key)?,
            formula_profile_key: formula,
        }],
        rng: FirstProductionRngContext {
            profile_revision: atom("rng profile", &overlay.rng.profile_revision)?,
            purpose_keys: vec![rng_purpose],
        },
    })
}
