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
    CanonicalReferencePlayableContent, CollisionClass, DefinitionFamily, DurableMigrationClass,
    EffectFamily, EligibilityScope, FIRST_PRODUCTION_CAPABILITY_PROFILE,
    FIRST_PRODUCTION_PROFILE_ID, FirstProductionAbility, FirstProductionArea,
    FirstProductionBehavior, FirstProductionCell, FirstProductionCompileTarget,
    FirstProductionContentSource, FirstProductionCreature, FirstProductionEffect,
    FirstProductionFormulaProfile, FirstProductionItem, FirstProductionLootEntry,
    FirstProductionLootTable, FirstProductionPresentation, FirstProductionRegion,
    FirstProductionRelocation, FirstProductionRevisionSet, FirstProductionRngContext,
    FirstProductionSpawn, FirstProductionTerrain, FirstProductionXpDefinition,
    LocalObjectCollisionPresence, MultiplicityClass, OwnerCapabilityRequirement,
    ProjectFilesystemLimits, REFERENCE_PLAYABLE_CAPABILITY_PROFILE,
    REFERENCE_PLAYABLE_CONTENT_PROFILE_ID, ReferenceDefinition, ReferenceDefinitionKind,
    ReferencePlayableContentSource, SpawnRecoveryClass, TransitionBinding, TransitionKey,
    compile_first_production, link_reference_playable,
};

pub const NATIVE_ENTRY_SOURCE_PROFILE: &str =
    "OTERYN_WORLD_PROJECT_SOURCE_PROFILE/v2-native-entry-qualification-1";
pub const NATIVE_ENTRY_DECLARATIONS_SCHEMA: &str =
    "OTERYN_WORLD_PROJECT_DECLARATIONS/v2-native-entry-qualification-1";
/// Explicit candidate ground/source-metadata admission, preserving entry-r1 geometry.
pub const NATIVE_SPELL_ENTRY_SOURCE_PROFILE: &str =
    "OTERYN_WORLD_PROJECT_SOURCE_PROFILE/v2-native-spell-entry-qualification-2";
pub const NATIVE_SPELL_ENTRY_DECLARATIONS_SCHEMA: &str =
    "OTERYN_WORLD_PROJECT_DECLARATIONS/v2-native-spell-entry-qualification-2";
pub const NATIVE_GROUND_SOURCE_REVISION: &str = "99902524e052f37574194466c2949c576e4ab269";
pub const NATIVE_GROUND_PROOF_SHA256: &str =
    "dbd056f27908b7172cb2a46a4df4fe849acca5acfce02f129c71145c6c646a95";
pub const NATIVE_GROUND_SOURCE_KEY: &str = "oteryn:source/canary-native-stone-ground";
pub const fn native_spell_entry_candidate_limits() -> ProjectFilesystemLimits {
    native_entry_first_slice_limits()
}

/// Licensing metadata required by #940 §3.
pub const NATIVE_ENTRY_LICENSING: &str = "oteryn-original-preproduction";
pub const NATIVE_ENTRY_COORDINATE_PROFILE: &str = "oteryn-world-spatial-v1";
pub const NATIVE_ENTRY_CONTRACT_REVISION: u32 = 1;
/// The one entry-room has exactly these three cells (#935, #937 §4).
pub const NATIVE_ENTRY_CELLS: usize = 3;
pub const NATIVE_ENTRY_PRESENTATIONS: usize = 3;
/// The one entry-room door (#162 comment 5865792400, owner decision A4-a): exactly one 4th
/// walkable cell, adjacent to the accepted three, carries exactly one typed door overlay.
pub const NATIVE_ENTRY_DOOR_CELLS: usize = 1;

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
    /// Absent retains unknown spell semantics. Existing accepted entry-r1 bytes
    /// remain unchanged; explicit metadata is hashed in the native overlay.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spell_tiles: Option<NativeSpellTileDocument>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub house_tiles: Option<super::NativeHouseTileDocument>,
    /// The one entry-room door (#162 A4-a): exactly one element, or qualification refuses.
    pub doors: Vec<NativeEntryDoor>,
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

/// D38/A4-a (#162 comment 5865792400): the entry room's one usable door — a 4th walkable cell
/// (`cell`) carrying a typed LocalObject overlay with a closed/open state pair and the two
/// transitions that move between them. `NativeFirstEntryDocument::doors` holds this in a `Vec` so
/// zero doors and more than one door are both expressible, ordinary-shaped JSON mistakes rather
/// than something the schema forbids outright; exactly one is ever accepted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEntryDoor {
    pub cell: NativeEntryCell,
    pub definition: ProjectV2DefinitionRef,
    pub closed_state: String,
    pub open_state: String,
    pub open_transition: NativeEntryDoorTransition,
    pub close_transition: NativeEntryDoorTransition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeEntryDoorTransition {
    pub key: String,
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
    frame: NativeEntryFrame,
    spell_tiles: Option<NativeSpellTileDocument>,
    house_tiles: Option<super::NativeHouseTileDocument>,
    door: CanonicalReferencePlayableContent,
}

impl NativeEntryProject {
    pub fn project(&self) -> &WorldProject {
        &self.project
    }

    pub fn source(&self) -> &FirstProductionContentSource {
        &self.source
    }

    /// The one entry-room door's own genuine Reference-profile content (#162 A4-a): real
    /// `REFERENCE_PLAYABLE_CONTENT_PROFILE_ID`, fully validated by `link_reference_playable`, and
    /// sharing this project's package/Content-Lock identity, so its content generation is always
    /// this qualified native-entry project's own.
    ///
    /// `placements` is deliberately empty (DECISION_REQUIRED, r4120444680): the accepted evidence
    /// manifest has no `CONTENT_WORLD` case bound to any target-sensitive claim yet
    /// (`REFERENCE_TARGET_CLAIM_CASE_BINDINGS` in `content/reference_playable.rs` is an
    /// accepted-empty array, excluded/read-only for this task), so `link_reference_playable`
    /// refuses *every* placement's evidence today, honest or not — there is no placement this
    /// task can add and still have `door()` be genuinely, fully linked. A future consumer derives
    /// the door's placement (key `accepted::DOOR_CELL.0`, its cell in `source().cells`, initial
    /// state `accepted::DOOR_CLOSED_STATE`) and constructs its own synthetic,
    /// deliberately-unpromoted `PlacementRef`, then builds a fence from this content and binds it
    /// through the existing `LocalObjectRuntime::bind` — exactly as the CW4 test fixtures in
    /// `world_runtime.rs` already do for ordinary Reference-sourced LocalObjects.
    pub fn door(&self) -> &CanonicalReferencePlayableContent {
        &self.door
    }

    pub(super) fn qualify(
        project: WorldProject,
        overlay: NativeFirstEntryDocument,
    ) -> Result<Self, ProjectError> {
        Self::qualify_for(project, overlay, false)
    }

    pub(super) fn qualify_candidate(
        project: WorldProject,
        overlay: NativeFirstEntryDocument,
    ) -> Result<Self, ProjectError> {
        Self::qualify_for(project, overlay, true)
    }

    fn qualify_for(
        project: WorldProject,
        overlay: NativeFirstEntryDocument,
        candidate: bool,
    ) -> Result<Self, ProjectError> {
        let (source, door) = lower(&project, &overlay, candidate)?;
        require_accepted_bindings(&project, &source, &door, candidate)?;
        if candidate {
            qualify_candidate_ground(&project, &overlay)?;
        }
        // The existing FirstProduction validators (cardinality, key uniqueness, population,
        // references) apply before a source counts as qualified (#937 §4).
        let compiled =
            compile_first_production(&source, FirstProductionCompileTarget::OrdinaryRelease)?;
        let qualified = Self {
            project,
            source,
            frame: overlay.frame,
            spell_tiles: overlay.spell_tiles,
            house_tiles: overlay.house_tiles,
            door,
        };
        // The lookup carrier can be reconstructed later, but its complete source
        // metadata must already qualify under the same compiled generation here.
        qualified.movement_cells(compiled.server_digest())?;
        Ok(qualified)
    }

