//! D1373 House arm of the existing fixed-slot actor carrier.
//!
//! This file is mounted once, as a private child of `runtime_actor_carrier`.
//! It adds no parallel actor map. Every reservation, materialized player and
//! unresolved commit occupies that carrier's existing `slots` and `occupied`
//! index. Native SQL ownership stays in the `house_scope` consumer.

use super::super::FreshAdmissionReplayKey;
use super::{
    ActorLocalGeneration, ActorLocalId, ActorState, CarrierError, CarrierScopeDomain,
    ChannelActorCarrier, ChannelRuntimeV1, CharacterId, CharacterLease, ExactActorRef,
    GameSessionId, NamespaceContinuityGuard, NodeId, ScopeOwnershipGeneration, Slot, WorldId,
    WriteFenceToken,
};
use sha2::{Digest, Sha256};

// One physical definition, moved without changing its accepted encoding from
// durability/house_scope_handoff.rs. That module re-exports this same type and
// retains its fallible Durability-facing runtime_scope() convenience method.
const HOUSE_KEY_PREFIX: &str = "oteryn:content.house.";
pub const HOUSE_KEY_MAX_BYTES: usize = 512;
const HOUSE_INSTANCE_DOMAIN: &[u8] = b"oteryn:house-scope-instance:v1";
pub const HOUSE_TILE_MAX_BYTES: usize = 128;
pub const HOUSE_SCOPE_CHARACTERS_MAX: u64 = 200;

/// Exact native identity of one House, independent of its origin Channel.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HouseId {
    world_id: WorldId,
    house_key: String,
}

impl HouseId {
    #[must_use]
    pub fn new(world_id: WorldId, house_key: &str) -> Option<Self> {
        let suffix = house_key.strip_prefix(HOUSE_KEY_PREFIX)?;
        let valid = house_key.len() <= HOUSE_KEY_MAX_BYTES
            && !suffix.is_empty()
            && suffix
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
        valid.then(|| Self {
            world_id,
            house_key: house_key.to_owned(),
        })
    }

    #[must_use]
    pub const fn world_id(&self) -> WorldId {
        self.world_id
    }

    #[must_use]
    pub fn house_key(&self) -> &str {
        &self.house_key
    }

    #[must_use]
    pub fn scope_key(&self) -> Vec<u8> {
        let mut key = Vec::with_capacity(17 + self.house_key.len());
        key.push(0x02);
        key.extend_from_slice(self.world_id.as_bytes());
        key.extend_from_slice(self.house_key.as_bytes());
        key
    }

    #[must_use]
    pub fn instance_id(&self) -> [u8; 16] {
        let digest = Sha256::new()
            .chain_update(HOUSE_INSTANCE_DOMAIN)
            .chain_update(self.world_id.as_bytes())
            .chain_update(self.house_key.as_bytes())
            .finalize();
        let mut id = [0_u8; 16];
        id.copy_from_slice(&digest[..16]);
        id[6] = (id[6] & 0x0f) | 0x70;
        id[8] = (id[8] & 0x3f) | 0x80;
        id
    }
}

/// Only the allocated native consumer implements this seal in production.
/// Its concrete authority value is private and never leaves its caller TX.
pub(crate) mod house_actor_authority_seal {
    pub trait Sealed {}
}

/// Independently locked native facts for this physical preparation. Public
/// House/assignment/receipt DTOs do not implement this trait. The native owner
/// reads the actual PREPARED row, node possession, assignment, current source
/// and original receipt key before constructing itself. This binding is not
/// active Content or admission authority: the House consumer independently
/// qualifies the real tile/0047 identity before materializing, and the stage
/// stays unbound until the actual player-state and ingress owners participate.
pub(crate) trait HouseActorAuthorityV1: house_actor_authority_seal::Sealed {
    fn house(&self) -> &HouseId;
    fn scope_generation(&self) -> ScopeOwnershipGeneration;
    fn node_id(&self) -> NodeId;
    fn node_registration_revision(&self) -> u64;
    fn character_id(&self) -> CharacterId;
    fn source_character_lease(&self) -> CharacterLease;
    fn source_session(&self) -> GameSessionId;
    fn destination_session(&self) -> GameSessionId;
    fn handoff_id(&self) -> [u8; 16];
    fn initial_receipt_key(&self) -> FreshAdmissionReplayKey;
    fn reserved_tile(&self) -> &[u8];
    fn transaction_identity(&self) -> &str;
}

