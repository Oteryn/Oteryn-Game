//! #162 §9: pure lowering of authored encounter `map_item transform` actions into
//! attribute-bearing `LocalObject` content (docs/architecture/
//! OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md §9, owner-accepted
//! 2026-09-28).
//!
//! Admitted shapes:
//! - `map_item transform` at a pre-authored `anchor`, optionally carrying `destination`,
//!   `revert_after_ms` and `revert_destination` (which needs both `revert_after_ms` and
//!   `destination`). The forward transition must carry the TRANSFORM intent family, and a
//!   synthesized inverse carries it too (§7 pairing).
//! - §10.4 (OD9): `map_item create` at a pre-authored `anchor`, with `destination` and
//!   `revert_after_ms` both required.
//!
//! `effect` is presentational and ignored. Everything else stays rejected fail-closed with a
//! named error: `at: death_position` (§10.3, OD8, in any rule, delayed or not), `revert_destination`
//! or `interaction` on a `create`, a `create` without `destination` or `revert_after_ms`, any other
//! operation, `interaction`, and any other field.
//!
//! Representation defaults (§9 leaves them to the owning lane):
//! - an encounter anchor lowers to `PlacementKey` `<encounter key>/anchor/<anchor key>`; a
//!   destination anchor lowers to one synthesized Generic marker placement under that key;
//! - a `LocalObject` state is keyed by the `ItemRef` key it renders as;
//! - `LoweredActionId` is `<encounter key>/<rule key>/<action index>`, with `/<branch>/<index>`
//!   appended per enclosing `one_of` branch;
//! - a `revert_destination` lowers to post-revert state `<action id>/post-revert` (declared
//!   `attribute_variant_of` the natural source state) and its dedicated inverse transition
//!   `<action id>/revert`;
//! - owner decision D90 (re-arm): the same action also lowers to a re-arm forward
//!   `<action id>/rearm` from the post-revert state back to the forward's target, bound like the
//!   authored forward (same family, capability, guards, attributes and `revert_after_ms`), whose
//!   unique inverse is the dedicated `<action id>/revert`. A covered teleporter therefore opens on
//!   every owning event, not once per scope generation.
//! - owner decision D91 (event-owned transitions): every edge this lowering binds at a transform
//!   anchor (the authored forward, the dedicated `/revert` inverse and the `/rearm` forward) is
//!   `TransitionOrigin::Event` at that anchor's placement, owned by the encounter key. None of
//!   them is USE-selectable or session-invocable there.
//! - §10.2/§10.4 (OD9), a pre-authored `create`: its anchor object gains the synthesized absent
//!   state `<action id>/absent` (`absent: true`, `collision: Absent`, no attributes), which is the
//!   placement's initial state; the forward `<action id>/create` (absent -> the created item's
//!   state, CREATE family) and its inverse `<action id>/remove` (back to absent, REMOVE family).
//!   The present state exposes `destination`, and the create carries the authored
//!   `revert_after_ms`. Both edges are `Event(<encounter key>)`. The revert lands on the absent
//!   initial state, where the create matches again, so no re-arm edge is lowered (D90). A kill
//!   while it is open re-arms the pending revert to the full duration and lowers nothing extra
//!   (owner decision Q2=b, `world_object_revert::ScopeRevertDriver::rearm_open_create`).

use super::{
    ContentError, FootprintCell, FootprintRelation, LOCAL_OBJECT_CREATE_INTENT_FAMILY,
    LOCAL_OBJECT_REMOVE_INTENT_FAMILY, LOCAL_OBJECT_TRANSFORM_INTENT_FAMILY,
    LocalObjectCollisionPresence, LocalObjectIntentFamily, LocalObjectStateAttributes,
    LocalObjectStateDefinition, LoweredActionId, MapRevisionRef, OwnerCapabilityRequirement,
    PlacementKey, PlacementRef, ProductionKey, ReferenceDefinitionKind,
    ReferencePlayableContentSource, SpatialAddress, TransitionBinding, TransitionEventOwner,
    TransitionKey, TypedDefinitionRef,
};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

const ADMITTED_FIELDS: [&str; 9] = [
    "kind",
    "operation",
    "item",
    "into",
    "anchor",
    "destination",
    "revert_destination",
    "revert_after_ms",
    "effect",
];