    /// The qualified cells as a Movement lookup index bound to `server_generation`.
    ///
    /// The door cell (#162 A4-a) is included as an ordinary `Walkable` Terrain cell (M2b,
    /// 5868482467): the index only ever answers "is this cell's *terrain* walkable", exactly as
    /// for any other cell in `source().cells`. It carries no LocalObject overlay state by
    /// itself. Whether the door currently *blocks* movement is the door `LocalObjectRuntime`'s
    /// own `blocking_cells()` (bound at Channel activation, see `activate_native_entry_room`),
    /// which the seam's `ComposedFreshAdmission::step` consults in the same Channel-owner turn,
    /// before this index's own terrain lookup ever runs. A closed door is Blocked and an open
    /// door is walkable; no path may leave a closed door walkable through this index alone.
    fn movement_cells(
        &self,
        server_generation: [u8; 32],
    ) -> Result<NativeEntryMovementCells, ProjectError> {
        use crate::content::static_cell_engine::{
            EngineeringCollisionClaim, EngineeringStaticCellClaim, EngineeringStaticCellIndex,
            EngineeringStaticCellScope,
        };
        let invalid = |_| ProjectError::InvalidProject("native entry movement cell scope");
        let scope = EngineeringStaticCellScope {
            world_id: self.source.world_id,
            coordinate_frame: crate::content::CoordinateFrameRef::new(&self.frame.coordinate_frame)
                .map_err(invalid)?,
            map_revision: crate::content::MapRevisionRef::new(self.source.revisions.map.as_str())
                .map_err(invalid)?,
            generation_digest: server_generation,
            content_lock: self.source.content_lock.clone(),
        };
        let claims = self
            .source
            .cells
            .iter()
            .map(|cell| EngineeringStaticCellClaim {
                scope: scope.clone(),
                cell: crate::content::LogicalCell {
                    x: cell.x,
                    y: cell.y,
                    z: i32::from(cell.z),
                },
                collision: EngineeringCollisionClaim::Qualified(cell.collision),
            })
            .collect();
        let index = EngineeringStaticCellIndex::from_claims(claims)
            .map_err(|_| ProjectError::InvalidProject("native entry movement cell index"))?;
        let spell_tiles = QualifiedSpellTiles::qualify(
            &self.project,
            &self.source.cells,
            scope.clone(),
            self.spell_tiles.as_ref(),
        )?;
        let house_tiles = super::QualifiedHouseTiles::qualify(
            &self.project,
            &self.source.cells,
            scope.clone(),
            self.house_tiles.as_ref(),
        )?;
        Ok(NativeEntryMovementCells {
            house_tiles,
            index: crate::content::native_cell_lookup::NativeMovementCollisionIndex::Entry(index),
            scope,
            spell_tiles,
        })
    }

    /// The qualified start cell of this project (#935). It must exist in the qualified source and
    /// be Walkable; otherwise there is no first-entry start.
    pub fn entry_start(&self) -> Result<NativeEntryStart, ProjectError> {
        self.source
            .cells
            .iter()
            .find(|cell| {
                cell.key.as_str() == accepted::START_CELL
                    && cell.collision == crate::content::CollisionClass::Walkable
            })
            .map(|cell| NativeEntryStart {
                x: cell.x,
                y: cell.y,
                floor: cell.z,
            })
            .ok_or(ProjectError::InvalidProject(
                "native entry start cell is missing or not walkable",
            ))
    }

    fn room_with_compiled(
        &self,
        compiled: crate::content::CompiledFirstProductionContent,
    ) -> Result<QualifiedNativeEntryRoom, ProjectError> {
        let movement_cells = self.movement_cells(compiled.server_digest())?;
        Ok(QualifiedNativeEntryRoom {
            compiled,
            frame_binding: self.frame_binding(),
            entry_start: self.entry_start()?,
            movement_cells,
            map_revision_digest: crate::content::digest::sha256(
                self.source.revisions.map.as_str().as_bytes(),
            ),
            door: self.door.clone(),
        })
    }

    /// Explicit candidate opt-in. Both the collision index and source-qualified
    /// spell tiles are reconstructed under the enriched artifact's outer digest.
    pub(crate) fn qualify_gameplay_room(
        &self,
        input: &crate::content::native_gameplay::NativeGameplayInput,
    ) -> Result<QualifiedNativeEntryRoom, ProjectError> {
        let classic =
            compile_first_production(&self.source, FirstProductionCompileTarget::OrdinaryRelease)?;
        let compiled = crate::content::native_gameplay::compile_native_gameplay(&classic, input)?;
        self.room_with_compiled(compiled)
    }

    /// The source-qualified native frame binding of this project (#935): the qualified frame and
    /// its digest over the World, source manifest and map revision it was qualified with.
    pub fn frame_binding(&self) -> NativeEntryFrameBinding {
        let mut bytes = Vec::with_capacity(256);
        bytes.extend_from_slice(NATIVE_ENTRY_FRAME_BINDING_DOMAIN);
        bytes.extend_from_slice(self.source.world_id.as_bytes());
        for part in [
            self.source
                .package_manifest
                .source_manifest_digest
                .as_str()
                .as_bytes(),
            self.source.revisions.map.as_str().as_bytes(),
            self.frame.coordinate_profile.as_bytes(),
            self.frame.coordinate_frame.as_bytes(),
        ] {
            bytes.extend_from_slice(&(part.len() as u64).to_be_bytes());
            bytes.extend_from_slice(part);
        }
        bytes.extend_from_slice(&self.frame.contract_revision.to_be_bytes());
        bytes.extend_from_slice(&self.frame.origin.x.to_be_bytes());
        bytes.extend_from_slice(&self.frame.origin.y.to_be_bytes());
        bytes.extend_from_slice(&self.frame.origin.floor.to_be_bytes());
        // The axes are closed canonical enums (East/South/Up); the domain tag binds them.
        NativeEntryFrameBinding {
            frame: self.frame.clone(),
            digest: crate::content::digest::sha256(&bytes),
        }
    }
}

const NATIVE_ENTRY_FRAME_BINDING_DOMAIN: &[u8] =
    b"OTERYN_NATIVE_ENTRY_FRAME_BINDING/v1;axes=east,south,up\0";

/// The qualified native frame of one bound entry room and its binding digest. It is produced only
/// from a natively qualified project; activation, the Channel pin and later location issuance
/// carry it unchanged (#935 "Activation issuer and current pin").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeEntryFrameBinding {
    frame: NativeEntryFrame,
    digest: [u8; 32],
}

impl NativeEntryFrameBinding {
    pub fn frame(&self) -> &NativeEntryFrame {
        &self.frame
    }

    pub const fn digest(&self) -> [u8; 32] {
        self.digest
    }
}

/// The committed entry room bound to one WorldId, qualified natively and compiled to its
/// deterministic ordinary-release pair, with the frame binding of the same qualification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedNativeEntryRoom {
    compiled: crate::content::CompiledFirstProductionContent,
    frame_binding: NativeEntryFrameBinding,
    entry_start: NativeEntryStart,
    map_revision_digest: [u8; 32],
    movement_cells: NativeEntryMovementCells,
    /// The one door's own genuine Reference-profile content (M2b, 5868482467): the same
    /// [`NativeEntryProject::door`] this room's qualification produced, carried through so the
    /// Channel activation owner can bind its `LocalObjectRuntime` from the exact qualified
    /// content, never a reconstruction of it.
    door: CanonicalReferencePlayableContent,
}

