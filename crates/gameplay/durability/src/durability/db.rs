use crate::durability::item_mint_audit::Type2Transaction;
use crate::durability::{DurabilityError, SchemaCompatibility, schema};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use sqlx::pool::{PoolConnection, PoolConnectionReturnDisposition};
use sqlx::postgres::{
    BudgetError, PgAuthenticationPolicy, PgConnectOptions, PgPoolOptions, PgSslMode, Postgres,
    ResourceBudget,
};
use sqlx::{Acquire, PgPool, Transaction};
use std::fmt::{self, Write as _};
use std::future::Future;
use std::net::IpAddr;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex as StdMutex, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const DB_PASS_DEADLINE_MILLIS: u64 = 2_000;
pub const DB_PASS_DEADLINE: Duration = Duration::from_millis(DB_PASS_DEADLINE_MILLIS);
pub const ROOT_RECOVERY_WINDOW: Duration = Duration::from_secs(5);
const HOLDER_IDLE_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const HOLDER_MAX_LIFETIME: Duration = Duration::from_secs(30 * 60);
const WP3_RUNTIME_WORKER_THREADS: usize = 1;
const WP3_RUNTIME_MAX_BLOCKING_THREADS: usize = 1;
const WP3_RUNTIME_WORKER_STACK_BYTES: usize = 2 * 1024 * 1024;
const STATEMENT_CACHE_CAPACITY: usize = 100;
const TLS_SERVER_NAME_MAX_BYTES: usize = 253;
const DATABASE_MAX_BYTES: usize = 63;
const USERNAME_MAX_BYTES: usize = 63;
const PASSWORD_MAX_BYTES: usize = 1_024;
const ROOT_CA_PEM_MAX_BYTES: usize = 22_768;
const RETAINED_CONFIG_MAX_BYTES: usize = 24_171;
const ROOT_CA_MAX_CERTIFICATES: usize = 4;
const ROOT_CA_MAX_DER_BYTES: usize = 4_096;
const ROOT_CA_MAX_AGGREGATE_DER_BYTES: usize = 16_384;
const ROOT_TOTAL_RESIDENT_BYTES: usize = 12_582_912;
const TRANSPORT_IP_TEXT_MAX_BYTES: usize = 39;

struct RootResidentLedger {
    used: AtomicUsize,
    limit: usize,
}

impl RootResidentLedger {
    const fn new(limit: usize) -> Self {
        Self {
            used: AtomicUsize::new(0),
            limit,
        }
    }

    fn try_reserve(&self, bytes: usize) -> Result<(), DurabilityError> {
        let mut current = self.used.load(Ordering::Acquire);
        loop {
            let next = current
                .checked_add(bytes)
                .ok_or(DurabilityError::RootUnavailable)?;
            if next > self.limit {
                return Err(DurabilityError::RootUnavailable);
            }
            match self.used.compare_exchange_weak(
                current,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Ok(()),
                Err(observed) => current = observed,
            }
        }
    }

    fn release(&self, bytes: usize) {
        let released = self
            .used
            .try_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                current.checked_sub(bytes)
            });
        debug_assert!(released.is_ok(), "root resident charge underflow");
    }

    #[cfg(test)]
    fn used(&self) -> usize {
        self.used.load(Ordering::Acquire)
    }
}

static ROOT_RESIDENT_LEDGER: RootResidentLedger =
    RootResidentLedger::new(ROOT_TOTAL_RESIDENT_BYTES);
static PRODUCTION_ROOT_ADMITTED: AtomicBool = AtomicBool::new(false);
static PRODUCTION_RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
static PRODUCTION_RUNTIME_INIT: StdMutex<()> = StdMutex::new(());

struct ProductionRootAdmission {
    committed: bool,
}

impl ProductionRootAdmission {
    fn try_acquire() -> Result<Self, DurabilityError> {
        PRODUCTION_ROOT_ADMITTED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| DurabilityError::RootUnavailable)?;
        Ok(Self { committed: false })
    }

    fn commit(mut self) {
        self.committed = true;
    }
}

impl Drop for ProductionRootAdmission {
    fn drop(&mut self) {
        if !self.committed {
            PRODUCTION_ROOT_ADMITTED.store(false, Ordering::Release);
        }
    }
}

#[derive(Clone)]
enum RootLedgerHandle {
    Process(&'static RootResidentLedger),
    #[cfg(any(test, feature = "test-support"))]
    Test(Arc<RootResidentLedger>),
}

impl RootLedgerHandle {
    fn ledger(&self) -> &RootResidentLedger {
        match self {
            Self::Process(ledger) => ledger,
            #[cfg(any(test, feature = "test-support"))]
            Self::Test(ledger) => ledger,
        }
    }
}

struct RootBudgetAdapter {
    ledger: RootLedgerHandle,
}

impl ResourceBudget for RootBudgetAdapter {
    fn try_reserve(&self, bytes: usize) -> Result<(), BudgetError> {
        self.ledger
            .ledger()
            .try_reserve(bytes)
            .map_err(|_| BudgetError::Unavailable)
    }

    fn release(&self, bytes: usize) {
        self.ledger.ledger().release(bytes);
    }
}

struct RootIReservation {
    ledger: RootLedgerHandle,
    bytes: usize,
}

impl RootIReservation {
    fn try_new(ledger: RootLedgerHandle, bytes: usize) -> Result<Self, DurabilityError> {
        ledger.ledger().try_reserve(bytes)?;
        Ok(Self { ledger, bytes })
    }

    fn resource_budget(&self) -> Arc<dyn ResourceBudget> {
        Arc::new(RootBudgetAdapter {
            ledger: self.ledger.clone(),
        })
    }

