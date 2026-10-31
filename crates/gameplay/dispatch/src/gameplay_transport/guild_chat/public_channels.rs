//! S04c extends the existing CHAT owner and its World relay; no second chat service.
use super::*;
use crate::durability::chat_public::{HelpAction, HelpReceipt, PublicChatAccess};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::gameplay_transport::connection::AdmittedSession;
use oteryn_gameplay_social_rules::public_channels::{PublicChatFacts, PublicRoom};
use oteryn_protocol_oteryn::chat::public_channels::CAPABILITY_CHAT_PUBLIC_CHANNELS_V1;

#[derive(Default)]
pub(super) struct PublicRoomState {
    admitted: Option<AdmittedSession>,
    known: bool,
    selected: bool,
    facts: Option<PublicChatFacts>,
    rook_ready_at: Option<SemanticTimeMicros>,
}

fn room_rule(room: ChatRoom) -> PublicRoom {
    match room {
        ChatRoom::World => PublicRoom::World,
        ChatRoom::English => PublicRoom::English,
        ChatRoom::Help => PublicRoom::Help,
        ChatRoom::Advertising => PublicRoom::Advertising,
        ChatRoom::AdvertisingRookgaard => PublicRoom::AdvertisingRookgaard,
        ChatRoom::Tutor => PublicRoom::Tutor,
        ChatRoom::Gamemaster => PublicRoom::Gamemaster,
        ChatRoom::GuildLeaders => PublicRoom::GuildLeaders,
    }
}

impl PublicRoomState {
    fn matches(&self, admitted: &AdmittedSession) -> bool {
        self.admitted.as_ref().is_some_and(|current| {
            current.game_session_id == admitted.game_session_id
                && current.world_id == admitted.world_id
                && current.channel_id == admitted.channel_id
                && current.runtime_actor == admitted.runtime_actor
                && current.item_fence == admitted.item_fence
                && current.continuity.connection_generation
                    == admitted.continuity.connection_generation
                && self.selected
                    == admitted
                        .continuity
                        .selected_capabilities
                        .contains(CAPABILITY_CHAT_PUBLIC_CHANNELS_V1)
        })
    }

    pub(super) fn can_join(&self, room: ChatRoom) -> bool {
        if room.requires_public_channels() && !self.selected {
            return false;
        }
        match self.facts {
            Some(facts) => facts.can_join(room_rule(room)),
            None => !self.known && !room.requires_public_channels(),
        }
    }
}

impl ChatRuntime {
    /// The existing private-channel list uses this view, including legacy negotiation.
    pub(in crate::gameplay_transport) fn public_room_list(
        &self,
        session: GameSessionId,
    ) -> ChatRoomSet {
        let mut rooms = ChatRoomSet::default();
        if self.relay().is_none() {
            return rooms;
        }
        let Ok(sessions) = self.sessions.lock() else {
            return rooms;
        };
        let Some(entry) = sessions.get(&session) else {
            return rooms;
        };
        for room in ChatRoom::ALL {
            if entry.public_rooms.can_join(room) {
                rooms.insert(room);
            }
        }
        rooms
    }

