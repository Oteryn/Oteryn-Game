//! S17 strict candidate wire. Family 16 is distinct from dormant SPECTATE1 (15)
//! and player commands. Offers require the composed native consumers.
pub use crate::charm_wire::CyclopediaWireError as WireError;
use crate::charm_wire::{push_message_field, push_varint_field, read_bytes, read_varint};
mod media;
pub use media::{DisplayTile, PublicMedia};
pub const CAPABILITY_CAST_VIEW_V1: u32 = 289;
pub const CAPABILITY_CAST_CONTROL_V1: u32 = 290;
pub const CAPABILITY_CAST_CHAT_V1: u32 = 291;
pub const CAPABILITY_CAST_CONSENT_V1: u32 = 292;
pub const COMMAND_TYPE_CAST_CONTROL: u32 = 502;
pub const COMMAND_TYPE_CAST_CONSENT: u32 = 503;
pub const COMMAND_TYPE_CAST_CHAT: u32 = 504;
pub const COMMAND_TYPE_CAST_MODERATE: u32 = 505;
pub const STATE_DOMAIN_CAST_VIEW: u32 = 102;
pub const STATE_DOMAIN_CAST_CASTER_CHAT: u32 = 103;
pub const CAST_OBSERVER_FAMILY: u64 = 16;
/// CLCAST: next free Message kind 22 plus assigned lane offset 30.
pub const MESSAGE_KIND_CAST_CONSENT_QUERY: u64 = 52;
pub const MESSAGE_KIND_CAST_READY: u64 = 53;
pub const MESSAGE_KIND_CAST_ROSTER: u64 = 54;
pub const MESSAGE_KIND_CAST_CONTROL_QUERY: u64 = 55;
/// Next domain-103 delta type 2 plus CLCAST offset 30.
pub const DELTA_TYPE_CAST_ROSTER: u32 = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scene {
    pub tiles: Vec<(u16, u16, u16)>,
    pub actors: Vec<(u32, u16, u16, u16, u8)>,
}