/// The qualified room's cells as the Movement kernel's direct-lookup index, scoped to the exact
/// World, frame, map revision, content lock and compiled server generation they were qualified
/// with (#935: "Positive evidence must prove the actual qualified native frame through loaded
/// cells"). The index grants no active status; the Channel's pin supplies the current scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeEntryMovementCells {
    spell_tiles: QualifiedSpellTiles,
    house_tiles: super::QualifiedHouseTiles,
    index: crate::content::native_cell_lookup::NativeMovementCollisionIndex,
    scope: crate::content::static_cell_engine::EngineeringStaticCellScope,
}

impl NativeEntryMovementCells {
    pub(crate) const fn house_tiles(&self) -> &super::QualifiedHouseTiles {
        &self.house_tiles
    }
    pub(crate) const fn spell_tiles(&self) -> &QualifiedSpellTiles {
        &self.spell_tiles
    }
    pub(crate) const fn index(
        &self,
    ) -> &crate::content::native_cell_lookup::NativeMovementCollisionIndex {
        &self.index
    }
    pub(crate) fn source_world(
        &self,
    ) -> Option<&super::native_spell_world::QualifiedNativeSpellWorld> {
        self.index.source_world()
    }

    pub(crate) const fn scope(
        &self,
    ) -> &crate::content::static_cell_engine::EngineeringStaticCellScope {
        &self.scope
    }
}

/// The first-entry start cell selected by the Game-owned first-entry source (#935): the accepted
/// `oteryn:cell/entry-start`, attested by the qualified source to exist and be Walkable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeEntryStart {
    pub x: i32,
    pub y: i32,
    pub floor: i16,
}

impl QualifiedNativeEntryRoom {
    pub(crate) fn source_world(
        &self,
    ) -> Option<&super::native_spell_world::QualifiedNativeSpellWorld> {
        self.movement_cells.source_world()
    }
    pub fn compiled(&self) -> &crate::content::CompiledFirstProductionContent {
        &self.compiled
    }

    pub fn frame_binding(&self) -> &NativeEntryFrameBinding {
        &self.frame_binding
    }

    pub const fn entry_start(&self) -> NativeEntryStart {
        self.entry_start
    }

    pub const fn movement_cells(&self) -> &NativeEntryMovementCells {
        &self.movement_cells
    }

    /// SHA-256 of the qualified map revision this room was compiled with.
    pub const fn map_revision_digest(&self) -> [u8; 32] {
        self.map_revision_digest
    }

    /// The one door's own genuine, fully linked Reference-profile content (M2b, 5868482467).
    /// See [`NativeEntryProject::door`].
    pub fn door(&self) -> &CanonicalReferencePlayableContent {
        &self.door
    }
}

/// Rebuilds the committed entry room for `world_id` from its genuine source: bind, re-admit through
/// the native parser under the fixed limits, qualify and compile. Every caller (the activation
/// issuer and the node) derives digests and frame binding this way; none is supplied externally.
pub fn qualify_native_entry_room(
    world_id: crate::foundation::WorldId,
) -> Result<QualifiedNativeEntryRoom, ProjectError> {
    let documents = native_entry_room_documents(world_id)?;
    let snapshot = ProjectSnapshot::new(
        documents.documents().clone(),
        native_entry_first_slice_limits().project,
    )?;
    let project = snapshot.parse_native_entry()?;
    if project.source().world_id != world_id {
        return Err(ProjectError::InvalidProject(
            "native entry room WorldId does not match its binding",
        ));
    }
    let compiled = compile_first_production(
        project.source(),
        FirstProductionCompileTarget::OrdinaryRelease,
    )?;
    project.room_with_compiled(compiled)
}

/// Qualifies the exact pinned gameplay manifest selected by the operator and node.
/// This computes content identity only; it grants no activation authority.
pub fn qualify_native_entry_room_from_gameplay_manifest(
    world_id: crate::foundation::WorldId,
    manifest: &std::path::Path,
) -> Result<QualifiedNativeEntryRoom, ProjectError> {
    let input = crate::content::native_gameplay::NativeGameplayInput::from_manifest(manifest)?;
    qualify_selected_native_gameplay_room(world_id, &input)
}

/// Shared selection for operator issuance and node activation.
pub(crate) fn qualify_selected_native_gameplay_room(
    world_id: crate::foundation::WorldId,
    input: &crate::content::native_gameplay::NativeGameplayInput,
) -> Result<QualifiedNativeEntryRoom, ProjectError> {
    match input.source_world.as_ref() {
        Some(source) => {
            qualify_native_source_spell_world_with_gameplay(world_id, input, &source.bytes)
        }
        None => match input.native_map_profile {
            crate::content::native_gameplay::NativeGameplayMapProfile::AcceptedEntryR1 => {
                qualify_native_entry_room_with_gameplay(world_id, input)
            }
            crate::content::native_gameplay::NativeGameplayMapProfile::SourceQualifiedSpellEntryR2 => {
                qualify_native_spell_entry_room_with_gameplay(world_id, input)
            }
        },
    }
}

/// Qualify the same genuine native source with an explicitly supplied, pinned
/// candidate gameplay envelope. This never loads a global/embedded catalogue.
pub(crate) fn qualify_native_entry_room_with_gameplay(
    world_id: crate::foundation::WorldId,
    input: &crate::content::native_gameplay::NativeGameplayInput,
) -> Result<QualifiedNativeEntryRoom, ProjectError> {
    let documents = native_entry_room_documents(world_id)?;
    let snapshot = ProjectSnapshot::new(
        documents.documents().clone(),
        native_entry_first_slice_limits().project,
    )?;
    let project = snapshot.parse_native_entry()?;
    if project.source().world_id != world_id {
        return Err(ProjectError::InvalidProject(
            "native entry room WorldId does not match its binding",
        ));
    }
    project.qualify_gameplay_room(input)
}

/// Committed native entry-room source (#822 path). It deliberately carries no WorldId: the
/// canonical WorldId is issued per qualification run by the owning Platform World Registry issuer
/// and bound only through [`native_entry_room_documents`] (owner decision, #162).
pub const NATIVE_ENTRY_ROOM_SOURCE: &[u8] = include_bytes!("native_entry_room.json");
pub const NATIVE_ENTRY_ROOM_SOURCE_SCHEMA: &str = "OTERYN_NATIVE_ENTRY_ROOM_SOURCE/v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeEntryRoomSource {
    schema: String,
    licensing: String,
    records: Vec<ProjectReferenceRecord>,
    declarations: Vec<ProjectV2Declaration>,
    world: serde_json::Map<String, serde_json::Value>,
    placements: Vec<ProjectV2Placement>,
    native_first_entry: NativeFirstEntryDocument,
    #[serde(default)]
    imports: Vec<ImportBatch>,
    #[serde(default)]
    sources: Vec<ProjectV2Source>,
    #[serde(default)]
    source_identity_bindings: Vec<ProjectV2SourceIdentityBinding>,
}

/// Binds the committed entry-room source to one issued canonical WorldId and writes the native
/// project documents. The writer re-admits its own output through the native parser, so the
/// result is qualified or refused; a WorldId already present in the source refuses.
pub fn native_entry_room_documents(
    world_id: crate::foundation::WorldId,
) -> Result<CanonicalProjectDocuments, ProjectError> {
    native_room_documents_for(world_id, NATIVE_ENTRY_ROOM_SOURCE, false)
}
pub const NATIVE_SPELL_ENTRY_ROOM_SOURCE: &[u8] = include_bytes!("native_spell_entry_room.json");
pub const NATIVE_SPELL_ENTRY_ROOM_SOURCE_SCHEMA: &str = "OTERYN_NATIVE_SPELL_ENTRY_ROOM_SOURCE/v2";

