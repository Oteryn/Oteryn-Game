use super::{
    ContentError, ContentLockBinding, PackageManifestBinding, ProductionAtom, ProductionKey,
};
use crate::foundation::WorldId;
use serde::Deserialize;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceItemPhysicalClass {
    Physical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceItemStackClass {
    NonStackable,
    StackCapable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReferenceItemDestination {
    CharacterInventory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceItemDefinition {
    pub physical_class: ReferenceItemPhysicalClass,
    pub materializable: bool,
    pub stack_class: ReferenceItemStackClass,
    pub legal_destinations: Vec<ReferenceItemDestination>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceDefinitionKind {
    Generic,
    Effect(ReferenceEffectFamily),
    Item(ReferenceItemDefinition),
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

const REFERENCE_EVIDENCE_MANIFEST_JSON: &str =
    include_str!("../../../../docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json");
const REFERENCE_EVIDENCE_CASE_KEY_PREFIX: &str = "oteryn:reference.case.";

#[derive(Debug, Deserialize)]
struct AcceptedReferenceEvidenceManifest {
    schema_version: u64,
    manifest_revision: u64,
    status: String,
    cases: Vec<AcceptedReferenceEvidenceCase>,
}

#[derive(Debug, Deserialize)]
struct AcceptedReferenceEvidenceCase {
    case_id: String,
    domain: String,
    target: AcceptedReferenceEvidenceTarget,
    provenance: AcceptedReferenceEvidenceProvenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReferenceTargetClaim {
    SpatialAddress,
    PresentationFootprint,
    CollisionFootprint,
    OrderedPlacementSequence,
}

// D1 requires per-target-sensitive evidence binding. The evidence manifest currently has no
// CONTENT_WORLD mechanic case that can authorize any of these claims. Keep this exact-case
// consumer binding empty until a separately accepted manifest case establishes the semantics;
// a PROVEN case from another domain must never become geometry/footprint/order authority.
const REFERENCE_TARGET_CLAIM_CASE_BINDINGS: &[(&str, ReferenceTargetClaim)] = &[];

#[derive(Debug, Deserialize)]
struct AcceptedReferenceEvidenceTarget {
    evidence_class: String,
    sources: Vec<AcceptedReferenceEvidenceSource>,
}

#[derive(Debug, Deserialize)]
struct AcceptedReferenceEvidenceSource {
    source_type: String,
    provenance_state: String,
}

#[derive(Debug, Deserialize)]
struct AcceptedReferenceEvidenceProvenance {
    state: String,
    legal_review_state: String,
}

#[derive(Debug)]
struct ReferenceEvidenceAuthority {
    manifest: AcceptedReferenceEvidenceManifest,
}

impl ReferenceEvidenceAuthority {
    fn load() -> Result<Self, ContentError> {
        let manifest: AcceptedReferenceEvidenceManifest =
            serde_json::from_str(REFERENCE_EVIDENCE_MANIFEST_JSON).map_err(|_| {
                ContentError::InvalidArtifact(
                    "accepted Reference evidence manifest cannot be decoded",
                )
            })?;
        if manifest.schema_version != 1
            || manifest.manifest_revision == 0
            || manifest.status != "ACCEPTED"
        {
            return Err(ContentError::InvalidArtifact(
                "accepted Reference evidence manifest identity is invalid",
            ));
        }
        let mut case_ids = BTreeSet::new();
        for case in &manifest.cases {
            if case.case_id.is_empty() || !case_ids.insert(case.case_id.clone()) {
                return Err(ContentError::InvalidArtifact(
                    "accepted Reference evidence manifest case identity is invalid",
                ));
            }
        }
        Ok(Self { manifest })
    }

    fn manifest_revision_atom(&self) -> Result<ProductionAtom, ContentError> {
        ProductionAtom::new(
            "reference manifest revision",
            &format!("manifest-r{}", self.manifest.manifest_revision),
        )
    }

    fn require_case_bound_to_claim(
        &self,
        case: &AcceptedReferenceEvidenceCase,
        required_claim: ReferenceTargetClaim,
    ) -> Result<(), ContentError> {
        if case.domain != "CONTENT_WORLD" {
            return Err(ContentError::InvalidArtifact(
                "reference-playable evidence case domain does not match target-sensitive claim",
            ));
        }
        let bound_claim = REFERENCE_TARGET_CLAIM_CASE_BINDINGS
            .iter()
            .find_map(|(case_id, claim)| (case.case_id == *case_id).then_some(*claim));
        if bound_claim != Some(required_claim) {
            return Err(ContentError::InvalidArtifact(
                "reference-playable evidence case is not bound to target-sensitive claim",
            ));
        }
        Ok(())
    }

    fn resolve_case<'a>(
        &'a self,
        case_key: &ProductionKey,
    ) -> Result<&'a AcceptedReferenceEvidenceCase, ContentError> {
        let case_id = case_key
            .as_str()
            .strip_prefix(REFERENCE_EVIDENCE_CASE_KEY_PREFIX)
            .ok_or(ContentError::InvalidArtifact(
                "reference-playable evidence case key is invalid",
            ))?;
        self.manifest
            .cases
            .iter()
            .find(|case| case.case_id == case_id)
            .ok_or_else(|| ContentError::MissingReference {
                owner: case_key.as_str().to_owned(),
                target: "accepted Reference evidence case".to_owned(),
            })
    }
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

    fn from_manifest(value: &str) -> Result<Self, ContentError> {
        match value {
            "PROVEN" => Ok(Self::Proven),
            "OBSERVED" => Ok(Self::Observed),
            "DERIVED" => Ok(Self::Derived),
            "UNKNOWN" => Ok(Self::Unknown),
            "CONFLICT" => Ok(Self::Conflict),
            "DECLARED_DIFFERENCE" => Ok(Self::DeclaredDifference),
            _ => Err(ContentError::InvalidArtifact(
                "accepted Reference evidence class is unsupported",
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceBindingRef {
    manifest_revision: ProductionAtom,
    case_key: ProductionKey,
    disposition: EvidenceDisposition,
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

    pub fn from_accepted_case(case_key: ProductionKey) -> Result<Self, ContentError> {
        let authority = ReferenceEvidenceAuthority::load()?;
        let case = authority.resolve_case(&case_key)?;
        Ok(Self {
            manifest_revision: authority.manifest_revision_atom()?,
            case_key,
            disposition: EvidenceDisposition::from_manifest(&case.target.evidence_class)?,
        })
    }

    pub fn manifest_revision(&self) -> &ProductionAtom {
        &self.manifest_revision
    }

    pub fn case_key(&self) -> &ProductionKey {
        &self.case_key
    }

    pub const fn disposition(&self) -> EvidenceDisposition {
        self.disposition
    }

    fn require_reference_promotion(
        &self,
        authority: &ReferenceEvidenceAuthority,
        required_claim: ReferenceTargetClaim,
    ) -> Result<(), ContentError> {
        if self.manifest_revision != authority.manifest_revision_atom()? {
            return Err(ContentError::RevisionMismatch(
                "reference-playable evidence manifest revision",
            ));
        }

        let case = authority.resolve_case(&self.case_key)?;
        let actual_disposition = EvidenceDisposition::from_manifest(&case.target.evidence_class)?;
        if self.disposition != actual_disposition {
            return Err(ContentError::InvalidArtifact(
                "reference-playable evidence disposition does not match accepted manifest",
            ));
        }
        if !actual_disposition.is_reference_promotable() {
            return Err(ContentError::InvalidArtifact(
                "target-sensitive Reference claim lacks promotable evidence",
            ));
        }

        let all_sources_cleared = !case.target.sources.is_empty()
            && case
                .target
                .sources
                .iter()
                .all(|source| source.provenance_state == "CLEARED");
        let has_non_ots_source = case
            .target
            .sources
            .iter()
            .any(|source| source.source_type != "OTS_HYPOTHESIS_ONLY");
        if case.provenance.state != "CLEARED"
            || case.provenance.legal_review_state != "CLEARED"
            || !all_sources_cleared
            || !has_non_ots_source
        {
            return Err(ContentError::InvalidArtifact(
                "target-sensitive Reference claim lacks cleared provenance",
            ));
        }
        authority.require_case_bound_to_claim(case, required_claim)?;
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
    fn canonicalize_structure(&mut self) -> Result<(), ContentError> {
        if let Self::Qualified { members, .. } = self {
            members.sort();
            if members.windows(2).any(|pair| pair[0] == pair[1]) {
                return Err(ContentError::InvalidArtifact(
                    "reference-playable footprint contains duplicate member",
                ));
            }
        }
        Ok(())
    }

    fn require_resolved(&self) -> Result<(), ContentError> {
        if matches!(self, Self::Unresolved(_)) {
            return Err(ContentError::InvalidArtifact(
                "reference-playable footprint remains unresolved",
            ));
        }
        Ok(())
    }

    fn validate_for_reference(
        &self,
        authority: &ReferenceEvidenceAuthority,
        required_claim: ReferenceTargetClaim,
    ) -> Result<(), ContentError> {
        match self {
            Self::Qualified { evidence, .. } => {
                evidence.require_reference_promotion(authority, required_claim)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientSafeItemDefinition {
    pub physical_class: ReferenceItemPhysicalClass,
    pub stack_class: ReferenceItemStackClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientSafeDefinitionKind {
    Generic,
    Effect(ReferenceEffectFamily),
    Item(ClientSafeItemDefinition),
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
                    ReferenceDefinitionKind::Item(item) => {
                        ClientSafeDefinitionKind::Item(ClientSafeItemDefinition {
                            physical_class: item.physical_class,
                            stack_class: item.stack_class,
                        })
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

fn validate_item_definition(item: &ReferenceItemDefinition) -> Result<(), ContentError> {
    let mut destinations = BTreeSet::new();
    for destination in &item.legal_destinations {
        if !destinations.insert(*destination) {
            return Err(ContentError::InvalidArtifact(
                "reference-playable item duplicates legal destination capability",
            ));
        }
    }

    let inventory_legal = destinations.contains(&ReferenceItemDestination::CharacterInventory);
    if item.materializable && !inventory_legal {
        return Err(ContentError::InvalidArtifact(
            "reference-playable materializable item requires CharacterInventory destination capability",
        ));
    }
    if !item.materializable && inventory_legal {
        return Err(ContentError::InvalidArtifact(
            "reference-playable non-materializable item cannot declare CharacterInventory destination capability",
        ));
    }
    Ok(())
}

fn validate_definition_shape(definition: &ReferenceDefinition) -> Result<(), ContentError> {
    match (&definition.definition.family, &definition.kind) {
        (DefinitionFamily::Effect, ReferenceDefinitionKind::Effect(_)) => Ok(()),
        (DefinitionFamily::Item, ReferenceDefinitionKind::Item(item)) => {
            validate_item_definition(item)
        }
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
        (DefinitionFamily::Item, _) => Err(ContentError::InvalidArtifact(
            "reference-playable item requires typed static item semantics",
        )),
        (DefinitionFamily::LocalObject, _) => Err(ContentError::InvalidArtifact(
            "reference-playable local object requires finite state vocabulary",
        )),
        (_, ReferenceDefinitionKind::Effect(_))
        | (_, ReferenceDefinitionKind::Item(_))
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
    authority: &ReferenceEvidenceAuthority,
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
    placement.presentation_footprint.require_resolved()?;
    placement.collision_footprint.require_resolved()?;
    placement
        .address
        .evidence
        .require_reference_promotion(authority, ReferenceTargetClaim::SpatialAddress)?;
    placement
        .presentation_footprint
        .validate_for_reference(authority, ReferenceTargetClaim::PresentationFootprint)?;
    placement
        .collision_footprint
        .validate_for_reference(authority, ReferenceTargetClaim::CollisionFootprint)?;
    Ok(())
}

fn validate_ordering_structure(
    placements: &[PlacementRef],
    ordered: &OrderedPlacementSet,
) -> Result<(), ContentError> {
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

fn validate_ordering_evidence(
    ordered: &OrderedPlacementSet,
    authority: &ReferenceEvidenceAuthority,
) -> Result<(), ContentError> {
    ordered
        .evidence
        .require_reference_promotion(authority, ReferenceTargetClaim::OrderedPlacementSequence)
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
    let evidence_authority = ReferenceEvidenceAuthority::load()?;

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

    for placement in &mut source.placements {
        placement.presentation_footprint.canonicalize_structure()?;
        placement.collision_footprint.canonicalize_structure()?;
    }

    let mut placement_keys = BTreeSet::new();
    for placement in &source.placements {
        if !placement_keys.insert(placement.key.clone()) {
            return Err(ContentError::DuplicateKey(
                placement.key.as_str().to_owned(),
            ));
        }
    }
    let mut ordering_keys = BTreeSet::new();
    for ordered in &source.ordered_placements {
        if !ordering_keys.insert(ordered.field_key.clone()) {
            return Err(ContentError::DuplicateKey(
                ordered.field_key.as_str().to_owned(),
            ));
        }
        validate_ordering_structure(&source.placements, ordered)?;
    }

    for placement in &source.placements {
        validate_placement(&source, placement, &evidence_authority)?;
    }
    for ordered in &source.ordered_placements {
        validate_ordering_evidence(ordered, &evidence_authority)?;
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

#[cfg(test)]
mod corrective_tests {
    use super::*;

    #[test]
    fn accepted_case_from_other_domain_cannot_bind_content_world_target_claim()
    -> Result<(), ContentError> {
        let authority = ReferenceEvidenceAuthority::load()?;
        let key = ProductionKey::new(
            "oteryn:reference.case.ability_combat.light_healing.self_heal_semantics.v1",
        )?;
        let case = authority.resolve_case(&key)?;

        assert!(matches!(
            authority.require_case_bound_to_claim(case, ReferenceTargetClaim::SpatialAddress),
            Err(ContentError::InvalidArtifact(
                "reference-playable evidence case domain does not match target-sensitive claim"
            ))
        ));
        Ok(())
    }

    #[test]
    fn footprint_member_order_is_canonicalized() -> Result<(), ContentError> {
        let evidence = EvidenceBindingRef::new(
            ProductionAtom::new("reference manifest revision", "manifest-r1")?,
            ProductionKey::new(
                "oteryn:reference.case.ability_combat.light_healing.self_heal_semantics.v1",
            )?,
            EvidenceDisposition::Unknown,
        );
        let low = FootprintCell {
            dx: 0,
            dy: 0,
            dz: 0,
        };
        let high = FootprintCell {
            dx: 1,
            dy: 0,
            dz: 0,
        };
        let mut relation = FootprintRelation::Qualified {
            members: vec![high, low],
            evidence,
        };

        relation.canonicalize_structure()?;

        let FootprintRelation::Qualified { members, .. } = relation else {
            unreachable!("qualified footprint changed variant");
        };
        assert_eq!(members, vec![low, high]);
        Ok(())
    }
}
