//! Adapt the existing SQL-fenced standing-tile read; detached Item names or
//! client handles cannot construct this bridge's actual Ground selection.
use super::*;
use crate::durability::creature_source_items::SourceGroundStep;
use crate::durability::spell_item_transaction::StandingTileRead;
use crate::durability::spell_items_abi::DurableTileItem;
use oteryn_gameplay_entities::{SourceActorReservationProof, source_reservation_seal};

/// The actual committed translation is retained before the first Ground read.
/// Its actor reservation cannot be reconstructed from a later standing pose.
#[derive(Debug, Clone)]
pub struct PreparedSpiteStep {
    actor: ExactActorRef,
    session: GameSessionId,
    character: [u8; 16],
    lease: u64,
    movement: MovementPositionSnapshot,
    reset_epoch: u64,
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    node: [u8; 16],
    node_revision: u64,
    assignment_revision: u64,
    activation: u64,
    pins: [[u8; 32]; 4],
    target: SpellGroundTarget,
    binding: [u8; 32],
    stamp: crate::foundation::RuntimeWorkStamp,
    remorse_items: Vec<[u8; 16]>,
    ground_steps: Vec<SourceGroundStep>,
}
impl PreparedSpiteStep {
    pub fn from_committed_movement(
        runtime: &mut ChannelRuntimeV1,
        native: &crate::content::native_gameplay::NativeGameplayState,
        room: &crate::content::QualifiedNativeEntryRoom,
        actor: ExactActorRef,
        session: GameSessionId,
        movement: MovementPositionSnapshot,
        reset_epoch: u64,
    ) -> Option<Self> {
        Self::capture(
            runtime,
            native,
            room,
            actor,
            session,
            movement,
            reset_epoch,
            None,
        )
    }
    /// Item-id callbacks use the actual destination rows already locked by the
    /// movement owner. Ordinary tiles do not schedule another detached read.
    #[allow(
        clippy::too_many_arguments,
        reason = "the committed translation and the existing SQL destination read have independent authority"
    )]
    pub fn from_committed_movement_with_items(
        runtime: &mut ChannelRuntimeV1,
        native: &crate::content::native_gameplay::NativeGameplayState,
        room: &crate::content::QualifiedNativeEntryRoom,
        actor: ExactActorRef,
        session: GameSessionId,
        movement: MovementPositionSnapshot,
        reset_epoch: u64,
        read: &StandingTileRead,
    ) -> Option<Self> {
        Self::capture(
            runtime,
            native,
            room,
            actor,
            session,
            movement,
            reset_epoch,
            Some(read),
        )
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "the original current movement optionally carries its existing destination Item read"
    )]
    fn capture(
        runtime: &mut ChannelRuntimeV1,
        native: &crate::content::native_gameplay::NativeGameplayState,
        room: &crate::content::QualifiedNativeEntryRoom,
        actor: ExactActorRef,
        session: GameSessionId,
        movement: MovementPositionSnapshot,
        reset_epoch: u64,
        read: Option<&StandingTileRead>,
    ) -> Option<Self> {
        let at = movement.position();
        let remorse_items: Vec<_> = read
            .into_iter()
            .flat_map(|read| read.items())
            .filter(|item| {
                item.definition.family == "Item"
                    && item.definition.production_key == SourceStepKind::Remorse.item_key()
                    && item.definition.revision_ref == "definition-r1"
            })
            .map(|item| item.item_instance_id)
            .collect();
        let weeping_items: Vec<_> = read
            .into_iter()
            .flat_map(|read| read.items())
            .filter(|item| {
                item.definition.family == "Item"
                    && item.definition.production_key == "oteryn:item.tibia.i33876"
                    && item.definition.revision_ref == "definition-r1"
            })
            .map(|item| item.item_instance_id)
            .collect();
        if (remorse_items.is_empty()
            && weeping_items.is_empty()
            && ![
                (33743, 31628, 14),
                (33736, 31632, 14),
                (33750, 31632, 14),
                (33742, 31637, 14),
            ]
            .contains(&(at.x, at.y, at.floor)))
            || runtime.read_actor_position(actor).ok()? != movement
            || movement.last_translation_revision() == 0
            || movement.revision() != movement.last_translation_revision()
            || runtime
                .player_control_facts(actor, session)
                .ok()?
                .control_loss
                .is_some()
            || runtime.house_actor_state(actor, session).is_ok()
            || native.source_digest() != runtime.content_pin().server_artifact_digest()
            || !native.qualifies_soul_war_callbacks(runtime.content_pin().server_artifact_digest())
        {
            return None;
        }
        let target = SpellGroundTarget::for_native_tile_read(room, runtime, at).ok()?;
        if let Some(read) = read
            && (read.target() != &target
                || read.content_digest() != native.source_digest()
                || read.fence().game_session_id != session
                || read.fence().runtime_scope != runtime.binding().runtime_scope()
                || read.fence().scope_ownership_generation != runtime.binding().scope_generation())
        {
            return None;
        }
        let lease = runtime.bound_attacker_lease(actor, session).ok()?;
        let character = *lease.character_id().as_bytes();
        let lease = lease.generation();
        if let Some(read) = read
            && (read.fence().character_id.as_bytes() != &character
                || read.fence().character_lease_generation != lease)
        {
            return None;
        }
        let ground_steps = weeping_items
            .into_iter()
            .map(|item| {
                SourceGroundStep::from_current_player_step(
                    runtime,
                    room,
                    native,
                    actor,
                    session,
                    movement,
                    reset_epoch,
                    read?,
                    item,
                )
            })
            .collect::<Option<Vec<_>>>()?;
        let stamp = runtime.issue_owner_work().ok()?;
        let owner = runtime.binding();
        let pin = runtime.content_pin();
        let pins = [
            pin.server_artifact_digest(),
            pin.client_artifact_digest(),
            pin.frame_binding_digest(),
            runtime.map_revision_digest(),
        ];
        let mut hash = Sha256::new();
        hash.update(b"oteryn:spite-committed-step-read:v1");
        hash.update(actor.placement_identity());
        hash.update(session.as_bytes());
        hash.update(character);
        hash.update(lease.to_be_bytes());
        hash.update(movement.revision().to_be_bytes());
        hash.update(reset_epoch.to_be_bytes());
        hash.update(&target.spatial_position);
        hash.update(owner.world_id().as_bytes());
        match owner.runtime_scope() {
            RuntimeScopeRefV1::Channel { channel_id, .. } => {
                hash.update([1]);
                hash.update(channel_id.as_bytes());
            }
            RuntimeScopeRefV1::Instance { instance_id, .. } => {
                hash.update([2]);
                hash.update(instance_id);
            }
        }
        hash.update(actor.actor_local_generation().to_be_bytes());
        hash.update(movement.last_translation_revision().to_be_bytes());
        hash.update(owner.scope_generation().get().to_be_bytes());
        hash.update(pin.activation_sequence().to_be_bytes());
        for value in pins {
            hash.update(value);
        }
        for item in &remorse_items {
            hash.update(item);
        }
        for step in &ground_steps {
            hash.update(step.binding());
        }
        Some(Self {
            actor,
            session,
            character,
            lease,
            movement,
            reset_epoch,
            scope: owner.runtime_scope(),
            generation: owner.scope_generation(),
            node: *owner.node_id().as_bytes(),
            node_revision: owner.node_registration_revision(),
            assignment_revision: owner.source_revision(),
            activation: pin.activation_sequence(),
            pins,
            target,
            binding: hash.finalize().into(),
            stamp,
            remorse_items,
            ground_steps,
        })
    }
    pub fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        let owner = runtime.binding();
        let pin = runtime.content_pin();
        runtime
            .owner_fence()
            .is_ok_and(|fence| fence.accepts_stamp(self.stamp))
            && owner.runtime_scope() == self.scope
            && owner.scope_generation() == self.generation
            && owner.node_id().as_bytes() == &self.node
            && owner.node_registration_revision() == self.node_revision
            && owner.source_revision() == self.assignment_revision
            && pin.activation_sequence() == self.activation
            && [
                pin.server_artifact_digest(),
                pin.client_artifact_digest(),
                pin.frame_binding_digest(),
                runtime.map_revision_digest(),
            ] == self.pins
            && runtime.read_actor_position(self.actor) == Ok(self.movement)
            && runtime
                .bound_attacker_lease(self.actor, self.session)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == &self.character
                        && lease.generation() == self.lease
                })
    }
    pub fn actor(&self) -> ExactActorRef {
        self.actor
    }
    pub fn session(&self) -> GameSessionId {
        self.session
    }
    pub fn movement(&self) -> MovementPositionSnapshot {
        self.movement
    }
    pub fn target(&self) -> &SpellGroundTarget {
        &self.target
    }
    pub fn content_digest(&self) -> [u8; 32] {
        self.pins[0]
    }
    pub fn matches_fence(
        &self,
        fence: &crate::durability::item_transfer::CurrentCharacterItemFence,
    ) -> bool {
        fence.character_id.as_bytes() == &self.character
            && fence.game_session_id == self.session
            && fence.character_lease_generation == self.lease
            && fence.runtime_scope == self.scope
            && fence.scope_ownership_generation == self.generation
    }
    pub fn callback_count(&self) -> usize {
        self.remorse_items
            .len()
            .saturating_add(self.ground_steps.len())
            .saturating_add(1)
    }
    /// Only the exact UUIDs read at the committed destination can be consumed.
    /// A later tile read cannot introduce another Item-id callback.
    pub fn source_ground_steps(&self) -> &[SourceGroundStep] {
        &self.ground_steps
    }
    pub fn qualify_current_next(
        &self,
        runtime: &ChannelRuntimeV1,
        native: &crate::content::native_gameplay::NativeGameplayState,
        room: &crate::content::QualifiedNativeEntryRoom,
        read: &StandingTileRead,
        completed_items: &[[u8; 16]],
        fire_finished: bool,
    ) -> Option<Option<SoulWarSourceStep>> {
        if !self.current_for(runtime) || !self.matches_fence(read.fence()) {
            return None;
        }
        let selected = SoulWarSourceStep::from_standing_read(
            runtime,
            native,
            room,
            self.actor,
            self.session,
            self.movement,
            self.reset_epoch,
            read,
        )?
        .into_iter()
        .find(|input| {
            !completed_items.contains(&input.item_instance_id())
                && match input.kind() {
                    SourceStepKind::SpiteFire => !fire_finished,
                    SourceStepKind::Remorse => {
                        self.remorse_items.contains(&input.item_instance_id())
                    }
                    SourceStepKind::Energy => false,
                }
        });
        Some(selected)
    }
}
impl source_reservation_seal::Sealed for PreparedSpiteStep {}
impl SourceActorReservationProof for PreparedSpiteStep {
    fn owner(&self) -> (ExactActorRef, GameSessionId) {
        (self.actor, self.session)
    }
    fn event_identity(&self) -> [u8; 16] {
        *SourceStepOccurrence::from_binding_digest(self.binding).as_bytes()
    }
    fn source_binding_digest(&self) -> [u8; 32] {
        self.binding
    }
    fn affected_actors(&self) -> &[ExactActorRef] {
        &[]
    }
    fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.current_for(runtime)
    }
}

