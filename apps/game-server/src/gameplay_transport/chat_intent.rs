//! `CHAT_INTENT` for local speech, private messages and World rooms (CHAT-WIRE-1, CHAT-2B,
//! CHAT-0 §3, §5 and §7).
//!
//! A thin runtime over the pure rules of `crate::chat`: one entry per admitted GameSession holds
//! the speaker name, the spam limiter, the open rooms and the undelivered-line queue. A `say`,
//! `whisper` or `yell` is admitted by the speaker's limiter and queued for every session of the
//! Channel that hears it; each connection drains its own queue into domain 12 deltas. Private and
//! room lines pass the same limiter before any lock, then go out over the World relay
//! (`super::chat_relay`), which hands every line it receives to [`ChatRuntime::deliver`]. Without
//! a configured, listening relay they answer `CHAT_UNAVAILABLE`. Nothing here is durable.

use std::collections::HashMap;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use oteryn_protocol_oteryn::chat::{
    ChatDisposition, ChatIntent, ChatIntentResult, ChatLine, ChatRoom, ChatSpeaker, ChatSpeechMode,
};
use oteryn_protocol_oteryn::npc_service::{MAX_OPEN_CONVERSATIONS_PER_NPC, TALK_RANGE_TILES};
use oteryn_simulation_determinism::SemanticTimeMicros;

use super::actor_spell::ChannelSpellStates;
use super::chat_relay::ChatRelay;
use super::npc_talk::{NpcTalkBook, TalkNpc, talk};
use super::{ComposedFreshAdmission, FreshAdmissionStore};
use crate::chat::relay::{RelayDestination, RelayLine};
use crate::chat::{
    ChatEgress, ChatLimiter, ChatPosition, ChatRefusal, ChatText, SpeechMode, YellGate, local_line,
    local_listeners, private_line, room_line,
};
use crate::domain;
use crate::domain::character_name::CharacterName;
use crate::domain::premium::PremiumGate;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
use oteryn_protocol_oteryn::chat::ChatRoomSet;

/// How often a connection drains the lines queued for its session.
pub(crate) const CHAT_REFRESH: std::time::Duration = std::time::Duration::from_millis(250);

/// The result of an intent that changes nothing.
const fn result(disposition: ChatDisposition) -> ChatIntentResult {
    ChatIntentResult {
        disposition,
        wait_seconds: 0,
    }
}

struct ChatSession {
    name: String,
    /// The session's CharacterId, the key of the private lines it receives; `None` receives none.
    character: Option<[u8; 16]>,
    limiter: ChatLimiter,
    /// The rooms the session has open, and the set its client was last told.
    rooms: ChatRoomSet,
    rooms_shown: ChatRoomSet,
    egress: ChatEgress,
    /// The NPC this session is in conversation with (NPC-TALK-1).
    engaged: Option<ExactActorRef>,
    /// When the session joined its NPC's customer queue: the newest customer has the highest
    /// (NPC-TALK-2, NPC-BEHAVIOUR-0 §7).
    joined: u64,
    /// The walk-away lines of the conversations moves closed, oldest first, for the next domain 7
    /// payload.
    walked_away: Vec<String>,
    /// The session selected capability 3: NPCs answer its `say` (NPC-CMD-1a).
    npc_service: bool,
}

/// The chat state of the admitted sessions of one Channel. The lock is never held across an
/// await and is always taken after the runtime and spell-state locks.
#[derive(Default)]
pub(crate) struct ChatRuntime {
    sessions: Mutex<HashMap<GameSessionId, ChatSession>>,
    /// What the NPCs of the Channel say; unset until the boot wiring installs it.
    npc_talk: OnceLock<Arc<NpcTalkBook>>,
    /// The World relay; unset on a node without one, which refuses room and private lines.
    relay: OnceLock<Arc<ChatRelay>>,
    /// The last customer queue position handed out.
    joins: AtomicU64,
}

impl ChatRuntime {
    /// [`Self::admit_as`] for a session that receives no private line.
    #[cfg(test)]
    pub(crate) fn admit(&self, session: GameSessionId, name: &str, now: SemanticTimeMicros) {
        self.admit_as(session, name, None, now);
    }

    /// Admission, reconnect or transfer of `session` playing `character`: keeps its limiter (a
    /// mute survives a reconnect) and its open rooms, drops undelivered lines (a replacement
    /// snapshot drops lines, never replays).
    pub(crate) fn admit_as(
        &self,
        session: GameSessionId,
        name: &str,
        character: Option<[u8; 16]>,
        now: SemanticTimeMicros,
    ) {
        if let Ok(mut sessions) = self.sessions.lock() {
            let entry = sessions.entry(session).or_insert_with(|| ChatSession {
                name: name.to_owned(),
                character,
                limiter: ChatLimiter::new(now),
                rooms: ChatRoomSet::default(),
                rooms_shown: ChatRoomSet::default(),
                egress: ChatEgress::new(),
                engaged: None,
                joined: 0,
                walked_away: Vec::new(),
                npc_service: false,
            });
            entry.name = name.to_owned();
            entry.character = character;
            entry.egress.clear();
            entry.engaged = None;
            entry.walked_away.clear();
            entry.npc_service = false;
        }
    }

    /// Installs the World relay once; a second install is ignored.
    pub(crate) fn install_relay(&self, relay: Arc<ChatRelay>) {
        let _ = self.relay.set(relay);
    }

    /// The World relay, once installed.
    pub(crate) fn relay(&self) -> Option<&ChatRelay> {
        self.relay.get().map(AsRef::as_ref)
    }

    fn relay_ready(&self) -> bool {
        self.relay().is_some_and(ChatRelay::is_ready)
    }

    /// The open rooms of `session`, which its client is now shown (a snapshot).
    pub(crate) fn show_rooms(&self, session: GameSessionId) -> ChatRoomSet {
        let Ok(mut sessions) = self.sessions.lock() else {
            return ChatRoomSet::default();
        };
        let Some(entry) = sessions.get_mut(&session) else {
            return ChatRoomSet::default();
        };
        entry.rooms_shown = entry.rooms;
        entry.rooms
    }

    /// The open rooms of `session` when they differ from what its client was last shown; the
    /// client is shown them from now on (a domain 12 delta type 2).
    pub(crate) fn take_rooms_change(&self, session: GameSessionId) -> Option<ChatRoomSet> {
        let mut sessions = self.sessions.lock().ok()?;
        let entry = sessions.get_mut(&session)?;
        (entry.rooms != entry.rooms_shown).then(|| {
            entry.rooms_shown = entry.rooms;
            entry.rooms
        })
    }

    /// Opens or closes `room` for `session`: no Channel state, no line. Closing a room that is
    /// not open is `OK`; without a configured relay no room can be opened.
    fn set_room(&self, session: GameSessionId, room: ChatRoom, open: bool) -> ChatIntentResult {
        if open && self.relay().is_none() {
            return result(ChatDisposition::ChatUnavailable);
        }
        let Ok(mut sessions) = self.sessions.lock() else {
            return result(ChatDisposition::ChatUnavailable);
        };
        let Some(entry) = sessions.get_mut(&session) else {
            return result(ChatDisposition::ChatUnavailable);
        };
        if open {
            entry.rooms.insert(room);
        } else {
            entry.rooms.remove(room);
        }
        result(ChatDisposition::Ok)
    }

