use super::*;
use crate::content::encounter_map_item::{
    anchor_placement_key, lower_map_item_transforms, marker_placement,
};
use crate::content::{
    CanonicalReferencePlayableContent, ClientProjectionClass, ContentError, ContentLockBinding,
    ContentLockEntry, CoordinateFrameRef, DefinitionFamily, DefinitionRevisionRef,
    EvidenceBindingRef, EvidenceDisposition, LOCAL_OBJECT_TRANSFORM_INTENT_FAMILY,
    LocalObjectCollisionPresence, LocalObjectStateDefinition, MapRevisionRef,
    OwnerCapabilityRequirement, PackageManifestBinding, PlacementRef, ProductionAtom,
    REFERENCE_PLAYABLE_CAPABILITY_PROFILE, REFERENCE_PLAYABLE_CONTENT_PROFILE_ID,
    ReferenceDefinition, ReferenceDefinitionKind, ReferencePlayableContentSource, Sha256HexDigest,
    SpatialAddress, TransitionBinding, TransitionEventOwner, TypedDefinitionRef,
    link_reference_playable,
};
use crate::domain::interaction::RootSourceOccurrenceRef;
use crate::foundation::{
    ChannelId, CharacterId, CharacterLease, CommandId, CommandIngress, CommandRef,
    ConnectionGeneration, FreshAdmissionCommit, FreshAdmissionFacts, GameSessionAuthoritySnapshot,
    GameSessionId, GameSessionState, WorldId,
};
use crate::world_runtime::{
    LocalObjectCommand, LocalObjectUseOutcome, ScopeContentGenerationFence,
};
use oteryn_foundation::{ManualClock, Moment};
use serde_json::Value;
use std::error::Error;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const DUKE: &str = include_str!(
    "../../../../tools/content-schema/encounter-authoring/samples/the_duke_of_the_depths/encounter.json"
);
const DUKE_KEY: &str = "canary:encounter/the_duke_of_the_depths";
const DUKE_ACTION: &str = "canary:encounter/the_duke_of_the_depths/the_duke_of_the_depths_death/0";
const DUKE_DEATH: &str = "canary:occurrence/the_duke_of_the_depths/death/1";
const SEALED_ITEM: &str = "canary:item/1949";
const OPEN_ITEM: &str = "canary:item/22761";
const OPEN_TELEPORTER: &str = "oteryn:reference.transition.depth-teleporter-open";
// Synthetic, clearly-labeled state-only fixture (§7 Round 18): no authored sample qualifies.
const WALL_SEALED: &str = "oteryn:reference.state.timed-sealed-wall";
const WALL_CRACKED: &str = "oteryn:reference.state.timed-cracked-wall";
const CRACK: &str = "oteryn:reference.transition.timed-wall-crack";
const MEND: &str = "oteryn:reference.transition.timed-wall-mend";
const WALL_ACTION: &str = "oteryn:encounter/timed-wall/boss-death/0";
const WALL_OWNER: &str = "oteryn:encounter/timed-wall";
const WALL_A: &str = "oteryn:reference.placement.timed-wall-a";
const WALL_B: &str = "oteryn:reference.placement.timed-wall-b";

fn fixture<E>(_error: E) -> WorldRuntimeError {
    WorldRuntimeError::InvalidBinding("invalid §7 revert fixture")
}

fn uuid_v7(seed: u8) -> [u8; 16] {
    let mut bytes = [0_u8; 16];
    bytes[0] = 1;
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[15] = seed;
    bytes
}

fn world(seed: u8) -> Result<WorldId, WorldRuntimeError> {
    WorldId::decode(&uuid_v7(seed)).map_err(fixture)
}

fn scope() -> Result<RuntimeScopeRefV1, WorldRuntimeError> {
    Ok(RuntimeScopeRefV1::channel(
        world(1)?,
        ChannelId::decode(&uuid_v7(3)).map_err(fixture)?,
    ))
}

