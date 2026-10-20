use crate::content::encounter_map_item::*;
use crate::content::{
    CanonicalReferencePlayableContent, ClientProjectionClass, ContentError, ContentLockBinding,
    ContentLockEntry, CoordinateFrameRef, DefinitionFamily, DefinitionRevisionRef,
    EvidenceBindingRef, EvidenceDisposition, LOCAL_OBJECT_CREATE_INTENT_FAMILY,
    LOCAL_OBJECT_OPEN_INTENT_FAMILY, LOCAL_OBJECT_REMOVE_INTENT_FAMILY,
    LOCAL_OBJECT_TRANSFORM_INTENT_FAMILY, LocalObjectCollisionPresence, LocalObjectStateAttributes,
    LocalObjectStateDefinition, LogicalCell, LoweredActionId, MapRevisionRef,
    OwnerCapabilityRequirement, PackageManifestBinding, PlacementKey, PlacementRef, ProductionAtom,
    ProductionKey, REFERENCE_PLAYABLE_CAPABILITY_PROFILE, REFERENCE_PLAYABLE_CONTENT_PROFILE_ID,
    ReferenceDefinition, ReferenceDefinitionKind, ReferencePlayableContentSource, Sha256HexDigest,
    SpatialAddress, TransitionBinding, TransitionEventOwner, TransitionKey, TransitionOrigin,
    TypedDefinitionRef, link_reference_playable,
};
use crate::foundation::{
    ChannelId, CharacterId, CharacterLease, CommandId, CommandIngress, CommandRef,
    ConnectionGeneration, FreshAdmissionCommit, FreshAdmissionFacts, GameSessionAuthoritySnapshot,
    GameSessionId, GameSessionState, RuntimeScopeRefV1, ScopeOwnershipGeneration, WorldId,
};
use crate::world_runtime::{
    LocalObjectCommand, LocalObjectOperation, LocalObjectRuntime, ReferenceContentGeneration,
    ScopeContentGenerationFence, WorldRuntimeError,
};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const DUKE: &str = include_str!(
    "../../../../tools/content-schema/encounter-authoring/samples/the_duke_of_the_depths/encounter.json"
);
const LORD_OF_THE_LICE: &str = include_str!(
    "../../../../tools/content-schema/encounter-authoring/samples/the_lord_of_the_lice/encounter.json"
);
const OPEN_DECISION_SAMPLES: [(&str, &str); 6] = [
    (
        "death_priest_shargon",
        include_str!(
            "../../../../tools/content-schema/encounter-authoring/samples/death_priest_shargon/encounter.json"
        ),
    ),
    (
        "the_ravager",
        include_str!(
            "../../../../tools/content-schema/encounter-authoring/samples/the_ravager/encounter.json"
        ),
    ),
    (
        "mazzinor",
        include_str!(
            "../../../../tools/content-schema/encounter-authoring/samples/mazzinor/encounter.json"
        ),
    ),
    (
        "gaz_haragoth",
        include_str!(
            "../../../../tools/content-schema/encounter-authoring/samples/gaz_haragoth/encounter.json"
        ),
    ),
    (
        "cult_soul_remains",
        include_str!(
            "../../../../tools/content-schema/encounter-authoring/samples/cult_soul_remains/encounter.json"
        ),
    ),
    (
        "azerus",
        include_str!(
            "../../../../tools/content-schema/encounter-authoring/samples/azerus/encounter.json"
        ),
    ),
];

const DUKE_KEY: &str = "canary:encounter/the_duke_of_the_depths";
const DUKE_ACTION: &str = "canary:encounter/the_duke_of_the_depths/the_duke_of_the_depths_death/0";
const SEALED_ITEM: &str = "canary:item/1949";
const OPEN_ITEM: &str = "canary:item/22761";
const FORWARD: &str = "oteryn:reference.transition.depth-teleporter-open";
const PLAIN_INVERSE: &str = "oteryn:reference.transition.depth-teleporter-seal";
const VARIANT_INVERSE: &str = "oteryn:reference.transition.depth-teleporter-seal-variant";
const SEALED: &str = "oteryn:reference.state.sealed";
const OPEN: &str = "oteryn:reference.state.open";
const SEALED_VARIANT: &str = "oteryn:reference.state.sealed-variant";
const PLACEMENT_A: &str = "oteryn:reference.placement.depth-teleporter-a";
const PLACEMENT_B: &str = "oteryn:reference.placement.depth-teleporter-b";
const ACTION_A: &str = "oteryn:encounter/depth/rule/0";
const EVENT_OWNER: &str = "oteryn:encounter/depth";
const NO_INVERSE: &str = "revert_after_ms transition has no bound inverse at this placement";

fn fixture<E>(_error: E) -> WorldRuntimeError {
    WorldRuntimeError::InvalidBinding("invalid §9 lowering fixture")
}

fn uuid_v7(seed: u8) -> [u8; 16] {
    let mut bytes = [0_u8; 16];
    bytes[0] = 1;
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[15] = seed;
    bytes
}

fn world() -> Result<WorldId, WorldRuntimeError> {
    WorldId::decode(&uuid_v7(1)).map_err(fixture)
}

