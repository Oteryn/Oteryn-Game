//! The sealed World chat relay payload (CHAT-0 §5, CHAT-2).
//!
//! Pure rules of the relay: the fixed binary layout, AES-256-GCM sealing with a per-World key ring
//! and its 24 hour rotation overlap, the freshness check and the bounded de-duplication window.
//! The `LISTEN`/`NOTIFY` connection, the readiness gate and the delivery to sessions feed these
//! rules and live outside this module. Chat text is never logged or stored here (FND-02 §20).

use std::collections::{BTreeSet, HashMap};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use aws_lc_rs::aead::{AES_256_GCM, Aad, LessSafeKey, Nonce, UnboundKey};
use aws_lc_rs::rand::fill;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use oteryn_protocol_oteryn::chat::ChatRoom;

use super::TEXT_MAX_BYTES;

/// `CHAT0-RL-06`: the `NOTIFY` text is at most 2,048 bytes.
pub(crate) const NOTIFY_MAX_BYTES: usize = 2_048;
/// `CHAT0-RL-12`: a retired relay key still opens lines for 24 hours.
pub(crate) const KEY_OVERLAP_MICROS: u64 = 24 * 60 * 60 * 1_000_000;
/// `CHAT0-RL-13`: a line older than 30 s is dropped; its `message_id` is remembered that long.
pub(crate) const FRESHNESS_MICROS: u64 = 30 * 1_000_000;
/// Distinct `message_id`s kept per World inside the window. A full window refuses new lines
/// rather than forgetting a fresh one, so a captured line can never be admitted twice.
pub(crate) const REPLAY_WINDOW_LINES: usize = 8_192;
/// The sender name is at most 120 bytes (CHAT-0 §5).
pub(crate) const SENDER_NAME_MAX_BYTES: usize = 120;

const KEY_BYTES: usize = 32;
const NONCE_BYTES: usize = 12;
const TAG_BYTES: usize = 16;
const ID_BYTES: usize = 16;
/// kind, message id, sent at, destination (a recipient id, the widest), two length prefixes.
const PLAINTEXT_MAX_BYTES: usize =
    1 + ID_BYTES + 8 + ID_BYTES + 2 + SENDER_NAME_MAX_BYTES + 2 + TEXT_MAX_BYTES;
/// `key_id` + nonce + ciphertext + tag: at most 1,216 bytes (CHAT-0 §5).
pub(crate) const SEALED_MAX_BYTES: usize = 1 + NONCE_BYTES + PLAINTEXT_MAX_BYTES + TAG_BYTES;
const KIND_PRIVATE: u8 = 1;
const KIND_ROOM: u8 = 2;
const CHANNEL_PREFIX: &str = "oteryn_chat_";

/// Why a relay line is refused. Every refusal drops the line; none reaches a client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RelayError {
    /// The line would not fit the sealed bound, or a field is out of bounds.
    Oversized,
    /// The World id cannot name a `NOTIFY` channel.
    BadChannel,
    /// No usable relay key: none configured, or the key set is invalid.
    NoKey,
    /// The `NOTIFY` text is not standard base64 or is too short to hold a sealed line.
    Malformed,
    /// The line names a `key_id` the node does not hold, or one retired over 24 hours ago.
    UnknownKey,
    /// Authentication failed: a forged, altered or foreign-channel line.
    Unauthentic,
    /// The plaintext is not a valid layout.
    BadPayload,
    /// `sent_at` is outside the freshness window.
    Stale,
    /// The `message_id` was seen inside the window.
    Replayed,
    /// The de-duplication window is full of fresh lines.
    Saturated,
    /// The system random source failed.
    Random,
}

impl Display for RelayError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Oversized => "chat relay line is over the sealed bound",
            Self::BadChannel => "chat relay World id cannot name a channel",
            Self::NoKey => "chat relay has no usable key",
            Self::Malformed => "chat relay text is malformed",
            Self::UnknownKey => "chat relay line names an unknown or expired key",
            Self::Unauthentic => "chat relay line failed authentication",
            Self::BadPayload => "chat relay payload is malformed",
            Self::Stale => "chat relay line is outside the freshness window",
            Self::Replayed => "chat relay line was already seen",
            Self::Saturated => "chat relay de-duplication window is full",
            Self::Random => "chat relay random source failed",
        })
    }
}

impl Error for RelayError {}

/// Where a relayed line goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RelayDestination {
    /// A private message, keyed by the recipient CharacterId (server side only).
    Private { recipient: [u8; ID_BYTES] },
    /// A World room: each node delivers to its sessions that opened it.
    Room { room: ChatRoom },
}

/// The plaintext of one relayed line (CHAT-0 §5 layout).
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct RelayLine {
    pub(crate) message_id: [u8; ID_BYTES],
    /// The sender's database time, microseconds.
    pub(crate) sent_at_micros: u64,
    pub(crate) destination: RelayDestination,
    pub(crate) sender_name: String,
    pub(crate) text: String,
}

