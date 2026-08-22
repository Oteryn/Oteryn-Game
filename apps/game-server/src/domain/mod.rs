//! Protocol- and persistence-neutral Character and Item semantic core.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainError {
    NilUuid,
    UuidVersion,
    UuidVariant,
    ZeroRevision,
    RevisionExhausted,
    StaleCharacterRevision,
    InvalidLifecycleTransition,
    TerminalLifecycle,
    NotQuiescent,
    InvalidLimit,
    ExceedsEngineCeiling,
    ExceedsLimit,
    MissingPrimarySlot,
    ContainerSelfCycle,
    ContainerCycle,
    ContainerDirectLimit,
    ContainerDepthLimit,
    ContainerReachableLimit,
    ContainerAlreadyAttached,
    UnsupportedInterpretationContext,
    UnsupportedDefinitionRevision,
    LocationSurfaceNotAllowed,
    MismatchedWorld,
    InvalidFixtureProfileVersion,
    UnsupportedFixtureProfileVersion,
    FixtureProfileCannotActivateProduct,
}

impl Display for DomainError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NilUuid => "UUID identity must be non-nil",
            Self::UuidVersion => "UUID identity must use version 7",
            Self::UuidVariant => "UUID identity must use the RFC variant",
            Self::ZeroRevision => "revision must be non-zero",
            Self::RevisionExhausted => "revision space is exhausted",
            Self::StaleCharacterRevision => "character revision is stale",
            Self::InvalidLifecycleTransition => "character lifecycle transition is invalid",
            Self::TerminalLifecycle => "retired character lifecycle is terminal",
            Self::NotQuiescent => "character must be quiescent for this transition",
            Self::InvalidLimit => "semantic limit must be positive and internally consistent",
            Self::ExceedsEngineCeiling => "definition limit exceeds the injected engine ceiling",
            Self::ExceedsLimit => "value exceeds the definition limit",
            Self::MissingPrimarySlot => "equip pattern must claim its primary slot",
            Self::ContainerSelfCycle => "an item cannot contain itself",
            Self::ContainerCycle => "containment would create a cycle",
            Self::ContainerDirectLimit => "container direct-entry limit would be exceeded",
            Self::ContainerDepthLimit => "container nesting depth limit would be exceeded",
            Self::ContainerReachableLimit => "container reachable-item limit would be exceeded",
            Self::ContainerAlreadyAttached => "item already has an immediate container parent",
            Self::UnsupportedInterpretationContext => {
                "character interpretation context is incompatible"
            }
            Self::UnsupportedDefinitionRevision => "item definition revision is incompatible",
            Self::LocationSurfaceNotAllowed => {
                "item definition does not permit this location surface"
            }
            Self::MismatchedWorld => "item location is outside the item logical world",
            Self::InvalidFixtureProfileVersion => "fixture profile version must be non-zero",
            Self::UnsupportedFixtureProfileVersion => {
                "fixture profile version is not the expected version"
            }
            Self::FixtureProfileCannotActivateProduct => {
                "fixture-only structural limits cannot activate as product policy"
            }
        })
    }
}

impl Error for DomainError {}

fn validate_uuid_v7(bytes: &[u8; 16]) -> Result<(), DomainError> {
    if bytes.iter().all(|byte| *byte == 0) {
        return Err(DomainError::NilUuid);
    }
    if bytes[6] >> 4 != 0x07 {
        return Err(DomainError::UuidVersion);
    }
    if bytes[8] >> 6 != 0b10 {
        return Err(DomainError::UuidVariant);
    }
    Ok(())
}

macro_rules! strong_uuid_v7 {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; 16]);

        impl $name {
            pub fn from_bytes(bytes: [u8; 16]) -> Result<Self, DomainError> {
                validate_uuid_v7(&bytes)?;
                Ok(Self(bytes))
            }

            #[must_use]
            pub const fn as_bytes(&self) -> &[u8; 16] {
                &self.0
            }
        }
    };
}

