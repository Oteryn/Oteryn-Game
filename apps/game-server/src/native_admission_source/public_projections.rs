//! Public projections producer (`oteryn-game-public-projections-v1`): the
//! `highscores`, `character_profile` and `world_online` families and their
//! liveness watermarks (§3–§6, §8).
//!
//! The publisher pushes Game-owned public snapshots to Platform over its own
//! purpose-bound mTLS identity. Publication never affects gameplay, admission,
//! sessions or the change that caused it: every failure is only "not
//! delivered", and the undelivered outbox row (or the failed `world_online`
//! scan) holds the family's watermark back, so Platform's feed goes stale and
//! fails toward less disclosure.

use super::{
    SourceError, TransientCapacity,
    account_characters::{
        Delivery, EpochFence, FenceUnusable, WatermarkFacts, complete_through, decode_response,
        name_valid,
    },
    character_commands::source_authority_valid,
    descriptor::{Operation, ProducerDescriptor},
    http1_mtls,
    runtime_status::{NotDelivered, ReportClock, canonical_uuid, purpose_descriptor, refusal},
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    cmp::Ordering,
    collections::HashSet,
    future::Future,
    pin::{Pin, pin},
    sync::{
        Mutex,
        atomic::{self, AtomicU64},
    },
    task::Poll,
    time::Duration,
};

pub const HIGHSCORES_PATH: &str = "/internal/v1/game-public/highscores";
pub const PROFILE_PATH: &str = "/internal/v1/game-public/character-profile";
pub const ONLINE_PATH: &str = "/internal/v1/game-public/world-online";
pub const WATERMARK_PATH: &str = "/internal/v1/game-public/watermark";
/// `PUBPROJ-HS-REQUEST-BYTES`.
pub const HIGHSCORES_BYTES: usize = 262_144;
/// `PUBPROJ-PROFILE-REQUEST-BYTES`.
pub const PROFILE_BYTES: usize = 2_048;
/// `PUBPROJ-ONLINE-REQUEST-BYTES`.
pub const ONLINE_BYTES: usize = 524_288;
/// `PUBPROJ-WATERMARK-BYTES`.
pub const WATERMARK_BYTES: usize = 512;
/// `HIGHSCORES0-RL-02`: rows per (World, category, filter).
pub const MAX_HIGHSCORE_ROWS: usize = 1_000;
/// `PUBPROJ-ONLINE`: entries per World; a wire bound, never a truncation.
pub const MAX_ONLINE: usize = 4_096;
/// `PUBPROJ-ONLINE-WORLDS`: changed Worlds published concurrently.
pub const ONLINE_WORLDS: usize = 16;
/// `PUBPROJ-ONLINE-INTERVAL`.
pub const ONLINE_INTERVAL: Duration = Duration::from_secs(10);
/// `PUBPROJ-ONLINE-SCAN-DEADLINE`.
pub const ONLINE_SCAN_DEADLINE: Duration = Duration::from_secs(5);
/// A watermark is due this long after the previous attempt.
pub const WATERMARK_PERIOD: Duration = Duration::from_secs(3);
/// `PUBPROJ-WATERMARK-GAP`.
pub const MAX_WATERMARK_GAP: Duration = Duration::from_secs(10);
/// §6: a `400`, `401` or `409` holds the subject this long.
pub const DEFINITE_HOLD: Duration = Duration::from_secs(300);
/// §6: backoff after `429`, `503`, a timeout or a transport error.
pub const BACKOFF_MIN: Duration = Duration::from_secs(1);
pub const BACKOFF_MAX: Duration = Duration::from_secs(60);
/// Wait before retrying after an empty outbox or a failed step.
pub const IDLE: Duration = Duration::from_secs(1);
/// A store call that found the database connection busy is retried after this.
pub const STORE_RETRY: Duration = Duration::from_millis(100);
/// Held subjects passed to one outbox read; more are simply not skipped.
pub const MAX_HELD: usize = 1_024;
/// Highscores filters: 0 (all) and the five base vocations, numbered by
/// their wire vocation ids 1..5 (§4.1).
pub const FILTERS: usize = 6;
/// Compiled namespace of the public-projection descriptor; never a
/// configured source authority, so no other purpose's descriptor matches it.
pub const PURPOSE: &str = "OTERYN_GAME_PUBLIC_PROJECTIONS";

/// The families this producer publishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Highscores,
    CharacterProfile,
    WorldOnline,
}

impl Family {
    #[must_use]
    pub const fn wire(self) -> &'static str {
        match self {
            Self::Highscores => "highscores",
            Self::CharacterProfile => "character_profile",
            Self::WorldOnline => "world_online",
        }
    }
}

/// Native build-state vocation keys and their wire ids (§4.0): Game's
/// reference base-vocation enum (`ReferenceBaseVocation`: Druid 1, Knight 2,
/// Monk 3, Paladin 4, Sorcerer 5, None 6). A promoted vocation counts as its
/// base; `none` (also a Character without a build row) is 6. Any other key is
/// unclassifiable.
pub const VOCATIONS: [(&str, u8); 11] = [
    ("druid", 1),
    ("elder_druid", 1),
    ("knight", 2),
    ("elite_knight", 2),
    ("monk", 3),
    ("exalted_monk", 3),
    ("paladin", 4),
    ("royal_paladin", 4),
    ("sorcerer", 5),
    ("master_sorcerer", 5),
    ("none", 6),
];

/// The wire vocation ids (§4.0).
pub const VOCATION_IDS: std::ops::RangeInclusive<u8> = 1..=6;

#[must_use]
pub fn vocation_id(key: &str) -> Option<u8> {
    VOCATIONS.iter().find(|(k, _)| *k == key).map(|(_, id)| *id)
}

/// One highscores row (§4.1). `vocation` is `None` when the stored value is
/// unclassifiable; such a snapshot is refused, never altered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighscoreRow {
    pub rank: u64,
    pub character_id: String,
    pub name: String,
    pub vocation: Option<u8>,
    pub level: u64,
    pub value: u64,
}

