//! Shared local speech hearing rules.

/// Say and whisper reach ±8 × ±6 tiles on the speaker's floor (Canary `map_const.hpp:12-13`).
const SAY_RANGE_X: u32 = 8;
const SAY_RANGE_Y: u32 = 6;
/// Yell reaches ±18 × ±14 tiles over the floors of [`yell_floors`] (Canary `game.cpp:7632-7636`).
const YELL_RANGE_X: u32 = 18;
const YELL_RANGE_Y: u32 = 14;
/// Whisper text reaches adjacent tiles only; farther listeners see [`WHISPER_OBSCURED`].
const WHISPER_CLEAR_RANGE: u32 = 1;
/// What a listener beyond [`WHISPER_CLEAR_RANGE`] sees of a whisper (Canary `game.cpp:7497-7506`).
pub(crate) const WHISPER_OBSCURED: &str = "pspsps";

/// Local speech modes of `CHAT_INTENT.say` (CHAT-0 §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpeechMode {
    Say,
    Whisper,
    Yell,
}

/// A tile on the speaker's Channel. Positions of different Channels are never compared: the
/// runtime passes only listeners of the speaker's Channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ChatPosition {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) floor: u8,
}

impl ChatPosition {
    fn within(self, other: Self, range_x: u32, range_y: u32) -> bool {
        self.x.abs_diff(other.x) <= range_x && self.y.abs_diff(other.y) <= range_y
    }

    /// Chebyshev distance on one floor; `None` across floors.
    pub(crate) fn distance(self, other: Self) -> Option<u32> {
        (self.floor == other.floor).then(|| self.x.abs_diff(other.x).max(self.y.abs_diff(other.y)))
    }
}

/// What one listener receives of a local line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Heard {
    /// The line's text.
    Text,
    /// A whisper heard from beyond an adjacent tile: [`WHISPER_OBSCURED`].
    Obscured,
}

/// Whether and how a listener at `listener` hears a line spoken at `speaker` (CHAT-0 §3 with the
/// Canary ranges of §2). The speaker hears its own line.
pub(crate) fn hears(
    mode: SpeechMode,
    speaker: ChatPosition,
    listener: ChatPosition,
) -> Option<Heard> {
    match mode {
        SpeechMode::Say => (speaker.floor == listener.floor
            && speaker.within(listener, SAY_RANGE_X, SAY_RANGE_Y))
        .then_some(Heard::Text),
        SpeechMode::Whisper => {
            if speaker.floor != listener.floor
                || !speaker.within(listener, SAY_RANGE_X, SAY_RANGE_Y)
            {
                None
            } else if speaker.within(listener, WHISPER_CLEAR_RANGE, WHISPER_CLEAR_RANGE) {
                Some(Heard::Text)
            } else {
                Some(Heard::Obscured)
            }
        }
        SpeechMode::Yell => (yell_floors(speaker.floor).contains(&listener.floor)
            && speaker.within(listener, YELL_RANGE_X, YELL_RANGE_Y))
        .then_some(Heard::Text),
    }
}

/// Floors a yell reaches (Canary `spectators.cpp:125-137`): above ground floors 0-7, to 8 from
/// floor 6 and to 9 from floor 7; underground two floors up and down.
fn yell_floors(floor: u8) -> std::ops::RangeInclusive<u8> {
    match floor {
        0..=5 => 0..=7,
        6 => 0..=8,
        7 => 0..=9,
        _ => floor.saturating_sub(2)..=floor.saturating_add(2).min(15),
    }
}
