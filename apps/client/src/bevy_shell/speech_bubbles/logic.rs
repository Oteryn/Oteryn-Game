//! Local speech as map text: the merge, expiry, wording, colour and placement rules of OTClient's
//! StaticText, kept free of Bevy so the rules are unit-tested.
use oteryn_client_runtime::chat::{ChatLine, ChatSpeechMode};

/// Text wraps at this width, in logical pixels.
pub(super) const WRAP_WIDTH_PX: f32 = 275.0;
/// A text holds at most this many messages; the oldest drops.
pub(super) const MAX_MESSAGES: usize = 10;
const MIN_LIFETIME_MS: u64 = 3000;
const PER_CHAR_MS: u64 = 60;
/// The text rect sits at its tile point minus (width / 2, height) plus this.
const PLACEMENT_OFFSET: (f32, f32) = (20.0, 5.0);

pub(super) type Position = (i32, i32, i16);

/// One local line, reduced to what the map text needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Speech {
    pub name: String,
    pub mode: ChatSpeechMode,
    pub text: String,
    pub position: Position,
}

impl Speech {
    pub fn from_line(line: &ChatLine) -> Option<Self> {
        let ChatLine::Local {
            speaker_name,
            mode,
            text,
            position,
            ..
        } = line
        else {
            return None;
        };
        Some(Self {
            name: speaker_name.clone(),
            mode: *mode,
            text: text.clone(),
            position: (position.x, position.y, position.floor),
        })
    }
}

/// The lines of `lines` that are not already in `previous`: the longest suffix of `previous`
/// that opens `lines` is the part both logs share.
pub(super) fn new_lines<'a>(previous: &[ChatLine], lines: &'a [ChatLine]) -> &'a [ChatLine] {
    let overlap = (0..=previous.len().min(lines.len()))
        .rev()
        .find(|&n| previous[previous.len() - n..] == lines[..n])
        .unwrap_or(0);
    &lines[overlap..]
}

/// How long a message stays: 60 ms per character, at least 3 s, doubled for a yell.
pub(super) fn lifetime_ms(mode: ChatSpeechMode, text: &str) -> u64 {
    let base = (text.chars().count() as u64 * PER_CHAR_MS).max(MIN_LIFETIME_MS);
    if mode == ChatSpeechMode::Yell {
        base * 2
    } else {
        base
    }
}

/// The header line of a mode; a monster speaks without one.
pub(super) fn header(mode: ChatSpeechMode, name: &str) -> String {
    match mode {
        ChatSpeechMode::Say => format!("{name} says:\n"),
        ChatSpeechMode::Whisper => format!("{name} whispers:\n"),
        ChatSpeechMode::Yell => format!("{name} yells:\n"),
        ChatSpeechMode::MonsterSay => String::new(),
    }
}

pub(super) const fn colour(mode: ChatSpeechMode) -> [u8; 3] {
    match mode {
        ChatSpeechMode::MonsterSay => [254, 101, 0],
        ChatSpeechMode::Say | ChatSpeechMode::Whisper | ChatSpeechMode::Yell => [239, 239, 0],
    }
}

/// Top-left of a text of `size` for the tile point `dest`, kept inside `viewport`.
pub(super) fn layout_rect(dest: (f32, f32), size: (f32, f32), viewport: (f32, f32)) -> (f32, f32) {
    let x = dest.0 - size.0 / 2.0 + PLACEMENT_OFFSET.0;
    let y = dest.1 - size.1 + PLACEMENT_OFFSET.1;
    (
        x.min(viewport.0 - size.0).max(0.0),
        y.min(viewport.1 - size.1).max(0.0),
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Message {
    text: String,
    expires_ms: u64,
}

/// The messages one speaker has at one position.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct StaticText {
    name: String,
    mode: ChatSpeechMode,
    position: Position,
    messages: Vec<Message>,
}

impl StaticText {
    pub const fn position(&self) -> Position {
        self.position
    }

    pub const fn mode(&self) -> ChatSpeechMode {
        self.mode
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// The header, then the messages joined by newlines.
    pub fn compose(&self) -> String {
        let body: Vec<&str> = self.messages.iter().map(|m| m.text.as_str()).collect();
        header(self.mode, &self.name) + &body.join("\n")
    }

    fn push(&mut self, text: &str, now_ms: u64) {
        if self.messages.len() >= MAX_MESSAGES {
            self.messages.remove(0);
        }
        self.messages.push(Message {
            text: text.to_owned(),
            expires_ms: now_ms + lifetime_ms(self.mode, text),
        });
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Texts(Vec<StaticText>);

impl Texts {
    pub fn add(&mut self, speech: &Speech, now_ms: u64) {
        if let Some(text) = self.0.iter_mut().find(|t| {
            t.position == speech.position && t.name == speech.name && t.mode == speech.mode
        }) {
            text.push(&speech.text, now_ms);
            return;
        }
        let mut text = StaticText {
            name: speech.name.clone(),
            mode: speech.mode,
            position: speech.position,
            messages: Vec::new(),
        };
        text.push(&speech.text, now_ms);
        self.0.push(text);
    }

    /// Removes each text's first message once it has expired, and a text left without any.
    pub fn expire(&mut self, now_ms: u64) {
        for text in &mut self.0 {
            while text
                .messages
                .first()
                .is_some_and(|m| m.expires_ms <= now_ms)
            {
                text.messages.remove(0);
            }
        }
        self.0.retain(|t| !t.messages.is_empty());
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Only the texts of the camera floor are drawn.
    pub fn on_floor(&self, floor: i16) -> impl Iterator<Item = &StaticText> {
        self.0.iter().filter(move |t| t.position.2 == floor)
    }
}