    #[cfg(test)]
    fn bytes(&self) -> usize {
        self.bytes
    }
}

impl Drop for RootIReservation {
    fn drop(&mut self) {
        self.ledger.ledger().release(self.bytes);
    }
}

struct RootICharge {
    _reservation: Option<RootIReservation>,
}

fn checked_charge_add(total: usize, additional: usize) -> Result<usize, DurabilityError> {
    total
        .checked_add(additional)
        .ok_or(DurabilityError::RootUnavailable)
}

fn checked_runtime_stack_reservation(
    worker_stack_request: usize,
    blocking_pool_thread_cap: usize,
) -> Result<usize, DurabilityError> {
    worker_stack_request
        .checked_mul(blocking_pool_thread_cap)
        .ok_or(DurabilityError::RootUnavailable)
}

fn conservative_heap_resident_charge(requested: usize) -> Result<usize, DurabilityError> {
    if requested == 0 {
        return Ok(0);
    }
    let word = std::mem::size_of::<usize>();
    let alignment = word
        .checked_mul(2)
        .ok_or(DurabilityError::RootUnavailable)?;
    let metadata = word
        .checked_mul(2)
        .ok_or(DurabilityError::RootUnavailable)?;
    let minimum = word
        .checked_mul(4)
        .ok_or(DurabilityError::RootUnavailable)?;
    let with_metadata = requested
        .checked_add(metadata)
        .ok_or(DurabilityError::RootUnavailable)?;
    let rounded = with_metadata
        .checked_add(alignment - 1)
        .and_then(|value| value.checked_div(alignment))
        .and_then(|value| value.checked_mul(alignment))
        .ok_or(DurabilityError::RootUnavailable)?;
    Ok(rounded.max(minimum))
}

fn arc_allocation_request<T>() -> Result<usize, DurabilityError> {
    use std::alloc::Layout;
    let counters = Layout::array::<AtomicUsize>(2).map_err(|_| DurabilityError::RootUnavailable)?;
    let (layout, _) = counters
        .extend(Layout::new::<T>())
        .map_err(|_| DurabilityError::RootUnavailable)?;
    Ok(layout.pad_to_align().size())
}

fn root_i_reservation_bytes(lengths: [usize; 5]) -> Result<usize, DurabilityError> {
    let mut total = std::mem::size_of::<DurabilityRootConfig>()
        .checked_add(std::mem::size_of::<DurabilityRoot>())
        .ok_or(DurabilityError::RootUnavailable)?;

    for requested in lengths
        .into_iter()
        .chain(std::iter::once(TRANSPORT_IP_TEXT_MAX_BYTES))
    {
        total = checked_charge_add(total, conservative_heap_resident_charge(requested)?)?;
    }

    let pool_allocations = sqlx::pool::retained_pool_core_allocation_sizes::<Postgres>(1)
        .ok_or(DurabilityError::RootUnavailable)?;
    for requested in pool_allocations {
        total = checked_charge_add(total, conservative_heap_resident_charge(requested)?)?;
    }

    let maintenance = sqlx::pool::oteryn_wp3_root_maintenance_task_allocation_profile::<Postgres>();
    for requested in [
        maintenance.future_box_request,
        maintenance.task_cell_request,
    ] {
        if requested != 0 {
            total = checked_charge_add(total, conservative_heap_resident_charge(requested)?)?;
        }
    }

    total = checked_charge_add(
        total,
        conservative_heap_resident_charge(arc_allocation_request::<tokio::runtime::Runtime>()?)?,
    )?;

    let runtime_profile = tokio::runtime::oteryn_wp3_dedicated_runtime_allocation_profile(
        WP3_RUNTIME_WORKER_THREADS,
        WP3_RUNTIME_MAX_BLOCKING_THREADS,
        WP3_RUNTIME_WORKER_STACK_BYTES,
    )
    .ok_or(DurabilityError::RootUnavailable)?;
    if runtime_profile.scheduler_worker_count != WP3_RUNTIME_WORKER_THREADS
        || runtime_profile.blocking_pool_thread_cap
            != WP3_RUNTIME_WORKER_THREADS + WP3_RUNTIME_MAX_BLOCKING_THREADS
        || runtime_profile.additional_blocking_worker_limit != WP3_RUNTIME_MAX_BLOCKING_THREADS
        || runtime_profile.worker_stack_request != WP3_RUNTIME_WORKER_STACK_BYTES
    {
        return Err(DurabilityError::RootUnavailable);
    }
    for requested in runtime_profile.heap_requests {
        if requested != 0 {
            total = checked_charge_add(total, conservative_heap_resident_charge(requested)?)?;
        }
    }
    // Tokio launches the mandatory scheduler worker through this same blocking
    // pool, whose configured cap is worker_threads + max_blocking_threads. Every
    // thread created by BlockingPool::spawn_thread inherits thread_stack_size,
    // so root I must reserve stack backing for the full reachable pool cap rather
    // than only the mandatory scheduler worker.
    let runtime_stack_reservation = checked_runtime_stack_reservation(
        runtime_profile.worker_stack_request,
        runtime_profile.blocking_pool_thread_cap,
    )?;
    total = checked_charge_add(total, runtime_stack_reservation)?;

    for requested in [
        arc_allocation_request::<AtomicBool>()?,
        arc_allocation_request::<Mutex<()>>()?,
        arc_allocation_request::<RootICharge>()?,
        arc_allocation_request::<RootBudgetAdapter>()?,
        arc_allocation_request::<std::sync::OnceLock<Option<Arc<oteryn_gameplay_durability::durability::staff::StaffTrust>>>>()?,
    ] {
        total = checked_charge_add(total, conservative_heap_resident_charge(requested)?)?;
    }

    Ok(total)
}

struct FixedText<const N: usize> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize> FixedText<N> {
    const fn new() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
        }
    }

    fn as_str(&self) -> Result<&str, DurabilityError> {
        std::str::from_utf8(&self.bytes[..self.len]).map_err(|_| DurabilityError::RootUnavailable)
    }
}

impl<const N: usize> fmt::Write for FixedText<N> {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        let end = self.len.checked_add(value.len()).ok_or(fmt::Error)?;
        if end > N {
            return Err(fmt::Error);
        }
        self.bytes[self.len..end].copy_from_slice(value.as_bytes());
        self.len = end;
        Ok(())
    }
}

fn retain_transport_ip_text(transport_ip: IpAddr) -> Result<String, DurabilityError> {
    let mut text = FixedText::<TRANSPORT_IP_TEXT_MAX_BYTES>::new();
    write!(&mut text, "{transport_ip}").map_err(|_| DurabilityError::RootUnavailable)?;
    retain_exact_string(text.as_str()?)
}