// Text is private chat; it never reaches a log through `Debug` (FND-02 §20).
impl fmt::Debug for RelayLine {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("RelayLine")
            .field("destination", &self.destination)
            .finish_non_exhaustive()
    }
}

impl RelayLine {
    fn encode(&self) -> Result<Vec<u8>, RelayError> {
        if self.sender_name.len() > SENDER_NAME_MAX_BYTES || self.text.len() > TEXT_MAX_BYTES {
            return Err(RelayError::Oversized);
        }
        let mut out = Vec::with_capacity(PLAINTEXT_MAX_BYTES);
        match self.destination {
            RelayDestination::Private { .. } => out.push(KIND_PRIVATE),
            RelayDestination::Room { .. } => out.push(KIND_ROOM),
        }
        out.extend_from_slice(&self.message_id);
        out.extend_from_slice(&self.sent_at_micros.to_be_bytes());
        match self.destination {
            RelayDestination::Private { recipient } => out.extend_from_slice(&recipient),
            RelayDestination::Room { room } => out.push(room as u8),
        }
        push_prefixed(&mut out, self.sender_name.as_bytes())?;
        push_prefixed(&mut out, self.text.as_bytes())?;
        Ok(out)
    }

    fn decode(bytes: &[u8]) -> Result<Self, RelayError> {
        let mut cursor = Cursor(bytes);
        let kind = cursor.take(1)?[0];
        let message_id = cursor.array()?;
        let sent_at_micros = u64::from_be_bytes(cursor.array()?);
        let destination = match kind {
            KIND_PRIVATE => RelayDestination::Private {
                recipient: cursor.array()?,
            },
            KIND_ROOM => {
                let code = cursor.take(1)?[0];
                let room = ChatRoom::ALL
                    .into_iter()
                    .find(|room| *room as u8 == code)
                    .ok_or(RelayError::BadPayload)?;
                RelayDestination::Room { room }
            }
            _ => return Err(RelayError::BadPayload),
        };
        let sender_name = cursor.prefixed(SENDER_NAME_MAX_BYTES)?;
        let text = cursor.prefixed(TEXT_MAX_BYTES)?;
        if !cursor.0.is_empty() {
            return Err(RelayError::BadPayload);
        }
        Ok(Self {
            message_id,
            sent_at_micros,
            destination,
            sender_name,
            text,
        })
    }
}

fn push_prefixed(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), RelayError> {
    let len = u16::try_from(bytes.len()).map_err(|_| RelayError::Oversized)?;
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(bytes);
    Ok(())
}

struct Cursor<'a>(&'a [u8]);

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], RelayError> {
        if self.0.len() < n {
            return Err(RelayError::BadPayload);
        }
        let (head, rest) = self.0.split_at(n);
        self.0 = rest;
        Ok(head)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], RelayError> {
        self.take(N)?.try_into().map_err(|_| RelayError::BadPayload)
    }

    fn prefixed(&mut self, max: usize) -> Result<String, RelayError> {
        let len = usize::from(u16::from_be_bytes(self.array()?));
        if len > max {
            return Err(RelayError::BadPayload);
        }
        String::from_utf8(self.take(len)?.to_vec()).map_err(|_| RelayError::BadPayload)
    }
}

/// The `NOTIFY` channel of a World: `oteryn_chat_<32 hex digits>`, the canonical lower-case
/// UUID of the WorldId (as `node/config.rs` admits it, 36 characters with hyphens) without its
/// hyphens. The name is a safe unquoted identifier, 44 bytes, and distinct per World.
pub(crate) fn channel_name(world_id: &str) -> Result<String, RelayError> {
    let bytes = world_id.as_bytes();
    let canonical = bytes.len() == 36
        && bytes.iter().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => *byte == b'-',
            _ => byte.is_ascii_digit() || (b'a'..=b'f').contains(byte),
        });
    if !canonical {
        return Err(RelayError::BadChannel);
    }
    let hex: String = world_id.chars().filter(|c| *c != '-').collect();
    Ok(format!("{CHANNEL_PREFIX}{hex}"))
}

/// One relay key of a World. The bytes come from an environment secret, never the repository.
#[derive(Clone)]
pub(crate) struct RelayKey {
    key_id: u8,
    /// The cluster-wide instant from which the key is used to send. Every node opens with the key
    /// as soon as it holds it, but sends with it only from here, set after the whole cluster has
    /// the key, so a rolling update never sends a key a node does not hold yet.
    activate_at_micros: u64,
    key: [u8; KEY_BYTES],
}

