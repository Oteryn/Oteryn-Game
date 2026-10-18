//! The World chat relay of one node (CHAT-0 §5, CHAT-2B).
//!
//! Room and private lines cross the nodes of a World over one PostgreSQL `LISTEN`/`NOTIFY`
//! channel per World, `oteryn_chat_<hex>`. Each line is sealed with the World relay key ring
//! (`crate::chat::relay`) and sent with `pg_notify` in autocommit; every listener of the World
//! opens it, refuses a stale, replayed or unleased line and delivers it to its own sessions.
//! Delivery is at most once: the sender's `OK` means "sent", never "delivered".
//!
//! The relay runs on a small pool of its own, outside the WP3 durability root (the root is one
//! physical connection on its own runtime and carries durability work only). The pool takes the
//! root's verified endpoint and credentials. So that no sealed line reaches a server log, every
//! connection the pool opens pins statement and bind-parameter logging off with session `SET`s
//! ([`PIN_LOGGING_OFF`], CP decision D1400) before it carries anything. A session value outranks
//! the configuration file, so a later configuration reload cannot turn that logging back on for a
//! relay session. The relay's role needs `GRANT SET ON PARAMETER` on the settings that are not
//! user-settable (the provisioning note is in migration 0122). If any `SET` is refused, the relay refuses to start:
//! it is never ready, answers `CHAT_UNAVAILABLE`, and reports `chat_relay_disabled
//! reason=log_pin_refused` once. There is no fallback to operator-provisioned settings.
//! As a backstop the relay also reads the pinned values on every listener tick and in each
//! `pg_notify` statement. A node that is not listening, or whose notification queue is over half
//! full, refuses room and private lines the same way.
//!
//! Nothing here logs or stores chat text, sender or recipient (FND-02 §20): failures are counted
//! and reported as numbers only.
//!
//! VIP-2 (VIP0 §8.2, CP D1435): the same relay carries the presence hint (relay kind 3). A
//! session's committed admission queues `online`, its TERMINAL retirement `offline`; a death or
//! a reconnect inside session states 1-2 sends nothing. Hints wait in a bounded outbox coalesced
//! per character and are sent only while the relay is ready, so they stay inside `CHAT0-RL-09`;
//! a hint refused there is lost and corrected by the `VIP0-RL-09` reconcile. Every node,
//! the sender included, hands an admitted hint and its listener state to the installed
//! [`PresenceSink`], the VIP presence owner. `VIP0-RL-10` (hints per World per second) and the
//! relay part of `VIP0-RL-11` (hint latency, p99) are measured and reported as numbers.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use oteryn_foundation::CancellationToken;
use oteryn_protocol_oteryn::CharacterId;
use oteryn_protocol_oteryn::chat::ChatDisposition;
use sqlx::Executor as _;
use sqlx::Row as _;
use sqlx::postgres::{PgConnectOptions, PgListener, PgPool, PgPoolOptions};
use tokio::sync::Notify;

use super::chat_intent::ChatRuntime;
use super::{first, operator_event};
use crate::chat::ChatText;
use crate::chat::relay::{
    OutgoingLine, OutgoingPresence, PresenceHint, RelayDestination, RelayError, RelayKey,
    RelayKeyRing, Relayed, ReplayWindow, channel_name, seal, seal_presence,
};
use crate::domain::character_name::CharacterName;
use crate::foundation::GameSessionId;