fn checked_retained_config_input_len(lengths: [usize; 5]) -> Result<usize, DurabilityError> {
    lengths.into_iter().try_fold(0usize, |total, length| {
        total
            .checked_add(length)
            .ok_or(DurabilityError::RootUnavailable)
    })
}

fn validate_retained_config_input_lengths(lengths: [usize; 5]) -> Result<usize, DurabilityError> {
    let [tls, database, username, password, root_ca] = lengths;
    let aggregate = checked_retained_config_input_len(lengths)?;
    if tls > TLS_SERVER_NAME_MAX_BYTES
        || database > DATABASE_MAX_BYTES
        || username > USERNAME_MAX_BYTES
        || password > PASSWORD_MAX_BYTES
        || root_ca > ROOT_CA_PEM_MAX_BYTES
        || aggregate > RETAINED_CONFIG_MAX_BYTES
    {
        return Err(DurabilityError::RootUnavailable);
    }
    Ok(aggregate)
}

fn validate_dns_server_name(name: &str) -> Result<(), DurabilityError> {
    if !name.is_ascii() || name.ends_with('.') || name.parse::<IpAddr>().is_ok() {
        return Err(DurabilityError::RootUnavailable);
    }
    for label in name.split('.') {
        let bytes = label.as_bytes();
        if bytes.is_empty()
            || bytes.len() > 63
            || !bytes[0].is_ascii_alphanumeric()
            || !bytes[bytes.len() - 1].is_ascii_alphanumeric()
            || bytes
                .iter()
                .any(|byte| !byte.is_ascii_alphanumeric() && *byte != b'-')
        {
            return Err(DurabilityError::RootUnavailable);
        }
    }
    Ok(())
}

fn checked_root_der_total(current: usize, next: usize) -> Result<usize, DurabilityError> {
    let total = current
        .checked_add(next)
        .ok_or(DurabilityError::RootUnavailable)?;
    if total > ROOT_CA_MAX_AGGREGATE_DER_BYTES {
        return Err(DurabilityError::RootUnavailable);
    }
    Ok(total)
}

fn validate_root_ca_pem(root_ca_pem: &[u8]) -> Result<(), DurabilityError> {
    if !root_ca_pem.is_ascii() {
        return Err(DurabilityError::RootUnavailable);
    }

    let has_crlf = root_ca_pem.windows(2).any(|window| window == b"\r\n");
    for (index, byte) in root_ca_pem.iter().copied().enumerate() {
        if byte == b'\r' && root_ca_pem.get(index + 1).copied() != Some(b'\n') {
            return Err(DurabilityError::RootUnavailable);
        }
        if has_crlf
            && byte == b'\n'
            && index
                .checked_sub(1)
                .and_then(|i| root_ca_pem.get(i))
                .copied()
                != Some(b'\r')
        {
            return Err(DurabilityError::RootUnavailable);
        }
        if !has_crlf && byte == b'\r' {
            return Err(DurabilityError::RootUnavailable);
        }
    }

    let text = std::str::from_utf8(root_ca_pem).map_err(|_| DurabilityError::RootUnavailable)?;
    let mut lines: Vec<&str> = if has_crlf {
        text.split("\r\n").collect()
    } else {
        text.split('\n').collect()
    };
    if lines.last() == Some(&"") {
        lines.pop();
    }
    if lines.is_empty() || lines.iter().any(|line| line.is_empty()) {
        return Err(DurabilityError::RootUnavailable);
    }

    let mut index = 0usize;
    let mut certificate_count = 0usize;
    let mut aggregate_der = 0usize;
    while index < lines.len() {
        if lines[index] != "-----BEGIN CERTIFICATE-----" {
            return Err(DurabilityError::RootUnavailable);
        }
        certificate_count = certificate_count
            .checked_add(1)
            .ok_or(DurabilityError::RootUnavailable)?;
        if certificate_count > ROOT_CA_MAX_CERTIFICATES {
            return Err(DurabilityError::RootUnavailable);
        }
        index += 1;

        let start = index;
        while index < lines.len() && lines[index] != "-----END CERTIFICATE-----" {
            index += 1;
        }
        if start == index || index >= lines.len() {
            return Err(DurabilityError::RootUnavailable);
        }
        let encoded_lines = &lines[start..index];
        if encoded_lines
            .iter()
            .take(encoded_lines.len().saturating_sub(1))
            .any(|line| line.len() != 64)
            || encoded_lines
                .last()
                .is_none_or(|line| line.is_empty() || line.len() > 64)
        {
            return Err(DurabilityError::RootUnavailable);
        }

        let encoded_len = encoded_lines.iter().try_fold(0usize, |total, line| {
            total
                .checked_add(line.len())
                .ok_or(DurabilityError::RootUnavailable)
        })?;
        if encoded_len % 4 != 0 {
            return Err(DurabilityError::RootUnavailable);
        }
        let mut encoded = String::with_capacity(encoded_len);
        for line in encoded_lines {
            encoded.push_str(line);
        }
        let padding = encoded
            .as_bytes()
            .iter()
            .rev()
            .take_while(|byte| **byte == b'=')
            .count();
        if padding > 2
            || encoded.as_bytes()[..encoded.len().saturating_sub(padding)].contains(&b'=')
        {
            return Err(DurabilityError::RootUnavailable);
        }
        let decoded_len = encoded_len
            .checked_div(4)
            .and_then(|groups| groups.checked_mul(3))
            .and_then(|bytes| bytes.checked_sub(padding))
            .ok_or(DurabilityError::RootUnavailable)?;
        if decoded_len > ROOT_CA_MAX_DER_BYTES {
            return Err(DurabilityError::RootUnavailable);
        }
        let decoded = STANDARD
            .decode(encoded.as_bytes())
            .map_err(|_| DurabilityError::RootUnavailable)?;
        if decoded.len() != decoded_len {
            return Err(DurabilityError::RootUnavailable);
        }
        aggregate_der = checked_root_der_total(aggregate_der, decoded_len)?;
        index += 1;
    }

    if certificate_count == 0 {
        return Err(DurabilityError::RootUnavailable);
    }
    Ok(())
}