pub fn native_spell_entry_room_documents(
    world_id: crate::foundation::WorldId,
) -> Result<CanonicalProjectDocuments, ProjectError> {
    native_room_documents_for(world_id, NATIVE_SPELL_ENTRY_ROOM_SOURCE, true)
}
fn native_room_documents_for(
    world_id: crate::foundation::WorldId,
    bytes: &[u8],
    candidate: bool,
) -> Result<CanonicalProjectDocuments, ProjectError> {
    let source: NativeEntryRoomSource = serde_json::from_slice(bytes)
        .map_err(|error| ProjectError::InvalidJson(error.to_string()))?;
    let expected_schema = if candidate {
        NATIVE_SPELL_ENTRY_ROOM_SOURCE_SCHEMA
    } else {
        NATIVE_ENTRY_ROOM_SOURCE_SCHEMA
    };
    if source.schema != expected_schema {
        return Err(ProjectError::InvalidProject(
            "native entry-room source schema mismatch",
        ));
    }
    let world_id = crate::content::production::encode_world_id(world_id);
    let mut world = source.world;
    if world
        .insert(
            "world_id".to_owned(),
            serde_json::Value::String(world_id.clone()),
        )
        .is_some()
    {
        return Err(ProjectError::InvalidProject(
            "native entry-room source must not carry a WorldId",
        ));
    }
    let world: ProjectV2World = serde_json::from_value(serde_json::Value::Object(world))
        .map_err(|error| ProjectError::InvalidJson(error.to_string()))?;
    let draft = ProjectV2Draft {
        core: ProjectDraft {
            project_revision: accepted::PACKAGE_REVISION.to_owned(),
            package_key: accepted::PACKAGE_KEY.to_owned(),
            semantic_schema_version: accepted::SEMANTIC_SCHEMA.to_owned(),
            licensing_metadata: source.licensing,
            world_id,
            coordinate_frame: world.coordinate_frame.clone(),
            records: source.records,
            imports: source.imports,
            metadata: vec![],
        },
        state: ProjectV2State {
            declarations: source.declarations,
            item_authoring: vec![],
            authoring_profiles: vec![],
            worlds: vec![world],
            placements: source.placements,
            appearance_bindings: vec![],
            assets: vec![],
            sources: source.sources,
            source_identity_bindings: source.source_identity_bindings,
            editor: vec![],
        },
    };
    if candidate {
        CanonicalProjectDocuments::from_native_spell_entry_draft(draft, source.native_first_entry)
    } else {
        CanonicalProjectDocuments::from_native_entry_draft(draft, source.native_first_entry)
    }
}

/// Owner-accepted values of `NATIVE-ENTRY-ROOM-PRODUCT-BINDINGS-V1` (#940 §1–§3) and the authored
/// entry-room choices of `PLAYER_FIRST_ENTRY_NATIVE_CONTENT_BINDING_V1` (#935). A spelling that is
/// not one of these is not product-policy evidence (#937 §3), so every other value refuses.
pub mod accepted {
    pub const DEFINITION_REVISION: &str = "oteryn:rev/entry-r1";
    pub const PACKAGE_KEY: &str = "oteryn:package/native-entry-room";
    pub const PACKAGE_REVISION: &str = "oteryn:package-rev/entry-r1";
    pub const SEMANTIC_SCHEMA: &str = "oteryn:schema/first-production-v1";
    pub const LOCK_TOKEN: &str = "lock:oteryn:package-rev/entry-r1";
    /// (content, map, ruleset, world_policy, compiler, canonicalization, sim_profile, profile)
    pub const REVISIONS: [&str; 8] = [
        "oteryn:content/entry-r1",
        "oteryn:map/entry-r1",
        "oteryn:ruleset/entry-r1",
        "oteryn:world-policy/entry-r1",
        "oteryn:compiler/first-production-r1",
        "oteryn:canonicalization/first-production-r1",
        "oteryn:sim/entry-r1",
        "FIRST_PRODUCTION_CONTENT_PROFILE/v1",
    ];
    pub const REGION: &str = "oteryn:region/entry";
    pub const AREA: &str = "oteryn:area/entry-room";
    pub const TERRAIN: &str = "oteryn:terrain/stone-floor";
    /// (cell key, x, y, floor, walkable) — start, east, north (#935).
    pub const CELLS: [(&str, i32, i32, i16, bool); 3] = [
        ("oteryn:cell/entry-start", 0, 0, 0, true),
        ("oteryn:cell/entry-east", 1, 0, 0, true),
        ("oteryn:cell/entry-north", 0, -1, 0, false),
    ];
    /// The one door's 4th walkable cell (#162 A4-a): (cell key, x, y, floor, walkable), adjacent
    /// to `east` and to `north`.
    pub const DOOR_CELL: (&str, i32, i32, i16, bool) = ("oteryn:cell/entry-door", 1, -1, 0, true);
    /// The door LocalObject's own identity key.
    pub const DOOR_DEFINITION: &str = "oteryn:local-object/entry-door";
    pub const DOOR_CLOSED_STATE: &str = "oteryn:reference.state.closed";
    pub const DOOR_OPEN_STATE: &str = "oteryn:reference.state.open";
    pub const DOOR_OPEN_TRANSITION: &str = "oteryn:transition/entry-door-open";
    pub const DOOR_CLOSE_TRANSITION: &str = "oteryn:transition/entry-door-close";
    pub const DOOR_OPEN_INTENT: &str = "oteryn:reference.intent.entry-door-open";
    pub const DOOR_CLOSE_INTENT: &str = "oteryn:reference.intent.entry-door-close";
    /// Must equal `world_runtime::LOCAL_OBJECT_TRANSITION_CAPABILITY`: the CW4 kernel checks this
    /// capability key by value at bind time, not by shared Rust symbol, since `content` does not
    /// depend on `world_runtime`.
    pub const DOOR_OWNER_CAPABILITY: &str = "oteryn:runtime.capability.local-object-transition";
    /// Origin (x, y, floor), World bounds (min_x, min_y, max_x_exclusive, max_y_exclusive), floors.
    /// This is the accepted envelope for exactly the four placed cells (#162 A4-a): three room
    /// cells plus the one door cell. It is not grown to make refusal tests distinct; a door
    /// placed off this frame is refused by the same envelope check as any other placement.
    pub const ORIGIN: (i32, i32, i16) = (0, 0, 0);
    pub const BOUNDS: (i64, i64, i64, i64) = (0, -1, 2, 1);
    pub const FLOORS: [i16; 1] = [0];
    pub const RELOCATION: (&str, &str, &str) = (
        "oteryn:relocation/entry-east-return",
        "oteryn:cell/entry-east",
        "oteryn:cell/entry-start",
    );
    pub const BEHAVIOR: (&str, &str) = (
        "oteryn:behavior/passive-idle",
        "oteryn:policy/passive-idle-r1",
    );
    pub const CREATURE: (&str, &str) = ("oteryn:creature/rat", "oteryn:policy/creature-rat-r1");
    /// (presentation key, metadata token), sorted by key.
    pub const PRESENTATIONS: [(&str, &str); 3] = [
        ("oteryn:presentation/bite", "oteryn:appearance/bite-r1"),
        ("oteryn:presentation/cheese", "oteryn:appearance/cheese-r1"),
        ("oteryn:presentation/rat", "oteryn:appearance/rat-r1"),
    ];
    pub const SPAWN: &str = "oteryn:spawn/entry-rat";
    pub const SPAWN_CELL: &str = "oteryn:cell/entry-east";
    /// First-entry start cell (#935).
    pub const START_CELL: &str = "oteryn:cell/entry-start";
    pub const FORMULA: &str = "oteryn:formula/entry-melee-r1";
    pub const EFFECT: &str = "oteryn:effect/bite";
    pub const ABILITY: &str = "oteryn:ability/bite";
    pub const ITEM: &str = "oteryn:item/cheese";
    pub const LOOT_TABLE: &str = "oteryn:loot/rat";
    pub const LOOT_ENTRY: &str = "oteryn:loot-entry/rat-cheese";
    pub const XP: &str = "oteryn:xp/rat";
    pub const RNG_PURPOSE: &str = "oteryn:rng/rat-loot";
    pub const RNG_PROFILE: &str = "oteryn:rng-profile/entry-r1";
}