/// Disposition independently re-read by the native reconciliation owner under
/// the same locks. A historical receipt alone does not implement the owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HouseActorSettledState {
    Prepared,
    Released,
}
pub(crate) trait HouseActorReconcileAuthorityV1: HouseActorAuthorityV1 {
    fn settled_state(&self) -> HouseActorSettledState;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HousePlacementError {
    InvalidNativeBinding,
    NamespaceRetired,
    NamespaceChanged,
    NamespaceCapacity,
    TileOccupied,
    HandoffConflict,
    ReservationMismatch,
    AlreadyMaterialized,
    NotMaterialized,
    NotConsumed,
    CommitAlreadyStarted,
    CommitOutcomeUnresolved,
    RestartMaterializationRequired,
    Carrier(CarrierError),
}
impl From<CarrierError> for HousePlacementError {
    fn from(error: CarrierError) -> Self {
        Self::Carrier(error)
    }
}

type Result<T> = std::result::Result<T, HousePlacementError>;

/// Full logical operation key, excluding the transient physical SQL TX id.
/// Retry preserves the native initial receipt key rather than minting a key.
#[derive(Debug, Clone, PartialEq, Eq)]
struct HouseActorReservationKey {
    house: HouseId,
    scope_generation: ScopeOwnershipGeneration,
    character_id: CharacterId,
    source_character_lease: CharacterLease,
    source_session: GameSessionId,
    destination_session: GameSessionId,
    handoff_id: [u8; 16],
    initial_receipt_key: FreshAdmissionReplayKey,
    reserved_tile: Box<[u8]>,
}

impl HouseActorReservationKey {
    fn read(authority: &impl HouseActorAuthorityV1) -> Result<Self> {
        let handoff_id = authority.handoff_id();
        if handoff_id[6] >> 4 != 7
            || handoff_id[8] >> 6 != 2
            || authority.source_session() == authority.destination_session()
            || authority.reserved_tile().is_empty()
            || authority.reserved_tile().len() > HOUSE_TILE_MAX_BYTES
            || authority.node_registration_revision() == 0
            || authority.source_character_lease().character_id() != authority.character_id()
            || authority.transaction_identity().is_empty()
            || authority.transaction_identity().len() > 32
            || !authority
                .transaction_identity()
                .bytes()
                .all(|byte| byte.is_ascii_digit())
        {
            return Err(HousePlacementError::InvalidNativeBinding);
        }
        Ok(Self {
            house: authority.house().clone(),
            scope_generation: authority.scope_generation(),
            character_id: authority.character_id(),
            source_character_lease: authority.source_character_lease(),
            source_session: authority.source_session(),
            destination_session: authority.destination_session(),
            handoff_id,
            initial_receipt_key: authority.initial_receipt_key(),
            reserved_tile: authority.reserved_tile().into(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HousePlayerPhase {
    Reserved,
    Materialized,
    Consumed,
    CommitInFlight,
    Durable,
}

/// Payload of `Slot::HousePlayer`; boxed to retain the common slot footprint.
/// The actual entry position exists from reservation through every later phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HousePlayerSlot {
    key: HouseActorReservationKey,
    transaction_identity: String,
    phase: HousePlayerPhase,
    materialized: Option<HousePlayerMaterialized>,
}

/// Actual source-slot state, copied under its exclusive fenced owner borrow.
/// No synthetic default player state is used to materialize a House actor.
#[derive(Debug, Clone, PartialEq, Eq)]
struct HousePlayerMaterialized {
    source: HouseSourceBinding,
    actor: ActorState,
    // Preserve the exact previous position/context/revision as source history.
    // The House destination is the reservation key's native tile, never a
    // Channel VersionedPosition relabelled as a House position.
    source_position: super::VersionedPosition,
    lifecycle: Box<super::PlayerRuntimeState>,
    spell_combat: Box<super::runtime_actor_spell::ActorCombatState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HouseSourceBinding {
    actor: ExactActorRef,
    session: GameSessionId,
    lease: CharacterLease,
    fence: WriteFenceToken,
}

/// The real Channel owner remains exclusively borrowed during the House
/// transaction. Its existing write fence additionally survives an unknown
/// COMMIT result; only native reconciliation may subsequently lift it.
pub(crate) struct HouseSourceActorHold<'a> {
    carrier: &'a mut ChannelActorCarrier,
    continuity: &'a NamespaceContinuityGuard,
    actor: ExactActorRef,
    session: GameSessionId,
    lease: CharacterLease,
    fence: WriteFenceToken,
    retain_on_drop: bool,
}

impl ChannelRuntimeV1 {
    pub(crate) fn hold_house_source_actor(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Result<HouseSourceActorHold<'_>> {
        HouseSourceActorHold::acquire(&mut self.carrier, &self.continuity, actor, session)
    }
}

impl<'a> HouseSourceActorHold<'a> {
    fn acquire(
        carrier: &'a mut ChannelActorCarrier,
        continuity: &'a NamespaceContinuityGuard,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Result<Self> {
        let index = carrier.player_slot_index(continuity, actor.0, session)?;
        carrier.assert_slot_spell_unreserved(index)?;
        let authority = carrier
            .attacker_authority(index, actor.0.actor_local_generation.0)
            .ok_or(HousePlacementError::InvalidNativeBinding)?;
        let lease = authority
            .lease
            .ok_or(HousePlacementError::InvalidNativeBinding)?;
        if authority.fence.is_some() {
            return Err(CarrierError::WriteFenceBusy.into());
        }
        let ordinal = carrier
            .fence_transitions
            .checked_add(1)
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        let fence = WriteFenceToken::Transition(ordinal);
        // The authority entry is already present above; this publication does
        // not add another authority source or an actor map.
        carrier.fence_player_writes(continuity, actor.0, session, fence)?;
        carrier.fence_transitions = ordinal;
        Ok(Self {
            carrier,
            continuity,
            actor,
            session,
            lease,
            fence,
            retain_on_drop: false,
        })
    }

    fn materialization(&self, key: &HouseActorReservationKey) -> Result<HousePlayerMaterialized> {
        if key.source_session != self.session
            || key.source_character_lease != self.lease
            || key.character_id != self.lease.character_id()
            || key.house.world_id() != self.actor.world_id()
        {
            return Err(HousePlacementError::InvalidNativeBinding);
        }
        let index = self
            .carrier
            .player_slot_index(self.continuity, self.actor.0, self.session)?;
        self.carrier.assert_slot_spell_unreserved(index)?;
        let actual = self
            .carrier
            .attacker_authority(index, self.actor.0.actor_local_generation.0)
            .ok_or(HousePlacementError::InvalidNativeBinding)?;
        if actual.lease != Some(self.lease) || actual.fence != Some(self.fence) {
            return Err(HousePlacementError::InvalidNativeBinding);
        }
        match &self.carrier.slots[index] {
            Slot::Occupied {
                actor,
                position: Some(position),
                lifecycle,
                spell_combat,
                committed: true,
                ..
            } => Ok(HousePlayerMaterialized {
                source: HouseSourceBinding {
                    actor: self.actor,
                    session: self.session,
                    lease: self.lease,
                    fence: self.fence,
                },
                actor: *actor,
                source_position: *position,
                lifecycle: lifecycle.clone(),
                spell_combat: spell_combat.clone(),
            }),
            _ => Err(HousePlacementError::InvalidNativeBinding),
        }
    }

    pub(crate) fn begin_commit(&mut self) {
        self.retain_on_drop = true;
    }

    /// Called by the owning consumer only after the actual native COMMIT. The
    /// source player is removed from its original physical carrier exactly once.
    pub(crate) fn committed(mut self) -> Result<()> {
        if !self.retain_on_drop {
            return Err(HousePlacementError::NotConsumed);
        }
        self.carrier
            .remove_terminal_player(self.continuity, self.session, self.actor)?;
        self.retain_on_drop = true;
        Ok(())
    }
}
impl Drop for HouseSourceActorHold<'_> {
    fn drop(&mut self) {
        if !self.retain_on_drop {
            // Exact-token lift is denial-safe if the namespace was retired.
            let _ = self.carrier.lift_player_fence(
                self.continuity,
                self.actor.0,
                self.session,
                self.fence,
            );
        }
    }
}

/// Value handle to one actual House player slot. It grants neither transport
/// control nor current admission authority and cannot be decoded from a client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HousePlacedActor {
    house: HouseId,
    scope_generation: ScopeOwnershipGeneration,
    actor_local_id: ActorLocalId,
    actor_local_generation: ActorLocalGeneration,
    character_id: CharacterId,
    game_session_id: GameSessionId,
}
impl HousePlacedActor {
    pub(crate) fn house(&self) -> &HouseId {
        &self.house
    }
    pub(crate) const fn scope_generation(&self) -> ScopeOwnershipGeneration {
        self.scope_generation
    }
    pub(crate) const fn game_session_id(&self) -> GameSessionId {
        self.game_session_id
    }
    pub(crate) fn placement_identity(&self) -> [u8; 16] {
        let digest = Sha256::new()
            .chain_update(b"oteryn:runtime-house-player-placement:v1")
            .chain_update(self.house.scope_key())
            .chain_update(self.scope_generation.get().to_be_bytes())
            .chain_update(self.actor_local_id.0.to_be_bytes())
            .chain_update(self.actor_local_generation.0.to_be_bytes())
            .finalize();
        let mut result = [0_u8; 16];
        result.copy_from_slice(&digest[..16]);
        result
    }
}

/// Continuity outlives an individual carrier. A retired namespace cannot be
/// re-created at the same generation, even if its carrier has been removed.
struct HouseActorNamespace {
    house: HouseId,
    scope_generation: ScopeOwnershipGeneration,
    node_id: NodeId,
    node_registration_revision: u64,
    active: bool,
    generation_claimed: bool,
    carrier: Option<ChannelActorCarrier>,
}

/// One node-owned, explicitly bounded directory. Entries own the existing
/// carrier, not another actor truth. The caller-supplied bound is a qualification
/// parameter; D1373 does not select or activate a production RL04 value.
pub(crate) struct HouseActorDirectory {
    maximum_namespaces: usize,
    namespaces: Vec<HouseActorNamespace>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HouseActorReconcileOutcome {
    RolledBack,
    Durable(HousePlacedActor),
}

impl HouseActorDirectory {
    pub(crate) fn with_explicit_bound(maximum_namespaces: usize) -> Result<Self> {
        if maximum_namespaces == 0 {
            return Err(HousePlacementError::NamespaceCapacity);
        }
        maximum_namespaces
            .checked_mul(std::mem::size_of::<HouseActorNamespace>())
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        let mut namespaces = Vec::new();
        namespaces
            .try_reserve_exact(maximum_namespaces)
            .map_err(|_| CarrierError::AllocationFailed)?;
        Ok(Self {
            maximum_namespaces,
            namespaces,
        })
    }

    fn current_namespace(&mut self, authority: &impl HouseActorAuthorityV1) -> Result<usize> {
        let found = self
            .namespaces
            .iter()
            .position(|entry| &entry.house == authority.house());
        if let Some(index) = found {
            let entry = &mut self.namespaces[index];
            if entry.scope_generation == authority.scope_generation() {
                if !entry.active || !entry.generation_claimed || entry.carrier.is_none() {
                    return Err(HousePlacementError::NamespaceRetired);
                }
                if entry.node_id != authority.node_id()
                    || entry.node_registration_revision != authority.node_registration_revision()
                {
                    return Err(HousePlacementError::NamespaceChanged);
                }
                return Ok(index);
            }
            if authority.scope_generation().get() <= entry.scope_generation.get()
                || entry.active
                || entry
                    .carrier
                    .as_ref()
                    .is_some_and(|carrier| !carrier.occupied.is_empty())
            {
                return Err(HousePlacementError::NamespaceChanged);
            }
            let next = Self::construct_namespace(authority)?;
            self.namespaces[index] = next;
            return Ok(index);
        }
        if self.namespaces.len() == self.maximum_namespaces {
            return Err(HousePlacementError::NamespaceCapacity);
        }
        let entry = Self::construct_namespace(authority)?;
        let index = self.namespaces.len();
        self.namespaces.push(entry);
        Ok(index)
    }

    fn construct_namespace(authority: &impl HouseActorAuthorityV1) -> Result<HouseActorNamespace> {
        let carrier = ChannelActorCarrier::allocate_for_scope(
            authority.house().world_id(),
            CarrierScopeDomain::House(authority.house().clone()),
            authority.scope_generation(),
            usize::try_from(HOUSE_SCOPE_CHARACTERS_MAX)
                .map_err(|_| CarrierError::CapacityArithmeticOverflow)?,
        )?;
        Ok(HouseActorNamespace {
            house: authority.house().clone(),
            scope_generation: authority.scope_generation(),
            node_id: authority.node_id(),
            node_registration_revision: authority.node_registration_revision(),
            active: true,
            generation_claimed: true,
            carrier: Some(carrier),
        })
    }

    /// Reconciliation may drain a locally retired namespace, but can neither
    /// create one nor reopen/advance it. The full retained node binding still
    /// has to match the independently read native disposition.
    fn reconciliation_namespace(&self, authority: &impl HouseActorAuthorityV1) -> Result<usize> {
        let index = self
            .namespaces
            .iter()
            .position(|entry| &entry.house == authority.house())
            .ok_or(HousePlacementError::RestartMaterializationRequired)?;
        let entry = &self.namespaces[index];
        if entry.scope_generation != authority.scope_generation()
            || entry.node_id != authority.node_id()
            || entry.node_registration_revision != authority.node_registration_revision()
            || !entry.generation_claimed
            || entry.carrier.is_none()
        {
            return Err(HousePlacementError::NamespaceChanged);
        }
        Ok(index)
    }

    /// Native PREPARED read has already locked and settled any earlier SQL TX.
    /// No caller-supplied assignment DTO, success flag or historical receipt can
    /// construct the authority used here.
    pub(crate) fn reserve_entry<'a>(
        &'a mut self,
        authority: &impl HouseActorAuthorityV1,
    ) -> Result<HouseEntryPlacement<'a>> {
        let key = HouseActorReservationKey::read(authority)?;
        let namespace = self.current_namespace(authority)?;
        let entry = &mut self.namespaces[namespace];
        let carrier = entry
            .carrier
            .as_mut()
            .ok_or(HousePlacementError::NamespaceRetired)?;
        // A retained uncertain predecessor is never overwritten or freed from
        // an expected PREPARED request. The native consumer must reconcile it.
        for slot in carrier.slots.iter() {
            let Slot::HousePlayer { player, .. } = slot else {
                continue;
            };
            if player.key.handoff_id == key.handoff_id
                || player.key.destination_session == key.destination_session
                || player.key.character_id == key.character_id
            {
                return Err(if player.key == key {
                    HousePlacementError::CommitOutcomeUnresolved
                } else {
                    HousePlacementError::HandoffConflict
                });
            }
            if player.key.reserved_tile == key.reserved_tile {
                return Err(HousePlacementError::TileOccupied);
            }
        }
        let head = carrier.free_head.ok_or(CarrierError::CapacityExceeded)?;
        let index = usize::try_from(head).map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
        let Some(Slot::VacantReusable {
            generation,
            next_free,
        }) = carrier.slots.get(index)
        else {
            return Err(HousePlacementError::ReservationMismatch);
        };
        let prior_generation = *generation;
        let next_free = *next_free;
        let Some(generation) = prior_generation.checked_add(1) else {
            // Preserve the existing carrier's protected #541 exception: the
            // exhausted slot leaves the free list even though this admission
            // refuses. A subsequent attempt can use the next physical slot.
            carrier.slots[index] = Slot::Exhausted {
                generation: prior_generation,
            };
            carrier.free_head = next_free;
            return Err(CarrierError::ActorGenerationExhausted.into());
        };
        let local_id = ActorLocalId(
            head.checked_add(1)
                .ok_or(CarrierError::CapacityArithmeticOverflow)?,
        );
        let payload = Box::new(HousePlayerSlot {
            key: key.clone(),
            transaction_identity: authority.transaction_identity().to_owned(),
            phase: HousePlayerPhase::Reserved,
            materialized: None,
        });
        // All fallible checks/allocations precede one physical publication.
        carrier.slots[index] = Slot::HousePlayer {
            generation,
            player: payload,
        };
        carrier.free_head = next_free;
        carrier.occupied.push(head);
        Ok(HouseEntryPlacement {
            directory: self,
            namespace,
            actor_local_id: local_id,
            actor_local_generation: ActorLocalGeneration(generation),
            key,
            retain_on_drop: false,
        })
    }

    /// Denial-only namespace retirement. Unresolved or durable slots remain
    /// retained until their real native owner reconciles them; no history is
    /// inferred from local retirement.
    pub(crate) fn retire(
        &mut self,
        house: &HouseId,
        generation: ScopeOwnershipGeneration,
    ) -> Result<()> {
        let entry = self
            .namespaces
            .iter_mut()
            .find(|entry| &entry.house == house)
            .ok_or(HousePlacementError::NamespaceChanged)?;
        if entry.scope_generation != generation {
            return Err(HousePlacementError::NamespaceChanged);
        }
        entry.active = false;
        Ok(())
    }

    /// Reconcile a retained physical reservation only from a fresh native
    /// owning read, keeping the inherited receipt key. No SQL effects occur.
    /// A process that lost the actual source/player payload must obtain the
    /// existing native Character reconstruction before it can materialize;
    /// this method never fabricates a default player from receipt history.
    pub(crate) fn reconcile_entry(
        &mut self,
        authority: &impl HouseActorReconcileAuthorityV1,
        source: &mut ChannelRuntimeV1,
    ) -> Result<HouseActorReconcileOutcome> {
        self.reconcile_entry_with_source(authority, &mut source.carrier, &source.continuity)
    }

    fn reconcile_entry_with_source(
        &mut self,
        authority: &impl HouseActorReconcileAuthorityV1,
        source: &mut ChannelActorCarrier,
        source_continuity: &NamespaceContinuityGuard,
    ) -> Result<HouseActorReconcileOutcome> {
        let key = HouseActorReservationKey::read(authority)?;
        let namespace = self.reconciliation_namespace(authority)?;
        let carrier = self.namespaces[namespace]
            .carrier
            .as_mut()
            .ok_or(HousePlacementError::NamespaceRetired)?;
        let found = carrier
            .slots
            .iter()
            .enumerate()
            .find_map(|(index, slot)| {
                let Slot::HousePlayer { generation, player } = slot else {
                    return None;
                };
                (player.key.handoff_id == key.handoff_id).then_some((index, *generation, player))
            })
            .ok_or(HousePlacementError::RestartMaterializationRequired)?;
        let (index, generation, player) = found;
        if player.key != key
            || !matches!(
                player.phase,
                HousePlayerPhase::CommitInFlight | HousePlayerPhase::Durable
            )
        {
            return Err(HousePlacementError::ReservationMismatch);
        }
        let materialized = player
            .materialized
            .as_ref()
            .ok_or(HousePlacementError::NotMaterialized)?;
        let binding = materialized.source;
        let local_id = ActorLocalId(
            u32::try_from(index)
                .map_err(|_| CarrierError::CapacityArithmeticOverflow)?
                .checked_add(1)
                .ok_or(CarrierError::CapacityArithmeticOverflow)?,
        );
        let local_generation = ActorLocalGeneration(generation);
        match authority.settled_state() {
            HouseActorSettledState::Prepared => {
                if player.phase != HousePlayerPhase::CommitInFlight {
                    return Err(HousePlacementError::InvalidNativeBinding);
                }
                let source_index = source.player_slot_index(
                    source_continuity,
                    binding.actor.0,
                    binding.session,
                )?;
                let actual = source
                    .attacker_authority(source_index, binding.actor.0.actor_local_generation.0)
                    .ok_or(HousePlacementError::InvalidNativeBinding)?;
                if actual.lease != Some(binding.lease) || actual.fence != Some(binding.fence) {
                    return Err(HousePlacementError::InvalidNativeBinding);
                }
                if !source.lift_player_fence(
                    source_continuity,
                    binding.actor.0,
                    binding.session,
                    binding.fence,
                )? {
                    return Err(HousePlacementError::InvalidNativeBinding);
                }
                self.remove_exact(namespace, local_id, local_generation, &key);
                Ok(HouseActorReconcileOutcome::RolledBack)
            }
            HouseActorSettledState::Released => {
                // An independently valid different Channel is not proof that
                // this retained source was already removed. Validate the exact
                // original outer namespace and physical slot bounds first;
                // validate_ref deliberately permits an actually recycled slot.
                source.validate_ref(source_continuity, binding.actor.0)?;
                if source.contains_committed_player(
                    source_continuity,
                    binding.session,
                    binding.actor,
                ) {
                    let source_index = source.player_slot_index(
                        source_continuity,
                        binding.actor.0,
                        binding.session,
                    )?;
                    let actual = source
                        .attacker_authority(source_index, binding.actor.0.actor_local_generation.0)
                        .ok_or(HousePlacementError::InvalidNativeBinding)?;
                    if actual.lease != Some(binding.lease) || actual.fence != Some(binding.fence) {
                        return Err(HousePlacementError::InvalidNativeBinding);
                    }
                    // The mutable source borrow ended on an unknown outcome.
                    // The existing attack fence alone is not a full future
                    // ingress/cadence freeze. Never discard a changed source
                    // payload in favour of this earlier materialized snapshot.
                    if !matches!(&source.slots[source_index], Slot::Occupied {
                        actor, position: Some(position), lifecycle, spell_combat,
                        committed: true, ..
                    } if *actor == materialized.actor
                        && *position == materialized.source_position
                        && lifecycle == &materialized.lifecycle
                        && spell_combat == &materialized.spell_combat)
                    {
                        return Err(HousePlacementError::InvalidNativeBinding);
                    }
                    source.remove_terminal_player(
                        source_continuity,
                        binding.session,
                        binding.actor,
                    )?;
                }
                let Slot::HousePlayer { player, .. } = &mut carrier.slots[index] else {
                    return Err(HousePlacementError::ReservationMismatch);
                };
                player.phase = HousePlayerPhase::Durable;
                Ok(HouseActorReconcileOutcome::Durable(HousePlacedActor {
                    house: key.house,
                    scope_generation: key.scope_generation,
                    actor_local_id: local_id,
                    actor_local_generation: local_generation,
                    character_id: key.character_id,
                    game_session_id: key.destination_session,
                }))
            }
        }
    }

    fn remove_exact(
        &mut self,
        namespace: usize,
        local_id: ActorLocalId,
        local_generation: ActorLocalGeneration,
        key: &HouseActorReservationKey,
    ) {
        let Some(entry) = self.namespaces.get_mut(namespace) else {
            return;
        };
        let Some(carrier) = entry.carrier.as_mut() else {
            return;
        };
        let Some(index) = local_id
            .0
            .checked_sub(1)
            .and_then(|value| usize::try_from(value).ok())
        else {
            return;
        };
        if !matches!(carrier.slots.get(index), Some(Slot::HousePlayer { generation, player })
            if *generation == local_generation.0 && &player.key == key)
        {
            return;
        }
        carrier.slots[index] = Slot::VacantReusable {
            generation: local_generation.0,
            next_free: carrier.free_head,
        };
        carrier.free_head = Some(local_id.0 - 1);
        carrier.unindex_occupied(local_id.0 - 1);
    }
}

/// Exclusive owner borrow spans reservation, materialization, consumption and
/// the native caller's result. Drop before COMMIT initiation rolls back the
/// same slot even on cancellation/panic; once COMMIT starts, drop retains it.
pub(crate) struct HouseEntryPlacement<'a> {
    directory: &'a mut HouseActorDirectory,
    namespace: usize,
    actor_local_id: ActorLocalId,
    actor_local_generation: ActorLocalGeneration,
    key: HouseActorReservationKey,
    retain_on_drop: bool,
}

impl HouseEntryPlacement<'_> {
    fn player(&mut self) -> Result<&mut HousePlayerSlot> {
        let entry = self
            .directory
            .namespaces
            .get_mut(self.namespace)
            .ok_or(HousePlacementError::ReservationMismatch)?;
        if !entry.active
            || entry.house != self.key.house
            || entry.scope_generation != self.key.scope_generation
        {
            return Err(HousePlacementError::NamespaceRetired);
        }
        let carrier = entry
            .carrier
            .as_mut()
            .ok_or(HousePlacementError::NamespaceRetired)?;
        if carrier.scope_domain != CarrierScopeDomain::House(self.key.house.clone())
            || carrier.scope_generation != self.key.scope_generation
        {
            return Err(HousePlacementError::NamespaceChanged);
        }
        let index = usize::try_from(self.actor_local_id.0 - 1)
            .map_err(|_| CarrierError::CapacityArithmeticOverflow)?;
        match carrier.slots.get_mut(index) {
            Some(Slot::HousePlayer { generation, player })
                if *generation == self.actor_local_generation.0 && player.key == self.key =>
            {
                Ok(player)
            }
            _ => Err(HousePlacementError::ReservationMismatch),
        }
    }