#[derive(Debug, Clone, Copy)]
pub struct Selection {
    pub view: bool,
    pub control: bool,
    pub chat: bool,
    pub consent: bool,
}
impl Selection {
    pub fn validate(self, observer: bool) -> bool {
        if observer {
            self.view && !self.control && !self.consent
        } else {
            !self.chat || self.control
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Ready {
        epoch: u64,
        chat: bool,
        delay: u16,
    },
    CasterRoster {
        revision: u64,
        subjects: Vec<[u8; 16]>,
    },
    ConsentQuery,
    ControlQuery,
    Media {
        epoch: u64,
        sequence: u64,
        media: PublicMedia,
    },
    /// Owner-only control result. Never valid in an observer envelope.
    CasterState {
        cast_id: [u8; 16],
        revision: u64,
        epoch: u64,
        delay: u16,
        count: u8,
    },
    CasterChatEmpty,
    Scene {
        epoch: u64,
        sequence: u64,
        scene: Scene,
    },
    Start {
        revision: u64,
        access_revision: u64,
        delay: u16,
        cap: u8,
        password: bool,
    },
    Update {
        revision: u64,
        access_revision: u64,
        delay: u16,
        cap: u8,
        password: bool,
    },
    Stop {
        revision: u64,
    },
    ConsentSet {
        revision: u64,
        allowed: bool,
    },
    ConsentState {
        revision: u64,
        allowed: bool,
    },
    Join(Vec<u8>),
    Leave,
    Keepalive,
    Chat(String),
    Reply(String),
    Moderate {
        revision: u64,
        subject: [u8; 16],
        action: u8,
        key: [u8; 16],
    },
    State {
        revision: u64,
        epoch: u64,
        delay: u16,
        count: u8,
    },
    Slate {
        epoch: u64,
        sequence: u64,
    },
    Clear {
        epoch: u64,
    },
    ChatLine {
        epoch: u64,
        sequence: u64,
        handle: [u8; 16],
        text: String,
    },
    CasterLine {
        handle: [u8; 16],
        text: String,
    },
    End,
}
impl Message {
    pub fn client_to_server(&self) -> bool {
        matches!(
            self,
            Self::ConsentQuery
                | Self::ControlQuery
                | Self::Start { .. }
                | Self::Update { .. }
                | Self::Stop { .. }
                | Self::ConsentSet { .. }
                | Self::Join(_)
                | Self::Leave
                | Self::Keepalive
                | Self::Chat(_)
                | Self::Reply(_)
                | Self::Moderate { .. }
        )
    }
    pub fn encode(&self) -> Result<Vec<u8>, WireError> {
        let mut b = Vec::new();
        let kind = match self {
            Self::Ready { .. } => MESSAGE_KIND_CAST_READY,
            Self::CasterRoster { .. } => MESSAGE_KIND_CAST_ROSTER,
            Self::ConsentQuery => MESSAGE_KIND_CAST_CONSENT_QUERY,
            Self::ControlQuery => MESSAGE_KIND_CAST_CONTROL_QUERY,
            Self::Media { .. } => 21,
            Self::CasterState { .. } => 20,
            Self::CasterChatEmpty => 19,
            Self::Scene { .. } => 18,
            Self::Start { .. } => 1,
            Self::Update { .. } => 2,
            Self::Stop { .. } => 3,
            Self::ConsentSet { .. } => 4,
            Self::ConsentState { .. } => 5,
            Self::Join(_) => 6,
            Self::Leave => 7,
            Self::Keepalive => 8,
            Self::Chat(_) => 9,
            Self::Reply(_) => 10,
            Self::Moderate { .. } => 11,
            Self::State { .. } => 12,
            Self::Slate { .. } => 13,
            Self::Clear { .. } => 14,
            Self::ChatLine { .. } => 15,
            Self::CasterLine { .. } => 16,
            Self::End => 17,
        };
        push_varint_field(&mut b, 1, kind);
        match self {
            Self::Ready { epoch, chat, delay } => {
                if *epoch == 0 || *delay > 300 {
                    return Err(WireError::Malformed);
                }
                push_varint_field(&mut b, 2, *epoch);
                push_varint_field(&mut b, 3, u64::from(*chat));
                push_varint_field(&mut b, 4, u64::from(*delay));
            }
            Self::CasterRoster { revision, subjects } => {
                if *revision == 0
                    || subjects.len() > 50
                    || subjects.contains(&[0; 16])
                    || subjects.windows(2).any(|pair| pair[0] >= pair[1])
                {
                    return Err(WireError::Malformed);
                }
                push_varint_field(&mut b, 2, *revision);
                for id in subjects {
                    push_message_field(&mut b, 3, id);
                }
            }
            Self::Media {
                epoch,
                sequence,
                media,
            } => {
                if *epoch == 0 || *sequence == 0 {
                    return Err(WireError::Malformed);
                }
                push_varint_field(&mut b, 2, *epoch);
                push_varint_field(&mut b, 3, *sequence);
                push_message_field(&mut b, 4, &media.encode()?);
            }
            Self::Scene {
                epoch,
                sequence,
                scene,
            } => {
                if *epoch == 0
                    || *sequence == 0
                    || scene.tiles.len() > 512
                    || scene.actors.len() > 128
                    || scene.actors.iter().any(|a| a.4 > 100)
                {
                    return Err(WireError::Malformed);
                }
                push_varint_field(&mut b, 2, *epoch);
                push_varint_field(&mut b, 3, *sequence);
                for &(x, y, item) in &scene.tiles {
                    let mut tile = Vec::new();
                    for (i, v) in [x, y, item].into_iter().enumerate() {
                        push_varint_field(&mut tile, i as u64 + 1, u64::from(v));
                    }
                    push_message_field(&mut b, 4, &tile);
                }
                for &(handle, x, y, appearance, health) in &scene.actors {
                    let mut actor = Vec::new();
                    for (i, v) in [
                        u64::from(handle),
                        u64::from(x),
                        u64::from(y),
                        u64::from(appearance),
                        u64::from(health),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        push_varint_field(&mut actor, i as u64 + 1, v);
                    }
                    push_message_field(&mut b, 5, &actor);
                }
            }
            Self::Start {
                revision,
                access_revision,
                delay,
                cap,
                password,
            }
            | Self::Update {
                revision,
                access_revision,
                delay,
                cap,
                password,
            } => {
                if *delay > 300 || !(1..=50).contains(cap) || *access_revision == 0 {
                    return Err(WireError::Malformed);
                }
                for (field, value) in [
                    (2, *revision),
                    (3, *access_revision),
                    (4, u64::from(*delay)),
                    (5, u64::from(*cap)),
                    (6, u64::from(*password)),
                ] {
                    push_varint_field(&mut b, field, value);
                }
            }
            Self::Stop { revision } => push_varint_field(&mut b, 2, *revision),
            Self::ConsentSet { revision, allowed } | Self::ConsentState { revision, allowed } => {
                push_varint_field(&mut b, 2, *revision);
                push_varint_field(&mut b, 3, u64::from(*allowed));
            }
            Self::Join(ticket) => {
                if ticket.is_empty() || ticket.len() > 4096 {
                    return Err(WireError::LimitExceeded);
                }
                push_message_field(&mut b, 2, ticket);
            }
            Self::Chat(text) | Self::Reply(text) => text_field(&mut b, 2, text)?,
            Self::Moderate {
                revision,
                subject,
                action,
                key,
            } => {
                if !(1..=4).contains(action) || *key == [0; 16] {
                    return Err(WireError::Malformed);
                }
                push_varint_field(&mut b, 2, *revision);
                push_message_field(&mut b, 3, subject);
                push_varint_field(&mut b, 4, u64::from(*action));
                push_message_field(&mut b, 5, key);
            }
            Self::State {
                revision,
                epoch,
                delay,
                count,
            }
            | Self::CasterState {
                revision,
                epoch,
                delay,
                count,
                ..
            } => {
                if *epoch == 0 || *delay > 300 || *count > 50 {
                    return Err(WireError::Malformed);
                }
                for (field, value) in [
                    (2, *revision),
                    (3, *epoch),
                    (4, u64::from(*delay)),
                    (5, u64::from(*count)),
                ] {
                    push_varint_field(&mut b, field, value);
                }
                if let Self::CasterState { cast_id, .. } = self {
                    if *cast_id == [0; 16] {
                        return Err(WireError::Malformed);
                    }
                    push_message_field(&mut b, 6, cast_id);
                }
            }
            Self::Slate { epoch, sequence } => {
                if *epoch == 0 || *sequence == 0 {
                    return Err(WireError::Malformed);
                }
                push_varint_field(&mut b, 2, *epoch);
                push_varint_field(&mut b, 3, *sequence);
            }
            Self::Clear { epoch } => {
                if *epoch == 0 {
                    return Err(WireError::Malformed);
                }
                push_varint_field(&mut b, 2, *epoch);
            }
            Self::ChatLine {
                epoch,
                sequence,
                handle,
                text,
            } => {
                if *epoch == 0 || *sequence == 0 {
                    return Err(WireError::Malformed);
                }
                push_varint_field(&mut b, 2, *epoch);
                push_varint_field(&mut b, 3, *sequence);
                push_message_field(&mut b, 4, handle);
                line_field(&mut b, 5, text)?;
            }
            Self::CasterLine { handle, text } => {
                push_message_field(&mut b, 2, handle);
                line_field(&mut b, 3, text)?;
            }
            Self::Leave
            | Self::Keepalive
            | Self::End
            | Self::CasterChatEmpty
            | Self::ConsentQuery
            | Self::ControlQuery => {}
        }
        Ok(b)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, WireError> {
        if bytes.len() > 16384 {
            return Err(WireError::LimitExceeded);
        }
        let mut r = Reader { bytes, cursor: 0 };
        let kind = r.var(1)?;
        let result = match kind {
            MESSAGE_KIND_CAST_READY => Self::Ready {
                epoch: r.var(2)?,
                chat: r.boolean(3)?,
                delay: r.narrow(4)?,
            },
            MESSAGE_KIND_CAST_ROSTER => {
                let revision = r.var(2)?;
                let mut subjects = Vec::new();
                while r.bytes.get(r.cursor) == Some(&26) {
                    if subjects.len() >= 50 {
                        return Err(WireError::LimitExceeded);
                    }
                    subjects.push(r.id(3)?);
                }
                Self::CasterRoster { revision, subjects }
            }
            MESSAGE_KIND_CAST_CONSENT_QUERY => Self::ConsentQuery,
            MESSAGE_KIND_CAST_CONTROL_QUERY => Self::ControlQuery,
            21 => Self::Media {
                epoch: r.var(2)?,
                sequence: r.var(3)?,
                media: PublicMedia::decode(r.data(4, 8192)?)?,
            },
            20 => Self::CasterState {
                revision: r.var(2)?,
                epoch: r.var(3)?,
                delay: r.narrow(4)?,
                count: r.narrow(5)?,
                cast_id: r.id(6)?,
            },
            19 => Self::CasterChatEmpty,
            18 => {
                let epoch = r.var(2)?;
                let sequence = r.var(3)?;
                let mut tiles = Vec::new();
                let mut actors = Vec::new();
                while r.bytes.get(r.cursor) == Some(&34) {
                    if tiles.len() >= 512 {
                        return Err(WireError::LimitExceeded);
                    }
                    let bytes = r.data(4, 32)?;
                    let mut tile = Reader { bytes, cursor: 0 };
                    tiles.push((tile.narrow(1)?, tile.narrow(2)?, tile.narrow(3)?));
                    if tile.cursor != bytes.len() {
                        return Err(WireError::Malformed);
                    }
                }
                while r.bytes.get(r.cursor) == Some(&42) {
                    if actors.len() >= 128 {
                        return Err(WireError::LimitExceeded);
                    }
                    let bytes = r.data(5, 48)?;
                    let mut actor = Reader { bytes, cursor: 0 };
                    actors.push((
                        actor.narrow(1)?,
                        actor.narrow(2)?,
                        actor.narrow(3)?,
                        actor.narrow(4)?,
                        actor.narrow(5)?,
                    ));
                    if actor.cursor != bytes.len() {
                        return Err(WireError::Malformed);
                    }
                }
                Self::Scene {
                    epoch,
                    sequence,
                    scene: Scene { tiles, actors },
                }
            }
            1 | 2 => {
                let revision = r.var(2)?;
                let access_revision = r.var(3)?;
                let delay = r.var(4)?.try_into().map_err(|_| WireError::Malformed)?;
                let cap = r.var(5)?.try_into().map_err(|_| WireError::Malformed)?;
                let password = r.boolean(6)?;
                if kind == 1 {
                    Self::Start {
                        revision,
                        access_revision,
                        delay,
                        cap,
                        password,
                    }
                } else {
                    Self::Update {
                        revision,
                        access_revision,
                        delay,
                        cap,
                        password,
                    }
                }
            }
            3 => Self::Stop {
                revision: r.var(2)?,
            },
            4 => Self::ConsentSet {
                revision: r.var(2)?,
                allowed: r.boolean(3)?,
            },
            5 => Self::ConsentState {
                revision: r.var(2)?,
                allowed: r.boolean(3)?,
            },
            6 => Self::Join(r.data(2, 4096)?.to_vec()),
            7 => Self::Leave,
            8 => Self::Keepalive,
            9 => Self::Chat(r.text(2)?),
            10 => Self::Reply(r.text(2)?),
            11 => Self::Moderate {
                revision: r.var(2)?,
                subject: r.id(3)?,
                action: r.var(4)?.try_into().map_err(|_| WireError::Malformed)?,
                key: r.id(5)?,
            },
            12 => Self::State {
                revision: r.var(2)?,
                epoch: r.var(3)?,
                delay: r.var(4)?.try_into().map_err(|_| WireError::Malformed)?,
                count: r.var(5)?.try_into().map_err(|_| WireError::Malformed)?,
            },
            13 => Self::Slate {
                epoch: r.var(2)?,
                sequence: r.var(3)?,
            },
            14 => Self::Clear { epoch: r.var(2)? },
            15 => Self::ChatLine {
                epoch: r.var(2)?,
                sequence: r.var(3)?,
                handle: r.id(4)?,
                text: r.line(5)?,
            },
            16 => Self::CasterLine {
                handle: r.id(2)?,
                text: r.line(3)?,
            },
            17 => Self::End,
            _ => return Err(WireError::Malformed),
        };
        if r.cursor != bytes.len() || result.encode()?.as_slice() != bytes {
            return Err(WireError::Malformed);
        }
        Ok(result)
    }
}
fn line_field(b: &mut Vec<u8>, field: u64, text: &str) -> Result<(), WireError> {
    if text.is_empty() || text.len() > 320 || text.chars().any(char::is_control) {
        return Err(WireError::Malformed);
    }
    push_message_field(b, field, text.as_bytes());
    Ok(())
}
fn text_field(b: &mut Vec<u8>, field: u64, text: &str) -> Result<(), WireError> {
    if text.is_empty() || text.len() > 256 || text.chars().any(char::is_control) {
        return Err(WireError::Malformed);
    }
    push_message_field(b, field, text.as_bytes());
    Ok(())
}
struct Reader<'a> {
    bytes: &'a [u8],
    cursor: usize,
}
impl<'a> Reader<'a> {
    fn line(&mut self, field: u64) -> Result<String, WireError> {
        std::str::from_utf8(self.data(field, 320)?)
            .map(str::to_owned)
            .map_err(|_| WireError::Malformed)
    }
    fn narrow<T: TryFrom<u64>>(&mut self, field: u64) -> Result<T, WireError> {
        self.var(field)?
            .try_into()
            .map_err(|_| WireError::Malformed)
    }
    fn var(&mut self, field: u64) -> Result<u64, WireError> {
        if read_varint(self.bytes, &mut self.cursor)? != field << 3 {
            return Err(WireError::Malformed);
        }
        read_varint(self.bytes, &mut self.cursor)
    }
    fn boolean(&mut self, field: u64) -> Result<bool, WireError> {
        match self.var(field)? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(WireError::Malformed),
        }
    }
    fn data(&mut self, field: u64, max: usize) -> Result<&'a [u8], WireError> {
        if read_varint(self.bytes, &mut self.cursor)? != (field << 3) | 2 {
            return Err(WireError::Malformed);
        }
        let bytes = read_bytes(self.bytes, &mut self.cursor)?;
        if bytes.len() > max {
            return Err(WireError::LimitExceeded);
        }
        Ok(bytes)
    }
    fn id(&mut self, field: u64) -> Result<[u8; 16], WireError> {
        self.data(field, 16)?
            .try_into()
            .map_err(|_| WireError::Malformed)
    }
    fn text(&mut self, field: u64) -> Result<String, WireError> {
        std::str::from_utf8(self.data(field, 256)?)
            .map(str::to_owned)
            .map_err(|_| WireError::Malformed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    pub generation: u64,
    pub message: Message,
}
impl Envelope {
    pub fn encode(&self) -> Result<Vec<u8>, WireError> {
        if !matches!(
            self.message,
            Message::Join(_)
                | Message::Ready { .. }
                | Message::Leave
                | Message::Keepalive
                | Message::Chat(_)
                | Message::Scene { .. }
                | Message::Media { .. }
                | Message::State { .. }
                | Message::Slate { .. }
                | Message::Clear { .. }
                | Message::ChatLine { .. }
                | Message::End
        ) {
            return Err(WireError::Malformed);
        }
        if (self.generation == 0) != matches!(self.message, Message::Join(_)) {
            return Err(WireError::Malformed);
        }
        let mut b = Vec::new();
        push_varint_field(&mut b, 1, CAST_OBSERVER_FAMILY);
        push_varint_field(&mut b, 2, self.generation);
        push_message_field(&mut b, 4, &self.message.encode()?);
        Ok(b)
    }
    pub fn decode(bytes: &[u8], from_client: bool) -> Result<Self, WireError> {
        if bytes.len() > 16432 {
            return Err(WireError::LimitExceeded);
        }
        let mut r = Reader { bytes, cursor: 0 };
        if r.var(1)? != CAST_OBSERVER_FAMILY {
            return Err(WireError::Malformed);
        }
        let generation = r.var(2)?;
        let message = Message::decode(r.data(4, 16384)?)?;
        // Observer ingress is an allowlist, never a player-command envelope.
        if message.client_to_server() != from_client
            || (from_client
                && !matches!(
                    message,
                    Message::Join(_) | Message::Leave | Message::Keepalive | Message::Chat(_)
                ))
        {
            return Err(WireError::Malformed);
        }
        let value = Self {
            generation,
            message,
        };
        if r.cursor != bytes.len() || value.encode()?.as_slice() != bytes {
            return Err(WireError::Malformed);
        }
        Ok(value)
    }
}