/// The environment variable naming the relay key file: one key per line,
/// `<key_id> <activate_at_micros> <base64 of 32 bytes>`. Unset means no relay.
pub(crate) const KEYS_FILE_ENV: &str = "OTERYN_CHAT_RELAY_KEYS_FILE";
const KEYS_FILE_MAX_BYTES: usize = 4_096;
/// One connection listens; the others send.
const POOL_CONNECTIONS: u32 = 4;
const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(2);
const SEND_TIMEOUT: Duration = Duration::from_secs(2);
/// The listener's own work (clock, queue usage, lease) runs once per tick.
const TICK: Duration = Duration::from_secs(1);
const TICK_TIMEOUT: Duration = Duration::from_secs(2);
const RECONNECT_BACKOFF: Duration = Duration::from_secs(1);
/// Over half of the notification queue in use refuses chat (CHAT-0 §5).
const QUEUE_USAGE_MAX: f64 = 0.5;
const DROP_REPORT: Duration = Duration::from_secs(60);
/// VIP0 §8.2: presence hints waiting for the relay, one per character. A full outbox refuses a
/// new character's hint (counted); the `VIP0-RL-09` reconcile corrects what it missed.
pub(crate) const PRESENCE_OUTBOX_MAX: usize = 1_024;
/// Local sessions that publish presence; one per admitted session of this node.
pub(crate) const PRESENCE_PUBLISHERS_MAX: usize = 65_536;
/// Hint latency samples kept between two reports, for the `VIP0-RL-11` p99.
const PRESENCE_LATENCY_SAMPLES: usize = 1_024;

/// VIP0 §8.2: what consumes the presence side of the relay on this node (the VIP presence owner).
/// Every call is advisory input: the sink re-reads the durable rows before it shows anything.
pub(crate) trait PresenceSink: Send + Sync {
    /// An authentic, fresh, first-seen hint for `character` (its `online` value is advisory).
    fn hint(&self, character: CharacterId, online: bool);
    /// The listener became ready (`true`) or stopped being ready (`false`).
    fn listener(&self, up: bool);
    /// `session` left this node; its watcher state goes with it.
    fn session_ended(&self, session: GameSessionId);
}

/// The VIP presence owner consumes the relay. Its errors drop the input: the hint is advisory,
/// and a lost listener transition or watcher is corrected by its own reconcile and expiry.
impl PresenceSink for crate::durability::vip_presence::PresenceOwner {
    fn hint(&self, character: CharacterId, _online: bool) {
        if let Ok(target) = crate::domain::CharacterId::from_bytes(*character.as_bytes()) {
            let _ = self.target_hint(target);
        }
    }

    fn listener(&self, up: bool) {
        let _ = if up {
            self.listener_reconnected()
        } else {
            self.listener_down()
        };
    }

    fn session_ended(&self, session: GameSessionId) {
        let _ = self.forget(session);
    }
}

/// The bounded presence outbox, coalesced per character: a hint is advisory and the receiver
/// re-reads the row, so only the newest transition of a character needs to be sent.
#[derive(Default)]
struct PresenceOutbox {
    queue: VecDeque<(CharacterId, bool)>,
}

impl PresenceOutbox {
    /// `false` when the outbox is full and holds no hint for `character`.
    fn push(&mut self, character: CharacterId, online: bool) -> bool {
        if let Some(queued) = self.queue.iter_mut().find(|(id, _)| *id == character) {
            queued.1 = online;
            return true;
        }
        if self.queue.len() >= PRESENCE_OUTBOX_MAX {
            return false;
        }
        self.queue.push_back((character, online));
        true
    }

    fn pop(&mut self) -> Option<(CharacterId, bool)> {
        self.queue.pop_front()
    }
}

/// `VIP0-RL-10` and `VIP0-RL-11` measurement between two reports. Numbers only.
#[derive(Default)]
struct PresenceMeter {
    sent: AtomicU64,
    refused: AtomicU64,
    received: AtomicU64,
    /// The current one-second send window: its start and count, and the peak count.
    rate: Mutex<(Option<Instant>, u64, u64)>,
    /// Send-to-receive latency of admitted hints, database microseconds.
    latencies: Mutex<VecDeque<u64>>,
}