    /// Install the player in its already-reserved physical slot. No separate
    /// position initialization, guessed ChannelId or second actor is involved.
    pub(crate) fn materialize(&mut self, source: &HouseSourceActorHold<'_>) -> Result<()> {
        let materialized = source.materialization(&self.key)?;
        let player = self.player()?;
        if player.phase != HousePlayerPhase::Reserved || player.materialized.is_some() {
            return Err(HousePlacementError::AlreadyMaterialized);
        }
        player.materialized = Some(materialized);
        player.phase = HousePlayerPhase::Materialized;
        Ok(())
    }

    /// Consumption changes this same materialized slot. Its tile still blocks
    /// collision; consuming a reservation never makes an occupied tile free.
    pub(crate) fn consume_reservation(&mut self) -> Result<()> {
        let player = self.player()?;
        if player.phase != HousePlayerPhase::Materialized || player.materialized.is_none() {
            return Err(HousePlacementError::NotMaterialized);
        }
        player.phase = HousePlayerPhase::Consumed;
        Ok(())
    }

    /// The owning native claim writer checks this actual slot, exact initial
    /// receipt/handoff/session/tile and physical TX before any durable effect.
    pub(crate) fn verify_consumed(&mut self, authority: &impl HouseActorAuthorityV1) -> Result<()> {
        if HouseActorReservationKey::read(authority)? != self.key {
            return Err(HousePlacementError::ReservationMismatch);
        }
        let namespace = self
            .directory
            .namespaces
            .get(self.namespace)
            .ok_or(HousePlacementError::ReservationMismatch)?;
        if namespace.node_id != authority.node_id()
            || namespace.node_registration_revision != authority.node_registration_revision()
        {
            return Err(HousePlacementError::NamespaceChanged);
        }
        let player = self.player()?;
        if player.phase != HousePlayerPhase::Consumed
            || player.materialized.is_none()
            || player.transaction_identity != authority.transaction_identity()
        {
            return Err(HousePlacementError::NotConsumed);
        }
        Ok(())
    }

