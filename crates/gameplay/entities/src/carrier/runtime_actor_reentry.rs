//! DISCONNECT_REENTRY_PVE_PROTECTION_OWNER_DECISION: a committed unexpected-loss
//! recovery grants four seconds on the actual owner clock. Fresh admission has
//! no recovery event and therefore no window. This data lives in its actor slot.
//!
//! S15a: a fresh login instead starts Crystal's login protection (`loginProtectionPeriod`,
//! `config.lua.dist:61`), which only monster target selection reads (`monster.cpp:475`,
//! `:1014`); it does not refuse the player's own actions the way the reentry window does.
use super::{
    CarrierError, ChannelActorCarrier, ChannelRuntimeV1, ExactActorRef, GameSessionId,
    PlayerActorReservation, Slot,
};

/// S15a: Crystal 201189f8 `loginProtectionPeriod = 10 * 1000` (`config.lua.dist:61`,
/// default `configmanager.cpp:347`), in owner-clock microseconds.
pub const FRESH_LOGIN_PROTECTION_MICROS: u64 = 10_000_000;

impl ChannelRuntimeV1 {
    /// S15a: commit a fresh admission and start its login protection at `now_us` (Crystal
    /// sets `lastLoad` on login, `protocolgame.cpp:834`). The deadline is written once, by the
    /// commit, so nothing later can extend it.
    pub fn commit_fresh_session_at(
        &mut self,
        reservation: PlayerActorReservation,
        now_us: u64,
    ) -> Result<ExactActorRef, CarrierError> {
        let until = now_us
            .checked_add(FRESH_LOGIN_PROTECTION_MICROS)
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        let session = reservation.game_session_id;
        let actor = self.commit_fresh_session(reservation)?;
        let index = self
            .carrier
            .player_slot_index(&self.continuity, actor.0, session)?;
        if let Slot::Occupied { spell_combat, .. } = &mut self.carrier.slots[index] {
            spell_combat.fresh_login_until = Some(until);
        }
        Ok(actor)
    }

    /// Whether monster target selection skips this player now: the reconnect window (S15b)
    /// or the fresh-login period (S15a, Crystal `Player::checkLoginDelay`,
    /// `player.cpp:6434-6436`, inclusive deadline).
    pub fn current_player_monster_target_protection(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        now_us: u64,
    ) -> Result<bool, CarrierError> {
        let index = self
            .carrier
            .player_slot_index(&self.continuity, actor.0, session)?;
        let login = match &self.carrier.slots[index] {
            Slot::Occupied { spell_combat, .. } => spell_combat
                .fresh_login_until
                .is_some_and(|until| now_us <= until),
            _ => false,
        };
        // Crystal's login delay also protects an actor held in combat after control loss.
        // The reconnect query requires restored control, so consult it only after login
        // protection has expired; both reads still prove the exact actor and session.
        if login {
            Ok(true)
        } else {
            self.current_player_reentry_protection(actor, session, now_us)
        }
    }

    pub fn current_player_reentry_protection(
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
                lifecycle,
                spell_combat,
                ..
            } if lifecycle.control_loss.is_none() => Ok(spell_combat
                .reentry
                .is_some_and(|(_, until)| now_us < until)),
            _ => Err(CarrierError::ControlLossConflict),
        }
    }

    /// The transport invokes this only after its genuine CompleteReconnect COMMIT.
    /// A matching actual loss is required to start a window; retries retain its
    /// original deadline and cannot extend the benefit or invent another epoch.
    pub fn restore_control_after_committed_reentry(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
        epoch: u64,
        now_us: u64,
    ) -> Result<(), CarrierError> {
        let index = self
            .carrier
            .player_slot_index(&self.continuity, actor.0, session)?;
        self.carrier
            .restore_control_after_committed_reentry_at(index, epoch, now_us)
    }
}

impl ChannelActorCarrier {
    pub(super) fn restore_control_after_committed_reentry_at(
        &mut self,
        index: usize,
        epoch: u64,
        now_us: u64,
    ) -> Result<(), CarrierError> {
        self.assert_slot_spell_unreserved(index)?;
        let until = now_us
            .checked_add(4_000_000)
            .ok_or(CarrierError::CapacityArithmeticOverflow)?;
        match &mut self.slots[index] {
            Slot::Occupied {
                lifecycle,
                spell_combat,
                ..
            } if epoch != 0
                && lifecycle
                    .control_loss
                    .is_some_and(|mark| mark.epoch == epoch)
                && spell_combat
                    .reentry
                    .is_none_or(|(previous, _)| epoch > previous) =>
            {
                lifecycle.control_loss = None;
                spell_combat.reentry = Some((epoch, until));
                Ok(())
            }
            Slot::Occupied {
                lifecycle,
                spell_combat,
                ..
            } if lifecycle.control_loss.is_none()
                && spell_combat
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
#[path = "runtime_actor_fresh_login_tests.rs"]
mod fresh_login_tests;

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
    use super::*;
    use crate::ControlLossMark;
    use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
    use oteryn_gameplay_foundation::foundation::ChannelId;
    use oteryn_gameplay_foundation::foundation::NodeId;
    use oteryn_gameplay_foundation::foundation::WorldId;
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