impl PresenceMeter {
    fn sent(&self) {
        self.sent.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut rate) = self.rate.lock() {
            let (start, count, peak) = &mut *rate;
            if start.is_none_or(|at| at.elapsed() >= Duration::from_secs(1)) {
                *start = Some(Instant::now());
                *count = 0;
            }
            *count += 1;
            *peak = (*peak).max(*count);
        }
    }

    fn refused(&self) {
        self.refused.fetch_add(1, Ordering::Relaxed);
    }

    fn received(&self, latency_micros: u64) {
        self.received.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut latencies) = self.latencies.lock() {
            if latencies.len() >= PRESENCE_LATENCY_SAMPLES {
                latencies.pop_front();
            }
            latencies.push_back(latency_micros);
        }
    }

    /// `(sent, refused, received, peak per second, p99 latency)` since the last take.
    fn take(&self) -> (u64, u64, u64, u64, u64) {
        let peak = self
            .rate
            .lock()
            .map_or(0, |mut rate| std::mem::take(&mut rate.2));
        let mut samples: Vec<u64> = self
            .latencies
            .lock()
            .map(|mut latencies| latencies.drain(..).collect())
            .unwrap_or_default();
        samples.sort_unstable();
        // Nearest rank: the smallest sample at or above 99% of them.
        let p99 = (samples.len() * 99)
            .div_ceil(100)
            .checked_sub(1)
            .and_then(|rank| samples.get(rank).copied())
            .unwrap_or(0);
        (
            self.sent.swap(0, Ordering::Relaxed),
            self.refused.swap(0, Ordering::Relaxed),
            self.received.swap(0, Ordering::Relaxed),
            peak,
            p99,
        )
    }
}

/// Why the relay is not configured. Fixed reasons only: no path, key or file content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RelayConfigError {
    File,
    Format,
    Ring,
    World,
    Endpoint,
}

impl std::fmt::Display for RelayConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::File => "key_file_unreadable",
            Self::Format => "key_file_malformed",
            Self::Ring => "key_ring_invalid",
            Self::World => "world_id_invalid",
            Self::Endpoint => "database_endpoint_unavailable",
        })
    }
}

/// Parses the relay key file.
pub(crate) fn parse_keys(content: &[u8]) -> Result<RelayKeyRing, RelayConfigError> {
    let text = std::str::from_utf8(content).map_err(|_| RelayConfigError::Format)?;
    let mut keys = Vec::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let mut fields = line.split_ascii_whitespace();
        let (Some(id), Some(activate), Some(key), None) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            return Err(RelayConfigError::Format);
        };
        let id = id.parse::<u8>().map_err(|_| RelayConfigError::Format)?;
        let activate = activate
            .parse::<u64>()
            .map_err(|_| RelayConfigError::Format)?;
        let key: [u8; 32] = STANDARD
            .decode(key)
            .ok()
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or(RelayConfigError::Format)?;
        keys.push(RelayKey::new(id, activate, key));
    }
    RelayKeyRing::new(keys).map_err(|_| RelayConfigError::Ring)
}