/// One (World, category, filter) snapshot (§4.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighscoresSnapshot {
    pub world_id: String,
    pub category: u8,
    pub filter: u8,
    pub projection_epoch: u64,
    pub projection_revision: u64,
    pub source_observed_at: i64,
    /// Unix seconds.
    pub computed_at: i64,
    pub rows: Vec<HighscoreRow>,
}

/// One HIGHSCORES-0 snapshot of (World, category) at one revision: the six
/// filter lists, published as six snapshots with that revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighscoresBatch {
    pub world_id: String,
    pub category: u8,
    pub projection_epoch: u64,
    pub projection_revision: u64,
    pub source_observed_at: i64,
    pub computed_at: i64,
    pub lists: [Vec<HighscoreRow>; FILTERS],
}

impl HighscoresBatch {
    #[must_use]
    pub fn subject(&self) -> String {
        format!("{}:{}", self.world_id, self.category)
    }

    #[must_use]
    pub fn snapshot(&self, filter: u8) -> HighscoresSnapshot {
        HighscoresSnapshot {
            world_id: self.world_id.clone(),
            category: self.category,
            filter,
            projection_epoch: self.projection_epoch,
            projection_revision: self.projection_revision,
            source_observed_at: self.source_observed_at,
            computed_at: self.computed_at,
            rows: self
                .lists
                .get(usize::from(filter))
                .cloned()
                .unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileGuild {
    pub guild_id: String,
    pub rank_level: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileEntry {
    pub world_id: String,
    pub name: String,
    pub vocation: u8,
    pub level: u64,
    /// The Character's own guild-visibility setting (§4.2); `false` when it
    /// is unknown. The guild fields are sent either way.
    pub guild_visible: bool,
    pub guild: Option<ProfileGuild>,
}

/// One Character's profile (§4.2); `None` publishes `profile: []`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileSnapshot {
    pub character_id: String,
    pub projection_epoch: u64,
    pub projection_revision: u64,
    pub source_observed_at: i64,
    pub profile: Option<ProfileEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnlineEntry {
    pub name: String,
    pub vocation: u8,
    pub level: u64,
}

/// One World's online list (§4.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnlineSnapshot {
    pub world_id: String,
    pub projection_epoch: u64,
    pub projection_revision: u64,
    pub source_observed_at: i64,
    pub characters: Vec<OnlineEntry>,
}

#[derive(Serialize)]
struct HighscoreRowWire<'a> {
    rank: String,
    character_id: &'a str,
    name: &'a str,
    vocation: u8,
    level: String,
    value: String,
}

#[derive(Serialize)]
struct HighscoresWire<'a> {
    contract_version: u8,
    operation: &'static str,
    source_authority: &'a str,
    world_id: &'a str,
    category: u8,
    filter: u8,
    projection_epoch: String,
    projection_revision: String,
    source_observed_at: String,
    computed_at: String,
    rows: Vec<HighscoreRowWire<'a>>,
}

#[derive(Serialize)]
struct GuildWire<'a> {
    guild_id: &'a str,
    rank_level: u8,
}

#[derive(Serialize)]
struct ProfileEntryWire<'a> {
    world_id: &'a str,
    name: &'a str,
    vocation: u8,
    level: String,
    guild_visible: bool,
    guild: Vec<GuildWire<'a>>,
}

#[derive(Serialize)]
struct ProfileWire<'a> {
    contract_version: u8,
    operation: &'static str,
    source_authority: &'a str,
    character_id: &'a str,
    projection_epoch: String,
    projection_revision: String,
    source_observed_at: String,
    profile: Vec<ProfileEntryWire<'a>>,
}

#[derive(Serialize)]
struct OnlineEntryWire<'a> {
    name: &'a str,
    vocation: u8,
    level: String,
}

#[derive(Serialize)]
struct OnlineWire<'a> {
    contract_version: u8,
    operation: &'static str,
    source_authority: &'a str,
    world_id: &'a str,
    projection_epoch: String,
    projection_revision: String,
    source_observed_at: String,
    characters: &'a [OnlineEntryWire<'a>],
}

#[derive(Serialize)]
struct WatermarkWire<'a> {
    contract_version: u8,
    operation: &'static str,
    source_authority: &'a str,
    family: &'static str,
    projection_epoch: String,
    complete_through: String,
    observed_at: String,
}

fn envelope_valid(authority: &str, epoch: u64, revision: u64, observed: i64) -> bool {
    source_authority_valid(authority) && epoch != 0 && revision != 0 && observed >= 0
}

fn finish(body: Result<String, serde_json::Error>, bound: usize) -> Result<String, SourceError> {
    let body = body.map_err(|_| SourceError::InvalidInput)?;
    if body.len() > bound || body.contains('\\') {
        return Err(SourceError::CapacityExceeded);
    }
    Ok(body)
}

/// Exact §4.1 body of one filter. Refuses anything outside the contract grammar.
pub fn encode_highscores(
    source_authority: &str,
    snapshot: &HighscoresSnapshot,
) -> Result<String, SourceError> {
    let rows = &snapshot.rows;
    let mut seen = HashSet::with_capacity(rows.len());
    // Competition ranks (1, 2, 2, 4): each row shares the previous rank or
    // takes its own 1-based position.
    let ranks = rows.iter().enumerate().all(|(index, row)| {
        let position = u64::try_from(index + 1).unwrap_or(u64::MAX);
        row.rank == position || (index > 0 && row.rank == rows[index - 1].rank)
    });
    let valid = envelope_valid(
        source_authority,
        snapshot.projection_epoch,
        snapshot.projection_revision,
        snapshot.source_observed_at,
    ) && canonical_uuid(&snapshot.world_id, Some(b'7'))
        && (1..=11).contains(&snapshot.category)
        && usize::from(snapshot.filter) < FILTERS
        && snapshot.computed_at > 0
        && rows.len() <= MAX_HIGHSCORE_ROWS
        && ranks
        && rows.iter().all(|row| {
            canonical_uuid(&row.character_id, Some(b'7'))
                && seen.insert(row.character_id.as_str())
                && name_valid(&row.name)
                // A base-vocation list holds only that base vocation.
                && row.vocation.is_some_and(|id| {
                    VOCATION_IDS.contains(&id) && (snapshot.filter == 0 || id == snapshot.filter)
                })
                && row.level != 0
        });
    if !valid {
        return Err(SourceError::InvalidInput);
    }
    let rows = rows
        .iter()
        .map(|row| HighscoreRowWire {
            rank: row.rank.to_string(),
            character_id: &row.character_id,
            name: &row.name,
            vocation: row.vocation.unwrap_or_default(),
            level: row.level.to_string(),
            value: row.value.to_string(),
        })
        .collect();
    finish(
        serde_json::to_string(&HighscoresWire {
            contract_version: 1,
            operation: "PublishWorldHighscoresV1",
            source_authority,
            world_id: &snapshot.world_id,
            category: snapshot.category,
            filter: snapshot.filter,
            projection_epoch: snapshot.projection_epoch.to_string(),
            projection_revision: snapshot.projection_revision.to_string(),
            source_observed_at: snapshot.source_observed_at.to_string(),
            computed_at: snapshot.computed_at.to_string(),
            rows,
        }),
        HIGHSCORES_BYTES,
    )
}

