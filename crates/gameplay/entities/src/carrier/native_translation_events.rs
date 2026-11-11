//! Committed source onMove observations in the existing position owner.
//! Records are not movement, Item or actor-admission grants.
use super::*;
use oteryn_gameplay_content::content::creature_policy::CompiledCreaturePolicy;
use oteryn_gameplay_content::content::{
    ProjectV2DefinitionRef, QualifiedNativeEntryRoom,
    bosses::activity_template::PinnedActivitySourceRoom as Room,
    native_gameplay::NativeGameplayState,
};
use oteryn_gameplay_foundation::foundation::RuntimeWorkStamp;
use sha2::{Digest, Sha256};

/// Immutable DATA from the one successful position commit. It grants neither
/// source callback authority nor a second relocation. Source families qualify
/// their own callbacks against this receipt and retain it across later teleports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommittedMovementOccurrence {
    mover: ExactActorRef,
    before: MovementPositionSnapshot,
    after: MovementPositionSnapshot,
    observed_us: u64,
    binding: [u8; 32],
}
impl CommittedMovementOccurrence {
    pub fn mover(self) -> ExactActorRef {
        self.mover
    }
    pub fn before(self) -> MovementPositionSnapshot {
        self.before
    }
    pub fn after(self) -> MovementPositionSnapshot {
        self.after
    }
    pub fn observed_us(self) -> u64 {
        self.observed_us
    }
    pub fn binding(self) -> [u8; 32] {
        self.binding
    }
    pub(super) fn from_commit(
        before: PositionSnapshot,
        after: VersionedPosition,
        observed_us: u64,
    ) -> Self {
        let mover = ExactActorRef(before.actor_ref);
        let (scope_kind, scope_identity) = match mover.runtime_scope() {
            RuntimeScopeRefV1::Channel { channel_id, .. } => (1u8, *channel_id.as_bytes()),
            RuntimeScopeRefV1::Instance { instance_id, .. } => (2u8, instance_id),
        };
        let binding = Sha256::new()
            .chain_update(b"oteryn:committed-movement-occurrence:v1")
            .chain_update(mover.world_id().as_bytes())
            .chain_update([scope_kind])
            .chain_update(scope_identity)
            .chain_update(mover.scope_generation().get().to_be_bytes())
            .chain_update(mover.placement_identity())
            .chain_update(mover.actor_local_generation().to_be_bytes())
            .chain_update(before.version.revision.to_be_bytes())
            .chain_update(after.revision.to_be_bytes())
            .chain_update(before.version.position.x.to_be_bytes())
            .chain_update(before.version.position.y.to_be_bytes())
            .chain_update(before.version.position.floor.to_be_bytes())
            .chain_update(after.position.x.to_be_bytes())
            .chain_update(after.position.y.to_be_bytes())
            .chain_update(after.position.floor.to_be_bytes())
            .finalize()
            .into();
        Self {
            mover,
            before: MovementPositionSnapshot(before),
            after: MovementPositionSnapshot(PositionSnapshot {
                actor_ref: before.actor_ref,
                version: after,
            }),
            observed_us,
            binding,
        }
    }
}

