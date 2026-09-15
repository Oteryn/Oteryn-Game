use sqlx::postgres::{BudgetError, PgConnectOptions, PgPoolOptions, PgSslMode, ResourceBudget};
use std::error::Error;
use std::net::IpAddr;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const SLOT_LIMIT: usize = 4_194_304;
const ROOT_LIMIT: usize = 12_582_912;
const ROOT_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const HOLDER_IDLE_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const HOLDER_MAX_LIFETIME: Duration = Duration::from_secs(30 * 60);
const TRANSPORT_DENIAL_MARKER: &str = "OTERYN_WP3_CONNECTION_OWNER_DENIAL_UNAVAILABLE";
const DENIAL_MARKER: &str = "OTERYN_WP3_TLS_DENIAL_UNAVAILABLE";
const POSITIVE_MARKER: &str = "OTERYN_WP3_PG17_TLS13_VERIFY_FULL_POSITIVE";
const ORDINARY_CONTROL_MARKER: &str = "OTERYN_WP3_ORDINARY_PROFILE_CONTROL_POSITIVE";
const SELECTED_PROFILE_MARKER: &str = "OTERYN_WP3_SELECTED_ROOT_PROFILE_COMPONENTS";
// Frozen source-derived selected connection-generation enclosure: the configured
// built-in-root verifier enclosure (4_475_170) plus fixed TLS state (27_485).
// Provider residency is charged separately through try_reserve_provider_shared.
const SELECTED_CONNECTION_GENERATION_ALLOWANCE: usize = 4_475_170 + 27_485;
const TRACE_LIMIT: usize = 64;

#[derive(Clone, Copy, Default)]
struct Snapshot {
    ordinary: usize,
    root: usize,
    peak_ordinary: usize,
    peak_root: usize,
    provider_shared: usize,
    denied_bytes: usize,
    denied_ordinary: usize,
    denied_root: usize,
    denied_tls_phase: bool,
}

#[derive(Clone, Copy)]
struct Event {
    op: char,
    bytes: usize,
    ordinary: usize,
    root: usize,
}

#[derive(Default)]
struct State {
    ordinary: usize,
    root: usize,
    peak_ordinary: usize,
    peak_root: usize,
    provider_shared: usize,
    denied_bytes: usize,
    denied_ordinary: usize,
    denied_root: usize,
    denied_tls_phase: bool,
    events: Vec<Event>,
}

impl State {
    fn record(&mut self, op: char, bytes: usize) {
        if self.events.len() == TRACE_LIMIT {
            self.events.remove(0);
        }
        self.events.push(Event {
            op,
            bytes,
            ordinary: self.ordinary,
            root: self.root,
        });
    }

    fn record_denial(&mut self, bytes: usize) {
        self.denied_bytes = bytes;
        self.denied_ordinary = self.ordinary;
        self.denied_root = self.root;
    }
}

struct Ledger {
    ordinary_limit: usize,
    root_limit: usize,
    state: Mutex<State>,
    deny_after_provider: AtomicBool,
}

impl Ledger {
    fn new(ordinary_limit: usize, root_limit: usize) -> Self {
        Self {
            ordinary_limit,
            root_limit,
            state: Mutex::new(State::default()),
            deny_after_provider: AtomicBool::new(false),
        }
    }

    fn snapshot(&self) -> Snapshot {
        let state = self.state.lock().unwrap();
        Snapshot {
            ordinary: state.ordinary,
            root: state.root,
            peak_ordinary: state.peak_ordinary,
            peak_root: state.peak_root,
            provider_shared: state.provider_shared,
            denied_bytes: state.denied_bytes,
            denied_ordinary: state.denied_ordinary,
            denied_root: state.denied_root,
            denied_tls_phase: state.denied_tls_phase,
        }
    }

    fn trace(&self) -> String {
        let state = self.state.lock().unwrap();
        state
            .events
            .iter()
            .map(|event| {
                format!(
                    "{}:{}@ordinary={},root={}",
                    event.op, event.bytes, event.ordinary, event.root
                )
            })
            .collect::<Vec<_>>()
            .join(";")
    }
}