/// Exact §4.2 body.
pub fn encode_profile(
    source_authority: &str,
    snapshot: &ProfileSnapshot,
) -> Result<String, SourceError> {
    let entry_valid = snapshot.profile.as_ref().is_none_or(|entry| {
        canonical_uuid(&entry.world_id, Some(b'7'))
            && name_valid(&entry.name)
            && entry.level != 0
            && VOCATION_IDS.contains(&entry.vocation)
            && entry.guild.as_ref().is_none_or(|guild| {
                canonical_uuid(&guild.guild_id, Some(b'7')) && (1..=20).contains(&guild.rank_level)
            })
    });
    if !envelope_valid(
        source_authority,
        snapshot.projection_epoch,
        snapshot.projection_revision,
        snapshot.source_observed_at,
    ) || !canonical_uuid(&snapshot.character_id, Some(b'7'))
        || !entry_valid
    {
        return Err(SourceError::InvalidInput);
    }
    let profile = snapshot
        .profile
        .iter()
        .map(|entry| ProfileEntryWire {
            world_id: &entry.world_id,
            name: &entry.name,
            vocation: entry.vocation,
            level: entry.level.to_string(),
            guild_visible: entry.guild_visible,
            guild: entry
                .guild
                .iter()
                .map(|guild| GuildWire {
                    guild_id: &guild.guild_id,
                    rank_level: guild.rank_level,
                })
                .collect(),
        })
        .collect();
    finish(
        serde_json::to_string(&ProfileWire {
            contract_version: 1,
            operation: "PublishCharacterProfileV1",
            source_authority,
            character_id: &snapshot.character_id,
            projection_epoch: snapshot.projection_epoch.to_string(),
            projection_revision: snapshot.projection_revision.to_string(),
            source_observed_at: snapshot.source_observed_at.to_string(),
            profile,
        }),
        PROFILE_BYTES,
    )
}

fn online_entries(characters: &[OnlineEntry]) -> Option<Vec<OnlineEntryWire<'_>>> {
    let valid = characters.len() <= MAX_ONLINE
        && characters
            .windows(2)
            .all(|w| w[0].name.as_bytes() < w[1].name.as_bytes())
        && characters
            .iter()
            .all(|c| name_valid(&c.name) && VOCATION_IDS.contains(&c.vocation) && c.level != 0);
    valid.then(|| {
        characters
            .iter()
            .map(|c| OnlineEntryWire {
                name: &c.name,
                vocation: c.vocation,
                level: c.level.to_string(),
            })
            .collect()
    })
}

/// SHA-256 of a World's `characters` member as sent: the content compared
/// between scans. `None` when the list is outside the contract.
#[must_use]
pub fn online_digest(characters: &[OnlineEntry]) -> Option<[u8; 32]> {
    let entries = online_entries(characters)?;
    let raw = serde_json::to_vec(&entries).ok()?;
    Some(Sha256::digest(raw).into())
}

/// Exact §4.6 body.
pub fn encode_online(
    source_authority: &str,
    snapshot: &OnlineSnapshot,
) -> Result<String, SourceError> {
    let entries = online_entries(&snapshot.characters).ok_or(SourceError::InvalidInput)?;
    if !envelope_valid(
        source_authority,
        snapshot.projection_epoch,
        snapshot.projection_revision,
        snapshot.source_observed_at,
    ) || !canonical_uuid(&snapshot.world_id, Some(b'7'))
    {
        return Err(SourceError::InvalidInput);
    }
    finish(
        serde_json::to_string(&OnlineWire {
            contract_version: 1,
            operation: "PublishWorldOnlineV1",
            source_authority,
            world_id: &snapshot.world_id,
            projection_epoch: snapshot.projection_epoch.to_string(),
            projection_revision: snapshot.projection_revision.to_string(),
            source_observed_at: snapshot.source_observed_at.to_string(),
            characters: &entries,
        }),
        ONLINE_BYTES,
    )
}

/// Exact §5.1 body. `complete_through` never passes `observed_at`.
pub fn encode_watermark(
    source_authority: &str,
    family: Family,
    projection_epoch: u64,
    complete_through: i64,
    observed_at: i64,
) -> Result<String, SourceError> {
    if !source_authority_valid(source_authority)
        || projection_epoch == 0
        || complete_through < 0
        || complete_through > observed_at
    {
        return Err(SourceError::InvalidInput);
    }
    finish(
        serde_json::to_string(&WatermarkWire {
            contract_version: 1,
            operation: "PublishPublicProjectionWatermarkV1",
            source_authority,
            family: family.wire(),
            projection_epoch: projection_epoch.to_string(),
            complete_through: complete_through.to_string(),
            observed_at: observed_at.to_string(),
        }),
        WATERMARK_BYTES,
    )
}

/// Public-projection client identity (§3): its own certificate and key,
/// never those of the LCFA projection, evidence, status or command purposes.
pub struct PublicProjectionDescriptor(ProducerDescriptor);

