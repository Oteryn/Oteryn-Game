//! S04b private channels extend the existing World relay and durable session owners.
//! Runtime state is only negotiated bindings and last shown metadata; egress uses the shared bound.
use super::ComposedFreshAdmission;
use super::connection::AdmittedSession;
use crate::chat::{ChatText, relay::RelayDestination};
use crate::domain::premium::PremiumStatus;
use crate::durability::{
    item_transfer::CurrentCharacterItemFence,
    private_channel::{ChannelView, JoinedChannel, Prepared},
};
use crate::foundation::{ChannelRuntimeV1, GameSessionId, RuntimeScopeRefV1};
use oteryn_protocol_oteryn::chat::private_channel::{
    CAPABILITY_CHAT_PRIVATE_CHANNEL_V1, ChatChannelList, PrivateChannelIntent,
};
use oteryn_protocol_oteryn::chat::{ChatDisposition, ChatIntentResult, ChatLine};
use std::collections::HashMap;
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};

#[derive(Default)]
pub(super) struct PrivateChannelRuntime {
    entries: Mutex<HashMap<GameSessionId, Binding>>,
    admissions: AtomicU64,
}
struct Binding {
    admitted: AdmittedSession,
    reservation: u64,
    ready: bool,
    joined: Vec<JoinedChannel>,
    shown: Option<ChatChannelList>,
    force: bool,
}
fn same(left: &AdmittedSession, right: &AdmittedSession) -> bool {
    left.world_id == right.world_id
        && left.channel_id == right.channel_id
        && left.item_fence == right.item_fence
        && left.runtime_actor == right.runtime_actor
        && left.continuity.connection_generation == right.continuity.connection_generation
        && left.game_session_id == right.game_session_id
        && right
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_CHAT_PRIVATE_CHANNEL_V1)
}
impl PrivateChannelRuntime {
    /// A later admission or forget invalidates this reservation before any async work ends.
    fn begin(&self, admitted: AdmittedSession) -> Option<u64> {
        let mut entries = self.entries.lock().ok()?;
        if entries.get(&admitted.game_session_id).is_some_and(|entry| {
            entry.admitted.continuity.connection_generation
                > admitted.continuity.connection_generation
        }) {
            return None;
        }
        let reservation = self
            .admissions
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .ok()?
            .checked_add(1)?;
        entries.insert(
            admitted.game_session_id,
            Binding {
                admitted,
                reservation,
                ready: false,
                joined: Vec::new(),
                shown: None,
                force: true,
            },
        );
        Some(reservation)
    }
    fn finish(
        &self,
        session: GameSessionId,
        reservation: u64,
        joined: Option<Vec<JoinedChannel>>,
    ) -> bool {
        let Ok(mut entries) = self.entries.lock() else {
            return false;
        };
        let Some(entry) = entries
            .get_mut(&session)
            .filter(|entry| entry.reservation == reservation)
        else {
            return false;
        };
        if let Some(joined) = joined {
            entry.ready = true;
            entry.joined = joined;
        } else {
            entries.remove(&session);
        }
        true
    }
    pub(super) fn forget(&self, session: GameSessionId) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.remove(&session);
        }
    }
    pub(super) fn negotiated(&self, session: GameSessionId) -> bool {
        self.entries
            .lock()
            .is_ok_and(|entries| entries.get(&session).is_some_and(|entry| entry.ready))
    }
    pub(super) fn matches(&self, admitted: &AdmittedSession) -> bool {
        self.entries.lock().is_ok_and(|entries| {
            entries
                .get(&admitted.game_session_id)
                .is_some_and(|entry| entry.ready && same(&entry.admitted, admitted))
        })
    }
    /// Queue interest is bounded to the receiver's own/joined channels. It is no disclosure
    /// grant: exclusions, session changes and epochs are proved under SQL custody at drain.
    pub(super) fn accepts_relay(
        &self,
        session: GameSessionId,
        channel: u16,
        epoch: u64,
        sent_at: u64,
    ) -> bool {
        self.entries.lock().is_ok_and(|entries| {
            entries.get(&session).is_some_and(|entry| {
                entry.ready
                    && entry.joined.iter().any(|interest| {
                        interest.channel_id == channel
                            && interest.epoch == epoch
                            && interest.since_micros <= sent_at
                    })
            })
        })
    }
    fn remember(&self, admitted: &AdmittedSession, joined: Vec<JoinedChannel>) {
        if let Ok(mut entries) = self.entries.lock()
            && let Some(entry) = entries
                .get_mut(&admitted.game_session_id)
                .filter(|entry| entry.ready && same(&entry.admitted, admitted))
        {
            entry.joined = joined;
        }
    }
    fn force_list(&self, session: GameSessionId) {
        if let Ok(mut entries) = self.entries.lock()
            && let Some(entry) = entries.get_mut(&session)
        {
            entry.force = true;
        }
    }
    fn list(&self, admitted: &AdmittedSession, view: ChannelView) -> Option<ChatChannelList> {
        let mut entries = self.entries.lock().ok()?;
        let entry = entries
            .get_mut(&admitted.game_session_id)
            .filter(|entry| entry.ready && same(&entry.admitted, admitted))?;
        entry.joined = view.joined;
        if !entry.force && entry.shown.as_ref() == Some(&view.list) {
            return None;
        }
        entry.force = false;
        entry.shown = Some(view.list.clone());
        Some(view.list)
    }
}