/// An active session in `scope()` at scope generation 1, for the session command path.
fn session() -> TestResult<(GameSessionAuthoritySnapshot<u64>, GameSessionId)> {
    let character = CharacterId::decode(&uuid_v7(90)).map_err(fixture)?;
    let session = GameSessionId::decode(&uuid_v7(10)).map_err(fixture)?;
    let mut nonce = [0_u8; 32];
    nonce[31] = 10;
    let channel = ChannelId::decode(&uuid_v7(3)).map_err(fixture)?;
    let facts =
        FreshAdmissionFacts::new(nonce, character, world(1)?, channel, 1, 1).map_err(fixture)?;
    let commit = FreshAdmissionCommit::from_facts(session, facts, 99_u64).map_err(fixture)?;
    let snapshot = GameSessionAuthoritySnapshot::new(
        commit,
        GameSessionState::Active,
        ConnectionGeneration::new(1).map_err(fixture)?,
        Some(99_u64),
        CharacterLease::new(character, 1).map_err(fixture)?,
        generation(1)?,
    );
    Ok((snapshot, session))
}

/// The durable session command path (`LocalObjectRuntime::apply`) for one transition.
fn apply_session(
    runtime: &mut LocalObjectRuntime,
    ingress: &mut CommandIngress,
    command_id: u64,
    transition: &str,
) -> TestResult<Result<String, WorldRuntimeError>> {
    let (authority, session) = session()?;
    let command = LocalObjectCommand::new(
        CommandRef::new(session, CommandId::new(command_id).map_err(fixture)?),
        ConnectionGeneration::new(1).map_err(fixture)?,
        runtime.placement_key().clone(),
        runtime.incarnation(),
        runtime.content_generation().clone(),
        LocalObjectOperation::new(TransitionKey::new(transition)?),
        runtime.revision(),
    );
    Ok(runtime
        .apply(&authority, &command, ingress, &BTreeSet::new())
        .map(|result| result.disposition().to_owned()))
}

fn generation(value: u64) -> Result<ScopeOwnershipGeneration, WorldRuntimeError> {
    ScopeOwnershipGeneration::new(value).map_err(fixture)
}

fn object_definition() -> Result<TypedDefinitionRef, ContentError> {
    Ok(TypedDefinitionRef::new(
        DefinitionFamily::LocalObject,
        ProductionKey::new("oteryn:reference.object.timed-object")?,
        DefinitionRevisionRef::new("definition-r1")?,
    ))
}

fn marker_definition() -> Result<TypedDefinitionRef, ContentError> {
    Ok(TypedDefinitionRef::new(
        DefinitionFamily::Terrain,
        ProductionKey::new("oteryn:reference.terrain.encounter-anchor-marker")?,
        DefinitionRevisionRef::new("definition-r1")?,
    ))
}

fn state(
    key: &str,
    collision: LocalObjectCollisionPresence,
) -> Result<LocalObjectStateDefinition, ContentError> {
    Ok(LocalObjectStateDefinition {
        key: ProductionKey::new(key)?,
        collision,
        attribute_variant_of: None,
        absent: false,
    })
}

fn transition(key: &str, source: &str, target: &str) -> Result<TransitionBinding, ContentError> {
    Ok(TransitionBinding {
        key: TransitionKey::new(key)?,
        definition: object_definition()?,
        source_state: ProductionKey::new(source)?,
        normalized_intent_family: ProductionKey::new(LOCAL_OBJECT_TRANSFORM_INTENT_FAMILY)?,
        target_state: ProductionKey::new(target)?,
        owner_capability: OwnerCapabilityRequirement {
            capability_key: ProductionKey::new(
                "oteryn:runtime.capability.local-object-transition",
            )?,
        },
        policy_guard_refs: vec![],
    })
}

fn source_with(
    states: Vec<LocalObjectStateDefinition>,
    transitions: Vec<TransitionBinding>,
    lock: &str,
) -> TestResult<ReferencePlayableContentSource> {
    let package_key = ProductionKey::new("oteryn:content.timed-revert")?;
    let package_revision = ProductionAtom::new("package revision", "package-r1")?;
    let package_manifest = PackageManifestBinding::new(
        package_key.clone(),
        package_revision.clone(),
        ProductionAtom::new("schema", "schema-v1")?,
        ProductionAtom::new("license", "license:project-owned-v1")?,
        Sha256HexDigest::new("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd")?,
    );
    let provenance = package_manifest.package_provenance_digest()?;
    Ok(ReferencePlayableContentSource {
        profile_revision: ProductionAtom::new("profile", REFERENCE_PLAYABLE_CONTENT_PROFILE_ID)?,
        capability_profile: ProductionAtom::new(
            "capability profile",
            REFERENCE_PLAYABLE_CAPABILITY_PROFILE,
        )?,
        package_manifest,
        content_lock: ContentLockBinding {
            revision_digest_token: ProductionAtom::new("content lock", lock)?,
            entries: vec![ContentLockEntry::exact(
                package_key,
                package_revision,
                provenance,
            )],
        },
        world_id: world(1)?,
        coordinate_frame: CoordinateFrameRef::new("global-target-2026-09-27")?,
        definitions: vec![
            ReferenceDefinition {
                definition: object_definition()?,
                kind: ReferenceDefinitionKind::LocalObjectStates(states),
                client_projection: ClientProjectionClass::ClientSafe,
            },
            ReferenceDefinition {
                definition: marker_definition()?,
                kind: ReferenceDefinitionKind::Generic,
                client_projection: ClientProjectionClass::ClientSafe,
            },
        ],
        placements: vec![],
        ordered_placements: vec![],
        transitions,
    })
}