fn pin(ok: bool, reason: &'static str) -> Result<(), ProjectError> {
    if ok { Ok(()) } else { refuse(reason) }
}

#[allow(clippy::too_many_lines)]
fn require_accepted_bindings(
    project: &WorldProject,
    source: &FirstProductionContentSource,
    door: &CanonicalReferencePlayableContent,
    candidate: bool,
) -> Result<(), ProjectError> {
    use accepted as a;
    let manifest = &source.package_manifest;
    pin(
        manifest.package_key.as_str() == a::PACKAGE_KEY
            && manifest.package_revision.as_str() == a::PACKAGE_REVISION
            && manifest.semantic_schema_version.as_str() == a::SEMANTIC_SCHEMA
            && source.content_lock.revision_digest_token.as_str() == a::LOCK_TOKEN,
        "native entry package identity is not the accepted binding",
    )?;
    let r = &source.revisions;
    let revisions = [
        &r.content,
        &r.map,
        &r.ruleset,
        &r.world_policy,
        &r.compiler,
        &r.canonicalization,
        &r.sim_profile,
        &r.profile_revision,
    ];
    pin(
        revisions
            .iter()
            .zip(a::REVISIONS)
            .enumerate()
            .all(|(index, (actual, expected))| {
                actual.as_str()
                    == if candidate && index == 1 {
                        "oteryn:map/entry-spell-r2"
                    } else {
                        expected
                    }
            }),
        "native entry revision set is not the accepted binding",
    )?;
    pin(
        source.regions.len() == 1
            && source.regions[0].key.as_str() == a::REGION
            && source.areas.len() == 1
            && source.areas[0].key.as_str() == a::AREA
            && source.terrains.len() == 1
            && source.terrains[0].key.as_str() == a::TERRAIN,
        "native entry region, area or terrain is not the accepted binding",
    )?;
    let mut cells: Vec<_> = source
        .cells
        .iter()
        .map(|cell| {
            (
                cell.key.as_str(),
                cell.x,
                cell.y,
                cell.z,
                cell.collision == CollisionClass::Walkable,
            )
        })
        .collect();
    cells.sort_unstable();
    // #162 A4-a: the accepted set is the three room cells plus the one door cell — a bijection
    // of exactly `NATIVE_ENTRY_CELLS + NATIVE_ENTRY_DOOR_CELLS` FirstProduction Terrain cells.
    let mut expected_cells: Vec<_> = a::CELLS.into_iter().chain([a::DOOR_CELL]).collect();
    expected_cells.sort_unstable();
    pin(
        cells == expected_cells,
        "native entry cells are not the accepted start, east, north and door",
    )?;
    let world = project
        .v2
        .as_ref()
        .and_then(|state| state.worlds.first())
        .ok_or(ProjectError::InvalidProject("native entry World missing"))?;
    pin(
        (
            world.bounds.min_x,
            world.bounds.min_y,
            world.bounds.max_x_exclusive,
            world.bounds.max_y_exclusive,
        ) == a::BOUNDS
            && world.floors == a::FLOORS,
        "native entry World envelope is not the accepted entry-room",
    )?;
    let relocation = &source.relocations[0];
    pin(
        source.relocations.len() == 1
            && (
                relocation.key.as_str(),
                relocation.from_cell.as_str(),
                relocation.to_cell.as_str(),
            ) == a::RELOCATION,
        "native entry relocation is not the accepted binding",
    )?;
    let behavior = &source.behaviors[0];
    let creature = &source.creatures[0];
    pin(
        (behavior.key.as_str(), behavior.policy_revision.as_str()) == a::BEHAVIOR
            && (creature.key.as_str(), creature.policy_revision.as_str()) == a::CREATURE
            && creature.presentation_key.as_str() == a::PRESENTATIONS[2].0,
        "native entry behavior or creature policy is not the accepted binding",
    )?;
    let mut presentations: Vec<_> = source
        .presentations
        .iter()
        .map(|presentation| {
            (
                presentation.key.as_str(),
                presentation.metadata_token.as_str(),
            )
        })
        .collect();
    presentations.sort_unstable();
    pin(
        presentations == a::PRESENTATIONS,
        "native entry presentations are not the accepted binding",
    )?;
    let spawn = &source.spawns[0];
    pin(
        spawn.key.as_str() == a::SPAWN
            && spawn.cell_key.as_str() == a::SPAWN_CELL
            && spawn.population_limit == 1
            && spawn.recovery == SpawnRecoveryClass::EphemeralScopeReset
            && spawn.multiplicity == MultiplicityClass::ChannelLocalRepeatable
            && spawn.eligibility_scope == EligibilityScope::CharacterWorld,
        "native entry spawn is not the accepted binding",
    )?;
    let loot = &source.loot_tables[0];
    pin(
        source.formula_profiles[0].key.as_str() == a::FORMULA
            && source.effects[0].key.as_str() == a::EFFECT
            && source.abilities[0].key.as_str() == a::ABILITY
            && source.abilities[0].presentation_key.as_str() == a::PRESENTATIONS[0].0
            && source.items[0].key.as_str() == a::ITEM
            && source.items[0].presentation_key.as_str() == a::PRESENTATIONS[1].0
            && loot.key.as_str() == a::LOOT_TABLE
            && loot.entries.len() == 1
            && loot.entries[0].key.as_str() == a::LOOT_ENTRY
            && source.xp_definitions[0].key.as_str() == a::XP
            && source.rng.purpose_keys.len() == 1
            && source.rng.purpose_keys[0].as_str() == a::RNG_PURPOSE
            && source.rng.profile_revision.as_str() == a::RNG_PROFILE,
        "native entry ability, item, loot, XP or RNG is not the accepted binding",
    )?;
    let [door_definition] = door.definitions.as_slice() else {
        return refuse("native entry door is not the accepted binding");
    };
    pin(
        door_definition.definition.key().as_str() == a::DOOR_DEFINITION,
        "native entry door is not the accepted binding",
    )
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
    candidate: bool,
) -> Result<
    (
        FirstProductionContentSource,
        CanonicalReferencePlayableContent,
    ),
    ProjectError,
