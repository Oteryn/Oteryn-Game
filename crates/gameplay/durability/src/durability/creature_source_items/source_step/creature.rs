//! Sealed original33949 tile DATA from the existing scope Item transaction.
//! A Creature cohort binds only a real committed translation, never a Player
//! fence, standing-position difference, invented command or replacement Item.
use super::*;
use crate::foundation::{MovementLocalPosition, ScopeOwnershipGeneration};
use oteryn_gameplay_entities::CompanionSnapshot;
use oteryn_gameplay_entities::carrier::runtime_actor_source_reservation::{
    ScopeCreatureReservationProof, source_reservation_seal,
};
use std::sync::Arc;

/// All owned storage needed to bind one original Item after the position
/// commit. This is allocated while movement can still refuse without effects.
pub struct PreparedSourceCreatureItem {
    item: [u8; 16],
    definitions: Vec<TypedDefinitionRef>,
    encounter: TypedDefinitionRef,
    target: SpellGroundTarget,
    occurrence: String,
}

#[derive(Debug)]
pub struct SourceCreatureGroundTile {
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    node: [u8; 16],
    node_revision: u64,
    assignment_revision: u64,
    activation: u64,
    content: [u8; 32],
    client: [u8; 32],
    map: [u8; 32],
    frame: [u8; 32],
    reset_epoch: u64,
    position: MovementLocalPosition,
    target: SpellGroundTarget,
    items: Vec<([u8; 16], TypedDefinitionRef)>,
}

impl SourceCreatureGroundTile {
    /// The reader independently obtains its own original rows. Detached caller
    /// DATA cannot manufacture this private cohort by supplying Item UUIDs.
    #[allow(
        clippy::too_many_arguments,
        reason = "current scope SQL, native map/content and reset independently bind the original destination"
    )]
    pub async fn read_in_transaction(
        tx: &mut Transaction<'_, Postgres>,
        authority: &SpellItemScopeAuthority,
        runtime: &ChannelRuntimeV1,
        room: &crate::content::QualifiedNativeEntryRoom,
        native: &crate::content::native_gameplay::NativeGameplayState,
        reset_epoch: u64,
        position: MovementLocalPosition,
    ) -> Result<Option<Arc<Self>>> {
        let binding = runtime.binding();
        let pin = runtime.content_pin();
        let target = SpellGroundTarget::for_native_tile_read(room, runtime, position)
            .map_err(SpellItemError::Rejected)?;
        if runtime.owner_fence().is_err()
            || authority.runtime_scope() != binding.runtime_scope()
            || authority.generation() != binding.scope_generation().get()
            || native.source_digest() != pin.server_artifact_digest()
            || !native.qualifies_soul_war_callbacks(pin.server_artifact_digest())
        {
            return Err(SpellItemError::Rejected("current Creature STEP tile owner"));
        }
        let read = read_creature_scope_tile_in_transaction(tx, authority, &target).await?;
        let mut items = Vec::new();
        for item in read.items {
            if SourceGroundStepKind::from_definition(&item.definition).is_none() {
                continue;
            }
            if !uuid_v7(&item.item_instance_id)
                || native.item_index().is_none_or(|index| {
                    index
                        .definition_ref(
                            "Item",
                            &item.definition.production_key,
                            &item.definition.revision_ref,
                        )
                        .is_none()
                })
            {
                return Err(SpellItemError::Rejected("original Creature STEP Item"));
            }
            items.push((item.item_instance_id, item.definition));
        }
        if !items.iter().any(|(_, definition)| {
            SourceGroundStepKind::from_definition(definition)
                == Some(SourceGroundStepKind::DeadAspect)
        }) {
            return Ok(None);
        }
        Ok(Some(Arc::new(Self {
            scope: binding.runtime_scope(),
            generation: binding.scope_generation(),
            node: *binding.node_id().as_bytes(),
            node_revision: binding.node_registration_revision(),
            assignment_revision: binding.source_revision(),
            activation: pin.activation_sequence(),
            content: pin.server_artifact_digest(),
            client: pin.client_artifact_digest(),
            map: runtime.map_revision_digest(),
            frame: pin.frame_binding_digest(),
            reset_epoch,
            position,
            target,
            items,
        })))
    }

    fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        let binding = runtime.binding();
        let pin = runtime.content_pin();
        runtime.owner_fence().is_ok()
            && binding.runtime_scope() == self.scope
            && binding.scope_generation() == self.generation
            && binding.node_id().as_bytes() == &self.node
            && binding.node_registration_revision() == self.node_revision
            && binding.source_revision() == self.assignment_revision
            && pin.activation_sequence() == self.activation
            && pin.server_artifact_digest() == self.content
            && pin.client_artifact_digest() == self.client
            && runtime.map_revision_digest() == self.map
            && pin.frame_binding_digest() == self.frame
    }
}