fn retain_exact_string(input: &str) -> Result<String, DurabilityError> {
    let mut retained = String::with_capacity(input.len());
    if retained.capacity() != input.len() {
        return Err(DurabilityError::RootUnavailable);
    }
    retained.push_str(input);
    Ok(retained)
}

fn retain_exact_bytes(input: &[u8]) -> Result<Vec<u8>, DurabilityError> {
    let mut retained = Vec::with_capacity(input.len());
    if retained.capacity() != input.len() {
        return Err(DurabilityError::RootUnavailable);
    }
    retained.extend_from_slice(input);
    Ok(retained)
}

/// Explicit first-slice configuration for the process durability root.
///
/// Every identity- and secret-bearing field is supplied by the caller. Building
/// the SQLx profile from this value never consults `PG*`, passfiles, OS-user
/// defaults, Unix-domain-socket discovery, client certificates, or implicit
/// trust roots.
pub struct DurabilityRootConfig {
    port: u16,
    transport_ip_text: String,
    tls_server_name: String,
    database: String,
    username: String,
    password: String,
    root_ca_pem: Vec<u8>,
    root_i_reservation: RootIReservation,
    production_admission: Option<ProductionRootAdmission>,
}

impl DurabilityRootConfig {
    pub fn new(
        transport_ip: IpAddr,
        port: u16,
        tls_server_name: &str,
        database: &str,
        username: &str,
        password: &str,
        root_ca_pem: &[u8],
    ) -> Result<Self, DurabilityError> {
        Self::new_with_ledger(
            transport_ip,
            port,
            tls_server_name,
            database,
            username,
            password,
            root_ca_pem,
            RootLedgerHandle::Process(&ROOT_RESIDENT_LEDGER),
        )
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn new_with_isolated_test_ledger(
        transport_ip: IpAddr,
        port: u16,
        tls_server_name: &str,
        database: &str,
        username: &str,
        password: &str,
        root_ca_pem: &[u8],
    ) -> Result<Self, DurabilityError> {
        Self::new_with_ledger(
            transport_ip,
            port,
            tls_server_name,
            database,
            username,
            password,
            root_ca_pem,
            RootLedgerHandle::Test(Arc::new(RootResidentLedger::new(ROOT_TOTAL_RESIDENT_BYTES))),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new_with_ledger(
        transport_ip: IpAddr,
        port: u16,
        tls_server_name: &str,
        database: &str,
        username: &str,
        password: &str,
        root_ca_pem: &[u8],
        ledger: RootLedgerHandle,
    ) -> Result<Self, DurabilityError> {
        let lengths = [
            tls_server_name.len(),
            database.len(),
            username.len(),
            password.len(),
            root_ca_pem.len(),
        ];
        let aggregate = validate_retained_config_input_lengths(lengths)?;
        if port == 0
            || tls_server_name.trim().is_empty()
            || tls_server_name.parse::<IpAddr>().is_ok()
            || database.trim().is_empty()
            || username.trim().is_empty()
            || password.is_empty()
            || root_ca_pem.is_empty()
        {
            return Err(DurabilityError::InvalidConfiguration);
        }
        validate_dns_server_name(tls_server_name)?;
        validate_root_ca_pem(root_ca_pem)?;

        let production_admission = if matches!(&ledger, RootLedgerHandle::Process(_)) {
            Some(ProductionRootAdmission::try_acquire()?)
        } else {
            None
        };
        let reservation_bytes = root_i_reservation_bytes(lengths)?;
        let root_i_reservation = RootIReservation::try_new(ledger, reservation_bytes)?;

        let transport_ip_text = retain_transport_ip_text(transport_ip)?;
        let tls_server_name = retain_exact_string(tls_server_name)?;
        let database = retain_exact_string(database)?;
        let username = retain_exact_string(username)?;
        let password = retain_exact_string(password)?;
        let root_ca_pem = retain_exact_bytes(root_ca_pem)?;
        let retained_capacity = checked_retained_config_input_len([
            tls_server_name.capacity(),
            database.capacity(),
            username.capacity(),
            password.capacity(),
            root_ca_pem.capacity(),
        ])?;
        if retained_capacity != aggregate
            || transport_ip_text.capacity() > TRANSPORT_IP_TEXT_MAX_BYTES
        {
            return Err(DurabilityError::RootUnavailable);
        }

        Ok(Self {
            port,
            transport_ip_text,
            tls_server_name,
            database,
            username,
            password,
            root_ca_pem,
            root_i_reservation,
            production_admission,
        })
    }

    fn into_connect_options(
        self,
    ) -> (
        PgConnectOptions,
        PgConnectOptions,
        RootIReservation,
        Option<ProductionRootAdmission>,
    ) {
        let Self {
            port,
            transport_ip_text,
            tls_server_name,
            database,
            username,
            password,
            root_ca_pem,
            root_i_reservation,
            production_admission,
        } = self;
        let resource_budget = root_i_reservation.resource_budget();
        let options = PgConnectOptions::new_without_environment_owned(
            tls_server_name,
            transport_ip_text,
            port,
            database,
            username,
            password,
        )
        .ssl_mode(PgSslMode::VerifyFull)
        .ssl_root_cert_from_pem(root_ca_pem)
        .ssl_use_default_roots(false)
        .ssl_client_auth_none()
        .ssl_tls13_only(true)
        .ssl_session_resumption(false)
        .authentication_policy(PgAuthenticationPolicy::ScramSha256)
        .statement_cache_capacity(STATEMENT_CACHE_CAPACITY);
        // The chat relay pool (CHAT-2B) takes the same verified endpoint and credentials but
        // stays outside the WP3 root budget: it is a separate pool on the node runtime and
        // never carries durability work.
        let relay_options = options.clone();
        let options = options.oteryn_wp3_resource_budget(resource_budget);
        (
            options,
            relay_options,
            root_i_reservation,
            production_admission,
        )
    }
}

/// Build the accepted lazy max-one pool without establishing a connection.
///
/// Connection establishment is deliberately left to serialized root
/// maintenance; active work must use a ready-only `try_begin()` path.
#[cfg(test)]
fn build_root_pool(config: DurabilityRootConfig) -> (PgPool, RootIReservation) {
    let (options, _relay_options, root_i_reservation, _admission) = config.into_connect_options();
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .min_connections(0)
        .acquire_timeout(ROOT_RECOVERY_WINDOW)
        .idle_timeout(HOLDER_IDLE_TIMEOUT)
        .max_lifetime(HOLDER_MAX_LIFETIME)
        .connect_lazy_with(options);
    (pool, root_i_reservation)
}

fn build_wp3_runtime_owned() -> Result<tokio::runtime::Runtime, DurabilityError> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(WP3_RUNTIME_WORKER_THREADS)
        .max_blocking_threads(WP3_RUNTIME_MAX_BLOCKING_THREADS)
        .thread_stack_size(WP3_RUNTIME_WORKER_STACK_BYTES)
        .enable_io()
        .enable_time()
        .build()
        .map_err(|_| DurabilityError::RootUnavailable)?;
    if runtime.handle().runtime_flavor() != tokio::runtime::RuntimeFlavor::MultiThread {
        return Err(DurabilityError::RootUnavailable);
    }
    Ok(runtime)
}

fn production_runtime_handle() -> Result<tokio::runtime::Handle, DurabilityError> {
    if let Some(runtime) = PRODUCTION_RUNTIME.get() {
        return Ok(runtime.handle().clone());
    }

    let _init = match PRODUCTION_RUNTIME_INIT.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some(runtime) = PRODUCTION_RUNTIME.get() {
        return Ok(runtime.handle().clone());
    }

    PRODUCTION_RUNTIME
        .set(build_wp3_runtime_owned()?)
        .map_err(|_| DurabilityError::RootUnavailable)?;
    Ok(PRODUCTION_RUNTIME
        .get()
        .ok_or(DurabilityError::RootUnavailable)?
        .handle()
        .clone())
}

fn build_production_root_pool(
    config: DurabilityRootConfig,
    handle: &tokio::runtime::Handle,
) -> (
    PgPool,
    PgConnectOptions,
    RootIReservation,
    Option<ProductionRootAdmission>,
    tokio::runtime::OterynWp3TaskAllocationProfile,
) {
    let (options, relay_options, root_i_reservation, admission) = config.into_connect_options();
    let (pool, maintenance_profile) = PgPoolOptions::new()
        .max_connections(1)
        .min_connections(0)
        .acquire_timeout(ROOT_RECOVERY_WINDOW)
        .idle_timeout(HOLDER_IDLE_TIMEOUT)
        .max_lifetime(HOLDER_MAX_LIFETIME)
        .connect_lazy_with_oteryn_wp3_runtime(options, handle);
    (
        pool,
        relay_options,
        root_i_reservation,
        admission,
        maintenance_profile,
    )
}

/// Process-scoped owner of the accepted max-one durability holder.
///
/// Active work may only take an already-idle holder through
/// [`DurabilityRoot::try_acquire_ready`]. Connection establishment is confined
/// to [`DurabilityRoot::maintain_ready_once`], which consumes one coalesced
/// demand and never loops or self-retries.
#[derive(Clone)]
enum RootRuntime {
    Dedicated(tokio::runtime::Handle),
    #[cfg(any(test, feature = "test-support"))]
    OwnedTestFixture(Arc<tokio::runtime::Runtime>),
    #[cfg(any(test, feature = "test-support"))]
    AmbientTestFixture,
}

impl RootRuntime {
    fn handle(&self) -> Option<&tokio::runtime::Handle> {
        match self {
            Self::Dedicated(handle) => Some(handle),
            #[cfg(any(test, feature = "test-support"))]
            Self::OwnedTestFixture(runtime) => Some(runtime.handle()),
            #[cfg(any(test, feature = "test-support"))]
            Self::AmbientTestFixture => None,
        }
    }
}

#[derive(Clone)]
pub struct DurabilityRoot {
    pool: PgPool,
    runtime: RootRuntime,
    ready_demand: Arc<AtomicBool>,
    maintenance: Arc<Mutex<()>>,
    /// The deadline of the next semantic pass in milliseconds, shared by every clone:
    /// `DB_PASS_DEADLINE` except while a [`BootPassDeadline`] guard lives.
    pass_deadline_millis: Arc<AtomicU64>,
    /// Connect options for the chat relay pool (CHAT-2B): the root endpoint without the WP3
    /// budget. `None` when the root was built without them.
    relay_options: Option<Box<PgConnectOptions>>,
    _root_i_charge: Option<Arc<RootICharge>>,
    pub(super) staff_trust: Arc<std::sync::OnceLock<Option<Arc<oteryn_gameplay_durability::durability::staff::StaffTrust>>>>,
}

/// While it lives, every semantic pass of its root runs under the boot deadline
/// (NODE-BOOT-DEADLINE-1); dropping it restores `DB_PASS_DEADLINE`. Guards do not nest.
#[must_use = "the boot deadline ends when the guard is dropped"]
pub struct BootPassDeadline {
    pass_deadline_millis: Arc<AtomicU64>,
}

impl Drop for BootPassDeadline {
    fn drop(&mut self) {
        self.pass_deadline_millis
            .store(DB_PASS_DEADLINE_MILLIS, Ordering::Release);
    }
}

impl DurabilityRoot {
    /// Longest semantic-pass deadline a boot step may take (NODE-BOOT-DEADLINE-1).
    pub const MAX_BOOT_PASS_DEADLINE: Duration = Duration::from_secs(60);

    pub fn new(config: DurabilityRootConfig) -> Result<Self, DurabilityError> {
        let production = config.production_admission.is_some();
        let expected_maintenance =
            sqlx::pool::oteryn_wp3_root_maintenance_task_allocation_profile::<Postgres>();

        let runtime = if production {
            RootRuntime::Dedicated(production_runtime_handle()?)
        } else {
            #[cfg(any(test, feature = "test-support"))]
            {
                RootRuntime::OwnedTestFixture(Arc::new(build_wp3_runtime_owned()?))
            }
            #[cfg(not(any(test, feature = "test-support")))]
            {
                return Err(DurabilityError::RootUnavailable);
            }
        };
        let runtime_handle = runtime.handle().ok_or(DurabilityError::RootUnavailable)?;
        let (pool, relay_options, root_i_reservation, admission, actual_maintenance) =
            build_production_root_pool(config, runtime_handle);
        if actual_maintenance != expected_maintenance {
            return Err(DurabilityError::RootUnavailable);
        }

        let root_i_charge = if production {
            std::mem::forget(root_i_reservation);
            let admission = admission.ok_or(DurabilityError::RootUnavailable)?;
            admission.commit();
            None
        } else {
            Some(Arc::new(RootICharge {
                _reservation: Some(root_i_reservation),
            }))
        };

        Ok(Self {
            pool,
            runtime,
            ready_demand: Arc::new(AtomicBool::new(true)),
            maintenance: Arc::new(Mutex::new(())),
            pass_deadline_millis: Arc::new(AtomicU64::new(DB_PASS_DEADLINE_MILLIS)),
            relay_options: Some(Box::new(relay_options)),
            _root_i_charge: root_i_charge,
            staff_trust: Arc::default(),
        })
    }

    /// Connect options for the World chat relay pool (CHAT-2B): the root's verified endpoint
    /// and credentials without the WP3 root budget. The relay pool is separate from the root
    /// holder and carries no durability work.
    pub fn chat_relay_connect_options(&self) -> Option<PgConnectOptions> {
        self.relay_options.as_deref().cloned()
    }

    #[cfg(any(test, feature = "test-support"))]
    #[allow(dead_code)]
    pub fn connect_test_runtime(database_url: &str) -> Result<Self, DurabilityError> {
        Ok(Self {
            pool: PgPoolOptions::new()
                .max_connections(1)
                .min_connections(0)
                .acquire_timeout(ROOT_RECOVERY_WINDOW)
                .idle_timeout(HOLDER_IDLE_TIMEOUT)
                .max_lifetime(HOLDER_MAX_LIFETIME)
                .connect_lazy(database_url)?,
            runtime: RootRuntime::AmbientTestFixture,
            ready_demand: Arc::new(AtomicBool::new(true)),
            maintenance: Arc::new(Mutex::new(())),
            pass_deadline_millis: Arc::new(AtomicU64::new(DB_PASS_DEADLINE_MILLIS)),
            relay_options: <PgConnectOptions as std::str::FromStr>::from_str(database_url)
                .ok()
                .map(Box::new),
            _root_i_charge: None,
            staff_trust: Arc::default(),
        })
    }

    pub fn spawn_task<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        match &self.runtime {
            RootRuntime::Dedicated(handle) => handle.spawn(future),
            #[cfg(any(test, feature = "test-support"))]
            RootRuntime::OwnedTestFixture(runtime) => runtime.handle().spawn(future),
            #[cfg(any(test, feature = "test-support"))]
            RootRuntime::AmbientTestFixture => tokio::spawn(future),
        }
    }

    /// Record a genuine event that permits one later maintenance window.
    pub fn request_ready(&self) {
        self.ready_demand.store(true, Ordering::Release);
    }

    /// Checkout only an already-idle holder. This method never establishes a connection.
    pub fn try_acquire_ready(&self) -> Result<PoolConnection<Postgres>, DurabilityError> {
        if let Some(holder) = self.pool.try_acquire() {
            return Ok(holder);
        }

        self.request_ready();
        Err(DurabilityError::RootUnavailable)
    }

    /// Consume at most one coalesced demand and run one bounded maintenance window.
    ///
    /// A failed window does not re-arm itself. A demand arriving while this window
    /// is running remains recorded and may authorize one later call.
    pub async fn maintain_ready_once(&self) -> Result<bool, DurabilityError> {
        match &self.runtime {
            RootRuntime::Dedicated(_) => {
                let root = self.clone();
                await_root_task(
                    self.spawn_task(async move { root.maintain_ready_once_inner().await }),
                )
                .await
            }
            #[cfg(any(test, feature = "test-support"))]
            RootRuntime::OwnedTestFixture(_) => {
                let root = self.clone();
                await_root_task(
                    self.spawn_task(async move { root.maintain_ready_once_inner().await }),
                )
                .await
            }
            #[cfg(any(test, feature = "test-support"))]
            RootRuntime::AmbientTestFixture => self.maintain_ready_once_inner().await,
        }
    }

    async fn maintain_ready_once_inner(&self) -> Result<bool, DurabilityError> {
        let _maintenance = self.maintenance.lock().await;
        if !self.ready_demand.swap(false, Ordering::AcqRel) {
            return Ok(self.pool.num_idle() > 0);
        }

        let deadline = Instant::now() + ROOT_RECOVERY_WINDOW;
        let remaining = deadline.saturating_duration_since(Instant::now());
        let acquired = tokio::time::timeout(remaining, self.pool.acquire())
            .await
            .map_err(|_| DurabilityError::RootUnavailable)?;
        let mut holder = match acquired {
            Ok(holder) => holder,
            Err(error) => return Err(DurabilityError::from(error)),
        };

        let remaining = deadline.saturating_duration_since(Instant::now());
        let compatibility =
            match tokio::time::timeout(remaining, schema::inspect_connection(&mut holder)).await {
                Ok(Ok(compatibility)) => compatibility,
                Ok(Err(error)) => {
                    holder.close_on_drop();
                    return Err(error);
                }
                Err(_) => {
                    holder.close_on_drop();
                    return Err(DurabilityError::RootUnavailable);
                }
            };
        if compatibility != SchemaCompatibility::Compatible {
            holder.close_on_drop();
            return Err(DurabilityError::SchemaIncompatible(compatibility));
        }

        match holder.return_to_pool_observed_until(deadline).await {
            PoolConnectionReturnDisposition::ReturnedToIdle => Ok(true),
            PoolConnectionReturnDisposition::RetiredClosed
            | PoolConnectionReturnDisposition::NoEvidence => Err(DurabilityError::RootUnavailable),
        }
    }

    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.pool.num_idle() > 0
    }