    /// Delivers one line received from the World relay to the sessions of this node it is for:
    /// a private line to the session playing its recipient, a room line to every session that
    /// has the room open. Nothing is written or logged.
    pub(crate) fn deliver(&self, line: &RelayLine) {
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        let name = line.sender_name.as_str();
        match line.destination {
            RelayDestination::Private { recipient } => {
                for entry in sessions.values_mut() {
                    if entry.character.as_ref() == Some(recipient.as_bytes()) {
                        entry.egress.push(private_line(name, &line.text));
                    }
                }
            }
            RelayDestination::Room { room } => {
                for entry in sessions.values_mut() {
                    if entry.rooms.contains(room) {
                        entry.egress.push(room_line(room, name, &line.text));
                    }
                }
            }
        }
    }

    /// Installs the NPC talk of the Channel once; a second install is ignored.
    pub(crate) fn install_npc_talk(&self, book: Arc<NpcTalkBook>) {
        let _ = self.npc_talk.set(book);
    }

    /// What the NPCs of the Channel say, once installed.
    pub(crate) fn npc_talk(&self) -> Option<&NpcTalkBook> {
        self.npc_talk.get().map(AsRef::as_ref)
    }

    pub(crate) fn knows(&self, session: GameSessionId) -> bool {
        self.sessions
            .lock()
            .is_ok_and(|sessions| sessions.contains_key(&session))
    }

    /// A reconnect of a known session: drops undelivered lines, keeps the name and limiter.
    pub(crate) fn refresh(&self, session: GameSessionId) {
        if let Ok(mut sessions) = self.sessions.lock()
            && let Some(entry) = sessions.get_mut(&session)
        {
            entry.egress.clear();
            entry.engaged = None;
            entry.walked_away.clear();
            entry.npc_service = false;
        }
    }

    /// `session` selected capability 3 (admission, reconnect or transfer): NPCs answer its
    /// `say`, and it starts without a conversation, as its domain 7 snapshot is empty.
    pub(crate) fn set_npc_service(&self, session: GameSessionId) {
        if let Ok(mut sessions) = self.sessions.lock()
            && let Some(entry) = sessions.get_mut(&session)
        {
            entry.npc_service = true;
            entry.engaged = None;
            entry.walked_away.clear();
        }
    }

    /// The NPC `session` is in conversation with.
    pub(crate) fn engaged(&self, session: GameSessionId) -> Option<ExactActorRef> {
        self.sessions
            .lock()
            .ok()
            .and_then(|sessions| sessions.get(&session).and_then(|entry| entry.engaged))
    }

    /// Ends the conversation of `session` when it is held with `npc`.
    pub(crate) fn end_engagement(&self, session: GameSessionId, npc: ExactActorRef) -> bool {
        let Ok(mut sessions) = self.sessions.lock() else {
            return false;
        };
        match sessions.get_mut(&session) {
            Some(entry) if entry.engaged == Some(npc) => {
                entry.engaged = None;
                true
            }
            _ => false,
        }
    }

    /// Runs `step` on the conversation of `session`. A step that opens a conversation with an NPC
    /// that already has `NPC0-RL-05` customers is undone: `None`, as for an unknown session.
    pub(crate) fn converse<R>(
        &self,
        session: GameSessionId,
        step: impl FnOnce(&mut Option<ExactActorRef>) -> R,
    ) -> Option<R> {
        let mut sessions = self.sessions.lock().ok()?;
        let entry = sessions.get_mut(&session)?;
        let prior = entry.engaged;
        let result = step(&mut entry.engaged);
        if let Some(npc) = entry.engaged
            && prior != Some(npc)
        {
            if crowded(&sessions, session, npc) {
                sessions.get_mut(&session)?.engaged = prior;
                return None;
            }
            sessions.get_mut(&session)?.joined = self.next_join();
        }
        Some(result)
    }

