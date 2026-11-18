//! chsw / GAME-CHANNEL-01 §7-§10: in-game Channel switch (`channel-switch-v1.md`).
//!
//! A switch is the ordinary logout release (safe exit) after the switch locks pass; the client
//! then enters the target Channel by a fresh admission, whose receipt insert revalidates the
//! cooldown and the database locks and advances the guard (migration 2858).
use super::logout::LogoutReply;
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::foundation::{ChannelRuntimeV1, CommandId, ExactActorRef, GameSessionId, RuntimeScopeRefV1};
use oteryn_protocol_oteryn::channel_switch::{self as wire, SwitchLock, SwitchStatus};
use oteryn_protocol_oteryn::logout::LogoutResult;

/// What the connection does after one switch command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelSwitchOutcome {
    /// `ACCEPTED` with this status; the session goes on.
    Status(Vec<u8>),
    /// `REJECTED` with this status; nothing changed.
    Refused(Vec<u8>),
    /// The safe exit committed: `ACCEPTED` with this status, then the transport closes.
    Switched(Vec<u8>),
    /// The release outcome is unknown: no result, the transport closes and the control-loss
    /// lifecycle reconciles the durable row (as for an unknown logout).
    Unknown,
}

/// The lock a logout refusal means for a switch.
pub(super) fn logout_lock(reply: LogoutReply) -> Option<SwitchLock> {
    match reply {
        LogoutReply::Result(LogoutResult::Accepted) | LogoutReply::Unknown => None,
        LogoutReply::Result(LogoutResult::InFight { .. }) => Some(SwitchLock::Combat),
        LogoutReply::Result(LogoutResult::Busy) => Some(SwitchLock::Checkpoint),
        LogoutReply::NoLogout => Some(SwitchLock::NoLogout),
    }
}

fn status_of(lock: SwitchLock) -> SwitchStatus {
    SwitchStatus {
        lock,
        cooldown_left_ms: 0,
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// The current session, bound to this node's Channel runtime, with the capability selected.
    async fn switch_session_current(&self, admitted: AdmittedSession, id: u64) -> bool {
        let Some(fence) = admitted.item_fence else {
            return false;
        };
        if CommandId::new(id).is_err() {
            return false;
        }
        let runtime = self.runtime.lock().await;
        let binding = runtime.binding();
        admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_CHANNEL_SWITCH_V1)
            && fence.game_session_id == admitted.game_session_id
            && fence.connection_generation.get() == admitted.continuity.connection_generation
            && admitted.world_id == binding.world_id()
            && Some(admitted.channel_id) == binding.channel_id()
            && fence.runtime_scope == binding.runtime_scope()
            && fence.scope_ownership_generation == binding.scope_generation()
            && runtime.owner_fence().is_ok()
    }

    /// The runtime locks of §8 that hold without a release: trade, transitions, an instance
    /// scope (boss, raid and event rooms) and a running fight.
    async fn switch_runtime_lock(&self, admitted: AdmittedSession) -> Option<SwitchLock> {
        let session = admitted.game_session_id;
        let Some(actor) = admitted.runtime_actor else {
            return Some(SwitchLock::Checkpoint);
        };
        if self.states.player_trades.transitioning(session) {
            return Some(SwitchLock::ValueTransaction);
        }
        if self.states.player_trades.open(session) {
            return Some(SwitchLock::Trade);
        }
        if self.states.newhaven_travel.held(session)
            || self.states.dawnport_departures.held(session)
            || self
                .states
                .dawnport_trials
                .lock()
                .map_or(true, |held| held.contains(&session))
        {
            return Some(SwitchLock::Transition);
        }
        if matches!(actor.runtime_scope(), RuntimeScopeRefV1::Instance { .. }) {
            return Some(SwitchLock::Event);
        }
        let runtime = self.runtime.lock().await;
        self.switch_encounter_lock(&runtime, actor, session)
    }

    async fn switch_status(
        &self,
        admitted: AdmittedSession,
        target: Option<[u8; 16]>,
    ) -> Option<SwitchStatus> {
        if let Some(lock) = self.switch_runtime_lock(admitted).await {
            return Some(status_of(lock));
        }
        let durable = self.switch_durable_status(admitted, target).await;
        // A Grizzly visit stays open after the fight, until its room owner completes the return.
        if durable.is_some_and(|status| status.lock == SwitchLock::Event) {
            return durable;
        }
        let actor = admitted.runtime_actor?;
        let Some(zones) = self.logout_zones(actor).await else {
            return Some(status_of(SwitchLock::Checkpoint));
        };
        if zones.no_logout {
            return Some(status_of(SwitchLock::NoLogout));
        }
        if self.logout_in_fight(actor, admitted.game_session_id, zones.protection_zone).await.is_some() {
            return Some(status_of(SwitchLock::Combat));
        }
        durable
    }

    pub(super) fn switch_encounter_lock(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<SwitchLock> {
        let Ok(position) = runtime.read_actor_position(actor) else {
            return Some(SwitchLock::Checkpoint);
        };
        let at = position.position();
        if self.encounter_blocks_channel_switch(runtime, actor, session, at) {
            return Some(SwitchLock::Event);
        }
        let Ok(states) = self.states.spell_states.try_lock() else {
            return Some(SwitchLock::Checkpoint);
        };
        states.encounter_phases.blocks_channel_switch(runtime, actor, session, at)
            .then_some(SwitchLock::Event)
    }

    async fn switch_durable_status(
        &self,
        admitted: AdmittedSession,
        target: Option<[u8; 16]>,
    ) -> Option<SwitchStatus> {
        let fence = admitted.item_fence?;
        self.root
            .read_channel_switch_preflight(
                self.character,
                self.holder,
                fence,
                *admitted.world_id.as_bytes(),
                target,
            )
            .await
            .ok()
            .flatten()
    }

    /// Command 6247: the locks and the cooldown that apply to any switch now.
    pub(super) async fn channel_switch_status_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        payload: &[u8],
    ) -> Option<ChannelSwitchOutcome> {
        wire::decode_status_request(payload)?;
        if !self.switch_session_current(admitted, id).await {
            return None;
        }
        let status = self.switch_status(admitted, None).await?;
        wire::encode_status(status).map(ChannelSwitchOutcome::Status)
    }

    /// Command 6251: the online count of each current Channel of this World.
    pub(super) async fn channel_load_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        payload: &[u8],
    ) -> Option<ChannelSwitchOutcome> {
        wire::decode_status_request(payload)?;
        if !self.switch_session_current(admitted, id).await {
            return None;
        }
        let load = self
            .root
            .read_channel_load(
                self.character,
                self.holder,
                admitted.item_fence?,
                *admitted.world_id.as_bytes(),
            )
            .await
            .ok()
            .flatten()?;
        wire::encode_load(&load).map(ChannelSwitchOutcome::Status)
    }

    /// Command 6248: every lock first, then the ordinary logout release as the safe exit.
    pub(super) async fn channel_switch_request_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        payload: &[u8],
    ) -> Option<ChannelSwitchOutcome> {
        let target = wire::decode_request(payload)?;
        if !self.switch_session_current(admitted, id).await {
            return None;
        }
        let refuse =
            |status: SwitchStatus| wire::encode_status(status).map(ChannelSwitchOutcome::Refused);
        if &target == admitted.channel_id.as_bytes() {
            return refuse(status_of(SwitchLock::SameChannel));
        }
        let status = self.switch_status(admitted, Some(target)).await?;
        if status != SwitchStatus::READY {
            return refuse(status);
        }
        let reply = match self.logout_session_as(admitted, true).await {
            Ok(reply) => reply,
            Err(lock) => return refuse(status_of(lock)),
        };
        match (reply, logout_lock(reply)) {
            (LogoutReply::Unknown, _) => Some(ChannelSwitchOutcome::Unknown),
            (_, Some(lock)) => refuse(status_of(lock)),
            (_, None) => {
                wire::encode_status(SwitchStatus::READY).map(ChannelSwitchOutcome::Switched)
            }
        }
    }
}

