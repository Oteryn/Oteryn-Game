use super::*;
use oteryn_protocol_oteryn::chat::private_channel::PrivateChannelIntent;

impl ChatRuntime {
    pub(super) fn sender_gate(
        &self,
        session: GameSessionId,
        intent: &ChatIntent,
        now: SemanticTimeMicros,
    ) -> Option<ChatIntentResult> {
        let (mode, text) = match intent {
            ChatIntent::Say { mode, text } => (mode, text),
            ChatIntent::OpenRoom(room) => return Some(self.set_room(session, *room, true)),
            ChatIntent::CloseRoom(room) => return Some(self.set_room(session, *room, false)),
            ChatIntent::Guild { text } => return self.pre_gate_relay(session, None, text, now),
            // A create reads the Premium gate after this gate (`chat_speak`).
            ChatIntent::PrivateChannel(PrivateChannelIntent::Create) => return None,
            ChatIntent::PrivateChannel(intent) => {
                return Some(self.private_channel(session, intent, now));
            }
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
    pub(in crate::gameplay_transport) fn pre_gate_relay(
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
}