impl PublicProjectionDescriptor {
    /// `other_purposes` are the certificate chains of every other Platform
    /// identity this host holds; none may share the public-projection key.
    pub fn new(
        connect_endpoint: (String, u16),
        peer_name: String,
        roots: Vec<CertificateDer<'static>>,
        client_chain: Vec<CertificateDer<'static>>,
        client_key: PrivateKeyDer<'static>,
        other_purposes: &[&[CertificateDer<'static>]],
    ) -> Result<Self, SourceError> {
        purpose_descriptor(
            PURPOSE,
            connect_endpoint,
            peer_name,
            roots,
            client_chain,
            client_key,
            other_purposes,
        )
        .map(Self)
    }
}

/// Where publications go: Platform over mTLS, or a test sink. Sends may run
/// concurrently (one per family, and one per World for `world_online`).
pub trait PublicSink: Send + Sync {
    fn send(
        &self,
        operation: Operation,
        body: &str,
    ) -> impl Future<Output = Result<Delivery, NotDelivered>> + Send;
}

/// The production sink: each exchange on its own connection and capacity;
/// the publisher bounds how many run at once (`PUBPROJ-INFLIGHT`).
pub struct MtlsPublicSink {
    descriptor: PublicProjectionDescriptor,
}

impl MtlsPublicSink {
    #[must_use]
    pub const fn new(descriptor: PublicProjectionDescriptor) -> Self {
        Self { descriptor }
    }
}

impl PublicSink for MtlsPublicSink {
    async fn send(&self, operation: Operation, body: &str) -> Result<Delivery, NotDelivered> {
        let capacity = TransientCapacity::new();
        let mut permit = capacity
            .try_queue()
            .map_err(|_| NotDelivered::Unavailable)?;
        permit
            .try_activate()
            .map_err(|_| NotDelivered::Unavailable)?;
        let (status, raw) =
            http1_mtls::exchange_with_status(&self.descriptor.0, operation, body, &mut permit)
                .await
                .map_err(|_| NotDelivered::Unavailable)?;
        if status == 200 {
            return decode_response(&raw).map_err(|_| NotDelivered::InvalidResponse);
        }
        Err(refusal(status, &raw))
    }
}

/// The outbox watermark facts of both outbox families, read in one statement
/// on the database clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutboxFacts {
    pub projection_epoch: u64,
    /// `created_at` (Unix ms) of the oldest undelivered profile row.
    pub profile_oldest_ms: Option<i64>,
    /// `None` while the HIGHSCORES-0 source tables do not exist: then no
    /// highscores watermark is sent and the family stays stale. Otherwise
    /// the oldest undelivered highscores row, if any.
    pub highscores: Option<Option<i64>>,
    pub now_ms: i64,
}

/// The publisher's stored `world_online` snapshot of one World.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnlineState {
    pub world_id: String,
    pub projection_epoch: u64,
    pub projection_revision: u64,
    pub characters_digest: [u8; 32],
    pub source_observed_at: i64,
    pub acked: bool,
}

/// One World at the scan's cut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnlineWorld {
    pub world_id: String,
    /// `None` when more characters qualify than `PUBPROJ-ONLINE`.
    pub characters: Option<Vec<OnlineEntry>>,
    pub state: Option<OnlineState>,
}

/// One scan's read: the epoch, the cut time (database clock, Unix ms) and
/// every World's qualifying online list at that cut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnlineCut {
    pub projection_epoch: u64,
    pub cut_ms: i64,
    pub worlds: Vec<OnlineWorld>,
}

/// The Game side of the public projections (§5). Every call is one database
/// transaction; an `Err` is "not now".
pub trait PublicStore: Send + Sync {
    fn outbox_facts(&self) -> impl Future<Output = Result<OutboxFacts, ()>> + Send;
    /// The current profile of the Character with the oldest undelivered
    /// change, skipping `held` Characters and any new holder of a name whose
    /// vacating publication is still undelivered (§7.3).
    fn next_profile(
        &self,
        held: &[String],
    ) -> impl Future<Output = Result<Option<ProfileSnapshot>, ()>> + Send;
    fn clear_profile(
        &self,
        character_id: &str,
        projection_epoch: u64,
        projection_revision: u64,
    ) -> impl Future<Output = Result<(), ()>> + Send;
    /// The current HIGHSCORES-0 snapshot of the (World, category) with the
    /// oldest undelivered revision, skipping `held` subjects.
    fn next_highscores(
        &self,
        held: &[String],
    ) -> impl Future<Output = Result<Option<HighscoresBatch>, ()>> + Send;
    fn clear_highscores(
        &self,
        world_id: &str,
        category: u8,
        projection_epoch: u64,
        projection_revision: u64,
    ) -> impl Future<Output = Result<(), ()>> + Send;
    fn online_cut(&self) -> impl Future<Output = Result<OnlineCut, ()>> + Send;
    /// Store new revisions, unacknowledged, before anything of them is sent.
    fn persist_online(&self, states: &[OnlineState])
    -> impl Future<Output = Result<(), ()>> + Send;
    /// Mark exactly these (World, epoch, revision) snapshots acknowledged.
    fn ack_online(&self, states: &[OnlineState]) -> impl Future<Output = Result<(), ()>> + Send;
}

/// The last scan that completed in time: its epoch, cut and when it was read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoodCut {
    pub projection_epoch: u64,
    pub cut_ms: i64,
    /// The publisher clock when the cut read returned (never before the cut).
    pub read_at: Duration,
}

/// A subject whose last attempt got `400`, `401`, `409` or was unencodable.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Held {
    family: Family,
    subject: String,
    until: Duration,
}

/// The outbox loop's state: watermark timing, per-family backoff, held
/// subjects and the acknowledged filters of the highscores batch in progress.
#[derive(Debug, Default)]
pub struct OutboxLoop {
    pub watermark_started: Duration,
    pub watermark_due: Duration,
    backoff: [Duration; 2],
    retry_at: [Duration; 2],
    held: Vec<Held>,
    turn: usize,
    highscores_source: bool,
    progress: Option<(String, u64, u64, u8)>,
}