> {
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
        || (!candidate && !project.imports.batches.is_empty())
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
    if (origin.x, origin.y, origin.floor) != accepted::ORIGIN {
        return refuse("native entry origin is not the accepted entry-room origin");
    }
    if !in_bounds(
        &world.bounds,
        &world.floors,
        origin.x,
        origin.y,
        origin.floor,
    ) {
        return refuse("native entry origin outside the selected World");
    }

    // Region and three cells bijective with three Terrain placements, plus the one door cell
    // (#162 A4-a). Full placement cardinality is enforced here, before any placement is indexed,
    // so malformed content (including zero placements) fails closed instead of panicking.
    let region = ProductionKey::new(&overlay.region.key)?;
    if overlay.cells.len() != NATIVE_ENTRY_CELLS {
        return refuse("native entry requires exactly three cells and placements");
    }
    if overlay.doors.len() != NATIVE_ENTRY_DOOR_CELLS {
        return refuse("native entry requires exactly one door");
    }
    if state.placements.len() != NATIVE_ENTRY_CELLS + NATIVE_ENTRY_DOOR_CELLS {
        return refuse("native entry requires exactly three cells and placements");
    }
    let door_overlay = &overlay.doors[0];
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
    // Cardinality was just checked above, but a checked lookup keeps this panic-free even if that
    // invariant is ever weakened elsewhere.
    let terrain_ref = &state
        .placements
        .first()
        .ok_or(ProjectError::InvalidProject(
            "native entry requires at least one Terrain placement",
        ))?
        .definition;
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

    // The one door's 4th walkable cell (#162 A4-a): same bijective placement-matching discipline
    // as the three room cells above (cardinality already enforced above), plus its own adjacency
    // requirement.
    let door_cell = &door_overlay.cell;
    let NativeEntryCollision::Walkable = door_cell.collision else {
        return refuse("native entry door cell must be walkable");
    };
    let mut door_matching = state
        .placements
        .iter()
        .filter(|placement| placement.key == door_cell.placement_key);
    let (Some(door_placement), None) = (door_matching.next(), door_matching.next()) else {
        return refuse("native entry door cell does not match exactly one placement");
    };
    let door_area_ok = door_placement.area.as_ref().is_some_and(|area_ref| {
        area_ref.family == ProjectV2Family::Area
            && area_ref.key == area_identity.key
            && area_ref.revision == area_identity.revision
    });
    if door_placement.world != world.key
        || door_placement.coordinate_frame != frame.coordinate_frame
        || door_placement.map_revision != overlay.revisions.map
        || door_placement.definition != *terrain_ref
        || !door_area_ok
        || door_placement.document.is_some()
        || door_placement.parent_placement.is_some()
    {
        return refuse("native entry door placement binding mismatch");
    }
    if !in_bounds(
        &world.bounds,
        &world.floors,
        door_placement.x,
        door_placement.y,
        door_placement.floor,
    ) {
        return refuse("native entry door cell is outside the World");
    }
    // Widened to i64 (r4120672740): a malformed project's declared World bounds can span the
    // full i32 coordinate range, and an i32 subtraction/sum here would overflow before the later
    // accepted-coordinate pin ever runs.
    let door_adjacent = coordinates.iter().any(|&(x, y, floor)| {
        floor == door_placement.floor
            && (i64::from(door_placement.x) - i64::from(x)).abs()
                + (i64::from(door_placement.y) - i64::from(y)).abs()
                == 1
    });
    if !door_adjacent {
        return refuse("native entry door cell must be adjacent to the entry room");
    }
    if door_cell.region_key != overlay.region.key
        || !coordinates.insert((door_placement.x, door_placement.y, door_placement.floor))
        || !cell_keys.insert(door_cell.placement_key.clone())
    {
        return refuse("native entry door cell region mismatch or duplicated");
    }
    cells.push(FirstProductionCell {
        key: ProductionKey::new(&door_cell.placement_key)?,
        region_key: region.clone(),
        area_key: area.clone(),
        terrain_key: terrain.clone(),
        x: door_placement.x,
        y: door_placement.y,
        z: door_placement.floor,
        collision: CollisionClass::Walkable,
    });
    let door_world_id = decode_world_id(&project.reference.world_id)?;
    let door_coordinate_frame = crate::content::CoordinateFrameRef::new(&frame.coordinate_frame)?;

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
    let door_definition_ref = &door_overlay.definition;
    let graph_refs: Vec<&ProjectV2DefinitionRef> = [
        terrain_ref,
        behavior_ref,
        creature_ref,
        ability_ref,
        &overlay.xp.formula,
        item_ref,
        door_definition_ref,
    ]
    .into_iter()
    .chain(presentation_refs.iter().copied())
    .collect();
    if graph_refs
        .iter()
        .any(|reference| reference.revision != accepted::DEFINITION_REVISION)
        || area_identity.revision != accepted::DEFINITION_REVISION
        || effect_identity.revision != accepted::DEFINITION_REVISION
    {
        return refuse("native entry definition revision is not the accepted revision");
    }
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

    // The one door LocalObject (#162 A4-a): resolved from the same admitted Reference-records
    // graph as every other typed reference above, then linked through the real, unmodified
    // Reference profile — genuine `REFERENCE_PLAYABLE_CONTENT_PROFILE_ID`, real
    // `link_reference_playable` semantic validation, never faked or skipped. FirstProduction has
    // no LocalObject record, so this content is separate from — but shares this same project's
    // package/Content-Lock identity with — the FirstProduction source above.
    require_family(door_definition_ref, ProjectV2Family::LocalObject)?;
    let ProjectReferenceRecord::LocalObject {
        client_projection: door_client_projection,
        states: door_state_documents,
        ..
    } = find_record(records, door_definition_ref)?
    else {
        return refuse("native entry door definition must be a LocalObject record");
    };
    // #162 r4121206658: the door is a real admitted client interaction target (USE-WIRE-V1), so
    // its client projection must be genuinely ClientSafe, not merely absent-of-ServerOnly. A
    // ServerOnly door would link (the Reference profile does not require ClientSafe), but the
    // client could never legitimately learn its state or states, so it is refused here before
    // linking rather than left to a downstream consumer to notice.
    pin(
        *door_client_projection == ProjectionDocument::ClientSafe,
        "native entry door client_projection must be ClientSafe",
    )?;
    let door_ref = crate::content::TypedDefinitionRef::new(
        DefinitionFamily::LocalObject,
        ProductionKey::new(&door_definition_ref.key)?,
        crate::content::DefinitionRevisionRef::new(&door_definition_ref.revision)?,
    );
    let mut door_states = Vec::with_capacity(door_state_documents.len());
    for state_document in door_state_documents {
        door_states.push(state_document.lower()?);
    }
    let door_closed_key = ProductionKey::new(&door_overlay.closed_state)?;
    let door_open_key = ProductionKey::new(&door_overlay.open_state)?;
    let door_has_state = |key: &ProductionKey, collision: LocalObjectCollisionPresence| {
        door_states
            .iter()
            .any(|state| &state.key == key && state.collision == collision)
    };
    if door_overlay.closed_state != accepted::DOOR_CLOSED_STATE
        || door_overlay.open_state != accepted::DOOR_OPEN_STATE
        || door_states.len() != 2
        || !door_has_state(&door_closed_key, LocalObjectCollisionPresence::Present)
        || !door_has_state(&door_open_key, LocalObjectCollisionPresence::Absent)
    {
        return refuse("native entry door declares an unknown state");
    }
    if door_overlay.open_transition.key != accepted::DOOR_OPEN_TRANSITION
        || door_overlay.close_transition.key != accepted::DOOR_CLOSE_TRANSITION
    {
        return refuse("native entry door transition is not the accepted binding");
    }
    let door_owner_capability = OwnerCapabilityRequirement {
        capability_key: ProductionKey::new(accepted::DOOR_OWNER_CAPABILITY)?,
    };
    let door_transitions = vec![
        TransitionBinding {
            key: TransitionKey::new(&door_overlay.open_transition.key)?,
            definition: door_ref.clone(),
            source_state: door_closed_key.clone(),
            normalized_intent_family: ProductionKey::new(accepted::DOOR_OPEN_INTENT)?,
            target_state: door_open_key.clone(),
            owner_capability: door_owner_capability.clone(),
            policy_guard_refs: vec![],
        },
        TransitionBinding {
            key: TransitionKey::new(&door_overlay.close_transition.key)?,
            definition: door_ref.clone(),
            source_state: door_open_key.clone(),
            normalized_intent_family: ProductionKey::new(accepted::DOOR_CLOSE_INTENT)?,
            target_state: door_closed_key.clone(),
            owner_capability: door_owner_capability,
            policy_guard_refs: vec![],
        },
    ];
    // #162 A4-a, DECISION_REQUIRED (r4120444680): the door's placement is deliberately not
    // included here. `content/reference_playable.rs`'s `REFERENCE_TARGET_CLAIM_CASE_BINDINGS` is
    // an accepted-empty array ("the evidence manifest currently has no CONTENT_WORLD mechanic
    // case that can authorize any of these claims") and is excluded/read-only for this task, so
    // `link_reference_playable`'s `validate_placement` -> `require_reference_promotion` refuses
    // *every* placement's SpatialAddress/PresentationFootprint/CollisionFootprint claim
    // unconditionally today, regardless of how genuine its evidence is — there is no honest
    // evidence this task can construct that passes. Building a placement into this draft would
    // therefore always fail to link; appending one after linking (the prior approach) fabricated
    // evidence and mislabeled the unlinked result canonical. Neither is acceptable, so `door()`
    // stays genuinely, fully linked (empty `placements`, exactly as validated) and carries only
    // what the linker actually canonicalizes: the one LocalObject definition and its two
    // transitions. A future consumer derives the door's placement (key `accepted::DOOR_CELL.0`,
    // its cell in `source().cells`, initial state `accepted::DOOR_CLOSED_STATE`) and constructs
    // its own synthetic, deliberately-unpromoted `PlacementRef`, exactly as the existing CW4 test
    // fixtures in `world_runtime.rs` already do for ordinary Reference-sourced LocalObjects.
    let door_content = link_reference_playable(ReferencePlayableContentSource {
        profile_revision: ProductionAtom::new(
            "reference profile revision",
            REFERENCE_PLAYABLE_CONTENT_PROFILE_ID,
        )?,
        capability_profile: ProductionAtom::new(
            "reference capability profile",
            REFERENCE_PLAYABLE_CAPABILITY_PROFILE,
        )?,
        package_manifest: package_manifest.clone(),
        content_lock: content_lock.clone(),
        world_id: door_world_id,
        coordinate_frame: door_coordinate_frame,
        definitions: vec![ReferenceDefinition {
            definition: door_ref,
            kind: ReferenceDefinitionKind::LocalObjectStates(door_states),
            client_projection: door_client_projection.lower(),
        }],
        placements: Vec::new(),
        ordered_placements: Vec::new(),
        transitions: door_transitions,
    })?;

    let source = FirstProductionContentSource {
        package_manifest,
        content_lock,
        world_id: door_world_id,
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
    };
    Ok((source, door_content))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::content::LogicalCell;

    fn test_world_id() -> crate::foundation::WorldId {
        let mut bytes = [0_u8; 16];
        bytes[0] = 1;
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = 1;
        crate::foundation::WorldId::decode(&bytes).expect("valid UUIDv7 WorldId")
    }

    /// M2b (#162 5868482467): the door cell is a genuine, `Walkable` FirstProduction Terrain
    /// cell (`source().cells`) *and* is now included in the active Movement index built by
    /// `movement_cells()`, together with the door's own `LocalObjectRuntime` (bound at Channel
    /// activation) — this index alone answers only "is this cell's terrain walkable", never
    /// whether the door currently blocks it.
    #[test]
    fn movement_cells_include_the_door_cell_as_walkable_terrain() {
        let room = qualify_native_entry_room(test_world_id()).expect("qualified native entry room");
        let movement = room.movement_cells();
        let door_cell = LogicalCell {
            x: accepted::DOOR_CELL.1,
            y: accepted::DOOR_CELL.2,
            z: i32::from(accepted::DOOR_CELL.3),
        };
        let start_cell = LogicalCell {
            x: accepted::CELLS[0].1,
            y: accepted::CELLS[0].2,
            z: i32::from(accepted::CELLS[0].3),
        };
        assert!(matches!(
            movement.index().lookup(movement.scope(), door_cell),
            Ok(crate::content::CollisionClass::Walkable)
        ));
        assert!(
            movement
                .index()
                .lookup(movement.scope(), start_cell)
                .is_ok()
        );
        assert!(
            room.compiled()
                .server_digest()
                .iter()
                .any(|byte| *byte != 0)
        );
        // The door's own genuine, fully linked Reference-profile content is carried through
        // qualification unchanged (M2b): exactly one LocalObject definition, ClientSafe (r4121206658),
        // with no placements (DECISION_REQUIRED, r4120444680).
        assert_eq!(room.door().definitions.len(), 1);
        assert!(room.door().placements.is_empty());
        assert_eq!(
            room.door().definitions[0].client_projection,
            crate::content::ClientProjectionClass::ClientSafe
        );
    }
}