fn teleporter() -> Result<TypedDefinitionRef, ContentError> {
    Ok(TypedDefinitionRef::new(
        DefinitionFamily::LocalObject,
        ProductionKey::new("oteryn:reference.object.depth-teleporter")?,
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
    variant_of: Option<&str>,
) -> Result<LocalObjectStateDefinition, ContentError> {
    Ok(LocalObjectStateDefinition {
        key: ProductionKey::new(key)?,
        collision,
        attribute_variant_of: variant_of.map(ProductionKey::new).transpose()?,
        absent: false,
    })
}

fn transition(key: &str, source: &str, target: &str) -> Result<TransitionBinding, ContentError> {
    Ok(TransitionBinding {
        key: TransitionKey::new(key)?,
        definition: teleporter()?,
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
) -> TestResult<ReferencePlayableContentSource> {
    let package_key = ProductionKey::new("oteryn:content.encounter-map-item")?;
    let package_revision = ProductionAtom::new("package revision", "package-r1")?;
    let package_manifest = PackageManifestBinding::new(
        package_key.clone(),
        package_revision.clone(),
        ProductionAtom::new("schema", "schema-v1")?,
        ProductionAtom::new("license", "license:project-owned-v1")?,
        Sha256HexDigest::new("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc")?,
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
            revision_digest_token: ProductionAtom::new("content lock", "lock:map-item-r1")?,
            entries: vec![ContentLockEntry::exact(
                package_key,
                package_revision,
                provenance,
            )],
        },
        world_id: world()?,
        coordinate_frame: CoordinateFrameRef::new("global-target-2026-09-27")?,
        definitions: vec![
            ReferenceDefinition {
                definition: teleporter()?,
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

/// A deliberately unpromoted placement injected after linking, exactly as the CW4 runtime
/// fixtures do (no accepted CONTENT_WORLD case can promote a placement claim yet).
fn placement_at(
    content: &CanonicalReferencePlayableContent,
    key: PlacementKey,
    definition: TypedDefinitionRef,
    x: i32,
) -> TestResult<PlacementRef> {
    let witness = EvidenceBindingRef::new(
        ProductionAtom::new("reference manifest revision", "manifest-r0")?,
        ProductionKey::new("oteryn:cw4.encounter-map-item-placement")?,
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

fn teleporter_placement(
    content: &CanonicalReferencePlayableContent,
    key: &str,
    initial: &str,
    x: i32,
) -> TestResult<PlacementRef> {
    let mut placement = placement_at(content, PlacementKey::new(key)?, teleporter()?, x)?;
    placement.local_object_initial_state = Some(ProductionKey::new(initial)?);
    Ok(placement)
}

fn duke_source() -> TestResult<ReferencePlayableContentSource> {
    source_with(
        vec![
            state(SEALED_ITEM, LocalObjectCollisionPresence::Absent, None)?,
            state(OPEN_ITEM, LocalObjectCollisionPresence::Absent, None)?,
        ],
        vec![transition(FORWARD, SEALED_ITEM, OPEN_ITEM)?],
    )
}

fn duke_anchors() -> TestResult<BTreeMap<String, TypedDefinitionRef>> {
    Ok(BTreeMap::from([(
        "exit_teleporter".to_owned(),
        teleporter()?,
    )]))
}

fn duke_anchor() -> Result<PlacementKey, ContentError> {
    anchor_placement_key(DUKE_KEY, "exit_teleporter")
}

fn duke_post_revert() -> Result<ProductionKey, ContentError> {
    ProductionKey::new(&format!("{DUKE_ACTION}/post-revert"))
}

fn duke_revert() -> Result<TransitionKey, ContentError> {
    TransitionKey::new(&format!("{DUKE_ACTION}/revert"))
}

fn duke_rearm() -> Result<TransitionKey, ContentError> {
    TransitionKey::new(&format!("{DUKE_ACTION}/rearm"))
}

/// The duke sample lowered and linked, with its anchor and destination markers injected.
fn duke_content() -> TestResult<CanonicalReferencePlayableContent> {
    let mut source = duke_source()?;
    let lowered = lower_map_item_transforms(DUKE, &source, &duke_anchors()?)?;
    lowered.apply_to_source(&mut source)?;
    let mut content = link_reference_playable(source)?;
    let anchor = duke_anchor()?;
    let tables = lowered
        .placements
        .get(&anchor)
        .ok_or(fixture("lowered anchor tables"))?;
    let mut placement = teleporter_placement(&content, anchor.as_str(), SEALED_ITEM, 100)?;
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
    Ok(content)
}

fn authority(
    session_seed: u8,
) -> Result<
    (
        GameSessionAuthoritySnapshot<u64>,
        GameSessionId,
        RuntimeScopeRefV1,
    ),
    WorldRuntimeError,
> {
    let world = world()?;
    let channel = ChannelId::decode(&uuid_v7(3)).map_err(fixture)?;
    let character =
        CharacterId::decode(&uuid_v7(session_seed.wrapping_add(80))).map_err(fixture)?;
    let session = GameSessionId::decode(&uuid_v7(session_seed)).map_err(fixture)?;
    let mut nonce = [0_u8; 32];
    nonce[31] = session_seed.max(1);
    let facts =
        FreshAdmissionFacts::new(nonce, character, world, channel, 1, 1).map_err(fixture)?;
    let commit = FreshAdmissionCommit::from_facts(session, facts, 99_u64).map_err(fixture)?;
    let snapshot = GameSessionAuthoritySnapshot::new(
        commit,
        GameSessionState::Active,
        ConnectionGeneration::new(1).map_err(fixture)?,
        Some(99_u64),
        CharacterLease::new(character, 1).map_err(fixture)?,
        ScopeOwnershipGeneration::new(1).map_err(fixture)?,
    );
    Ok((
        snapshot,
        session,
        RuntimeScopeRefV1::channel(world, channel),
    ))
}

fn bind(
    content: &CanonicalReferencePlayableContent,
    placement: &str,
    transitions: &[&str],
) -> Result<LocalObjectRuntime, WorldRuntimeError> {
    let (_, _, scope) = authority(10)?;
    let generation = ScopeOwnershipGeneration::new(1).map_err(fixture)?;
    let fence = ScopeContentGenerationFence::for_test(
        scope,
        generation,
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
        generation,
        &PlacementKey::new(placement)?,
        1,
        &keys,
    )
}

fn rejects(result: Result<LocalObjectRuntime, WorldRuntimeError>, reason: &str) -> bool {
    matches!(result, Err(WorldRuntimeError::InvalidBinding(actual)) if actual == reason)
}

fn invoke(
    runtime: &mut LocalObjectRuntime,
    ingress: &mut CommandIngress,
    command_id: u64,
    transition: &str,
) -> TestResult<String> {
    let (authority, session, _) = authority(10)?;
    let command = LocalObjectCommand::new(
        CommandRef::new(session, CommandId::new(command_id).map_err(fixture)?),
        ConnectionGeneration::new(1).map_err(fixture)?,
        runtime.placement_key().clone(),
        runtime.incarnation(),
        runtime.content_generation().clone(),
        LocalObjectOperation::new(TransitionKey::new(transition)?),
        runtime.revision(),
    );
    let result = runtime.apply(&authority, &command, ingress, &BTreeSet::new())?;
    Ok(result.disposition().to_owned())
}

fn destination_of(runtime: &LocalObjectRuntime) -> Option<&str> {
    runtime
        .attributes()
        .and_then(|attributes| attributes.destination.as_ref())
        .map(PlacementKey::as_str)
}

fn duke_with(edit: impl FnOnce(&mut Vec<Value>) -> Option<()>) -> TestResult<String> {
    let mut encounter: Value = serde_json::from_str(DUKE)?;
    let actions = encounter
        .pointer_mut("/rules/0/actions")
        .and_then(Value::as_array_mut)
        .ok_or(fixture("duke actions"))?;
    edit(actions).ok_or(fixture("duke action edit"))?;
    Ok(serde_json::to_string(&encounter)?)
}

fn edit_first(edit: impl FnOnce(&mut Map<String, Value>)) -> TestResult<String> {
    duke_with(|actions| {
        edit(actions.first_mut()?.as_object_mut()?);
        Some(())
    })
}

/// Plain (not lowered) teleporter content for the §7/§9 bind-rule cases: a `sealed -> open`
/// forward, a plain `open -> sealed` inverse and an `open -> sealed-variant` candidate.
fn plain_content(
    sealed_variant_of: Option<&str>,
    variant_variant_of: Option<&str>,
) -> TestResult<CanonicalReferencePlayableContent> {
    Ok(link_reference_playable(source_with(
        vec![
            state(
                SEALED,
                LocalObjectCollisionPresence::Present,
                sealed_variant_of,
            )?,
            state(OPEN, LocalObjectCollisionPresence::Absent, None)?,
            state(
                SEALED_VARIANT,
                LocalObjectCollisionPresence::Present,
                variant_variant_of,
            )?,
        ],
        vec![
            transition(FORWARD, SEALED, OPEN)?,
            transition(PLAIN_INVERSE, OPEN, SEALED)?,
            transition(VARIANT_INVERSE, OPEN, SEALED_VARIANT)?,
        ],
    )?)?)
}

/// Injects placement A (carrying `durations`) and placement B (carrying none). When
/// `durations` is non-empty, every content transition is event-owned at A by `EVENT_OWNER`
/// (D91: a timed forward and its inverse must be); B keeps every edge `PlayerUse`.
fn with_revert(
    mut content: CanonicalReferencePlayableContent,
    durations: &[(&str, &str, u64)],
) -> TestResult<CanonicalReferencePlayableContent> {
    let mut a = teleporter_placement(&content, PLACEMENT_A, SEALED, 100)?;
    let b = teleporter_placement(&content, PLACEMENT_B, SEALED, 300)?;
    for (transition, action, ms) in durations {
        a.local_object_revert_after_ms.insert(
            (
                TransitionKey::new(transition)?,
                LoweredActionId::new(action)?,
            ),
            *ms,
        );
    }
    if !durations.is_empty() {
        for transition in &content.transitions {
            a.local_object_event_transitions.insert(
                transition.key.clone(),
                TransitionEventOwner::new(EVENT_OWNER)?,
            );
        }
    }
    content.placements = vec![a, b];
    Ok(content)
}

#[test]
fn duke_sample_lowers_destination_post_revert_variant_and_dedicated_inverse() -> TestResult {
    let lowered = lower_map_item_transforms(DUKE, &duke_source()?, &duke_anchors()?)?;
    let tables = lowered
        .placements
        .get(&duke_anchor()?)
        .ok_or(fixture("anchor tables"))?;
    let destination = |key: &ProductionKey| {
        tables
            .state_attributes
            .get(key)
            .map(|attributes| attributes.destination.clone())
    };
    let reward = anchor_placement_key(DUKE_KEY, "reward_destination")?;
    let warzone = anchor_placement_key(DUKE_KEY, "warzone_exit")?;

    // The natural source state has no entry: `revert_destination` is never baked into it.
    assert_eq!(destination(&ProductionKey::new(SEALED_ITEM)?), None);
    assert_eq!(
        destination(&ProductionKey::new(OPEN_ITEM)?),
        Some(Some(reward.clone()))
    );
    assert_eq!(
        destination(&duke_post_revert()?),
        Some(Some(warzone.clone()))
    );
    assert_eq!(tables.state_attributes.len(), 2);
    assert_eq!(
        lowered.post_revert_states,
        vec![(
            teleporter()?,
            state(
                &format!("{DUKE_ACTION}/post-revert"),
                LocalObjectCollisionPresence::Absent,
                Some(SEALED_ITEM),
            )?
        )]
    );
    assert_eq!(
        lowered.inverse_transitions,
        vec![transition(
            &format!("{DUKE_ACTION}/revert"),
            OPEN_ITEM,
            &format!("{DUKE_ACTION}/post-revert"),
        )?]
    );
    // D90: the re-arm forward leaves the post-revert variant for the forward's target state
    // and carries the same authored duration under its own key.
    assert_eq!(
        lowered.rearm_transitions,
        vec![transition(
            &format!("{DUKE_ACTION}/rearm"),
            &format!("{DUKE_ACTION}/post-revert"),
            OPEN_ITEM,
        )?]
    );
    let action = LoweredActionId::new(DUKE_ACTION)?;
    assert_eq!(
        tables.revert_after_ms,
        BTreeMap::from([
            ((TransitionKey::new(FORWARD)?, action.clone()), 1_200_000),
            ((duke_rearm()?, action), 1_200_000),
        ])
    );
    // D91: the forward, the dedicated inverse and the re-arm forward are all event-owned at
    // the anchor by the encounter.
    let owner = TransitionEventOwner::new(DUKE_KEY)?;
    assert_eq!(
        tables.event_transitions,
        BTreeMap::from([
            (TransitionKey::new(FORWARD)?, owner.clone()),
            (duke_revert()?, owner.clone()),
            (duke_rearm()?, owner),
        ])
    );
    assert_eq!(
        lowered.destination_markers,
        BTreeSet::from([reward, warzone])
    );
    Ok(())
}

#[test]
fn duke_lowered_content_binds_under_the_widened_rule_and_exposes_attributes_by_state() -> TestResult
{
    let content = duke_content()?;
    let anchor = duke_anchor()?;
    let revert = duke_revert()?;
    let rearm = duke_rearm()?;
    let action = LoweredActionId::new(DUKE_ACTION)?;
    let mut runtime = bind(
        &content,
        anchor.as_str(),
        &[FORWARD, rearm.as_str(), revert.as_str()],
    )?;
    let mut ingress = CommandIngress::new();

    // D90: the widened unique-inverse search accepts the re-arm forward unchanged; the
    // authored forward and the re-arm forward share the dedicated revert as their inverse.
    assert_eq!(
        runtime.revert_inverse(&TransitionKey::new(FORWARD)?),
        Some(&revert)
    );
    assert_eq!(runtime.revert_inverse(&rearm), Some(&revert));
    assert_eq!(runtime.revert_after_ms(&rearm, &action), Some(1_200_000));

    // Freshly bound in the natural source state: no destination is reachable.
    assert_eq!(runtime.state_key().as_str(), SEALED_ITEM);
    assert_eq!(runtime.attributes(), None);
    assert_eq!(
        runtime.revert_after_ms(&TransitionKey::new(FORWARD)?, &action),
        Some(1_200_000)
    );
    assert_eq!(runtime.revert_after_ms(&revert, &action), None);

    // #1144 P1 4125535249: the timed forward is not session-invocable (the session path never
    // schedules its revert), refused before ingress or mutation like an unbound transition.
    let refused = invoke(&mut runtime, &mut ingress, 1, FORWARD)
        .err()
        .and_then(|error| error.downcast::<WorldRuntimeError>().ok());
    assert!(matches!(
        refused.as_deref(),
        Some(WorldRuntimeError::InvalidBinding(
            "command names a transition this local-object runtime does not bind"
        ))
    ));
    assert_eq!(ingress.outstanding(), 0);
    assert_eq!(runtime.revision(), 0);

    // #1144 P1 4125881398: a scope operation without the driver's scheduling capability is
    // refused too, before mutation.
    let (_, _, scope) = authority(10)?;
    let generation = ScopeOwnershipGeneration::new(1).map_err(fixture)?;
    let forward = crate::world_runtime::ScopeLocalObjectOperation::new(
        runtime.placement_key().clone(),
        runtime.incarnation(),
        runtime.content_generation().clone(),
        LocalObjectOperation::new(TransitionKey::new(FORWARD)?),
        runtime.revision(),
        TransitionEventOwner::new(DUKE_KEY)?,
    );
    assert!(matches!(
        runtime.apply_scope_operation(
            scope,
            generation,
            &forward,
            &BTreeSet::new(),
            None,
            |_| Ok::<(), std::convert::Infallible>(()),
        ),
        Err(WorldRuntimeError::InvalidBinding(
            "timed transition commits only through the revert scheduler"
        ))
    ));
    assert_eq!(runtime.revision(), 0);

    // It commits only through the §7 revert driver, which records its revert.
    let clock = oteryn_foundation::ManualClock::new(oteryn_foundation::Moment::ZERO);
    let mut driver = crate::world_object_revert::ScopeRevertDriver::new(
        scope,
        crate::world_object_revert::TestIssuer::new(generation),
        std::sync::Arc::new(clock),
        crate::world_object_revert::RevertDriverLimits::registered(),
    );
    let revisions =
        crate::interaction::SemanticRevisionContext::new("content:r1", "ruleset:r1", "sim:v1")?;
    let death = crate::interaction::ChildOccurrenceRef::for_root(
        &crate::interaction::RootSourceOccurrenceRef::new("encounter:duke-death/1")?,
        "encounter:map_item",
        "encounter:anchor",
        "transform",
        None,
        &revisions,
    )?;
    let committed = driver.apply_forward(
        &mut runtime,
        &death,
        &revisions,
        &action,
        &forward,
        &BTreeSet::new(),
    )?;
    assert_eq!(committed.outcome.disposition(), "COMMITTED");
    assert!(committed.scheduled.is_some());
    assert_eq!(driver.record_count(), 1);
    assert_eq!(
        destination_of(&runtime),
        Some(anchor_placement_key(DUKE_KEY, "reward_destination")?.as_str())
    );

    // D91: the untimed dedicated inverse is event-owned too, so the open teleporter offers
    // USE nothing and a session command naming the inverse is refused before ingress.
    assert_eq!(
        runtime.attempt_use(1, &BTreeSet::new())?,
        crate::world_runtime::LocalObjectUseOutcome::NothingToUse
    );
    let refused = invoke(&mut runtime, &mut ingress, 1, revert.as_str())
        .err()
        .and_then(|error| error.downcast::<WorldRuntimeError>().ok());
    assert!(matches!(
        refused.as_deref(),
        Some(WorldRuntimeError::InvalidBinding(
            "command names a transition this local-object runtime does not bind"
        ))
    ));
    assert_eq!(ingress.outstanding(), 0);
    assert_eq!(runtime.revision(), 1);

    // Its owning event's scope operation lands it on the post-revert variant, which exposes
    // `revert_destination`.
    let inverse = crate::world_runtime::ScopeLocalObjectOperation::new(
        runtime.placement_key().clone(),
        runtime.incarnation(),
        runtime.content_generation().clone(),
        LocalObjectOperation::new(revert.clone()),
        runtime.revision(),
        TransitionEventOwner::new(DUKE_KEY)?,
    );
    let reverted = runtime.apply_scope_operation(
        scope,
        generation,
        &inverse,
        &BTreeSet::new(),
        None,
        |_| Ok::<(), std::convert::Infallible>(()),
    )?;
    assert!(matches!(reverted, Ok(outcome) if outcome.disposition() == "COMMITTED"));
    assert_eq!(runtime.state_key(), &duke_post_revert()?);
    assert_eq!(
        destination_of(&runtime),
        Some(anchor_placement_key(DUKE_KEY, "warzone_exit")?.as_str())
    );
    Ok(())
}

#[test]
fn widened_inverse_rule_checks_only_the_candidate_targets_own_declared_variant() -> TestResult {
    let forward_with_revert = [(FORWARD, ACTION_A, 1_000)];

    // The candidate target declares itself a variant of the forward source: accepted.
    let widened = with_revert(plain_content(None, Some(SEALED))?, &forward_with_revert)?;
    assert!(bind(&widened, PLACEMENT_A, &[FORWARD, VARIANT_INVERSE]).is_ok());

    // The reversed direction (the forward source names the candidate target): rejected.
    let reversed = with_revert(
        plain_content(Some(SEALED_VARIANT), None)?,
        &forward_with_revert,
    )?;
    assert!(rejects(
        bind(&reversed, PLACEMENT_A, &[FORWARD, VARIANT_INVERSE]),
        NO_INVERSE
    ));

    // A widened candidate next to the plain inverse is ambiguous: uniqueness still holds.
    assert!(rejects(
        bind(
            &widened,
            PLACEMENT_A,
            &[FORWARD, PLAIN_INVERSE, VARIANT_INVERSE]
        ),
        "revert_after_ms transition has an ambiguous bound inverse at this placement"
    ));
    Ok(())
}

#[test]
fn a_timed_forward_and_its_inverse_must_share_one_event_owner() -> TestResult {
    // D91 at `bind`: the timed forward and its validated inverse are event-owned by one owner.
    let timed = with_revert(plain_content(None, None)?, &[(FORWARD, ACTION_A, 1_000)])?;
    let runtime = bind(&timed, PLACEMENT_A, &[FORWARD, PLAIN_INVERSE])?;
    let owner = TransitionEventOwner::new(EVENT_OWNER)?;
    assert_eq!(
        runtime.transition_origin(&TransitionKey::new(FORWARD)?),
        TransitionOrigin::Event(owner.clone())
    );
    assert_eq!(
        runtime.transition_origin(&TransitionKey::new(PLAIN_INVERSE)?),
        TransitionOrigin::Event(owner)
    );

    // A player-use timed forward is rejected: USE could otherwise commit it unscheduled.
    let mut player_forward = timed.clone();
    player_forward.placements[0]
        .local_object_event_transitions
        .remove(&TransitionKey::new(FORWARD)?);
    assert!(rejects(
        bind(&player_forward, PLACEMENT_A, &[FORWARD, PLAIN_INVERSE]),
        "revert_after_ms transition is not event-owned at this placement"
    ));

    // A player-use inverse, or one owned by another event, is rejected too.
    let mut player_inverse = timed.clone();
    player_inverse.placements[0]
        .local_object_event_transitions
        .remove(&TransitionKey::new(PLAIN_INVERSE)?);
    let mut foreign_inverse = timed.clone();
    foreign_inverse.placements[0]
        .local_object_event_transitions
        .insert(
            TransitionKey::new(PLAIN_INVERSE)?,
            TransitionEventOwner::new("oteryn:encounter/elsewhere")?,
        );
    for content in [&player_inverse, &foreign_inverse] {
        assert!(rejects(
            bind(content, PLACEMENT_A, &[FORWARD, PLAIN_INVERSE]),
            "revert_after_ms inverse is not owned by its forward's event at this placement"
        ));
    }

    // Placement B names no event origin: every edge is player-use.
    let untimed = bind(&timed, PLACEMENT_B, &[FORWARD, PLAIN_INVERSE])?;
    assert_eq!(
        untimed.transition_origin(&TransitionKey::new(FORWARD)?),
        TransitionOrigin::PlayerUse
    );

    // An event origin naming a transition outside the content is refused by the re-link.
    let mut dangling = timed;
    dangling.placements[0]
        .local_object_event_transitions
        .insert(
            TransitionKey::new("oteryn:reference.transition.nowhere")?,
            TransitionEventOwner::new(EVENT_OWNER)?,
        );
    assert!(matches!(
        bind(&dangling, PLACEMENT_A, &[FORWARD, PLAIN_INVERSE]),
        Err(WorldRuntimeError::Content(
            ContentError::MissingReference { .. }
        ))
    ));
    Ok(())
}

#[test]
fn the_event_origin_table_is_part_of_the_binding_identity() -> TestResult {
    // Owner decision D91 (#1187 P2): the same content with a different PLAYER_USE/EVENT
    // split, or another owner, binds under a different generation; an empty table leaves the
    // content generation's bytes unchanged.
    let player_use = with_revert(plain_content(None, None)?, &[])?;
    let base = ReferenceContentGeneration::from_content(&player_use)?;
    let with_owner = |owner: &str| -> TestResult<CanonicalReferencePlayableContent> {
        let mut content = player_use.clone();
        content.placements[0].local_object_event_transitions.insert(
            TransitionKey::new(FORWARD)?,
            TransitionEventOwner::new(owner)?,
        );
        Ok(content)
    };
    let event = with_owner(EVENT_OWNER)?;
    let other_owner = with_owner("oteryn:encounter/elsewhere")?;
    // Placements are outside the Content generation itself, so its bytes and locks do not move.
    assert_eq!(ReferenceContentGeneration::from_content(&event)?, base);

    let keys = [FORWARD, PLAIN_INVERSE];
    let player_use_runtime = bind(&player_use, PLACEMENT_A, &keys)?;
    let mut event_runtime = bind(&event, PLACEMENT_A, &keys)?;
    let other_runtime = bind(&other_owner, PLACEMENT_A, &keys)?;
    assert_eq!(player_use_runtime.content_generation(), &base);
    assert_ne!(event_runtime.content_generation(), &base);
    assert_ne!(
        other_runtime.content_generation(),
        event_runtime.content_generation()
    );
    assert_eq!(
        bind(&event, PLACEMENT_A, &keys)?.content_generation(),
        event_runtime.content_generation()
    );

    // An operation fenced on another origin binding is a BINDING_MISMATCH, not a commit.
    let (_, _, scope) = authority(10)?;
    let generation = ScopeOwnershipGeneration::new(1).map_err(fixture)?;
    let stale = crate::world_runtime::ScopeLocalObjectOperation::new(
        event_runtime.placement_key().clone(),
        event_runtime.incarnation(),
        other_runtime.content_generation().clone(),
        LocalObjectOperation::new(TransitionKey::new(FORWARD)?),
        event_runtime.revision(),
        TransitionEventOwner::new(EVENT_OWNER)?,
    );
    let outcome = event_runtime.apply_scope_operation(
        scope,
        generation,
        &stale,
        &BTreeSet::new(),
        None,
        |_| Ok::<(), std::convert::Infallible>(()),
    )?;
    assert!(matches!(outcome, Ok(outcome) if outcome.disposition() == "BINDING_MISMATCH"));
    assert_eq!(event_runtime.revision(), 0);
    Ok(())
}

#[test]
fn widened_inverse_rule_is_inert_for_plain_content_and_scoped_to_bound_transitions() -> TestResult {
    let plain = with_revert(plain_content(None, None)?, &[(FORWARD, ACTION_A, 1_000)])?;

    // A plain `a -> b` / `b -> a` pair binds exactly as under the equality rule.
    let runtime = bind(&plain, PLACEMENT_A, &[FORWARD, PLAIN_INVERSE])?;
    assert_eq!(runtime.attributes(), None);
    // With no declared variant, a candidate landing elsewhere never qualifies.
    assert!(rejects(
        bind(&plain, PLACEMENT_A, &[FORWARD, VARIANT_INVERSE]),
        NO_INVERSE
    ));
    // An inverse present in Content but not bound at this placement does not count.
    assert!(rejects(bind(&plain, PLACEMENT_A, &[FORWARD]), NO_INVERSE));
    // A revert naming a transition this placement does not bind is rejected.
    assert!(rejects(
        bind(&plain, PLACEMENT_A, &[PLAIN_INVERSE]),
        "revert_after_ms names a transition this placement does not bind"
    ));
    // A placement without §9 tables binds exactly as before.
    let untouched = bind(&plain, PLACEMENT_B, &[FORWARD])?;
    assert_eq!(untouched.attributes(), None);
    Ok(())
}

#[test]
fn attribute_variant_mismatch_and_bad_tables_never_reach_a_bound_runtime() -> TestResult {
    // A collision mismatch injected after linking is caught by bind's re-link.
    let mut mismatched = with_revert(plain_content(None, Some(SEALED))?, &[])?;
    let variant = mismatched
        .definitions
        .iter_mut()
        .find_map(|definition| match &mut definition.kind {
            ReferenceDefinitionKind::LocalObjectStates(states) => Some(states),
            _ => None,
        })
        .and_then(|states| {
            states
                .iter_mut()
                .find(|state| state.key.as_str() == SEALED_VARIANT)
        })
        .ok_or(fixture("variant state"))?;
    variant.collision = LocalObjectCollisionPresence::Absent;
    assert!(matches!(
        bind(&mismatched, PLACEMENT_A, &[FORWARD]),
        Err(WorldRuntimeError::Content(ContentError::InvalidArtifact(
            "reference-playable local object attribute variant must share its base state's collision presence"
        )))
    ));

    // Placement tables are re-validated at bind: an undeclared state, a dangling destination.
    let mut unknown_state = with_revert(plain_content(None, None)?, &[])?;
    unknown_state.placements[0]
        .local_object_state_attributes
        .insert(
            ProductionKey::new("oteryn:reference.state.undeclared")?,
            LocalObjectStateAttributes { destination: None },
        );
    assert!(matches!(
        bind(&unknown_state, PLACEMENT_A, &[FORWARD]),
        Err(WorldRuntimeError::Content(
            ContentError::MissingReference { .. }
        ))
    ));
    let mut dangling = with_revert(plain_content(None, None)?, &[])?;
    dangling.placements[0].local_object_state_attributes.insert(
        ProductionKey::new(OPEN)?,
        LocalObjectStateAttributes {
            destination: Some(PlacementKey::new("oteryn:reference.placement.nowhere")?),
        },
    );
    assert!(matches!(
        bind(&dangling, PLACEMENT_A, &[FORWARD]),
        Err(WorldRuntimeError::Content(
            ContentError::MissingReference { .. }
        ))
    ));

    // A zero revert duration injected after linking is rejected by bind's re-validation.
    let zero = with_revert(plain_content(None, None)?, &[(FORWARD, ACTION_A, 0)])?;
    assert!(matches!(
        bind(&zero, PLACEMENT_A, &[FORWARD, PLAIN_INVERSE]),
        Err(WorldRuntimeError::Content(ContentError::InvalidArtifact(
            "reference-playable placement revert duration must be positive"
        )))
    ));
    Ok(())
}

#[test]
fn uncovered_map_item_shapes_are_rejected_with_named_errors() -> TestResult {
    let action = || DUKE_ACTION.to_owned();
    let lower =
        |json: &str| -> TestResult<Result<LoweredEncounterMapItems, EncounterMapItemError>> {
            Ok(lower_map_item_transforms(
                json,
                &duke_source()?,
                &duke_anchors()?,
            ))
        };

    let death_position = edit_first(|object| {
        object.remove("anchor");
        object.insert("at".to_owned(), Value::from("death_position"));
    })?;
    assert_eq!(
        lower(&death_position)?,
        Err(EncounterMapItemError::DeathPositionNotAdmitted { action: action() })
    );

    // §10.4: the duke action as a `create` still carries `revert_destination`.
    let create = edit_first(|object| {
        object.insert("operation".to_owned(), Value::from("create"));
        object.remove("into");
    })?;
    assert_eq!(
        lower(&create)?,
        Err(EncounterMapItemError::CreateRevertDestinationNotAdmitted { action: action() })
    );

    let interaction = edit_first(|object| {
        object.insert(
            "interaction".to_owned(),
            Value::from("canary:interaction/4951"),
        );
    })?;
    assert_eq!(
        lower(&interaction)?,
        Err(EncounterMapItemError::InteractionNotAdmitted { action: action() })
    );

    let other = edit_first(|object| {
        object.insert("charges".to_owned(), Value::from(3));
    })?;
    assert_eq!(
        lower(&other)?,
        Err(EncounterMapItemError::UnsupportedField {
            action: action(),
            field: "charges".to_owned(),
        })
    );

    let remove = edit_first(|object| {
        object.insert("operation".to_owned(), Value::from("remove"));
    })?;
    assert_eq!(
        lower(&remove)?,
        Err(EncounterMapItemError::OperationNotAdmitted {
            action: action(),
            operation: "remove".to_owned(),
        })
    );

    let no_revert = edit_first(|object| {
        object.remove("revert_after_ms");
    })?;
    assert_eq!(
        lower(&no_revert)?,
        Err(EncounterMapItemError::RevertDestinationWithoutRevert { action: action() })
    );

    let no_destination = edit_first(|object| {
        object.remove("destination");
    })?;
    assert_eq!(
        lower(&no_destination)?,
        Err(EncounterMapItemError::RevertDestinationWithoutDestination { action: action() })
    );
    Ok(())
}

fn set_family(
    content: &mut CanonicalReferencePlayableContent,
    transition: &str,
    family: &str,
) -> TestResult {
    let binding = content
        .transitions
        .iter_mut()
        .find(|binding| binding.key.as_str() == transition)
        .ok_or(fixture("transition to re-family"))?;
    binding.normalized_intent_family = ProductionKey::new(family)?;
    Ok(())
}

#[test]
fn timed_binding_requires_the_paired_intent_family() -> TestResult {
    let timed = [(FORWARD, ACTION_A, 1_000)];

    // An inverse matching by states but carrying the wrong family is rejected by name.
    let mut mismatched = with_revert(plain_content(None, None)?, &timed)?;
    set_family(
        &mut mismatched,
        PLAIN_INVERSE,
        LOCAL_OBJECT_CREATE_INTENT_FAMILY,
    )?;
    assert!(rejects(
        bind(&mismatched, PLACEMENT_A, &[FORWARD, PLAIN_INVERSE]),
        "revert_after_ms inverse does not carry the paired intent family"
    ));

    // A timed forward without a recognized family is rejected; untimed it binds as before.
    let mut unfamilied = with_revert(plain_content(None, None)?, &timed)?;
    set_family(
        &mut unfamilied,
        FORWARD,
        "oteryn:reference.intent.world-object-foreign",
    )?;
    assert!(rejects(
        bind(&unfamilied, PLACEMENT_A, &[FORWARD, PLAIN_INVERSE]),
        "revert_after_ms transition carries no recognized intent family"
    ));
    assert!(bind(&unfamilied, PLACEMENT_B, &[FORWARD, PLAIN_INVERSE]).is_ok());

    // CREATE pairs with REMOVE, not with itself.
    let mut create_remove = with_revert(plain_content(None, None)?, &timed)?;
    set_family(
        &mut create_remove,
        FORWARD,
        LOCAL_OBJECT_CREATE_INTENT_FAMILY,
    )?;
    set_family(
        &mut create_remove,
        PLAIN_INVERSE,
        LOCAL_OBJECT_REMOVE_INTENT_FAMILY,
    )?;
    assert!(bind(&create_remove, PLACEMENT_A, &[FORWARD, PLAIN_INVERSE]).is_ok());
    set_family(
        &mut create_remove,
        PLAIN_INVERSE,
        LOCAL_OBJECT_CREATE_INTENT_FAMILY,
    )?;
    assert!(rejects(
        bind(&create_remove, PLACEMENT_A, &[FORWARD, PLAIN_INVERSE]),
        "revert_after_ms inverse does not carry the paired intent family"
    ));
    Ok(())
}

#[test]
fn lowering_requires_a_transform_family_forward() -> TestResult {
    let mut source = duke_source()?;
    let forward = source
        .transitions
        .first_mut()
        .ok_or(fixture("duke forward"))?;
    forward.normalized_intent_family = ProductionKey::new(LOCAL_OBJECT_OPEN_INTENT_FAMILY)?;
    assert_eq!(
        lower_map_item_transforms(DUKE, &source, &duke_anchors()?),
        Err(EncounterMapItemError::ForwardNotTransformFamily {
            action: DUKE_ACTION.to_owned(),
        })
    );
    Ok(())
}

#[test]
fn two_actions_entering_one_state_with_different_attributes_are_rejected() -> TestResult {
    let with_second = |destination: Option<&str>| {
        duke_with(|actions| {
            let mut copy = actions.first()?.clone();
            let object = copy.as_object_mut()?;
            object.remove("revert_destination");
            object.remove("revert_after_ms");
            match destination {
                Some(anchor) => object.insert("destination".to_owned(), Value::from(anchor)),
                None => object.remove("destination"),
            };
            actions.push(copy);
            Some(())
        })
    };
    let expected = Err(EncounterMapItemError::ConflictingTargetAttributes {
        placement: duke_anchor()?.as_str().to_owned(),
        state: OPEN_ITEM.to_owned(),
    });
    for destination in [Some("warzone_exit"), None] {
        assert_eq!(
            lower_map_item_transforms(
                &with_second(destination)?,
                &duke_source()?,
                &duke_anchors()?
            ),
            expected
        );
    }
    // The same attributes entering the same state are not a conflict.
    assert!(
        lower_map_item_transforms(
            &with_second(Some("reward_destination"))?,
            &duke_source()?,
            &duke_anchors()?
        )
        .is_ok()
    );
    Ok(())
}

#[test]
fn od8_samples_stay_rejected_and_od9_and_covered_samples_are_admitted() -> TestResult {
    for (name, json) in OPEN_DECISION_SAMPLES {
        let result = admitted_map_item_transforms(json);
        let open_decision_9 = matches!(name, "death_priest_shargon" | "the_ravager");
        let expected = match &result {
            // §10.4: OD9 is admitted as one `create` and no transform.
            Ok(admitted) => {
                open_decision_9 && admitted.transforms.is_empty() && admitted.creates.len() == 1
            }
            // §10.3: OD8 stays rejected fail-closed.
            Err(EncounterMapItemError::DeathPositionNotAdmitted { .. }) => !open_decision_9,
            _ => false,
        };
        assert!(expected, "{name}: {result:?}");
    }

    // `the_lord_of_the_lice` also carries a presentational `effect`, which is ignored.
    let lice = admitted_map_item_transforms(LORD_OF_THE_LICE)?;
    assert_eq!(lice.transforms.len(), 1);
    assert_eq!(
        lice.transforms[0].destination.as_deref(),
        Some("godbreaker")
    );
    assert_eq!(
        lice.transforms[0].revert_destination.as_deref(),
        Some("ascendant_exit")
    );
    assert_eq!(lice.transforms[0].revert_after_ms, Some(60_000));
    Ok(())
}

#[test]
fn revert_durations_are_keyed_per_action_and_per_placement() -> TestResult {
    // Two authored actions at one placement invoke the same transition with different
    // durations; a third invokes it with none.
    let json = duke_with(|actions| {
        let template = actions.first()?.clone();
        actions.clear();
        for duration in [Some(1_000_u64), Some(2_000), None] {
            let mut copy = template.clone();
            let object = copy.as_object_mut()?;
            object.remove("destination");
            object.remove("revert_destination");
            match duration {
                Some(ms) => object.insert("revert_after_ms".to_owned(), Value::from(ms)),
                None => object.remove("revert_after_ms"),
            };
            actions.push(copy);
        }
        Some(())
    })?;
    let lowered = lower_map_item_transforms(&json, &duke_source()?, &duke_anchors()?)?;
    let tables = lowered
        .placements
        .get(&duke_anchor()?)
        .ok_or(fixture("anchor tables"))?;
    let forward = TransitionKey::new(FORWARD)?;
    let action = |index: usize| {
        LoweredActionId::new(&format!("{DUKE_KEY}/the_duke_of_the_depths_death/{index}"))
    };
    assert_eq!(
        tables.revert_after_ms,
        BTreeMap::from([
            ((forward.clone(), action(0)?), 1_000),
            ((forward, action(1)?), 2_000),
        ])
    );
    assert!(lowered.post_revert_states.is_empty());
    assert!(lowered.rearm_transitions.is_empty());
    assert!(tables.state_attributes.is_empty());

    // Two placements bind the same shared transition; only the authored one carries it.
    let content = with_revert(plain_content(None, None)?, &[(FORWARD, ACTION_A, 1_000)])?;
    let a = bind(&content, PLACEMENT_A, &[FORWARD, PLAIN_INVERSE])?;
    let b = bind(&content, PLACEMENT_B, &[FORWARD, PLAIN_INVERSE])?;
    let key = TransitionKey::new(FORWARD)?;
    let id = LoweredActionId::new(ACTION_A)?;
    assert_eq!(a.revert_after_ms(&key, &id), Some(1_000));
    assert_eq!(b.revert_after_ms(&key, &id), None);
    Ok(())
}

// §10.4 (OD9): the pre-authored `CREATE` teleporters, `(encounter key, exit anchor, JSON)`.
const OD9_SAMPLES: [(&str, &str, &str); 2] = [
    (
        "canary:encounter/death_priest_shargon",
        "shargon_exit",
        OPEN_DECISION_SAMPLES[0].1,
    ),
    (
        "canary:encounter/the_ravager",
        "ravager_exit",
        OPEN_DECISION_SAMPLES[1].1,
    ),
];

/// The OD9 action id: `<encounter key>/<rule key>/0`, the rule being `<boss>_death`.
fn od9_action(key: &str) -> TestResult<String> {
    let boss = key.rsplit('/').next().ok_or(fixture("encounter key"))?;
    Ok(format!("{key}/{boss}_death/0"))
}

/// An OD9 encounter lowered against a teleporter definition that declares only the created
/// item, linked, with its anchor (at the lowered initial state) and markers injected.
fn od9_content(
    encounter: &str,
    key: &str,
) -> TestResult<(CanonicalReferencePlayableContent, LoweredEncounterMapItems)> {
    let mut source = source_with(
        vec![state(
            SEALED_ITEM,
            LocalObjectCollisionPresence::Absent,
            None,
        )?],
        vec![],
    )?;
    let lowered = lower_map_item_transforms(encounter, &source, &duke_anchors()?)?;
    lowered.apply_to_source(&mut source)?;
    let mut content = link_reference_playable(source)?;
    let anchor = anchor_placement_key(key, "exit_teleporter")?;
    let tables = lowered
        .placements
        .get(&anchor)
        .ok_or(fixture("lowered anchor tables"))?;
    let mut placement = placement_at(&content, anchor, teleporter()?, 100)?;
    placement.local_object_initial_state = tables.initial_state.clone();
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
    Ok((content, lowered))
}

#[test]
fn od9_samples_lower_to_an_absent_start_and_bind_the_create_remove_pair() -> TestResult {
    for (key, exit, encounter) in OD9_SAMPLES {
        let action = od9_action(key)?;
        let absent = ProductionKey::new(&format!("{action}/absent"))?;
        let create = TransitionKey::new(&format!("{action}/create"))?;
        let remove = TransitionKey::new(&format!("{action}/remove"))?;
        let owner = TransitionEventOwner::new(key)?;
        let anchor = anchor_placement_key(key, "exit_teleporter")?;
        let exit = anchor_placement_key(key, exit)?;
        let (content, lowered) = od9_content(encounter, key)?;

        // The absent state: `absent: true`, collision Absent, no variant, no attributes.
        assert_eq!(
            lowered.absent_states,
            vec![(
                teleporter()?,
                LocalObjectStateDefinition {
                    key: absent.clone(),
                    collision: LocalObjectCollisionPresence::Absent,
                    attribute_variant_of: None,
                    absent: true,
                },
            )]
        );
        // The create/remove pair with the CREATE/REMOVE families, and no §9 edges.
        let pair = lowered
            .create_transitions
            .iter()
            .map(|edge| {
                (
                    edge.key.clone(),
                    edge.source_state.as_str().to_owned(),
                    edge.normalized_intent_family.as_str().to_owned(),
                    edge.target_state.as_str().to_owned(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            pair,
            vec![
                (
                    create.clone(),
                    absent.as_str().to_owned(),
                    LOCAL_OBJECT_CREATE_INTENT_FAMILY.to_owned(),
                    SEALED_ITEM.to_owned(),
                ),
                (
                    remove.clone(),
                    SEALED_ITEM.to_owned(),
                    LOCAL_OBJECT_REMOVE_INTENT_FAMILY.to_owned(),
                    absent.as_str().to_owned(),
                ),
            ]
        );
        assert!(lowered.post_revert_states.is_empty());
        assert!(lowered.inverse_transitions.is_empty());
        assert!(lowered.rearm_transitions.is_empty());
        assert_eq!(lowered.destination_markers, BTreeSet::from([exit.clone()]));
        let tables = lowered
            .placements
            .get(&anchor)
            .ok_or(fixture("anchor tables"))?;
        assert_eq!(tables.initial_state, Some(absent.clone()));
        assert_eq!(
            tables.state_attributes,
            BTreeMap::from([(
                ProductionKey::new(SEALED_ITEM)?,
                LocalObjectStateAttributes {
                    destination: Some(exit.clone()),
                },
            )])
        );
        assert_eq!(
            tables.revert_after_ms,
            BTreeMap::from([((create.clone(), LoweredActionId::new(&action)?), 300_000)])
        );
        assert_eq!(
            tables.event_transitions,
            BTreeMap::from([
                (create.clone(), owner.clone()),
                (remove.clone(), owner.clone())
            ])
        );

        // `bind` accepts remove as the unique inverse of create under the unwidened rule.
        // Before any kill the placement is absent, blocks nothing and exposes no destination.
        let mut runtime = bind(
            &content,
            anchor.as_str(),
            &[create.as_str(), remove.as_str()],
        )?;
        assert_eq!(runtime.revert_inverse(&create), Some(&remove));
        assert_eq!(runtime.state_key(), &absent);
        assert!(runtime.is_absent());
        assert_eq!(runtime.attributes(), None);
        assert!(runtime.blocking_cells().is_empty());
        for edge in [&create, &remove] {
            assert_eq!(
                runtime.transition_origin(edge),
                TransitionOrigin::Event(owner.clone())
            );
        }
        // Stepping on or using the anchor does nothing, and no session command commits.
        assert_eq!(
            runtime.attempt_use(0, &BTreeSet::new())?,
            crate::world_runtime::LocalObjectUseOutcome::NothingToUse
        );
        let mut ingress = CommandIngress::new();
        assert!(invoke(&mut runtime, &mut ingress, 1, create.as_str()).is_err());
        assert_eq!(ingress.outstanding(), 0);
        assert_eq!((runtime.state_key(), runtime.revision()), (&absent, 0));
    }
    Ok(())
}

#[test]
fn od9_create_rejections_and_od8_in_a_delayed_rule_stay_named() -> TestResult {
    let (key, _, encounter) = OD9_SAMPLES[0];
    let action = || od9_action(key);
    let edit = |edit: &dyn Fn(&mut Value) -> Option<()>| -> TestResult<String> {
        let mut json: Value = serde_json::from_str(encounter)?;
        edit(&mut json).ok_or(fixture("OD9 sample edit"))?;
        Ok(serde_json::to_string(&json)?)
    };
    let edit_create = |change: &dyn Fn(&mut Map<String, Value>)| {
        edit(&|json| {
            change(json.pointer_mut("/rules/0/actions/0")?.as_object_mut()?);
            Some(())
        })
    };
    let lower = |json: &str| {
        let source = source_with(
            vec![state(
                SEALED_ITEM,
                LocalObjectCollisionPresence::Absent,
                None,
            )?],
            vec![],
        )?;
        Ok::<_, Box<dyn Error>>(lower_map_item_transforms(json, &source, &duke_anchors()?))
    };

    let revert_destination = edit_create(&|object| {
        object.insert("revert_destination".to_owned(), Value::from("shargon_exit"));
    })?;
    assert_eq!(
        lower(&revert_destination)?,
        Err(EncounterMapItemError::CreateRevertDestinationNotAdmitted { action: action()? })
    );
    let interaction = edit_create(&|object| {
        object.insert(
            "interaction".to_owned(),
            Value::from("canary:interaction/4951"),
        );
    })?;
    assert_eq!(
        lower(&interaction)?,
        Err(EncounterMapItemError::InteractionNotAdmitted { action: action()? })
    );
    let into = edit_create(&|object| {
        object.insert(
            "into".to_owned(),
            serde_json::json!({"family": "Item", "key": OPEN_ITEM}),
        );
    })?;
    assert_eq!(
        lower(&into)?,
        Err(EncounterMapItemError::UnsupportedField {
            action: action()?,
            field: "into".to_owned(),
        })
    );
    let no_destination = edit_create(&|object| {
        object.remove("destination");
    })?;
    assert_eq!(
        lower(&no_destination)?,
        Err(EncounterMapItemError::CreateWithoutDestination { action: action()? })
    );
    let no_revert = edit_create(&|object| {
        object.remove("revert_after_ms");
    })?;
    assert_eq!(
        lower(&no_revert)?,
        Err(EncounterMapItemError::CreateWithoutRevert { action: action()? })
    );
    let undeclared = edit_create(&|object| {
        object.insert(
            "item".to_owned(),
            serde_json::json!({"family": "Item", "key": OPEN_ITEM}),
        );
    })?;
    assert_eq!(
        lower(&undeclared)?,
        Err(EncounterMapItemError::CreatedItemNotDeclared { action: action()? })
    );
    let shared = edit(&|json| {
        let actions = json.pointer_mut("/rules/0/actions")?.as_array_mut()?;
        let copy = actions.first()?.clone();
        actions.push(copy);
        Some(())
    })?;
    assert_eq!(
        lower(&shared)?,
        Err(EncounterMapItemError::CreateAnchorShared {
            anchor: "exit_teleporter".to_owned(),
        })
    );

    // §10.3 (OD8): `at: death_position` stays rejected, also in a delayed rule.
    let delayed_death_position = edit(&|json| {
        json.pointer_mut("/rules/0")?
            .as_object_mut()?
            .insert("delay_ms".to_owned(), Value::from(1_000));
        let object = json.pointer_mut("/rules/0/actions/0")?.as_object_mut()?;
        object.remove("anchor");
        object.insert("at".to_owned(), Value::from("death_position"));
        Some(())
    })?;
    assert_eq!(
        lower(&delayed_death_position)?,
        Err(EncounterMapItemError::DeathPositionNotAdmitted { action: action()? })
    );
    Ok(())
}

#[test]
fn the_absent_marker_is_validated_fail_closed_at_link_and_bind() -> TestResult {
    let absent = |collision, variant_of: Option<&str>| -> TestResult<_> {
        let mut state = state("oteryn:reference.state.absent", collision, variant_of)?;
        state.absent = true;
        Ok(state)
    };
    let link = |states: Vec<LocalObjectStateDefinition>| -> TestResult<_> {
        Ok(link_reference_playable(source_with(states, vec![])?))
    };
    let refused = |result: Result<CanonicalReferencePlayableContent, ContentError>,
                   reason: &str| {
        matches!(result, Err(ContentError::InvalidArtifact(actual)) if actual == reason)
    };

    // An absent state is collision Absent and never an attribute variant.
    assert!(refused(
        link(vec![absent(LocalObjectCollisionPresence::Present, None)?])?,
        "reference-playable local object absent state must declare collision Absent"
    ));
    assert!(refused(
        link(vec![
            state(SEALED_ITEM, LocalObjectCollisionPresence::Absent, None)?,
            absent(LocalObjectCollisionPresence::Absent, Some(SEALED_ITEM))?,
        ])?,
        "reference-playable local object absent state must not be an attribute variant"
    ));
    // No state is an attribute variant of an absent state (nothing is rendered there).
    assert!(refused(
        link(vec![
            absent(LocalObjectCollisionPresence::Absent, None)?,
            state(
                SEALED_ITEM,
                LocalObjectCollisionPresence::Absent,
                Some("oteryn:reference.state.absent"),
            )?,
        ])?,
        "reference-playable local object attribute variant must not name an absent base state"
    ));

    // A placement carrying attributes for an absent state never reaches a bound runtime.
    let (key, exit, encounter) = OD9_SAMPLES[0];
    let action = od9_action(key)?;
    let (mut content, _) = od9_content(encounter, key)?;
    let anchor = anchor_placement_key(key, "exit_teleporter")?;
    let placement = content
        .placements
        .iter_mut()
        .find(|placement| placement.key == anchor)
        .ok_or(fixture("OD9 anchor placement"))?;
    placement.local_object_state_attributes.insert(
        ProductionKey::new(&format!("{action}/absent"))?,
        LocalObjectStateAttributes {
            destination: Some(anchor_placement_key(key, exit)?),
        },
    );
    let bound = bind(
        &content,
        anchor.as_str(),
        &[&format!("{action}/create"), &format!("{action}/remove")],
    );
    assert!(matches!(
        bound,
        Err(WorldRuntimeError::Content(ContentError::InvalidArtifact(reason)))
            if reason == "reference-playable placement declares attributes for an absent local object state"
    ));
    Ok(())
}
