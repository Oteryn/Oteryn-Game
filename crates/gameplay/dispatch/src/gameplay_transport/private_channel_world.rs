//! S04b private channels extend the existing World relay and durable session owners.
//! Runtime state is only negotiated bindings and last shown metadata; egress uses the shared bound.
use super::ComposedFreshAdmission;
use super::connection::AdmittedSession;
use crate::chat::{ChatText, relay::RelayDestination};
use crate::domain::premium::PremiumStatus;
use crate::durability::{
    item_transfer::CurrentCharacterItemFence,
    private_channel::{ChannelMember, ChannelView, JoinedChannel, MemberView, Prepared},
};
use crate::foundation::{ChannelRuntimeV1, GameSessionId};
use oteryn_protocol_oteryn::chat::line_extended::ChannelNotice;
use oteryn_protocol_oteryn::chat::private_channel::{
    CAPABILITY_CHAT_PRIVATE_CHANNEL_V1, ChatChannelList, PrivateChannelIntent,
};
use oteryn_protocol_oteryn::chat::private_channel_extended::{
    CAPABILITY_CHAT_CHANNEL_PARTICIPANTS_V1, CAPABILITY_CHAT_JOINED_CHANNELS_V1,
    ChannelListExtension,
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
    /// Capabilities 494 and 495: the extension last shown and the members of the last read.
    shown_extension: Option<ChannelListExtension>,
    members: Option<Vec<ChannelMember>>,
}

/// Capabilities 494 and 495: the list delta when the list or its extension changed, and the
/// member notices since the previous read.
pub(super) type ExtendedChannelList = (
    Option<(ChatChannelList, ChannelListExtension)>,
    Vec<ChannelNotice>,
);

#[path = "private_channel_members.rs"]
mod members;
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
                shown_extension: None,
                members: None,
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
    /// The sessions of the other joined members the last read showed.
    fn probe(&self, admitted: &AdmittedSession) -> Vec<[u8; 16]> {
        self.entries
            .lock()
            .ok()
            .and_then(|entries| {
                let entry = entries
                    .get(&admitted.game_session_id)
                    .filter(|entry| entry.ready && same(&entry.admitted, admitted))?;
                Some(members::probe(
                    *admitted.game_session_id.as_bytes(),
                    entry.members.as_deref()?,
                ))
            })
            .unwrap_or_default()
    }
    /// The first read after admission is the baseline: it shows the list, never notices.
    fn list_extended(
        &self,
        admitted: &AdmittedSession,
        read: MemberView,
        joined_selected: bool,
        participants_selected: bool,
    ) -> Option<ExtendedChannelList> {
        let mut entries = self.entries.lock().ok()?;
        let entry = entries
            .get_mut(&admitted.game_session_id)
            .filter(|entry| entry.ready && same(&entry.admitted, admitted))?;
        let MemberView {
            view,
            members,
            active,
        } = read;
        entry.joined = view.joined;
        let notices = match entry.members.as_deref() {
            Some(before) if participants_selected => members::notices(
                *admitted.game_session_id.as_bytes(),
                before,
                &members,
                &active,
            ),
            _ => Vec::new(),
        };
        let extension = members::extension(
            &view.list,
            &entry.joined,
            &members,
            joined_selected,
            participants_selected,
        );
        entry.members = Some(members);
        if !entry.force
            && entry.shown.as_ref() == Some(&view.list)
            && entry.shown_extension.as_ref() == Some(&extension)
        {
            return Some((None, notices));
        }
        entry.force = false;
        entry.shown = Some(view.list.clone());
        entry.shown_extension = Some(extension.clone());
        Some((Some((view.list, extension)), notices))
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
        || binding.channel_id() != Some(admitted.channel_id)
        || fence.runtime_scope != binding.runtime_scope()
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
        // Membership operations and explicit queries request a fresh list. Invitations
        // and exclusions use the next durable diff rather than forcing an unchanged list.
        if matches!(
            intent,
            PrivateChannelIntent::Create
                | PrivateChannelIntent::Join { .. }
                | PrivateChannelIntent::Leave { .. }
                | PrivateChannelIntent::List
        ) {
            self.states
                .chat
                .private_world
                .force_list(admitted.game_session_id);
        }
        reply
    }

    /// The fence of a session whose capability-80 negotiation is ready, admitting it again on
    /// the refresh cadence while the root's sole holder was busy at snapshot admission.
    async fn private_channel_listing_fence(
        &self,
        admitted: AdmittedSession,
    ) -> Option<CurrentCharacterItemFence> {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_CHAT_PRIVATE_CHANNEL_V1)
        {
            return None;
        }
        // Snapshot admission can find the root's sole holder busy. Retry on the existing
        // refresh cadence so that transient unavailability cannot suppress the initial list
        // for the entire connection. Admission still re-proves the complete current fence.
        if !self
            .states
            .chat
            .private_world
            .negotiated(admitted.game_session_id)
        {
            self.private_channel_admit(admitted, true).await;
        }
        if !self
            .states
            .chat
            .private_world
            .negotiated(admitted.game_session_id)
            || !self.states.chat.private_world.matches(&admitted)
        {
            return None;
        }
        let runtime = self.runtime.lock().await;
        qualified(&admitted, &runtime)
    }

    pub(super) async fn private_channel_list(
        &self,
        admitted: AdmittedSession,
    ) -> Option<ChatChannelList> {
        let fence = self.private_channel_listing_fence(admitted).await?;
        let mut view = self
            .root
            .read_private_channel_list(self.character, self.holder, fence)
            .await
            .ok()??;
        view.list.public_rooms = self.states.chat.public_room_list(admitted.game_session_id);
        self.states.chat.private_world.list(&admitted, view)
    }

    /// Capabilities 494 and 495: the list with its extension, and the member notices, from one
    /// durable read of the list and the members of every channel the session is in.
    pub(super) async fn private_channel_list_extended(
        &self,
        admitted: AdmittedSession,
    ) -> ExtendedChannelList {
        let selected = &admitted.continuity.selected_capabilities;
        let joined_selected = selected.contains(CAPABILITY_CHAT_JOINED_CHANNELS_V1);
        let participants_selected = selected.contains(CAPABILITY_CHAT_CHANNEL_PARTICIPANTS_V1);
        if !joined_selected && !participants_selected {
            return (None, Vec::new());
        }
        let Some(fence) = self.private_channel_listing_fence(admitted).await else {
            return (None, Vec::new());
        };
        let probe = self.states.chat.private_world.probe(&admitted);
        let Ok(Some(mut read)) = self
            .root
            .read_private_channel_members(self.character, self.holder, fence, probe)
            .await
        else {
            return (None, Vec::new());
        };
        read.view.list.public_rooms = self.states.chat.public_room_list(admitted.game_session_id);
        self.states
            .chat
            .private_world
            .list_extended(&admitted, read, joined_selected, participants_selected)
            .unwrap_or_default()
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