impl RelayKey {
    pub(crate) fn new(key_id: u8, activate_at_micros: u64, key: [u8; KEY_BYTES]) -> Self {
        Self {
            key_id,
            activate_at_micros,
            key,
        }
    }
}

impl fmt::Debug for RelayKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("RelayKey")
            .field("key_id", &self.key_id)
            .finish_non_exhaustive()
    }
}

/// The keys a node holds for one World. It sends with the newest key already activated and opens
/// with every key held except one retired (superseded by an activated key) over `CHAT0-RL-12`.
#[derive(Debug, Clone)]
pub(crate) struct RelayKeyRing {
    /// Sorted by `activate_at_micros`; `key_id`s are distinct.
    keys: Vec<RelayKey>,
}

impl RelayKeyRing {
    pub(crate) fn new(mut keys: Vec<RelayKey>) -> Result<Self, RelayError> {
        if keys.is_empty() {
            return Err(RelayError::NoKey);
        }
        keys.sort_by_key(|key| (key.activate_at_micros, key.key_id));
        let mut ids: Vec<u8> = keys.iter().map(|key| key.key_id).collect();
        ids.sort_unstable();
        ids.dedup();
        if ids.len() != keys.len() {
            return Err(RelayError::NoKey);
        }
        Ok(Self { keys })
    }

    fn sending(&self, now_micros: u64) -> Result<&RelayKey, RelayError> {
        self.keys
            .iter()
            .rev()
            .find(|key| key.activate_at_micros <= now_micros)
            .ok_or(RelayError::NoKey)
    }

    fn opening(&self, key_id: u8, now_micros: u64) -> Result<&RelayKey, RelayError> {
        let index = self
            .keys
            .iter()
            .position(|key| key.key_id == key_id)
            .ok_or(RelayError::UnknownKey)?;
        // A key is retired when the next one is activated and opens lines for 24 hours after that.
        if let Some(next) = self.keys.get(index + 1)
            && now_micros >= next.activate_at_micros.saturating_add(KEY_OVERLAP_MICROS)
        {
            return Err(RelayError::UnknownKey);
        }
        Ok(&self.keys[index])
    }
}

fn aead_key(key: &RelayKey) -> Result<LessSafeKey, RelayError> {
    UnboundKey::new(&AES_256_GCM, &key.key)
        .map(LessSafeKey::new)
        .map_err(|_| RelayError::NoKey)
}

/// A fresh random 96-bit nonce per line (many nodes share one key, so no counter).
fn random_nonce() -> Result<[u8; NONCE_BYTES], RelayError> {
    let mut nonce = [0_u8; NONCE_BYTES];
    fill(&mut nonce).map_err(|_| RelayError::Random)?;
    Ok(nonce)
}

/// A fresh random 128-bit `message_id`; only [`seal`] draws one, so no caller can pick or reuse it.
fn random_message_id() -> Result<[u8; ID_BYTES], RelayError> {
    let mut id = [0_u8; ID_BYTES];
    fill(&mut id).map_err(|_| RelayError::Random)?;
    Ok(id)
}

/// What a sender supplies for one line; the `message_id` is generated inside [`seal`].
pub(crate) struct OutgoingLine {
    /// The sender's database time, microseconds.
    pub(crate) sent_at_micros: u64,
    pub(crate) destination: RelayDestination,
    pub(crate) sender_name: String,
    pub(crate) text: String,
}

/// Seals `line` for `channel` with the newest key and a fresh random nonce, and returns the
/// `NOTIFY` text (standard base64 of `key_id || nonce || ciphertext || tag`). The associated data
/// is the channel name and the `key_id`. A line over the bound is `Oversized`, never truncated or
/// split. The `message_id` and the nonce are drawn here so no caller can repeat either.
pub(crate) fn seal(
    ring: &RelayKeyRing,
    channel: &str,
    line: OutgoingLine,
) -> Result<String, RelayError> {
    let line = RelayLine {
        message_id: random_message_id()?,
        sent_at_micros: line.sent_at_micros,
        destination: line.destination,
        sender_name: line.sender_name,
        text: line.text,
    };
    seal_with_nonce(ring, channel, &line, random_nonce()?)
}

fn seal_with_nonce(
    ring: &RelayKeyRing,
    channel: &str,
    line: &RelayLine,
    nonce: [u8; NONCE_BYTES],
) -> Result<String, RelayError> {
    let key = ring.sending(line.sent_at_micros)?;
    let mut buffer = line.encode()?;
    let sealing = aead_key(key)?;
    let aad = [channel.as_bytes(), &[key.key_id]].concat();
    sealing
        .seal_in_place_append_tag(
            Nonce::assume_unique_for_key(nonce),
            Aad::from(aad),
            &mut buffer,
        )
        .map_err(|_| RelayError::Oversized)?;
    let mut sealed = Vec::with_capacity(1 + NONCE_BYTES + buffer.len());
    sealed.push(key.key_id);
    sealed.extend_from_slice(&nonce);
    sealed.extend_from_slice(&buffer);
    if sealed.len() > SEALED_MAX_BYTES {
        return Err(RelayError::Oversized);
    }
    let text = STANDARD.encode(sealed);
    if text.len() > NOTIFY_MAX_BYTES {
        return Err(RelayError::Oversized);
    }
    Ok(text)
}