macro_rules! authority_methods {
    () => {
        fn channel_switch_execute(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _id: u64,
            _command_type: u32,
            _payload: &[u8],
        ) -> impl std::future::Future<
            Output = Option<$crate::gameplay_transport::channel_switch::ChannelSwitchOutcome>,
        > {
            async { None }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn channel_switch_execute(
            &self,
            admitted: connection::AdmittedSession,
            id: u64,
            command_type: u32,
            payload: &[u8],
        ) -> Option<channel_switch::ChannelSwitchOutcome> {
            match command_type {
                oteryn_protocol_oteryn::channel_switch::COMMAND_TYPE_CHANNEL_SWITCH_STATUS => {
                    self.channel_switch_status_wire(admitted, id, payload).await
                }
                oteryn_protocol_oteryn::channel_switch::COMMAND_TYPE_CHANNEL_SWITCH_REQUEST => {
                    Box::pin(self.channel_switch_request_wire(admitted, id, payload)).await
                }
                oteryn_protocol_oteryn::channel_switch::COMMAND_TYPE_CHANNEL_LOAD => {
                    self.channel_load_wire(admitted, id, payload).await
                }
                _ => None,
            }
        }
    };
}
pub(super) use composed_methods;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refused_safe_exit_is_a_switch_lock_and_only_an_accepted_one_switches() {
        assert_eq!(
            logout_lock(LogoutReply::Result(LogoutResult::InFight {
                retry_after_ms: 1
            })),
            Some(SwitchLock::Combat)
        );
        assert_eq!(
            logout_lock(LogoutReply::Result(LogoutResult::Busy)),
            Some(SwitchLock::Checkpoint)
        );
        assert_eq!(
            logout_lock(LogoutReply::NoLogout),
            Some(SwitchLock::NoLogout)
        );
        assert_eq!(
            logout_lock(LogoutReply::Result(LogoutResult::Accepted)),
            None
        );
        assert_eq!(logout_lock(LogoutReply::Unknown), None);
    }
}