struct CurrentStandingItems<'a> {
    read: &'a StandingTileRead,
    actor: ExactActorRef,
    session: GameSessionId,
    movement: MovementPositionSnapshot,
    selected: Option<&'a DurableTileItem>,
}
impl SoulWarStepItemRead for CurrentStandingItems<'_> {
    fn current(&self, runtime: &ChannelRuntimeV1) -> bool {
        let fence = self.read.fence();
        runtime.owner_fence().is_ok()
            && fence.game_session_id == self.session
            && fence.runtime_scope == runtime.binding().runtime_scope()
            && fence.scope_ownership_generation == runtime.binding().scope_generation()
            && self.read.content_digest() == runtime.content_pin().server_artifact_digest()
            && runtime.read_actor_position(self.actor) == Ok(self.movement)
    }
    fn items(&self, position: MovementLocalPosition) -> Vec<SourceStepGroundItem> {
        if position != self.movement.position() {
            return Vec::new();
        }
        self.selected
            .into_iter()
            .map(|item| SourceStepGroundItem {
                item: item.item_instance_id,
                definition: item.definition.clone(),
                position,
            })
            .collect()
    }
}
impl SoulWarSourceStep {
    /// The caller keeps the physical lane through the subsequent fused write;
    /// that write independently rechecks each original UID and reconciles first.
    #[allow(
        clippy::too_many_arguments,
        reason = "existing SQL read, current movement and native room are independent owners"
    )]
    pub fn from_standing_read(
        runtime: &ChannelRuntimeV1,
        native: &crate::content::native_gameplay::NativeGameplayState,
        room: &crate::content::QualifiedNativeEntryRoom,
        actor: ExactActorRef,
        session: GameSessionId,
        movement: MovementPositionSnapshot,
        reset_epoch: u64,
        read: &StandingTileRead,
    ) -> Option<Vec<Self>> {
        let expected =
            SpellGroundTarget::for_native_tile_read(room, runtime, movement.position()).ok()?;
        if read.target() != &expected {
            return None;
        }
        let reader = CurrentStandingItems {
            read,
            actor,
            session,
            movement,
            selected: None,
        };
        if !reader.current(runtime) {
            return None;
        }
        Some(
            read.items()
                .iter()
                .filter_map(|item| {
                    let reader = CurrentStandingItems {
                        read,
                        actor,
                        session,
                        movement,
                        selected: Some(item),
                    };
                    Self::from_current_owner(
                        runtime,
                        native,
                        room,
                        actor,
                        session,
                        movement,
                        reset_epoch,
                        item.item_instance_id,
                        &reader,
                    )
                })
                .collect(),
        )
    }
}