impl ResourceBudget for Ledger {
    fn try_reserve(&self, bytes: usize) -> Result<(), BudgetError> {
        let Ok(mut state) = self.state.lock() else {
            return Err(BudgetError::Unavailable);
        };
        state.record('R', bytes);
        // The observed provider-shared reservation is the TLS phase boundary.
        // Fund the same bounded transport prefix as the positive profile, then
        // deny its next ordinary TLS allocation, independently of byte sizes.
        if self.deny_after_provider.load(Ordering::Acquire) && state.provider_shared != 0 {
            state.denied_tls_phase = true;
            state.record_denial(bytes);
            return Err(BudgetError::Unavailable);
        }
        let Some(ordinary) = state.ordinary.checked_add(bytes) else {
            state.record_denial(bytes);
            return Err(BudgetError::Overflow);
        };
        let Some(root) = state.root.checked_add(bytes) else {
            state.record_denial(bytes);
            return Err(BudgetError::Overflow);
        };
        if ordinary > self.ordinary_limit || root > self.root_limit {
            state.record_denial(bytes);
            return Err(BudgetError::Unavailable);
        }
        state.ordinary = ordinary;
        state.root = root;
        state.peak_ordinary = state.peak_ordinary.max(ordinary);
        state.peak_root = state.peak_root.max(root);
        Ok(())
    }

    fn release(&self, bytes: usize) {
        let Ok(mut state) = self.state.lock() else {
            std::process::abort();
        };
        state.record('L', bytes);
        let Some(ordinary) = state.ordinary.checked_sub(bytes) else {
            std::process::abort();
        };
        let Some(root) = state.root.checked_sub(bytes) else {
            std::process::abort();
        };
        state.ordinary = ordinary;
        state.root = root;
    }

    fn try_reserve_provider_shared(&self, bytes: usize) -> Result<(), BudgetError> {
        let Ok(mut state) = self.state.lock() else {
            return Err(BudgetError::Unavailable);
        };
        state.record('P', bytes);
        let Some(root) = state.root.checked_add(bytes) else {
            state.record_denial(bytes);
            return Err(BudgetError::Overflow);
        };
        if root > self.root_limit {
            state.record_denial(bytes);
            return Err(BudgetError::Unavailable);
        }
        state.root = root;
        state.provider_shared = state
            .provider_shared
            .checked_add(bytes)
            .ok_or(BudgetError::Overflow)?;
        state.peak_root = state.peak_root.max(root);
        Ok(())
    }
}

fn ordinary_options(admin_url: &str, ca_path: &Path) -> Result<PgConnectOptions, Box<dyn Error>> {
    let options: PgConnectOptions = admin_url.parse()?;
    Ok(options
        .host("localhost")
        .ssl_mode(PgSslMode::VerifyFull)
        .ssl_root_cert(ca_path))
}

fn selected_options(
    admin_url: &str,
    ca_path: &Path,
    owner: Arc<dyn ResourceBudget>,
) -> Result<sqlx::postgres::OterynRootProfile, Box<dyn Error>> {
    let parsed: PgConnectOptions = admin_url.parse()?;
    let transport_ip: IpAddr = parsed
        .get_host()
        .parse()
        .map_err(|_| "selected qualification requires a literal-IP PostgreSQL transport address")?;
    let authority = admin_url
        .split_once("://")
        .and_then(|(_, rest)| rest.split_once('@').map(|(userinfo, _)| userinfo))
        .ok_or("selected qualification requires explicit URL credentials")?;
    let (username, password) = authority
        .split_once(':')
        .ok_or("selected qualification requires an explicit password")?;
    if username != parsed.get_username() || username.contains('%') || password.contains('%') {
        return Err("selected qualification requires literal configured test credentials".into());
    }
    let database = parsed
        .get_database()
        .ok_or("selected qualification requires an explicit database")?;
    let root_ca_pem = std::fs::read(ca_path)?;
    Ok(PgConnectOptions::new_oteryn_root_profile(
        transport_ip,
        parsed.get_port(),
        "localhost",
        database,
        username,
        password,
        &root_ca_pem,
        owner,
    )?)
}