    #[must_use]
    pub fn has_ready_demand(&self) -> bool {
        self.ready_demand.load(Ordering::Acquire)
    }

    /// Stable identity of this root shared by all of its clones.
    pub fn root_identity(&self) -> usize {
        Arc::as_ptr(&self.maintenance) as usize
    }

    /// Liveness of this root's identity; it expires only when every clone of
    /// the root is gone, so a reused identity is never mistaken for this root.
    pub fn root_liveness(&self) -> std::sync::Weak<dyn std::any::Any + Send + Sync> {
        let liveness: Arc<dyn std::any::Any + Send + Sync> = self.maintenance.clone();
        Arc::downgrade(&liveness)
    }

    /// Run the semantic passes issued while the returned guard lives under `deadline`,
    /// bounded to `DB_PASS_DEADLINE..=MAX_BOOT_PASS_DEADLINE`. Only the sequential boot
    /// steps take it, before the node serves, so no gameplay pass runs under it; the
    /// PostgreSQL transaction, statement and lock timeouts follow the same deadline.
    pub fn boot_pass_deadline(&self, deadline: Duration) -> BootPassDeadline {
        let bounded = deadline.clamp(DB_PASS_DEADLINE, Self::MAX_BOOT_PASS_DEADLINE);
        let millis = u64::try_from(bounded.as_millis()).unwrap_or(DB_PASS_DEADLINE_MILLIS);
        self.pass_deadline_millis.store(millis, Ordering::Release);
        BootPassDeadline {
            pass_deadline_millis: Arc::clone(&self.pass_deadline_millis),
        }
    }