    fn public_begin(&self, admitted: AdmittedSession) -> bool {
        let Ok(mut sessions) = self.sessions.lock() else {
            return false;
        };
        let Some(entry) = sessions.get_mut(&admitted.game_session_id) else {
            return false;
        };
        if entry.public_rooms.admitted.as_ref().is_some_and(|current| {
            current.continuity.connection_generation > admitted.continuity.connection_generation
                || (current.continuity.connection_generation
                    == admitted.continuity.connection_generation
                    && !entry.public_rooms.matches(&admitted))
        }) || admitted
            .item_fence
            .is_some_and(|fence| entry.character != Some(*fence.character_id.as_bytes()))
        {
            return false;
        }
        entry.public_rooms.admitted = Some(admitted);
        entry.public_rooms.known = true;
        entry.public_rooms.selected = admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_CHAT_PUBLIC_CHANNELS_V1);
        entry.public_rooms.facts = None;
        // A replacement snapshot drops queued chat, including arrivals between chat_open
        // and this bound admission. Old-capability room lines cannot reach its new peer.
        entry.egress.clear();
        true
    }

    fn public_update(&self, admitted: &AdmittedSession, facts: Option<PublicChatFacts>) -> bool {
        let Ok(mut sessions) = self.sessions.lock() else {
            return false;
        };
        let Some(entry) = sessions.get_mut(&admitted.game_session_id) else {
            return false;
        };
        if !entry.public_rooms.matches(admitted) {
            return false;
        }
        entry.public_rooms.facts = facts;
        for room in ChatRoom::ALL {
            if !entry.public_rooms.can_join(room) {
                entry.rooms.remove(room);
            }
        }
        true
    }

    fn public_set_room(
        &self,
        admitted: &AdmittedSession,
        room: ChatRoom,
        open: bool,
    ) -> Option<ChatIntentResult> {
        if open && self.relay().is_none() {
            return Some(result(ChatDisposition::ChatUnavailable));
        }
        let mut sessions = self.sessions.lock().ok()?;
        let entry = sessions.get_mut(&admitted.game_session_id)?;
        if !entry.public_rooms.matches(admitted) {
            return None;
        }
        if open && !entry.public_rooms.can_join(room) {
            return Some(result(ChatDisposition::NotAllowed));
        }
        if open {
            entry.rooms.insert(room);
        } else {
            entry.rooms.remove(room);
        }
        Some(result(ChatDisposition::Ok))
    }

    fn public_show_rooms(&self, admitted: &AdmittedSession) -> ChatRoomSet {
        let Ok(mut sessions) = self.sessions.lock() else {
            return ChatRoomSet::default();
        };
        let Some(entry) = sessions.get_mut(&admitted.game_session_id) else {
            return ChatRoomSet::default();
        };
        if !entry.public_rooms.matches(admitted) {
            return ChatRoomSet::default();
        }
        entry.rooms_shown = entry.rooms;
        entry.rooms
    }

    /// The ordinary room-set delta is bound just like queued room lines.
    pub(in crate::gameplay_transport) fn take_public_rooms_change(
        &self,
        admitted: &AdmittedSession,
    ) -> Option<ChatRoomSet> {
        let mut sessions = self.sessions.lock().ok()?;
        let entry = sessions.get_mut(&admitted.game_session_id)?;
        if !entry.public_rooms.matches(admitted) || entry.rooms == entry.rooms_shown {
            return None;
        }
        entry.rooms_shown = entry.rooms;
        Some(entry.rooms)
    }

    /// The existing drain calls this for public room candidates. A stale connection never
    /// consumes the replacement connection's queue, nor receives its newly negotiated rooms.
    pub(in crate::gameplay_transport) fn accept_public_room_line(
        &self,
        admitted: &AdmittedSession,
        candidate: &ChatLine,
    ) -> Option<ChatLine> {
        let ChatLine::Room { room, .. } = candidate else {
            return None;
        };
        let mut sessions = self.sessions.lock().ok()?;
        let entry = sessions.get_mut(&admitted.game_session_id)?;
        if !entry.public_rooms.matches(admitted) || entry.egress.peek() != Some(candidate) {
            return None;
        }
        if entry.public_rooms.can_join(*room) && entry.rooms.contains(*room) {
            entry.egress.next_for_write()
        } else {
            entry.egress.next_for_write();
            entry.egress.delivered();
            None
        }
    }

    fn public_name(&self, admitted: &AdmittedSession) -> Option<CharacterName> {
        let sessions = self.sessions.lock().ok()?;
        let entry = sessions.get(&admitted.game_session_id)?;
        if !entry.public_rooms.matches(admitted) {
            return None;
        }
        CharacterName::parse(&entry.name).ok()
    }

    fn public_send_gate(
        &self,
        admitted: &AdmittedSession,
        room: ChatRoom,
        now: SemanticTimeMicros,
    ) -> Option<ChatIntentResult> {
        if !self.relay_ready() {
            return Some(result(ChatDisposition::ChatUnavailable));
        }
        let Ok(mut sessions) = self.sessions.lock() else {
            return Some(result(ChatDisposition::ChatUnavailable));
        };
        let Some(entry) = sessions.get_mut(&admitted.game_session_id) else {
            return Some(result(ChatDisposition::ChatUnavailable));
        };
        if !entry.public_rooms.matches(admitted) {
            return Some(result(ChatDisposition::Rejected));
        }
        if !entry.rooms.contains(room) {
            return Some(result(ChatDisposition::RoomNotOpen));
        }
        // Global spam mute precedes the room-specific cooldown, as it does for mainland
        // Advertising in the shared limiter. This branch cannot consume a bucket line.
        if entry.limiter.is_muted(now) {
            return entry.limiter.pre_gate_relay(false, now).err().map(refusal);
        }
        let gm = entry
            .public_rooms
            .facts
            .is_some_and(|f| f.role.group_id >= 4);
        if !gm
            && room == ChatRoom::AdvertisingRookgaard
            && let Some(until) = entry
                .public_rooms
                .rook_ready_at
                .filter(|until| *until > now)
        {
            return Some(ChatIntentResult {
                disposition: ChatDisposition::Exhausted,
                wait_seconds: u32::try_from((until.get() - now.get()).div_ceil(1_000_000))
                    .unwrap_or(120),
            });
        }
        entry
            .limiter
            .pre_gate_relay(!gm && room == ChatRoom::Advertising, now)
            .err()
            .map(refusal)
    }

    fn public_claim_interval(
        &self,
        admitted: &AdmittedSession,
        room: ChatRoom,
        now: SemanticTimeMicros,
    ) -> Option<ChatIntentResult> {
        let Ok(mut sessions) = self.sessions.lock() else {
            return Some(result(ChatDisposition::ChatUnavailable));
        };
        let Some(entry) = sessions.get_mut(&admitted.game_session_id) else {
            return Some(result(ChatDisposition::ChatUnavailable));
        };
        if !entry.public_rooms.matches(admitted) {
            return Some(result(ChatDisposition::Rejected));
        }
        if entry
            .public_rooms
            .facts
            .is_some_and(|f| f.role.group_id >= 4)
        {
            return None;
        }
        if room == ChatRoom::Advertising {
            return entry.limiter.claim_advertising(now).err().map(refusal);
        }
        if room == ChatRoom::AdvertisingRookgaard {
            if let Some(until) = entry
                .public_rooms
                .rook_ready_at
                .filter(|until| *until > now)
            {
                return Some(ChatIntentResult {
                    disposition: ChatDisposition::Exhausted,
                    wait_seconds: u32::try_from((until.get() - now.get()).div_ceil(1_000_000))
                        .unwrap_or(120),
                });
            }
            // Crystal advertising-rook.lua:9, independent of mainland channel 5.
            entry.public_rooms.rook_ready_at = Some(SemanticTimeMicros::from_micros(
                now.get().saturating_add(120_000_000),
            ));
        }
        None
    }
}

