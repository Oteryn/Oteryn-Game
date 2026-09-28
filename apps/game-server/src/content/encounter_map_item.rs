//! #162 §9: pure lowering of authored encounter `map_item transform` actions into
//! attribute-bearing `LocalObject` content (docs/architecture/
//! OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md §9, owner-accepted
//! 2026-09-28).
//!
//! Admitted shape: `map_item transform` at a pre-authored `anchor`, optionally carrying
//! `destination`, `revert_after_ms` and `revert_destination` (which needs both `revert_after_ms`
//! and `destination`); `effect` is presentational and ignored. The forward transition must carry
//! the TRANSFORM intent family, and a synthesized inverse carries it too (§7 pairing). Everything else stays rejected fail-closed with a named
//! error: `at: death_position` (§7 open decision 8), `create` carrying `destination` (§7 open
//! decision 9), any other non-`transform` operation, `interaction`, and any other field.
//!
//! Representation defaults (§9 leaves them to the owning lane):
//! - an encounter anchor lowers to `PlacementKey` `<encounter key>/anchor/<anchor key>`; a
//!   destination anchor lowers to one synthesized Generic marker placement under that key;
//! - a `LocalObject` state is keyed by the `ItemRef` key it renders as;
//! - `LoweredActionId` is `<encounter key>/<rule key>/<action index>`, with `/<branch>/<index>`
//!   appended per enclosing `one_of` branch;
//! - a `revert_destination` lowers to post-revert state `<action id>/post-revert` (declared
//!   `attribute_variant_of` the natural source state) and its dedicated inverse transition
//!   `<action id>/revert`.

