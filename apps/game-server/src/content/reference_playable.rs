use super::{
    ContentError, ContentLockBinding, PackageManifestBinding, ProductionAtom, ProductionKey,
};
use crate::foundation::WorldId;
use std::collections::BTreeSet;

pub const REFERENCE_PLAYABLE_CONTENT_PROFILE_ID: &str = "REFERENCE_PLAYABLE_CONTENT_PROFILE/v1";
pub const REFERENCE_PLAYABLE_CAPABILITY_PROFILE: &str = "content:reference-playable-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DefinitionFamily {
    Terrain,
    Presentation,
    LocalObject,
    Creature,
    Item,
    Ability,
    Effect,
    Formula,
    Behavior,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DefinitionRevisionRef(ProductionAtom);

impl DefinitionRevisionRef {
    pub fn new(value: &str) -> Result<Self, ContentError> {
        Ok(Self(ProductionAtom::new(
            "reference-playable definition revision",
            value,
        )?))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TypedDefinitionRef {
    family: DefinitionFamily,
    key: ProductionKey,
    revision: DefinitionRevisionRef,
}

impl TypedDefinitionRef {
    pub fn new(
        family: DefinitionFamily,
        key: ProductionKey,
        revision: DefinitionRevisionRef,
    ) -> Self {
        Self {
            family,
            key,
            revision,
        }
    }

    pub const fn family(&self) -> DefinitionFamily {
        self.family
    }

    pub fn key(&self) -> &ProductionKey {
        &self.key
    }

    pub fn revision(&self) -> &DefinitionRevisionRef {
        &self.revision
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceEffectFamily {
    Damage,
    Heal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceDefinitionKind {
    Generic,
    Effect(ReferenceEffectFamily),
    LocalObjectStates(Vec<ProductionKey>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientProjectionClass {
    ServerOnly,
    ClientSafe,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceDefinition {
    pub definition: TypedDefinitionRef,
    pub kind: ReferenceDefinitionKind,
    pub client_projection: ClientProjectionClass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceDisposition {
    Proven,
    Observed,
    Derived,
    Unknown,
    Conflict,
    DeclaredDifference,
    OtsHypothesisOnly,
    ObservedPostTarget,
}

impl EvidenceDisposition {
    pub const fn is_reference_promotable(self) -> bool {
        matches!(self, Self::Proven)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceBindingRef {
    pub manifest_revision: ProductionAtom,
    pub case_key: ProductionKey,
    pub disposition: EvidenceDisposition,
}

impl EvidenceBindingRef {
    pub fn new(
        manifest_revision: ProductionAtom,
        case_key: ProductionKey,
        disposition: EvidenceDisposition,
    ) -> Self {
        Self {
            manifest_revision,
            case_key,
            disposition,
        }
    }

    fn require_reference_promotion(&self) -> Result<(), ContentError> {
        if !self.disposition.is_reference_promotable() {
            return Err(ContentError::InvalidArtifact(
                "target-sensitive Reference claim lacks promotable evidence",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PlacementKey(ProductionKey);

impl PlacementKey {
    pub fn new(value: &str) -> Result<Self, ContentError> {
        Ok(Self(ProductionKey::new(value)?))
    }

    pub fn as_production_key(&self) -> &ProductionKey {
        &self.0
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MapRevisionRef(ProductionAtom);

impl MapRevisionRef {
    pub fn new(value: &str) -> Result<Self, ContentError> {
        Ok(Self(ProductionAtom::new(
            "reference-playable map revision",
            value,
        )?))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CoordinateFrameRef(ProductionAtom);

impl CoordinateFrameRef {
    pub fn new(value: &str) -> Result<Self, ContentError> {
        Ok(Self(ProductionAtom::new(
            "reference-playable coordinate frame",
            value,
        )?))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LogicalCell {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpatialAddress {
    pub world_id: WorldId,
    pub coordinate_frame: CoordinateFrameRef,
    pub cell: LogicalCell,
    pub evidence: EvidenceBindingRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FootprintCell {
    pub dx: i32,
    pub dy: i32,
    pub dz: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnresolvedTargetField {
    Unknown,
    Conflict,
    OtsHypothesisOnly,
    ObservedPostTarget,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FootprintRelation {
    Qualified {
        members: Vec<FootprintCell>,
        evidence: EvidenceBindingRef,
    },
    Unresolved(UnresolvedTargetField),
}

impl FootprintRelation {
    fn validate_for_reference(&self) -> Result<(), ContentError> {
        match self {
            Self::Qualified { members, evidence } => {
                evidence.require_reference_promotion()?;
                let mut seen = BTreeSet::new();
                for member in members {
                    if !seen.insert(*member) {
                        return Err(ContentError::InvalidArtifact(
                            "reference-playable footprint contains duplicate member",
                        ));
                    }
                }
                Ok(())
            }
            Self::Unresolved(_) => Err(ContentError::InvalidArtifact(
                "reference-playable footprint remains unresolved",
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementRef {
    pub key: PlacementKey,
    pub map_revision: MapRevisionRef,
    pub definition: TypedDefinitionRef,
    pub address: SpatialAddress,
    pub presentation_footprint: FootprintRelation,
    pub collision_footprint: FootprintRelation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderedPlacementSet {
    pub field_key: ProductionKey,
    pub placement_keys: Vec<PlacementKey>,
    pub evidence: EvidenceBindingRef,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TransitionKey(ProductionKey);

impl TransitionKey {
    pub fn new(value: &str) -> Result<Self, ContentError> {
        Ok(Self(ProductionKey::new(value)?))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerCapabilityRequirement {
    pub capability_key: ProductionKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionBinding {
    pub key: TransitionKey,
    pub definition: TypedDefinitionRef,
    pub source_state: ProductionKey,
    pub normalized_intent_family: ProductionKey,
    pub target_state: ProductionKey,
    pub owner_capability: OwnerCapabilityRequirement,
    pub policy_guard_refs: Vec<ProductionKey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferencePlayableContentSource {
    pub profile_revision: ProductionAtom,
    pub capability_profile: ProductionAtom,
    pub package_manifest: PackageManifestBinding,
    pub content_lock: ContentLockBinding,
    pub world_id: WorldId,
    pub coordinate_frame: CoordinateFrameRef,
    pub definitions: Vec<ReferenceDefinition>,
    pub placements: Vec<PlacementRef>,
    pub ordered_placements: Vec<OrderedPlacementSet>,
    pub transitions: Vec<TransitionBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientSafeDefinitionKind {
    Generic,
    Effect(ReferenceEffectFamily),
    LocalObjectStates(Vec<ProductionKey>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientSafeDefinitionRef {
    pub definition: TypedDefinitionRef,
    pub kind: ClientSafeDefinitionKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalReferencePlayableContent {
    pub profile_revision: ProductionAtom,
    pub capability_profile: ProductionAtom,
    pub package_manifest: PackageManifestBinding,
    pub content_lock: ContentLockBinding,
    pub world_id: WorldId,
    pub coordinate_frame: CoordinateFrameRef,
    pub definitions: Vec<ReferenceDefinition>,
    pub placements: Vec<PlacementRef>,
    pub ordered_placements: Vec<OrderedPlacementSet>,
    pub transitions: Vec<TransitionBinding>,
}

impl CanonicalReferencePlayableContent {
    pub fn client_safe_definitions(&self) -> Vec<ClientSafeDefinitionRef> {
        self.definitions
            .iter()
            .filter(|definition| definition.client_projection == ClientProjectionClass::ClientSafe)
            .map(|definition| ClientSafeDefinitionRef {
                definition: definition.definition.clone(),
                kind: match &definition.kind {
                    ReferenceDefinitionKind::Generic => ClientSafeDefinitionKind::Generic,
                    ReferenceDefinitionKind::Effect(family) => {
                        ClientSafeDefinitionKind::Effect(*family)
                    }
                    ReferenceDefinitionKind::LocalObjectStates(states) => {
                        ClientSafeDefinitionKind::LocalObjectStates(states.clone())
                    }
                },
            })
            .collect()
    }
}

fn validate_content_lock(
    package: &PackageManifestBinding,
    content_lock: &ContentLockBinding,
) -> Result<(), ContentError> {
    let expected_digest = package.package_provenance_digest()?;
    let mut matches = content_lock
        .entries
        .iter()
        .filter(|entry| entry.package_key == package.package_key);
    let root = matches.next().ok_or(ContentError::InvalidArtifact(
        "reference-playable Content Lock lacks root package",
    ))?;
    if matches.next().is_some() {
        return Err(ContentError::InvalidArtifact(
            "reference-playable Content Lock duplicates root package",
        ));
    }
    if root.floating
        || root.dependency
        || root.package_revision != package.package_revision
        || root.package_provenance_digest != expected_digest
    {
        return Err(ContentError::InvalidArtifact(
            "reference-playable Content Lock does not bind exact root package provenance",
        ));
    }
    Ok(())
}

fn validate_definition_shape(definition: &ReferenceDefinition) -> Result<(), ContentError> {
    match (&definition.definition.family, &definition.kind) {
        (DefinitionFamily::Effect, ReferenceDefinitionKind::Effect(_)) => Ok(()),
        (DefinitionFamily::LocalObject, ReferenceDefinitionKind::LocalObjectStates(states)) => {
            if states.is_empty() {
                return Err(ContentError::InvalidArtifact(
                    "reference-playable local object requires finite state vocabulary",
                ));
            }
            let mut unique = BTreeSet::new();
            for state in states {
                if !unique.insert(state.clone()) {
                    return Err(ContentError::DuplicateKey(state.as_str().to_owned()));
                }
            }
            Ok(())
        }
        (DefinitionFamily::Effect, _) => Err(ContentError::InvalidArtifact(
            "reference-playable effect definition requires typed effect family",
        )),
        (DefinitionFamily::LocalObject, _) => Err(ContentError::InvalidArtifact(
            "reference-playable local object requires finite state vocabulary",
        )),
        (_, ReferenceDefinitionKind::Effect(_))
        | (_, ReferenceDefinitionKind::LocalObjectStates(_)) => Err(ContentError::InvalidArtifact(
            "reference-playable definition kind does not match definition family",
        )),
        (_, ReferenceDefinitionKind::Generic) => Ok(()),
    }
}

fn resolve_definition<'a>(
    definitions: &'a [ReferenceDefinition],
    reference: &TypedDefinitionRef,
) -> Result<&'a ReferenceDefinition, ContentError> {
    if let Some(definition) = definitions
        .iter()
        .find(|candidate| candidate.definition == *reference)
    {
        return Ok(definition);
    }
    if definitions.iter().any(|candidate| {
        candidate.definition.family == reference.family && candidate.definition.key == reference.key
    }) {
        return Err(ContentError::RevisionMismatch(
            "reference-playable definition revision",
        ));
    }
    Err(ContentError::MissingReference {
        owner: format!("{:?}:{}", reference.family, reference.key.as_str()),
        target: "exact typed definition".to_owned(),
    })
}

fn validate_placement(
    source: &ReferencePlayableContentSource,
    placement: &PlacementRef,
) -> Result<(), ContentError> {
    resolve_definition(&source.definitions, &placement.definition)?;
    if placement.address.world_id != source.world_id {
        return Err(ContentError::InvalidArtifact(
            "reference-playable placement WorldId mismatch",
        ));
    }
    if placement.address.coordinate_frame != source.coordinate_frame {
        return Err(ContentError::InvalidArtifact(
            "reference-playable coordinate frame mismatch",
        ));
    }
    placement.address.evidence.require_reference_promotion()?;
    placement.presentation_footprint.validate_for_reference()?;
    placement.collision_footprint.validate_for_reference()?;
    Ok(())
}

fn validate_ordering(
    placements: &[PlacementRef],
    ordered: &OrderedPlacementSet,
) -> Result<(), ContentError> {
    ordered.evidence.require_reference_promotion()?;
    let mut seen = BTreeSet::new();
    for key in &ordered.placement_keys {
        if !seen.insert(key.clone()) {
            return Err(ContentError::DuplicateKey(key.as_str().to_owned()));
        }
        if !placements.iter().any(|placement| placement.key == *key) {
            return Err(ContentError::MissingReference {
                owner: ordered.field_key.as_str().to_owned(),
                target: key.as_str().to_owned(),
            });
        }
    }
    Ok(())
}

fn validate_transition(
    definitions: &[ReferenceDefinition],
    transition: &TransitionBinding,
) -> Result<(), ContentError> {
    let definition = resolve_definition(definitions, &transition.definition)?;
    if transition.definition.family != DefinitionFamily::LocalObject {
        return Err(ContentError::InvalidArtifact(
            "reference-playable transition must target local-object definition",
        ));
    }
    let ReferenceDefinitionKind::LocalObjectStates(states) = &definition.kind else {
        return Err(ContentError::InvalidArtifact(
            "reference-playable transition target lacks state vocabulary",
        ));
    };
    for state in [&transition.source_state, &transition.target_state] {
        if !states.contains(state) {
            return Err(ContentError::MissingReference {
                owner: transition.key.as_str().to_owned(),
                target: state.as_str().to_owned(),
            });
        }
    }
    Ok(())
}

pub fn link_reference_playable(
    mut source: ReferencePlayableContentSource,
) -> Result<CanonicalReferencePlayableContent, ContentError> {
    if source.profile_revision.as_str() != REFERENCE_PLAYABLE_CONTENT_PROFILE_ID {
        return Err(ContentError::RevisionMismatch(
            "reference-playable profile revision",
        ));
    }
    if source.capability_profile.as_str() != REFERENCE_PLAYABLE_CAPABILITY_PROFILE {
        return Err(ContentError::RevisionMismatch(
            "reference-playable capability profile",
        ));
    }
    validate_content_lock(&source.package_manifest, &source.content_lock)?;

    let mut definition_keys = BTreeSet::new();
    for definition in &source.definitions {
        validate_definition_shape(definition)?;
        let identity = (
            definition.definition.family,
            definition.definition.key.clone(),
        );
        if !definition_keys.insert(identity) {
            return Err(ContentError::DuplicateKey(
                definition.definition.key.as_str().to_owned(),
            ));
        }
    }

    let mut placement_keys = BTreeSet::new();
    for placement in &source.placements {
        if !placement_keys.insert(placement.key.clone()) {
            return Err(ContentError::DuplicateKey(
                placement.key.as_str().to_owned(),
            ));
        }
        validate_placement(&source, placement)?;
    }

    let mut ordering_keys = BTreeSet::new();
    for ordered in &source.ordered_placements {
        if !ordering_keys.insert(ordered.field_key.clone()) {
            return Err(ContentError::DuplicateKey(
                ordered.field_key.as_str().to_owned(),
            ));
        }
        validate_ordering(&source.placements, ordered)?;
    }

    let mut transition_keys = BTreeSet::new();
    for transition in &source.transitions {
        if !transition_keys.insert(transition.key.clone()) {
            return Err(ContentError::DuplicateKey(
                transition.key.as_str().to_owned(),
            ));
        }
        validate_transition(&source.definitions, transition)?;
    }

    source.definitions.sort_by(|left, right| {
        left.definition
            .family
            .cmp(&right.definition.family)
            .then_with(|| left.definition.key.cmp(&right.definition.key))
            .then_with(|| left.definition.revision.cmp(&right.definition.revision))
    });
    source
        .placements
        .sort_by(|left, right| left.key.cmp(&right.key));
    source
        .ordered_placements
        .sort_by(|left, right| left.field_key.cmp(&right.field_key));
    source
        .transitions
        .sort_by(|left, right| left.key.cmp(&right.key));

    Ok(CanonicalReferencePlayableContent {
        profile_revision: source.profile_revision,
        capability_profile: source.capability_profile,
        package_manifest: source.package_manifest,
        content_lock: source.content_lock,
        world_id: source.world_id,
        coordinate_frame: source.coordinate_frame,
        definitions: source.definitions,
        placements: source.placements,
        ordered_placements: source.ordered_placements,
        transitions: source.transitions,
    })
}