fn help_command(text: &str) -> Option<(HelpAction, &str)> {
    text.strip_prefix("!mute ")
        .map(|name| (HelpAction::Mute, name))
        .or_else(|| {
            text.strip_prefix("!unmute ")
                .map(|name| (HelpAction::Unmute, name))
        })
}

impl ComposedFreshAdmission<'_, '_, '_> {
    async fn public_facts(
        &self,
        admitted: AdmittedSession,
    ) -> Option<(CurrentCharacterItemFence, PublicChatAccess, PublicChatFacts)> {
        let fence = admitted.item_fence?;
        let actor = admitted.runtime_actor?;
        if fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
        {
            return None;
        }
        let (level, has_vocation) = {
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
            {
                return None;
            }
            let states = self.states.spell_states.lock().await;
            match states.get(&runtime, actor, admitted.game_session_id) {
                Some(state) => (state.character_facts().level, true),
                None => (self.states.chat.level(admitted.game_session_id), false),
            }
        };
        let access = self
            .root
            .read_public_chat_access(self.character, self.holder, fence)
            .await
            .ok()??;
        let facts = PublicChatFacts {
            role: access.role,
            level,
            has_vocation,
            premium: false,
            guild_leader: access.guild_leader,
        };
        Some((fence, access, facts))
    }

    pub(in crate::gameplay_transport) async fn public_chat_admit(
        &self,
        admitted: AdmittedSession,
        _rooms: ChatRoomSet,
    ) -> ChatRoomSet {
        if !self.states.chat.public_begin(admitted) {
            return ChatRoomSet::default();
        }
        let facts = self.public_facts(admitted).await.map(|(_, _, facts)| facts);
        if !self.states.chat.public_update(&admitted, facts) {
            return ChatRoomSet::default();
        }
        self.states.chat.public_show_rooms(&admitted)
    }

    /// Called by the existing CHAT wire adapter with the original admitted connection fence.
    /// All public-room intents finish here; other chat owners keep their existing paths.
    pub(in crate::gameplay_transport) async fn public_chat_preflight(
        &self,
        admitted: AdmittedSession,
        command_id: u64,
        intent: &ChatIntent,
    ) -> Option<Option<ChatIntentResult>> {
        if !matches!(
            intent,
            ChatIntent::OpenRoom(_) | ChatIntent::CloseRoom(_) | ChatIntent::Room { .. }
        ) {
            return None;
        }
        let help = match intent {
            ChatIntent::Room {
                room: ChatRoom::Help,
                text,
            } => ChatText::parse(text).ok().and_then(|text| {
                help_command(text.as_str()).map(|(action, name)| (action, name.to_owned()))
            }),
            _ => None,
        };
        // A durable Help result precedes current role/mute/room gates. In particular,
        // a role revocation after COMMIT cannot turn its unresolved result into a refusal.
        if let Some((action, target)) = &help {
            let Some(fence) = admitted.item_fence else {
                return Some(None);
            };
            if fence.game_session_id != admitted.game_session_id
                || fence.connection_generation.get() != admitted.continuity.connection_generation
                || fence.runtime_scope
                    != crate::foundation::RuntimeScopeRefV1::channel(
                        admitted.world_id,
                        admitted.channel_id,
                    )
            {
                return Some(None);
            }
            match self
                .root
                .read_help_channel_receipt(
                    self.character,
                    self.holder,
                    fence,
                    command_id,
                    *action,
                    target.clone(),
                )
                .await
            {
                Ok(HelpReceipt::Retained(result)) => return Some(Some(result)),
                Ok(HelpReceipt::Unrecorded) => {}
                Ok(HelpReceipt::StaleFence) | Err(_) => return Some(None),
            }
        }
        Some(
            self.public_chat_intent(admitted, command_id, intent, help.as_ref())
                .await,
        )
    }

    async fn public_help_refusal(
        &self,
        fence: CurrentCharacterItemFence,
        command_id: u64,
        request: Option<&(HelpAction, String)>,
        refusal: ChatIntentResult,
    ) -> Option<ChatIntentResult> {
        let Some((action, target)) = request else {
            return Some(refusal);
        };
        self.root
            .refuse_help_channel(
                self.character,
                self.holder,
                fence,
                command_id,
                (*action, target.clone()),
                refusal,
            )
            .await
            .ok()
            .flatten()
    }

    async fn public_chat_intent(
        &self,
        admitted: AdmittedSession,
        command_id: u64,
        intent: &ChatIntent,
        help: Option<&(HelpAction, String)>,
    ) -> Option<ChatIntentResult> {
        let room = match intent {
            ChatIntent::OpenRoom(room)
            | ChatIntent::CloseRoom(room)
            | ChatIntent::Room { room, .. } => *room,
            _ => return None,
        };
        let selected = admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_CHAT_PUBLIC_CHANNELS_V1);
        if room.requires_public_channels() && !selected {
            return Some(result(ChatDisposition::Rejected));
        }
        let session = admitted.game_session_id;
        let Some((fence, access, mut facts)) = self.public_facts(admitted).await else {
            self.states.chat.public_update(&admitted, None);
            // Without current role facts we cannot classify a command-looking Help line
            // as ordinary speech or a moderator mutation. Preserve its pending identity.
            return help
                .is_none()
                .then(|| result(ChatDisposition::ChatUnavailable));
        };
        if !self.states.chat.public_update(&admitted, Some(facts)) {
            return None;
        }
        if matches!(intent, ChatIntent::CloseRoom(_)) {
            return self.states.chat.public_set_room(&admitted, room, false);
        }
        if !facts.can_join(room_rule(room)) {
            return Some(result(ChatDisposition::NotAllowed));
        }
        if matches!(intent, ChatIntent::OpenRoom(_)) {
            return self.states.chat.public_set_room(&admitted, room, true);
        }
        let ChatIntent::Room { text, .. } = intent else {
            return Some(result(ChatDisposition::Rejected));
        };
        let Ok(text) = ChatText::parse(text) else {
            return Some(result(ChatDisposition::Rejected));
        };
        // Crystal treats these strings as ordinary text for group1. Only a recognized
        // moderator command journals terminal preflight refusals in the Help owner.
        let moderation = help.filter(|_| facts.role.group_id >= 2);
        if let Some(wait_seconds) = self.staff_mute_wait(session).await {
            return self
                .public_help_refusal(
                    fence,
                    command_id,
                    moderation,
                    ChatIntentResult {
                        disposition: ChatDisposition::Muted,
                        wait_seconds,
                    },
                )
                .await;
        }
        let now = self.owner_now();
        if let Some(refused) = self.states.chat.public_send_gate(&admitted, room, now) {
            return self
                .public_help_refusal(fence, command_id, moderation, refused)
                .await;
        }
        if !facts.can_speak(room_rule(room))
            && matches!(room, ChatRoom::English | ChatRoom::Advertising)
        {
            facts.premium = self.session_premium_gate(session).await.premium_current();
        }
        if !facts.can_speak(room_rule(room)) {
            return self
                .public_help_refusal(
                    fence,
                    command_id,
                    moderation,
                    result(ChatDisposition::LevelTooLow),
                )
                .await;
        }
        if room == ChatRoom::Help && access.help_wait_seconds > 0 {
            return self
                .public_help_refusal(
                    fence,
                    command_id,
                    moderation,
                    ChatIntentResult {
                        disposition: ChatDisposition::Muted,
                        wait_seconds: access.help_wait_seconds,
                    },
                )
                .await;
        }
        let Some(name) = self.states.chat.public_name(&admitted) else {
            return self
                .public_help_refusal(
                    fence,
                    command_id,
                    moderation,
                    result(ChatDisposition::ChatUnavailable),
                )
                .await;
        };
        let Some(relay) = self.states.chat.relay() else {
            return self
                .public_help_refusal(
                    fence,
                    command_id,
                    moderation,
                    result(ChatDisposition::ChatUnavailable),
                )
                .await;
        };
        if let Some((action, target)) = moderation {
            let Ok(target) = CharacterName::parse(target) else {
                return self
                    .public_help_refusal(
                        fence,
                        command_id,
                        moderation,
                        result(ChatDisposition::NotOnline),
                    )
                    .await;
            };
            return Some(
                match self
                    .root
                    .mutate_help_channel(
                        self.character,
                        self.holder,
                        fence,
                        command_id,
                        *action,
                        target,
                    )
                    .await
                {
                    Ok(Some(done)) => {
                        if done.applied
                            && let Some(target) = done.target_name
                        {
                            let announcement = match *action {
                                HelpAction::Mute => format!(
                                    "{target} has been muted by {} for using Help Channel inappropriately.",
                                    name.as_str()
                                ),
                                HelpAction::Unmute => format!("{target} has been unmuted."),
                            };
                            if let Ok(announcement) = ChatText::parse(&announcement) {
                                // The durable result is final; relay delivery remains at most once.
                                let _ = relay
                                    .send(RelayDestination::Room { room }, name, announcement)
                                    .await;
                            }
                        }
                        done.result
                    }
                    Ok(None) => return None, // Stale authority supplies no terminal receipt.
                    Err(_) => return None, // The writer may have committed: retain pending ingress.
                },
            );
        }
        if let Some(refused) = self.states.chat.public_claim_interval(&admitted, room, now) {
            return Some(refused);
        }
        Some(
            match relay
                .send(RelayDestination::Room { room }, name, text)
                .await
            {
                Ok(()) => result(ChatDisposition::Ok),
                Err(disposition) => result(disposition),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn peer(
        session: GameSessionId,
        generation: u64,
        selected: bool,
    ) -> Result<AdmittedSession, Box<dyn std::error::Error>> {
        use crate::gameplay_transport::connection::{FirstEntryOutcome, SessionContinuity};
        let mut continuity = SessionContinuity::FRESH;
        continuity.connection_generation = generation;
        continuity.selected_capabilities =
            crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
                if selected {
                    &[6, 7, 80, 259]
                } else {
                    &[6, 7, 80]
                },
            )
            .ok_or("capability selection")?;
        Ok(AdmittedSession {
            game_session_id: session,
            world_id: crate::foundation::WorldId::decode(session.as_bytes())?,
            channel_id: crate::foundation::ChannelId::decode(session.as_bytes())?,
            runtime_actor: None,
            first_entry: FirstEntryOutcome::Positioned,
            controller: None,
            continuity,
            item_fence: None,
        })
    }
    #[test]
    fn public_help_commands_require_exact_prefix_and_preserve_target() {
        assert_eq!(help_command("!mute Aela"), Some((HelpAction::Mute, "Aela")));
        assert_eq!(
            help_command("!unmute Aela"),
            Some((HelpAction::Unmute, "Aela"))
        );
        for text in ["!mute", "!MUTE Aela", "x!mute Aela", "!unmute"] {
            assert_eq!(help_command(text), None);
        }
    }

    #[test]
    fn public_capability_offer_closes_dependencies_without_a_new_route() {
        use crate::gameplay_transport::capabilities::{self, SelectedCapabilities};
        let offered = capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES;
        assert!(
            offered
                .iter()
                .any(|entry| entry.id == CAPABILITY_CHAT_PUBLIC_CHANNELS_V1)
        );
        for unsupported in [
            capabilities::PRODUCTION_OFFERED_CAPABILITIES,
            capabilities::ITEM_VIEW_OFFERED_CAPABILITIES,
        ] {
            assert!(
                !unsupported
                    .iter()
                    .any(|entry| entry.id == CAPABILITY_CHAT_PUBLIC_CHANNELS_V1)
            );
        }
        for supported in [&[259][..], &[7, 259], &[80, 259], &[7, 80, 259]] {
            let selected = SelectedCapabilities::select(offered, supported);
            assert_eq!(
                selected
                    .is_some_and(|selected| selected.contains(CAPABILITY_CHAT_PUBLIC_CHANNELS_V1)),
                supported.contains(&7) && supported.contains(&80)
            );
        }
    }
    #[test]
    fn public_room_negotiation_never_admits_new_rooms_to_an_old_peer() {
        let mut state = PublicRoomState::default();
        assert!(state.can_join(ChatRoom::Help));
        assert!(!state.can_join(ChatRoom::Tutor));
        state.known = true;
        state.facts = Some(PublicChatFacts {
            role: crate::durability::chat_public::ChatRole {
                group_id: 4,
                account_type: 4,
                access: true,
            },
            level: 20,
            has_vocation: true,
            premium: false,
            guild_leader: false,
        });
        assert!(!state.can_join(ChatRoom::Tutor));
        state.selected = true;
        assert!(state.can_join(ChatRoom::Tutor));
        assert!(state.can_join(ChatRoom::Gamemaster));
        state.facts = None;
        assert!(!state.can_join(ChatRoom::Help));
    }

    #[test]
    fn public_advertising_intervals_are_independent_and_gamemasters_bypass_them()
    -> Result<(), Box<dyn std::error::Error>> {
        let session =
            GameSessionId::decode(&[1, 144, 0, 0, 0, 1, 112, 0, 128, 0, 0, 0, 0, 0, 0, 1])?;
        let chat = ChatRuntime::default();
        let now = SemanticTimeMicros::from_micros(10_000_000);
        chat.admit_as(session, "Aela", None, now);
        let admitted = peer(session, 1, true)?;
        assert!(chat.public_begin(admitted));
        assert_eq!(
            chat.public_claim_interval(&admitted, ChatRoom::AdvertisingRookgaard, now),
            None
        );
        assert_eq!(
            chat.public_claim_interval(&admitted, ChatRoom::Advertising, now),
            None
        );
        for room in [ChatRoom::Advertising, ChatRoom::AdvertisingRookgaard] {
            assert_eq!(
                chat.public_claim_interval(&admitted, room, now),
                Some(ChatIntentResult {
                    disposition: ChatDisposition::Exhausted,
                    wait_seconds: 120
                })
            );
        }
        let before = SemanticTimeMicros::from_micros(now.get() + 119_999_999);
        assert_eq!(
            chat.public_claim_interval(&admitted, ChatRoom::AdvertisingRookgaard, before),
            Some(ChatIntentResult {
                disposition: ChatDisposition::Exhausted,
                wait_seconds: 1
            })
        );
        let exact = SemanticTimeMicros::from_micros(now.get() + 120_000_000);
        assert_eq!(
            chat.public_claim_interval(&admitted, ChatRoom::AdvertisingRookgaard, exact),
            None
        );
        assert!(chat.public_update(
            &admitted,
            Some(PublicChatFacts {
                role: crate::durability::chat_public::ChatRole {
                    group_id: 4,
                    account_type: 4,
                    access: true
                },
                level: 1,
                has_vocation: false,
                premium: false,
                guild_leader: false
            })
        ));
        for room in [ChatRoom::Advertising, ChatRoom::AdvertisingRookgaard] {
            assert_eq!(chat.public_claim_interval(&admitted, room, exact), None);
        }
        Ok(())
    }

    #[test]
    fn public_global_spam_mute_precedes_rookgaard_interval()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::str::FromStr as _;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let _context = runtime.enter();
        let bytes = [1, 144, 0, 0, 0, 1, 112, 0, 128, 0, 0, 0, 0, 0, 0, 1];
        let session = GameSessionId::decode(&bytes)?;
        let chat = ChatRuntime::default();
        let now = SemanticTimeMicros::from_micros(1_000_000);
        let ring = crate::chat::relay::RelayKeyRing::new(vec![crate::chat::relay::RelayKey::new(
            1, 0, [1; 32],
        )])?;
        let options =
            sqlx::postgres::PgConnectOptions::from_str("postgresql://nobody@127.0.0.1:1/none")?;
        let relay = ChatRelay::new(ring, bytes, options)
            .map_err(|error| format!("relay fixture: {error:?}"))?;
        relay.force_ready(now.get());
        chat.install_relay(Arc::new(relay));
        chat.admit_as(session, "Aela", None, now);
        let admitted = peer(session, 1, true)?;
        assert!(chat.public_begin(admitted));
        assert!(chat.public_update(
            &admitted,
            Some(PublicChatFacts {
                role: crate::durability::chat_public::ChatRole::default(),
                level: 20,
                has_vocation: false,
                premium: false,
                guild_leader: false
            })
        ));
        for room in [ChatRoom::Help, ChatRoom::AdvertisingRookgaard] {
            assert_eq!(
                chat.set_room(session, room, true).disposition,
                ChatDisposition::Ok
            );
        }
        assert_eq!(
            chat.public_claim_interval(&admitted, ChatRoom::AdvertisingRookgaard, now),
            None
        );
        for _ in 0..4 {
            assert_eq!(chat.public_send_gate(&admitted, ChatRoom::Help, now), None);
        }
        let muted = Some(ChatIntentResult {
            disposition: ChatDisposition::Muted,
            wait_seconds: 5,
        });
        assert_eq!(chat.public_send_gate(&admitted, ChatRoom::Help, now), muted);
        assert_eq!(
            chat.public_send_gate(&admitted, ChatRoom::AdvertisingRookgaard, now),
            muted
        );
        Ok(())
    }

    #[test]
    fn public_old_read_cannot_restore_rooms_after_legacy_reconnect()
    -> Result<(), Box<dyn std::error::Error>> {
        let session =
            GameSessionId::decode(&[1, 144, 0, 0, 0, 1, 112, 0, 128, 0, 0, 0, 0, 0, 0, 1])?;
        let chat = ChatRuntime::default();
        chat.admit_as(
            session,
            "Aela",
            None,
            SemanticTimeMicros::from_micros(1_000_000),
        );
        let old = peer(session, 1, true)?;
        let current = peer(session, 2, false)?;
        let facts = PublicChatFacts {
            role: crate::durability::chat_public::ChatRole {
                group_id: 4,
                account_type: 4,
                access: true,
            },
            level: 20,
            has_vocation: true,
            premium: false,
            guild_leader: false,
        };
        assert!(chat.public_begin(old));
        assert!(chat.public_update(&old, Some(facts)));
        let tutor_line = ChatLine::Room {
            room: ChatRoom::Tutor,
            speaker_name: "Aela".into(),
            text: "old queue".into(),
        };
        {
            let mut sessions = chat.sessions.lock().map_err(|_| "sessions poisoned")?;
            let entry = sessions.get_mut(&session).ok_or("chat session")?;
            entry.rooms.insert(ChatRoom::Tutor);
            entry.egress.push(tutor_line);
        }
        assert!(chat.public_begin(current));
        assert!(chat.public_update(&current, Some(facts)));
        // Complete the old SQL read after the newer legacy connection has admitted.
        assert!(!chat.public_update(&old, Some(facts)));
        assert!(!chat.public_begin(old));
        assert_eq!(chat.public_name(&old), None);
        assert_eq!(chat.public_set_room(&old, ChatRoom::Tutor, false), None);
        let line = ChatLine::Room {
            room: ChatRoom::Help,
            speaker_name: "Aela".into(),
            text: "current queue".into(),
        };
        {
            let mut sessions = chat.sessions.lock().map_err(|_| "sessions poisoned")?;
            let entry = sessions.get_mut(&session).ok_or("chat session")?;
            assert!(entry.public_rooms.matches(&current));
            assert_eq!(entry.egress.peek(), None);
            for room in [
                ChatRoom::Tutor,
                ChatRoom::Gamemaster,
                ChatRoom::GuildLeaders,
                ChatRoom::AdvertisingRookgaard,
            ] {
                assert!(!entry.public_rooms.can_join(room));
                assert!(!entry.rooms.contains(room));
            }
            entry.rooms.insert(ChatRoom::Help);
            entry.egress.push(line.clone());
        }
        assert_eq!(chat.accept_public_room_line(&old, &line), None);
        assert_eq!(chat.accept_public_room_line(&current, &line), Some(line));
        assert_eq!(chat.take_public_rooms_change(&old), None);
        assert!(
            chat.take_public_rooms_change(&current)
                .is_some_and(|rooms| rooms.contains(ChatRoom::Help))
        );
        let newer = peer(session, 3, true)?;
        assert!(chat.public_begin(newer));
        assert!(chat.public_update(&newer, Some(facts)));
        {
            let mut sessions = chat.sessions.lock().map_err(|_| "sessions poisoned")?;
            sessions
                .get_mut(&session)
                .ok_or("chat session")?
                .rooms
                .insert(ChatRoom::Tutor);
        }
        // A replaced legacy connection neither sends nor consumes the selected peer's bit 6.
        assert_eq!(chat.take_public_rooms_change(&current), None);
        assert!(
            chat.take_public_rooms_change(&newer)
                .is_some_and(|rooms| rooms.contains(ChatRoom::Tutor))
        );
        assert_eq!(chat.take_public_rooms_change(&newer), None);
        Ok(())
    }
}