    fn next_join(&self) -> u64 {
        self.joins.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// The newest customer of `npc`, the tail of its queue: the one the NPC faces
    /// (NPC-BEHAVIOUR-0 §7; GAME-AI reads it to turn the NPC).
    #[allow(dead_code, reason = "GAME-AI reads it to turn the NPC (NPC-ACTOR-1)")]
    pub(crate) fn newest_customer(&self, npc: ExactActorRef) -> Option<GameSessionId> {
        let sessions = self.sessions.lock().ok()?;
        sessions
            .iter()
            .filter(|(_, entry)| entry.engaged == Some(npc))
            .max_by_key(|(_, entry)| entry.joined)
            .map(|(session, _)| *session)
    }

    /// The range check after a committed move or relocation of the customer `session`
    /// (NPC-BEHAVIOUR-0 §7, `NPC0-RL-07`): a conversation whose NPC is gone, on another floor or
    /// more than [`TALK_RANGE_TILES`] tiles away closes, and the NPC's walk-away line waits for
    /// the session's next domain 7 payload. Whether it closed.
    pub(crate) fn close_out_of_range(
        &self,
        runtime: &ChannelRuntimeV1,
        session: GameSessionId,
    ) -> bool {
        let Some(npc) = self.engaged(session) else {
            return false;
        };
        let customer = runtime
            .positioned_player_for_session(session)
            .ok()
            .flatten()
            .map(|(_, snapshot)| snapshot.position());
        let visible = runtime.visible_entities();
        let placed = visible.npcs.iter().find(|placed| placed.actor == npc);
        let near = customer.zip(placed).is_some_and(|(at, placed)| {
            at.floor == placed.position.floor
                && at.x.abs_diff(placed.position.x) <= TALK_RANGE_TILES
                && at.y.abs_diff(placed.position.y) <= TALK_RANGE_TILES
        });
        if near {
            return false;
        }
        let line = self
            .npc_talk()
            .and_then(|book| runtime.npc_key(npc).and_then(|key| book.get(key)))
            .map(|entry| entry.farewell().to_owned());
        let Ok(mut sessions) = self.sessions.lock() else {
            return false;
        };
        match sessions.get_mut(&session) {
            Some(entry) if entry.engaged == Some(npc) => {
                entry.engaged = None;
                entry.walked_away.extend(line);
                true
            }
            _ => false,
        }
    }

    /// A walk-away line of an earlier conversation, pending as a close would leave it.
    #[cfg(test)]
    pub(crate) fn push_walked_away(&self, session: GameSessionId, line: &str) {
        if let Ok(mut sessions) = self.sessions.lock()
            && let Some(entry) = sessions.get_mut(&session)
        {
            entry.walked_away.push(line.to_owned());
        }
    }

    /// The walk-away lines of the conversations moves closed since the last payload, once.
    pub(crate) fn take_walked_away(&self, session: GameSessionId) -> Vec<String> {
        self.sessions
            .lock()
            .ok()
            .and_then(|mut sessions| {
                sessions
                    .get_mut(&session)
                    .map(|entry| std::mem::take(&mut entry.walked_away))
            })
            .unwrap_or_default()
    }

    pub(crate) fn forget(&self, session: GameSessionId) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.remove(&session);
        }
    }

    /// The next undelivered line of `session`, oldest first. It counts against the undelivered
    /// bound until [`Self::delivered`].
    pub(crate) fn next(&self, session: GameSessionId) -> Option<ChatLine> {
        let mut sessions = self.sessions.lock().ok()?;
        sessions.get_mut(&session)?.egress.next_for_write()
    }

    /// Every undelivered line of `session`, oldest first, as if each were written at once.
    #[cfg(test)]
    pub(crate) fn drain(&self, session: GameSessionId) -> Vec<ChatLine> {
        let mut lines = Vec::new();
        while let Some(line) = self.next(session) {
            self.delivered(session);
            lines.push(line);
        }
        lines
    }

    /// The line last returned by [`Self::next`] reached the client: it no longer counts against
    /// the session's undelivered bound.
    pub(crate) fn delivered(&self, session: GameSessionId) {
        if let Ok(mut sessions) = self.sessions.lock()
            && let Some(entry) = sessions.get_mut(&session)
        {
            entry.egress.delivered();
        }
    }

    /// The answer of a cast by a muted `session` (CHAT-0 §6: a muted character cannot cast any
    /// spell; the existing rejected disposition, no new one). `None` when the cast may go on.
    pub(crate) fn muted_cast(
        &self,
        session: GameSessionId,
        now: SemanticTimeMicros,
    ) -> Option<super::actor_spell::SpellCastOutcome> {
        self.is_muted(session, now)
            .then(super::actor_spell::SpellCastOutcome::rejected)
    }

    /// The gate in front of the Channel lock (CHAT-0 §6): a line of a muted session or beyond its
    /// bucket (a yell included), a yell in its cooldown and a yell repeated soon after an
    /// under-level refusal (which takes its line) are refused here, before the runtime lock and
    /// the entity clone, so a flood costs the Channel only this map lookup. A private or room line
    /// is gated here too ([`Self::pre_gate_relay`]), and opening or closing a room is answered here
    /// outright. `None` lets the intent go on.
    pub(crate) fn pre_gate(
        &self,
        session: GameSessionId,
        intent: &ChatIntent,
        now: SemanticTimeMicros,
    ) -> Option<ChatIntentResult> {
        let (mode, text) = match intent {
            ChatIntent::Say { mode, text } => (mode, text),
            ChatIntent::OpenRoom(room) => return Some(self.set_room(session, *room, true)),
            ChatIntent::CloseRoom(room) => return Some(self.set_room(session, *room, false)),
            ChatIntent::Private { text, .. } => {
                return self.pre_gate_relay(session, None, text, now);
            }
            ChatIntent::Room { room, text } => {
                return self.pre_gate_relay(session, Some(*room), text, now);
            }
        };
        // Unusable text is refused here too, without the Channel and without a line, so it
        // cannot be repeated against the locks for free.
        if ChatText::parse(text).is_err() {
            return Some(result(ChatDisposition::Rejected));
        }
        // A session without chat state (its open failed) or a poisoned map ends in
        // `ChatUnavailable` on the full path too; refusing it here keeps it off the locks.
        let unavailable = || Some(result(ChatDisposition::ChatUnavailable));
        let Ok(mut sessions) = self.sessions.lock() else {
            return unavailable();
        };
        let Some(entry) = sessions.get_mut(&session) else {
            return unavailable();
        };
        entry
            .limiter
            .pre_gate(speech_mode(*mode), now)
            .err()
            .map(refusal)
    }

    /// The gate of a private line (`room` `None`) or a room line, before the Channel locks, the
    /// recipient read and the relay: unusable text is `REJECTED`, an unknown session or a relay
    /// that is not ready `CHAT_UNAVAILABLE`, a room that is not open `ROOM_NOT_OPEN`, all without
    /// a line; then the limiter (mute, the Advertising interval, the bucket) takes its line.
    fn pre_gate_relay(
        &self,
        session: GameSessionId,
        room: Option<ChatRoom>,
        text: &str,
        now: SemanticTimeMicros,
    ) -> Option<ChatIntentResult> {
        if ChatText::parse(text).is_err() {
            return Some(result(ChatDisposition::Rejected));
        }
        let unavailable = || Some(result(ChatDisposition::ChatUnavailable));
        let Ok(mut sessions) = self.sessions.lock() else {
            return unavailable();
        };
        let Some(entry) = sessions.get_mut(&session) else {
            return unavailable();
        };
        if !self.relay_ready() {
            return unavailable();
        }
        if room.is_some_and(|room| !entry.rooms.contains(room)) {
            return Some(result(ChatDisposition::RoomNotOpen));
        }
        entry
            .limiter
            .pre_gate_relay(room == Some(ChatRoom::Advertising), now)
            .err()
            .map(refusal)
    }

    /// The name of a session about to send a relay line, after recording its recipient
    /// (`CHAT0-RL-05`) or claiming the Advertising interval; a refusal when it cannot send.
    fn relay_sender(
        &self,
        session: GameSessionId,
        room: Option<ChatRoom>,
        recipient: Option<[u8; 16]>,
        now: SemanticTimeMicros,
    ) -> Result<CharacterName, ChatIntentResult> {
        let unavailable = result(ChatDisposition::ChatUnavailable);
        let mut sessions = self.sessions.lock().map_err(|_| unavailable)?;
        let entry = sessions.get_mut(&session).ok_or(unavailable)?;
        let name = CharacterName::parse(&entry.name).map_err(|_| unavailable)?;
        if let Some(recipient) = recipient {
            if entry.character == Some(recipient) {
                return Err(result(ChatDisposition::Rejected));
            }
            entry
                .limiter
                .admit_recipient(recipient, now)
                .map_err(refusal)?;
        }
        if room == Some(ChatRoom::Advertising) {
            entry.limiter.claim_advertising(now).map_err(refusal)?;
        }
        Ok(name)
    }

    /// Whether `session` is muted at `now`.
    fn is_muted(&self, session: GameSessionId, now: SemanticTimeMicros) -> bool {
        self.sessions
            .lock()
            .ok()
            .and_then(|sessions| sessions.get(&session).map(|s| s.limiter.is_muted(now)))
            .unwrap_or(false)
    }
}

fn speech_mode(mode: ChatSpeechMode) -> SpeechMode {
    match mode {
        ChatSpeechMode::Say => SpeechMode::Say,
        ChatSpeechMode::Whisper => SpeechMode::Whisper,
        ChatSpeechMode::Yell => SpeechMode::Yell,
    }
}

fn refusal(refusal: ChatRefusal) -> ChatIntentResult {
    match refusal {
        ChatRefusal::Muted { seconds } => ChatIntentResult {
            disposition: ChatDisposition::Muted,
            wait_seconds: seconds,
        },
        ChatRefusal::Exhausted { seconds } => ChatIntentResult {
            disposition: ChatDisposition::Exhausted,
            wait_seconds: seconds,
        },
        ChatRefusal::LevelTooLow => result(ChatDisposition::LevelTooLow),
    }
}