use super::{
    ContentError, FootprintCell, FootprintRelation, LOCAL_OBJECT_TRANSFORM_INTENT_FAMILY,
    LocalObjectIntentFamily, LocalObjectStateAttributes, LocalObjectStateDefinition,
    LoweredActionId, MapRevisionRef, PlacementKey, PlacementRef, ProductionKey,
    ReferenceDefinitionKind, ReferencePlayableContentSource, SpatialAddress, TransitionBinding,
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

/// A named, fail-closed lowering rejection. `action` is the would-be `LoweredActionId`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncounterMapItemError {
    InvalidEncounter(&'static str),
    /// §7 open decision 8: a runtime-resolved `at: death_position` has no pre-authored placement.
    DeathPositionNotAdmitted {
        action: String,
    },
    /// §7 open decision 9: §9 admits only `transform`, never `create` carrying `destination`.
    CreateWithDestinationNotAdmitted {
        action: String,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedEncounterMapItems {
    pub encounter: String,
    pub transforms: Vec<AdmittedMapItemTransform>,
}

/// The per-placement §9 tables lowered for one transform anchor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoweredPlacementTables {
    pub definition: TypedDefinitionRef,
    pub state_attributes: BTreeMap<ProductionKey, LocalObjectStateAttributes>,
    pub revert_after_ms: BTreeMap<(TransitionKey, LoweredActionId), u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoweredEncounterMapItems {
    /// Post-revert states to add to their `LocalObject` definition's vocabulary.
    pub post_revert_states: Vec<(TypedDefinitionRef, LocalObjectStateDefinition)>,
    /// Dedicated inverse transitions, one per `revert_destination`-bearing action.
    pub inverse_transitions: Vec<TransitionBinding>,
    /// Keyed by the transform anchor's placement.
    pub placements: BTreeMap<PlacementKey, LoweredPlacementTables>,
    /// Destination anchors; each is one Generic marker placement (`marker_placement`).
    pub destination_markers: BTreeSet<PlacementKey>,
}

impl LoweredEncounterMapItems {
    /// Adds the synthesized post-revert states and inverse transitions to `source` before it is
    /// linked; the linker then validates every declared `attribute_variant_of`.
    pub fn apply_to_source(
        &self,
        source: &mut ReferencePlayableContentSource,
    ) -> Result<(), EncounterMapItemError> {
        for (definition, state) in &self.post_revert_states {
            let states = source
                .definitions
                .iter_mut()
                .find(|candidate| &candidate.definition == definition)
                .and_then(|candidate| match &mut candidate.kind {
                    ReferenceDefinitionKind::LocalObjectStates(states) => Some(states),
                    _ => None,
                })
                .ok_or(EncounterMapItemError::InvalidEncounter(
                    "post-revert state targets a definition absent from the content",
                ))?;
            states.push(state.clone());
        }
        source
            .transitions
            .extend(self.inverse_transitions.iter().cloned());
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
) -> Result<AdmittedMapItemTransform, EncounterMapItemError> {
    let named = || action.to_owned();
    if object.contains_key("at") {
        return Err(EncounterMapItemError::DeathPositionNotAdmitted { action: named() });
    }
    let operation = string_field(object, "operation").unwrap_or_default();
    if operation == "create" && object.contains_key("destination") {
        return Err(EncounterMapItemError::CreateWithDestinationNotAdmitted { action: named() });
    }
    if operation != "transform" {
        return Err(EncounterMapItemError::OperationNotAdmitted {
            action: named(),
            operation: operation.to_owned(),
        });
    }
    if object.contains_key("interaction") {
        return Err(EncounterMapItemError::InteractionNotAdmitted { action: named() });
    }
    if let Some(field) = object
        .keys()
        .find(|field| !ADMITTED_FIELDS.contains(&field.as_str()))
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
        "map_item transform requires a pre-authored anchor",
    ))?;
    let revert_after_ms = match object.get("revert_after_ms") {
        None => None,
        Some(value) => Some(value.as_u64().filter(|ms| *ms > 0).ok_or(
            EncounterMapItemError::InvalidEncounter("revert_after_ms must be a positive integer"),
        )?),
    };
    let revert_destination = anchor_of("revert_destination")?;
    if revert_destination.is_some() && revert_after_ms.is_none() {
        return Err(EncounterMapItemError::RevertDestinationWithoutRevert { action: named() });
    }
    let destination = anchor_of("destination")?;
    if revert_destination.is_some() && destination.is_none() {
        return Err(EncounterMapItemError::RevertDestinationWithoutDestination { action: named() });
    }
    Ok(AdmittedMapItemTransform {
        action: LoweredActionId::new(action)?,
        anchor,
        item: item_key(object, "item")?,
        into: item_key(object, "into")?,
        destination,
        revert_destination,
        revert_after_ms,
    })
}

fn walk_actions(
    prefix: &str,
    actions: &[Value],
    anchors: &BTreeSet<String>,
    admitted: &mut Vec<AdmittedMapItemTransform>,
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

/// Validates every `map_item` action of one encounter against §9's admitted shape, using the
/// encounter alone.
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
    let mut transforms = Vec::new();
    for rule in rules {
        let rule_key = rule.get("key").and_then(Value::as_str).ok_or(
            EncounterMapItemError::InvalidEncounter("encounter rule requires a key"),
        )?;
        let actions = rule.get("actions").and_then(Value::as_array).ok_or(
            EncounterMapItemError::InvalidEncounter("encounter rule requires actions"),
        )?;
        walk_actions(
            &format!("{key}/{rule_key}"),
            actions,
            &anchors,
            &mut transforms,
        )?;
    }
    Ok(AdmittedEncounterMapItems {
        encounter: key.to_owned(),
        transforms,
    })
}

/// Lowers every admitted `map_item transform` of one encounter against `source`, whose
/// `LocalObject` definitions and transitions the transform anchors already carry.
/// `anchor_objects` names the `LocalObject` definition authored at each transform anchor.
pub fn lower_map_item_transforms(
    encounter_json: &str,
    source: &ReferencePlayableContentSource,
    anchor_objects: &BTreeMap<String, TypedDefinitionRef>,
) -> Result<LoweredEncounterMapItems, EncounterMapItemError> {
    let admitted = admitted_map_item_transforms(encounter_json)?;
    let mut lowered = LoweredEncounterMapItems {
        post_revert_states: Vec::new(),
        inverse_transitions: Vec::new(),
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
                });
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
            },
        ));
        lowered.inverse_transitions.push(TransitionBinding {
            key: TransitionKey::new(&format!("{action}/revert"))?,
            definition: definition.clone(),
            source_state: forward.target_state.clone(),
            normalized_intent_family: ProductionKey::new(LOCAL_OBJECT_TRANSFORM_INTENT_FAMILY)?,
            target_state: post_revert.clone(),
            owner_capability: forward.owner_capability.clone(),
            policy_guard_refs: forward.policy_guard_refs.clone(),
        });
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
    Ok(lowered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{
        CanonicalReferencePlayableContent, ClientProjectionClass, ContentLockBinding,
        ContentLockEntry, CoordinateFrameRef, DefinitionFamily, DefinitionRevisionRef,
        EvidenceBindingRef, EvidenceDisposition, LOCAL_OBJECT_CREATE_INTENT_FAMILY,
        LOCAL_OBJECT_OPEN_INTENT_FAMILY, LOCAL_OBJECT_REMOVE_INTENT_FAMILY,
        LocalObjectCollisionPresence, LogicalCell, OwnerCapabilityRequirement,
        PackageManifestBinding, ProductionAtom, REFERENCE_PLAYABLE_CAPABILITY_PROFILE,
        REFERENCE_PLAYABLE_CONTENT_PROFILE_ID, ReferenceDefinition, Sha256HexDigest,
        link_reference_playable,
    };
    use crate::foundation::{
        ChannelId, CharacterId, CharacterLease, CommandId, CommandIngress, CommandRef,
        ConnectionGeneration, FreshAdmissionCommit, FreshAdmissionFacts,
        GameSessionAuthoritySnapshot, GameSessionId, GameSessionState, RuntimeScopeRefV1,
        ScopeOwnershipGeneration, WorldId,
    };
    use crate::world_runtime::{
        LocalObjectCommand, LocalObjectOperation, LocalObjectRuntime, ReferenceContentGeneration,
        ScopeContentGenerationFence, WorldRuntimeError,
    };
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
    const DUKE_ACTION: &str =
        "canary:encounter/the_duke_of_the_depths/the_duke_of_the_depths_death/0";
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
        })
    }

    fn transition(
        key: &str,
        source: &str,
        target: &str,
    ) -> Result<TransitionBinding, ContentError> {
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
            Sha256HexDigest::new(
                "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            )?,
        );
        let provenance = package_manifest.package_provenance_digest()?;
        Ok(ReferencePlayableContentSource {
            profile_revision: ProductionAtom::new(
                "profile",
                REFERENCE_PLAYABLE_CONTENT_PROFILE_ID,
            )?,
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

    /// Injects placement A (carrying `durations`) and placement B (carrying none).
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
        assert_eq!(
            tables.revert_after_ms,
            BTreeMap::from([(
                (
                    TransitionKey::new(FORWARD)?,
                    LoweredActionId::new(DUKE_ACTION)?
                ),
                1_200_000
            )])
        );
        assert_eq!(
            lowered.destination_markers,
            BTreeSet::from([reward, warzone])
        );
        Ok(())
    }

    #[test]
    fn duke_lowered_content_binds_under_the_widened_rule_and_exposes_attributes_by_state()
    -> TestResult {
        let content = duke_content()?;
        let anchor = duke_anchor()?;
        let revert = duke_revert()?;
        let action = LoweredActionId::new(DUKE_ACTION)?;
        let mut runtime = bind(&content, anchor.as_str(), &[FORWARD, revert.as_str()])?;
        let mut ingress = CommandIngress::new();

        // Freshly bound in the natural source state: no destination is reachable.
        assert_eq!(runtime.state_key().as_str(), SEALED_ITEM);
        assert_eq!(runtime.attributes(), None);
        assert_eq!(
            runtime.revert_after_ms(&TransitionKey::new(FORWARD)?, &action),
            Some(1_200_000)
        );
        assert_eq!(runtime.revert_after_ms(&revert, &action), None);

        assert_eq!(invoke(&mut runtime, &mut ingress, 1, FORWARD)?, "COMMITTED");
        assert_eq!(
            destination_of(&runtime),
            Some(anchor_placement_key(DUKE_KEY, "reward_destination")?.as_str())
        );

        // The dedicated inverse (invoked directly; the timed §7 driver is a later allocation)
        // lands on the post-revert variant, which exposes `revert_destination`.
        assert_eq!(
            invoke(&mut runtime, &mut ingress, 2, revert.as_str())?,
            "COMMITTED"
        );
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
    fn widened_inverse_rule_is_inert_for_plain_content_and_scoped_to_bound_transitions()
    -> TestResult {
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

        let create = edit_first(|object| {
            object.insert("operation".to_owned(), Value::from("create"));
            object.remove("into");
        })?;
        assert_eq!(
            lower(&create)?,
            Err(EncounterMapItemError::CreateWithDestinationNotAdmitted { action: action() })
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
    fn open_decision_samples_stay_rejected_and_covered_samples_are_admitted() -> TestResult {
        for (name, json) in OPEN_DECISION_SAMPLES {
            let result = admitted_map_item_transforms(json);
            let open_decision_9 = matches!(name, "death_priest_shargon" | "the_ravager");
            let rejected = match &result {
                Err(EncounterMapItemError::CreateWithDestinationNotAdmitted { .. }) => {
                    open_decision_9
                }
                Err(EncounterMapItemError::DeathPositionNotAdmitted { .. }) => !open_decision_9,
                _ => false,
            };
            assert!(rejected, "{name}: {result:?}");
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
}
