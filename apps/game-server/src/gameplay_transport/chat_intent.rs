//! `CHAT_INTENT` for local speech (CHAT-WIRE-1, CHAT-0 §3 and §7).
//!
//! A thin runtime over the pure rules of `crate::chat`: one entry per admitted GameSession holds
//! the speaker name, the spam limiter and the undelivered-line queue. A `say`, `whisper` or `yell`
//! is admitted by the speaker's limiter and queued for every session of the Channel that hears
//! it; each connection drains its own queue into domain 12 deltas. Private messages, rooms and
//! the World relay are CHAT-2 and answer `CHAT_UNAVAILABLE`. Nothing here is durable.

use std::collections::HashMap;
use std::num::NonZeroU64;
use std::sync::{Arc, Mutex, OnceLock};

use oteryn_protocol_oteryn::chat::{
    ChatDisposition, ChatIntent, ChatIntentResult, ChatLine, ChatSpeaker, ChatSpeechMode,
};
use oteryn_simulation_determinism::SemanticTimeMicros;

use super::actor_spell::ChannelSpellStates;
use super::npc_talk::{NpcTalkBook, TalkNpc, talk};
use super::{ComposedFreshAdmission, FreshAdmissionStore};
use crate::chat::{
    ChatEgress, ChatLimiter, ChatPosition, ChatRefusal, ChatText, SpeechMode, YellGate, local_line,
    local_listeners,
};
use crate::domain;
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
}

/// The chat state of the admitted sessions of one Channel. The lock is never held across an
/// await and is always taken after the runtime and spell-state locks.
#[derive(Default)]
pub(crate) struct ChatRuntime {
    sessions: Mutex<HashMap<GameSessionId, ChatSession>>,
    /// What the NPCs of the Channel say; unset until the boot wiring installs it.
    npc_talk: OnceLock<Arc<NpcTalkBook>>,
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
            });
            entry.name = name.to_owned();
            entry.egress.clear();
            entry.engaged = None;
        }
    }

    /// Installs the NPC talk of the Channel once; a second install is ignored.
    #[allow(
        dead_code,
        reason = "boot wiring follows NPC-PLACE-1b (no runtime NPC before it)"
    )]
    pub(crate) fn install_npc_talk(&self, book: Arc<NpcTalkBook>) {
        let _ = self.npc_talk.set(book);
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
        }
    }

    pub(crate) fn forget(&self, session: GameSessionId) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.remove(&session);
        }
    }

    /// Every undelivered line of `session`, oldest first.
    pub(crate) fn drain(&self, session: GameSessionId) -> Vec<ChatLine> {
        let Ok(mut sessions) = self.sessions.lock() else {
            return Vec::new();
        };
        let Some(entry) = sessions.get_mut(&session) else {
            return Vec::new();
        };
        std::iter::from_fn(|| entry.egress.pop()).collect()
    }

    /// Whether `session` is muted at `now` (read by the cast path).
    #[allow(dead_code, reason = "read by the cast path when it takes the mute")]
    pub(crate) fn is_muted(&self, session: GameSessionId, now: SemanticTimeMicros) -> bool {
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

/// One intent of the admitted `actor`, under the runtime and spell-state locks the caller holds.
pub(crate) fn chat_in_channel(
    chat: &ChatRuntime,
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    actor: ExactActorRef,
    session: GameSessionId,
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
        premium_current: false,
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
        npc_answer(book, runtime, &mut sessions, session, at, &text, &listeners);
    }
    result(ChatDisposition::Ok)
}

/// The answer of a nearby NPC to a `say`, queued like any other local line (NPC-TALK-1).
fn npc_answer(
    book: &NpcTalkBook,
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
    let Some(reply) = talk(at, text, &npcs, &mut speaking.engaged) else {
        return;
    };
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
        let runtime = self.runtime.lock().await;
        let states = self.spell_states.lock().await;
        chat_in_channel(&self.chat, &runtime, &states, actor, session, intent, now)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, reason = "test fixtures")]
mod tests {
    use super::*;
    use crate::gameplay_transport::actor_spell::tests::runtime_with_capacity;
    use crate::gameplay_transport::npc_talk::NpcTalkEntry;

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