    fn pass_deadline(&self) -> Duration {
        Duration::from_millis(self.pass_deadline_millis.load(Ordering::Acquire))
    }

    pub fn try_issue_semantic_pass(&self) -> Result<IssuedSemanticPass, DurabilityError> {
        let holder = self.try_acquire_ready()?;
        Ok(IssuedSemanticPass {
            root: self.clone(),
            holder,
            deadline: Instant::now() + self.pass_deadline(),
        })
    }

    /// [`Self::try_issue_semantic_pass`] for work owed to an outer operation
    /// budget: the pass deadline is the earlier of the ordinary one and
    /// `deadline`, and an already expired budget issues nothing.
    pub fn try_issue_semantic_pass_by(
        &self,
        deadline: Instant,
    ) -> Result<IssuedSemanticPass, DurabilityError> {
        let now = Instant::now();
        if deadline <= now {
            return Err(DurabilityError::RootPassDeadlineExceeded);
        }
        let holder = self.try_acquire_ready()?;
        Ok(IssuedSemanticPass {
            root: self.clone(),
            holder,
            deadline: bounded_pass_deadline(now, self.pass_deadline(), deadline),
        })
    }
}

/// The earlier of the ordinary pass deadline from `now` and an outer `budget`.
fn bounded_pass_deadline(now: Instant, pass: Duration, budget: Instant) -> Instant {
    (now + pass).min(budget)
}

