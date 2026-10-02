//! DISCONNECT_REENTRY_PVE_PROTECTION_OWNER_DECISION: a committed unexpected-loss
//! recovery grants four seconds on the actual owner clock. Fresh admission has
//! no recovery event and therefore no window. This data lives in its actor slot.
use super::{CarrierError, ChannelRuntimeV1, ExactActorRef, GameSessionId, Slot};

impl ChannelRuntimeV1 {
    pub(crate) fn current_player_reentry_protection(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        now_us: u64,
    ) -> Result<bool, CarrierError> {
        let index = self
            .carrier
            .player_slot_index(&self.continuity, actor.0, session)?;
        match &self.carrier.slots[index] {
            Slot::Occupied {
                control_loss: None,
                spell_combat,
                ..
            } => Ok(spell_combat
                .reentry
                .is_some_and(|(_, until)| now_us < until)),
            _ => Err(CarrierError::ControlLossConflict),
        }
    }

    /// The transport invokes this only after its genuine CompleteReconnect COMMIT.
    /// A matching actual loss is required to start a window; retries retain its
    /// original deadline and cannot extend the benefit or invent another epoch.
    pub(crate) fn restore_control_after_committed_reentry(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
        epoch: u64,
        now_us: u64,
    ) -> Result<(), CarrierError> {
        let index = self
            .carrier
            .player_slot_index(&self.continuity, actor.0, session)?;
        self.carrier.assert_slot_spell_unreserved(index)?;
        let until = now_us
            .checked_add(4_000_000)
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        match &mut self.carrier.slots[index] {
            Slot::Occupied {
                control_loss,
                spell_combat,
                ..
            } if epoch != 0
                && control_loss.is_some_and(|mark| mark.epoch == epoch)
                && spell_combat
                    .reentry
                    .is_none_or(|(previous, _)| epoch > previous) =>
            {
                *control_loss = None;
                spell_combat.reentry = Some((epoch, until));
                Ok(())
            }
            Slot::Occupied {
                control_loss: None,
                spell_combat,
                ..
            } if spell_combat
                .reentry
                .is_some_and(|(previous, _)| previous == epoch) =>
            {
                Ok(())
            }
            _ => Err(CarrierError::ControlLossConflict),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
    use super::*;
    use crate::foundation::{ChannelContentPin, ChannelId, ControlLossMark, NodeId, WorldId};
    fn id(n: u8) -> [u8; 16] {
        [1, 0, 0, 0, 0, n, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, n]
    }
    #[test]
    fn actual_loss_epoch_starts_four_seconds_and_retry_cannot_extend_it() {
        let world = WorldId::decode(&id(1)).unwrap();
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(2)).unwrap(),
            NodeId::decode(&id(3)).unwrap(),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            4,
            ChannelContentPin::test(world),
        )
        .unwrap();
        let session = GameSessionId::decode(&id(4)).unwrap();
        let reserved = runtime.reserve_fresh_session(session).unwrap();
        let actor = runtime.commit_fresh_session(reserved).unwrap();
        assert_eq!(
            runtime.current_player_reentry_protection(actor, session, 0),
            Ok(false)
        );
        assert!(
            runtime
                .restore_control_after_committed_reentry(actor, session, 1, 100)
                .is_err()
        );
        runtime
            .record_control_loss(
                actor,
                session,
                ControlLossMark {
                    epoch: 1,
                    grace_deadline: 100,
                },
            )
            .unwrap();
        assert!(
            runtime
                .current_player_reentry_protection(actor, session, 100)
                .is_err()
        );
        assert!(
            runtime
                .restore_control_after_committed_reentry(actor, session, 2, 100)
                .is_err()
        );
        runtime
            .restore_control_after_committed_reentry(actor, session, 1, 100)
            .unwrap();
        assert_eq!(
            runtime.current_player_reentry_protection(actor, session, 4_000_099),
            Ok(true)
        );
        runtime
            .restore_control_after_committed_reentry(actor, session, 1, 3_000_000)
            .unwrap();
        assert_eq!(
            runtime.current_player_reentry_protection(actor, session, 4_000_100),
            Ok(false)
        );
        assert!(
            runtime
                .restore_control_after_committed_reentry(actor, session, 2, 4_000_100)
                .is_err()
        );
        runtime
            .record_control_loss(
                actor,
                session,
                ControlLossMark {
                    epoch: 2,
                    grace_deadline: 200,
                },
            )
            .unwrap();
        runtime
            .restore_control_after_committed_reentry(actor, session, 2, 5_000_000)
            .unwrap();
        assert_eq!(
            runtime.current_player_reentry_protection(actor, session, 8_999_999),
            Ok(true)
        );
        assert_eq!(
            runtime.current_player_reentry_protection(actor, session, 9_000_000),
            Ok(false)
        );
        let wrong = GameSessionId::decode(&id(5)).unwrap();
        assert!(
            runtime
                .current_player_reentry_protection(actor, wrong, 5_000_000)
                .is_err()
        );
    }
}
