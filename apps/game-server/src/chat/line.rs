//! Listener choice and line values for local, private and room chat (CHAT-0 §3 and §7).
//!
//! Pure: the runtime passes the positions of the speaker's Channel and sends what comes back.

use oteryn_protocol_oteryn::chat::{ChatLine, ChatRoom, ChatSpeaker, ChatSpeechMode};
use oteryn_protocol_oteryn::world_spatial::ActorPosition;

use super::{ChatPosition, ChatText, Heard, SpeechMode, WHISPER_OBSCURED, hears};

/// The listeners of a local line and what each hears, in the order given. The speaker is chosen
/// like any listener, so it hears its own line when its position is among `listeners`.
pub(crate) fn local_listeners<K>(
    mode: SpeechMode,
    speaker: ChatPosition,
    listeners: impl IntoIterator<Item = (K, ChatPosition)>,
) -> Vec<(K, Heard)> {
    listeners
        .into_iter()
        .filter_map(|(key, at)| hears(mode, speaker, at).map(|heard| (key, heard)))
        .collect()
}

/// The `LOCAL` line one listener receives. A yell is upper-cased; a whisper heard from beyond an
/// adjacent tile carries [`WHISPER_OBSCURED`] as its text.
pub(crate) fn local_line(
    speaker: ChatSpeaker,
    speaker_name: &str,
    mode: SpeechMode,
    text: &ChatText,
    at: ChatPosition,
    heard: Heard,
) -> ChatLine {
    let shown = match (heard, mode) {
        (Heard::Obscured, _) => WHISPER_OBSCURED.to_owned(),
        (Heard::Text, SpeechMode::Yell) => text.yelled().as_str().to_owned(),
        (Heard::Text, _) => text.as_str().to_owned(),
    };
    ChatLine::Local {
        speaker,
        speaker_name: speaker_name.to_owned(),
        mode: wire_mode(mode),
        text: shown,
        position: ActorPosition {
            x: at.x,
            y: at.y,
            floor: i16::from(at.floor),
        },
    }
}

pub(crate) fn private_line(speaker_name: &str, text: &ChatText) -> ChatLine {
    ChatLine::Private {
        speaker_name: speaker_name.to_owned(),
        text: text.as_str().to_owned(),
    }
}

pub(crate) fn room_line(room: ChatRoom, speaker_name: &str, text: &ChatText) -> ChatLine {
    ChatLine::Room {
        room,
        speaker_name: speaker_name.to_owned(),
        text: text.as_str().to_owned(),
    }
}

fn wire_mode(mode: SpeechMode) -> ChatSpeechMode {
    match mode {
        SpeechMode::Say => ChatSpeechMode::Say,
        SpeechMode::Whisper => ChatSpeechMode::Whisper,
        SpeechMode::Yell => ChatSpeechMode::Yell,
    }
}
