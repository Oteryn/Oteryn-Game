//! A physical push never restores the pushed player's input authority.
use super::*;
use std::sync::Arc;

#[derive(Debug)]
pub(super) struct ForcedPlayerStep {
    pusher: (ExactActorRef, GameSessionId, MovementPositionSnapshot),
    target: (ExactActorRef, GameSessionId, MovementPositionSnapshot),
    leases: (CharacterLease, CharacterLease),
    delta: (i32, i32),
}

/// Dropping the caller's turn, including cancellation, revokes its push proof.
pub struct ForcedPlayerStepGuard {
    _hold: Arc<ForcedPlayerStep>,
}

impl ForcedPlayerStep {
    fn current(&self, runtime: &ChannelRuntimeV1) -> bool {
        let (pusher, source_session, source_at) = self.pusher;
        let (target, target_session, target_at) = self.target;
        runtime.owner_fence().is_ok()
            && runtime
                .player_control_facts(pusher, source_session)
                .is_ok_and(|facts| facts.control_loss.is_none())
            && runtime.player_control_facts(target, target_session).is_ok()
            && runtime.bound_attacker_lease(pusher, source_session) == Ok(self.leases.0)
            && runtime.bound_attacker_lease(target, target_session) == Ok(self.leases.1)
            && runtime.read_actor_position(pusher) == Ok(source_at)
            && runtime.read_actor_position(target) == Ok(target_at)
            && source_at.context() == runtime.pinned_movement_context()
            && target_at.context() == runtime.pinned_movement_context()
    }
}

impl ChannelRuntimeV1 {
    /// Qualify one cardinal physical push from an independently controlled player.
    pub fn prepare_player_push(
        &mut self,
        pusher: (ExactActorRef, GameSessionId, MovementPositionSnapshot),
        target: (ExactActorRef, GameSessionId, MovementPositionSnapshot),
        destination: MovementLocalPosition,
    ) -> Result<ForcedPlayerStepGuard, CarrierError> {
        let from = target.2.position();
        let source = pusher.2.position();
        let dx = destination
            .x
            .checked_sub(from.x)
            .ok_or(CarrierError::PlayerReservationMismatch)?;
        let dy = destination
            .y
            .checked_sub(from.y)
            .ok_or(CarrierError::PlayerReservationMismatch)?;
        if pusher.0 == target.0
            || destination.floor != from.floor
            || !matches!((dx, dy), (0, -1) | (1, 0) | (0, 1) | (-1, 0))
            || source.floor != from.floor
            || source.x.abs_diff(from.x) > 1
            || source.y.abs_diff(from.y) > 1
        {
            return Err(CarrierError::PlayerReservationMismatch);
        }
        let proof = ForcedPlayerStep {
            pusher,
            target,
            leases: (
                self.bound_attacker_lease(pusher.0, pusher.1)?,
                self.bound_attacker_lease(target.0, target.1)?,
            ),
            delta: (dx, dy),
        };
        if !proof.current(self) {
            return Err(CarrierError::PlayerReservationMismatch);
        }
        // Retain only current witnesses: at most one per live player slot.
        let mut held = std::mem::take(&mut self.forced_player_steps);
        held.retain(|weak| weak.upgrade().is_some_and(|proof| proof.current(self)));
        self.forced_player_steps = held;
        if self
            .forced_player_steps
            .iter()
            .filter_map(std::sync::Weak::upgrade)
            .any(|held| held.target.0 == target.0)
        {
            return Err(CarrierError::PlayerReservationMismatch);
        }
        self.forced_player_steps
            .try_reserve(1)
            .map_err(|_| CarrierError::CapacityExceeded)?;
        let proof = Arc::new(proof);
        self.forced_player_steps.push(Arc::downgrade(&proof));
        Ok(ForcedPlayerStepGuard { _hold: proof })
    }

    /// Read-only physical witness; it grants neither commands nor damage writes.
    pub fn forced_player_step_current(&self, actor: ExactActorRef, session: GameSessionId) -> bool {
        self.forced_player_steps
            .iter()
            .filter_map(std::sync::Weak::upgrade)
            .any(|proof| {
                proof.target.0 == actor && proof.target.1 == session && proof.current(self)
            })
    }

    /// Input still requires control; only the exact live push witness can move grace.
    pub fn player_step_allowed(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        delta: (i32, i32),
    ) -> bool {
        if let Some(proof) = self
            .forced_player_steps
            .iter()
            .filter_map(std::sync::Weak::upgrade)
            .find(|proof| proof.target.0 == actor && proof.target.1 == session)
        {
            return proof.delta == delta && proof.current(self);
        }
        self.player_control_facts(actor, session)
            .is_ok_and(|facts| facts.control_loss.is_none())
    }
}