/// §10.4: the fields a pre-authored `create` may carry.
const ADMITTED_CREATE_FIELDS: [&str; 7] = [
    "kind",
    "operation",
    "item",
    "anchor",
    "destination",
    "revert_after_ms",
    "effect",
];

/// §10.2: the capability of the synthesized create/remove pair, the one
/// `LocalObjectRuntime::bind` supports (as `native_entry::accepted::DOOR_OWNER_CAPABILITY`).
const CREATED_OBJECT_CAPABILITY: &str = "oteryn:runtime.capability.local-object-transition";

/// A named, fail-closed lowering rejection. `action` is the would-be `LoweredActionId`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncounterMapItemError {
    InvalidEncounter(&'static str),
    /// §10.3 (OD8): a runtime-resolved `at: death_position` stays rejected until its
    /// prerequisites land, in every rule, delayed or not.
    DeathPositionNotAdmitted {
        action: String,
    },
    /// §10.4: a `create` must carry `destination` and `revert_after_ms`.
    CreateWithoutDestination {
        action: String,
    },
    CreateWithoutRevert {
        action: String,
    },
    /// §10.4: reverting a `create` lands on absent, which carries no destination.
    CreateRevertDestinationNotAdmitted {
        action: String,
    },
    /// §10.4: the created item is not a state of the anchor object's vocabulary.
    CreatedItemNotDeclared {
        action: String,
    },
    /// §10.4: a `create` anchor starts absent, so no other `map_item` action may share it.
    CreateAnchorShared {
        anchor: String,
    },
    OperationNotAdmitted {
        action: String,
        operation: String,
    },
    /// §9 design point 4: `interaction` is not a covered attribute.
    InteractionNotAdmitted {
        action: String,
    },
    UnsupportedField {
        action: String,
        field: String,
    },
    RevertDestinationWithoutRevert {
        action: String,
    },
    /// A `revert_destination` without the `destination` it reverts from.
    RevertDestinationWithoutDestination {
        action: String,
    },
    UnknownAnchor {
        action: String,
        anchor: String,
    },
    /// The transform anchor has no `LocalObject` definition in the supplied content.
    UnboundAnchor {
        action: String,
        anchor: String,
    },
    /// Zero or several content transitions match the authored `item` -> `into` edge.
    UnresolvedTransition {
        action: String,
    },
    /// The resolved forward transition does not carry §7's TRANSFORM intent family.
    ForwardNotTransformFamily {
        action: String,
    },
    /// Round 5 P2: two actions at one placement enter one state with different attributes.
    ConflictingTargetAttributes {
        placement: String,
        state: String,
    },
    Content(ContentError),
}

impl From<ContentError> for EncounterMapItemError {
    fn from(error: ContentError) -> Self {
        Self::Content(error)
    }
}

impl std::fmt::Display for EncounterMapItemError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "encounter map_item lowering rejected: {self:?}")
    }
}

impl std::error::Error for EncounterMapItemError {}

/// One admitted `map_item transform`, validated against the encounter alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedMapItemTransform {
    pub action: LoweredActionId,
    pub anchor: String,
    pub item: ProductionKey,
    pub into: ProductionKey,
    pub destination: Option<String>,
    pub revert_destination: Option<String>,
    pub revert_after_ms: Option<u64>,
}

/// §10.4: one admitted pre-authored `map_item create`, validated against the encounter alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedMapItemCreate {
    pub action: LoweredActionId,
    pub anchor: String,
    pub item: ProductionKey,
    pub destination: String,
    pub revert_after_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedEncounterMapItems {
    pub encounter: String,
    pub transforms: Vec<AdmittedMapItemTransform>,
    pub creates: Vec<AdmittedMapItemCreate>,
}

enum AdmittedMapItem {
    Transform(AdmittedMapItemTransform),
    Create(AdmittedMapItemCreate),
}