/// A deliberately unpromoted placement injected after linking, as the CW4 fixtures do.
fn placement_at(
    content: &CanonicalReferencePlayableContent,
    key: PlacementKey,
    definition: TypedDefinitionRef,
    x: i32,
) -> TestResult<PlacementRef> {
    let witness = EvidenceBindingRef::new(
        ProductionAtom::new("reference manifest revision", "manifest-r0")?,
        ProductionKey::new("oteryn:cw4.timed-revert-placement")?,
        EvidenceDisposition::Unknown,
    );
    Ok(marker_placement(
        key,
        definition,
        MapRevisionRef::new("map-r1")?,
        SpatialAddress {
            world_id: content.world_id,
            coordinate_frame: content.coordinate_frame.clone(),
            cell: LogicalCell { x, y: 10, z: 7 },
            evidence: witness,
        },
    ))
}

fn object_at(
    content: &CanonicalReferencePlayableContent,
    key: &PlacementKey,
    initial: &str,
    x: i32,
) -> TestResult<PlacementRef> {
    let mut placement = placement_at(content, key.clone(), object_definition()?, x)?;
    placement.local_object_initial_state = Some(ProductionKey::new(initial)?);
    Ok(placement)
}

fn duke_anchor() -> Result<PlacementKey, ContentError> {
    anchor_placement_key(DUKE_KEY, "exit_teleporter")
}

fn duke_destination(anchor: &str) -> Result<PlacementKey, ContentError> {
    anchor_placement_key(DUKE_KEY, anchor)
}

/// D90: the lowered re-arm forward (post-revert variant -> open), bound beside the forward.
fn duke_rearm() -> String {
    format!("{DUKE_ACTION}/rearm")
}

/// `the_duke_of_the_depths` lowered by task A's §9 lowering, linked, with its anchor and
/// destination markers injected. Returns the content and the dedicated inverse.
fn duke_content(encounter: &str) -> TestResult<(CanonicalReferencePlayableContent, TransitionKey)> {
    let mut source = source_with(
        vec![
            state(SEALED_ITEM, LocalObjectCollisionPresence::Absent)?,
            state(OPEN_ITEM, LocalObjectCollisionPresence::Absent)?,
        ],
        vec![transition(OPEN_TELEPORTER, SEALED_ITEM, OPEN_ITEM)?],
        "lock:timed-revert-r1",
    )?;
    let anchors = BTreeMap::from([("exit_teleporter".to_owned(), object_definition()?)]);
    let lowered = lower_map_item_transforms(encounter, &source, &anchors)?;
    lowered.apply_to_source(&mut source)?;
    let mut content = link_reference_playable(source)?;
    let anchor = duke_anchor()?;
    let tables = lowered
        .placements
        .get(&anchor)
        .ok_or(fixture("lowered anchor tables"))?;
    let mut placement = object_at(&content, &anchor, SEALED_ITEM, 100)?;
    placement.local_object_state_attributes = tables.state_attributes.clone();
    placement.local_object_revert_after_ms = tables.revert_after_ms.clone();
    placement.local_object_event_transitions = tables.event_transitions.clone();
    let mut placements = vec![placement];
    for (x, marker) in (200..).zip(&lowered.destination_markers) {
        placements.push(placement_at(
            &content,
            marker.clone(),
            marker_definition()?,
            x,
        )?);
    }
    content.placements = placements;
    let revert = TransitionKey::new(&format!("{DUKE_ACTION}/revert"))?;
    Ok((content, revert))
}