const MAX_SOURCES: usize = 16;
const MAX_PENDING: usize = 1024;
#[derive(Debug, Clone, PartialEq, Eq)]
struct Source {
    actor: ExactActorRef,
    creature: ProjectV2DefinitionRef,
    policy: Arc<CompiledCreaturePolicy>,
    stamp: RuntimeWorkStamp,
    activation: u64,
    server: [u8; 32],
    client: [u8; 32],
    frame: [u8; 32],
    map: [u8; 32],
    body: [u8; 32],
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Registration {
    source: Arc<Source>,
    observed_us: u64,
}
impl Registration {
    pub(super) fn actor(&self) -> ActorRef {
        self.source.actor.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedRottenTranslation {
    source: Arc<Source>,
    movement: CommittedMovementOccurrence,
    player: bool,
    occurrence: [u8; 32],
}
impl CommittedRottenTranslation {
    pub fn movement(&self) -> CommittedMovementOccurrence {
        self.movement
    }
    pub fn observer(&self) -> ExactActorRef {
        self.source.actor
    }
    pub fn mover(&self) -> ExactActorRef {
        self.movement.mover()
    }
    pub fn is_player(&self) -> bool {
        self.player
    }
    pub fn before(&self) -> MovementPositionSnapshot {
        self.movement.before()
    }
    pub fn after(&self) -> MovementPositionSnapshot {
        self.movement.after()
    }
    pub fn observed_us(&self) -> u64 {
        self.movement.observed_us()
    }
    pub fn creature(&self) -> &ProjectV2DefinitionRef {
        &self.source.creature
    }
    pub fn source_body(&self) -> [u8; 32] {
        self.source.body
    }
    pub fn occurrence(&self) -> [u8; 32] {
        self.occurrence
    }
    pub fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        let pin = runtime.content_pin();
        runtime
            .owner_fence()
            .is_ok_and(|f| f.accepts_stamp(self.source.stamp))
            && runtime.binding().runtime_scope() == self.source.actor.runtime_scope()
            && runtime.binding().scope_generation() == self.source.actor.scope_generation()
            && pin.activation_sequence() == self.source.activation
            && pin.server_artifact_digest() == self.source.server
            && pin.client_artifact_digest() == self.source.client
            && pin.frame_binding_digest() == self.source.frame
            && pin.map_revision_digest() == self.source.map
    }
    pub fn server_digest(&self) -> [u8; 32] {
        self.source.server
    }
    pub fn client_digest(&self) -> [u8; 32] {
        self.source.client
    }
    pub fn frame_digest(&self) -> [u8; 32] {
        self.source.frame
    }
    pub fn map_digest(&self) -> [u8; 32] {
        self.source.map
    }
}

impl ChannelRuntimeV1 {
    pub fn register_rotten_movement_source(
        &mut self,
        native: &NativeGameplayState,
        room: &QualifiedNativeEntryRoom,
        actor: ExactActorRef,
        now: u64,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        let snapshot = self.companion_snapshot(actor)?;
        let key = &snapshot.state.policy.definition_key;
        let expected_room = match key.as_str() {
            "oteryn:creature.vemiath" => Room::Vemiath,
            "oteryn:creature.chagorz" => Room::Chagorz,
            "oteryn:creature.bakragore" => Room::Bakragore,
            _ => return Err(CarrierError::InvalidCreatureTarget),
        };
        let pin = self.content_pin();
        if !matches!(
            self.binding().runtime_scope(),
            RuntimeScopeRefV1::Instance { .. }
        ) || room.pinned_activity_source_room() != Some(expected_room)
            || room.frame_binding().digest() != pin.frame_binding_digest()
            || room.map_revision_digest() != pin.map_revision_digest()
            || native.source_digest() != pin.server_artifact_digest()
            || !native.qualifies_rotten_blood_callbacks(pin.server_artifact_digest())
            || snapshot.state.master.is_some()
            || self.native_summon_master(actor).is_some()
        {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        let policy = native
            .policies()
            .policies()
            .iter()
            .find(|policy| {
                policy.definition_key == *key
                    && policy.definition_revision == snapshot.state.policy.definition_revision
            })
            .ok_or(CarrierError::InvalidCreatureTarget)?;
        if policy.as_ref() != snapshot.state.policy.as_ref() {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        let record = native
            .creature_profiles()
            .records
            .iter()
            .find(|record| {
                record.profile.target.key == *key
                    && record.profile.target.revision == policy.definition_revision
            })
            .ok_or(CarrierError::InvalidCreatureTarget)?;
        let body = Sha256::digest(
            serde_json::to_vec(record).map_err(|_| CarrierError::InvalidCommitBinding)?,
        )
        .into();
        let creature = record.profile.target.clone();
        let policy = Arc::clone(policy);
        let activation = pin.activation_sequence();
        let (server, client, frame, map) = (
            pin.server_artifact_digest(),
            pin.client_artifact_digest(),
            pin.frame_binding_digest(),
            pin.map_revision_digest(),
        );
        let stamp = self.issue_owner_work()?;
        if self
            .carrier
            .native_move_sources
            .iter()
            .any(|s| s.source.actor == actor)
        {
            return Ok(());
        }
        if self.carrier.native_move_sources.len() >= MAX_SOURCES {
            return Err(CarrierError::CapacityExceeded);
        }
        self.carrier
            .native_move_sources
            .try_reserve(1)
            .map_err(|_| CarrierError::CapacityExceeded)?;
        self.carrier.native_move_sources.push(Registration {
            source: Arc::new(Source {
                actor,
                creature,
                policy,
                stamp,
                activation,
                server,
                client,
                frame,
                map,
                body,
            }),
            observed_us: now,
        });
        Ok(())
    }
    /// Observation metadata from the EXISTING owner clock, not a pace/timer authority.
    pub fn note_rotten_movement_time(&mut self, now: u64) -> Result<(), CarrierError> {
        self.owner_fence()?;
        if now < self.carrier.movement_observed_us {
            return Err(CarrierError::PlanConflict);
        }
        self.carrier.movement_observed_us = now;
        if self
            .carrier
            .native_move_sources
            .iter()
            .any(|s| now < s.observed_us)
        {
            return Err(CarrierError::PlanConflict);
        }
        for source in &mut self.carrier.native_move_sources {
            source.observed_us = now;
        }
        Ok(())
    }
    /// Capture the same canonical movement DATA in a source-specific callback.
    /// Later teleports may replace this lookup; retained copies remain unchanged.
    pub fn committed_movement_occurrence(
        &self,
        actor: ExactActorRef,
    ) -> Result<Option<CommittedMovementOccurrence>, CarrierError> {
        self.owner_fence()?;
        self.carrier.validate_ref(&self.continuity, actor.0)?;
        Ok(self
            .carrier
            .movement_latest
            .iter()
            .find(|event| event.mover == actor)
            .copied())
    }
    pub fn note_committed_movement_time(&mut self, now: u64) -> Result<(), CarrierError> {
        self.note_rotten_movement_time(now)
    }
    #[cfg(feature = "test-support")]
    pub fn rotten_movement_diagnostics_for_test(&self, mover: ExactActorRef) -> Vec<String> {
        let mut rows = vec![format!(
            "sources={} pending={} mover={:?} latest={:?}",
            self.carrier.native_move_sources.len(),
            self.carrier.native_move_events.len(),
            self.read_actor_position(mover),
            self.committed_movement_occurrence(mover)
        )];
        for source in &self.carrier.native_move_sources {
            let policy_match = self
                .companion_snapshot_including_dead(source.source.actor)
                .map(|snapshot| snapshot.state.policy.as_ref() == source.source.policy.as_ref());
            let context_match = self
                .read_actor_position(source.source.actor)
                .ok()
                .zip(self.read_actor_position(mover).ok())
                .map(|(a, b)| a.0.version.context == b.0.version.context);
            rows.push(format!(
                "policy_match={policy_match:?} context_match={context_match:?}"
            ));
            rows.push(format!(
                "source={} actor={:?} pose={:?} current_slot={:?} scope={:?}",
                source.source.creature.key,
                source.source.actor,
                self.read_actor_position(source.source.actor),
                self.carrier
                    .validate_ref(&self.continuity, source.source.actor.0),
                self.binding().runtime_scope()
            ));
        }
        rows
    }
    pub fn rotten_movement_pending(&self) -> bool {
        !self.carrier.native_move_events.is_empty()
    }
    pub fn pending_rotten_movement(
        &self,
        limit: usize,
    ) -> Result<Vec<CommittedRottenTranslation>, CarrierError> {
        self.owner_fence()?;
        Ok(self
            .carrier
            .native_move_events
            .iter()
            .take(limit.min(MAX_PENDING))
            .cloned()
            .collect())
    }
    pub fn acknowledge_rotten_movement(
        &mut self,
        event: &CommittedRottenTranslation,
    ) -> Result<(), CarrierError> {
        if !event.current_for(self) {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        let Some(index) = self
            .carrier
            .native_move_events
            .iter()
            .position(|pending| pending == event)
        else {
            return Ok(());
        };
        self.carrier.native_move_events.remove(index);
        Ok(())
    }
}
fn observed(observer: LocalPosition, before: LocalPosition, after: LocalPosition) -> bool {
    // These three sealed Instance maps have only source floor15. Normal steps
    // use CPP's expanded old rectangle; nonadjacent moves use old/new union.
    let in_box = |point: LocalPosition| {
        observer.floor == point.floor
            && observer.x.abs_diff(point.x) <= 11
            && observer.y.abs_diff(point.y) <= 11
    };
    if before.floor == after.floor
        && before.x.abs_diff(after.x) <= 1
        && before.y.abs_diff(after.y) <= 1
    {
        observer.floor == before.floor
            && i64::from(observer.x) >= i64::from(before.x.min(after.x)) - 11
            && i64::from(observer.x) <= i64::from(before.x.max(after.x)) + 11
            && i64::from(observer.y) >= i64::from(before.y.min(after.y)) - 11
            && i64::from(observer.y) <= i64::from(before.y.max(after.y)) + 11
    } else {
        in_box(before) || in_box(after)
    }
}
impl ChannelActorCarrier {
    pub(super) fn stage_rotten_translation(
        &mut self,
        continuity: &NamespaceContinuityGuard,
        before: PositionSnapshot,
        after: VersionedPosition,
        movement: Option<CommittedMovementOccurrence>,
    ) -> Result<Vec<CommittedRottenTranslation>, CarrierError> {
        if before.version.position == after.position || self.native_move_sources.is_empty() {
            return Ok(Vec::new());
        }
        if !self.native_move_events.is_empty() {
            return Err(CarrierError::PlanConflict);
        }
        let movement = movement.ok_or(CarrierError::PlanConflict)?;
        let mover_index = self.validate_ref(continuity, before.actor_ref)?;
        let player = matches!(
            &self.slots[mover_index],
            Slot::Occupied {
                committed: true,
                game_session_id: Some(_),
                ..
            }
        );
        let mover = ExactActorRef(before.actor_ref);
        let mut staged = Vec::new();
        staged
            .try_reserve(self.native_move_sources.len())
            .map_err(|_| CarrierError::CapacityExceeded)?;
        for registration in &self.native_move_sources {
            let source = &registration.source;
            if !player
                && !(source.actor == mover && source.creature.key == "oteryn:creature.bakragore")
            {
                continue;
            }
            let Ok(index) = self.validate_ref(continuity, source.actor.0) else {
                continue;
            };
            let Slot::CreatureOccupied {
                companion: Some(state),
                position: Some(at),
                ..
            } = &self.slots[index]
            else {
                continue;
            };
            // HP0 is still a real spectator until canonical removal completes.
            if state.policy.as_ref() != source.policy.as_ref()
                || at.context != before.version.context
                || !observed(at.position, before.version.position, after.position)
            {
                continue;
            }
            let occurrence = Sha256::new()
                .chain_update(b"oteryn:rotten-committed-translation:v1")
                .chain_update(source.actor.placement_identity())
                .chain_update(source.actor.actor_local_generation().to_be_bytes())
                .chain_update(mover.placement_identity())
                .chain_update(mover.actor_local_generation().to_be_bytes())
                .chain_update(after.revision.to_be_bytes())
                .chain_update(source.server)
                .finalize()
                .into();
            staged.push(CommittedRottenTranslation {
                source: Arc::clone(source),
                movement,
                player,
                occurrence,
            });
        }
        if self.native_move_events.len().saturating_add(staged.len()) > MAX_PENDING {
            return Err(CarrierError::CapacityExceeded);
        }
        self.native_move_events
            .try_reserve(staged.len())
            .map_err(|_| CarrierError::CapacityExceeded)?;
        Ok(staged)
    }
}

#[cfg(test)]
mod spectator_tests {
    use super::*;
    fn at(x: i32, y: i32) -> LocalPosition {
        LocalPosition { x, y, floor: 15 }
    }
    #[test]
    fn diagonal_translation_includes_expanded_old_cpp_spectator_rectangle() {
        let old = at(100, 100);
        let new = at(101, 101);
        assert!(observed(at(89, 112), old, new));
        assert!(observed(at(112, 89), old, new));
        assert!(!observed(at(88, 112), old, new));
        assert!(!observed(at(112, 88), old, new));
    }
    #[test]
    fn distant_translation_uses_old_new_union_and_excludes_gap() {
        let old = at(100, 100);
        let new = at(150, 150);
        assert!(observed(at(89, 89), old, new));
        assert!(observed(at(161, 161), old, new));
        assert!(!observed(at(125, 125), old, new));
        assert!(!observed(
            LocalPosition {
                x: 100,
                y: 100,
                floor: 14
            },
            old,
            new
        ));
    }
}
