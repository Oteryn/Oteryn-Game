//! The existing player lifecycle while the slot belongs to a named House.
//! Each entry proves current House custody before touching the native slot.
use super::*;

impl ChannelRuntimeV1 {
    /// Install only the durable admission selection on this actual House slot.
    /// Hazard combat continues to use the existing Channel-only reader.
    pub fn install_house_hazard_level(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
        level: u16,
    ) -> Result<(), CarrierError> {
        if !(1..=12).contains(&level) {
            return Err(CarrierError::InvalidCreatureTarget);
        }
        let index = self.house_lifecycle_index(actor, session)?;
        self.carrier.assert_slot_spell_unreserved(index)?;
        let Slot::Occupied { spell_combat, .. } = &mut self.carrier.slots[index] else {
            return Err(CarrierError::NotCreature);
        };
        spell_combat.hazard = Some((session, level));
        Ok(())
    }

    fn house_lifecycle_index(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Result<usize, CarrierError> {
        self.house_actor_state(actor, session)?;
        usize::try_from(actor.actor_local_id())
            .ok()
            .and_then(|index| index.checked_sub(1))
            .ok_or(CarrierError::InvalidActorIdentity)
    }

    /// The same native spell reservations remain exclusive while the actor is
    /// in a House. The generic Channel reservation gate stays unchanged.
    pub fn assert_house_actor_spell_unreserved(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Result<(), CarrierError> {
        let index = self.house_lifecycle_index(actor, session)?;
        self.carrier.assert_slot_spell_unreserved(index)
    }

    /// Captures may observe a fenced resident, but input owners must refuse
    /// writes until the exact native transition settles.
    pub fn house_player_writes_fenced(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Result<bool, CarrierError> {
        let index = self.house_lifecycle_index(actor, session)?;
        self.carrier
            .attackers
            .iter()
            .find(|entry| {
                entry.index == index && entry.generation == actor.actor_local_generation()
            })
            .map(|entry| entry.authority.fence.is_some())
            .ok_or(CarrierError::SupersededAttackerSession)
    }

    /// Mirror the committed loss on the existing slot using the same native
    /// epoch conflict and reservation rules as the Channel lifecycle.
    pub fn record_house_control_loss(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
        mark: ControlLossMark,
    ) -> Result<(), CarrierError> {
        let index = self.house_lifecycle_index(actor, session)?;
        self.carrier.record_player_control_loss_at(index, mark)
    }

    /// A genuine CompleteReconnect COMMIT restores the same actor and the
    /// existing bounded reentry window. Retrying that epoch cannot extend it.
    pub fn restore_house_control_after_committed_reentry(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
        epoch: u64,
        now_us: u64,
    ) -> Result<(), CarrierError> {
        let index = self.house_lifecycle_index(actor, session)?;
        self.carrier
            .restore_control_after_committed_reentry_at(index, epoch, now_us)?;
        self.carrier.lift_player_fence_at(
            index,
            actor.actor_local_generation(),
            WriteFenceToken::ControlLoss(epoch),
        )?;
        Ok(())
    }

    /// Install or join the exact native transition token under House custody.
    pub fn fence_house_player_writes(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
        token: WriteFenceToken,
    ) -> Result<WriteFenceSet, CarrierError> {
        let index = self.house_lifecycle_index(actor, session)?;
        self.carrier
            .fence_player_writes_at(index, actor.actor_local_generation(), token)
    }

    /// A different token cannot lift a House actor's current native write fence.
    pub fn lift_house_player_fence(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
        token: WriteFenceToken,
    ) -> Result<bool, CarrierError> {
        let index = self.house_lifecycle_index(actor, session)?;
        self.carrier
            .lift_player_fence_at(index, actor.actor_local_generation(), token)
    }

    /// Exact native retirement after the composition proves durable TERMINAL.
    /// Companion cleanup, reservations, generation reuse and occupied indexes
    /// remain the existing carrier owners; no Channel placement is fabricated.
    pub fn remove_terminal_house_session(
        &mut self,
        session: GameSessionId,
        actor: ExactActorRef,
    ) -> Result<(), CarrierError> {
        let state = self.house_actor_state(actor, session)?;
        let index = self.house_lifecycle_index(actor, session)?;
        self.carrier.assert_slot_spell_unreserved(index)?;
        self.remove_companions_of_proven_player(actor, session)?;
        let (_, removed) =
            self.carrier
                .remove_keeping_indexes_at(&self.continuity, actor.0, index)?;
        self.carrier.unindex_slot(removed);
        self.carrier
            .house_custody
            .retain(|resident| resident.actor != actor || resident.session != session);
        if !self.carrier.house_custody.iter().any(|resident| {
            resident.key == state.house
                && resident.scope == state.scope
                && resident.generation == state.generation
        }) {
            self.retire_house_hireling_projection(&state.house, state.scope, state.generation);
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::carrier::tests::{runtime, session};

    fn uuid(seed: u8) -> [u8; 16] {
        let mut value = [0; 16];
        value[6] = 0x70;
        value[8] = 0x80;
        value[15] = seed;
        value
    }

    fn resident(runtime: &mut ChannelRuntimeV1, seed: u8) -> (ExactActorRef, GameSessionId) {
        let session = session(u64::from(seed));
        let lease = CharacterLease::new(CharacterId::decode(&uuid(seed)).expect("character"), 1)
            .expect("lease");
        let position = LocalPosition {
            x: i32::from(seed),
            y: 1,
            floor: 7,
        };
        let state = HouseActorState {
            house: "house:lifecycle".into(),
            scope: RuntimeScopeRefV1::Instance {
                world_id: runtime.binding.world_id,
                instance_id: uuid(80),
            },
            generation: ScopeOwnershipGeneration::new(1).expect("generation"),
            position,
            lease,
            origin_position: LocalPosition {
                x: 0,
                y: 0,
                floor: 7,
            },
        };
        let actor = runtime
            .restore_transfer_actor(
                crate::carrier::tests::session(100 + u64::from(seed)),
                session,
                lease,
                position,
                Some(state),
            )
            .expect("House resident");
        (actor, session)
    }

    #[test]
    fn house_loss_reentry_preserves_native_epoch_fence_and_deadline_rules() {
        let mut runtime = runtime(5);
        let (actor, session) = resident(&mut runtime, 40);
        let mark = ControlLossMark {
            epoch: 1,
            grace_deadline: 90,
        };
        assert!(runtime.player_control_facts(actor, session).is_err());
        assert!(runtime.record_control_loss(actor, session, mark).is_err());
        runtime
            .record_house_control_loss(actor, session, mark)
            .expect("loss");
        runtime
            .record_house_control_loss(actor, session, mark)
            .expect("same decision");
        assert!(
            runtime
                .record_house_control_loss(
                    actor,
                    session,
                    ControlLossMark {
                        grace_deadline: 91,
                        ..mark
                    }
                )
                .is_err()
        );
        let token = ChannelRuntimeV1::grace_expiry_fence(1);
        assert_eq!(
            runtime.fence_house_player_writes(actor, session, token),
            Ok(WriteFenceSet::Fenced)
        );
        assert_eq!(
            runtime.fence_house_player_writes(actor, session, token),
            Ok(WriteFenceSet::Joined)
        );
        assert_eq!(
            runtime.fence_house_player_writes(
                actor,
                session,
                ChannelRuntimeV1::transition_fence(2)
            ),
            Err(CarrierError::WriteFenceBusy)
        );
        assert_eq!(
            runtime.lift_house_player_fence(actor, session, ChannelRuntimeV1::transition_fence(2)),
            Ok(false)
        );
        assert_eq!(runtime.house_player_writes_fenced(actor, session), Ok(true));
        assert!(
            runtime
                .restore_house_control_after_committed_reentry(actor, session, 2, 100)
                .is_err()
        );
        runtime
            .restore_house_control_after_committed_reentry(actor, session, 1, 100)
            .expect("reentry");
        assert_eq!(
            runtime
                .house_player_control_facts(actor, session)
                .expect("facts")
                .control_loss,
            None
        );
        assert_eq!(
            runtime.house_player_writes_fenced(actor, session),
            Ok(false)
        );
        runtime
            .restore_house_control_after_committed_reentry(actor, session, 1, 3_000_000)
            .expect("idempotent reentry");
        let index = runtime
            .house_lifecycle_index(actor, session)
            .expect("current slot");
        let reentry = match &runtime.carrier.slots[index] {
            Slot::Occupied { spell_combat, .. } => Some(spell_combat.reentry),
            _ => None,
        }
        .expect("resident slot");
        assert_eq!(
            reentry,
            Some((1, 4_000_100)),
            "retry never extends protection"
        );
        assert!(
            runtime.player_control_facts(actor, session).is_err(),
            "no Channel admission was added"
        );
    }

    #[test]
    fn house_lifecycle_rechecks_owner_and_session_before_touching_a_slot() {
        let mut runtime = runtime(5);
        let (actor, held) = resident(&mut runtime, 40);
        let other = session(41);
        let token = ChannelRuntimeV1::transition_fence(1);
        assert!(
            runtime
                .fence_house_player_writes(actor, other, token)
                .is_err()
        );
        assert!(runtime.remove_terminal_house_session(other, actor).is_err());
        assert_eq!(runtime.house_player_writes_fenced(actor, held), Ok(false));
        runtime
            .house_owners
            .get_mut("house:lifecycle")
            .expect("owner")
            .generation = ScopeOwnershipGeneration::new(2).expect("successor owner");
        assert!(
            runtime
                .record_house_control_loss(
                    actor,
                    held,
                    ControlLossMark {
                        epoch: 1,
                        grace_deadline: 90
                    }
                )
                .is_err()
        );
        assert!(runtime.remove_terminal_house_session(held, actor).is_err());
    }

    #[test]
    fn terminal_house_cleanup_keeps_other_residents_and_reuses_generation_safely() {
        let mut runtime = runtime(5);
        let (old, ended) = resident(&mut runtime, 40);
        let (other, held) = resident(&mut runtime, 41);
        runtime
            .fence_house_player_writes(old, ended, ChannelRuntimeV1::transition_fence(1))
            .expect("terminal transition fence");
        runtime
            .remove_terminal_house_session(ended, old)
            .expect("proven terminal cleanup");
        assert!(runtime.house_actor_state(old, ended).is_err());
        assert!(runtime.house_actor_state(other, held).is_ok());
        let (successor, session) = resident(&mut runtime, 42);
        assert_eq!(successor.actor_local_id(), old.actor_local_id());
        assert!(successor.actor_local_generation() > old.actor_local_generation());
        assert!(runtime.remove_terminal_house_session(ended, old).is_err());
        assert!(runtime.house_actor_state(successor, session).is_ok());
    }
}
