//! Logged-in transport for signed operators. Negotiation carries bytes, never authority.
use super::ComposedFreshAdmission;
use super::connection::{AdmittedSession, FreshAdmissionAuthority};
use crate::foundation::GameSessionId;
use oteryn_protocol_oteryn::staff as wire;
use std::time::Duration;

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
                || Some(admitted.channel_id) != binding.channel_id()
                || fence.runtime_scope != binding.runtime_scope()
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
        self.staff_only_mute_wait(session).await.is_some()
    }
    pub(super) async fn staff_mute_wait(&self, session: GameSessionId) -> Option<u32> {
        // This shared gate covers every player-chat path. Staff's spell gate
        // remains staff-only: Platform MUTE does not restrict spell casting.
        let mut platform_wait = None;
        if platform_enforcement_enabled(self.states.platform_enforcement_enabled) {
            use super::connection::AdmissionRefusal;
            use crate::durability::DurabilityError;
            let read = async {
                let retry = super::AdmissionRetryBudget::for_foreground_command(self.root).ok()?;
                retry
                    .retry_unavailable(async || {
                        self.root
                            .platform_mute_for_session(self.character, session)
                            .await
                            .map_err(|error| match error {
                                DurabilityError::RootUnavailable
                                | DurabilityError::Unavailable
                                | DurabilityError::Database(_)
                                | DurabilityError::RootPassDeadlineExceeded => {
                                    AdmissionRefusal::Unavailable
                                }
                                _ => AdmissionRefusal::Rejected,
                            })
                    })
                    .await
                    .ok()
            }
            .await;
            // No failure is cached. The next chat command retries current truth.
            platform_wait = read
                .unwrap_or(Some(1))
                .map(|wait| wait.clamp(1, oteryn_protocol_oteryn::chat::MAX_CHAT_WAIT_SECONDS));
        }
        match (self.staff_only_mute_wait(session).await, platform_wait) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        }
    }
    async fn staff_only_mute_wait(&self, session: GameSessionId) -> Option<u32> {
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
fn platform_enforcement_enabled(serving_enabled: bool) -> bool {
    #[cfg(test)]
    if let Some(enabled) = test_clock::enforcement() {
        return enabled;
    }
    serving_enabled
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
    // Each PG scenario owns a current-thread runtime on its own OS thread.
    // A missing/future clock in one scenario must not affect another fixture.
    use std::cell::Cell;
    thread_local! {
        static NOW: Cell<Option<Option<(u64, u64)>>> = const { Cell::new(None) };
        static ENFORCEMENT: Cell<Option<bool>> = const { Cell::new(None) };
    }
    pub(in super::super) fn set(now: Option<(u64, u64)>) {
        NOW.set(Some(now));
    }
    pub(in crate::gameplay_transport) fn get() -> Option<Option<(u64, u64)>> {
        NOW.get()
    }
    pub(in crate::gameplay_transport) fn set_enforcement(enabled: bool) {
        ENFORCEMENT.set(Some(enabled));
    }
    pub(in crate::gameplay_transport) fn enforcement() -> Option<bool> {
        ENFORCEMENT.get()
    }
    #[test]
    fn staff_test_clock_overrides_are_isolated_between_scenarios() {
        let available = std::thread::spawn(|| {
            set(Some((1, 1)));
            let unavailable = std::thread::spawn(|| {
                assert_eq!(get(), None);
                set(None);
                assert_eq!(get(), Some(None));
            });
            assert!(unavailable.join().is_ok());
            assert_eq!(get(), Some(Some((1, 1))));
        });
        assert!(available.join().is_ok());
        assert_eq!(get(), None);
    }
}
