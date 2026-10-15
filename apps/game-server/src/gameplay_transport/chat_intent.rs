//! `CHAT_INTENT` for local speech (CHAT-WIRE-1, CHAT-0 §3 and §7).
//!
//! A thin runtime over the pure rules of `crate::chat`: one entry per admitted GameSession holds
//! the speaker name, the spam limiter and the undelivered-line queue. A `say`, `whisper` or `yell`
//! is admitted by the speaker's limiter and queued for every session of the Channel that hears
//! it; each connection drains its own queue into domain 12 deltas. Private messages, rooms and
//! the World relay are CHAT-2 and answer `CHAT_UNAVAILABLE`. Nothing here is durable.

use std::collections::HashMap;
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use oteryn_protocol_oteryn::chat::{
    ChatDisposition, ChatIntent, ChatIntentResult, ChatLine, ChatSpeaker, ChatSpeechMode,
};
use oteryn_protocol_oteryn::npc_service::{MAX_OPEN_CONVERSATIONS_PER_NPC, TALK_RANGE_TILES};
use oteryn_simulation_determinism::SemanticTimeMicros;

use super::actor_spell::ChannelSpellStates;
use super::npc_talk::{NpcTalkBook, TalkNpc, talk};
use super::{ComposedFreshAdmission, FreshAdmissionStore};
use crate::chat::{
    ChatEgress, ChatLimiter, ChatPosition, ChatRefusal, ChatText, SpeechMode, YellGate, local_line,
    local_listeners,
};
use crate::domain;
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
    limiter: ChatLimiter,
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
    /// The last customer queue position handed out.
    joins: AtomicU64,
}

impl ChatRuntime {
    /// Admission, reconnect or transfer of `session`: keeps its limiter (a mute survives a
    /// reconnect), drops undelivered lines (a replacement snapshot drops lines, never replays).
    pub(crate) fn admit(&self, session: GameSessionId, name: &str, now: SemanticTimeMicros) {
        if let Ok(mut sessions) = self.sessions.lock() {
            let entry = sessions.entry(session).or_insert_with(|| ChatSession {
                name: name.to_owned(),
                limiter: ChatLimiter::new(now),
                egress: ChatEgress::new(),
                engaged: None,
                joined: 0,
                walked_away: Vec::new(),
                npc_service: false,
            });
            entry.name = name.to_owned();
            entry.egress.clear();
            entry.engaged = None;
            entry.walked_away.clear();
            entry.npc_service = false;
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
    /// the entity clone, so a flood costs the Channel only this map lookup. An intent the runtime
    /// does not serve yet (private messages and rooms, CHAT-2) is refused here too: it takes no
    /// line, so letting it reach the locks would make it a free flood. `None` lets the intent go on.
    pub(crate) fn pre_gate(
        &self,
        session: GameSessionId,
        intent: &ChatIntent,
        now: SemanticTimeMicros,
    ) -> Option<ChatIntentResult> {
        let ChatIntent::Say { mode, text } = intent else {
            // CHAT-2 gates these here when it implements them.
            return Some(result(ChatDisposition::ChatUnavailable));
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
        // CHAT-2: private messages, rooms and the World relay.
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
    /// The speaker name of `session`'s Character from the durable root, read once per session.
    async fn chat_speaker_name(&self, session: GameSessionId) -> Option<String> {
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let (current, _) = store.current_session_at(session).await.ok()?;
        let character_id =
            domain::CharacterId::from_bytes(*current.commit().character_id().as_bytes()).ok()?;
        let name = self
            .root
            .read_character_chat_name(self.character, character_id)
            .await
            .ok()??;
        Some(name.as_str().to_owned())
    }

    /// Opens (or keeps) the chat state of `session`; no room is open before CHAT-2.
    pub(super) async fn chat_open(&self, session: GameSessionId) -> ChatRoomSet {
        let name = if self.chat.knows(session) {
            None
        } else {
            self.chat_speaker_name(session).await
        };
        // Without a readable name the session is not admitted: it neither speaks nor hears.
        if let Some(name) = name {
            self.chat.admit(session, &name, self.owner_now());
        } else if self.chat.knows(session) {
            self.chat.refresh(session);
        }
        ChatRoomSet::default()
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
    fn rooms_and_private_messages_wait_for_chat_two() {
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
    fn the_pre_gate_refuses_unserved_intents_without_the_channel() {
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
            ChatIntent::CloseRoom(ChatRoom::Help),
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
        // No line was taken: a say still passes the gate.
        assert!(chat.pre_gate(first.1, &say("x"), BASE).is_none());
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