    /// Called immediately before the actual SQL COMMIT future is polled. Any
    /// subsequent cancel/error has an unknown outcome and retains this slot.
    pub(crate) fn begin_commit(&mut self) -> Result<()> {
        let player = self.player()?;
        if player.phase != HousePlayerPhase::Consumed {
            return Err(HousePlacementError::NotConsumed);
        }
        player.phase = HousePlayerPhase::CommitInFlight;
        self.retain_on_drop = true;
        Ok(())
    }

    /// Only the native consumer of an actual successful COMMIT calls this.
    /// The result is a physical placement handle, not a transport/ADMIT grant.
    pub(crate) fn committed(mut self) -> Result<HousePlacedActor> {
        let player = self.player()?;
        if player.phase != HousePlayerPhase::CommitInFlight {
            return Err(HousePlacementError::CommitAlreadyStarted);
        }
        player.phase = HousePlayerPhase::Durable;
        self.retain_on_drop = true;
        Ok(HousePlacedActor {
            house: self.key.house.clone(),
            scope_generation: self.key.scope_generation,
            actor_local_id: self.actor_local_id,
            actor_local_generation: self.actor_local_generation,
            character_id: self.key.character_id,
            game_session_id: self.key.destination_session,
        })
    }
}

impl Drop for HouseEntryPlacement<'_> {
    fn drop(&mut self) {
        if !self.retain_on_drop {
            self.directory.remove_exact(
                self.namespace,
                self.actor_local_id,
                self.actor_local_generation,
                &self.key,
            );
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod house_actor_placement_tests {
    use super::super::{LocalPosition, PreProductionContinuityGrant, PreProductionPositionContext};
    use super::*;

    // Pure carrier fixtures implement the private native seal only in tests.
    // These tests qualify arena behavior, never SQL ownership or admission.
    #[derive(Clone)]
    struct Authority {
        house: HouseId,
        generation: ScopeOwnershipGeneration,
        node: NodeId,
        node_revision: u64,
        lease: CharacterLease,
        source: GameSessionId,
        destination: GameSessionId,
        handoff: [u8; 16],
        initial: FreshAdmissionReplayKey,
        tile: Vec<u8>,
        txid: String,
        settled: HouseActorSettledState,
    }
    impl house_actor_authority_seal::Sealed for Authority {}
    impl HouseActorAuthorityV1 for Authority {
        fn house(&self) -> &HouseId {
            &self.house
        }
        fn scope_generation(&self) -> ScopeOwnershipGeneration {
            self.generation
        }
        fn node_id(&self) -> NodeId {
            self.node
        }
        fn node_registration_revision(&self) -> u64 {
            self.node_revision
        }
        fn character_id(&self) -> CharacterId {
            self.lease.character_id()
        }
        fn source_character_lease(&self) -> CharacterLease {
            self.lease
        }
        fn source_session(&self) -> GameSessionId {
            self.source
        }
        fn destination_session(&self) -> GameSessionId {
            self.destination
        }
        fn handoff_id(&self) -> [u8; 16] {
            self.handoff
        }
        fn initial_receipt_key(&self) -> FreshAdmissionReplayKey {
            self.initial
        }
        fn reserved_tile(&self) -> &[u8] {
            &self.tile
        }
        fn transaction_identity(&self) -> &str {
            &self.txid
        }
    }
    impl HouseActorReconcileAuthorityV1 for Authority {
        fn settled_state(&self) -> HouseActorSettledState {
            self.settled
        }
    }
    fn uuid(seed: u8) -> [u8; 16] {
        let mut id = [seed; 16];
        id[6] = 0x70;
        id[8] = (id[8] & 0x3f) | 0x80;
        id
    }
    fn key(seed: u8) -> FreshAdmissionReplayKey {
        let mut key = [seed; 33];
        key[0] = 1;
        FreshAdmissionReplayKey::decode(&key).expect("fixture initial receipt key")
    }
    fn authority(seed: u8) -> Authority {
        let world = WorldId::decode(&uuid(1)).expect("world");
        let character = CharacterId::decode(&uuid(seed)).expect("character");
        Authority {
            house: HouseId::new(world, "oteryn:content.house.native_a").expect("house"),
            generation: ScopeOwnershipGeneration::new(1).expect("generation"),
            node: NodeId::decode(&uuid(3)).expect("node"),
            node_revision: 7,
            lease: CharacterLease::new(character, 5).expect("lease"),
            source: GameSessionId::decode(&uuid(seed + 1)).expect("source"),
            destination: GameSessionId::decode(&uuid(seed + 2)).expect("destination"),
            handoff: uuid(seed + 3),
            initial: key(seed),
            tile: vec![0, 0, 0, 4, 0, 0, 0, 7, 0, 2],
            txid: "1024".into(),
            settled: HouseActorSettledState::Prepared,
        }
    }
    fn source(
        authority: &Authority,
    ) -> (NamespaceContinuityGuard, ChannelActorCarrier, ExactActorRef) {
        let channel = super::super::ChannelId::decode(&uuid(2)).expect("channel");
        let mut continuity =
            NamespaceContinuityGuard::from_pre_production_grant(PreProductionContinuityGrant {
                world_id: authority.house.world_id(),
                channel_id: channel,
                scope_generation: ScopeOwnershipGeneration::new(5).expect("source generation"),
            });
        let mut carrier = ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 2)
            .expect("source arena");
        let actor = carrier
            .admit_bound_test_attacker(&continuity, authority.source, authority.lease)
            .expect("real player slot");
        carrier
            .initialize_position(
                &continuity,
                actor.0,
                PreProductionPositionContext {
                    world_id: authority.house.world_id(),
                    channel_id: channel,
                    scope_generation: continuity.current_generation,
                    coordinate_frame_marker: 11,
                    map_revision_marker: 12,
                    content_generation_marker: 13,
                },
                LocalPosition {
                    x: -7,
                    y: 10,
                    floor: -2,
                },
            )
            .expect("source position");
        if let Slot::Occupied {
            actor,
            spell_combat,
            ..
        } = &mut carrier.slots[0]
        {
            *actor = ActorState(91);
            spell_combat.maximum_health = 135;
            spell_combat.paralysed = true;
        }
        (continuity, carrier, actor)
    }
    fn only_player(directory: &HouseActorDirectory) -> &HousePlayerSlot {
        let carrier = directory.namespaces[0]
            .carrier
            .as_ref()
            .expect("House carrier");
        assert_eq!(carrier.occupied.len(), 1);
        let Slot::HousePlayer { player, .. } = &carrier.slots[carrier.occupied[0] as usize] else {
            panic!("one physical House player slot");
        };
        player
    }
    fn unresolved(
        authority: &Authority,
        directory: &mut HouseActorDirectory,
        continuity: &NamespaceContinuityGuard,
        carrier: &mut ChannelActorCarrier,
        actor: ExactActorRef,
    ) {
        let mut source =
            HouseSourceActorHold::acquire(carrier, continuity, actor, authority.source)
                .expect("source hold");
        let mut placement = directory.reserve_entry(authority).expect("reserve");
        placement
            .materialize(&source)
            .expect("actual source materialization");
        placement
            .consume_reservation()
            .expect("consume same reservation");
        placement
            .verify_consumed(authority)
            .expect("same native binding");
        placement.begin_commit().expect("commit initiated");
        source.begin_commit();
        // Simulates only cancellation after COMMIT initiation. No database ran.
    }

    #[test]
    fn house_actor_placement_preserves_native_identity_domain() {
        let a = authority(10);
        let mut expected = vec![2];
        expected.extend_from_slice(a.house.world_id().as_bytes());
        expected.extend_from_slice(a.house.house_key().as_bytes());
        assert_eq!(a.house.scope_key(), expected);
        let second = HouseId::new(a.house.world_id(), "oteryn:content.house.native_b")
            .expect("second house");
        assert_ne!(a.house.scope_key(), second.scope_key());
        assert_ne!(a.house.instance_id(), second.instance_id());
        assert_eq!(a.house.instance_id()[6] >> 4, 7);
        assert_eq!(a.house.instance_id()[8] >> 6, 2);
        assert!(HouseId::new(a.house.world_id(), "oteryn:content.house.").is_none());
        assert!(HouseId::new(a.house.world_id(), "oteryn:content.house.Bad").is_none());
    }

    #[test]
    fn house_actor_placement_reserves_one_real_slot_and_blocks_tile_before_consume() {
        let a = authority(10);
        let mut directory = HouseActorDirectory::with_explicit_bound(1).expect("directory");
        let mut first = directory.reserve_entry(&a).expect("reserve first");
        assert_eq!(
            only_player(first.directory).phase,
            HousePlayerPhase::Reserved
        );
        let b = authority(30);
        assert!(matches!(
            first.directory.reserve_entry(&b),
            Err(HousePlacementError::TileOccupied)
        ));
        assert_eq!(
            only_player(first.directory).key.reserved_tile.as_ref(),
            a.tile
        );
        assert_eq!(
            first.consume_reservation(),
            Err(HousePlacementError::NotMaterialized)
        );
        drop(first);
        let carrier = directory.namespaces[0]
            .carrier
            .as_ref()
            .expect("same carrier");
        assert!(carrier.occupied.is_empty());
        assert!(matches!(
            carrier.slots[0],
            Slot::VacantReusable { generation: 1, .. }
        ));
        let second = directory.reserve_entry(&b).expect("freed physical slot");
        assert_eq!(second.actor_local_id, ActorLocalId(1));
        assert_eq!(second.actor_local_generation, ActorLocalGeneration(2));
    }

    #[test]
    fn house_actor_placement_materializes_exact_source_slot_without_default_reset() {
        let a = authority(10);
        let (continuity, mut carrier, actor) = source(&a);
        let before = carrier.slots[0].clone();
        let mut directory = HouseActorDirectory::with_explicit_bound(1).expect("directory");
        {
            let source = HouseSourceActorHold::acquire(&mut carrier, &continuity, actor, a.source)
                .expect("source hold");
            let mut placement = directory.reserve_entry(&a).expect("reserve");
            placement.materialize(&source).expect("materialize");
            let stored = only_player(placement.directory)
                .materialized
                .as_ref()
                .expect("actual payload");
            let Slot::Occupied {
                actor: prior_actor,
                position: Some(position),
                lifecycle,
                spell_combat,
                ..
            } = &before
            else {
                panic!("positioned source");
            };
            assert_eq!(stored.actor, *prior_actor);
            assert_eq!(stored.source_position, *position);
            assert_eq!(&stored.lifecycle, lifecycle);
            assert_eq!(&stored.spell_combat, spell_combat);
            assert_eq!(stored.actor, ActorState(91));
            assert_eq!(stored.spell_combat.maximum_health, 135);
            assert!(stored.spell_combat.paralysed);
            assert_eq!(
                placement.materialize(&source),
                Err(HousePlacementError::AlreadyMaterialized)
            );
        }
        assert_eq!(carrier.slots[0], before);
        assert_eq!(
            carrier
                .attacker_authority(0, actor.0.actor_local_generation.0)
                .expect("source authority")
                .fence,
            None
        );
        assert!(
            directory.namespaces[0]
                .carrier
                .as_ref()
                .expect("House arena")
                .occupied
                .is_empty()
        );
    }

    #[test]
    fn house_actor_placement_consumption_binds_initial_key_lease_tile_and_physical_transaction() {
        let a = authority(10);
        let (continuity, mut carrier, actor) = source(&a);
        let source = HouseSourceActorHold::acquire(&mut carrier, &continuity, actor, a.source)
            .expect("hold");
        let mut directory = HouseActorDirectory::with_explicit_bound(1).expect("directory");
        let mut placement = directory.reserve_entry(&a).expect("reserve");
        placement.materialize(&source).expect("materialize");
        assert_eq!(
            placement.verify_consumed(&a),
            Err(HousePlacementError::NotConsumed)
        );
        placement.consume_reservation().expect("consume");
        let mut changed = a.clone();
        changed.initial = key(99);
        assert_eq!(
            placement.verify_consumed(&changed),
            Err(HousePlacementError::ReservationMismatch)
        );
        changed = a.clone();
        changed.lease = CharacterLease::new(a.lease.character_id(), 6).expect("new lease");
        assert_eq!(
            placement.verify_consumed(&changed),
            Err(HousePlacementError::ReservationMismatch)
        );
        changed = a.clone();
        changed.tile[3] += 1;
        assert_eq!(
            placement.verify_consumed(&changed),
            Err(HousePlacementError::ReservationMismatch)
        );
        changed = a.clone();
        changed.txid = "1025".into();
        assert_eq!(
            placement.verify_consumed(&changed),
            Err(HousePlacementError::NotConsumed)
        );
        changed = a.clone();
        changed.node_revision += 1;
        assert_eq!(
            placement.verify_consumed(&changed),
            Err(HousePlacementError::NamespaceChanged)
        );
        changed = a.clone();
        changed.node = NodeId::decode(&uuid(98)).expect("other node");
        assert_eq!(
            placement.verify_consumed(&changed),
            Err(HousePlacementError::NamespaceChanged)
        );
        placement
            .verify_consumed(&a)
            .expect("original native identity");
        let b = authority(30);
        assert!(matches!(
            placement.directory.reserve_entry(&b),
            Err(HousePlacementError::TileOccupied)
        ));
    }

    #[test]
    fn house_actor_placement_unknown_commit_retains_both_owners_and_prepared_reconcile_rolls_back()
    {
        let a = authority(10);
        let (continuity, mut carrier, actor) = source(&a);
        let before = carrier.slots[0].clone();
        let mut directory = HouseActorDirectory::with_explicit_bound(1).expect("directory");
        unresolved(&a, &mut directory, &continuity, &mut carrier, actor);
        assert_eq!(
            only_player(&directory).phase,
            HousePlayerPhase::CommitInFlight
        );
        assert!(
            carrier
                .attacker_authority(0, actor.0.actor_local_generation.0)
                .expect("source authority")
                .fence
                .is_some()
        );
        assert!(matches!(
            directory.reserve_entry(&a),
            Err(HousePlacementError::CommitOutcomeUnresolved)
        ));
        let mut settled = a.clone();
        settled.txid = "1026".into();
        assert_eq!(
            directory.reconcile_entry_with_source(&settled, &mut carrier, &continuity),
            Ok(HouseActorReconcileOutcome::RolledBack)
        );
        assert_eq!(carrier.slots[0], before);
        assert_eq!(
            carrier
                .attacker_authority(0, actor.0.actor_local_generation.0)
                .expect("source authority")
                .fence,
            None
        );
        assert!(
            directory.namespaces[0]
                .carrier
                .as_ref()
                .expect("House arena")
                .occupied
                .is_empty()
        );
        assert_eq!(
            directory.reconcile_entry_with_source(&settled, &mut carrier, &continuity),
            Err(HousePlacementError::RestartMaterializationRequired)
        );
    }

    #[test]
    fn house_actor_placement_released_reconcile_is_same_slot_and_keeps_recycled_source_safe() {
        let a = authority(10);
        let (continuity, mut carrier, actor) = source(&a);
        let mut directory = HouseActorDirectory::with_explicit_bound(1).expect("directory");
        unresolved(&a, &mut directory, &continuity, &mut carrier, actor);
        let mut released = a.clone();
        released.settled = HouseActorSettledState::Released;
        released.txid = "1027".into();
        let outcome = directory
            .reconcile_entry_with_source(&released, &mut carrier, &continuity)
            .expect("released native disposition");
        let HouseActorReconcileOutcome::Durable(placed) = outcome else {
            panic!("durable physical slot");
        };
        assert_eq!(placed.actor_local_id, ActorLocalId(1));
        assert_eq!(placed.actor_local_generation, ActorLocalGeneration(1));
        assert_eq!(only_player(&directory).phase, HousePlayerPhase::Durable);
        assert!(!carrier.contains_committed_player(&continuity, a.source, actor));
        let new_session = GameSessionId::decode(&uuid(90)).expect("new session");
        let next = carrier
            .admit_bound_test_attacker(&continuity, new_session, a.lease)
            .expect("recycled source slot");
        assert_ne!(next, actor);
        let before = carrier.slots.clone();
        assert_eq!(
            directory.reconcile_entry_with_source(&released, &mut carrier, &continuity),
            Ok(HouseActorReconcileOutcome::Durable(placed))
        );
        assert_eq!(carrier.slots, before);
    }

    #[test]
    fn house_actor_placement_changed_reconcile_key_never_frees_retained_reservation() {
        let a = authority(10);
        let (continuity, mut carrier, actor) = source(&a);
        let mut directory = HouseActorDirectory::with_explicit_bound(1).expect("directory");
        unresolved(&a, &mut directory, &continuity, &mut carrier, actor);
        let before = only_player(&directory).clone();
        let source_before = carrier.slots.clone();
        let mut changed = a.clone();
        changed.initial = key(80);
        assert_eq!(
            directory.reconcile_entry_with_source(&changed, &mut carrier, &continuity),
            Err(HousePlacementError::ReservationMismatch)
        );
        assert_eq!(only_player(&directory), &before);
        assert_eq!(carrier.slots, source_before);
    }

    #[test]
    fn house_actor_placement_retired_unresolved_prepared_cleanup_never_reopens_namespace() {
        let a = authority(10);
        let (continuity, mut carrier, actor) = source(&a);
        let mut directory = HouseActorDirectory::with_explicit_bound(1).expect("directory");
        unresolved(&a, &mut directory, &continuity, &mut carrier, actor);
        directory
            .retire(&a.house, a.generation)
            .expect("denial-only retire");
        assert!(!directory.namespaces[0].active);
        assert_eq!(
            only_player(&directory).phase,
            HousePlayerPhase::CommitInFlight
        );
        let mut prepared = a.clone();
        prepared.txid = "1030".into();
        assert_eq!(
            directory.reconcile_entry_with_source(&prepared, &mut carrier, &continuity),
            Ok(HouseActorReconcileOutcome::RolledBack)
        );
        assert!(!directory.namespaces[0].active);
        assert!(
            directory.namespaces[0]
                .carrier
                .as_ref()
                .expect("retained namespace")
                .occupied
                .is_empty()
        );
        assert_eq!(
            carrier
                .attacker_authority(0, actor.0.actor_local_generation.0)
                .expect("source authority")
                .fence,
            None
        );
        assert!(matches!(
            directory.reserve_entry(&a),
            Err(HousePlacementError::NamespaceRetired)
        ));
    }

    #[test]
    fn house_actor_placement_channel_methods_refuse_house_domain() {
        let a = authority(10);
        let (continuity, _, actor) = source(&a);
        let mut directory = HouseActorDirectory::with_explicit_bound(1).expect("directory");
        let placement = directory.reserve_entry(&a).expect("reserve");
        let carrier = placement.directory.namespaces[0]
            .carrier
            .as_mut()
            .expect("House arena");
        let before = carrier.slots.clone();
        assert!(matches!(
            carrier.reserve_player(&continuity, a.source),
            Err(CarrierError::WrongScope)
        ));
        assert_eq!(
            carrier.lookup(&continuity, actor.0),
            Err(CarrierError::WrongScope)
        );
        assert!(carrier.test_player_actor(a.destination).is_none());
        assert_eq!(carrier.slots, before);
    }

    #[test]
    fn house_actor_placement_namespace_is_bounded_and_never_rebootstraps_retired_generation() {
        let a = authority(10);
        let mut directory = HouseActorDirectory::with_explicit_bound(1).expect("directory");
        drop(directory.reserve_entry(&a).expect("reserve then rollback"));
        let mut changed = a.clone();
        changed.node_revision += 1;
        assert!(matches!(
            directory.reserve_entry(&changed),
            Err(HousePlacementError::NamespaceChanged)
        ));
        changed = a.clone();
        changed.house =
            HouseId::new(a.house.world_id(), "oteryn:content.house.native_b").expect("other House");
        assert!(matches!(
            directory.reserve_entry(&changed),
            Err(HousePlacementError::NamespaceCapacity)
        ));
        directory.retire(&a.house, a.generation).expect("retire");
        assert!(matches!(
            directory.reserve_entry(&a),
            Err(HousePlacementError::NamespaceRetired)
        ));
        changed = a.clone();
        changed.generation =
            ScopeOwnershipGeneration::new(2).expect("actual successor generation fixture");
        let next = directory
            .reserve_entry(&changed)
            .expect("empty retired namespace, actual newer fixture authority");
        assert_eq!(next.key.scope_generation, changed.generation);
    }

    #[test]
    fn house_actor_placement_exhausted_generation_retires_head_and_allows_next_slot() {
        let a = authority(10);
        let mut directory = HouseActorDirectory::with_explicit_bound(1).expect("directory");
        drop(
            directory
                .reserve_entry(&a)
                .expect("claim namespace then rollback"),
        );
        let carrier = directory.namespaces[0]
            .carrier
            .as_mut()
            .expect("House arena");
        let Slot::VacantReusable { generation, .. } = &mut carrier.slots[0] else {
            panic!("free head");
        };
        *generation = u64::MAX;
        assert!(matches!(
            directory.reserve_entry(&a),
            Err(HousePlacementError::Carrier(
                CarrierError::ActorGenerationExhausted
            ))
        ));
        let carrier = directory.namespaces[0]
            .carrier
            .as_ref()
            .expect("same arena");
        assert_eq!(carrier.free_head, Some(1));
        assert!(matches!(
            carrier.slots[0],
            Slot::Exhausted {
                generation: u64::MAX
            }
        ));
        assert!(carrier.occupied.is_empty());
        let next = directory
            .reserve_entry(&a)
            .expect("next reusable physical slot");
        assert_eq!(next.actor_local_id, ActorLocalId(2));
        assert_eq!(next.actor_local_generation, ActorLocalGeneration(1));
    }

    #[test]
    fn house_actor_placement_released_wrong_source_channel_does_not_settle_either_owner() {
        let a = authority(10);
        let (continuity, mut carrier, actor) = source(&a);
        let mut directory = HouseActorDirectory::with_explicit_bound(1).expect("directory");
        unresolved(&a, &mut directory, &continuity, &mut carrier, actor);
        let mut other_continuity =
            NamespaceContinuityGuard::from_pre_production_grant(PreProductionContinuityGrant {
                world_id: a.house.world_id(),
                channel_id: super::super::ChannelId::decode(&uuid(99))
                    .expect("other valid Channel"),
                scope_generation: continuity.current_generation,
            });
        let mut other = ChannelActorCarrier::bootstrap_pre_production(&mut other_continuity, 2)
            .expect("other valid arena");
        let mut released = a.clone();
        released.settled = HouseActorSettledState::Released;
        let house_before = only_player(&directory).clone();
        let source_before = carrier.slots.clone();
        let wrong_before = other.slots.clone();
        assert_eq!(
            directory.reconcile_entry_with_source(&released, &mut other, &other_continuity),
            Err(HousePlacementError::Carrier(CarrierError::WrongScope))
        );
        assert_eq!(only_player(&directory), &house_before);
        assert_eq!(carrier.slots, source_before);
        assert_eq!(other.slots, wrong_before);
        assert!(
            carrier
                .attacker_authority(0, actor.0.actor_local_generation.0)
                .expect("original authority")
                .fence
                .is_some()
        );
    }

    #[test]
    fn house_actor_placement_released_changed_source_payload_is_retained_for_native_reconciliation()
    {
        let a = authority(10);
        for change_position in [true, false] {
            let (continuity, mut carrier, actor) = source(&a);
            let mut directory = HouseActorDirectory::with_explicit_bound(1).expect("directory");
            unresolved(&a, &mut directory, &continuity, &mut carrier, actor);
            if let Slot::Occupied {
                position: Some(position),
                spell_combat,
                ..
            } = &mut carrier.slots[0]
            {
                if change_position {
                    position.position.x += 1;
                    position.revision += 1;
                } else {
                    spell_combat.paralysed = false;
                }
            }
            let mut released = a.clone();
            released.settled = HouseActorSettledState::Released;
            let house_before = only_player(&directory).clone();
            let source_before = carrier.slots.clone();
            assert_eq!(
                directory.reconcile_entry_with_source(&released, &mut carrier, &continuity),
                Err(HousePlacementError::InvalidNativeBinding)
            );
            assert_eq!(only_player(&directory), &house_before);
            assert_eq!(carrier.slots, source_before);
            assert!(
                carrier
                    .attacker_authority(0, actor.0.actor_local_generation.0)
                    .expect("source authority")
                    .fence
                    .is_some()
            );
        }
    }

    #[test]
    fn house_actor_placement_unpositioned_or_changed_lease_source_cannot_materialize() {
        let a = authority(10);
        let (continuity, mut carrier, actor) = source(&a);
        if let Slot::Occupied { position, .. } = &mut carrier.slots[0] {
            *position = None;
        }
        let source = HouseSourceActorHold::acquire(&mut carrier, &continuity, actor, a.source)
            .expect("hold");
        let mut directory = HouseActorDirectory::with_explicit_bound(1).expect("directory");
        let mut placement = directory.reserve_entry(&a).expect("reserve");
        assert_eq!(
            placement.materialize(&source),
            Err(HousePlacementError::InvalidNativeBinding)
        );
        assert_eq!(
            only_player(placement.directory).phase,
            HousePlayerPhase::Reserved
        );
        let mut changed_key = placement.key.clone();
        changed_key.source_character_lease =
            CharacterLease::new(a.lease.character_id(), 6).expect("other lease");
        assert_eq!(
            source.materialization(&changed_key),
            Err(HousePlacementError::InvalidNativeBinding)
        );
    }

    #[test]
    fn house_actor_placement_panic_before_commit_rolls_back_exact_slot_and_source_fence() {
        let a = authority(10);
        let (continuity, mut carrier, actor) = source(&a);
        let before = carrier.slots.clone();
        let mut directory = HouseActorDirectory::with_explicit_bound(1).expect("directory");
        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let source = HouseSourceActorHold::acquire(&mut carrier, &continuity, actor, a.source)
                .expect("hold");
            let mut placement = directory.reserve_entry(&a).expect("reserve");
            placement.materialize(&source).expect("materialize");
            placement.consume_reservation().expect("consume");
            panic!("injected cancellation before SQL COMMIT");
        }));
        assert!(panicked.is_err());
        assert_eq!(carrier.slots, before);
        assert_eq!(
            carrier
                .attacker_authority(0, actor.0.actor_local_generation.0)
                .expect("source authority")
                .fence,
            None
        );
        assert!(
            directory.namespaces[0]
                .carrier
                .as_ref()
                .expect("House arena")
                .occupied
                .is_empty()
        );
    }

    #[test]
    fn house_actor_placement_npc_paths_preserve_validated_channel_identity() {
        use super::super::npc_actor::{
            NpcPlacement, NpcPlacementError, NpcPlacementTable, NpcPlacements,
        };
        use super::super::{ChannelContentPin, ChannelId, MovementFacing};

        fn runtime(seed: u8, capacity: usize) -> ChannelRuntimeV1 {
            let world = WorldId::decode(&uuid(1)).expect("fixture World");
            ChannelRuntimeV1::from_committed_assignment(
                world,
                ChannelId::decode(&uuid(seed)).expect("fixture Channel"),
                NodeId::decode(&uuid(3)).expect("fixture Node"),
                7,
                5,
                11,
                "runtime-scope-assignment:11",
                capacity,
                ChannelContentPin::test(world),
            )
            .expect("actual Channel runtime under a pure fixture assignment")
        }

        fn assert_npcs(
            runtime: &ChannelRuntimeV1,
            expected_channel: ChannelId,
            actors: &[ExactActorRef],
            keys: &[&str],
        ) {
            // The enum's Channel payload is precisely the old carrier.channel_id.
            // Compare that stored value independently of the continuity-based NPC write.
            let CarrierScopeDomain::Channel(stored_channel) = &runtime.carrier.scope_domain else {
                panic!("NPCs require the actual Channel carrier domain");
            };
            assert_eq!(*stored_channel, expected_channel);
            assert_eq!(runtime.continuity.channel_id, *stored_channel);
            assert_eq!(
                runtime
                    .carrier
                    .validate_current_continuity(&runtime.continuity),
                Ok(())
            );
            assert_eq!(actors.len(), keys.len());
            for (actor, key) in actors.iter().zip(keys) {
                assert_eq!(actor.channel_id(), *stored_channel);
                assert_eq!(actor.world_id(), runtime.carrier.world_id);
                assert_eq!(actor.scope_generation(), runtime.carrier.scope_generation);
                assert_eq!(runtime.npc_key(*actor), Some(*key));
                let index = usize::try_from(actor.actor_local_id() - 1).expect("NPC slot index");
                let Slot::NpcOccupied { position, .. } = &runtime.carrier.slots[index] else {
                    panic!("the returned NPC is in the original physical arena");
                };
                assert_eq!(position.context.channel_id, *stored_channel);
                let mut wrong_channel = *actor;
                wrong_channel.0.channel_id = ChannelId::decode(&uuid(222)).expect("other Channel");
                assert_eq!(runtime.npc_key(wrong_channel), None);
            }
            assert_eq!(
                runtime
                    .visible_entities()
                    .npcs
                    .iter()
                    .map(|npc| npc.actor)
                    .collect::<Vec<_>>(),
                actors
            );
        }

        fn assert_wrong_scope_is_inert(runtime: &mut ChannelRuntimeV1, table: &NpcPlacementTable) {
            for production in [false, true] {
                let before = runtime.carrier.clone();
                let catalogue_called = std::cell::Cell::new(false);
                assert_eq!(
                    runtime.place_npcs(
                        table,
                        |_| {
                            catalogue_called.set(true);
                            true
                        },
                        production
                    ),
                    Err(NpcPlacementError::Carrier(CarrierError::WrongScope))
                );
                assert!(
                    !catalogue_called.get(),
                    "continuity is checked before NPC eligibility"
                );
                assert_eq!(runtime.carrier, before);
            }
        }

        let placement = |x| NpcPlacement {
            floor: -2,
            y: 7,
            x,
            direction: MovementFacing::South,
        };
        let table = NpcPlacementTable {
            npcs: vec![
                NpcPlacements {
                    key: "oteryn:npc.a".into(),
                    placements: vec![placement(4), placement(5)],
                },
                NpcPlacements {
                    key: "oteryn:npc.b".into(),
                    placements: vec![placement(6)],
                },
            ],
        };
        let empty = NpcPlacementTable::default();
        for seed in [2, 19, 99] {
            let channel = ChannelId::decode(&uuid(seed)).expect("expected stored Channel");
            for production in [false, true] {
                let mut current = runtime(seed, 4);
                let outcome = current
                    .place_npcs(&table, |_| true, production)
                    .expect("NPC placement");
                assert!(outcome.not_admitted.is_empty());
                assert_npcs(
                    &current,
                    channel,
                    &outcome.actors,
                    &["oteryn:npc.a", "oteryn:npc.a", "oteryn:npc.b"],
                );
                let before = current.carrier.clone();
                assert_eq!(
                    current.place_npcs(&table, |_| true, production),
                    Err(NpcPlacementError::AlreadyPlaced)
                );
                assert_eq!(current.carrier, before);
            }

            // Production refusal cannot leak the admitted prefix. The same unchanged
            // owner can then take the non-production path, which places only that prefix.
            let mut current = runtime(seed, 4);
            let before = current.carrier.clone();
            assert_eq!(
                current.place_npcs(&table, |key| key == "oteryn:npc.a", true),
                Err(NpcPlacementError::NpcNotAdmitted("oteryn:npc.b".into()))
            );
            assert_eq!(current.carrier, before);
            let outcome = current
                .place_npcs(&table, |key| key == "oteryn:npc.a", false)
                .expect("eligible NPC prefix");
            assert_eq!(outcome.not_admitted, vec!["oteryn:npc.b"]);
            assert_npcs(
                &current,
                channel,
                &outcome.actors,
                &["oteryn:npc.a", "oteryn:npc.a"],
            );

            let mut current = runtime(seed, 4);
            let before = current.carrier.clone();
            let outcome = current
                .place_npcs(&table, |_| false, false)
                .expect("all held outside production");
            assert_eq!(outcome.not_admitted, vec!["oteryn:npc.a", "oteryn:npc.b"]);
            assert_npcs(&current, channel, &outcome.actors, &[]);
            assert_eq!(current.carrier, before);
            for production in [false, true] {
                let outcome = current
                    .place_npcs(&empty, |_| true, production)
                    .expect("empty NPC table");
                assert_npcs(&current, channel, &outcome.actors, &[]);
                assert_eq!(current.carrier, before);
            }

            let mut current = runtime(seed, 2);
            let before = current.carrier.clone();
            assert_eq!(
                current.place_npcs(&table, |_| true, true),
                Err(NpcPlacementError::Carrier(CarrierError::CapacityExceeded))
            );
            assert_eq!(current.carrier, before);

            // The first NPC is physically staged before the second slot exhausts.
            // Rollback must restore that complete original Channel arena.
            let mut current = runtime(seed, 4);
            let Slot::VacantReusable { generation, .. } = &mut current.carrier.slots[1] else {
                panic!("second reusable NPC slot");
            };
            *generation = u64::MAX;
            let before = current.carrier.clone();
            assert_eq!(
                current.place_npcs(&table, |_| true, true),
                Err(NpcPlacementError::Carrier(
                    CarrierError::ActorGenerationExhausted
                ))
            );
            assert_eq!(current.carrier, before);

            for invalid in 0..4 {
                let mut current = runtime(seed, 4);
                match invalid {
                    0 => {
                        current.continuity.channel_id =
                            ChannelId::decode(&uuid(222)).expect("wrong Channel")
                    }
                    1 => {
                        current.continuity.world_id =
                            WorldId::decode(&uuid(223)).expect("wrong World")
                    }
                    2 => {
                        current.continuity.current_generation =
                            ScopeOwnershipGeneration::new(6).expect("different generation")
                    }
                    _ => current.retire_owner_cycle(),
                }
                assert_wrong_scope_is_inert(&mut current, &table);
                assert_wrong_scope_is_inert(&mut current, &empty);
            }
        }

        // A real House arena must never acquire a Channel identity via the NPC path.
        // This deliberately invalid composition is confined to the pure negative fixture.
        let a = authority(10);
        let mut directory = HouseActorDirectory::with_explicit_bound(1).expect("House directory");
        drop(
            directory
                .reserve_entry(&a)
                .expect("actual House namespace, then rollback"),
        );
        let mut wrong_domain = runtime(2, 4);
        wrong_domain.carrier = directory.namespaces[0]
            .carrier
            .take()
            .expect("actual House carrier");
        assert_wrong_scope_is_inert(&mut wrong_domain, &table);
        assert_wrong_scope_is_inert(&mut wrong_domain, &empty);
        let before = wrong_domain.carrier.clone();
        let visible = wrong_domain.visible_entities();
        assert!(
            visible.players.is_empty() && visible.creatures.is_empty() && visible.npcs.is_empty()
        );
        assert_eq!(
            wrong_domain.current_live_creature_candidates(4),
            Err(CarrierError::WrongScope)
        );
        assert_eq!(wrong_domain.carrier, before);
    }
}