/// Bounded source import closure for the explicitly authored four-cell spell map.
/// Source data supplies ground class/speed/blocking; placement and PZ/floor flags
/// are explicit original Oteryn design, not attributed to source OTBM coordinates.
fn qualify_candidate_ground(
    project: &WorldProject,
    overlay: &NativeFirstEntryDocument,
) -> Result<(), ProjectError> {
    let invalid =
        || ProjectError::InvalidProject("native spell entry source ground proof mismatch");
    let state = project.v2().ok_or_else(invalid)?;
    let [batch] = project.imports.batches.as_slice() else {
        return Err(invalid());
    };
    if batch.batch_id != "native-canary-ground-416-r1"
        || batch.source_repository != "https://github.com/opentibiabr/canary"
        || batch.source_revision != NATIVE_GROUND_SOURCE_REVISION
        || batch.source_artifact_sha256 != NATIVE_GROUND_PROOF_SHA256
        || batch.access_disposition != "REFERENCE_ONLY"
        || batch.source_generation_profile != "native-ground-source-proof-r1"
        || batch.importer != "canary-ground-source-proof-r1"
        || batch.mapper != "tools/content-schema/native-gameplay/build_ground_source.py"
        || batch.mapper_revision != "r1"
        || batch.mapper_sha256 != "eaa431e3530bc0b79579d681bd9cb35f88a4dc4c5f010e4988a7cb053886299e"
        || !batch.candidates.is_empty()
        || !batch.reimport_states.is_empty()
    {
        return Err(invalid());
    }
    let [source] = state.sources.as_slice() else {
        return Err(invalid());
    };
    if source.key != NATIVE_GROUND_SOURCE_KEY
        || source.revision != NATIVE_GROUND_SOURCE_REVISION
        || source.import_batch_id != batch.batch_id
        || source.sha256 != NATIVE_GROUND_PROOF_SHA256
        || source.evidence != ProjectV2EvidenceClass::Proven
    {
        return Err(invalid());
    }
    let [binding] = state.source_identity_bindings.as_slice() else {
        return Err(invalid());
    };
    if binding.source_key != source.key
        || binding.source_revision != source.revision
        || binding.identity_namespace != "ots/item_server_id"
        || binding.external_id != "416"
        || binding.target.family != ProjectV2Family::Terrain
        || binding.target.key != "oteryn:terrain/stone-floor"
        || binding.target.revision != "oteryn:rev/entry-r1"
        || binding.disposition != ProjectV2SourceIdentityDisposition::AcceptedAlias
    {
        return Err(invalid());
    }
    let tiles = overlay.spell_tiles.as_ref().ok_or_else(invalid)?;
    if tiles.schema != NATIVE_SPELL_TILE_SCHEMA || tiles.tiles.len() != 4 {
        return Err(invalid());
    }
    for tile in &tiles.tiles {
        let ground = tile.ground.as_ref().ok_or_else(invalid)?;
        let obstacle = tile.cell_key == "oteryn:cell/entry-north";
        if ground.terrain_key != binding.target.key
            || ground.source_binding.as_ref() != Some(binding)
            || tile.ground_speed != Some(100)
            || !tile.top_items.is_empty()
            || tile.flags
                != (NativeSpellTileFlags {
                    block_solid: obstacle,
                    block_projectile: obstacle,
                    immovable_block_solid: obstacle,
                    immovable_block_item: false,
                    immovable_nonfield_block_item: false,
                    floor_change: false,
                    protection_zone: false,
                })
        {
            return Err(invalid());
        }
    }
    Ok(())
}