/// Preallocated scope proof whose identity already binds the original actor,
/// predecessor, destination and complete Item cohort before movement. The
/// private publication additionally retains the actual commit occurrence.
#[derive(Debug, Clone)]
pub struct SourceCreatureGroundStep {
    tile: Arc<SourceCreatureGroundTile>,
    actors: [ExactActorRef; 1],
    before: MovementPositionSnapshot,
    subject: CompanionSnapshot,
    movement: Option<CommittedMovementOccurrence>,
    binding: [u8; 32],
    remove_creature: bool,
}

impl SourceCreatureGroundStep {
    pub fn prepare(
        runtime: &ChannelRuntimeV1,
        tile: Arc<SourceCreatureGroundTile>,
        actor: ExactActorRef,
        before: MovementPositionSnapshot,
    ) -> Option<Self> {
        let subject = runtime.companion_snapshot(actor).ok()?;
        let policy = runtime
            .companion_policy(&subject.state.policy.definition_key)
            .ok()?;
        if !tile.current_for(runtime)
            || runtime.read_actor_position(actor).ok()? != before
            || subject.state.policy != policy
            || runtime.assert_actor_spell_unreserved(actor).is_err()
            || before.position() == tile.position
        {
            return None;
        }
        let remove_creature = matches!(
            policy.display_name.as_str(),
            "Lesser Splinter of Madness"
                | "Greater Splinter of Madness"
                | "Mighty Splinter of Madness"
        ) && tile.items.first().is_some_and(|(_, definition)| {
            SourceGroundStepKind::from_definition(definition)
                == Some(SourceGroundStepKind::DeadAspect)
        });
        // These actual source profiles are masterless XML/source creations.
        // This callback acquires no authority over a player-owned companion.
        if remove_creature
            && (subject.state.master.is_some() || runtime.native_summon_master(actor).is_some())
        {
            return None;
        }
        let mut hash = Sha256::new();
        hash.update(b"oteryn:source-ground-creature-cohort:v1");
        hash.update(actor.placement_identity());
        hash.update(actor.actor_local_generation().to_be_bytes());
        hash.update(before.revision().to_be_bytes());
        hash.update(tile.scope.world_id().as_bytes());
        match tile.scope {
            RuntimeScopeRefV1::Channel { channel_id, .. } => {
                hash.update([1]);
                hash.update(channel_id.as_bytes());
            }
            RuntimeScopeRefV1::Instance { instance_id, .. } => {
                hash.update([2]);
                hash.update(instance_id);
            }
        }
        for value in [
            tile.generation.get(),
            tile.node_revision,
            tile.assignment_revision,
            tile.activation,
            tile.reset_epoch,
        ] {
            hash.update(value.to_be_bytes());
        }
        hash.update(tile.node);
        for pin in [tile.content, tile.client, tile.map, tile.frame] {
            hash.update(pin);
        }
        hash.update(&tile.target.spatial_position);
        hash.update(policy.definition_key.as_bytes());
        hash.update(policy.definition_revision.as_bytes());
        for (item, definition) in &tile.items {
            hash.update(item);
            hash.update(definition.production_key.as_bytes());
        }
        Some(Self {
            tile,
            actors: [actor],
            before,
            subject,
            movement: None,
            binding: hash.finalize().into(),
            remove_creature,
        })
    }

    pub fn actor(&self) -> ExactActorRef {
        self.actors[0]
    }
    pub fn binding(&self) -> [u8; 32] {
        self.binding
    }
    pub fn position(&self) -> MovementLocalPosition {
        self.tile.position
    }
    pub fn observed_us(&self) -> Option<u64> {
        self.movement.map(|movement| movement.observed_us())
    }
    pub fn definition_key(&self) -> &str {
        &self.subject.state.policy.definition_key
    }
    pub fn content_digest(&self) -> [u8; 32] {
        self.tile.content
    }
    pub fn removes_creature(&self) -> bool {
        self.remove_creature
    }
    pub fn item_count(&self) -> usize {
        self.tile
            .items
            .iter()
            .take_while(|(_, definition)| {
                SourceGroundStepKind::from_definition(definition)
                    == Some(SourceGroundStepKind::DeadAspect)
            })
            .count()
    }
    /// The original Item callback vector stops on Lua nil/false. Both
    /// Weeping and Remains return nil for a nonplayer; later33949 originals
    /// and the captured following callback must therefore remain untouched.
    pub fn stops_dispatch(&self) -> bool {
        self.item_count() != self.tile.items.len()
    }