/// The canonical text of a UUID: lower-case hex with hyphens.
fn uuid_text(bytes: &[u8; 16]) -> String {
    let mut text = String::with_capacity(36);
    for (index, byte) in bytes.iter().enumerate() {
        if matches!(index, 4 | 6 | 8 | 10) {
            text.push('-');
        }
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

/// The relay of World `world` from the environment: `None` without a key file, or when the
/// configuration is unusable (reported once, by reason only).
pub(crate) fn configured(world: [u8; 16], options: Option<PgConnectOptions>) -> Option<ChatRelay> {
    let path = std::env::var_os(KEYS_FILE_ENV)?;
    let relay = crate::node::secure_file::read_checked(
        std::path::Path::new(&path),
        crate::node::secure_file::FileClass::Secret,
        crate::node::secure_file::effective_uid(),
        KEYS_FILE_MAX_BYTES,
    )
    .map_err(|_| RelayConfigError::File)
    .and_then(|content| parse_keys(&content))
    .and_then(|ring| ChatRelay::new(ring, world, options.ok_or(RelayConfigError::Endpoint)?));
    match relay {
        Ok(relay) => Some(relay),
        Err(error) => {
            operator_event(&format!("chat_relay_disabled reason={error}"));
            None
        }
    }
}

/// Pins statement and bind-parameter logging off for the rest of the session (CP decision
/// D1400). Sent as one simple query with no bind parameter, before the connection carries
/// anything. Each `SET` but the user-settable `log_parameter_max_length_on_error` needs
/// `GRANT SET ON PARAMETER` for the session's role.
const PIN_LOGGING_OFF: &str = "SET SESSION log_parameter_max_length = 0; \
     SET SESSION log_parameter_max_length_on_error = 0; \
     SET SESSION log_statement = 'none'; \
     SET SESSION log_min_duration_statement = -1";

/// True while the session holds the values [`PIN_LOGGING_OFF`] pins. Reading needs no privilege.
/// A macro so that the statements using it stay `&'static str`.
macro_rules! logging_pinned_off {
    () => {
        "current_setting('log_parameter_max_length') = '0' \
         AND current_setting('log_parameter_max_length_on_error') = '0' \
         AND current_setting('log_statement') = 'none' \
         AND current_setting('log_min_duration_statement') = '-1'"
    };
}

/// The pool of the relay: every connection it opens pins logging off first, and a refused pin
/// marks the relay `refused` for good.
fn relay_pool(options: PgConnectOptions, refused: Arc<AtomicBool>) -> PgPool {
    PgPoolOptions::new()
        .max_connections(POOL_CONNECTIONS)
        .min_connections(0)
        .acquire_timeout(ACQUIRE_TIMEOUT)
        .after_connect(move |connection, _meta| {
            let refused = Arc::clone(&refused);
            Box::pin(async move {
                match pin_logging_off(connection).await {
                    Ok(true) => Ok(()),
                    // The server refused a `SET`, or the session does not hold what was pinned:
                    // the relay refuses to start. Only a fixed reason leaves this function, never
                    // the server's error text.
                    Ok(false) | Err(sqlx::Error::Database(_)) => {
                        refused.store(true, Ordering::Release);
                        Err(sqlx::Error::Configuration(
                            "chat relay connection refused the logging pin".into(),
                        ))
                    }
                    Err(_) => Err(sqlx::Error::Configuration(
                        "chat relay connection lost while pinning logging".into(),
                    )),
                }
            })
        })
        .connect_lazy_with(options)
}

/// Pins logging off on `connection` and reads back that the session holds the pinned values.
/// `Err(Database)` when the server refused a `SET`.
async fn pin_logging_off(connection: &mut sqlx::PgConnection) -> Result<bool, sqlx::Error> {
    connection.execute(sqlx::raw_sql(PIN_LOGGING_OFF)).await?;
    logs_nothing(connection).await
}

/// Whether `connection` holds the values [`PIN_LOGGING_OFF`] pins.
async fn logs_nothing(connection: &mut sqlx::PgConnection) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(concat!("SELECT ", logging_pinned_off!()))
        .fetch_one(connection)
        .await
}

/// Sends `sealed` on `channel`. Every relay session pinned logging off when it connected, so
/// binding `sealed` cannot reach a server log; as a backstop the statement also notifies only
/// while the session still holds the pinned values. `false` when the backstop refused the line.
async fn notify_quietly<'c>(
    executor: impl sqlx::PgExecutor<'c>,
    channel: &str,
    sealed: &str,
) -> Result<bool, sqlx::Error> {
    let sent = sqlx::query(concat!(
        "SELECT pg_notify($1, $2) WHERE ",
        logging_pinned_off!()
    ))
    .bind(channel)
    .bind(sealed)
    .fetch_optional(executor)
    .await?;
    Ok(sent.is_some())
}

/// The answer to one send: only a notified line (one row) is sent. No row (the session does not
/// hold the logging pin), an error or a timeout is `ChatUnavailable`.
fn sent_or_unavailable<E>(
    notified: Result<Result<bool, sqlx::Error>, E>,
) -> Result<(), ChatDisposition> {
    match notified {
        Ok(Ok(true)) => Ok(()),
        _ => Err(ChatDisposition::ChatUnavailable),
    }
}