/// The per-placement §9 tables lowered for one transform anchor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoweredPlacementTables {
    pub definition: TypedDefinitionRef,
    pub state_attributes: BTreeMap<ProductionKey, LocalObjectStateAttributes>,
    pub revert_after_ms: BTreeMap<(TransitionKey, LoweredActionId), u64>,
    /// D91: every transition lowered at this anchor, as `TransitionOrigin::Event` of its owner.
    pub event_transitions: BTreeMap<TransitionKey, TransitionEventOwner>,
    /// §10.4: the synthesized absent state for a `create` anchor. `None` for a transform anchor,
    /// whose initial state is the caller's authored natural state.
    pub initial_state: Option<ProductionKey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoweredEncounterMapItems {
    /// Post-revert states to add to their `LocalObject` definition's vocabulary.
    pub post_revert_states: Vec<(TypedDefinitionRef, LocalObjectStateDefinition)>,
    /// Dedicated inverse transitions, one per `revert_destination`-bearing action.
    pub inverse_transitions: Vec<TransitionBinding>,
    /// D90 re-arm forwards (post-revert state -> forward target), one per
    /// `revert_destination`-bearing action.
    pub rearm_transitions: Vec<TransitionBinding>,
    /// §10.2 absent states to add to their `LocalObject` definition's vocabulary, one per
    /// `create`.
    pub absent_states: Vec<(TypedDefinitionRef, LocalObjectStateDefinition)>,
    /// §10.2 create forwards and their remove inverses, two per `create`.
    pub create_transitions: Vec<TransitionBinding>,
    /// Keyed by the transform anchor's placement.
    pub placements: BTreeMap<PlacementKey, LoweredPlacementTables>,
    /// Destination anchors; each is one Generic marker placement (`marker_placement`).
    pub destination_markers: BTreeSet<PlacementKey>,
}

impl LoweredEncounterMapItems {
    /// Adds the synthesized post-revert and absent states, inverse transitions, re-arm forwards
    /// and create/remove pairs to `source` before it is linked; the linker then validates every
    /// declared `attribute_variant_of` and `absent` marker.
    pub fn apply_to_source(
        &self,
        source: &mut ReferencePlayableContentSource,
    ) -> Result<(), EncounterMapItemError> {
        for (definition, state) in self.post_revert_states.iter().chain(&self.absent_states) {
            let states = source
                .definitions
                .iter_mut()
                .find(|candidate| &candidate.definition == definition)
                .and_then(|candidate| match &mut candidate.kind {
                    ReferenceDefinitionKind::LocalObjectStates(states) => Some(states),
                    _ => None,
                })
                .ok_or(EncounterMapItemError::InvalidEncounter(
                    "synthesized state targets a definition absent from the content",
                ))?;
            states.push(state.clone());
        }
        source
            .transitions
            .extend(self.inverse_transitions.iter().cloned());
        source
            .transitions
            .extend(self.rearm_transitions.iter().cloned());
        source
            .transitions
            .extend(self.create_transitions.iter().cloned());
        Ok(())
    }
}

/// Anchor `anchor` of encounter `encounter` as a `PlacementKey`.
pub fn anchor_placement_key(encounter: &str, anchor: &str) -> Result<PlacementKey, ContentError> {
    PlacementKey::new(&format!("{encounter}/anchor/{anchor}"))
}

/// One synthesized Generic marker placement for a destination anchor. The anchor's spatial
/// binding (§8 item 2) is the caller's; the marker carries no `LocalObject` state.
pub fn marker_placement(
    key: PlacementKey,
    definition: TypedDefinitionRef,
    map_revision: MapRevisionRef,
    address: SpatialAddress,
) -> PlacementRef {
    let footprint = FootprintRelation::Qualified {
        members: vec![FootprintCell {
            dx: 0,
            dy: 0,
            dz: 0,
        }],
        evidence: address.evidence.clone(),
    };
    PlacementRef {
        key,
        map_revision,
        definition,
        address,
        presentation_footprint: footprint.clone(),
        collision_footprint: footprint,
        local_object_initial_state: None,
        local_object_state_attributes: BTreeMap::new(),
        local_object_revert_after_ms: BTreeMap::new(),
        local_object_event_transitions: BTreeMap::new(),
    }
}

fn string_field<'a>(object: &'a Map<String, Value>, field: &str) -> Option<&'a str> {
    object.get(field).and_then(Value::as_str)
}

fn item_key(
    object: &Map<String, Value>,
    field: &str,
) -> Result<ProductionKey, EncounterMapItemError> {
    let item = object.get(field).and_then(Value::as_object).ok_or(
        EncounterMapItemError::InvalidEncounter(
            "map_item transform requires item and into ItemRefs",
        ),
    )?;
    if string_field(item, "family") != Some("Item") {
        return Err(EncounterMapItemError::InvalidEncounter(
            "map_item ItemRef must name the Item family",
        ));
    }
    let key = string_field(item, "key").ok_or(EncounterMapItemError::InvalidEncounter(
        "map_item ItemRef requires a key",
    ))?;
    Ok(ProductionKey::new(key)?)
}