pub async fn await_root_task<T>(
    task: tokio::task::JoinHandle<Result<T, DurabilityError>>,
) -> Result<T, DurabilityError>
where
    T: Send + 'static,
{
    match task.await {
        Ok(result) => result,
        Err(error) if error.is_panic() => std::panic::resume_unwind(error.into_panic()),
        Err(_) => Err(DurabilityError::RootTaskFailed),
    }
}

pub struct IssuedSemanticPass {
    root: DurabilityRoot,
    holder: PoolConnection<Postgres>,
    deadline: Instant,
}

impl IssuedSemanticPass {
    pub async fn run<T, F>(self, operation: F) -> Result<T, DurabilityError>
    where
        T: Send,
        F: for<'a> FnOnce(
                &'a mut PoolConnection<Postgres>,
                Instant,
            )
                -> Pin<Box<dyn Future<Output = Result<T, DurabilityError>> + Send + 'a>>
            + Send,
    {
        let mut context = ();
        self.run_with_context(&mut context, move |holder, deadline, _| {
            operation(holder, deadline)
        })
        .await
    }

    /// Keep borrowed real owner context scoped to this existing bounded pass.
    /// The context is an input borrow, not a captured 'static controller or a
    /// copied authority. Holder acquisition, deadline, unknown-COMMIT handling
    /// and observed pool return are exactly the ordinary semantic-pass path.
    pub async fn run_with_context<T, C, F>(
        mut self,
        context: &mut C,
        operation: F,
    ) -> Result<T, DurabilityError>
    where
        T: Send,
        C: Send,
        F: for<'a> FnOnce(
                &'a mut PoolConnection<Postgres>,
                Instant,
                &'a mut C,
            )
                -> Pin<Box<dyn Future<Output = Result<T, DurabilityError>> + Send + 'a>>
            + Send,
    {
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        let result = match tokio::time::timeout(
            remaining,
            operation(&mut self.holder, self.deadline, context),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err(DurabilityError::RootPassDeadlineExceeded),
        };

        if matches!(&result, Err(DurabilityError::CommitOutcomeUnknown)) {
            self.holder.close_on_drop();
            self.root.request_ready();
            return result;
        }

        match self
            .holder
            .return_to_pool_observed_until(self.deadline)
            .await
        {
            PoolConnectionReturnDisposition::ReturnedToIdle => result,
            PoolConnectionReturnDisposition::RetiredClosed => {
                self.root.request_ready();
                result
            }
            PoolConnectionReturnDisposition::NoEvidence => Err(DurabilityError::InvalidStoredState),
        }
    }
}