/// Synthetic walls: `sealed` (blocking) --CRACK--> `cracked` --MEND--> `sealed`, with the
/// given `(transition, action, ms)` revert durations on every placement. D91: when any
/// duration is given, CRACK and MEND are both event-owned by `WALL_OWNER`; untimed walls keep
/// both `PlayerUse`.
fn wall_content(
    timed: &[(&str, &str, u64)],
    placements: &[(&str, i32)],
    lock: &str,
) -> TestResult<CanonicalReferencePlayableContent> {
    let mut content = link_reference_playable(source_with(
        vec![
            state(WALL_SEALED, LocalObjectCollisionPresence::Present)?,
            state(WALL_CRACKED, LocalObjectCollisionPresence::Absent)?,
        ],
        vec![
            transition(CRACK, WALL_SEALED, WALL_CRACKED)?,
            transition(MEND, WALL_CRACKED, WALL_SEALED)?,
        ],
        lock,
    )?)?;
    let mut injected = Vec::new();
    for (key, x) in placements {
        let mut placement = object_at(&content, &PlacementKey::new(key)?, WALL_SEALED, *x)?;
        for (transition, action, ms) in timed {
            placement.local_object_revert_after_ms.insert(
                (
                    TransitionKey::new(transition)?,
                    LoweredActionId::new(action)?,
                ),
                *ms,
            );
        }
        if !timed.is_empty() {
            for transition in [CRACK, MEND] {
                placement.local_object_event_transitions.insert(
                    TransitionKey::new(transition)?,
                    TransitionEventOwner::new(WALL_OWNER)?,
                );
            }
        }
        injected.push(placement);
    }
    content.placements = injected;
    Ok(content)
}

fn bind_at(
    content: &CanonicalReferencePlayableContent,
    placement: &PlacementKey,
    transitions: &[&str],
    incarnation: u64,
) -> Result<LocalObjectRuntime, WorldRuntimeError> {
    bind_in(
        content,
        placement,
        transitions,
        incarnation,
        scope()?,
        generation(1)?,
    )
}

/// `bind_at` under an explicit scope and scope generation.
fn bind_in(
    content: &CanonicalReferencePlayableContent,
    placement: &PlacementKey,
    transitions: &[&str],
    incarnation: u64,
    scope: RuntimeScopeRefV1,
    scope_generation: ScopeOwnershipGeneration,
) -> Result<LocalObjectRuntime, WorldRuntimeError> {
    let fence = ScopeContentGenerationFence::for_test(
        scope,
        scope_generation,
        ReferenceContentGeneration::from_content(content)?,
    );
    let keys = transitions
        .iter()
        .copied()
        .map(TransitionKey::new)
        .collect::<Result<Vec<_>, _>>()?;
    LocalObjectRuntime::bind(
        content,
        &fence,
        scope,
        scope_generation,
        placement,
        incarnation,
        &keys,
    )
}

fn walls(
    content: &CanonicalReferencePlayableContent,
    keys: &[&str],
) -> TestResult<BTreeMap<PlacementKey, LocalObjectRuntime>> {
    let mut runtimes = BTreeMap::new();
    for key in keys {
        let key = PlacementKey::new(key)?;
        runtimes.insert(key.clone(), bind_at(content, &key, &[CRACK, MEND], 1)?);
    }
    Ok(runtimes)
}

fn driver(
    clock: &ManualClock,
    limits: RevertDriverLimits,
) -> TestResult<ScopeRevertDriver<TestIssuer>> {
    Ok(ScopeRevertDriver::new(
        scope()?,
        TestIssuer::new(generation(1)?),
        Arc::new(clock.clone()),
        limits,
    ))
}

/// A registered-limits driver owned by `scope` at `scope_generation`.
fn driver_in(
    clock: &ManualClock,
    scope: RuntimeScopeRefV1,
    scope_generation: ScopeOwnershipGeneration,
) -> ScopeRevertDriver<TestIssuer> {
    ScopeRevertDriver::new(
        scope,
        TestIssuer::new(scope_generation),
        Arc::new(clock.clone()),
        RevertDriverLimits::registered(),
    )
}