fn classify(
    action: &str,
    object: &Map<String, Value>,
    anchors: &BTreeSet<String>,
) -> Result<AdmittedMapItem, EncounterMapItemError> {
    let named = || action.to_owned();
    if object.contains_key("at") {
        return Err(EncounterMapItemError::DeathPositionNotAdmitted { action: named() });
    }
    let operation = string_field(object, "operation").unwrap_or_default();
    let create = operation == "create";
    if operation != "transform" && !create {
        return Err(EncounterMapItemError::OperationNotAdmitted {
            action: named(),
            operation: operation.to_owned(),
        });
    }
    if object.contains_key("interaction") {
        return Err(EncounterMapItemError::InteractionNotAdmitted { action: named() });
    }
    if create && object.contains_key("revert_destination") {
        return Err(EncounterMapItemError::CreateRevertDestinationNotAdmitted { action: named() });
    }
    let admitted_fields: &[&str] = if create {
        &ADMITTED_CREATE_FIELDS
    } else {
        &ADMITTED_FIELDS
    };
    if let Some(field) = object
        .keys()
        .find(|field| !admitted_fields.contains(&field.as_str()))
    {
        return Err(EncounterMapItemError::UnsupportedField {
            action: named(),
            field: field.clone(),
        });
    }
    let anchor_of = |field: &str| -> Result<Option<String>, EncounterMapItemError> {
        let Some(value) = object.get(field) else {
            return Ok(None);
        };
        let anchor = value.as_str().unwrap_or_default();
        if !anchors.contains(anchor) {
            return Err(EncounterMapItemError::UnknownAnchor {
                action: named(),
                anchor: anchor.to_owned(),
            });
        }
        Ok(Some(anchor.to_owned()))
    };
    let anchor = anchor_of("anchor")?.ok_or(EncounterMapItemError::InvalidEncounter(
        "map_item transform or create requires a pre-authored anchor",
    ))?;
    let revert_after_ms = match object.get("revert_after_ms") {
        None => None,
        Some(value) => Some(value.as_u64().filter(|ms| *ms > 0).ok_or(
            EncounterMapItemError::InvalidEncounter("revert_after_ms must be a positive integer"),
        )?),
    };
    if create {
        let destination = anchor_of("destination")?
            .ok_or(EncounterMapItemError::CreateWithoutDestination { action: named() })?;
        let revert_after_ms = revert_after_ms
            .ok_or(EncounterMapItemError::CreateWithoutRevert { action: named() })?;
        return Ok(AdmittedMapItem::Create(AdmittedMapItemCreate {
            action: LoweredActionId::new(action)?,
            anchor,
            item: item_key(object, "item")?,
            destination,
            revert_after_ms,
        }));
    }
    let revert_destination = anchor_of("revert_destination")?;
    if revert_destination.is_some() && revert_after_ms.is_none() {
        return Err(EncounterMapItemError::RevertDestinationWithoutRevert { action: named() });
    }
    let destination = anchor_of("destination")?;
    if revert_destination.is_some() && destination.is_none() {
        return Err(EncounterMapItemError::RevertDestinationWithoutDestination { action: named() });
    }
    Ok(AdmittedMapItem::Transform(AdmittedMapItemTransform {
        action: LoweredActionId::new(action)?,
        anchor,
        item: item_key(object, "item")?,
        into: item_key(object, "into")?,
        destination,
        revert_destination,
        revert_after_ms,
    }))
}