fn qualified(
    admitted: &AdmittedSession,
    runtime: &ChannelRuntimeV1,
) -> Option<CurrentCharacterItemFence> {
    let fence = admitted.item_fence?;
    let actor = admitted.runtime_actor?;
    let binding = runtime.binding();
    if runtime.owner_fence().is_err()
        || !runtime
            .player_control_facts(actor, admitted.game_session_id)
            .is_ok_and(|facts| facts.control_loss.is_none())
        || fence.game_session_id != admitted.game_session_id
        || fence.connection_generation.get() != admitted.continuity.connection_generation
        || admitted.world_id != binding.world_id()
        || admitted.channel_id != binding.channel_id()
        || fence.runtime_scope
            != RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id())
        || fence.scope_ownership_generation != binding.scope_generation()
    {
        return None;
    }
    Some(fence)
}
fn result(disposition: ChatDisposition) -> ChatIntentResult {
    ChatIntentResult {
        disposition,
        wait_seconds: 0,
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn private_channel_admit(&self, admitted: AdmittedSession, selected: bool) {
        let selected = selected
            && admitted
                .continuity
                .selected_capabilities
                .contains(CAPABILITY_CHAT_PRIVATE_CHANNEL_V1);
        let fence = {
            let runtime = self.runtime.lock().await;
            qualified(&admitted, &runtime)
        };
        let Some(fence) = fence else {
            return;
        };
        let Some(reservation) = self.states.chat.private_world.begin(admitted) else {
            return;
        };
        let mut joined = None;
        if matches!(
            self.root
                .private_channel_admit(self.character, self.holder, fence, selected)
                .await,
            Ok(true)
        ) && selected
            && let Ok(Some(view)) = self
                .root
                .read_private_channel_list(self.character, self.holder, fence)
                .await
        {
            let current = {
                let runtime = self.runtime.lock().await;
                qualified(&admitted, &runtime)
            };
            if current == Some(fence) {
                joined = Some(view.joined);
            }
        }
        self.states
            .chat
            .private_world
            .finish(admitted.game_session_id, reservation, joined);
    }

    async fn private_channel_refresh_interest(
        &self,
        admitted: AdmittedSession,
        fence: CurrentCharacterItemFence,
    ) {
        let joined = self
            .root
            .read_private_channel_list(self.character, self.holder, fence)
            .await
            .ok()
            .flatten()
            .map(|view| view.joined)
            .unwrap_or_default();
        self.states.chat.private_world.remember(&admitted, joined);
    }

    pub(super) async fn private_channel_command(
        &self,
        admitted: AdmittedSession,
        command_id: u64,
        intent: &PrivateChannelIntent,
    ) -> Option<ChatIntentResult> {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_CHAT_PRIVATE_CHANNEL_V1)
            || !self.states.chat.private_world.matches(&admitted)
        {
            return Some(result(ChatDisposition::Rejected));
        }
        let fence = {
            let runtime = self.runtime.lock().await;
            qualified(&admitted, &runtime)
        };
        let Some(fence) = fence else {
            return Some(result(ChatDisposition::Rejected));
        };
        // Replay before any current Premium gate, mute, spam token or new relay publication.
        let prepared = match self
            .root
            .prepare_private_channel(
                self.character,
                self.holder,
                fence,
                command_id,
                intent.clone(),
            )
            .await
        {
            Ok(Prepared::Replayed(reply)) => {
                if reply.disposition == ChatDisposition::Ok {
                    self.private_channel_refresh_interest(admitted, fence).await;
                }
                return Some(reply);
            }
            Ok(Prepared::Fresh(write)) => write,
            Err(_) => return None,
        };
        let sealed = if let PrivateChannelIntent::Say { text, .. } = intent {
            let Ok(text) = ChatText::parse(text) else {
                return prepared
                    .refuse(result(ChatDisposition::Rejected))
                    .await
                    .ok();
            };
            if let Some(wait_seconds) = self.staff_mute_wait(admitted.game_session_id).await {
                return prepared
                    .refuse(ChatIntentResult {
                        disposition: ChatDisposition::Muted,
                        wait_seconds,
                    })
                    .await
                    .ok();
            }
            if let Some(publication) = prepared.publication.as_ref() {
                if let Some(refused) = self.states.chat.pre_gate_relay(
                    admitted.game_session_id,
                    None,
                    text.as_str(),
                    self.owner_now(),
                ) {
                    return prepared.refuse(refused).await.ok();
                }
                let Some(relay) = self.states.chat.relay() else {
                    return prepared
                        .refuse(result(ChatDisposition::ChatUnavailable))
                        .await
                        .ok();
                };
                match relay.seal_line(
                    RelayDestination::PrivateChannel {
                        channel_id: publication.channel_id,
                        epoch: publication.epoch,
                    },
                    publication.sender_name.clone(),
                    text,
                ) {
                    Ok(sealed) => Some(sealed),
                    Err(disposition) => return prepared.refuse(result(disposition)).await.ok(),
                }
            } else {
                None
            }
        } else {
            None
        };
        let reply = if matches!(intent, PrivateChannelIntent::Create) {
            self.with_premium_gate(prepared.account, move |status, tx| {
                Box::pin(async move { prepared.commit(status, tx, sealed).await })
            })
            .await
            .ok()
            .and_then(std::result::Result::ok)
        } else {
            prepared
                .commit(PremiumStatus::NotCurrent, None, sealed)
                .await
                .ok()
        };
        if reply.is_some_and(|reply| reply.disposition == ChatDisposition::Ok)
            && matches!(
                intent,
                PrivateChannelIntent::Create
                    | PrivateChannelIntent::Join { .. }
                    | PrivateChannelIntent::Leave { .. }
            )
        {
            self.private_channel_refresh_interest(admitted, fence).await;
        }
        self.states
            .chat
            .private_world
            .force_list(admitted.game_session_id);
        reply
    }

    pub(super) async fn private_channel_list(
        &self,
        admitted: AdmittedSession,
    ) -> Option<ChatChannelList> {
        if !self
            .states
            .chat
            .private_world
            .negotiated(admitted.game_session_id)
            || !self.states.chat.private_world.matches(&admitted)
        {
            return None;
        }
        let fence = {
            let runtime = self.runtime.lock().await;
            qualified(&admitted, &runtime)
        }?;
        let mut view = self
            .root
            .read_private_channel_list(self.character, self.holder, fence)
            .await
            .ok()??;
        view.list.public_rooms = self.states.chat.public_room_list(admitted.game_session_id);
        self.states.chat.private_world.list(&admitted, view)
    }

    pub(super) async fn private_channel_accept(
        &self,
        admitted: AdmittedSession,
        candidate: &ChatLine,
        destination: crate::chat::egress::PrivateChannelChatDestination,
    ) -> Option<ChatLine> {
        if !self.states.chat.private_world.matches(&admitted) {
            self.states
                .chat
                .accept_private_channel_line(&admitted, candidate, destination, false);
            return None;
        }
        let fence = {
            let runtime = self.runtime.lock().await;
            qualified(&admitted, &runtime)
        };
        let accepted = if let Some(fence) = fence {
            self.root
                .with_private_channel_custody(
                    self.character,
                    self.holder,
                    fence,
                    (
                        destination.channel_id,
                        destination.epoch,
                        destination.sent_at_micros,
                    ),
                    |allowed| {
                        self.states.chat.accept_private_channel_line(
                            &admitted,
                            candidate,
                            destination,
                            allowed,
                        )
                    },
                )
                .await
                .ok()
                .flatten()
        } else {
            None
        };
        if accepted.is_none() {
            self.states
                .chat
                .accept_private_channel_line(&admitted, candidate, destination, false);
        }
        accepted
    }
}

#[cfg(test)]
#[path = "private_channel_world_tests.rs"]
mod tests;