/// Candidate only: exact source-provenance ground semantics on the four original
/// cells, enriched under the explicitly supplied gameplay envelope outer digest.
pub(crate) fn qualify_native_spell_entry_room_with_gameplay(
    world_id: crate::foundation::WorldId,
    input: &crate::content::native_gameplay::NativeGameplayInput,
) -> Result<QualifiedNativeEntryRoom, ProjectError> {
    let documents = native_spell_entry_room_documents(world_id)?;
    let snapshot = ProjectSnapshot::new(
        documents.documents().clone(),
        native_spell_entry_candidate_limits().project,
    )?;
    let project = snapshot.parse_native_spell_entry()?;
    if project.source().world_id != world_id {
        return Err(ProjectError::InvalidProject(
            "candidate spell room World binding mismatch",
        ));
    }
    project.qualify_gameplay_room(input)
}

#[cfg(test)]
mod spell_entry_candidate_tests {
    use super::*;
    fn world() -> crate::foundation::WorldId {
        crate::foundation::WorldId::decode(&[
            0x01, 0x9a, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 9,
        ])
        .unwrap()
    }
    #[test]
    fn candidate_ground_is_qualified_and_accepted_parser_stays_closed() {
        let documents = native_spell_entry_room_documents(world()).unwrap();
        let snapshot = ProjectSnapshot::new(
            documents.documents().clone(),
            native_spell_entry_candidate_limits().project,
        )
        .unwrap();
        assert!(snapshot.parse_native_entry().is_err());
        assert!(
            snapshot
                .parse(native_spell_entry_candidate_limits().project)
                .is_err()
        );
        let candidate = snapshot.parse_native_spell_entry().unwrap();
        let compiled = compile_first_production(
            candidate.source(),
            FirstProductionCompileTarget::OrdinaryRelease,
        )
        .unwrap();
        let room = candidate.room_with_compiled(compiled).unwrap();
        let start = room.entry_start();
        let cells = room.movement_cells();
        let tile = cells
            .spell_tiles()
            .lookup(
                cells.scope(),
                crate::content::LogicalCell {
                    x: start.x,
                    y: start.y,
                    z: i32::from(start.floor),
                },
            )
            .unwrap();
        assert_eq!(tile.ground_source_id(), Some(416));
        assert_eq!(tile.ground_speed(), Some(100));
        assert!(tile.ground_present());
        let accepted = qualify_native_entry_room(world()).unwrap();
        assert_ne!(
            room.compiled().server_digest(),
            accepted.compiled().server_digest()
        );
        assert_eq!(room.entry_start(), accepted.entry_start());
        assert_ne!(room.map_revision_digest(), accepted.map_revision_digest());
    }
    #[test]
    fn source_policy_rejects_tampered_speed_flags_and_provenance() {
        let source: NativeEntryRoomSource =
            serde_json::from_slice(NATIVE_SPELL_ENTRY_ROOM_SOURCE).unwrap();
        let docs = native_spell_entry_room_documents(world()).unwrap();
        let snapshot = ProjectSnapshot::new(
            docs.documents().clone(),
            native_spell_entry_candidate_limits().project,
        )
        .unwrap();
        let candidate = snapshot.parse_native_spell_entry().unwrap();
        let mut overlay = source.native_first_entry;
        overlay.spell_tiles.as_mut().unwrap().tiles[0].ground_speed = Some(150);
        assert!(qualify_candidate_ground(candidate.project(), &overlay).is_err());
        overlay.spell_tiles.as_mut().unwrap().tiles[0].ground_speed = Some(100);
        overlay.spell_tiles.as_mut().unwrap().tiles[0]
            .flags
            .protection_zone = true;
        assert!(qualify_candidate_ground(candidate.project(), &overlay).is_err());
        overlay.spell_tiles.as_mut().unwrap().tiles[0]
            .flags
            .protection_zone = false;
        overlay.spell_tiles.as_mut().unwrap().tiles[0]
            .ground
            .as_mut()
            .unwrap()
            .source_binding
            .as_mut()
            .unwrap()
            .external_id = "418".into();
        assert!(qualify_candidate_ground(candidate.project(), &overlay).is_err());
    }
}

/// Explicit profile3 qualification. This wraps the actual source-safe client map
/// and server source closure together; it never changes the original entry APIs.
pub(crate) fn qualify_native_source_spell_world_with_gameplay(
    world: crate::foundation::WorldId,
    input: &crate::content::native_gameplay::NativeGameplayInput,
    source: &[u8],
) -> Result<QualifiedNativeEntryRoom, ProjectError> {
    let base = qualify_native_entry_room_with_gameplay(world, input)?;
    let pair = crate::content::native_source_world_carrier::compile(base.compiled(), source)?;
    let compiled = base
        .compiled()
        .with_native_artifact_pair(pair.server, pair.client);
    let mut scope = base.movement_cells().scope().clone();
    scope.coordinate_frame =
        crate::content::CoordinateFrameRef::new(super::native_spell_world::SOURCE_WORLD_FRAME)
            .map_err(|_| ProjectError::InvalidProject("source map frame"))?;
    scope.map_revision =
        crate::content::MapRevisionRef::new(super::native_spell_world::SOURCE_WORLD_MAP)
            .map_err(|_| ProjectError::InvalidProject("source map revision"))?;
    scope.generation_digest = compiled.server_digest();
    let map = super::native_spell_world::qualify(source, scope.clone())?;
    let spell_tiles = map.spell_tiles();
    let entry_start = map.entry_start();
    let house_tiles = super::QualifiedHouseTiles::from_source_world(&map);
    let mut frame = base.frame_binding.frame.clone();
    frame.coordinate_frame = super::native_spell_world::SOURCE_WORLD_FRAME.into();
    let mut binding = b"OTERYN_NATIVE_SOURCE_WORLD_FRAME_BINDING/v1;axes=east,south,up\0".to_vec();
    binding.extend_from_slice(world.as_bytes());
    binding.extend_from_slice(&crate::content::digest::sha256(source));
    binding.extend_from_slice(
        &serde_json::to_vec(&frame)
            .map_err(|_| ProjectError::InvalidProject("source map frame encoding"))?,
    );
    let frame_binding = NativeEntryFrameBinding {
        frame,
        digest: crate::content::digest::sha256(&binding),
    };
    Ok(QualifiedNativeEntryRoom {
        compiled,
        frame_binding,
        entry_start,
        map_revision_digest: crate::content::digest::sha256(
            super::native_spell_world::SOURCE_WORLD_MAP.as_bytes(),
        ),
        movement_cells: NativeEntryMovementCells {
            scope,
            index: crate::content::native_cell_lookup::NativeMovementCollisionIndex::Source(map),
            spell_tiles,
            house_tiles,
        },
        // Original definitions remain the baseline dependency. Their old physical
        // door placement is never admitted into this source map. Consumers must
        // select the source-world static owner instead of this baseline door.
        door: base.door,
    })
}
