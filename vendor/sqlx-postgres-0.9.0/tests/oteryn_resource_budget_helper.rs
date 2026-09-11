use sqlx::postgres::{BudgetError, PgConnectOptions, PgConnection, PgSslMode, ResourceBudget};
use std::error::Error;
use std::path::Path;
use std::sync::{Arc, Mutex};

const SLOT_LIMIT: usize = 4_194_304;
const ROOT_LIMIT: usize = 12_582_912;
const DENIAL_MARKER: &str = "OTERYN_WP3_TLS_DENIAL_UNAVAILABLE";
const POSITIVE_MARKER: &str = "OTERYN_WP3_PG17_TLS13_VERIFY_FULL_POSITIVE";
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
}

impl Ledger {
    fn new(ordinary_limit: usize, root_limit: usize) -> Self {
        Self {
            ordinary_limit,
            root_limit,
            state: Mutex::new(State::default()),
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

fn options(admin_url: &str, ca_path: &Path) -> Result<PgConnectOptions, Box<dyn Error>> {
    let options: PgConnectOptions = admin_url.parse()?;
    Ok(options
        .host("localhost")
        .ssl_mode(PgSslMode::VerifyFull)
        .ssl_root_cert(ca_path))
}

fn main() -> Result<(), Box<dyn Error>> {
    let mode = std::env::var("OTERYN_WP3_MODE")?;
    let admin_url = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL")?;
    let ca_path = std::env::var("OTERYN_WP3_CA_CERT")?;
    let options = options(&admin_url, Path::new(&ca_path))?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    match mode.as_str() {
        "deny" => {
            let ledger = Arc::new(Ledger::new(0, 0));
            let owner: Arc<dyn ResourceBudget> = ledger.clone();
            let error = match runtime.block_on(PgConnection::establish_with_resource_budget(
                &options, owner,
            )) {
                Ok(connection) => {
                    drop(connection);
                    return Err("underfunded owner unexpectedly established TLS".into());
                }
                Err(error) => error,
            };
            match error {
                sqlx::Error::Tls(source) => {
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
                    return Err(
                        format!("resource denial escaped through wrong SQLx surface: {other}")
                            .into(),
                    );
                }
            }
            let snapshot = ledger.snapshot();
            if snapshot.ordinary != 0
                || snapshot.root != 0
                || snapshot.peak_ordinary != 0
                || snapshot.peak_root != 0
                || snapshot.provider_shared != 0
            {
                return Err("denied owner retained or acquired resource debt".into());
            }
            println!("{DENIAL_MARKER}");
        }
        "positive" => {
            let ledger = Arc::new(Ledger::new(SLOT_LIMIT, ROOT_LIMIT));
            let owner: Arc<dyn ResourceBudget> = ledger.clone();
            let mut connection = match runtime.block_on(
                PgConnection::establish_with_resource_budget(&options, owner),
            ) {
                Ok(connection) => connection,
                Err(error) => {
                    let snapshot = ledger.snapshot();
                    let trace = ledger.trace();
                    return Err(format!(
                        "funded owner-aware TLS failed: {error}; denied_bytes={}, denied_at_ordinary={}, denied_at_root={}, peak_ordinary={}, peak_root={}, provider_shared={}, trace=[{}]",
                        snapshot.denied_bytes,
                        snapshot.denied_ordinary,
                        snapshot.denied_root,
                        snapshot.peak_ordinary,
                        snapshot.peak_root,
                        snapshot.provider_shared,
                        trace
                    )
                    .into());
                }
            };
            let (server_version_num, ssl_enabled, ssl_version) = runtime.block_on(async {
                let server_version_num: String = sqlx::query_scalar("SHOW server_version_num")
                    .fetch_one(&mut connection)
                    .await?;
                let (ssl_enabled, ssl_version): (bool, Option<String>) = sqlx::query_as(
                    "SELECT ssl, version FROM pg_stat_ssl WHERE pid = pg_backend_pid()",
                )
                .fetch_one(&mut connection)
                .await?;
                Ok::<_, sqlx::Error>((server_version_num, ssl_enabled, ssl_version))
            })?;
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
            drop(connection);

            let snapshot = ledger.snapshot();
            if snapshot.peak_ordinary == 0 || snapshot.peak_ordinary > SLOT_LIMIT {
                return Err("ordinary TLS custody was not bounded by the slot ledger".into());
            }
            if snapshot.provider_shared == 0 || snapshot.peak_root > ROOT_LIMIT {
                return Err(
                    "AWS-LC provider residency was not charged to the same root ledger".into(),
                );
            }
            if snapshot.ordinary != 0 || snapshot.root != snapshot.provider_shared {
                return Err(format!(
                    "connection teardown left invalid custody: ordinary={}, root={}, provider_shared={}",
                    snapshot.ordinary, snapshot.root, snapshot.provider_shared
                )
                .into());
            }
            println!(
                "{POSITIVE_MARKER} peak_ordinary={} peak_root={} provider_shared={} retained_root={}",
                snapshot.peak_ordinary,
                snapshot.peak_root,
                snapshot.provider_shared,
                snapshot.root
            );
        }
        other => return Err(format!("unknown qualification mode: {other}").into()),
    }

    Ok(())
}