/// Why the listener stops and reconnects. Never carries a database error's text.
#[derive(Debug)]
struct Lost;

impl From<sqlx::Error> for Lost {
    fn from(_: sqlx::Error) -> Self {
        Self
    }
}

/// The World chat relay of this node.
pub(crate) struct ChatRelay {
    ring: RelayKeyRing,
    channel: String,
    world: [u8; 16],
    pool: PgPool,
    /// While the relay is ready (listening, the notification queue at most half full and the
    /// replay window admitting what it stamps): the clock a send stamps, microseconds, and when it
    /// was read. One value, so a send never pairs a ready state with a clock it would drop.
    clock: Mutex<Option<(u64, Instant)>>,
    /// Lines refused by the listener since the last report.
    dropped: AtomicU64,
    /// Set for good once a connection refused the logging pin: the relay never starts.
    refused: Arc<AtomicBool>,
    /// VIP-2: the presence owner fed by admitted hints and listener state.
    presence: OnceLock<Arc<dyn PresenceSink>>,
    /// The character of each local session that published `online`.
    publishers: Mutex<HashMap<GameSessionId, CharacterId>>,
    outbox: Mutex<PresenceOutbox>,
    outbox_ready: Notify,
    meter: PresenceMeter,
    /// Whether the sink was last told the listener is ready.
    presence_up: AtomicBool,
}

impl ChatRelay {
    pub(crate) fn new(
        ring: RelayKeyRing,
        world: [u8; 16],
        options: PgConnectOptions,
    ) -> Result<Self, RelayConfigError> {
        let channel = channel_name(&uuid_text(&world)).map_err(|_| RelayConfigError::World)?;
        let refused = Arc::new(AtomicBool::new(false));
        Ok(Self {
            ring,
            channel,
            world,
            pool: relay_pool(options, Arc::clone(&refused)),
            clock: Mutex::new(None),
            dropped: AtomicU64::new(0),
            refused,
            presence: OnceLock::new(),
            publishers: Mutex::new(HashMap::new()),
            outbox: Mutex::new(PresenceOutbox::default()),
            outbox_ready: Notify::new(),
            meter: PresenceMeter::default(),
            presence_up: AtomicBool::new(false),
        })
    }

    /// Installs the presence owner once; `false` when one is already installed.
    pub(crate) fn install_presence(&self, sink: Arc<dyn PresenceSink>) -> bool {
        self.presence.set(sink).is_ok()
    }

    /// VIP0 §8.2: `session` of `character` entered session state 1 here (its admission
    /// committed). Queues one `online` hint after that commit.
    pub(crate) fn presence_login(&self, session: GameSessionId, character: CharacterId) {
        let admitted = self.publishers.lock().is_ok_and(|mut publishers| {
            let room =
                publishers.len() < PRESENCE_PUBLISHERS_MAX || publishers.contains_key(&session);
            if room {
                publishers.insert(session, character);
            }
            room
        });
        if admitted {
            self.queue_presence(character, true);
        } else {
            self.meter.refused();
        }
    }

    /// VIP0 §8.2: `session` left session states 1-2 here (its TERMINAL commit retired it).
    /// Queues one `offline` hint for the character it published and forgets its watcher state.
    pub(crate) fn presence_logout(&self, session: GameSessionId) {
        let character = self
            .publishers
            .lock()
            .ok()
            .and_then(|mut publishers| publishers.remove(&session));
        if let Some(sink) = self.presence.get() {
            sink.session_ended(session);
        }
        if let Some(character) = character {
            self.queue_presence(character, false);
        }
    }

    fn queue_presence(&self, character: CharacterId, online: bool) {
        let queued = self
            .outbox
            .lock()
            .is_ok_and(|mut outbox| outbox.push(character, online));
        if queued {
            self.outbox_ready.notify_one();
        } else {
            self.meter.refused();
        }
    }