/// [`chat_in_premium_channel`] for a speaker without a Premium fence read: it reads as Free.
#[cfg(test)]
pub(crate) fn chat_in_channel(
    chat: &ChatRuntime,
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    actor: ExactActorRef,
    session: GameSessionId,
    intent: &ChatIntent,
    now: SemanticTimeMicros,
) -> ChatIntentResult {
    chat_in_premium_channel(
        chat,
        runtime,
        states,
        (actor, session),
        PremiumGate::CLOSED,
        intent,
        now,
    )
}

/// One intent of the admitted `actor`, under the runtime and spell-state locks the caller holds.
/// `premium` is the speaker's Premium gate (PREM-WIRE-1), read for the yell level gate.
pub(crate) fn chat_in_premium_channel(
    chat: &ChatRuntime,
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    (actor, session): (ExactActorRef, GameSessionId),
    premium: PremiumGate,
    intent: &ChatIntent,
    now: SemanticTimeMicros,
) -> ChatIntentResult {
    let ChatIntent::Say { mode, text } = intent else {
        // Private and room lines never take the Channel path (`ComposedFreshAdmission::chat_relay`).
        return result(ChatDisposition::ChatUnavailable);
    };
    let Ok(text) = ChatText::parse(text) else {
        return result(ChatDisposition::Rejected);
    };
    let mode = speech_mode(*mode);
    let visible = runtime.visible_entities();
    let Some(speaker) = visible.players.iter().find(|entry| entry.actor == actor) else {
        return result(ChatDisposition::Rejected);
    };
    let (Some(generation), Ok(floor)) = (
        NonZeroU64::new(speaker.generation),
        u8::try_from(speaker.position.floor),
    ) else {
        return result(ChatDisposition::Rejected);
    };
    let at = ChatPosition {
        x: speaker.position.x,
        y: speaker.position.y,
        floor,
    };
    let speaker_id = ChatSpeaker {
        identity: actor.placement_identity(),
        generation,
    };
    // An actor without cast facts is level 1: it can talk but not yell.
    let gate = YellGate {
        level: states
            .get(runtime, actor, session)
            .map_or(1, |state| state.character_facts().level),
        premium_current: premium.premium_current(),
    };
    let Ok(mut sessions) = chat.sessions.lock() else {
        return result(ChatDisposition::ChatUnavailable);
    };
    let Some(speaking) = sessions.get_mut(&session) else {
        return result(ChatDisposition::ChatUnavailable);
    };
    if let Err(why) = speaking.limiter.admit_local(mode, gate, now) {
        return refusal(why);
    }
    let speaker_name = speaking.name.clone();
    let listeners: Vec<(GameSessionId, ChatPosition)> = sessions
        .keys()
        .filter_map(|listener| {
            let (_, snapshot) = runtime.positioned_player_for_session(*listener).ok()??;
            let position = snapshot.position();
            Some((
                *listener,
                ChatPosition {
                    x: position.x,
                    y: position.y,
                    floor: u8::try_from(position.floor).ok()?,
                },
            ))
        })
        .collect();
    for (listener, heard) in local_listeners(mode, at, listeners.iter().copied()) {
        if let Some(entry) = sessions.get_mut(&listener) {
            entry.egress.push(local_line(
                speaker_id,
                &speaker_name,
                mode,
                &text,
                at,
                heard,
            ));
        }
    }
    if mode == SpeechMode::Say
        && let Some(book) = chat.npc_talk.get()
    {
        npc_answer(
            book,
            &chat.joins,
            runtime,
            &mut sessions,
            session,
            at,
            &text,
            &listeners,
        );
    }
    result(ChatDisposition::Ok)
}

/// Whether `npc` already has `NPC0-RL-05` customers other than `session`.
fn crowded(
    sessions: &HashMap<GameSessionId, ChatSession>,
    session: GameSessionId,
    npc: ExactActorRef,
) -> bool {
    let others = sessions
        .iter()
        .filter(|(id, entry)| **id != session && entry.engaged == Some(npc))
        .count();
    others >= MAX_OPEN_CONVERSATIONS_PER_NPC as usize
}

