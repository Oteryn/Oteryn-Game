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
//! root's verified endpoint and credentials. Each of its connections sets
//! `log_parameter_max_length_on_error = 0` and requires `log_parameter_max_length = 0`, so no
//! sealed line reaches a server log; a node whose database does not assert both is not ready and
//! answers `CHAT_UNAVAILABLE`. A configuration reload can change the setting of an open session,
//! so the listener checks it every tick and each send checks it on its own connection first. A node that is not listening, or whose notification queue is over
//! half full, refuses room and private lines the same way.
//!
//! Nothing here logs or stores chat text, sender or recipient (FND-02 §20): failures are counted
//! and reported as numbers only.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use oteryn_foundation::CancellationToken;
use oteryn_protocol_oteryn::chat::ChatDisposition;
use sqlx::Executor as _;
use sqlx::Row as _;
use sqlx::postgres::{PgConnectOptions, PgListener, PgPool, PgPoolOptions};

use super::chat_intent::ChatRuntime;
use super::{first, operator_event};
use crate::chat::ChatText;
use crate::chat::relay::{
    OutgoingLine, RelayDestination, RelayError, RelayKey, RelayKeyRing, ReplayWindow, channel_name,
    seal,
};
use crate::domain::character_name::CharacterName;

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

/// The pool of the relay: each connection must assert that the server logs no bind parameter.
fn relay_pool(options: PgConnectOptions) -> PgPool {
    PgPoolOptions::new()
        .max_connections(POOL_CONNECTIONS)
        .min_connections(0)
        .acquire_timeout(ACQUIRE_TIMEOUT)
        .after_connect(|connection, _meta| {
            Box::pin(async move {
                connection
                    .execute("SET log_parameter_max_length_on_error = 0")
                    .await?;
                if !logs_no_parameters(connection).await? {
                    return Err(sqlx::Error::Configuration(
                        "chat relay connection may log bind parameters".into(),
                    ));
                }
                Ok(())
            })
        })
        .connect_lazy_with(options)
}

/// Whether `connection` logs no bind parameter now. A configuration reload reaches a session after
/// its current command and can raise `log_parameter_max_length`, so a check at connect alone is not
/// enough: the listener repeats it every tick and each send repeats it on its own connection.
const LOGS_NO_PARAMETERS: &str = "SELECT current_setting('log_parameter_max_length') = '0' \
     AND current_setting('log_parameter_max_length_on_error') = '0'";

async fn logs_no_parameters(connection: &mut sqlx::PgConnection) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(LOGS_NO_PARAMETERS)
        .fetch_one(connection)
        .await
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
}

impl ChatRelay {
    pub(crate) fn new(
        ring: RelayKeyRing,
        world: [u8; 16],
        options: PgConnectOptions,
    ) -> Result<Self, RelayConfigError> {
        let channel = channel_name(&uuid_text(&world)).map_err(|_| RelayConfigError::World)?;
        Ok(Self {
            ring,
            channel,
            world,
            pool: relay_pool(options),
            clock: Mutex::new(None),
            dropped: AtomicU64::new(0),
        })
    }

    /// Whether the relay may send now.
    pub(crate) fn is_ready(&self) -> bool {
        self.db_now().is_some()
    }

    /// The clock a send stamps now, extrapolated from the last reading; `None` when not ready.
    fn db_now(&self) -> Option<u64> {
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
        // The precondition is checked again on the connection that sends, right before the line.
        let notify = async {
            let mut connection = self.pool.acquire().await?;
            if !logs_no_parameters(&mut connection).await? {
                return Ok(false);
            }
            sqlx::query("SELECT pg_notify($1, $2)")
                .bind(&self.channel)
                .bind(&sealed)
                .execute(&mut *connection)
                .await
                .map(|_| true)
        };
        match tokio::time::timeout(SEND_TIMEOUT, notify).await {
            Ok(Ok(true)) => Ok(()),
            _ => Err(ChatDisposition::ChatUnavailable),
        }
    }

    /// Listens until `shutdown`, reconnecting after a lost connection; then parks. One replay
    /// window lives across reconnects, so a reconnect cannot admit a line twice, and each
    /// reconnect moves its floor, so a line sent while the listener was down stays lost.
    pub(crate) async fn run(&self, chat: &ChatRuntime, shutdown: &CancellationToken) {
        self.run_from(None, chat, shutdown).await;
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
        let row = sqlx::query(
            "SELECT floor(extract(epoch FROM clock_timestamp()) * 1000000)::bigint AS now, \
             pg_notification_queue_usage() AS usage, \
             current_setting('log_parameter_max_length') = '0' \
             AND current_setting('log_parameter_max_length_on_error') = '0' AS quiet",
        )
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
        // A reload that lets the server log bind parameters makes the node not ready on this tick.
        let clock = window
            .admitting_clock(now)
            .filter(|_| quiet && usage <= QUEUE_USAGE_MAX);
        *self.clock.lock().map_err(|_| Lost)? = clock.map(|clock| (clock, read_at));
        Ok(())
    }

    fn not_ready(&self) {
        if let Ok(mut clock) = self.clock.lock() {
            *clock = None;
        }
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
        let opened = match (window.as_mut(), self.db_now()) {
            (Some(window), Some(now)) => window.receive(&self.ring, &self.channel, payload, now),
            _ => Err(RelayError::Unleased),
        };
        match opened {
            Ok(line) => chat.deliver(&line),
            Err(_) => {
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
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, reason = "test fixtures")]
mod tests;
