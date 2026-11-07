//! Authorization captured synchronously from the existing native think batch.
use super::*;
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementPositionSnapshot, RuntimeScopeRefV1,
    RuntimeWorkStamp, ScopeOwnershipGeneration, ThinkOccurrence,
};
use oteryn_gameplay_entities::carrier::runtime_actor_source_reservation::{
    SourceActorReservationProof, source_reservation_seal,
};

#[derive(Debug, Clone)]
pub struct QualifiedRottenAreaSource {
    occurrence: ThinkOccurrence,
    key: String,
    position: [i32; 3],
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    activation: u64,
    pins: [[u8; 32]; 4],
    stamp: RuntimeWorkStamp,
    body: [u8; 32],
    frame: crate::content::PublishedWorldSourceFrame,
}
impl QualifiedRottenAreaSource {
    pub fn from_reserved_current(
        runtime: &mut ChannelRuntimeV1,
        native: &crate::content::native_gameplay::NativeGameplayState,
        room: &crate::content::QualifiedNativeEntryRoom,
        world: &crate::content::QualifiedPublishedWorldContext,
        occurrence: ThinkOccurrence,
    ) -> Option<Self> {
        if occurrence.sequence == 0
            || !world.matches(room, runtime.content_pin())
            || !native
                .qualifies_rotten_blood_callbacks(runtime.content_pin().server_artifact_digest())
            || !runtime.contains_live_creature(occurrence.actor)
        {
            return None;
        }
        let snapshot = runtime.companion_snapshot(occurrence.actor).ok()?;
        if !oteryn_gameplay_quests::quest::rotten_blood::area_icons::MINIONS
            .contains(&snapshot.state.policy.definition_key.as_str())
            || !native
                .policies()
                .policies()
                .iter()
                .any(|policy| policy.as_ref() == snapshot.state.policy.as_ref())
        {
            return None;
        }
        let at = runtime
            .read_actor_position(occurrence.actor)
            .ok()?
            .position();
        let position = world
            .source_frame()
            .source_position((at.x, at.y, at.floor))?;
        let stamp = runtime.issue_owner_work().ok()?;
        let pin = runtime.content_pin();
        Some(Self {
            occurrence,
            key: snapshot.state.policy.definition_key.clone(),
            position,
            scope: runtime.binding().runtime_scope(),
            generation: runtime.binding().scope_generation(),
            activation: pin.activation_sequence(),
            pins: [
                pin.server_artifact_digest(),
                pin.client_artifact_digest(),
                pin.frame_binding_digest(),
                runtime.map_revision_digest(),
            ],
            stamp,
            body: crate::content::native_rotten_blood_callbacks::area_icons_source_body_digest()?,
            frame: world.source_frame(),
        })
    }
    fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        let pin = runtime.content_pin();
        runtime
            .owner_fence()
            .is_ok_and(|fence| fence.accepts_stamp(self.stamp))
            && runtime.binding().runtime_scope() == self.scope
            && runtime.binding().scope_generation() == self.generation
            && pin.activation_sequence() == self.activation
            && self.pins
                == [
                    pin.server_artifact_digest(),
                    pin.client_artifact_digest(),
                    pin.frame_binding_digest(),
                    runtime.map_revision_digest(),
                ]
    }
    pub fn target(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        event: [u8; 16],
    ) -> Option<QualifiedRottenAreaTarget> {
        if !self.current_for(runtime) || SourceThinkOccurrence::from_bytes(event).is_none() {
            return None;
        }
        runtime.player_control_facts(actor, session).ok()?;
        let lease = runtime.bound_attacker_lease(actor, session).ok()?;
        let pose = runtime.read_actor_position(actor).ok()?;
        let at = pose.position();
        let position = self.frame.source_position((at.x, at.y, at.floor))?;
        if position[2] != self.position[2]
            || position[0].abs_diff(self.position[0]) > 15
            || position[1].abs_diff(self.position[1]) > 15
        {
            return None;
        }
        let mut hash = Sha256::new();
        hash.update(b"oteryn:rotten-area-native-target:v1");
        hash.update(event);
        hash.update(self.occurrence.actor.placement_identity());
        hash.update(self.occurrence.actor.actor_local_generation().to_be_bytes());
        hash.update(self.occurrence.sequence.to_be_bytes());
        hash.update(actor.placement_identity());
        hash.update(actor.actor_local_generation().to_be_bytes());
        hash.update(session.as_bytes());
        hash.update(lease.character_id().as_bytes());
        hash.update(lease.generation().to_be_bytes());
        hash.update(self.body);
        hash.update(self.scope.world_id().as_bytes());
        match self.scope {
            RuntimeScopeRefV1::Channel { channel_id, .. } => {
                hash.update([1]);
                hash.update(channel_id.as_bytes())
            }
            RuntimeScopeRefV1::Instance { instance_id, .. } => {
                hash.update([2]);
                hash.update(instance_id)
            }
        }
        hash.update(self.generation.get().to_be_bytes());
        hash.update(self.activation.to_be_bytes());
        for pin in self.pins {
            hash.update(pin)
        }
        hash.update(pose.revision().to_be_bytes());
        Some(QualifiedRottenAreaTarget {
            source: self.clone(),
            actor,
            session,
            character: *lease.character_id().as_bytes(),
            lease: lease.generation(),
            pose,
            event,
            binding: hash.finalize().into(),
            affected: [self.occurrence.actor],
        })
    }
    pub fn position(&self) -> [i32; 3] {
        self.position
    }
    pub fn occurrence(&self) -> ThinkOccurrence {
        self.occurrence
    }
}
#[derive(Debug, Clone)]
pub struct QualifiedRottenAreaTarget {
    source: QualifiedRottenAreaSource,
    actor: ExactActorRef,
    session: GameSessionId,
    character: [u8; 16],
    lease: u64,
    pose: MovementPositionSnapshot,
    event: [u8; 16],
    binding: [u8; 32],
    affected: [ExactActorRef; 1],
}
impl QualifiedRottenAreaTarget {
    pub fn actor(&self) -> ExactActorRef {
        self.actor
    }
    pub fn session(&self) -> GameSessionId {
        self.session
    }
    pub fn occurrence(&self) -> ThinkOccurrence {
        self.source.occurrence
    }
    pub fn event_identity(&self) -> [u8; 16] {
        self.event
    }
    pub fn source_position(&self) -> [i32; 3] {
        self.source.position
    }
    pub fn player_source_position(&self) -> Option<[i32; 3]> {
        let p = self.pose.position();
        self.source.frame.source_position((p.x, p.y, p.floor))
    }
    pub fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.source.current_for(runtime)
            && runtime.read_actor_position(self.actor) == Ok(self.pose)
            && runtime
                .bound_attacker_lease(self.actor, self.session)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == &self.character
                        && lease.generation() == self.lease
                })
            && runtime
                .companion_snapshot(self.source.occurrence.actor)
                .is_ok_and(|snapshot| {
                    snapshot.health > 0 && snapshot.state.policy.definition_key == self.source.key
                })
    }
}
impl source_reservation_seal::Sealed for QualifiedRottenAreaTarget {}
impl SourceActorReservationProof for QualifiedRottenAreaTarget {
    fn owner(&self) -> (ExactActorRef, GameSessionId) {
        (self.actor, self.session)
    }
    fn event_identity(&self) -> [u8; 16] {
        self.event
    }
    fn source_binding_digest(&self) -> [u8; 32] {
        self.binding
    }
    fn affected_actors(&self) -> &[ExactActorRef] {
        &self.affected
    }
    fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.current_for(runtime)
    }
}
impl QualifiedRottenAreaThink {
    pub fn from_reserved_target(
        runtime: &ChannelRuntimeV1,
        target: &QualifiedRottenAreaTarget,
        current: &super::current::CurrentRottenAreaPlayerFacts,
    ) -> Option<Self> {
        use oteryn_gameplay_quests::quest::rotten_blood::area_icons as rule;
        if !target.current_for(runtime)
            || !current.current_for(runtime)
            || current.taint().actor != target.actor
            || current.taint().session != target.session
            || current.fence().character_id.as_bytes() != &target.character
            || current.fence().character_lease_generation != target.lease
        {
            return None;
        }
        let [x, y, z] = target.player_source_position()?;
        let z = i16::try_from(z).ok()?;
        let clocks = current.clocks();
        let now = current.taint().observed_at_seconds;
        let mask = u8::from(now.saturating_sub(clocks[0]) >= rule::SECONDS[0])
            | (u8::from(
                rule::in_upper_left_source_cell(x, y, z)
                    && now.saturating_sub(clocks[1]) >= rule::SECONDS[1],
            ) << 1)
            | (u8::from(
                rule::in_lower_left_source_cell(x, y, z)
                    && now.saturating_sub(clocks[2]) >= rule::SECONDS[2],
            ) << 2);
        if mask == 0 {
            return None;
        }
        let mut hash = Sha256::new();
        hash.update(target.binding);
        hash.update(now.to_be_bytes());
        for value in clocks {
            hash.update(value.to_be_bytes())
        }
        Some(Self {
            fence: current.fence(),
            cause: SourceThinkOccurrence::from_bytes(target.event)?,
            mask,
            clocks_before: clocks,
            original_binding: hash.finalize().into(),
        })
    }
}