fn walk_actions(
    prefix: &str,
    actions: &[Value],
    anchors: &BTreeSet<String>,
    admitted: &mut Vec<AdmittedMapItem>,
) -> Result<(), EncounterMapItemError> {
    for (index, action) in actions.iter().enumerate() {
        let path = format!("{prefix}/{index}");
        let object = action
            .as_object()
            .ok_or(EncounterMapItemError::InvalidEncounter(
                "encounter action must be an object",
            ))?;
        match string_field(object, "kind") {
            Some("map_item") => admitted.push(classify(&path, object, anchors)?),
            Some("one_of") => {
                let branches = object.get("branches").and_then(Value::as_array).ok_or(
                    EncounterMapItemError::InvalidEncounter("one_of requires branches"),
                )?;
                for (branch_index, branch) in branches.iter().enumerate() {
                    let nested = branch.get("actions").and_then(Value::as_array).ok_or(
                        EncounterMapItemError::InvalidEncounter("one_of branch requires actions"),
                    )?;
                    walk_actions(&format!("{path}/{branch_index}"), nested, anchors, admitted)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// Validates every `map_item` action of one encounter against the §9 and §10.4 admitted shapes,
/// using the encounter alone.
pub fn admitted_map_item_transforms(
    encounter_json: &str,
) -> Result<AdmittedEncounterMapItems, EncounterMapItemError> {
    let encounter: Value = serde_json::from_str(encounter_json)
        .map_err(|_error| EncounterMapItemError::InvalidEncounter("encounter is not valid JSON"))?;
    let key = encounter
        .pointer("/identity/key")
        .and_then(Value::as_str)
        .ok_or(EncounterMapItemError::InvalidEncounter(
            "encounter requires an identity key",
        ))?;
    let anchors = encounter
        .get("anchors")
        .and_then(Value::as_array)
        .map(|anchors| {
            anchors
                .iter()
                .filter_map(|anchor| anchor.get("key").and_then(Value::as_str))
                .map(str::to_owned)
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    let rules = encounter.get("rules").and_then(Value::as_array).ok_or(
        EncounterMapItemError::InvalidEncounter("encounter requires rules"),
    )?;
    let mut items = Vec::new();
    for rule in rules {
        let rule_key = rule.get("key").and_then(Value::as_str).ok_or(
            EncounterMapItemError::InvalidEncounter("encounter rule requires a key"),
        )?;
        let actions = rule.get("actions").and_then(Value::as_array).ok_or(
            EncounterMapItemError::InvalidEncounter("encounter rule requires actions"),
        )?;
        walk_actions(&format!("{key}/{rule_key}"), actions, &anchors, &mut items)?;
    }
    let mut transforms = Vec::new();
    let mut creates = Vec::new();
    for item in items {
        match item {
            AdmittedMapItem::Transform(transform) => transforms.push(transform),
            AdmittedMapItem::Create(create) => creates.push(create),
        }
    }
    // §10.4: a create anchor starts absent; any other action at it would need another start.
    for create in &creates {
        let sharing = creates
            .iter()
            .filter(|other| other.anchor == create.anchor)
            .count()
            + transforms
                .iter()
                .filter(|transform| transform.anchor == create.anchor)
                .count();
        if sharing > 1 {
            return Err(EncounterMapItemError::CreateAnchorShared {
                anchor: create.anchor.clone(),
            });
        }
    }
    Ok(AdmittedEncounterMapItems {
        encounter: key.to_owned(),
        transforms,
        creates,
    })
}

/// Lowers every admitted `map_item transform` and pre-authored `create` of one encounter against
/// `source`, whose `LocalObject` definitions (and, for a transform, transitions) the anchors
/// already carry. `anchor_objects` names the `LocalObject` definition authored at each anchor.
pub fn lower_map_item_transforms(
    encounter_json: &str,
    source: &ReferencePlayableContentSource,
    anchor_objects: &BTreeMap<String, TypedDefinitionRef>,
) -> Result<LoweredEncounterMapItems, EncounterMapItemError> {
    let admitted = admitted_map_item_transforms(encounter_json)?;
    let owner = TransitionEventOwner::new(&admitted.encounter)?;
    let mut lowered = LoweredEncounterMapItems {
        post_revert_states: Vec::new(),
        inverse_transitions: Vec::new(),
        rearm_transitions: Vec::new(),
        absent_states: Vec::new(),
        create_transitions: Vec::new(),
        placements: BTreeMap::new(),
        destination_markers: BTreeSet::new(),
    };
    let mut entered: BTreeMap<(PlacementKey, ProductionKey), Option<PlacementKey>> =
        BTreeMap::new();
    for transform in &admitted.transforms {
        let action = transform.action.as_str();
        let unbound = || EncounterMapItemError::UnboundAnchor {
            action: action.to_owned(),
            anchor: transform.anchor.clone(),
        };
        let definition = anchor_objects.get(&transform.anchor).ok_or_else(unbound)?;
        let states = source
            .definitions
            .iter()
            .find(|candidate| &candidate.definition == definition)
            .and_then(|candidate| match &candidate.kind {
                ReferenceDefinitionKind::LocalObjectStates(states) => Some(states),
                _ => None,
            })
            .ok_or_else(unbound)?;
        let mut forwards = source.transitions.iter().filter(|transition| {
            &transition.definition == definition
                && transition.source_state == transform.item
                && transition.target_state == transform.into
        });
        let forward = match (forwards.next(), forwards.next()) {
            (Some(forward), None) => forward,
            _ => {
                return Err(EncounterMapItemError::UnresolvedTransition {
                    action: action.to_owned(),
                });
            }
        };
        // `map_item transform` lowers onto a TRANSFORM-family transition only (§7 pairing).
        if LocalObjectIntentFamily::from_key(&forward.normalized_intent_family)
            != Some(LocalObjectIntentFamily::Transform)
        {
            return Err(EncounterMapItemError::ForwardNotTransformFamily {
                action: action.to_owned(),
            });
        }
        let placement = anchor_placement_key(&admitted.encounter, &transform.anchor)?;
        let destination = transform
            .destination
            .as_deref()
            .map(|anchor| anchor_placement_key(&admitted.encounter, anchor))
            .transpose()?;
        let slot = (placement.clone(), forward.target_state.clone());
        if entered
            .get(&slot)
            .is_some_and(|existing| existing != &destination)
        {
            return Err(EncounterMapItemError::ConflictingTargetAttributes {
                placement: placement.as_str().to_owned(),
                state: forward.target_state.as_str().to_owned(),
            });
        }
        entered.insert(slot, destination.clone());
        let tables =
            lowered
                .placements
                .entry(placement)
                .or_insert_with(|| LoweredPlacementTables {
                    definition: definition.clone(),
                    state_attributes: BTreeMap::new(),
                    revert_after_ms: BTreeMap::new(),
                    event_transitions: BTreeMap::new(),
                    initial_state: None,
                });
        // D91: the owning event commits the forward; players never USE it at this anchor.
        tables
            .event_transitions
            .insert(forward.key.clone(), owner.clone());
        if let Some(destination) = destination {
            lowered.destination_markers.insert(destination.clone());
            tables.state_attributes.insert(
                forward.target_state.clone(),
                LocalObjectStateAttributes {
                    destination: Some(destination),
                },
            );
        }
        if let Some(ms) = transform.revert_after_ms {
            tables
                .revert_after_ms
                .insert((forward.key.clone(), transform.action.clone()), ms);
        }
        let Some(revert_destination) = &transform.revert_destination else {
            continue;
        };
        // Admission already requires `revert_after_ms` beside `revert_destination`.
        let revert_after_ms = transform.revert_after_ms.ok_or(
            EncounterMapItemError::RevertDestinationWithoutRevert {
                action: action.to_owned(),
            },
        )?;
        let revert_destination = anchor_placement_key(&admitted.encounter, revert_destination)?;
        let source_collision = states
            .iter()
            .find(|state| state.key == forward.source_state)
            .map(|state| state.collision)
            .ok_or_else(unbound)?;
        let post_revert = ProductionKey::new(&format!("{action}/post-revert"))?;
        lowered.post_revert_states.push((
            definition.clone(),
            LocalObjectStateDefinition {
                key: post_revert.clone(),
                collision: source_collision,
                attribute_variant_of: Some(forward.source_state.clone()),
                absent: false,
            },
        ));
        let revert = TransitionKey::new(&format!("{action}/revert"))?;
        tables
            .event_transitions
            .insert(revert.clone(), owner.clone());
        lowered.inverse_transitions.push(TransitionBinding {
            key: revert,
            definition: definition.clone(),
            source_state: forward.target_state.clone(),
            normalized_intent_family: ProductionKey::new(LOCAL_OBJECT_TRANSFORM_INTENT_FAMILY)?,
            target_state: post_revert.clone(),
            owner_capability: forward.owner_capability.clone(),
            policy_guard_refs: forward.policy_guard_refs.clone(),
        });
        // D90: the re-arm forward C -> B, bound like A -> B. It enters the same target state, so
        // it exposes the same `destination`, and it carries the same authored duration, keyed by
        // its own `TransitionKey`; its unique inverse is the dedicated revert above.
        let rearm = TransitionKey::new(&format!("{action}/rearm"))?;
        tables
            .event_transitions
            .insert(rearm.clone(), owner.clone());
        lowered.rearm_transitions.push(TransitionBinding {
            key: rearm.clone(),
            definition: definition.clone(),
            source_state: post_revert.clone(),
            normalized_intent_family: forward.normalized_intent_family.clone(),
            target_state: forward.target_state.clone(),
            owner_capability: forward.owner_capability.clone(),
            policy_guard_refs: forward.policy_guard_refs.clone(),
        });
        tables
            .revert_after_ms
            .insert((rearm, transform.action.clone()), revert_after_ms);
        lowered
            .destination_markers
            .insert(revert_destination.clone());
        tables.state_attributes.insert(
            post_revert,
            LocalObjectStateAttributes {
                destination: Some(revert_destination),
            },
        );
    }
    for create in &admitted.creates {
        lower_create(
            create,
            &admitted.encounter,
            &owner,
            source,
            anchor_objects,
            &mut lowered,
        )?;
    }
    Ok(lowered)
}

/// §10.2/§10.4: one pre-authored `create` onto its anchor placement. Admission already made the
/// anchor exclusive to this action.
fn lower_create(
    create: &AdmittedMapItemCreate,
    encounter: &str,
    owner: &TransitionEventOwner,
    source: &ReferencePlayableContentSource,
    anchor_objects: &BTreeMap<String, TypedDefinitionRef>,
    lowered: &mut LoweredEncounterMapItems,
) -> Result<(), EncounterMapItemError> {
    let action = create.action.as_str();
    let unbound = || EncounterMapItemError::UnboundAnchor {
        action: action.to_owned(),
        anchor: create.anchor.clone(),
    };
    let definition = anchor_objects.get(&create.anchor).ok_or_else(unbound)?;
    let states = source
        .definitions
        .iter()
        .find(|candidate| &candidate.definition == definition)
        .and_then(|candidate| match &candidate.kind {
            ReferenceDefinitionKind::LocalObjectStates(states) => Some(states),
            _ => None,
        })
        .ok_or_else(unbound)?;
    // The present state is the created item's own state (§9 convention), never an absent one.
    if !states
        .iter()
        .any(|state| state.key == create.item && !state.absent)
    {
        return Err(EncounterMapItemError::CreatedItemNotDeclared {
            action: action.to_owned(),
        });
    }
    let absent = ProductionKey::new(&format!("{action}/absent"))?;
    lowered.absent_states.push((
        definition.clone(),
        LocalObjectStateDefinition {
            key: absent.clone(),
            collision: LocalObjectCollisionPresence::Absent,
            attribute_variant_of: None,
            absent: true,
        },
    ));
    let capability = OwnerCapabilityRequirement {
        capability_key: ProductionKey::new(CREATED_OBJECT_CAPABILITY)?,
    };
    let forward = TransitionKey::new(&format!("{action}/create"))?;
    let inverse = TransitionKey::new(&format!("{action}/remove"))?;
    for (key, family, from, to) in [
        (
            &forward,
            LOCAL_OBJECT_CREATE_INTENT_FAMILY,
            &absent,
            &create.item,
        ),
        (
            &inverse,
            LOCAL_OBJECT_REMOVE_INTENT_FAMILY,
            &create.item,
            &absent,
        ),
    ] {
        lowered.create_transitions.push(TransitionBinding {
            key: key.clone(),
            definition: definition.clone(),
            source_state: from.clone(),
            normalized_intent_family: ProductionKey::new(family)?,
            target_state: to.clone(),
            owner_capability: capability.clone(),
            policy_guard_refs: Vec::new(),
        });
    }
    let destination = anchor_placement_key(encounter, &create.destination)?;
    lowered.destination_markers.insert(destination.clone());
    let placement = anchor_placement_key(encounter, &create.anchor)?;
    lowered.placements.insert(
        placement,
        LoweredPlacementTables {
            definition: definition.clone(),
            state_attributes: BTreeMap::from([(
                create.item.clone(),
                LocalObjectStateAttributes {
                    destination: Some(destination),
                },
            )]),
            revert_after_ms: BTreeMap::from([(
                (forward.clone(), create.action.clone()),
                create.revert_after_ms,
            )]),
            // D91: the encounter commits the create, and its revert driver the remove.
            event_transitions: BTreeMap::from([(forward, owner.clone()), (inverse, owner.clone())]),
            initial_state: Some(absent),
        },
    );
    Ok(())
}