    /// Called in the uninterrupted position-owner turn before any other source
    /// callback. The actual commit must match the preallocated original proof.
    pub fn bind_committed_movement(
        &mut self,
        runtime: &ChannelRuntimeV1,
        committed: CommittedMovementOccurrence,
    ) -> Option<()> {
        if self.movement.is_some()
            || committed.mover() != self.actor()
            || committed.before() != self.before
            || committed.after().position() != self.tile.position
            || committed.after().revision() != committed.after().last_translation_revision()
            || runtime.committed_movement_occurrence(self.actor()).ok()? != Some(committed)
        {
            return None;
        }
        self.subject.position = committed.after().position();
        self.subject.position_revision = committed.after().revision();
        self.movement = Some(committed);
        self.current_for(runtime).then_some(())
    }

    pub fn prepare_item_inputs(&self) -> Vec<PreparedSourceCreatureItem> {
        self.tile
            .items
            .iter()
            .take(self.item_count())
            .map(|(item, definition)| PreparedSourceCreatureItem {
                item: *item,
                definitions: vec![definition.clone()],
                encounter: TypedDefinitionRef {
                    family: "Encounter".into(),
                    production_key: "oteryn:encounter.soul_war_taint_zones".into(),
                    revision_ref: "definition-r1".into(),
                },
                target: self.tile.target.clone(),
                occurrence: String::with_capacity("source-ground-step:".len() + 64),
            })
            .collect()
    }
}

impl PreparedSourceCreatureItem {
    pub fn bind(self, proof: &SourceCreatureGroundStep) -> Option<SourceGroundStep> {
        use std::fmt::Write;
        let movement = proof.movement?;
        let Self {
            item,
            definitions,
            encounter,
            target,
            mut occurrence,
        } = self;
        let cast: [u8; 32] = Sha256::new()
            .chain_update(b"oteryn:source-ground-creature-step:v1")
            .chain_update(proof.binding)
            .chain_update(movement.binding())
            .chain_update(item)
            .finalize()
            .into();
        occurrence.push_str("source-ground-step:");
        for byte in cast {
            write!(&mut occurrence, "{byte:02x}").ok()?;
        }
        Some(SourceGroundStep {
            kind: SourceGroundStepKind::DeadAspect,
            cause: SourceGroundCause {
                scope: proof.tile.scope,
                generation: proof.tile.generation.get(),
                node: proof.tile.node,
                node_revision: proof.tile.node_revision,
                assignment_revision: proof.tile.assignment_revision,
                source_items: definitions,
                definition: encounter,
                occurrence,
                content: proof.tile.content,
                client: proof.tile.client,
                activation: proof.tile.activation,
                map: proof.tile.map,
                frame: proof.tile.frame,
                body: SOURCE_BODY,
                cast,
                actor: proof.actor(),
                placement: proof.actor().placement_identity(),
                subject: SourceGroundSubject::Creature(Arc::clone(&proof.subject.state.policy)),
                movement: movement.after(),
                committed: movement,
                reset_epoch: proof.tile.reset_epoch,
                item,
                target,
            },
        })
    }
}

impl source_reservation_seal::Sealed for SourceCreatureGroundStep {}
impl ScopeCreatureReservationProof for SourceCreatureGroundStep {
    fn scope(&self) -> (RuntimeScopeRefV1, ScopeOwnershipGeneration) {
        (self.tile.scope, self.tile.generation)
    }
    fn event_identity(&self) -> [u8; 16] {
        let mut event = [0; 16];
        event.copy_from_slice(&self.binding[..16]);
        event
    }
    fn source_binding_digest(&self) -> [u8; 32] {
        self.binding
    }
    fn affected_creatures(&self) -> &[ExactActorRef] {
        &self.actors
    }
    fn permits_non_leaf_item_only(&self) -> bool {
        !self.remove_creature
    }
    fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.tile.current_for(runtime)
            && runtime.validate_companion_snapshot(&self.subject).is_ok()
            && runtime.read_actor_position(self.actor()).ok()
                == Some(
                    self.movement
                        .map_or(self.before, |movement| movement.after()),
                )
            && self.movement.is_none_or(|movement| {
                runtime
                    .committed_movement_occurrence(self.actor())
                    .ok()
                    .flatten()
                    == Some(movement)
            })
    }
}