/// The scan loop's state: Worlds held after a definite refusal.
#[derive(Debug, Default)]
pub struct ScanLoop {
    held: Vec<Held>,
}

impl OutboxLoop {
    fn held(&self, family: Family, now: Duration) -> Vec<String> {
        self.held
            .iter()
            .filter(|h| h.family == family && h.until > now)
            .take(MAX_HELD)
            .map(|h| h.subject.clone())
            .collect()
    }
}

enum Step {
    Idle,
    Delivered,
    Failed,
}

const fn definite(not_delivered: NotDelivered) -> bool {
    matches!(
        not_delivered,
        NotDelivered::InvalidReport
            | NotDelivered::Malformed
            | NotDelivered::Unauthenticated
            | NotDelivered::Conflict
    )
}

fn class(result: &Result<Delivery, NotDelivered>) -> &'static str {
    match result {
        Ok(Delivery::Accepted) => "accepted",
        Ok(Delivery::Superseded) => "superseded",
        Err(not_delivered) => not_delivered.class(),
    }
}

/// `work`, or `None` once `budget` has passed on `clock`.
async fn within<C: ReportClock, T>(
    clock: &C,
    budget: Duration,
    work: impl Future<Output = T>,
) -> Option<T> {
    let mut work = pin!(work);
    let mut expiry = pin!(clock.sleep(budget));
    std::future::poll_fn(|cx| match work.as_mut().poll(cx) {
        Poll::Ready(output) => Poll::Ready(Some(output)),
        Poll::Pending => expiry.as_mut().poll(cx).map(|()| None),
    })
    .await
}

/// Both outputs, polling the two futures concurrently on one task.
async fn join2<A: Future, B: Future>(a: A, b: B) -> (A::Output, B::Output) {
    let mut a = pin!(a);
    let mut b = pin!(b);
    let (mut first, mut second) = (None, None);
    std::future::poll_fn(|cx| {
        if first.is_none()
            && let Poll::Ready(output) = a.as_mut().poll(cx)
        {
            first = Some(output);
        }
        if second.is_none()
            && let Poll::Ready(output) = b.as_mut().poll(cx)
        {
            second = Some(output);
        }
        match (first.take(), second.take()) {
            (Some(x), Some(y)) => Poll::Ready((x, y)),
            (x, y) => {
                first = x;
                second = y;
                Poll::Pending
            }
        }
    })
    .await
}

/// Every output in order, polling the futures concurrently on one task.
async fn join_all<F: Future>(futures: Vec<F>) -> Vec<F::Output> {
    let mut futures: Vec<Pin<Box<F>>> = futures.into_iter().map(Box::pin).collect();
    let mut outputs: Vec<Option<F::Output>> = futures.iter().map(|_| None).collect();
    std::future::poll_fn(|cx| {
        let mut pending = false;
        for (future, output) in futures.iter_mut().zip(outputs.iter_mut()) {
            if output.is_none() {
                match future.as_mut().poll(cx) {
                    Poll::Ready(value) => *output = Some(value),
                    Poll::Pending => pending = true,
                }
            }
        }
        if pending {
            Poll::Pending
        } else {
            Poll::Ready(outputs.iter_mut().filter_map(Option::take).collect())
        }
    })
    .await
}

/// One publisher per Game authority store.
pub struct Publisher<C, S, K, F> {
    pub clock: C,
    pub store: S,
    pub sink: K,
    pub fence: Mutex<F>,
    pub source_authority: String,
    /// Maximum Character transaction duration (§5.1); it also bounds each
    /// store call, since every store call is one such transaction.
    pub max_transaction: Duration,
    /// §6 operator alarm: definite refusals (`400`, `401`, `409`, unencodable).
    pub alarms: AtomicU64,
    pub last_cut: Mutex<Option<GoodCut>>,
}

fn log(family: Family, operation: &str, class: &str, elapsed: Duration) {
    // §7.4: family, operation, result class and timing only; never a name,
    // an id or a body.
    eprintln!(
        "oteryn-game-server event=public_projections family={} operation={operation} result={class} elapsed_ms={}",
        family.wire(),
        elapsed.as_millis()
    );
}

fn seconds(ms: i64) -> i64 {
    ms.div_euclid(1000)
}

impl<C: ReportClock, S: PublicStore, K: PublicSink, F: EpochFence> Publisher<C, S, K, F> {
    pub fn new(
        clock: C,
        store: S,
        sink: K,
        fence: F,
        source_authority: String,
        max_transaction: Duration,
    ) -> Self {
        Self {
            clock,
            store,
            sink,
            fence: Mutex::new(fence),
            source_authority,
            max_transaction,
            alarms: AtomicU64::new(0),
            last_cut: Mutex::new(None),
        }
    }