    /// The hints waiting in the outbox, oldest first, for the rules tests.
    #[cfg(test)]
    pub(crate) fn queued_presence(&self) -> Vec<(CharacterId, bool)> {
        self.outbox
            .lock()
            .map(|outbox| outbox.queue.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Sends the outbox until `shutdown`, then parks. A hint the relay cannot send now (not
    /// ready, the queue over `CHAT0-RL-09`, or the send failed) is counted and dropped.
    async fn run_presence(&self, shutdown: &CancellationToken) {
        loop {
            if first(self.outbox_ready.notified(), shutdown.cancelled())
                .await
                .is_none()
            {
                std::future::pending::<()>().await;
            }
            while let Some((character, online)) =
                self.outbox.lock().ok().and_then(|mut outbox| outbox.pop())
            {
                match self.send_presence(character, online).await {
                    Ok(()) => self.meter.sent(),
                    Err(_) => self.meter.refused(),
                }
            }
        }
    }

    /// Seals one presence hint and sends it to every listener of the World, this node included.
    async fn send_presence(
        &self,
        character: CharacterId,
        online: bool,
    ) -> Result<(), ChatDisposition> {
        let sent_at_micros = self.db_now().ok_or(ChatDisposition::ChatUnavailable)?;
        let sealed = seal_presence(
            &self.ring,
            &self.channel,
            OutgoingPresence {
                sent_at_micros,
                character,
                online,
            },
        )
        .map_err(|_| ChatDisposition::ChatUnavailable)?;
        let notify = notify_quietly(&self.pool, &self.channel, &sealed);
        sent_or_unavailable(tokio::time::timeout(SEND_TIMEOUT, notify).await)
    }

    /// Tells the sink of a change of the listener's readiness only.
    fn presence_listener(&self, up: bool) {
        if self.presence_up.swap(up, Ordering::AcqRel) != up
            && let Some(sink) = self.presence.get()
        {
            sink.listener(up);
        }
    }

    fn presence_hint(&self, hint: &PresenceHint, now: u64) {
        self.meter.received(now.saturating_sub(hint.sent_at_micros));
        if let Some(sink) = self.presence.get() {
            sink.hint(hint.character, hint.online);
        }
    }

    /// Whether the relay may send now.
    pub(crate) fn is_ready(&self) -> bool {
        self.db_now().is_some()
    }

    /// Whether a connection refused the logging pin, so the relay refuses to start.
    fn pin_refused(&self) -> bool {
        self.refused.load(Ordering::Acquire)
    }

    /// The clock a send stamps now, extrapolated from the last reading; `None` when not ready or
    /// when the relay refused to start.
    fn db_now(&self) -> Option<u64> {
        if self.pin_refused() {
            return None;
        }
        let (reading, at) = (*self.clock.lock().ok()?)?;
        let elapsed = u64::try_from(at.elapsed().as_micros()).ok()?;
        Some(reading.saturating_add(elapsed))
    }

    /// Marks the relay ready at database time `now`, for the rules tests.
    #[cfg(test)]
    pub(crate) fn force_ready(&self, now: u64) {
        if let Ok(mut clock) = self.clock.lock() {
            *clock = Some((now, Instant::now()));
        }
    }

    /// Seals one line and sends it to every listener of the World, this node included.
    /// `Rejected` for a line over the sealed bound; `ChatUnavailable` when the relay is not ready
    /// or the send fails.
    pub(crate) async fn send(
        &self,
        destination: RelayDestination,
        sender_name: CharacterName,
        text: ChatText,
    ) -> Result<(), ChatDisposition> {
        let sent_at_micros = self.db_now().ok_or(ChatDisposition::ChatUnavailable)?;
        let sealed = seal(
            &self.ring,
            &self.channel,
            OutgoingLine {
                sent_at_micros,
                destination,
                sender_name,
                text,
            },
        )
        .map_err(|error| match error {
            RelayError::Oversized => ChatDisposition::Rejected,
            _ => ChatDisposition::ChatUnavailable,
        })?;
        let notify = notify_quietly(&self.pool, &self.channel, &sealed);
        sent_or_unavailable(tokio::time::timeout(SEND_TIMEOUT, notify).await)
    }

    /// Listens until `shutdown`, reconnecting after a lost connection; then parks. One replay
    /// window lives across reconnects, so a reconnect cannot admit a line twice, and each
    /// reconnect moves its floor, so a line sent while the listener was down stays lost. A
    /// connection that refused the logging pin stops the relay for good: it closes its pool,
    /// reports once and parks without retrying.
    /// The presence outbox is sent alongside, on the same task.
    pub(crate) async fn run(&self, chat: &ChatRuntime, shutdown: &CancellationToken) {
        let _ = first(
            self.run_presence(shutdown),
            self.run_from(None, chat, shutdown),
        )
        .await;
    }

    /// [`Self::run`] from `window` (`None` reads the stored lease on the first tick).
    async fn run_from(
        &self,
        mut window: Option<ReplayWindow>,
        chat: &ChatRuntime,
        shutdown: &CancellationToken,
    ) {
        let mut reported = Instant::now();
        let mut reconnect = false;
        loop {
            let listened = first(
                self.listen(chat, &mut window, &mut reported, reconnect),
                shutdown.cancelled(),
            )
            .await;
            reconnect = true;
            self.not_ready();
            if self.pin_refused() {
                self.pool.close().await;
                operator_event("chat_relay_disabled reason=log_pin_refused");
                break;
            }
            if listened.is_none()
                || first(tokio::time::sleep(RECONNECT_BACKOFF), shutdown.cancelled())
                    .await
                    .is_none()
            {
                break;
            }
        }
        std::future::pending::<()>().await;
    }

    async fn listen(
        &self,
        chat: &ChatRuntime,
        window: &mut Option<ReplayWindow>,
        reported: &mut Instant,
        reconnect: bool,
    ) -> Lost {
        let mut listener = match PgListener::connect_with(&self.pool).await {
            Ok(listener) => listener,
            Err(_) => return Lost,
        };
        listener.eager_reconnect(false);
        if listener.listen(&self.channel).await.is_err() {
            return Lost;
        }
        let mut next_tick = Instant::now();
        let mut reconnect = reconnect;
        loop {
            if Instant::now() >= next_tick {
                let ticked = self.tick(&mut listener, window, reconnect);
                match tokio::time::timeout(TICK_TIMEOUT, ticked).await {
                    Ok(Ok(())) => {}
                    _ => return Lost,
                }
                reconnect = false;
                next_tick = Instant::now() + TICK;
                self.report_drops(reported);
            }
            let remaining = next_tick.saturating_duration_since(Instant::now());
            match tokio::time::timeout(remaining, listener.try_recv()).await {
                Err(_) => {}
                Ok(Ok(Some(notification))) => {
                    self.receive(chat, window, notification.channel(), notification.payload());
                }
                Ok(Ok(None) | Err(_)) => return Lost,
            }
        }
    }

    /// The listener's clock reading, its lease and the notification queue usage. The first tick
    /// after a `reconnect` moves the window's floor past the reading first; the relay is ready
    /// only once the reading reaches that floor.
    async fn tick(
        &self,
        listener: &mut PgListener,
        window: &mut Option<ReplayWindow>,
        reconnect: bool,
    ) -> Result<(), Lost> {
        // A pool connection that refused the logging pin stops the listener too.
        if self.pin_refused() {
            return Err(Lost);
        }
        let row = sqlx::query(concat!(
            "SELECT floor(extract(epoch FROM clock_timestamp()) * 1000000)::bigint AS now, \
             pg_notification_queue_usage() AS usage, ",
            logging_pinned_off!(),
            " AS quiet"
        ))
        .fetch_one(&mut *listener)
        .await?;
        let now = u64::try_from(row.try_get::<i64, _>("now")?).map_err(|_| Lost)?;
        let usage: f64 = row.try_get("usage")?;
        let quiet: bool = row.try_get("quiet")?;
        let read_at = Instant::now();
        if let Some(window) = window.as_mut().filter(|_| reconnect) {
            window.reconnected(now);
        } else if window.is_none() {
            let stored: Option<i64> = sqlx::query_scalar(
                "SELECT leased_until_micros FROM game_chat_relay_leases \
                 WHERE world_id = encode($1, 'hex')::uuid",
            )
            .bind(self.world.as_slice())
            .fetch_optional(&mut *listener)
            .await?;
            let stored = stored.map(u64::try_from).transpose().map_err(|_| Lost)?;
            *window = Some(ReplayWindow::resume(now, stored));
        }
        let window = window.as_mut().ok_or(Lost)?;
        if let Some(until) = window.lease_due(now) {
            sqlx::query(
                "INSERT INTO game_chat_relay_leases (world_id, leased_until_micros) \
                 VALUES (encode($1, 'hex')::uuid, $2) \
                 ON CONFLICT (world_id) DO UPDATE SET leased_until_micros = \
                 GREATEST(game_chat_relay_leases.leased_until_micros, \
                          EXCLUDED.leased_until_micros)",
            )
            .bind(self.world.as_slice())
            .bind(i64::try_from(until).map_err(|_| Lost)?)
            .execute(&mut *listener)
            .await?;
            window.lease_stored(until);
        }
        // Sends are stamped with the window's clock, and the relay is not ready until the window
        // admits what it stamps: after a start, a reconnect or a database clock stepped back a
        // send would otherwise return OK and be dropped here.
        // A listener session that no longer holds the logging pin makes the node not ready.
        let clock = window
            .admitting_clock(now)
            .filter(|_| quiet && usage <= QUEUE_USAGE_MAX);
        *self.clock.lock().map_err(|_| Lost)? = clock.map(|clock| (clock, read_at));
        // VIP0 §8.2: the presence owner shows UNCONFIRMED while the relay is not ready and
        // reconciles at once when it is ready again (`VIP0-RL-09`).
        self.presence_listener(clock.is_some());
        Ok(())
    }

    fn not_ready(&self) {
        if let Ok(mut clock) = self.clock.lock() {
            *clock = None;
        }
        self.presence_listener(false);
    }

    /// Opens one notification and delivers it; a refused line is only counted.
    fn receive(
        &self,
        chat: &ChatRuntime,
        window: &mut Option<ReplayWindow>,
        channel: &str,
        payload: &str,
    ) {
        if channel != self.channel {
            return;
        }
        let now = self.db_now();
        let opened = match (window.as_mut(), now) {
            (Some(window), Some(now)) => {
                window.receive_relayed(&self.ring, &self.channel, payload, now)
            }
            _ => Err(RelayError::Unleased),
        };
        match (opened, now) {
            (Ok(Relayed::Line(line)), _) => chat.deliver(&line),
            (Ok(Relayed::Presence(hint)), Some(now)) => self.presence_hint(&hint, now),
            _ => {
                self.dropped.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    fn report_drops(&self, reported: &mut Instant) {
        if reported.elapsed() < DROP_REPORT {
            return;
        }
        *reported = Instant::now();
        let dropped = self.dropped.swap(0, Ordering::Relaxed);
        if dropped > 0 {
            operator_event(&format!("chat_relay_dropped lines={dropped}"));
        }
        let (sent, refused, received, peak, p99) = self.meter.take();
        if sent + refused + received > 0 {
            operator_event(&format!(
                "vip_presence_relay sent={sent} refused={refused} received={received} \
                 peak_per_second={peak} hint_p99_micros={p99}"
            ));
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, reason = "test fixtures")]
mod tests;
