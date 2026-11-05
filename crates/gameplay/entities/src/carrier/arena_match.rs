//! SV15 uses the existing qualified native creature and exact position owners.
use super::*;

impl ChannelRuntimeV1 {
    pub fn spawn_arena_opponent(
        &mut self,
        creature: &str,
        at: MovementLocalPosition,
        map_admits: impl FnOnce(&ChannelRuntimeV1, MovementLocalPosition) -> Option<bool>,
    ) -> Result<ExactActorRef, EncounterSpawnError> {
        self.owner_fence().map_err(EncounterSpawnError::Native)?;
        let policy = self
            .companion_policy(creature)
            .map_err(EncounterSpawnError::Native)?;
        let revision = policy.definition_revision.clone();
        self.admit_native_masterless_at(
            creature,
            &revision,
            self.content_pin().server_artifact_digest(),
            at,
            map_admits,
        )
    }

    /// An administrative arena exit is not a creature death or reward source.
    pub fn remove_arena_opponent(&mut self, actor: ExactActorRef) -> Result<(), CarrierError> {
        self.owner_fence()?;
        self.carrier.validate_ref(&self.continuity, actor.0)?;
        self.companion_snapshot_including_dead(actor)?;
        self.carrier.remove(&self.continuity, actor.0).map(|_| ())
    }

    /// The caller proves the donor destination against the current map under this same lock.
    pub fn relocate_arena_player(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
        expected: MovementPositionSnapshot,
        at: MovementLocalPosition,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        let facts = self.player_control_facts(actor, session)?;
        if facts.control_loss.is_some() || expected.0.actor_ref != actor.0 {
            return Err(CarrierError::PositionSnapshotMismatch);
        }
        self.carrier.compare_commit_position(
            &self.continuity,
            expected.0,
            expected.0.version.context,
            LocalPosition {
                x: at.x,
                y: at.y,
                floor: at.floor,
            },
        )?;
        Ok(())
    }
}