    /// §5 epoch fence `F_pub` for one publication of `epoch`: refused while
    /// the fence is unusable or above the epoch (a restored store not yet
    /// raised); a higher epoch first persists `F_pub := epoch`.
    fn admit(&self, epoch: u64) -> Result<(), &'static str> {
        let Ok(mut fence) = self.fence.lock() else {
            return Err("fence_invalid");
        };
        let current = fence.read().map_err(|FenceUnusable| "fence_invalid")?;
        match epoch.cmp(&current) {
            Ordering::Less => Err("fence_below"),
            Ordering::Equal => Ok(()),
            Ordering::Greater => fence
                .persist(epoch)
                .map_err(|FenceUnusable| "fence_unwritable"),
        }
    }

    /// `call` until it succeeds or the next retry would start after `until`:
    /// a busy or re-establishing database connection is only "not now".
    async fn retrying<T, R: Future<Output = Result<T, ()>>>(
        &self,
        until: Duration,
        mut call: impl FnMut() -> R,
    ) -> Result<T, ()> {
        loop {
            let result = call().await;
            if result.is_ok() || self.clock.elapsed() + STORE_RETRY >= until {
                return result;
            }
            self.clock.sleep(STORE_RETRY).await;
        }
    }

    fn alarm(&self) -> u64 {
        self.alarms.fetch_add(1, atomic::Ordering::Relaxed) + 1
    }

    /// Record one outbox delivery result: a definite refusal holds the
    /// subject for `DEFINITE_HOLD` and raises the alarm; a transient one
    /// backs the family off; a delivery resets its backoff.
    fn settle(
        &self,
        state: &mut OutboxLoop,
        family: Family,
        subject: &str,
        result: &Result<Delivery, NotDelivered>,
    ) {
        let lane = usize::from(family == Family::Highscores);
        let now = self.clock.elapsed();
        match result {
            Ok(_) => state.backoff[lane] = Duration::ZERO,
            Err(not_delivered) if definite(*not_delivered) => {
                let alarms = self.alarm();
                eprintln!(
                    "oteryn-game-server event=public_projections family={} result=held alarms={alarms}",
                    family.wire()
                );
                state
                    .held
                    .retain(|h| !(h.family == family && h.subject == subject));
                state.held.push(Held {
                    family,
                    subject: subject.to_owned(),
                    until: now + DEFINITE_HOLD,
                });
            }
            Err(_) => {
                let backoff = (state.backoff[lane] * 2).clamp(BACKOFF_MIN, BACKOFF_MAX);
                state.backoff[lane] = backoff;
                state.retry_at[lane] = now + backoff;
            }
        }
    }

    /// One send of `operation` within `budget`; a cut exchange is not
    /// delivered, and its retry carries the same body.
    async fn deliver(
        &self,
        operation: Operation,
        body: &str,
        budget: Duration,
    ) -> Result<Delivery, NotDelivered> {
        if budget.is_zero() {
            return Err(NotDelivered::Unavailable);
        }
        within(&self.clock, budget, self.sink.send(operation, body))
            .await
            .unwrap_or(Err(NotDelivered::Unavailable))
    }

    /// The `character_profile` and `highscores` watermarks, sent concurrently
    /// within `MAX_WATERMARK_GAP` of `started`.
    async fn outbox_watermarks(&self, state: &mut OutboxLoop, started: Duration) {
        const OPERATION: &str = "PublishPublicProjectionWatermarkV1";
        let facts = match self.store.outbox_facts().await {
            Ok(facts) => facts,
            Err(()) => {
                let elapsed = self.clock.elapsed().saturating_sub(started);
                log(Family::CharacterProfile, OPERATION, "unavailable", elapsed);
                log(Family::Highscores, OPERATION, "unavailable", elapsed);
                return;
            }
        };
        state.highscores_source = facts.highscores.is_some();
        if let Err(refused) = self.admit(facts.projection_epoch) {
            log(Family::CharacterProfile, OPERATION, refused, Duration::ZERO);
            log(Family::Highscores, OPERATION, refused, Duration::ZERO);
            return;
        }
        let body = |oldest: Option<i64>| {
            let through = complete_through(
                &WatermarkFacts {
                    projection_epoch: facts.projection_epoch,
                    oldest_undelivered_ms: oldest,
                    now_ms: facts.now_ms,
                },
                self.max_transaction,
            )?;
            Some((through, seconds(facts.now_ms)))
        };
        let send = |family: Family, through: Option<(i64, i64)>| async move {
            let result = match through.map(|(through, observed)| {
                encode_watermark(
                    &self.source_authority,
                    family,
                    facts.projection_epoch,
                    through,
                    observed,
                )
            }) {
                None | Some(Err(_)) => Err(NotDelivered::InvalidReport),
                Some(Ok(body)) => {
                    let budget = (started + MAX_WATERMARK_GAP).saturating_sub(self.clock.elapsed());
                    self.deliver(Operation::PublishPublicProjectionWatermarkV1, &body, budget)
                        .await
                }
            };
            log(
                family,
                OPERATION,
                class(&result),
                self.clock.elapsed().saturating_sub(started),
            );
        };
        let profile = send(Family::CharacterProfile, body(facts.profile_oldest_ms));
        match facts.highscores {
            Some(oldest) => {
                join2(profile, send(Family::Highscores, body(oldest))).await;
            }
            None => profile.await,
        }
    }

    async fn profile_step(&self, state: &mut OutboxLoop, phase_end: Duration) -> Step {
        const OPERATION: &str = "PublishCharacterProfileV1";
        let held = state.held(Family::CharacterProfile, self.clock.elapsed());
        let Ok(Some(snapshot)) = self.store.next_profile(&held).await else {
            return Step::Idle;
        };
        let started = self.clock.elapsed();
        let budget = phase_end.saturating_sub(started + self.max_transaction);
        let result = match encode_profile(&self.source_authority, &snapshot) {
            Err(_) => Err(NotDelivered::InvalidReport),
            Ok(body) => match self.admit(snapshot.projection_epoch) {
                // A fenced snapshot stays queued.
                Err(refused) => {
                    log(Family::CharacterProfile, OPERATION, refused, Duration::ZERO);
                    return Step::Failed;
                }
                Ok(()) => {
                    self.deliver(Operation::PublishCharacterProfileV1, &body, budget)
                        .await
                }
            },
        };
        log(
            Family::CharacterProfile,
            OPERATION,
            class(&result),
            self.clock.elapsed().saturating_sub(started),
        );
        self.settle(
            state,
            Family::CharacterProfile,
            &snapshot.character_id,
            &result,
        );
        if result.is_ok()
            && self
                .retrying(phase_end, || {
                    self.store.clear_profile(
                        &snapshot.character_id,
                        snapshot.projection_epoch,
                        snapshot.projection_revision,
                    )
                })
                .await
                .is_ok()
        {
            Step::Delivered
        } else {
            Step::Failed
        }
    }

    /// The six filters of one revision; each acknowledged filter is kept, so
    /// a later step sends only the rest. The outbox row clears once all six
    /// are acknowledged.
    async fn highscores_step(&self, state: &mut OutboxLoop, phase_end: Duration) -> Step {
        const OPERATION: &str = "PublishWorldHighscoresV1";
        let held = state.held(Family::Highscores, self.clock.elapsed());
        let Ok(Some(batch)) = self.store.next_highscores(&held).await else {
            return Step::Idle;
        };
        let subject = batch.subject();
        let mut acked = match &state.progress {
            Some((key, epoch, revision, mask))
                if *key == subject
                    && *epoch == batch.projection_epoch
                    && *revision == batch.projection_revision =>
            {
                *mask
            }
            _ => 0,
        };
        if let Err(refused) = self.admit(batch.projection_epoch) {
            log(Family::Highscores, OPERATION, refused, Duration::ZERO);
            return Step::Failed;
        }
        // All six lists are encoded before any is sent: one list outside the
        // contract refuses the whole revision, so no filter shows a list of a
        // snapshot that was never published whole.
        let bodies: Result<Vec<String>, SourceError> = (0..FILTERS)
            .map(|filter| {
                let filter = u8::try_from(filter).unwrap_or_default();
                encode_highscores(&self.source_authority, &batch.snapshot(filter))
            })
            .collect();
        let Ok(bodies) = bodies else {
            let result = Err(NotDelivered::InvalidReport);
            log(
                Family::Highscores,
                OPERATION,
                class(&result),
                Duration::ZERO,
            );
            self.settle(state, Family::Highscores, &subject, &result);
            return Step::Failed;
        };
        for (filter, body) in bodies.iter().enumerate() {
            let bit = 1u8 << filter;
            if acked & bit != 0 {
                continue;
            }
            let started = self.clock.elapsed();
            let budget = phase_end.saturating_sub(started + self.max_transaction);
            if budget.is_zero() {
                break;
            }
            let result = self
                .deliver(Operation::PublishWorldHighscoresV1, body, budget)
                .await;
            log(
                Family::Highscores,
                OPERATION,
                class(&result),
                self.clock.elapsed().saturating_sub(started),
            );
            self.settle(state, Family::Highscores, &subject, &result);
            if result.is_err() {
                break;
            }
            acked |= bit;
        }
        let all = (1u8 << FILTERS) - 1;
        if acked == all
            && self
                .retrying(phase_end, || {
                    self.store.clear_highscores(
                        &batch.world_id,
                        batch.category,
                        batch.projection_epoch,
                        batch.projection_revision,
                    )
                })
                .await
                .is_ok()
        {
            state.progress = None;
            return Step::Delivered;
        }
        state.progress = Some((
            subject,
            batch.projection_epoch,
            batch.projection_revision,
            acked,
        ));
        Step::Failed
    }

    /// The outbox watermarks when due, then at most one subject of one
    /// family (round robin over the families not backing off). Returns
    /// whether a subject was acknowledged and cleared.
    ///
    /// As in LCFA, a watermark attempt and the publication phase after it end
    /// by `MAX_WATERMARK_GAP` after that attempt started, so the next
    /// watermark attempt is never later than that.
    pub async fn outbox_step(&self, state: &mut OutboxLoop) -> bool {
        if self.clock.elapsed() >= state.watermark_due {
            let started = self.clock.elapsed();
            state.watermark_started = started;
            state.watermark_due = started + WATERMARK_PERIOD;
            self.outbox_watermarks(state, started).await;
        }
        let phase_end = state.watermark_started + MAX_WATERMARK_GAP;
        let now = self.clock.elapsed();
        state.held.retain(|h| h.until > now);
        for offset in 0..2 {
            let lane = (state.turn + offset) % 2;
            if self.clock.elapsed() < state.retry_at[lane]
                || (lane == 1 && !state.highscores_source)
            {
                continue;
            }
            if self.clock.elapsed() + self.max_transaction.saturating_mul(2) >= phase_end {
                return false;
            }
            state.turn = (lane + 1) % 2;
            let step = if lane == 0 {
                self.profile_step(state, phase_end).await
            } else {
                self.highscores_step(state, phase_end).await
            };
            match step {
                Step::Idle => {}
                Step::Delivered => return true,
                Step::Failed => return false,
            }
        }
        false
    }

    /// One `world_online` scan (§5.1). Returns whether it completed in time:
    /// no World over its bound, and every changed World acknowledged within
    /// `ONLINE_SCAN_DEADLINE` of the scan's start. Only then does its cut
    /// become the watermark's `complete_through`.
    pub async fn scan(&self, state: &mut ScanLoop) -> bool {
        const OPERATION: &str = "PublishWorldOnlineV1";
        let started = self.clock.elapsed();
        let deadline = started + ONLINE_SCAN_DEADLINE;
        // The cut is retried while the connection is busy, leaving the rest of
        // the scan at least `max_transaction` for its sends.
        let cut_until = deadline.saturating_sub(self.max_transaction);
        let Ok(cut) = self.retrying(cut_until, || self.store.online_cut()).await else {
            log(
                Family::WorldOnline,
                OPERATION,
                "unavailable",
                Duration::ZERO,
            );
            return false;
        };
        let read_at = self.clock.elapsed();
        if let Err(refused) = self.admit(cut.projection_epoch) {
            log(Family::WorldOnline, OPERATION, refused, Duration::ZERO);
            return false;
        }
        state.held.retain(|h| h.until > read_at);
        let observed = seconds(cut.cut_ms);
        let mut complete = observed >= 0;
        let mut persist = Vec::new();
        let mut pending = Vec::new();
        for world in cut.worlds {
            let Some(characters) = world.characters else {
                // §4.6: never truncated; the World and the family go stale.
                log(Family::WorldOnline, OPERATION, "over_bound", Duration::ZERO);
                complete = false;
                continue;
            };
            let Some(digest) = online_digest(&characters) else {
                self.alarm();
                log(
                    Family::WorldOnline,
                    OPERATION,
                    "invalid_report",
                    Duration::ZERO,
                );
                complete = false;
                continue;
            };
            let next = match &world.state {
                Some(s)
                    if s.projection_epoch == cut.projection_epoch
                        && s.characters_digest == digest =>
                {
                    if s.acked {
                        continue;
                    }
                    Some((s.projection_revision, s.source_observed_at, false))
                }
                Some(s) if s.projection_epoch > cut.projection_epoch => None,
                Some(s) => s
                    .projection_revision
                    .checked_add(1)
                    .map(|revision| (revision, observed, true)),
                None => Some((1, observed, true)),
            };
            let Some((revision, source_observed_at, changed)) = next else {
                complete = false;
                continue;
            };
            if state.held.iter().any(|h| h.subject == world.world_id) {
                complete = false;
                continue;
            }
            let snapshot = OnlineSnapshot {
                world_id: world.world_id.clone(),
                projection_epoch: cut.projection_epoch,
                projection_revision: revision,
                source_observed_at,
                characters,
            };
            let Ok(body) = encode_online(&self.source_authority, &snapshot) else {
                self.alarm();
                log(
                    Family::WorldOnline,
                    OPERATION,
                    "invalid_report",
                    Duration::ZERO,
                );
                complete = false;
                continue;
            };
            let stored = OnlineState {
                world_id: world.world_id,
                projection_epoch: cut.projection_epoch,
                projection_revision: revision,
                characters_digest: digest,
                source_observed_at,
                acked: false,
            };
            if changed {
                persist.push(stored.clone());
            }
            pending.push((stored, body));
        }
        // A revision is stored before it is sent, so it never names two contents.
        if !persist.is_empty()
            && self
                .retrying(deadline, || self.store.persist_online(&persist))
                .await
                .is_err()
        {
            log(
                Family::WorldOnline,
                OPERATION,
                "unavailable",
                Duration::ZERO,
            );
            return false;
        }
        let mut acked = Vec::new();
        for wave in pending.chunks(ONLINE_WORLDS) {
            let sends = wave.iter().map(|(stored, body)| async move {
                let begun = self.clock.elapsed();
                let result = self
                    .deliver(
                        Operation::PublishWorldOnlineV1,
                        body,
                        deadline.saturating_sub(begun),
                    )
                    .await;
                log(
                    Family::WorldOnline,
                    OPERATION,
                    class(&result),
                    self.clock.elapsed().saturating_sub(begun),
                );
                (stored, result)
            });
            for (stored, result) in join_all(sends.collect()).await {
                match result {
                    Ok(_) => acked.push(OnlineState {
                        acked: true,
                        ..stored.clone()
                    }),
                    Err(not_delivered) => {
                        complete = false;
                        if definite(not_delivered) {
                            let alarms = self.alarm();
                            eprintln!(
                                "oteryn-game-server event=public_projections family=world_online result=held alarms={alarms}"
                            );
                            state.held.push(Held {
                                family: Family::WorldOnline,
                                subject: stored.world_id.clone(),
                                until: self.clock.elapsed() + DEFINITE_HOLD,
                            });
                        }
                    }
                }
            }
        }
        // A scan acknowledged after its deadline counts as failed (§5.1).
        if self.clock.elapsed() > deadline {
            complete = false;
        }
        // A lost acknowledgement record only resends the same pair next scan.
        if !acked.is_empty() {
            let _ = self
                .retrying(deadline + ONLINE_SCAN_DEADLINE, || {
                    self.store.ack_online(&acked)
                })
                .await;
        }
        if complete && let Ok(mut last) = self.last_cut.lock() {
            *last = Some(GoodCut {
                projection_epoch: cut.projection_epoch,
                cut_ms: cut.cut_ms,
                read_at,
            });
        }
        complete
    }

    /// The `world_online` watermark (§5.1): `complete_through` is the cut of
    /// the last scan that completed in time; `observed_at` is that cut on the
    /// database clock advanced by the publisher's monotonic clock since the
    /// cut was read, so it never runs ahead of the database clock. Nothing is
    /// sent before the first scan completes.
    pub async fn online_watermark(&self) {
        const OPERATION: &str = "PublishPublicProjectionWatermarkV1";
        let started = self.clock.elapsed();
        let last = self.last_cut.lock().ok().and_then(|last| *last);
        let Some(cut) = last else {
            log(Family::WorldOnline, OPERATION, "no_scan", Duration::ZERO);
            return;
        };
        let since =
            i64::try_from(started.saturating_sub(cut.read_at).as_millis()).unwrap_or(i64::MAX);
        let observed = seconds(cut.cut_ms.saturating_add(since));
        let result = match encode_watermark(
            &self.source_authority,
            Family::WorldOnline,
            cut.projection_epoch,
            seconds(cut.cut_ms),
            observed,
        ) {
            Err(_) => Err(NotDelivered::InvalidReport),
            Ok(body) => match self.admit(cut.projection_epoch) {
                Err(refused) => {
                    log(Family::WorldOnline, OPERATION, refused, Duration::ZERO);
                    return;
                }
                Ok(()) => {
                    self.deliver(
                        Operation::PublishPublicProjectionWatermarkV1,
                        &body,
                        MAX_WATERMARK_GAP,
                    )
                    .await
                }
            },
        };
        log(
            Family::WorldOnline,
            OPERATION,
            class(&result),
            self.clock.elapsed().saturating_sub(started),
        );
    }

    async fn outbox_loop(&self) {
        let mut state = OutboxLoop::default();
        loop {
            if !self.outbox_step(&mut state).await {
                let wait = state
                    .watermark_due
                    .saturating_sub(self.clock.elapsed())
                    .min(IDLE);
                self.clock.sleep(wait).await;
            }
        }
    }

    async fn scan_loop(&self) {
        let mut state = ScanLoop::default();
        loop {
            let started = self.clock.elapsed();
            self.scan(&mut state).await;
            let next = (started + ONLINE_INTERVAL).saturating_sub(self.clock.elapsed());
            self.clock.sleep(next).await;
        }
    }

    async fn online_watermark_loop(&self) {
        loop {
            let started = self.clock.elapsed();
            self.online_watermark().await;
            let next = (started + WATERMARK_PERIOD).saturating_sub(self.clock.elapsed());
            self.clock.sleep(next).await;
        }
    }

    /// Publish until the task is dropped: the outbox families, the
    /// `world_online` scan and its watermark run concurrently on one task.
    pub async fn run(&self) {
        join2(
            self.outbox_loop(),
            join2(self.scan_loop(), self.online_watermark_loop()),
        )
        .await;
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[path = "public_projections_tests.rs"]
mod tests;