/// The answer of a nearby NPC to a `say`, queued like any other local line (NPC-TALK-1).
#[allow(
    clippy::too_many_arguments,
    reason = "the answer's whole context under the chat lock"
)]
fn npc_answer(
    book: &NpcTalkBook,
    joins: &AtomicU64,
    runtime: &ChannelRuntimeV1,
    sessions: &mut HashMap<GameSessionId, ChatSession>,
    session: GameSessionId,
    at: ChatPosition,
    text: &ChatText,
    listeners: &[(GameSessionId, ChatPosition)],
) {
    let visible = runtime.visible_entities();
    let mut spots = Vec::new();
    for npc in &visible.npcs {
        let (Some(entry), Ok(floor), Some(generation)) = (
            runtime.npc_key(npc.actor).and_then(|key| book.get(key)),
            u8::try_from(npc.position.floor),
            NonZeroU64::new(npc.generation),
        ) else {
            continue;
        };
        let position = ChatPosition {
            x: npc.position.x,
            y: npc.position.y,
            floor,
        };
        spots.push((npc.actor, generation, position, entry));
    }
    let npcs: Vec<TalkNpc<'_, ExactActorRef>> = spots
        .iter()
        .map(|(actor, _, position, entry)| TalkNpc {
            id: *actor,
            position: *position,
            entry,
        })
        .collect();
    let Some(speaking) = sessions.get_mut(&session) else {
        return;
    };
    if !speaking.npc_service {
        return;
    }
    let prior = speaking.engaged;
    let Some(reply) = talk(at, text, &npcs, &mut speaking.engaged) else {
        return;
    };
    if speaking.engaged.is_some_and(|npc| prior != Some(npc))
        && crowded(sessions, session, reply.id)
    {
        // The NPC is serving as many customers as it can: the greeting is not answered.
        if let Some(speaking) = sessions.get_mut(&session) {
            speaking.engaged = prior;
        }
        return;
    }
    if prior != Some(reply.id)
        && let Some(speaking) = sessions.get_mut(&session)
        && speaking.engaged == Some(reply.id)
    {
        speaking.joined = joins.fetch_add(1, Ordering::Relaxed) + 1;
    }
    let Some((actor, generation, position, entry)) =
        spots.iter().find(|(actor, ..)| *actor == reply.id)
    else {
        return;
    };
    let Ok(line) = ChatText::parse(&reply.text) else {
        return;
    };
    let npc_speaker = ChatSpeaker {
        identity: actor.placement_identity(),
        generation: *generation,
    };
    for (listener, heard) in local_listeners(SpeechMode::Say, *position, listeners.iter().copied())
    {
        if let Some(entry_of) = sessions.get_mut(&listener) {
            entry_of.egress.push(local_line(
                npc_speaker,
                &entry.name,
                SpeechMode::Say,
                &line,
                *position,
                heard,
            ));
        }
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// The speaker name and CharacterId of `session`'s Character from the durable root, read
    /// once per session.
    async fn chat_speaker_name(&self, session: GameSessionId) -> Option<(String, [u8; 16])> {
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let (current, _) = store.current_session_at(session).await.ok()?;
        let character = *current.commit().character_id().as_bytes();
        let character_id = domain::CharacterId::from_bytes(character).ok()?;
        let name = self
            .root
            .read_character_chat_name(self.character, character_id)
            .await
            .ok()??;
        Some((name.as_str().to_owned(), character))
    }

    /// Opens (or keeps) the chat state of `session` and answers its open rooms (the snapshot).
    pub(super) async fn chat_open(&self, session: GameSessionId) -> ChatRoomSet {
        let name = if self.chat.knows(session) {
            None
        } else {
            self.chat_speaker_name(session).await
        };
        // Without a readable name the session is not admitted: it neither speaks nor hears.
        if let Some((name, character)) = name {
            self.chat
                .admit_as(session, &name, Some(character), self.owner_now());
        } else if self.chat.knows(session) {
            self.chat.refresh(session);
        }
        self.chat.show_rooms(session)
    }

    pub(super) async fn chat_speak(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        intent: &ChatIntent,
    ) -> ChatIntentResult {
        let now = self.owner_now();
        if let Some(refused) = self.chat.pre_gate(session, intent, now) {
            return refused;
        }
        if matches!(intent, ChatIntent::Private { .. } | ChatIntent::Room { .. }) {
            return self.chat_relay(actor, session, intent, now).await;
        }
        // Only a yell reads the Premium gate; say and whisper never open the latch transaction.
        let premium = match intent {
            ChatIntent::Say {
                mode: ChatSpeechMode::Yell,
                ..
            } => self.session_premium_gate(session).await,
            _ => PremiumGate::CLOSED,
        };
        let runtime = self.runtime.lock().await;
        let states = self.spell_states.lock().await;
        chat_in_premium_channel(
            &self.chat,
            &runtime,
            &states,
            (actor, session),
            premium,
            intent,
            now,
        )
    }

    /// A private or room line that passed the gate: the sender's level (and vocation, for
    /// Advertising) from its cast facts under a brief hold of the Channel locks, the recipient
    /// from the durable root, then the relay. No lock is held across the recipient read, the
    /// Premium read or the send.
    async fn chat_relay(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        intent: &ChatIntent,
        now: SemanticTimeMicros,
    ) -> ChatIntentResult {
        let (room, recipient_name, text) = match intent {
            ChatIntent::Private {
                recipient_name,
                text,
            } => (None, Some(recipient_name), text),
            ChatIntent::Room { room, text } => (Some(*room), None, text),
            _ => return result(ChatDisposition::ChatUnavailable),
        };
        let Ok(text) = ChatText::parse(text) else {
            return result(ChatDisposition::Rejected);
        };
        let Some(relay) = self.chat.relay() else {
            return result(ChatDisposition::ChatUnavailable);
        };
        // An actor without cast facts is level 1 and has no vocation.
        let facts = {
            let runtime = self.runtime.lock().await;
            let states = self.spell_states.lock().await;
            states
                .get(&runtime, actor, session)
                .map(|state| state.character_facts().level)
        };
        let level = facts.unwrap_or(1);
        let free = YellGate {
            level,
            premium_current: false,
        };
        let recipient = match (room, recipient_name) {
            (Some(room), _) => {
                // Only an under-level line to a room that needs it reads the Premium gate.
                let allowed = free.allows_room(room)
                    || YellGate {
                        level,
                        premium_current: self.session_premium_gate(session).await.premium_current(),
                    }
                    .allows_room(room);
                if !allowed {
                    return result(ChatDisposition::LevelTooLow);
                }
                if room == ChatRoom::Advertising && facts.is_none() {
                    return result(ChatDisposition::NoVocation);
                }
                None
            }
            (None, Some(name)) => {
                if !free.allows_private() {
                    return result(ChatDisposition::LevelTooLow);
                }
                // A name that cannot be a Character name is no online Character either.
                let Ok(name) = CharacterName::parse(name) else {
                    return result(ChatDisposition::NotOnline);
                };
                match self
                    .root
                    .read_chat_recipient(self.character, *self.world_id.as_bytes(), &name)
                    .await
                {
                    Ok(Some(recipient)) => Some(recipient),
                    Ok(None) => return result(ChatDisposition::NotOnline),
                    Err(_) => return result(ChatDisposition::ChatUnavailable),
                }
            }
            (None, None) => return result(ChatDisposition::ChatUnavailable),
        };
        let name = match self.chat.relay_sender(session, room, recipient, now) {
            Ok(sender) => sender,
            Err(refused) => return refused,
        };
        let destination = match (room, recipient) {
            (Some(room), _) => RelayDestination::Room { room },
            (None, Some(recipient)) => {
                let Ok(recipient) = oteryn_protocol_oteryn::CharacterId::decode(&recipient) else {
                    return result(ChatDisposition::ChatUnavailable);
                };
                RelayDestination::Private { recipient }
            }
            (None, None) => return result(ChatDisposition::ChatUnavailable),
        };
        match relay.send(destination, name, text).await {
            Ok(()) => result(ChatDisposition::Ok),
            Err(disposition) => result(disposition),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, reason = "test fixtures")]
mod tests {
    use super::*;
    use crate::gameplay_transport::actor_spell::tests::runtime_with_capacity;
    use crate::gameplay_transport::npc_talk::NpcTalkEntry;
    use oteryn_protocol_oteryn::chat::ChatRoom;

    const BASE: SemanticTimeMicros = SemanticTimeMicros::from_micros(1_000_000);

    fn session_of(tag: u8) -> GameSessionId {
        GameSessionId::decode(&[
            0x01, 0x90, 0x00, 0x00, 0x00, tag, 0x70, 0x00, 0x80, 0x00, 0, 0, 0, 0, 0, tag,
        ])
        .expect("session")
    }

    /// Two admitted, positioned players at the entry tile; `far` is then walked east.
    fn two_players(
        far: u32,
    ) -> (
        ChannelRuntimeV1,
        ChatRuntime,
        (ExactActorRef, GameSessionId),
        (ExactActorRef, GameSessionId),
    ) {
        let (mut runtime, first, first_session) = runtime_with_capacity(0x71, 3);
        let second_session = session_of(0x72);
        let reservation = runtime
            .reserve_fresh_session(second_session)
            .expect("reserve");
        let second = runtime.commit_fresh_session(reservation).expect("commit");
        for actor in [first, second] {
            runtime
                .initialize_first_entry_position(actor)
                .expect("position");
        }
        for _ in 0..far {
            let at = runtime.read_actor_position(second).expect("position");
            let mut next = at.position();
            next.x += 1;
            runtime
                .borrow_movement_position()
                .commit_cardinal(at, next)
                .expect("step");
        }
        let chat = ChatRuntime::default();
        chat.admit(first_session, "Aela", BASE);
        chat.admit(second_session, "Bryn", BASE);
        (
            runtime,
            chat,
            (first, first_session),
            (second, second_session),
        )
    }

    fn say(text: &str) -> ChatIntent {
        ChatIntent::Say {
            mode: ChatSpeechMode::Say,
            text: text.to_owned(),
        }
    }

    fn speak(
        chat: &ChatRuntime,
        runtime: &ChannelRuntimeV1,
        who: (ExactActorRef, GameSessionId),
        intent: &ChatIntent,
        now: SemanticTimeMicros,
    ) -> ChatIntentResult {
        chat_in_channel(
            chat,
            runtime,
            &ChannelSpellStates::default(),
            who.0,
            who.1,
            intent,
            now,
        )
    }

    fn texts(lines: Vec<ChatLine>) -> Vec<(String, String)> {
        lines
            .into_iter()
            .map(|line| match line {
                ChatLine::Local {
                    speaker_name, text, ..
                } => (speaker_name, text),
                other => panic!("unexpected line {other:?}"),
            })
            .collect()
    }

    #[test]
    fn a_say_is_heard_by_the_nearby_session_and_by_the_speaker() {
        let (runtime, chat, first, second) = two_players(0);
        let done = speak(&chat, &runtime, first, &say("hello"), BASE);
        assert_eq!(done, result(ChatDisposition::Ok));
        let expected = vec![("Aela".to_owned(), "hello".to_owned())];
        assert_eq!(texts(chat.drain(second.1)), expected);
        assert_eq!(texts(chat.drain(first.1)), expected);
        assert!(chat.drain(second.1).is_empty());
    }

    #[test]
    fn a_say_out_of_range_is_not_heard() {
        let (runtime, chat, first, second) = two_players(40);
        let done = speak(&chat, &runtime, first, &say("hello"), BASE);
        assert_eq!(done, result(ChatDisposition::Ok));
        assert!(chat.drain(second.1).is_empty());
        assert_eq!(chat.drain(first.1).len(), 1);
    }

    #[test]
    fn the_channel_path_never_sends_a_relay_line() {
        let (runtime, chat, first, second) = two_players(0);
        let private = ChatIntent::Private {
            recipient_name: "Bryn".to_owned(),
            text: "psst".to_owned(),
        };
        assert_eq!(
            speak(&chat, &runtime, first, &private, BASE),
            result(ChatDisposition::ChatUnavailable)
        );
        assert!(chat.drain(second.1).is_empty());
    }

    #[test]
    fn unusable_text_is_rejected_and_not_delivered() {
        let (runtime, chat, first, second) = two_players(0);
        for text in ["", "   ", "bell\u{7}"] {
            assert_eq!(
                speak(&chat, &runtime, first, &say(text), BASE),
                result(ChatDisposition::Rejected)
            );
        }
        assert!(chat.drain(second.1).is_empty());
    }

    #[test]
    fn a_yell_needs_level_twenty() {
        let (runtime, chat, first, second) = two_players(0);
        let yell = ChatIntent::Say {
            mode: ChatSpeechMode::Yell,
            text: "hey".to_owned(),
        };
        assert_eq!(
            speak(&chat, &runtime, first, &yell, BASE),
            result(ChatDisposition::LevelTooLow)
        );
        assert!(chat.drain(second.1).is_empty());
    }

    #[test]
    fn spam_exhausts_and_then_mutes() {
        let (runtime, chat, first, _) = two_players(0);
        let mut dispositions = Vec::new();
        for _ in 0..8 {
            dispositions.push(speak(&chat, &runtime, first, &say("x"), BASE).disposition);
        }
        assert_eq!(dispositions[0], ChatDisposition::Ok);
        assert!(
            dispositions
                .iter()
                .any(|d| matches!(d, ChatDisposition::Exhausted | ChatDisposition::Muted)),
            "{dispositions:?}"
        );
    }

    #[test]
    fn the_pre_gate_refuses_a_flood_without_the_channel_and_takes_no_line() {
        let (runtime, chat, first, _) = two_players(0);
        assert!(chat.pre_gate(first.1, &say("x"), BASE).is_none());
        // The gate takes no line: four lines still pass through the full path.
        for _ in 0..4 {
            assert_eq!(
                speak(&chat, &runtime, first, &say("x"), BASE).disposition,
                ChatDisposition::Ok
            );
        }
        let refused = chat
            .pre_gate(first.1, &say("x"), BASE)
            .expect("empty bucket");
        assert_eq!(refused.disposition, ChatDisposition::Muted);
        assert!(chat.muted_cast(first.1, BASE).is_some());
        assert!(chat.pre_gate(first.1, &say("x"), BASE).is_some());
        // Unusable text is refused before the Channel and takes no line.
        assert_eq!(
            chat.pre_gate(first.1, &say("   "), BASE)
                .expect("blank")
                .disposition,
            ChatDisposition::Rejected
        );
        // An unknown session (its chat open failed) is refused before the Channel.
        assert_eq!(
            chat.pre_gate(session_of(0x7f), &say("x"), BASE)
                .expect("unknown session")
                .disposition,
            ChatDisposition::ChatUnavailable
        );
    }

    #[test]
    fn without_a_relay_room_and_private_lines_are_unavailable() {
        let (_runtime, chat, first, _) = two_players(0);
        let unserved = [
            ChatIntent::Private {
                recipient_name: "Other".to_owned(),
                text: "x".to_owned(),
            },
            ChatIntent::Room {
                room: ChatRoom::World,
                text: "x".to_owned(),
            },
            ChatIntent::OpenRoom(ChatRoom::Help),
        ];
        for intent in &unserved {
            for session in [first.1, session_of(0x7f)] {
                assert_eq!(
                    chat.pre_gate(session, intent, BASE)
                        .expect("refused before the Channel")
                        .disposition,
                    ChatDisposition::ChatUnavailable,
                    "{intent:?}"
                );
            }
        }
        // Closing a room is answered outright; an unknown session has no rooms.
        let close = ChatIntent::CloseRoom(ChatRoom::Help);
        assert_eq!(
            chat.pre_gate(first.1, &close, BASE).map(|r| r.disposition),
            Some(ChatDisposition::Ok)
        );
        assert_eq!(
            chat.pre_gate(session_of(0x7f), &close, BASE)
                .map(|r| r.disposition),
            Some(ChatDisposition::ChatUnavailable)
        );
        assert_eq!(chat.take_rooms_change(first.1), None);
        // No line was taken: a say still passes the gate.
        assert!(chat.pre_gate(first.1, &say("x"), BASE).is_none());
    }

    /// Runs `test` with a relay installed in `chat` (ready when `ready`), inside a Tokio runtime
    /// for the relay's lazy pool. The pool never connects.
    fn with_relay(chat: &ChatRuntime, ready: bool, test: impl FnOnce()) {
        use std::str::FromStr as _;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let _context = runtime.enter();
        let ring = crate::chat::relay::RelayKeyRing::new(vec![crate::chat::relay::RelayKey::new(
            1, 0, [1; 32],
        )])
        .expect("ring");
        let options =
            sqlx::postgres::PgConnectOptions::from_str("postgresql://nobody@127.0.0.1:1/none")
                .expect("options");
        let relay = ChatRelay::new(ring, character(0x01), options).expect("relay");
        if ready {
            relay.force_ready(1_000_000);
        }
        chat.install_relay(Arc::new(relay));
        test();
    }

    fn room_intent(room: ChatRoom, text: &str) -> ChatIntent {
        ChatIntent::Room {
            room,
            text: text.to_owned(),
        }
    }

    fn character(marker: u8) -> [u8; 16] {
        let mut bytes = [marker; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes
    }

    fn relayed(destination: RelayDestination, sender: &str, text: &str) -> RelayLine {
        RelayLine {
            message_id: [9; 16],
            sent_at_micros: 1,
            destination,
            sender_name: CharacterName::parse(sender).expect("name"),
            text: ChatText::parse(text).expect("text"),
        }
    }

    #[test]
    fn rooms_open_and_close_and_their_client_view_follows() {
        let (_runtime, chat, first, _) = two_players(0);
        with_relay(&chat, false, || {
            assert_eq!(chat.show_rooms(first.1), ChatRoomSet::default());
            let open = ChatIntent::OpenRoom(ChatRoom::Help);
            assert_eq!(
                chat.pre_gate(first.1, &open, BASE).map(|r| r.disposition),
                Some(ChatDisposition::Ok)
            );
            let changed = chat.take_rooms_change(first.1).expect("changed");
            assert!(changed.contains(ChatRoom::Help));
            assert_eq!(chat.take_rooms_change(first.1), None);
            // Opening again changes nothing.
            chat.pre_gate(first.1, &open, BASE);
            assert_eq!(chat.take_rooms_change(first.1), None);
            chat.pre_gate(first.1, &ChatIntent::CloseRoom(ChatRoom::Help), BASE);
            assert_eq!(
                chat.take_rooms_change(first.1),
                Some(ChatRoomSet::default())
            );
            // A reconnect keeps the open rooms.
            chat.pre_gate(first.1, &open, BASE);
            chat.admit(first.1, "Aela", BASE);
            assert!(chat.show_rooms(first.1).contains(ChatRoom::Help));
        });
    }

    #[test]
    fn the_relay_pre_gate_refuses_before_the_channel() {
        let (_runtime, chat, first, _) = two_players(0);
        // Not ready: unavailable, even for an open room.
        with_relay(&chat, false, || {
            chat.pre_gate(first.1, &ChatIntent::OpenRoom(ChatRoom::Help), BASE);
            assert_eq!(
                chat.pre_gate(first.1, &room_intent(ChatRoom::Help, "x"), BASE)
                    .map(|r| r.disposition),
                Some(ChatDisposition::ChatUnavailable)
            );
        });
        let (_runtime, chat, first, _) = two_players(0);
        with_relay(&chat, true, || {
            assert_eq!(
                chat.pre_gate(first.1, &room_intent(ChatRoom::Help, "   "), BASE)
                    .map(|r| r.disposition),
                Some(ChatDisposition::Rejected)
            );
            assert_eq!(
                chat.pre_gate(first.1, &room_intent(ChatRoom::Help, "x"), BASE)
                    .map(|r| r.disposition),
                Some(ChatDisposition::RoomNotOpen)
            );
            chat.pre_gate(first.1, &ChatIntent::OpenRoom(ChatRoom::Help), BASE);
            assert!(
                chat.pre_gate(first.1, &room_intent(ChatRoom::Help, "x"), BASE)
                    .is_none()
            );
            let private = ChatIntent::Private {
                recipient_name: "Bryn".to_owned(),
                text: "x".to_owned(),
            };
            // The relay lines share the bucket: a flood ends in a mute before the Channel.
            let mut refused = None;
            for _ in 0..8 {
                refused = chat.pre_gate(first.1, &private, BASE);
                if refused.is_some() {
                    break;
                }
            }
            assert_eq!(refused.map(|r| r.disposition), Some(ChatDisposition::Muted));
            assert!(chat.muted_cast(first.1, BASE).is_some());
        });
    }

    #[test]
    fn a_private_line_to_oneself_is_rejected_and_recipients_are_limited() {
        let (_runtime, chat, first, _) = two_players(0);
        chat.admit_as(first.1, "Aela", Some(character(0x31)), BASE);
        assert_eq!(
            chat.relay_sender(first.1, None, Some(character(0x31)), BASE)
                .err()
                .map(|r| r.disposition),
            Some(ChatDisposition::Rejected)
        );
        for marker in 0..20 {
            assert!(
                chat.relay_sender(first.1, None, Some(character(0x40 + marker)), BASE)
                    .is_ok()
            );
        }
        // An earlier recipient is no new one.
        assert!(
            chat.relay_sender(first.1, None, Some(character(0x40)), BASE)
                .is_ok()
        );
        assert_eq!(
            chat.relay_sender(first.1, None, Some(character(0x60)), BASE)
                .err()
                .map(|r| r.disposition),
            Some(ChatDisposition::Muted)
        );
        assert_eq!(
            chat.relay_sender(session_of(0x7f), None, None, BASE)
                .err()
                .map(|r| r.disposition),
            Some(ChatDisposition::ChatUnavailable)
        );
    }

    #[test]
    fn one_advertising_line_per_interval() {
        let (_runtime, chat, first, _) = two_players(0);
        let advertising = Some(ChatRoom::Advertising);
        let name = chat
            .relay_sender(first.1, advertising, None, BASE)
            .expect("first line");
        assert_eq!(name.as_str(), "Aela");
        assert_eq!(
            chat.relay_sender(first.1, advertising, None, BASE)
                .err()
                .map(|r| r.disposition),
            Some(ChatDisposition::Exhausted)
        );
        let later = SemanticTimeMicros::from_micros(BASE.get() + 120_000_000);
        assert!(chat.relay_sender(first.1, advertising, None, later).is_ok());
        // Other rooms have no interval.
        assert!(
            chat.relay_sender(first.1, Some(ChatRoom::Help), None, BASE)
                .is_ok()
        );
    }

    #[test]
    fn relayed_lines_reach_the_recipient_and_the_open_rooms_only() {
        let (_runtime, chat, first, second) = two_players(0);
        chat.admit_as(first.1, "Aela", Some(character(0x31)), BASE);
        chat.admit_as(second.1, "Bryn", Some(character(0x32)), BASE);
        let recipient =
            oteryn_protocol_oteryn::CharacterId::decode(&character(0x32)).expect("character");
        chat.deliver(&relayed(
            RelayDestination::Private { recipient },
            "Cora",
            "psst",
        ));
        assert_eq!(
            chat.drain(second.1),
            vec![ChatLine::Private {
                speaker_name: "Cora".to_owned(),
                text: "psst".to_owned(),
            }]
        );
        assert!(chat.drain(first.1).is_empty());
        with_relay(&chat, false, || {
            chat.pre_gate(first.1, &ChatIntent::OpenRoom(ChatRoom::English), BASE);
        });
        chat.deliver(&relayed(
            RelayDestination::Room {
                room: ChatRoom::English,
            },
            "Cora",
            "sell",
        ));
        assert_eq!(
            chat.drain(first.1),
            vec![ChatLine::Room {
                room: ChatRoom::English,
                speaker_name: "Cora".to_owned(),
                text: "sell".to_owned(),
            }]
        );
        assert!(chat.drain(second.1).is_empty());
    }

    /// `chat_speak` without the async locks: the pre-gate, then the full Channel path, counting
    /// how often the full path (runtime and spell-state locks, entity clone) is entered.
    fn attempt(
        chat: &ChatRuntime,
        runtime: &ChannelRuntimeV1,
        who: (ExactActorRef, GameSessionId),
        intent: &ChatIntent,
        now: SemanticTimeMicros,
        channel_work: &mut u32,
    ) -> ChatIntentResult {
        if let Some(refused) = chat.pre_gate(who.1, intent, now) {
            return refused;
        }
        *channel_work += 1;
        speak(chat, runtime, who, intent, now)
    }

    #[test]
    fn repeated_under_level_yells_hit_the_limiter_before_the_channel() {
        let (runtime, chat, first, second) = two_players(0);
        let yell = ChatIntent::Say {
            mode: ChatSpeechMode::Yell,
            text: "hey".to_owned(),
        };
        let mut channel_work = 0;
        let mut dispositions = Vec::new();
        for _ in 0..16 {
            dispositions
                .push(attempt(&chat, &runtime, first, &yell, BASE, &mut channel_work).disposition);
        }
        // One full-path refusal, three refusals at the gate (each takes a line), then the empty
        // bucket starts the offence at the gate and the mute holds the rest there.
        assert_eq!(channel_work, 1);
        assert_eq!(dispositions[..4], [ChatDisposition::LevelTooLow; 4]);
        assert!(
            dispositions[4..]
                .iter()
                .all(|d| *d == ChatDisposition::Muted),
            "{dispositions:?}"
        );
        assert!(chat.muted_cast(first.1, BASE).is_some());
        assert!(chat.drain(second.1).is_empty());
        // The yells took the lines: a say is refused at the gate too.
        assert_eq!(
            attempt(&chat, &runtime, first, &say("x"), BASE, &mut channel_work).disposition,
            ChatDisposition::Muted
        );
        assert_eq!(channel_work, 1);
    }

    #[test]
    fn a_sustained_under_level_yell_flood_rarely_reaches_the_channel() {
        let (runtime, chat, first, _) = two_players(0);
        let yell = ChatIntent::Say {
            mode: ChatSpeechMode::Yell,
            text: "hey".to_owned(),
        };
        let mut channel_work = 0;
        let mut muted = 0;
        // One yell every 50 ms for ten minutes: 12,000 attempts.
        for step in 0..12_000_u64 {
            let now = SemanticTimeMicros::from_micros(BASE.get() + step * 50_000);
            let done = attempt(&chat, &runtime, first, &yell, now, &mut channel_work);
            assert_ne!(done.disposition, ChatDisposition::Ok);
            if done.disposition == ChatDisposition::Muted {
                muted += 1;
            }
        }
        // Each full-path refusal is followed by an offence whose mute grows as 5 s x n^2, so the
        // Channel sees a handful of attempts, not one per refill interval (240).
        assert!(channel_work <= 8, "{channel_work}");
        assert!(muted > 11_000, "{muted}");
    }

    #[test]
    fn a_muted_session_cannot_cast_until_the_mute_ends() {
        let (runtime, chat, first, second) = two_players(0);
        assert!(chat.muted_cast(first.1, BASE).is_none());
        for _ in 0..8 {
            speak(&chat, &runtime, first, &say("x"), BASE);
        }
        let outcome = chat.muted_cast(first.1, BASE).expect("muted cast");
        assert_eq!(
            outcome.disposition,
            oteryn_protocol_oteryn::actor_spell::SpellCastDisposition::Rejected
        );
        // Only the spammer is muted, and an unknown session is never held back.
        assert!(chat.muted_cast(second.1, BASE).is_none());
        assert!(chat.muted_cast(session_of(0x7f), BASE).is_none());
        let later = SemanticTimeMicros::from_micros(BASE.get() + 600_000_000);
        assert!(chat.muted_cast(first.1, later).is_none());
    }

    #[test]
    fn an_unadmitted_session_cannot_speak() {
        let (runtime, _, first, _) = two_players(0);
        let stranger = ChatRuntime::default();
        assert_eq!(
            speak(&stranger, &runtime, first, &say("hello"), BASE),
            result(ChatDisposition::ChatUnavailable)
        );
    }

    #[test]
    fn readmission_drops_queued_lines_and_keeps_the_limiter() {
        let (runtime, chat, first, second) = two_players(0);
        speak(&chat, &runtime, first, &say("hello"), BASE);
        chat.admit(second.1, "Bryn", BASE);
        assert!(chat.drain(second.1).is_empty());
        chat.forget(second.1);
        assert!(!chat.knows(second.1));
    }

    #[test]
    fn a_greeting_near_an_npc_is_answered_by_the_npc_to_everyone_in_range() {
        use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
        let (mut runtime, chat, first, second) = two_players(0);
        let spot = runtime
            .read_actor_position(first.0)
            .expect("position")
            .position();
        let table = NpcPlacementTable {
            npcs: vec![NpcPlacements {
                key: "oteryn:npc.alice".to_owned(),
                placements: vec![NpcPlacement {
                    floor: -i8::try_from(spot.floor).expect("floor"),
                    y: u16::try_from(spot.y).expect("y"),
                    x: u16::try_from(spot.x + 2).expect("x"),
                    direction: MovementFacing::South,
                }],
            }],
        };
        runtime.place_npcs(&table, |_| true, true).expect("placed");
        let book = NpcTalkBook::default().with(
            "oteryn:npc.alice",
            NpcTalkEntry::new("Alice", Some("Alice buys and sells goods.")),
        );
        chat.install_npc_talk(Arc::new(book));
        chat.set_npc_service(first.1);
        let said = |who, text: &str| speak(&chat, &runtime, who, &say(text), BASE);
        // A topic before a greeting is not answered.
        said(first, "trade");
        assert_eq!(texts(chat.drain(second.1)).len(), 1);
        chat.drain(first.1);
        said(first, "hello");
        let expected = vec![
            ("Aela".to_owned(), "hello".to_owned()),
            ("Alice".to_owned(), "Greetings, I am Alice.".to_owned()),
        ];
        assert_eq!(texts(chat.drain(first.1)), expected);
        assert_eq!(texts(chat.drain(second.1)), expected);
        said(first, "job");
        assert_eq!(
            texts(chat.drain(first.1)).last().map(|l| l.1.clone()),
            Some("Alice buys and sells goods.".to_owned())
        );
        said(first, "bye");
        assert_eq!(
            texts(chat.drain(second.1)).last().map(|l| l.1.clone()),
            Some("Farewell.".to_owned())
        );
    }
}