strong_uuid_v7!(CharacterId);
strong_uuid_v7!(ItemInstanceId);
strong_uuid_v7!(WorldId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CharacterRevision(u64);

impl CharacterRevision {
    pub fn new(value: u64) -> Result<Self, DomainError> {
        if value == 0 {
            return Err(DomainError::ZeroRevision);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    fn successor(self) -> Result<Self, DomainError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(DomainError::RevisionExhausted)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterLifecycle {
    Active,
    DeletionScheduled,
    Retired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterInterpretationContext<R> {
    profile: R,
    ruleset: R,
    content: R,
    starter_template: R,
}

impl<R> CharacterInterpretationContext<R> {
    #[must_use]
    pub const fn new(profile: R, ruleset: R, content: R, starter_template: R) -> Self {
        Self {
            profile,
            ruleset,
            content,
            starter_template,
        }
    }

    #[must_use]
    pub const fn profile(&self) -> &R {
        &self.profile
    }
    #[must_use]
    pub const fn ruleset(&self) -> &R {
        &self.ruleset
    }
    #[must_use]
    pub const fn content(&self) -> &R {
        &self.content
    }
    #[must_use]
    pub const fn starter_template(&self) -> &R {
        &self.starter_template
    }
}

impl<R> CharacterInterpretationContext<R>
where
    R: PartialEq,
{
    pub fn ensure_compatible(&self, expected: &Self) -> Result<(), DomainError> {
        if self != expected {
            return Err(DomainError::UnsupportedInterpretationContext);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterDefinitionRef<K, R> {
    key: K,
    revision: R,
}

impl<K, R> CharacterDefinitionRef<K, R> {
    #[must_use]
    pub const fn new(key: K, revision: R) -> Self {
        Self { key, revision }
    }

    #[must_use]
    pub const fn key(&self) -> &K {
        &self.key
    }

    #[must_use]
    pub const fn revision(&self) -> &R {
        &self.revision
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharacterBuildState<K, R> {
    Unselected,
    Selected {
        profession: CharacterDefinitionRef<K, R>,
        promotion: Option<CharacterDefinitionRef<K, R>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterProgressionFact<K, R, V> {
    definition: CharacterDefinitionRef<K, R>,
    value: V,
}

impl<K, R, V> CharacterProgressionFact<K, R, V> {
    #[must_use]
    pub const fn new(definition: CharacterDefinitionRef<K, R>, value: V) -> Self {
        Self { definition, value }
    }

    #[must_use]
    pub const fn definition(&self) -> &CharacterDefinitionRef<K, R> {
        &self.definition
    }

    #[must_use]
    pub const fn value(&self) -> &V {
        &self.value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quiescence {
    actor_absent: bool,
    no_playable_lease: bool,
}

impl Quiescence {
    pub const QUIESCENT: Self = Self {
        actor_absent: true,
        no_playable_lease: true,
    };
    pub const NOT_QUIESCENT: Self = Self {
        actor_absent: false,
        no_playable_lease: true,
    };

    #[must_use]
    pub const fn new(actor_absent: bool, no_playable_lease: bool) -> Self {
        Self {
            actor_absent,
            no_playable_lease,
        }
    }

    #[must_use]
    pub const fn is_quiescent(self) -> bool {
        self.actor_absent && self.no_playable_lease
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterRecord<R, B> {
    id: CharacterId,
    lifecycle: CharacterLifecycle,
    revision: CharacterRevision,
    context: CharacterInterpretationContext<R>,
    build: B,
}

impl<R, B> CharacterRecord<R, B> {
    #[must_use]
    pub const fn new(
        id: CharacterId,
        context: CharacterInterpretationContext<R>,
        build: B,
    ) -> Self {
        Self {
            id,
            lifecycle: CharacterLifecycle::Active,
            revision: CharacterRevision(1),
            context,
            build,
        }
    }

    #[must_use]
    pub const fn id(&self) -> CharacterId {
        self.id
    }
    #[must_use]
    pub const fn lifecycle(&self) -> CharacterLifecycle {
        self.lifecycle
    }
    #[must_use]
    pub const fn revision(&self) -> CharacterRevision {
        self.revision
    }
    #[must_use]
    pub const fn context(&self) -> &CharacterInterpretationContext<R> {
        &self.context
    }
    #[must_use]
    pub const fn build(&self) -> &B {
        &self.build
    }

    fn verify_revision(&self, expected: CharacterRevision) -> Result<(), DomainError> {
        if expected != self.revision {
            return Err(DomainError::StaleCharacterRevision);
        }
        Ok(())
    }

    pub fn schedule_deletion(
        &mut self,
        expected: CharacterRevision,
    ) -> Result<CharacterRevision, DomainError> {
        self.verify_revision(expected)?;
        match self.lifecycle {
            CharacterLifecycle::Active => {
                let next_revision = self.revision.successor()?;
                self.lifecycle = CharacterLifecycle::DeletionScheduled;
                self.revision = next_revision;
                Ok(next_revision)
            }
            CharacterLifecycle::Retired => Err(DomainError::TerminalLifecycle),
            CharacterLifecycle::DeletionScheduled => Err(DomainError::InvalidLifecycleTransition),
        }
    }

    pub fn restore(
        &mut self,
        expected: CharacterRevision,
    ) -> Result<CharacterRevision, DomainError> {
        self.verify_revision(expected)?;
        match self.lifecycle {
            CharacterLifecycle::DeletionScheduled => {
                let next_revision = self.revision.successor()?;
                self.lifecycle = CharacterLifecycle::Active;
                self.revision = next_revision;
                Ok(next_revision)
            }
            CharacterLifecycle::Retired => Err(DomainError::TerminalLifecycle),
            CharacterLifecycle::Active => Err(DomainError::InvalidLifecycleTransition),
        }
    }

    pub fn retire(
        &mut self,
        expected: CharacterRevision,
        quiescence: Quiescence,
    ) -> Result<CharacterRevision, DomainError> {
        self.verify_revision(expected)?;
        match self.lifecycle {
            CharacterLifecycle::Retired => return Err(DomainError::TerminalLifecycle),
            CharacterLifecycle::Active => return Err(DomainError::InvalidLifecycleTransition),
            CharacterLifecycle::DeletionScheduled => {}
        }
        if !quiescence.is_quiescent() {
            return Err(DomainError::NotQuiescent);
        }
        let next_revision = self.revision.successor()?;
        self.lifecycle = CharacterLifecycle::Retired;
        self.revision = next_revision;
        Ok(next_revision)
    }
}

impl<R, B> CharacterRecord<R, B>
where
    R: PartialEq,
{
    pub fn ensure_context(
        &self,
        expected: &CharacterInterpretationContext<R>,
    ) -> Result<(), DomainError> {
        self.context.ensure_compatible(expected)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineCeiling(u32);

impl EngineCeiling {
    pub fn new(value: u32) -> Result<Self, DomainError> {
        if value == 0 {
            return Err(DomainError::InvalidLimit);
        }
        Ok(Self(value))
    }
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefinitionLimit(u32);

impl DefinitionLimit {
    pub fn new(value: u32, engine: EngineCeiling) -> Result<Self, DomainError> {
        if value == 0 {
            return Err(DomainError::InvalidLimit);
        }
        if value > engine.0 {
            return Err(DomainError::ExceedsEngineCeiling);
        }
        Ok(Self(value))
    }
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackState {
    quantity: u32,
}

impl StackState {
    pub fn new(quantity: u32, limit: DefinitionLimit) -> Result<Self, DomainError> {
        if quantity == 0 {
            return Err(DomainError::InvalidLimit);
        }
        if quantity > limit.0 {
            return Err(DomainError::ExceedsLimit);
        }
        Ok(Self { quantity })
    }
    #[must_use]
    pub const fn quantity(self) -> u32 {
        self.quantity
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemDefinitionRef<K, R> {
    key: K,
    revision: R,
}

impl<K, R> ItemDefinitionRef<K, R> {
    #[must_use]
    pub const fn new(key: K, revision: R) -> Self {
        Self { key, revision }
    }
    #[must_use]
    pub const fn key(&self) -> &K {
        &self.key
    }
    #[must_use]
    pub const fn revision(&self) -> &R {
        &self.revision
    }
}

impl<K, R> ItemDefinitionRef<K, R>
where
    R: PartialEq,
{
    pub fn ensure_revision(&self, expected: &R) -> Result<(), DomainError> {
        if &self.revision != expected {
            return Err(DomainError::UnsupportedDefinitionRevision);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemInstance<K, R> {
    id: ItemInstanceId,
    world: WorldId,
    definition: ItemDefinitionRef<K, R>,
}

impl<K, R> ItemInstance<K, R> {
    #[must_use]
    pub const fn new(
        id: ItemInstanceId,
        world: WorldId,
        definition: ItemDefinitionRef<K, R>,
    ) -> Self {
        Self {
            id,
            world,
            definition,
        }
    }
    #[must_use]
    pub const fn id(&self) -> ItemInstanceId {
        self.id
    }
    #[must_use]
    pub const fn world(&self) -> WorldId {
        self.world
    }
    #[must_use]
    pub const fn definition(&self) -> &ItemDefinitionRef<K, R> {
        &self.definition
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemLocationSurface {
    CharacterInventory,
    CharacterEquipment,
    Container,
    Ground,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemLocationRef<I, E, C, G, P> {
    CharacterInventory {
        character_id: CharacterId,
        position: I,
    },
    CharacterEquipment {
        character_id: CharacterId,
        occupancy: E,
    },
    Container {
        parent_item_instance_id: ItemInstanceId,
        entry: C,
    },
    Ground {
        world_id: WorldId,
        runtime_scope: G,
        spatial_position: P,
    },
}

impl<I, E, C, G, P> ItemLocationRef<I, E, C, G, P> {
    #[must_use]
    pub const fn surface(&self) -> ItemLocationSurface {
        match self {
            Self::CharacterInventory { .. } => ItemLocationSurface::CharacterInventory,
            Self::CharacterEquipment { .. } => ItemLocationSurface::CharacterEquipment,
            Self::Container { .. } => ItemLocationSurface::Container,
            Self::Ground { .. } => ItemLocationSurface::Ground,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemPlacementPolicy {
    inventory: bool,
    equipment: bool,
    container: bool,
    ground: bool,
}

impl ItemPlacementPolicy {
    #[must_use]
    pub const fn new(inventory: bool, equipment: bool, container: bool, ground: bool) -> Self {
        Self {
            inventory,
            equipment,
            container,
            ground,
        }
    }

    pub fn validate<I, E, C, G, P>(
        self,
        location: &ItemLocationRef<I, E, C, G, P>,
    ) -> Result<(), DomainError> {
        let allowed = match location.surface() {
            ItemLocationSurface::CharacterInventory => self.inventory,
            ItemLocationSurface::CharacterEquipment => self.equipment,
            ItemLocationSurface::Container => self.container,
            ItemLocationSurface::Ground => self.ground,
        };
        if !allowed {
            return Err(DomainError::LocationSurfaceNotAllowed);
        }
        Ok(())
    }
}

impl<K, R> ItemInstance<K, R> {
    pub fn validate_location<I, E, C, G, P>(
        &self,
        policy: ItemPlacementPolicy,
        location: &ItemLocationRef<I, E, C, G, P>,
    ) -> Result<(), DomainError> {
        policy.validate(location)?;
        if let ItemLocationRef::Ground { world_id, .. } = location
            && *world_id != self.world
        {
            return Err(DomainError::MismatchedWorld);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixtureProfileVersion(u32);

impl FixtureProfileVersion {
    pub fn new(value: u32) -> Result<Self, DomainError> {
        if value == 0 {
            return Err(DomainError::InvalidFixtureProfileVersion);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureProfileActivation {
    StructuralTest,
    ProductPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StructuralFixtureProfile {
    version: FixtureProfileVersion,
    stack_engine_ceiling: EngineCeiling,
    container_limits: ContainerLimits,
}

impl StructuralFixtureProfile {
    #[must_use]
    pub const fn new(
        version: FixtureProfileVersion,
        stack_engine_ceiling: EngineCeiling,
        container_limits: ContainerLimits,
    ) -> Self {
        Self {
            version,
            stack_engine_ceiling,
            container_limits,
        }
    }

    pub fn activate(
        self,
        expected_version: FixtureProfileVersion,
        activation: FixtureProfileActivation,
    ) -> Result<Self, DomainError> {
        if self.version != expected_version {
            return Err(DomainError::UnsupportedFixtureProfileVersion);
        }
        if activation == FixtureProfileActivation::ProductPolicy {
            return Err(DomainError::FixtureProfileCannotActivateProduct);
        }
        Ok(self)
    }

    #[must_use]
    pub const fn version(self) -> FixtureProfileVersion {
        self.version
    }

    #[must_use]
    pub const fn stack_engine_ceiling(self) -> EngineCeiling {
        self.stack_engine_ceiling
    }

    #[must_use]
    pub const fn container_limits(self) -> ContainerLimits {
        self.container_limits
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquipPattern<S> {
    primary: S,
    claims: BTreeSet<S>,
}

impl<S> EquipPattern<S>
where
    S: Ord + Clone,
{
    pub fn new(primary: S, claims: impl IntoIterator<Item = S>) -> Result<Self, DomainError> {
        let claims: BTreeSet<S> = claims.into_iter().collect();
        if !claims.contains(&primary) {
            return Err(DomainError::MissingPrimarySlot);
        }
        Ok(Self { primary, claims })
    }

    #[must_use]
    pub const fn primary(&self) -> &S {
        &self.primary
    }

    #[must_use]
    pub fn claims(&self) -> impl Iterator<Item = &S> {
        self.claims.iter()
    }

    #[must_use]
    pub fn is_legal_against(&self, occupied: &BTreeSet<S>) -> bool {
        self.claims.is_disjoint(occupied)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContainerLimits {
    max_direct_entries: usize,
    max_nesting_depth: usize,
    max_reachable_items: usize,
}

impl ContainerLimits {
    pub fn new(
        max_direct_entries: usize,
        max_nesting_depth: usize,
        max_reachable_items: usize,
    ) -> Result<Self, DomainError> {
        if max_direct_entries == 0
            || max_nesting_depth == 0
            || max_reachable_items == 0
            || max_direct_entries > max_reachable_items
        {
            return Err(DomainError::InvalidLimit);
        }
        Ok(Self {
            max_direct_entries,
            max_nesting_depth,
            max_reachable_items,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainmentLegalityView {
    limits: ContainerLimits,
    parent_by_child: BTreeMap<ItemInstanceId, ItemInstanceId>,
}

impl ContainmentLegalityView {
    #[must_use]
    pub const fn new(limits: ContainerLimits) -> Self {
        Self {
            limits,
            parent_by_child: BTreeMap::new(),
        }
    }

    pub fn record_existing_edge(
        &mut self,
        child: ItemInstanceId,
        container: ItemInstanceId,
    ) -> Result<(), DomainError> {
        if self.parent_by_child.contains_key(&child) {
            return Err(DomainError::ContainerAlreadyAttached);
        }
        self.validate_attach(child, container)?;
        self.parent_by_child.insert(child, container);
        Ok(())
    }

    pub fn validate_attach(
        &self,
        child: ItemInstanceId,
        container: ItemInstanceId,
    ) -> Result<(), DomainError> {
        if self.parent_by_child.contains_key(&child) {
            return Err(DomainError::ContainerAlreadyAttached);
        }
        if child == container {
            return Err(DomainError::ContainerSelfCycle);
        }
        if self.ancestor_chain_contains(container, child) {
            return Err(DomainError::ContainerCycle);
        }
        let direct_entries = self
            .parent_by_child
            .values()
            .filter(|parent| **parent == container)
            .count();
        if direct_entries.saturating_add(1) > self.limits.max_direct_entries {
            return Err(DomainError::ContainerDirectLimit);
        }
        let deepest_new_depth = self
            .ancestor_count(container)
            .saturating_add(1)
            .saturating_add(self.subtree_max_relative_depth(child));
        if deepest_new_depth > self.limits.max_nesting_depth {
            return Err(DomainError::ContainerDepthLimit);
        }
        let added_subtree = self.subtree_size(child);
        let mut affected_container = Some(container);
        while let Some(current) = affected_container {
            let reachable_after = self
                .reachable_descendants(current)
                .saturating_add(added_subtree);
            if reachable_after > self.limits.max_reachable_items {
                return Err(DomainError::ContainerReachableLimit);
            }
            affected_container = self.parent_by_child.get(&current).copied();
        }
        Ok(())
    }

    fn ancestor_chain_contains(&self, start: ItemInstanceId, target: ItemInstanceId) -> bool {
        let mut current = Some(start);
        while let Some(item) = current {
            if item == target {
                return true;
            }
            current = self.parent_by_child.get(&item).copied();
        }
        false
    }

    fn ancestor_count(&self, start: ItemInstanceId) -> usize {
        let mut count = 0usize;
        let mut current = self.parent_by_child.get(&start).copied();
        while let Some(item) = current {
            count = count.saturating_add(1);
            current = self.parent_by_child.get(&item).copied();
        }
        count
    }

    fn is_descendant_of(&self, candidate: ItemInstanceId, root: ItemInstanceId) -> bool {
        let mut current = self.parent_by_child.get(&candidate).copied();
        while let Some(item) = current {
            if item == root {
                return true;
            }
            current = self.parent_by_child.get(&item).copied();
        }
        false
    }

    fn reachable_descendants(&self, root: ItemInstanceId) -> usize {
        self.parent_by_child
            .keys()
            .filter(|candidate| self.is_descendant_of(**candidate, root))
            .count()
    }

    fn subtree_size(&self, root: ItemInstanceId) -> usize {
        self.reachable_descendants(root).saturating_add(1)
    }

    fn subtree_max_relative_depth(&self, root: ItemInstanceId) -> usize {
        self.parent_by_child
            .keys()
            .filter_map(|candidate| self.descendant_depth_from(*candidate, root))
            .max()
            .unwrap_or(0)
    }

    fn descendant_depth_from(
        &self,
        candidate: ItemInstanceId,
        root: ItemInstanceId,
    ) -> Option<usize> {
        let mut depth = 0usize;
        let mut current = candidate;
        loop {
            if current == root {
                return Some(depth);
            }
            current = *self.parent_by_child.get(&current)?;
            depth = depth.saturating_add(1);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn uuid_v7(last: u8) -> [u8; 16] {
        [0x01, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, last]
    }

    #[test]
    fn strong_ids_require_non_nil_uuid_v7_rfc_variant() -> Result<(), DomainError> {
        let character = CharacterId::from_bytes(uuid_v7(1))?;
        let item = ItemInstanceId::from_bytes(uuid_v7(2))?;
        let world = WorldId::from_bytes(uuid_v7(3))?;
        assert_ne!(character.as_bytes(), item.as_bytes());
        assert_ne!(world.as_bytes(), &[0; 16]);
        assert_eq!(CharacterId::from_bytes([0; 16]), Err(DomainError::NilUuid));
        let mut wrong_version = uuid_v7(4);
        wrong_version[6] = 0x60;
        assert_eq!(
            CharacterId::from_bytes(wrong_version),
            Err(DomainError::UuidVersion)
        );
        let mut wrong_variant = uuid_v7(5);
        wrong_variant[8] = 0x40;
        assert_eq!(
            CharacterId::from_bytes(wrong_variant),
            Err(DomainError::UuidVariant)
        );
        Ok(())
    }

    #[test]
    fn character_lifecycle_is_revision_fenced_and_retirement_is_terminal() -> Result<(), DomainError>
    {
        let context = CharacterInterpretationContext::new(
            "fixture-profile",
            "rules-v1",
            "content-v1",
            "starter-v1",
        );
        let build = CharacterBuildState::<&str, &str>::Unselected;
        let mut character =
            CharacterRecord::new(CharacterId::from_bytes(uuid_v7(6))?, context, build);
        assert_eq!(character.lifecycle(), CharacterLifecycle::Active);
        let revision_one = CharacterRevision::new(1)?;
        let revision_two = character.schedule_deletion(revision_one)?;
        assert_eq!(revision_two, CharacterRevision::new(2)?);
        assert_eq!(character.lifecycle(), CharacterLifecycle::DeletionScheduled);
        assert_eq!(
            character.restore(revision_one),
            Err(DomainError::StaleCharacterRevision)
        );
        let revision_three = character.restore(revision_two)?;
        assert_eq!(character.lifecycle(), CharacterLifecycle::Active);
        let revision_four = character.schedule_deletion(revision_three)?;
        assert_eq!(
            character.retire(revision_four, Quiescence::NOT_QUIESCENT),
            Err(DomainError::NotQuiescent)
        );
        let revision_five = character.retire(revision_four, Quiescence::QUIESCENT)?;
        assert_eq!(revision_five, CharacterRevision::new(5)?);
        assert_eq!(character.lifecycle(), CharacterLifecycle::Retired);
        assert_eq!(
            character.restore(revision_five),
            Err(DomainError::TerminalLifecycle)
        );
        Ok(())
    }
    #[test]
    fn lifecycle_revision_exhaustion_does_not_partially_mutate_state() -> Result<(), DomainError> {
        let context = CharacterInterpretationContext::new(
            "fixture-profile",
            "rules-v1",
            "content-v1",
            "starter-v1",
        );
        let mut character = CharacterRecord::new(
            CharacterId::from_bytes(uuid_v7(40))?,
            context,
            CharacterBuildState::<&str, &str>::Unselected,
        );
        character.revision = CharacterRevision::new(u64::MAX)?;
        assert_eq!(
            character.schedule_deletion(character.revision()),
            Err(DomainError::RevisionExhausted)
        );
        assert_eq!(character.lifecycle(), CharacterLifecycle::Active);
        assert_eq!(character.revision().get(), u64::MAX);
        Ok(())
    }

    #[test]
    fn item_definition_context_and_stack_limits_are_explicit() -> Result<(), DomainError> {
        let engine = EngineCeiling::new(100)?;
        let definition_limit = DefinitionLimit::new(80, engine)?;
        let stack = StackState::new(80, definition_limit)?;
        assert_eq!(stack.quantity(), 80);
        assert_eq!(
            StackState::new(81, definition_limit),
            Err(DomainError::ExceedsLimit)
        );
        assert_eq!(
            DefinitionLimit::new(101, engine),
            Err(DomainError::ExceedsEngineCeiling)
        );

        let definition = ItemDefinitionRef::new("fixture:item.sword", "content-v1");
        let instance = ItemInstance::new(
            ItemInstanceId::from_bytes(uuid_v7(7))?,
            WorldId::from_bytes(uuid_v7(8))?,
            definition,
        );
        assert_eq!(instance.definition().key(), &"fixture:item.sword");
        assert_eq!(instance.definition().revision(), &"content-v1");
        Ok(())
    }

    #[test]
    fn equipment_pattern_claims_all_slots_atomically() -> Result<(), DomainError> {
        let pattern = EquipPattern::new("right-hand", ["right-hand", "left-hand"])?;
        let empty = BTreeSet::new();
        assert!(pattern.is_legal_against(&empty));
        let mut occupied = BTreeSet::new();
        occupied.insert("left-hand");
        assert!(!pattern.is_legal_against(&occupied));
        assert_eq!(
            EquipPattern::new("right-hand", ["left-hand"]),
            Err(DomainError::MissingPrimarySlot)
        );
        Ok(())
    }

    #[test]
    fn containment_legality_rejects_self_cycles_cycles_and_bounds() -> Result<(), DomainError> {
        let a = ItemInstanceId::from_bytes(uuid_v7(10))?;
        let b = ItemInstanceId::from_bytes(uuid_v7(11))?;
        let c = ItemInstanceId::from_bytes(uuid_v7(12))?;
        let d = ItemInstanceId::from_bytes(uuid_v7(13))?;
        let limits = ContainerLimits::new(2, 2, 3)?;
        let mut view = ContainmentLegalityView::new(limits);
        assert_eq!(
            view.validate_attach(a, a),
            Err(DomainError::ContainerSelfCycle)
        );
        view.record_existing_edge(a, b)?;
        view.record_existing_edge(c, a)?;
        assert_eq!(view.validate_attach(b, c), Err(DomainError::ContainerCycle));
        assert_eq!(
            view.validate_attach(d, c),
            Err(DomainError::ContainerDepthLimit)
        );
        view.record_existing_edge(d, b)?;
        let e = ItemInstanceId::from_bytes(uuid_v7(14))?;
        assert_eq!(
            view.validate_attach(e, b),
            Err(DomainError::ContainerDirectLimit)
        );
        assert_eq!(
            view.record_existing_edge(a, d),
            Err(DomainError::ContainerAlreadyAttached)
        );
        Ok(())
    }

    #[test]
    fn character_build_progression_and_context_are_explicit_and_revision_bound()
    -> Result<(), DomainError> {
        let context = CharacterInterpretationContext::new(
            "profile-v1",
            "rules-v1",
            "content-v1",
            "starter-v1",
        );
        let profession = CharacterDefinitionRef::new("fixture:profession.knight", "rules-v1");
        let build = CharacterBuildState::Selected {
            profession: profession.clone(),
            promotion: None,
        };
        let character = CharacterRecord::new(
            CharacterId::from_bytes(uuid_v7(20))?,
            context.clone(),
            build,
        );
        assert_eq!(
            character.build(),
            &CharacterBuildState::Selected {
                profession,
                promotion: None
            }
        );
        character.ensure_context(&context)?;
        let stale = CharacterInterpretationContext::new(
            "profile-v1",
            "rules-v0",
            "content-v1",
            "starter-v1",
        );
        assert_eq!(
            character.ensure_context(&stale),
            Err(DomainError::UnsupportedInterpretationContext)
        );

        let skill = CharacterDefinitionRef::new("fixture:skill.sword", "rules-v1");
        let fact = CharacterProgressionFact::new(skill, 42_u64);
        assert_eq!(fact.definition().key(), &"fixture:skill.sword");
        assert_eq!(fact.definition().revision(), &"rules-v1");
        assert_eq!(fact.value(), &42);
        Ok(())
    }

    #[test]
    fn item_locations_are_typed_definition_legal_and_world_scoped() -> Result<(), DomainError> {
        let world = WorldId::from_bytes(uuid_v7(21))?;
        let other_world = WorldId::from_bytes(uuid_v7(22))?;
        let definition = ItemDefinitionRef::new("fixture:item.sword", "content-v1");
        definition.ensure_revision(&"content-v1")?;
        assert_eq!(
            definition.ensure_revision(&"content-v0"),
            Err(DomainError::UnsupportedDefinitionRevision)
        );
        let instance =
            ItemInstance::new(ItemInstanceId::from_bytes(uuid_v7(23))?, world, definition);
        let policy = ItemPlacementPolicy::new(true, true, false, true);
        let inventory =
            ItemLocationRef::<u16, (), u16, &str, (i32, i32, i16)>::CharacterInventory {
                character_id: CharacterId::from_bytes(uuid_v7(24))?,
                position: 0,
            };
        instance.validate_location(policy, &inventory)?;
        let container = ItemLocationRef::<u16, (), u16, &str, (i32, i32, i16)>::Container {
            parent_item_instance_id: ItemInstanceId::from_bytes(uuid_v7(25))?,
            entry: 0,
        };
        assert_eq!(
            instance.validate_location(policy, &container),
            Err(DomainError::LocationSurfaceNotAllowed)
        );
        let wrong_ground = ItemLocationRef::<u16, (), u16, &str, (i32, i32, i16)>::Ground {
            world_id: other_world,
            runtime_scope: "channel-scope",
            spatial_position: (100, 100, 7),
        };
        assert_eq!(
            instance.validate_location(policy, &wrong_ground),
            Err(DomainError::MismatchedWorld)
        );
        Ok(())
    }

    #[test]
    fn structural_fixture_profile_is_versioned_and_cannot_become_product_policy()
    -> Result<(), DomainError> {
        let version_one = FixtureProfileVersion::new(1)?;
        let fixture = StructuralFixtureProfile::new(
            version_one,
            EngineCeiling::new(100)?,
            ContainerLimits::new(4, 3, 8)?,
        );
        let active = fixture.activate(version_one, FixtureProfileActivation::StructuralTest)?;
        assert_eq!(active.version().get(), 1);
        assert_eq!(active.stack_engine_ceiling().get(), 100);
        assert_eq!(
            fixture.activate(
                FixtureProfileVersion::new(2)?,
                FixtureProfileActivation::StructuralTest
            ),
            Err(DomainError::UnsupportedFixtureProfileVersion)
        );
        assert_eq!(
            fixture.activate(version_one, FixtureProfileActivation::ProductPolicy),
            Err(DomainError::FixtureProfileCannotActivateProduct)
        );
        assert_eq!(
            FixtureProfileVersion::new(0),
            Err(DomainError::InvalidFixtureProfileVersion)
        );
        Ok(())
    }

    #[test]
    fn containment_checks_detached_subtree_depth_and_ancestor_reachable_limits()
    -> Result<(), DomainError> {
        let root = ItemInstanceId::from_bytes(uuid_v7(30))?;
        let child = ItemInstanceId::from_bytes(uuid_v7(31))?;
        let grandchild = ItemInstanceId::from_bytes(uuid_v7(32))?;
        let destination = ItemInstanceId::from_bytes(uuid_v7(33))?;
        let limits = ContainerLimits::new(4, 2, 4)?;
        let mut view = ContainmentLegalityView::new(limits);
        view.record_existing_edge(grandchild, child)?;
        view.record_existing_edge(child, root)?;
        assert_eq!(
            view.validate_attach(root, destination),
            Err(DomainError::ContainerDepthLimit)
        );

        let ancestor = ItemInstanceId::from_bytes(uuid_v7(34))?;
        let destination = ItemInstanceId::from_bytes(uuid_v7(35))?;
        let existing = ItemInstanceId::from_bytes(uuid_v7(36))?;
        let subtree = ItemInstanceId::from_bytes(uuid_v7(37))?;
        let leaf = ItemInstanceId::from_bytes(uuid_v7(38))?;
        let limits = ContainerLimits::new(3, 4, 3)?;
        let mut view = ContainmentLegalityView::new(limits);
        view.record_existing_edge(destination, ancestor)?;
        view.record_existing_edge(existing, ancestor)?;
        view.record_existing_edge(leaf, subtree)?;
        assert_eq!(
            view.validate_attach(subtree, destination),
            Err(DomainError::ContainerReachableLimit)
        );
        Ok(())
    }
}