/// A scope operation executed by the fixture event that owns `transition`: the timed
/// wall's owner for CRACK/MEND, the duke encounter for every other edge.
fn operation(
    runtime: &LocalObjectRuntime,
    transition: &str,
    expected_revision: u64,
) -> TestResult<ScopeLocalObjectOperation> {
    let owner = if transition == CRACK || transition == MEND {
        WALL_OWNER
    } else {
        DUKE_KEY
    };
    operation_owned(
        runtime,
        transition,
        expected_revision,
        &TransitionEventOwner::new(owner)?,
    )
}

fn operation_owned(
    runtime: &LocalObjectRuntime,
    transition: &str,
    expected_revision: u64,
    owner: &TransitionEventOwner,
) -> TestResult<ScopeLocalObjectOperation> {
    Ok(ScopeLocalObjectOperation::new(
        runtime.placement_key().clone(),
        runtime.incarnation(),
        runtime.content_generation().clone(),
        LocalObjectOperation::new(TransitionKey::new(transition)?),
        expected_revision,
        owner.clone(),
    ))
}

fn forward(
    driver: &mut ScopeRevertDriver<TestIssuer>,
    runtimes: &mut BTreeMap<PlacementKey, LocalObjectRuntime>,
    placement: &str,
    occurrence: &str,
    action: &str,
    transition: &str,
) -> TestResult<ForwardOutcome> {
    let runtime = runtimes
        .get_mut(&PlacementKey::new(placement)?)
        .ok_or(fixture("runtime"))?;
    let operation = operation(runtime, transition, runtime.revision())?;
    Ok(driver.apply_forward(
        runtime,
        &forward_child(occurrence, None)?,
        &revisions()?,
        &LoweredActionId::new(action)?,
        &operation,
        &BTreeSet::new(),
    )?)
}

fn revisions() -> Result<SemanticRevisionContext, InteractionError> {
    SemanticRevisionContext::new("content:timed-revert-r1", "ruleset:r1", "sim:v1")
}

/// The forward operation's own GAME-INTERACTION child (an encounter action under its trigger
/// root), as the encounter owner would supply it.
fn forward_child(root: &str, ordinal: Option<u16>) -> Result<ChildOccurrenceRef, InteractionError> {
    ChildOccurrenceRef::for_root(
        &RootSourceOccurrenceRef::new(root)?,
        "encounter:map_item",
        "encounter:anchor",
        "transform",
        ordinal,
        &revisions()?,
    )
}

fn scheduled(outcome: ForwardOutcome) -> Result<ChildOccurrenceRef, WorldRuntimeError> {
    outcome.scheduled.ok_or(fixture("no revert scheduled"))
}

/// An untimed scope operation applied directly (another input in the meantime).
fn apply_plain(runtime: &mut LocalObjectRuntime, transition: &str) -> TestResult<String> {
    let operation = operation(runtime, transition, runtime.revision())?;
    let staged = runtime.apply_scope_operation(
        scope()?,
        generation(1)?,
        &operation,
        &BTreeSet::new(),
        None,
        |_| Ok::<(), Infallible>(()),
    )?;
    let outcome = match staged {
        Ok(outcome) => outcome,
        Err(never) => match never {},
    };
    Ok(outcome.disposition().to_owned())
}

fn state_of(
    runtimes: &BTreeMap<PlacementKey, LocalObjectRuntime>,
    placement: &str,
) -> TestResult<(String, u64)> {
    let runtime = runtimes
        .get(&PlacementKey::new(placement)?)
        .ok_or(fixture("runtime"))?;
    Ok((runtime.state_key().as_str().to_owned(), runtime.revision()))
}

fn at(state: &str, revision: u64) -> (String, u64) {
    (state.to_owned(), revision)
}

fn destination_of(runtime: &LocalObjectRuntime) -> Option<&str> {
    runtime
        .attributes()
        .and_then(|attributes| attributes.destination.as_ref())
        .map(PlacementKey::as_str)
}

fn disposition(outcome: &RevertOutcome) -> Option<&str> {
    match outcome {
        RevertOutcome::Prepared(outcome) => Some(outcome.disposition()),
        RevertOutcome::Fenced(_) => None,
    }
}

fn millis(ms: u64) -> Duration {
    Duration::from_millis(ms)
}

// Include topic files in this module to preserve test paths and private fixture access.
include!("world_object_revert_tests_duke.rs");
include!("world_object_revert_tests_lifecycle.rs");
include!("world_object_revert_tests_authority.rs");
include!("world_object_revert_tests_scheduling.rs");
include!("world_object_revert_tests_create.rs");
include!("world_object_revert_tests_rearm.rs");