/// Opens a `NOTIFY` text received on `channel`: authenticity, layout and freshness. `now_micros`
/// must be the monotonic reading of [`ReplayWindow::receive`], the only caller outside tests, so
/// key retirement and freshness never see the clock run backwards. Any `Err` drops the line
/// (counted by the caller); nothing is logged here.
fn open(
    ring: &RelayKeyRing,
    channel: &str,
    notify_text: &str,
    now_micros: u64,
) -> Result<RelayLine, RelayError> {
    if notify_text.len() > NOTIFY_MAX_BYTES {
        return Err(RelayError::Malformed);
    }
    let sealed = STANDARD
        .decode(notify_text)
        .map_err(|_| RelayError::Malformed)?;
    if sealed.len() < 1 + NONCE_BYTES + TAG_BYTES || sealed.len() > SEALED_MAX_BYTES {
        return Err(RelayError::Malformed);
    }
    let key_id = sealed[0];
    let key = ring.opening(key_id, now_micros)?;
    let nonce = Nonce::try_assume_unique_for_key(&sealed[1..=NONCE_BYTES])
        .map_err(|_| RelayError::Malformed)?;
    let aad = [channel.as_bytes(), &[key_id]].concat();
    let mut buffer = sealed[1 + NONCE_BYTES..].to_vec();
    let plaintext = aead_key(key)?
        .open_in_place(nonce, Aad::from(aad), &mut buffer)
        .map_err(|_| RelayError::Unauthentic)?;
    let line = RelayLine::decode(plaintext)?;
    if now_micros.abs_diff(line.sent_at_micros) > FRESHNESS_MICROS {
        return Err(RelayError::Stale);
    }
    Ok(line)
}

/// The bounded per-World de-duplication set (CHAT-0 §5): a `message_id` seen inside the freshness
/// window is refused, so a captured line cannot be sent again. An entry lives exactly until its
/// line's own freshness deadline (`sent_at + 30 s`), the last instant [`open`] still accepts it,
/// and expiry does not depend on arrival order.
#[derive(Debug, Default)]
pub(crate) struct ReplayWindow {
    /// The newest clock reading seen. Time never runs backwards here, so a database clock stepped
    /// back cannot make an evicted `message_id` admissible again.
    high_water: u64,
    expiry_of: HashMap<[u8; ID_BYTES], u64>,
    by_expiry: BTreeSet<(u64, [u8; ID_BYTES])>,
}

impl ReplayWindow {
    /// Receives one `NOTIFY` text: opens it and admits it once, on a clock that never runs
    /// backwards. `now_micros` is the node's database-time reading, raised to the highest reading
    /// seen, so a database clock stepped back can neither revive a retired key nor re-admit an
    /// evicted `message_id` or an expired line.
    pub(crate) fn receive(
        &mut self,
        ring: &RelayKeyRing,
        channel: &str,
        notify_text: &str,
        now_micros: u64,
    ) -> Result<RelayLine, RelayError> {
        let now_micros = self.observe(now_micros);
        let line = open(ring, channel, notify_text, now_micros)?;
        self.admit(&line, now_micros)?;
        Ok(line)
    }

    fn observe(&mut self, now_micros: u64) -> u64 {
        self.high_water = now_micros.max(self.high_water);
        self.high_water
    }

    /// Admits the line once. A line outside the freshness window is `Stale` here too, so nothing
    /// already expired is ever stored.
    fn admit(&mut self, line: &RelayLine, now_micros: u64) -> Result<(), RelayError> {
        let now_micros = self.observe(now_micros);
        while let Some(&(expiry, id)) = self.by_expiry.first() {
            if expiry >= now_micros {
                break;
            }
            self.by_expiry.remove(&(expiry, id));
            self.expiry_of.remove(&id);
        }
        if now_micros.abs_diff(line.sent_at_micros) > FRESHNESS_MICROS {
            return Err(RelayError::Stale);
        }
        if self.expiry_of.contains_key(&line.message_id) {
            return Err(RelayError::Replayed);
        }
        if self.expiry_of.len() >= REPLAY_WINDOW_LINES {
            return Err(RelayError::Saturated);
        }
        let expiry = line.sent_at_micros.saturating_add(FRESHNESS_MICROS);
        self.expiry_of.insert(line.message_id, expiry);
        self.by_expiry.insert((expiry, line.message_id));
        Ok(())
    }
}

#[cfg(test)]
mod tests;