pub async fn begin_semantic_transaction<'a>(
    holder: &'a mut PoolConnection<Postgres>,
    deadline: Instant,
) -> Result<Transaction<'a, Postgres>, DurabilityError> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(DurabilityError::RootPassDeadlineExceeded);
    }

    let mut transaction = tokio::time::timeout(remaining, holder.begin())
        .await
        .map_err(|_| DurabilityError::RootPassDeadlineExceeded)??;

    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(DurabilityError::RootPassDeadlineExceeded);
    }
    let millis = u64::try_from(remaining.as_millis().max(1))
        .map_err(|_| DurabilityError::RootPassDeadlineExceeded)?;
    let timeout_value = format!("{millis}ms");
    tokio::time::timeout(
        remaining,
        sqlx::query(
            "SELECT set_config('transaction_timeout', $1, true), \
             set_config('statement_timeout', $1, true), \
             set_config('lock_timeout', $1, true)",
        )
        .bind(timeout_value)
        .execute(&mut *transaction),
    )
    .await
    .map_err(|_| DurabilityError::RootPassDeadlineExceeded)??;

    Ok(transaction)
}

/// The opener of every transaction that can insert a type-2 audit event (GOLD-FEE-ACT-PACKET-1
/// §1.4): [`begin_semantic_transaction`], then the shared activation fence and the activation
/// read as its first statements, bounded by the same deadline settings.
pub async fn begin_type2_transaction<'a>(
    holder: &'a mut PoolConnection<Postgres>,
    deadline: Instant,
) -> Result<Type2Transaction<'a>, DurabilityError> {
    let transaction = begin_semantic_transaction(holder, deadline).await?;
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(DurabilityError::RootPassDeadlineExceeded);
    }
    Ok(
        tokio::time::timeout(remaining, Type2Transaction::open(transaction))
            .await
            .map_err(|_| DurabilityError::RootPassDeadlineExceeded)??,
    )
}

pub async fn commit_semantic_transaction(
    transaction: Transaction<'_, Postgres>,
    deadline: Instant,
) -> Result<(), DurabilityError> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(DurabilityError::CommitOutcomeUnknown);
    }

    match tokio::time::timeout(remaining, transaction.commit()).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(DurabilityError::from_commit_error(error)),
        Err(_) => Err(DurabilityError::CommitOutcomeUnknown),
    }
}

pub async fn connect(database_url: &str, max_connections: u32) -> Result<PgPool, DurabilityError> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(CONNECT_TIMEOUT)
        .connect(database_url)
        .await
        .map_err(DurabilityError::from)
}

#[cfg(test)]
#[path = "db_wp3_root_contract_tests.rs"]
mod wp3_root_contract_tests;

#[cfg(test)]
#[path = "db_runtime_scope_identity_red_tests.rs"]
mod runtime_scope_identity_red_tests;

/// Conservative first implementation: all journal writers and row-locking
/// readers serialize before any semantic clock sample. EXCLUSIVE still permits
/// ordinary snapshot readers. This intentionally makes no concurrency claim.
pub(super) async fn lock_admission_domain(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    record: &crate::foundation::ReconnectDurabilityRecordV1,
) -> Result<(), DurabilityError> {
    lock_admission_relations(transaction).await?;
    let identity = record.identity();
    let mut keys = vec![
        (
            b"account".as_slice(),
            identity.account_id().as_bytes().to_vec(),
        ),
        (
            b"character".as_slice(),
            identity.character_id().as_bytes().to_vec(),
        ),
        (
            b"session".as_slice(),
            identity.game_session_id().as_bytes().to_vec(),
        ),
        (
            b"transport".as_slice(),
            record.connection().transport_ref().to_bytes().to_vec(),
        ),
        (
            b"attempt".as_slice(),
            [
                identity.game_session_id().as_bytes().as_slice(),
                &identity.reconnect_attempt_ref().to_be_bytes(),
            ]
            .concat(),
        ),
        (
            b"epoch".as_slice(),
            [
                identity.character_id().as_bytes().as_slice(),
                &record.continuity().control_loss_epoch().get().to_be_bytes(),
            ]
            .concat(),
        ),
    ];
    let mut scope = super::admission_authority_guards::Writer::new(33);
    super::admission_authority_guards::write_scope(&mut scope, identity.runtime_scope())?;
    keys.push((b"runtime".as_slice(), scope.bytes));
    if let crate::foundation::ReconnectProofV1::ReauthenticatedRecovery {
        recovery_grant_nonce,
        ..
    } = record.proof()
    {
        keys.push((b"recovery-nonce".as_slice(), recovery_grant_nonce.to_vec()));
    }
    // Stable FNV-1a only chooses serialization buckets. Hash collisions may
    // over-serialize; complete typed identities remain mandatory SQL predicates.
    let mut physical: Vec<i64> = keys
        .into_iter()
        .map(|(domain, key)| {
            let mut hash = 0xcbf2_9ce4_8422_2325_u64;
            for byte in b"oteryn-admission-v1"
                .iter()
                .chain(domain)
                .chain([0].iter())
                .chain(&key)
            {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
            i64::from_be_bytes(hash.to_be_bytes())
        })
        .collect();
    physical.sort_unstable();
    physical.dedup();
    for key in physical {
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(key)
            .execute(&mut **transaction)
            .await?;
    }
    Ok(())
}

/// Shared strongest relation fence, acquired before domain keys and semantic time.
pub(super) async fn lock_admission_relations(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<(), DurabilityError> {
    // Lexical order, complete current ledger. Immediate FK/unique/index work on
    // these relations cannot introduce a new competing writer/row lock after L.
    for relation in [
        "game_durability_admission_account_guards",
        "game_durability_admission_character_guards",
        "game_durability_admission_guard_history",
        "game_durability_admission_lifecycle_receipts",
        "game_durability_admission_runtime_guards",
        "game_durability_admission_signing_trust_guards",
        "game_durability_control_loss_continuity",
        "game_durability_executor_custody",
        "game_durability_fresh_admission_receipts",
        "game_durability_reconnect_attempts",
        "game_durability_reconnect_pending_commands",
        "game_durability_reconnect_sessions",
        "game_durability_recovery_grant_consumptions",
        "game_durability_session_replacements",
        "game_durability_session_use_ledgers",
        "game_durability_session_use_memberships",
        "game_durability_transport_ref_reservations",
    ] {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "LOCK TABLE {relation} IN EXCLUSIVE MODE"
        )))
        .execute(&mut **transaction)
        .await?;
    }
    Ok(())
}
