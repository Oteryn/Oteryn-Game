//! Logged-in transport for signed operators. Negotiation carries bytes, never authority.
use super::connection::{AdmittedSession, FreshAdmissionAuthority};
use super::{ComposedFreshAdmission, capabilities::OfferedCapability};
use crate::foundation::GameSessionId;
use oteryn_protocol_oteryn::staff as wire;
use std::time::Duration;
pub(super) const OFFERED: OfferedCapability = OfferedCapability {
    id: wire::CAPABILITY_STAFF_V1,
    requires: &[],
};

/// A storage failure never becomes a terminal rejection of a possibly committed action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaffReply {
    Rejected,
    Result(Vec<u8>),
    Unknown,
}

fn trusted_clock() -> Option<(u64, u64)> {
    #[cfg(test)]
    if let Some(now) = test_clock::get() {
        return now;
    }
    let time = crate::premium::clock::system_trusted_now()?;
    Some((
        u64::try_from(time.lower_us()).ok()? / 1000,
        u64::try_from(time.upper_us()).ok()? / 1000,
    ))
}
pub(super) async fn dispatch<A: FreshAdmissionAuthority>(
    authority: &A,
    admitted: AdmittedSession,
    command: u32,
    payload: &[u8],
) -> StaffReply {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(wire::CAPABILITY_STAFF_V1)
        || !matches!(
            command,
            wire::COMMAND_TYPE_STAFF_REQUEST | wire::COMMAND_TYPE_STAFF_RECEIPT
        )
    {
        return StaffReply::Rejected;
    }
    tokio::time::timeout(
        Duration::from_secs(10),
        authority.staff_command(
            admitted,
            command == wire::COMMAND_TYPE_STAFF_RECEIPT,
            payload,
        ),
    )
    .await
    .unwrap_or(StaffReply::Unknown)
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn execute_staff(
        &self,
        admitted: AdmittedSession,
        receipt: bool,
        payload: &[u8],
    ) -> StaffReply {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_STAFF_V1)
        {
            return StaffReply::Rejected;
        }
        let Some((envelope, fence, actor)) = (|| {
            Some((
                wire::decode_envelope(payload).ok()?,
                admitted.item_fence?,
                admitted.runtime_actor?,
            ))
        })() else {
            return StaffReply::Rejected;
        };
        {
            let runtime = self.runtime.lock().await;
            let binding = runtime.binding();
            if runtime.owner_fence().is_err()
                || admitted.world_id != binding.world_id()
                || admitted.channel_id != binding.channel_id()
                || fence.runtime_scope
                    != crate::foundation::RuntimeScopeRefV1::channel(
                        binding.world_id(),
                        binding.channel_id(),
                    )
                || fence.scope_ownership_generation != binding.scope_generation()
                || !runtime
                    .player_control_facts(actor, admitted.game_session_id)
                    .is_ok_and(|f| f.control_loss.is_none())
                || fence.game_session_id != admitted.game_session_id
                || fence.connection_generation.get() != admitted.continuity.connection_generation
            {
                return StaffReply::Rejected;
            }
        }
        match self
            .root
            .submit_staff(
                self.character,
                self.holder,
                fence,
                envelope,
                receipt,
                trusted_clock,
            )
            .await
        {
            Ok(Some(bytes)) => StaffReply::Result(bytes),
            Ok(None) => StaffReply::Rejected,
            Err(error) => {
                #[cfg(test)]
                eprintln!("GM_TEST_STORAGE_ERROR {error:?}");
                let _ = error;
                StaffReply::Unknown
            }
        }
    }
    pub(super) async fn staff_muted(&self, session: GameSessionId) -> bool {
        self.staff_mute_wait(session).await.is_some()
    }
    pub(super) async fn staff_mute_wait(&self, session: GameSessionId) -> Option<u32> {
        // All booted nodes enforce DB truth, including nodes without operator trust.
        if !self.root.staff_moderation_configured() {
            return None;
        }
        let Some(fence) = super::character_gameplay_fence::current(self, session, false).await
        else {
            return Some(1);
        };
        let staff_wait = match self
            .root
            .staff_mute_for_session(self.character, self.holder, fence.item_fence())
            .await
        {
            Ok(Some(0)) => None,
            Ok(Some(until)) => match trusted_clock() {
                Some((lower, _)) if until <= lower => None,
                Some((lower, _)) => Some(
                    u32::try_from((until - lower).div_ceil(1000))
                        .unwrap_or(u32::MAX)
                        .clamp(1, oteryn_protocol_oteryn::chat::MAX_CHAT_WAIT_SECONDS),
                ),
                None => Some(1),
            },
            _ => Some(1),
        };
        staff_wait.map(|wait| {
            wait.max(
                self.states
                    .chat
                    .active_mute_wait(session, self.owner_now())
                    .unwrap_or(0),
            )
        })
    }
}
macro_rules! composed_methods {
    () => {
        async fn staff_command(
            &self,
            admitted: crate::gameplay_transport::connection::AdmittedSession,
            receipt: bool,
            payload: &[u8],
        ) -> crate::gameplay_transport::StaffReply {
            self.execute_staff(admitted, receipt, payload).await
        }
    };
}
pub(super) use composed_methods;

#[cfg(test)]
pub(super) mod test_clock {
    // Only this mechanic's PG tests override time; no runtime input can reach it.
    use std::sync::Mutex;
    static NOW: Mutex<Option<Option<(u64, u64)>>> = Mutex::new(None);
    pub(in super::super) fn set(now: Option<(u64, u64)>) {
        if let Ok(mut value) = NOW.lock() {
            *value = Some(now);
        }
    }
    pub(super) fn get() -> Option<Option<(u64, u64)>> {
        NOW.lock().ok().and_then(|now| *now)
    }
}