fn selected_holder_pool(options: sqlx::postgres::OterynRootProfile) -> sqlx::PgPool {
    PgPoolOptions::new()
        .max_connections(1)
        .min_connections(0)
        .acquire_timeout(ROOT_CONNECT_TIMEOUT)
        .idle_timeout(HOLDER_IDLE_TIMEOUT)
        .max_lifetime(HOLDER_MAX_LIFETIME)
        .connect_lazy_with(options.into_connect_options())
}

fn tls_denial_ledger() -> Ledger {
    Ledger::new(SLOT_LIMIT, ROOT_LIMIT)
}

fn prime_provider_residency(
    runtime: &tokio::runtime::Runtime,
    admin_url: &str,
    ca_path: &Path,
    owner: Arc<dyn ResourceBudget>,
) -> Result<(), Box<dyn Error>> {
    runtime.block_on(async {
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .min_connections(0)
            .acquire_timeout(ROOT_CONNECT_TIMEOUT)
            .connect_lazy_with(
                ordinary_options(admin_url, ca_path)?.with_resource_budget(owner),
            );
        let mut connection = pool.acquire().await?;
        connection.return_to_pool().await;
        pool.close().await;
        Ok::<_, Box<dyn Error>>(())
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let mode = std::env::var("OTERYN_WP3_MODE")?;
    let admin_url = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL")?;
    let ca_path = std::env::var("OTERYN_WP3_CA_CERT")?;
    let ca_path = Path::new(&ca_path);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    match mode.as_str() {
        "deny" | "deny-transport" => {
            let tls_phase = mode == "deny";
            // The selected profile itself owns precharged retained backing. For
            // transport denial, fund exactly that backing and leave no capacity
            // for the first connection-owner allocation; derive the amount by
            // constructing and destroying the real selected profile rather than
            // hard-coding a compensating allowance.
            let transport_profile_bytes = if tls_phase {
                0
            } else {
                let sizing = Arc::new(Ledger::new(usize::MAX, usize::MAX));
                let sizing_owner: Arc<dyn ResourceBudget> = sizing.clone();
                drop(selected_options(&admin_url, ca_path, sizing_owner)?);
                let snapshot = sizing.snapshot();
                if snapshot.ordinary == 0 || snapshot.ordinary != snapshot.root {
                    return Err("selected profile sizing did not isolate retained backing".into());
                }
                snapshot.ordinary
            };
            let ledger = Arc::new(if tls_phase {
                tls_denial_ledger()
            } else {
                Ledger::new(transport_profile_bytes, transport_profile_bytes)
            });
            let owner: Arc<dyn ResourceBudget> = ledger.clone();
            let options = selected_options(&admin_url, ca_path, owner.clone())?;
            if tls_phase {
                prime_provider_residency(&runtime, &admin_url, ca_path, owner.clone())?;
                ledger.deny_after_provider.store(true, Ordering::Release);
            }
            let pool = runtime.block_on(async { selected_holder_pool(options) });
            if runtime.block_on(pool.try_begin())?.is_some() {
                return Err(
                    "lazy empty holder unexpectedly produced a ready-only transaction".into(),
                );
            }
            let error = match runtime.block_on(pool.acquire()) {
                Ok(mut connection) => {
                    runtime.block_on(connection.return_to_pool());
                    runtime.block_on(pool.close());
                    return Err("underfunded holder unexpectedly established TLS".into());
                }
                Err(error) => error,
            };
            match &error {
                sqlx::Error::Io(source)
                    if !tls_phase && source.kind() == std::io::ErrorKind::OutOfMemory => {}
                sqlx::Error::Tls(source) if tls_phase => {
                    let Some(cause) = source.downcast_ref::<BudgetError>() else {
                        return Err(format!(
                            "resource denial lost canonical BudgetError source: {source}"
                        )
                        .into());
                    };
                    if cause != &BudgetError::Unavailable {
                        return Err(format!("unexpected budget denial cause: {cause:?}").into());
                    }
                }
                other => {
                    return Err(format!(
                        "{mode} resource denial escaped through unexpected holder surface: {other}"
                    )
                    .into());
                }
            }
            runtime.block_on(pool.close());
            drop(pool);
            drop(runtime);
            let snapshot = ledger.snapshot();
            if snapshot.denied_bytes == 0 {
                return Err("holder denial did not reach the caller-supplied root ledger".into());
            }
            if tls_phase {
                if !snapshot.denied_tls_phase
                    || snapshot.provider_shared == 0
                    || snapshot.peak_ordinary == 0
                    || snapshot.peak_ordinary > SLOT_LIMIT
                    || snapshot.peak_root > ROOT_LIMIT
                    || snapshot.ordinary != 0
                    || snapshot.root != snapshot.provider_shared
                {
                    return Err(
                        "TLS denial lacked its phase witness or violated final custody".into(),
                    );
                }
                println!(
                    "{DENIAL_MARKER} denied_bytes={} provider_shared={} retained_root={}",
                    snapshot.denied_bytes, snapshot.provider_shared, snapshot.root
                );
            } else {
                if snapshot.ordinary != 0
                    || snapshot.root != 0
                    || snapshot.peak_ordinary != transport_profile_bytes
                    || snapshot.peak_root != transport_profile_bytes
                    || snapshot.provider_shared != 0
                    || snapshot.denied_tls_phase
                {
                    return Err("denied connection owner retained or acquired resource debt".into());
                }
                println!("{TRANSPORT_DENIAL_MARKER}");
            }
        }
        "positive" | "ordinary-control" => {
            let selected = mode == "positive";
            let ordinary_limit = if selected {
                SELECTED_CONNECTION_GENERATION_ALLOWANCE
            } else {
                SLOT_LIMIT
            };
            let ledger = Arc::new(Ledger::new(ordinary_limit, ROOT_LIMIT));
            let owner: Arc<dyn ResourceBudget> = ledger.clone();
            let selected_options = if selected {
                let options = selected_options(&admin_url, ca_path, owner.clone())?;
                prime_provider_residency(&runtime, &admin_url, ca_path, owner.clone())?;
                Some(options)
            } else {
                None
            };
            let pool = runtime.block_on(async {
                Ok::<_, Box<dyn Error>>(if selected {
                    selected_holder_pool(selected_options.ok_or("selected profile missing")?)
                } else {
                    PgPoolOptions::new()
                        .max_connections(1)
                        .min_connections(0)
                        .acquire_timeout(ROOT_CONNECT_TIMEOUT)
                        .idle_timeout(HOLDER_IDLE_TIMEOUT)
                        .max_lifetime(HOLDER_MAX_LIFETIME)
                        .connect_lazy_with(
                            ordinary_options(&admin_url, ca_path)?
                                .with_resource_budget(owner.clone()),
                        )
                })
            })?;

            if runtime.block_on(pool.try_begin())?.is_some() {
                return Err(
                    "lazy empty holder unexpectedly manufactured an active connection".into(),
                );
            }

            let mut root_connection = runtime.block_on(async {
                match tokio::time::timeout(ROOT_CONNECT_TIMEOUT, pool.acquire()).await {
                    Ok(Ok(connection)) => Ok::<_, Box<dyn Error>>(connection),
                    Ok(Err(error)) => {
                        let snapshot = ledger.snapshot();
                        let trace = ledger.trace();
                        Err(format!(
                            "funded root-maintenance acquire failed: {error}; denied_bytes={}, denied_at_ordinary={}, denied_at_root={}, peak_ordinary={}, peak_root={}, provider_shared={}, trace=[{}]",
                            snapshot.denied_bytes,
                            snapshot.denied_ordinary,
                            snapshot.denied_root,
                            snapshot.peak_ordinary,
                            snapshot.peak_root,
                            snapshot.provider_shared,
                            trace
                        )
                        .into())
                    }
                    Err(_) => Err("funded root-maintenance acquire exceeded five seconds".into()),
                }
            })?;
            runtime.block_on(root_connection.return_to_pool());
            // `PoolConnection::return_to_pool()` removes the live connection,
            // but the emptied wrapper still owns a `Pool` clone. Destroy that
            // clone before closing the holder so the selected connect options
            // (and their profile reservation) can reach their real final drop.
            drop(root_connection);

            let mut transaction = runtime.block_on(async {
                for _ in 0..100 {
                    match pool.try_begin().await {
                        Ok(Some(transaction)) => {
                            return Ok::<_, Box<dyn Error>>(transaction);
                        }
                        Ok(None) => tokio::time::sleep(Duration::from_millis(10)).await,
                        Err(error) => return Err(Box::new(error)),
                    }
                }
                Err("established holder never became ready for try_begin()".into())
            })?;

            let (server_version_num, ssl_enabled, ssl_version) = runtime.block_on(async {
                let server_version_num: String = sqlx::query_scalar("SHOW server_version_num")
                    .fetch_one(&mut *transaction)
                    .await?;
                let (ssl_enabled, ssl_version): (bool, Option<String>) = sqlx::query_as(
                    "SELECT ssl, version FROM pg_stat_ssl WHERE pid = pg_backend_pid()",
                )
                .fetch_one(&mut *transaction)
                .await?;
                Ok::<_, sqlx::Error>((server_version_num, ssl_enabled, ssl_version))
            })?;
            runtime.block_on(transaction.rollback())?;
            if server_version_num != "170006" {
                return Err(format!(
                    "qualification reached wrong PostgreSQL version: {server_version_num}"
                )
                .into());
            }
            if !ssl_enabled || ssl_version.as_deref() != Some("TLSv1.3") {
                return Err(format!(
                    "qualification did not negotiate PostgreSQL TLS1.3: ssl={ssl_enabled}, version={ssl_version:?}"
                )
                .into());
            }

            runtime.block_on(pool.close());
            drop(pool);
            drop(runtime);

            let snapshot = ledger.snapshot();
            if snapshot.peak_ordinary == 0 || snapshot.peak_ordinary > ordinary_limit {
                return Err("TLS custody exceeded its selected profile allowance".into());
            }
            if snapshot.provider_shared == 0 || snapshot.peak_root > ROOT_LIMIT {
                return Err(
                    "AWS-LC provider residency was not charged to the same root ledger".into(),
                );
            }
            if snapshot.ordinary != 0 || snapshot.root != snapshot.provider_shared {
                return Err(format!(
                    "holder teardown left invalid custody: ordinary={}, root={}, provider_shared={}",
                    snapshot.ordinary, snapshot.root, snapshot.provider_shared
                )
                .into());
            }
            let marker = if selected {
                println!(
                    "{SELECTED_PROFILE_MARKER} allowance={} transport=literal-ip tls_identity=localhost ca=inline fixed_and_verifier={} provider_shared={} postgres_backing=exercised",
                    SELECTED_CONNECTION_GENERATION_ALLOWANCE,
                    SELECTED_CONNECTION_GENERATION_ALLOWANCE,
                    snapshot.provider_shared
                );
                POSITIVE_MARKER
            } else {
                ORDINARY_CONTROL_MARKER
            };
            println!(
                "{marker} peak_ordinary={} peak_root={} provider_shared={} retained_root={}",
                snapshot.peak_ordinary, snapshot.peak_root, snapshot.provider_shared, snapshot.root
            );
        }
        other => return Err(format!("unknown qualification mode: {other}").into()),
    }

    Ok(())
}

#[cfg(test)]
mod phase_tests {
    use super::*;

    #[test]
    fn tls_denial_requires_provider_witness_and_preserves_shared_finality() {
        let ledger = tls_denial_ledger();
        ledger
            .try_reserve(128)
            .expect("transport prefix must be funded before TLS denial");
        ledger.try_reserve_provider_shared(1024).unwrap();
        assert_eq!(ledger.try_reserve(64), Err(BudgetError::Unavailable));
        ledger.release(128);
        let snapshot = ledger.snapshot();
        assert!(snapshot.denied_tls_phase);
        assert_eq!(snapshot.denied_bytes, 64);
        assert_eq!(snapshot.ordinary, 0);
        assert_eq!(snapshot.root, snapshot.provider_shared);
        assert_eq!(snapshot.provider_shared, 1024);
    }
}
