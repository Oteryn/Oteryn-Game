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
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex as StdMutex, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
pub const DB_PASS_DEADLINE: Duration = Duration::from_secs(2);
pub(crate) const ROOT_RECOVERY_WINDOW: Duration = Duration::from_secs(5);
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
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
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
    #[cfg(test)]
    Test(Arc<RootResidentLedger>),
}

impl RootLedgerHandle {
    fn ledger(&self) -> &RootResidentLedger {
        match self {
            Self::Process(ledger) => ledger,
            #[cfg(test)]
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

    #[cfg(test)]
    pub(crate) fn new_with_isolated_test_ledger(
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
        .oteryn_wp3_resource_budget(resource_budget)
        .statement_cache_capacity(STATEMENT_CACHE_CAPACITY);
        (options, root_i_reservation, production_admission)
    }
}

/// Build the accepted lazy max-one pool without establishing a connection.
///
/// Connection establishment is deliberately left to serialized root
/// maintenance; active work must use a ready-only `try_begin()` path.
#[cfg(test)]
fn build_root_pool(config: DurabilityRootConfig) -> (PgPool, RootIReservation) {
    let (options, root_i_reservation, _admission) = config.into_connect_options();
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
    RootIReservation,
    Option<ProductionRootAdmission>,
    tokio::runtime::OterynWp3TaskAllocationProfile,
) {
    let (options, root_i_reservation, admission) = config.into_connect_options();
    let (pool, maintenance_profile) = PgPoolOptions::new()
        .max_connections(1)
        .min_connections(0)
        .acquire_timeout(ROOT_RECOVERY_WINDOW)
        .idle_timeout(HOLDER_IDLE_TIMEOUT)
        .max_lifetime(HOLDER_MAX_LIFETIME)
        .connect_lazy_with_oteryn_wp3_runtime(options, handle);
    (pool, root_i_reservation, admission, maintenance_profile)
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
    #[cfg(test)]
    OwnedTestFixture(Arc<tokio::runtime::Runtime>),
    #[cfg(test)]
    AmbientTestFixture,
}

impl RootRuntime {
    fn handle(&self) -> Option<&tokio::runtime::Handle> {
        match self {
            Self::Dedicated(handle) => Some(handle),
            #[cfg(test)]
            Self::OwnedTestFixture(runtime) => Some(runtime.handle()),
            #[cfg(test)]
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
    _root_i_charge: Option<Arc<RootICharge>>,
}

impl DurabilityRoot {
    pub fn new(config: DurabilityRootConfig) -> Result<Self, DurabilityError> {
        let production = config.production_admission.is_some();
        let expected_maintenance =
            sqlx::pool::oteryn_wp3_root_maintenance_task_allocation_profile::<Postgres>();

        let runtime = if production {
            RootRuntime::Dedicated(production_runtime_handle()?)
        } else {
            #[cfg(test)]
            {
                RootRuntime::OwnedTestFixture(Arc::new(build_wp3_runtime_owned()?))
            }
            #[cfg(not(test))]
            {
                return Err(DurabilityError::RootUnavailable);
            }
        };
        let runtime_handle = runtime.handle().ok_or(DurabilityError::RootUnavailable)?;
        let (pool, root_i_reservation, admission, actual_maintenance) =
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
            _root_i_charge: root_i_charge,
        })
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn connect_test_runtime(database_url: &str) -> Result<Self, DurabilityError> {
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
            _root_i_charge: None,
        })
    }

    pub(crate) fn spawn_task<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        match &self.runtime {
            RootRuntime::Dedicated(handle) => handle.spawn(future),
            #[cfg(test)]
            RootRuntime::OwnedTestFixture(runtime) => runtime.handle().spawn(future),
            #[cfg(test)]
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
            #[cfg(test)]
            RootRuntime::OwnedTestFixture(_) => {
                let root = self.clone();
                await_root_task(
                    self.spawn_task(async move { root.maintain_ready_once_inner().await }),
                )
                .await
            }
            #[cfg(test)]
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
    pub(crate) fn root_identity(&self) -> usize {
        Arc::as_ptr(&self.maintenance) as usize
    }

    /// Liveness of this root's identity; it expires only when every clone of
    /// the root is gone, so a reused identity is never mistaken for this root.
    pub(crate) fn root_liveness(&self) -> std::sync::Weak<dyn std::any::Any + Send + Sync> {
        let liveness: Arc<dyn std::any::Any + Send + Sync> = self.maintenance.clone();
        Arc::downgrade(&liveness)
    }

    pub(crate) fn try_issue_semantic_pass(&self) -> Result<IssuedSemanticPass, DurabilityError> {
        let holder = self.try_acquire_ready()?;
        Ok(IssuedSemanticPass {
            root: self.clone(),
            holder,
            deadline: Instant::now() + DB_PASS_DEADLINE,
        })
    }
}

pub(crate) async fn await_root_task<T>(
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

pub(crate) struct IssuedSemanticPass {
    root: DurabilityRoot,
    holder: PoolConnection<Postgres>,
    deadline: Instant,
}

impl IssuedSemanticPass {
    pub(crate) async fn run<T, F>(mut self, operation: F) -> Result<T, DurabilityError>
    where
        T: Send,
        F: for<'a> FnOnce(
                &'a mut PoolConnection<Postgres>,
                Instant,
            )
                -> Pin<Box<dyn Future<Output = Result<T, DurabilityError>> + Send + 'a>>
            + Send,
    {
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        let result =
            match tokio::time::timeout(remaining, operation(&mut self.holder, self.deadline)).await
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

pub(crate) async fn begin_semantic_transaction<'a>(
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

pub(crate) async fn commit_semantic_transaction(
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
mod wp3_root_contract_tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    fn config() -> Result<DurabilityRootConfig, DurabilityError> {
        DurabilityRootConfig::new_with_isolated_test_ledger(
            IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7)),
            5432,
            "db.example",
            "oteryn",
            "explicit",
            "test-secret",
            b"-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n",
        )
    }

    #[test]
    fn root_configuration_rejects_missing_identity_material_before_retention() {
        let ip = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7));
        let ca = b"-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n";

        assert!(matches!(
            DurabilityRootConfig::new(ip, 0, "db.example", "oteryn", "explicit", "secret", ca),
            Err(DurabilityError::InvalidConfiguration)
        ));
        assert!(matches!(
            DurabilityRootConfig::new(ip, 5432, "203.0.113.9", "oteryn", "explicit", "secret", ca),
            Err(DurabilityError::InvalidConfiguration)
        ));
        assert!(matches!(
            DurabilityRootConfig::new(ip, 5432, "db.example", " ", "explicit", "secret", ca),
            Err(DurabilityError::InvalidConfiguration)
        ));
        assert!(matches!(
            DurabilityRootConfig::new(ip, 5432, "db.example", "oteryn", " ", "secret", ca),
            Err(DurabilityError::InvalidConfiguration)
        ));
        assert!(matches!(
            DurabilityRootConfig::new(ip, 5432, "db.example", "oteryn", "explicit", "secret", &[]),
            Err(DurabilityError::InvalidConfiguration)
        ));
    }

    fn max_dns_server_name() -> String {
        format!(
            "{}.{}.{}.{}",
            "a".repeat(63),
            "b".repeat(63),
            "c".repeat(63),
            "d".repeat(61)
        )
    }

    fn pem_block(der_len: usize, newline: &str) -> Vec<u8> {
        let encoded = STANDARD.encode(vec![0x30; der_len]);
        let mut pem = String::from("-----BEGIN CERTIFICATE-----");
        pem.push_str(newline);
        for chunk in encoded.as_bytes().chunks(64) {
            for byte in chunk {
                pem.push(char::from(*byte));
            }
            pem.push_str(newline);
        }
        pem.push_str("-----END CERTIFICATE-----");
        pem.push_str(newline);
        pem.into_bytes()
    }

    fn pem_bundle(certificates: usize, der_len: usize, newline: &str) -> Vec<u8> {
        let block = pem_block(der_len, newline);
        let mut bundle = Vec::with_capacity(block.len() * certificates);
        for _ in 0..certificates {
            bundle.extend_from_slice(&block);
        }
        bundle
    }

    #[test]
    fn retained_configuration_accepts_exact_protected_maxima_before_copy() {
        let ip = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7));
        let tls = max_dns_server_name();
        let database = "d".repeat(DATABASE_MAX_BYTES);
        let username = "u".repeat(USERNAME_MAX_BYTES);
        let password = "p".repeat(PASSWORD_MAX_BYTES);
        let root_ca = pem_bundle(ROOT_CA_MAX_CERTIFICATES, ROOT_CA_MAX_DER_BYTES, "\r\n");

        assert_eq!(tls.len(), TLS_SERVER_NAME_MAX_BYTES);
        assert_eq!(root_ca.len(), ROOT_CA_PEM_MAX_BYTES);
        assert!(matches!(
            validate_retained_config_input_lengths([
                tls.len(),
                database.len(),
                username.len(),
                password.len(),
                root_ca.len(),
            ]),
            Ok(RETAINED_CONFIG_MAX_BYTES)
        ));

        let config = DurabilityRootConfig::new_with_isolated_test_ledger(
            ip, 5432, &tls, &database, &username, &password, &root_ca,
        );
        assert!(config.is_ok());
        let Ok(config) = config else {
            return;
        };
        assert_eq!(config.tls_server_name.capacity(), tls.len());
        assert_eq!(config.database.capacity(), database.len());
        assert_eq!(config.username.capacity(), username.len());
        assert_eq!(config.password.capacity(), password.len());
        assert_eq!(config.root_ca_pem.capacity(), root_ca.len());
    }

    #[test]
    fn retained_configuration_rejects_every_first_byte_above_as_unavailable() {
        let ip = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7));
        let valid_ca = pem_block(32, "\n");
        let tls_over = format!(
            "{}.{}.{}.{}",
            "a".repeat(63),
            "b".repeat(63),
            "c".repeat(63),
            "d".repeat(62)
        );
        let database_over = "d".repeat(DATABASE_MAX_BYTES + 1);
        let username_over = "u".repeat(USERNAME_MAX_BYTES + 1);
        let password_over = "p".repeat(PASSWORD_MAX_BYTES + 1);
        let mut root_ca_over = pem_bundle(ROOT_CA_MAX_CERTIFICATES, ROOT_CA_MAX_DER_BYTES, "\r\n");
        root_ca_over.push(b'X');

        assert_eq!(tls_over.len(), TLS_SERVER_NAME_MAX_BYTES + 1);
        assert_eq!(root_ca_over.len(), ROOT_CA_PEM_MAX_BYTES + 1);
        for result in [
            DurabilityRootConfig::new(ip, 5432, &tls_over, "d", "u", "p", &valid_ca),
            DurabilityRootConfig::new(ip, 5432, "a", &database_over, "u", "p", &valid_ca),
            DurabilityRootConfig::new(ip, 5432, "a", "d", &username_over, "p", &valid_ca),
            DurabilityRootConfig::new(ip, 5432, "a", "d", "u", &password_over, &valid_ca),
            DurabilityRootConfig::new(ip, 5432, "a", "d", "u", "p", &root_ca_over),
        ] {
            assert!(matches!(result, Err(DurabilityError::RootUnavailable)));
        }

        assert!(matches!(
            checked_retained_config_input_len([
                TLS_SERVER_NAME_MAX_BYTES + 1,
                DATABASE_MAX_BYTES,
                USERNAME_MAX_BYTES,
                PASSWORD_MAX_BYTES,
                ROOT_CA_PEM_MAX_BYTES,
            ]),
            Ok(value) if value == RETAINED_CONFIG_MAX_BYTES + 1
        ));
        assert!(matches!(
            validate_retained_config_input_lengths([
                TLS_SERVER_NAME_MAX_BYTES + 1,
                DATABASE_MAX_BYTES,
                USERNAME_MAX_BYTES,
                PASSWORD_MAX_BYTES,
                ROOT_CA_PEM_MAX_BYTES,
            ]),
            Err(DurabilityError::RootUnavailable)
        ));
        assert!(matches!(
            checked_retained_config_input_len([usize::MAX, 1, 0, 0, 0]),
            Err(DurabilityError::RootUnavailable)
        ));
    }

    #[test]
    fn retained_configuration_enforces_dns_and_secret_grammar() {
        let ip = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7));
        let ca = pem_block(32, "\n");
        for tls in [
            format!("{}.example", "a".repeat(64)),
            "db.example.".to_owned(),
            "-db.example".to_owned(),
            "db_.example".to_owned(),
            "d\u{00e9}b.example".to_owned(),
        ] {
            assert!(matches!(
                DurabilityRootConfig::new(ip, 5432, &tls, "d", "u", "p", &ca),
                Err(DurabilityError::RootUnavailable)
            ));
        }
        assert!(matches!(
            DurabilityRootConfig::new(ip, 5432, "db.example", "d", "u", "", &ca),
            Err(DurabilityError::InvalidConfiguration)
        ));
    }

    #[test]
    fn retained_root_ca_accepts_lf_and_crlf_canonical_bundles() {
        let ip = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7));
        for newline in ["\n", "\r\n"] {
            let ca = pem_bundle(ROOT_CA_MAX_CERTIFICATES, ROOT_CA_MAX_DER_BYTES, newline);
            assert!(
                DurabilityRootConfig::new_with_isolated_test_ledger(
                    ip,
                    5432,
                    "db.example",
                    "d",
                    "u",
                    "p",
                    &ca
                )
                .is_ok()
            );
        }
    }

    #[test]
    fn retained_root_ca_rejects_count_der_aggregate_and_noncanonical_text() {
        let ip = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7));
        let fifth_root = pem_bundle(ROOT_CA_MAX_CERTIFICATES + 1, 32, "\n");
        let oversized_der = pem_block(ROOT_CA_MAX_DER_BYTES + 1, "\n");
        let malformed =
            b"unrelated\n-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n";
        let non_ascii = [0xff_u8];
        let mut mixed_newline = pem_block(32, "\r\n");
        mixed_newline
            .extend_from_slice(b"-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n");

        for ca in [
            fifth_root.as_slice(),
            oversized_der.as_slice(),
            malformed.as_slice(),
            non_ascii.as_slice(),
            mixed_newline.as_slice(),
        ] {
            assert!(matches!(
                DurabilityRootConfig::new(ip, 5432, "db.example", "d", "u", "p", ca),
                Err(DurabilityError::RootUnavailable)
            ));
        }
        assert!(matches!(
            checked_root_der_total(ROOT_CA_MAX_AGGREGATE_DER_BYTES, 1),
            Err(DurabilityError::RootUnavailable)
        ));
    }

    #[test]
    fn selected_root_profile_is_strict_lazy_and_max_one() -> Result<(), Box<dyn std::error::Error>>
    {
        let config = config()?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        runtime.block_on(async {
            let (pool, _root_i_reservation) = build_root_pool(config);
            let options = pool.connect_options();

            assert_eq!(DB_PASS_DEADLINE, Duration::from_secs(2));
            assert_eq!(ROOT_RECOVERY_WINDOW, Duration::from_secs(5));
            assert_eq!(pool.options().get_max_connections(), 1);
            assert_eq!(pool.options().get_min_connections(), 0);
            assert_eq!(pool.options().get_acquire_timeout(), ROOT_RECOVERY_WINDOW);
            assert_eq!(pool.options().get_idle_timeout(), Some(HOLDER_IDLE_TIMEOUT));
            assert_eq!(pool.options().get_max_lifetime(), Some(HOLDER_MAX_LIFETIME));
            assert_eq!(options.get_host(), "db.example");
            assert_eq!(options.get_host_addr(), Some("203.0.113.7"));
            assert_eq!(options.get_port(), 5432);
            assert_eq!(options.get_username(), "explicit");
            assert_eq!(options.get_database(), Some("oteryn"));
            assert!(matches!(options.get_ssl_mode(), PgSslMode::VerifyFull));
            assert!(matches!(
                options.get_authentication_policy(),
                PgAuthenticationPolicy::ScramSha256
            ));
            assert!(options.get_ssl_tls13_only());
            assert!(!options.get_ssl_session_resumption());
            assert!(!options.get_ssl_use_default_roots());
            assert!(!options.has_ssl_client_auth());
            assert_eq!(pool.size(), 0);
            assert_eq!(pool.num_idle(), 0);
        });
        Ok(())
    }

    #[test]
    fn dedicated_runtime_representation_is_complete_for_frozen_topology()
    -> Result<(), Box<dyn std::error::Error>> {
        let profile = tokio::runtime::oteryn_wp3_dedicated_runtime_allocation_profile(
            WP3_RUNTIME_WORKER_THREADS,
            WP3_RUNTIME_MAX_BLOCKING_THREADS,
            WP3_RUNTIME_WORKER_STACK_BYTES,
        )
        .ok_or("missing WP3 runtime profile")?;

        assert_eq!(profile.scheduler_worker_count, 1);
        assert_eq!(profile.blocking_pool_thread_cap, 2);
        assert_eq!(profile.additional_blocking_worker_limit, 1);
        assert_eq!(profile.worker_stack_request, 2 * 1024 * 1024);
        assert_eq!(
            profile.blocking_pool_thread_cap,
            profile.scheduler_worker_count + profile.additional_blocking_worker_limit
        );
        assert_eq!(
            checked_runtime_stack_reservation(
                profile.worker_stack_request,
                profile.blocking_pool_thread_cap,
            )?,
            4 * 1024 * 1024
        );
        assert!(matches!(
            checked_runtime_stack_reservation(usize::MAX, 2),
            Err(DurabilityError::RootUnavailable)
        ));
        #[cfg(windows)]
        assert_eq!(
            profile
                .heap_requests
                .into_iter()
                .filter(|request| *request != 0)
                .count(),
            21
        );
        #[cfg(not(windows))]
        {
            assert_eq!(profile.heap_requests[17], 0);
            assert_eq!(
                profile
                    .heap_requests
                    .into_iter()
                    .filter(|request| *request != 0)
                    .count(),
                20
            );
        }
        Ok(())
    }

    #[test]
    fn retained_config_is_reserved_against_same_root_i_for_its_full_lifetime()
    -> Result<(), Box<dyn std::error::Error>> {
        let ip = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7));
        let ca = pem_block(32, "\n");
        let lengths = [
            "db.example".len(),
            "oteryn".len(),
            "explicit".len(),
            "test-secret".len(),
            ca.len(),
        ];
        let required = root_i_reservation_bytes(lengths)?;
        assert!(required < ROOT_TOTAL_RESIDENT_BYTES);

        let denied = Arc::new(RootResidentLedger::new(required - 1));
        assert!(matches!(
            DurabilityRootConfig::new_with_ledger(
                ip,
                5432,
                "db.example",
                "oteryn",
                "explicit",
                "test-secret",
                &ca,
                RootLedgerHandle::Test(denied.clone()),
            ),
            Err(DurabilityError::RootUnavailable)
        ));
        assert_eq!(denied.used(), 0);

        let ledger = Arc::new(RootResidentLedger::new(required));
        let config = DurabilityRootConfig::new_with_ledger(
            ip,
            5432,
            "db.example",
            "oteryn",
            "explicit",
            "test-secret",
            &ca,
            RootLedgerHandle::Test(ledger.clone()),
        )?;
        assert_eq!(config.root_i_reservation.bytes(), required);
        assert_eq!(ledger.used(), required);

        let root = DurabilityRoot::new(config)?;
        let other_owner = root.clone();
        assert_eq!(ledger.used(), required);
        drop(root);
        assert_eq!(ledger.used(), required);
        drop(other_owner);
        assert_eq!(ledger.used(), 0);
        Ok(())
    }

    #[test]
    fn no_demand_maintenance_is_a_noop_and_never_connects() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = DurabilityRoot::new(config()?)?;
        root.ready_demand.store(false, Ordering::Release);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        runtime.block_on(async {
            let maintained = root.maintain_ready_once().await?;
            assert!(!maintained);
            assert!(!root.is_ready());
            assert!(!root.has_ready_demand());
            assert_eq!(root.pool.size(), 0);
            Ok::<(), DurabilityError>(())
        })?;
        drop(root);
        Ok(())
    }
}

#[cfg(test)]
mod runtime_scope_identity_red_tests {
    use crate::durability::{AdmissionReconnectJournalV2, MigrationExecutor};
    use oteryn_game_server::foundation::{
        AccountPresenceClaimV1, AuthenticatedTransportRefV1, AuthorityEvidenceFenceV1, ChannelId,
        CharacterId, CharacterLease, CharacterWorldEligibilityClaimV1, CommandId,
        ConnectionGeneration, ControlLossEpochRefV1, Fnd02ReconciliationFenceV1,
        FreshAdmissionCommit, FreshAdmissionFacts, GameSessionAuthoritySnapshot, GameSessionId,
        GameSessionState, ProtectionEntitlementV1, ReconnectAttemptBudgetV1, ReconnectAttemptRef,
        ReconnectAuthorityFenceV1, ReconnectCandidateBindingV1, ReconnectCommitDispositionV1,
        ReconnectCompatibilityEvidenceV1, ReconnectConnectionFenceV1, ReconnectContinuityV1,
        ReconnectCurrentAuthorityV1, ReconnectDurabilityErrorV1, ReconnectDurabilityFlowV1,
        ReconnectDurabilityFlowV2, ReconnectDurabilityRecordV1, ReconnectDurableOutcomeV2,
        ReconnectDurableReconciliationSnapshotV1, ReconnectDurableReconciliationSnapshotV2,
        ReconnectDurableTerminalDispositionV1, ReconnectIdentityV1, ReconnectPrepareCompletionV1,
        ReconnectPrepareCompletionV2, ReconnectPrepareDispositionV1, ReconnectPrepareDispositionV2,
        ReconnectProjectionDecisionV2, ReconnectProofV1, RuntimeScopeRefV1,
        ScopeOwnershipGeneration, TerminalGameSessionReplacementAuthorizationV1, WorldId,
    };
    use sqlx::{Connection, Executor, PgConnection};
    use std::error::Error;
    use std::future::Future;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    const ACCOUNT: &str = "123e4567-e89b-12d3-a456-426614174000";
    static DB_SEQUENCE: AtomicU64 = AtomicU64::new(0);
    type TestResult = Result<(), Box<dyn Error>>;

    fn exact_current_authority(
        record: &ReconnectDurabilityRecordV1,
        observed_at: i64,
    ) -> Result<ReconnectCurrentAuthorityV1, ReconnectDurabilityErrorV1> {
        ReconnectCurrentAuthorityV1::from_current_facts(
            record,
            Some(AccountPresenceClaimV1::new(
                record.identity().account_id(),
                record.identity().character_id(),
            )?),
            Some(CharacterWorldEligibilityClaimV1::new(
                record.identity().character_id(),
                record.identity().world_id(),
            )),
            Some(ReconnectCandidateBindingV1::new(
                record.identity().game_session_id(),
                record.identity().reconnect_attempt_ref(),
                record.connection().candidate(),
                record.connection().transport_ref(),
                record.continuity().prepared_deadline(),
            )?),
            record.identity().runtime_scope(),
            record.connection().predecessor(),
            record.authority(),
            record.continuity().control_loss_epoch(),
            record.continuity().original_grace_deadline(),
            record.proof().clone(),
            record.fnd02().clone(),
            record.compatibility().clone(),
            GameSessionState::Reconnectable,
            false,
            observed_at,
        )
    }

    struct IsolatedDatabase {
        admin_url: String,
        database_name: String,
    }

    impl IsolatedDatabase {
        async fn create(test_name: &str) -> Result<Self, Box<dyn Error>> {
            let admin_url = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL")?;
            if !(admin_url.starts_with("postgresql://oteryn_test_admin:")
                && (admin_url.contains("@127.0.0.1:5432/")
                    || admin_url.contains("@localhost:5432/"))
                && admin_url.ends_with("/postgres")
                && !admin_url.contains(['?', '#', '\n', '\r']))
            {
                return Err("unsafe PostgreSQL test-admin URL".into());
            }
            let ordinal = DB_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let normalized: String = test_name
                .bytes()
                .map(|byte| {
                    if byte.is_ascii_alphanumeric() {
                        char::from(byte.to_ascii_lowercase())
                    } else {
                        '_'
                    }
                })
                .collect();
            let database_name = format!(
                "oteryn_game_review_{}_{}_{}",
                std::process::id(),
                ordinal,
                normalized
            );
            let mut admin = PgConnection::connect(&admin_url).await?;
            admin
                .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                    "CREATE DATABASE {database_name}"
                ))))
                .await?;
            Ok(Self {
                admin_url,
                database_name,
            })
        }

        fn database_url(&self) -> Result<String, Box<dyn Error>> {
            let prefix = self
                .admin_url
                .strip_suffix("/postgres")
                .ok_or("invalid PostgreSQL test-admin URL")?;
            Ok(format!("{prefix}/{}", self.database_name))
        }

        async fn cleanup(self) -> Result<(), Box<dyn Error>> {
            let mut admin = PgConnection::connect(&self.admin_url).await?;
            sqlx::query(
                "SELECT pg_terminate_backend(pid) FROM pg_stat_activity \
                 WHERE datname = $1 AND pid <> pg_backend_pid()",
            )
            .bind(&self.database_name)
            .execute(&mut admin)
            .await?;
            admin
                .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                    "DROP DATABASE IF EXISTS {}",
                    self.database_name
                ))))
                .await?;
            Ok(())
        }
    }

    fn run_postgres_test<F>(future: F) -> TestResult
    where
        F: Future<Output = TestResult>,
    {
        if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
            return Ok(());
        }
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(future)
    }

    async fn migrated_database(
        test_name: &str,
    ) -> Result<(IsolatedDatabase, String), Box<dyn Error>> {
        let database = IsolatedDatabase::create(test_name).await?;
        let database_url = database.database_url()?;
        let executor = MigrationExecutor::connect_migration(&database_url).await?;
        executor.apply_embedded_ledger().await?;
        Ok((database, database_url))
    }

    fn unix_now() -> Result<i64, Box<dyn Error>> {
        Ok(SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs()
            .try_into()?)
    }

    fn uuid_v7(raw: u64) -> [u8; 16] {
        let mut value = [0_u8; 16];
        value[8..].copy_from_slice(&raw.to_be_bytes());
        value[6] = 0x70;
        value[8] = (value[8] & 0x3f) | 0x80;
        value
    }

    fn game_session(raw: u64) -> Result<GameSessionId, ReconnectDurabilityErrorV1> {
        GameSessionId::decode(&uuid_v7(raw)).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)
    }

    fn character(raw: u64) -> Result<CharacterId, ReconnectDurabilityErrorV1> {
        CharacterId::decode(&uuid_v7(raw)).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)
    }

    fn world(raw: u64) -> Result<WorldId, ReconnectDurabilityErrorV1> {
        WorldId::decode(&uuid_v7(raw)).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)
    }

    fn channel(raw: u64) -> Result<ChannelId, ReconnectDurabilityErrorV1> {
        ChannelId::decode(&uuid_v7(raw)).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)
    }

    fn candidate_record() -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
        candidate_record_for_channel(13)
    }

    fn candidate_record_for_channel(
        channel_raw: u64,
    ) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
        let world_id = world(12)?;
        let identity = ReconnectIdentityV1::new(
            game_session(20)?,
            ReconnectAttemptRef::new(1).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            ACCOUNT,
            character(11)?,
            world_id,
            RuntimeScopeRefV1::channel(world_id, channel(channel_raw)?),
        )?;
        let connection = ReconnectConnectionFenceV1::new(
            ConnectionGeneration::new(7).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            ConnectionGeneration::new(8).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            AuthenticatedTransportRefV1::decode(&[0x71; 16])
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        )?;
        let authority = ReconnectAuthorityFenceV1::new(
            9,
            ScopeOwnershipGeneration::new(10)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        )?;
        let continuity = ReconnectContinuityV1::new(
            ControlLossEpochRefV1::new(3)?,
            120,
            115,
            ProtectionEntitlementV1::unused(),
        )?;
        let fnd02 = Fnd02ReconciliationFenceV1::new(
            CommandId::new(3).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            vec![],
            41,
            vec![],
        )?;
        let platform = AuthorityEvidenceFenceV1::new(
            "platform-security",
            "reconnect",
            "account",
            "sec:17",
            "decision:sec:17",
            100,
        )?;
        let trust = AuthorityEvidenceFenceV1::new(
            "proof-trust",
            "reconnect",
            "recovery-key",
            "trust:21",
            "decision:trust:21",
            101,
        )?;
        let compatibility = ReconnectCompatibilityEvidenceV1::new(
            1,
            1,
            "rules:1",
            "content:2",
            "map:3",
            "world:4",
            12,
            platform,
            trust,
            Some(110),
        )?;
        ReconnectDurabilityRecordV1::new(
            identity,
            connection,
            authority,
            continuity,
            ReconnectProofV1::ReauthenticatedRecovery {
                recovery_grant_nonce: [0x55; 32],
            },
            fnd02,
            compatibility,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn postgres_record(
        now: i64,
        session_raw: u64,
        attempt_raw: u64,
        transport_byte: u8,
        predecessor_generation: u64,
        protection_entitlement: ProtectionEntitlementV1,
    ) -> Result<ReconnectDurabilityRecordV1, ReconnectDurabilityErrorV1> {
        let world_id = world(12)?;
        let identity = ReconnectIdentityV1::new(
            game_session(session_raw)?,
            ReconnectAttemptRef::new(attempt_raw)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            ACCOUNT,
            character(11)?,
            world_id,
            RuntimeScopeRefV1::channel(world_id, channel(13)?),
        )?;
        let connection = ReconnectConnectionFenceV1::new(
            ConnectionGeneration::new(predecessor_generation)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            ConnectionGeneration::new(predecessor_generation + 1)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            AuthenticatedTransportRefV1::decode(&[transport_byte; 16])
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        )?;
        let authority = ReconnectAuthorityFenceV1::new(
            9,
            ScopeOwnershipGeneration::new(10)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        )?;
        let continuity = ReconnectContinuityV1::new(
            ControlLossEpochRefV1::new(3)?,
            now + 120,
            now + 115,
            protection_entitlement,
        )?;
        let fnd02 = Fnd02ReconciliationFenceV1::new(
            CommandId::new(3).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            vec![],
            41,
            vec![],
        )?;
        let platform = AuthorityEvidenceFenceV1::new(
            "platform-security",
            "reconnect",
            "account",
            "sec:17",
            "decision:sec:17",
            now,
        )?;
        let trust = AuthorityEvidenceFenceV1::new(
            "proof-trust",
            "reconnect",
            "recovery-key",
            "trust:21",
            "decision:trust:21",
            now,
        )?;
        let compatibility = ReconnectCompatibilityEvidenceV1::new(
            1,
            1,
            "rules:1",
            "content:2",
            "map:3",
            "world:4",
            12,
            platform,
            trust,
            Some(now + 110),
        )?;
        ReconnectDurabilityRecordV1::new(
            identity,
            connection,
            authority,
            continuity,
            ReconnectProofV1::ReauthenticatedRecovery {
                recovery_grant_nonce: [transport_byte; 32],
            },
            fnd02,
            compatibility,
        )
    }

    fn replacement_authorization(
        candidate: &ReconnectDurabilityRecordV1,
        predecessor_raw: u64,
        current_connection_generation: u64,
    ) -> Result<TerminalGameSessionReplacementAuthorizationV1, ReconnectDurabilityErrorV1> {
        let facts =
            FreshAdmissionFacts::new([0x44; 32], character(11)?, world(12)?, channel(13)?, 9, 10)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let initial_transport = AuthenticatedTransportRefV1::decode(&[0x70; 16])
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let predecessor = game_session(predecessor_raw)?;
        let commit = FreshAdmissionCommit::from_facts(predecessor, facts, initial_transport)
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let snapshot = GameSessionAuthoritySnapshot::from_current_facts(
            commit,
            GameSessionState::Terminal,
            ConnectionGeneration::new(current_connection_generation)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            None,
            CharacterLease::new(character(11)?, 9)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            Some(CharacterWorldEligibilityClaimV1::new(
                character(11)?,
                world(12)?,
            )),
            RuntimeScopeRefV1::channel(world(12)?, channel(13)?),
            ScopeOwnershipGeneration::new(10)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        )?
        .with_control_loss_continuity(
            candidate.continuity().control_loss_epoch(),
            candidate.continuity().original_grace_deadline(),
        )?;
        TerminalGameSessionReplacementAuthorizationV1::from_current_authority(
            ACCOUNT,
            Some(&AccountPresenceClaimV1::new(ACCOUNT, character(11)?)?),
            predecessor,
            candidate.identity().game_session_id(),
            snapshot,
            candidate,
        )
    }

    async fn seed_current_actor_anchor(
        database_url: &str,
        session_raw: u64,
        now: i64,
    ) -> Result<(), Box<dyn Error>> {
        let mut connection = PgConnection::connect(database_url).await?;
        sqlx::query(
            "INSERT INTO game_durability_reconnect_sessions (\
                game_session_id, account_id, character_id, world_id, runtime_scope_kind, \
                runtime_scope_world_id, runtime_scope_channel_id, runtime_scope_instance_id, \
                control_loss_epoch, original_grace_deadline, predecessor_generation, \
                character_lease_generation, scope_ownership_generation, current_generation, \
                session_state\
             ) VALUES (\
                encode($1, 'hex')::uuid, $2::text::uuid, encode($3, 'hex')::uuid, \
                encode($4, 'hex')::uuid, 1, encode($4, 'hex')::uuid, \
                encode($5, 'hex')::uuid, NULL, 3, $6, 7, 9, 10, 7, 1\
             )",
        )
        .bind(uuid_v7(session_raw).as_slice())
        .bind(ACCOUNT)
        .bind(uuid_v7(11).as_slice())
        .bind(uuid_v7(12).as_slice())
        .bind(uuid_v7(13).as_slice())
        .bind(now + 120)
        .execute(&mut connection)
        .await?;
        sqlx::query(
            "INSERT INTO game_durability_control_loss_continuity (\
                character_id, control_loss_epoch, account_id, world_id, context_game_session_id, \
                original_grace_deadline, protection_entitlement_state, protection_rearm_state\
             ) VALUES (encode($1, 'hex')::uuid, 3, $2::text::uuid, encode($3, 'hex')::uuid, \
                       encode($4, 'hex')::uuid, $5, 1, 1)",
        )
        .bind(uuid_v7(11).as_slice())
        .bind(ACCOUNT)
        .bind(uuid_v7(12).as_slice())
        .bind(uuid_v7(session_raw).as_slice())
        .bind(now + 120)
        .execute(&mut connection)
        .await?;
        // Independent complete history for this positive owning fixture.
        sqlx::query("INSERT INTO game_durability_session_use_ledgers VALUES (encode($1,'hex')::uuid,1,TRUE,1,1)")
            .bind(uuid_v7(11).as_slice()).execute(&mut connection).await?;
        sqlx::query("INSERT INTO game_durability_session_use_memberships VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,$3)")
            .bind(uuid_v7(session_raw).as_slice()).bind(uuid_v7(11).as_slice()).bind([1_u8;16].as_slice()).execute(&mut connection).await?;
        connection.close().await?;
        Ok(())
    }

    fn predecessor_snapshot(
        current_channel_raw: u64,
    ) -> Result<GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>, ReconnectDurabilityErrorV1>
    {
        predecessor_snapshot_with_world_eligibility(
            current_channel_raw,
            Some(CharacterWorldEligibilityClaimV1::new(
                character(11)?,
                world(12)?,
            )),
        )
    }

    fn predecessor_snapshot_with_world_eligibility(
        current_channel_raw: u64,
        current_character_world_eligibility: Option<CharacterWorldEligibilityClaimV1>,
    ) -> Result<GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>, ReconnectDurabilityErrorV1>
    {
        let facts =
            FreshAdmissionFacts::new([0x44; 32], character(11)?, world(12)?, channel(13)?, 9, 10)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let initial_transport = AuthenticatedTransportRefV1::decode(&[0x70; 16])
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        let commit = FreshAdmissionCommit::from_facts(game_session(10)?, facts, initial_transport)
            .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?;
        GameSessionAuthoritySnapshot::from_current_facts(
            commit,
            GameSessionState::Terminal,
            ConnectionGeneration::new(7).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            None,
            CharacterLease::new(character(11)?, 9)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            current_character_world_eligibility,
            RuntimeScopeRefV1::channel(world(12)?, channel(current_channel_raw)?),
            ScopeOwnershipGeneration::new(10)
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
        )?
        .with_control_loss_continuity(ControlLossEpochRefV1::new(3)?, 120)
    }

    fn prepared_flow(
        record: &ReconnectDurabilityRecordV1,
    ) -> Result<ReconnectDurabilityFlowV2, ReconnectDurabilityErrorV1> {
        let mut budget = ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
        budget.reserve(
            record.identity().reconnect_attempt_ref(),
            record.connection().transport_ref(),
        )?;
        let (mut flow, request) = ReconnectDurabilityFlowV2::begin(record.clone(), None);
        flow.accept_prepare_completion(
            ReconnectPrepareCompletionV2::for_request(
                &request,
                ReconnectPrepareDispositionV2::Prepared,
            ),
            &mut budget,
        )?;
        Ok(flow)
    }

    fn current_authority(
        record: &ReconnectDurabilityRecordV1,
        runtime_scope: RuntimeScopeRefV1,
        connection_generation: ConnectionGeneration,
        control_loss_epoch: ControlLossEpochRefV1,
        proof: ReconnectProofV1,
    ) -> Result<ReconnectCurrentAuthorityV1, ReconnectDurabilityErrorV1> {
        current_authority_with_bindings(
            record,
            Some(CharacterWorldEligibilityClaimV1::new(
                record.identity().character_id(),
                record.identity().world_id(),
            )),
            Some(ReconnectCandidateBindingV1::new(
                record.identity().game_session_id(),
                record.identity().reconnect_attempt_ref(),
                record.connection().candidate(),
                record.connection().transport_ref(),
                record.continuity().prepared_deadline(),
            )?),
            runtime_scope,
            connection_generation,
            control_loss_epoch,
            proof,
            105,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn current_authority_with_bindings(
        record: &ReconnectDurabilityRecordV1,
        current_character_world_eligibility: Option<CharacterWorldEligibilityClaimV1>,
        current_candidate: Option<ReconnectCandidateBindingV1>,
        runtime_scope: RuntimeScopeRefV1,
        connection_generation: ConnectionGeneration,
        control_loss_epoch: ControlLossEpochRefV1,
        proof: ReconnectProofV1,
        observed_at: i64,
    ) -> Result<ReconnectCurrentAuthorityV1, ReconnectDurabilityErrorV1> {
        ReconnectCurrentAuthorityV1::from_current_facts(
            record,
            Some(AccountPresenceClaimV1::new(
                record.identity().account_id(),
                record.identity().character_id(),
            )?),
            current_character_world_eligibility,
            current_candidate,
            runtime_scope,
            connection_generation,
            record.authority(),
            control_loss_epoch,
            record.continuity().original_grace_deadline(),
            proof,
            record.fnd02().clone(),
            record.compatibility().clone(),
            GameSessionState::Reconnectable,
            false,
            observed_at,
        )
    }

    #[test]
    fn terminal_replacement_requires_independent_current_character_world_eligibility()
    -> Result<(), ReconnectDurabilityErrorV1> {
        let candidate = candidate_record()?;
        assert!(
            TerminalGameSessionReplacementAuthorizationV1::from_current_authority(
                ACCOUNT,
                Some(&AccountPresenceClaimV1::new(ACCOUNT, character(11)?)?),
                game_session(10)?,
                game_session(20)?,
                predecessor_snapshot_with_world_eligibility(
                    13,
                    Some(CharacterWorldEligibilityClaimV1::new(
                        character(11)?,
                        world(12)?,
                    )),
                )?,
                &candidate,
            )
            .is_ok()
        );

        for current_eligibility in [
            None,
            Some(CharacterWorldEligibilityClaimV1::new(
                character(11)?,
                world(99)?,
            )),
            Some(CharacterWorldEligibilityClaimV1::new(
                character(99)?,
                world(12)?,
            )),
        ] {
            assert_eq!(
                TerminalGameSessionReplacementAuthorizationV1::from_current_authority(
                    ACCOUNT,
                    Some(&AccountPresenceClaimV1::new(ACCOUNT, character(11)?)?),
                    game_session(10)?,
                    game_session(20)?,
                    predecessor_snapshot_with_world_eligibility(13, current_eligibility)?,
                    &candidate,
                ),
                Err(ReconnectDurabilityErrorV1::StaleAuthority)
            );
        }
        Ok(())
    }

    #[test]
    fn terminal_replacement_authorization_requires_current_account_presence_at_prepare()
    -> Result<(), ReconnectDurabilityErrorV1> {
        let candidate = candidate_record()?;
        let exact_presence = AccountPresenceClaimV1::new(ACCOUNT, character(11)?)?;
        let reassigned_presence = AccountPresenceClaimV1::new(ACCOUNT, character(99)?)?;

        assert!(
            TerminalGameSessionReplacementAuthorizationV1::from_current_authority(
                ACCOUNT,
                Some(&exact_presence),
                game_session(10)?,
                game_session(20)?,
                predecessor_snapshot(13)?,
                &candidate,
            )
            .is_ok()
        );

        for current_presence in [None, Some(&reassigned_presence)] {
            assert_eq!(
                TerminalGameSessionReplacementAuthorizationV1::from_current_authority(
                    ACCOUNT,
                    current_presence,
                    game_session(10)?,
                    game_session(20)?,
                    predecessor_snapshot(13)?,
                    &candidate,
                ),
                Err(ReconnectDurabilityErrorV1::StaleAuthority)
            );
        }
        Ok(())
    }

    #[test]
    fn v2_final_revalidation_requires_current_world_and_live_candidate_binding()
    -> Result<(), ReconnectDurabilityErrorV1> {
        let record = candidate_record()?;
        let exact_world = Some(CharacterWorldEligibilityClaimV1::new(
            record.identity().character_id(),
            record.identity().world_id(),
        ));
        let exact_candidate = ReconnectCandidateBindingV1::new(
            record.identity().game_session_id(),
            record.identity().reconnect_attempt_ref(),
            record.connection().candidate(),
            record.connection().transport_ref(),
            record.continuity().prepared_deadline(),
        )?;
        let exact_scope = record.identity().runtime_scope();
        let exact_connection = record.connection().predecessor();
        let exact_epoch = record.continuity().control_loss_epoch();
        let exact_proof = record.proof().clone();

        let mut exact_flow = prepared_flow(&record)?;
        assert!(
            exact_flow
                .authorize_commit(
                    current_authority_with_bindings(
                        &record,
                        exact_world,
                        Some(exact_candidate),
                        exact_scope,
                        exact_connection,
                        exact_epoch,
                        exact_proof.clone(),
                        105,
                    )?,
                    104,
                )
                .is_ok()
        );

        let mut missing_world_flow = prepared_flow(&record)?;
        assert_eq!(
            missing_world_flow.authorize_commit(
                current_authority_with_bindings(
                    &record,
                    None,
                    Some(exact_candidate),
                    exact_scope,
                    exact_connection,
                    exact_epoch,
                    exact_proof.clone(),
                    105,
                )?,
                104,
            ),
            Err(ReconnectDurabilityErrorV1::StaleAuthority)
        );

        let mut reassigned_world_flow = prepared_flow(&record)?;
        assert_eq!(
            reassigned_world_flow.authorize_commit(
                current_authority_with_bindings(
                    &record,
                    Some(CharacterWorldEligibilityClaimV1::new(
                        character(11)?,
                        world(99)?,
                    )),
                    Some(exact_candidate),
                    exact_scope,
                    exact_connection,
                    exact_epoch,
                    exact_proof.clone(),
                    105,
                )?,
                104,
            ),
            Err(ReconnectDurabilityErrorV1::StaleAuthority)
        );

        let mut missing_candidate_flow = prepared_flow(&record)?;
        assert_eq!(
            missing_candidate_flow.authorize_commit(
                current_authority_with_bindings(
                    &record,
                    exact_world,
                    None,
                    exact_scope,
                    exact_connection,
                    exact_epoch,
                    exact_proof.clone(),
                    105,
                )?,
                104,
            ),
            Err(ReconnectDurabilityErrorV1::StaleAuthority)
        );

        let rebound_candidate = ReconnectCandidateBindingV1::new(
            record.identity().game_session_id(),
            record.identity().reconnect_attempt_ref(),
            record.connection().candidate(),
            AuthenticatedTransportRefV1::decode(&[0x72; 16])
                .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            record.continuity().prepared_deadline(),
        )?;
        let mut rebound_candidate_flow = prepared_flow(&record)?;
        assert_eq!(
            rebound_candidate_flow.authorize_commit(
                current_authority_with_bindings(
                    &record,
                    exact_world,
                    Some(rebound_candidate),
                    exact_scope,
                    exact_connection,
                    exact_epoch,
                    exact_proof.clone(),
                    105,
                )?,
                104,
            ),
            Err(ReconnectDurabilityErrorV1::StaleAuthority)
        );

        let changed_attempt_candidate = ReconnectCandidateBindingV1::new(
            record.identity().game_session_id(),
            ReconnectAttemptRef::new(2).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            record.connection().candidate(),
            record.connection().transport_ref(),
            record.continuity().prepared_deadline(),
        )?;
        let mut changed_attempt_flow = prepared_flow(&record)?;
        assert_eq!(
            changed_attempt_flow.authorize_commit(
                current_authority_with_bindings(
                    &record,
                    exact_world,
                    Some(changed_attempt_candidate),
                    exact_scope,
                    exact_connection,
                    exact_epoch,
                    exact_proof,
                    105,
                )?,
                104,
            ),
            Err(ReconnectDurabilityErrorV1::StaleAuthority)
        );
        Ok(())
    }

    #[test]
    fn v2_committed_reconciliation_requires_current_world_and_live_candidate_binding()
    -> Result<(), ReconnectDurabilityErrorV1> {
        let record = candidate_record()?;
        let snapshot = ReconnectDurableReconciliationSnapshotV2::new(
            record.clone(),
            ReconnectDurableOutcomeV2::Committed {
                current_generation: record.connection().candidate(),
                current_transport_ref: record.connection().transport_ref(),
            },
        );
        let reconcile =
            |current_character_world_eligibility: Option<CharacterWorldEligibilityClaimV1>,
             current_candidate: Option<ReconnectCandidateBindingV1>| {
                let mut budget =
                    ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
                budget.reserve(
                    record.identity().reconnect_attempt_ref(),
                    record.connection().transport_ref(),
                )?;
                let (mut flow, request) = ReconnectDurabilityFlowV2::begin(record.clone(), None);
                flow.accept_prepare_completion(
                    ReconnectPrepareCompletionV2::for_request(
                        &request,
                        ReconnectPrepareDispositionV2::Ambiguous,
                    ),
                    &mut budget,
                )?;
                flow.accept_reconciliation(
                    snapshot.clone(),
                    current_authority_with_bindings(
                        &record,
                        current_character_world_eligibility,
                        current_candidate,
                        record.identity().runtime_scope(),
                        record.connection().predecessor(),
                        record.continuity().control_loss_epoch(),
                        record.proof().clone(),
                        105,
                    )?,
                    &mut budget,
                )
            };

        let exact_world = Some(CharacterWorldEligibilityClaimV1::new(
            record.identity().character_id(),
            record.identity().world_id(),
        ));
        let exact_candidate = Some(ReconnectCandidateBindingV1::new(
            record.identity().game_session_id(),
            record.identity().reconnect_attempt_ref(),
            record.connection().candidate(),
            record.connection().transport_ref(),
            record.continuity().prepared_deadline(),
        )?);
        assert!(matches!(
            reconcile(exact_world, exact_candidate)?,
            ReconnectProjectionDecisionV2::InstallController { .. }
        ));
        assert_eq!(
            reconcile(None, exact_candidate),
            Err(ReconnectDurabilityErrorV1::ReconciliationMismatch)
        );
        assert_eq!(
            reconcile(exact_world, None),
            Err(ReconnectDurabilityErrorV1::ReconciliationMismatch)
        );
        Ok(())
    }

    #[test]
    fn v2_terminal_reconciliation_survives_scope_advance_but_committed_stays_fenced()
    -> Result<(), ReconnectDurabilityErrorV1> {
        let record = candidate_record()?;
        let advanced_authority = ReconnectCurrentAuthorityV1::from_current_facts(
            &record,
            Some(AccountPresenceClaimV1::new(
                record.identity().account_id(),
                record.identity().character_id(),
            )?),
            Some(CharacterWorldEligibilityClaimV1::new(
                record.identity().character_id(),
                record.identity().world_id(),
            )),
            Some(ReconnectCandidateBindingV1::new(
                record.identity().game_session_id(),
                record.identity().reconnect_attempt_ref(),
                record.connection().candidate(),
                record.connection().transport_ref(),
                record.continuity().prepared_deadline(),
            )?),
            record.identity().runtime_scope(),
            record.connection().predecessor(),
            ReconnectAuthorityFenceV1::new(
                record.authority().character_lease_generation(),
                ScopeOwnershipGeneration::new(11)
                    .map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            )?,
            record.continuity().control_loss_epoch(),
            record.continuity().original_grace_deadline(),
            record.proof().clone(),
            record.fnd02().clone(),
            record.compatibility().clone(),
            GameSessionState::Reconnectable,
            false,
            105,
        )?;

        let reconciliation_flow = || {
            let mut budget =
                ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
            budget.reserve(
                record.identity().reconnect_attempt_ref(),
                record.connection().transport_ref(),
            )?;
            let (mut flow, request) = ReconnectDurabilityFlowV2::begin(record.clone(), None);
            flow.accept_prepare_completion(
                ReconnectPrepareCompletionV2::for_request(
                    &request,
                    ReconnectPrepareDispositionV2::Ambiguous,
                ),
                &mut budget,
            )?;
            Ok::<_, ReconnectDurabilityErrorV1>((flow, budget))
        };

        let (mut terminal_flow, mut terminal_budget) = reconciliation_flow()?;
        let terminal_snapshot = ReconnectDurableReconciliationSnapshotV2::new(
            record.clone(),
            ReconnectDurableOutcomeV2::Terminal {
                disposition: ReconnectDurableTerminalDispositionV1::TransportRefCollision,
            },
        );
        assert_eq!(
            terminal_flow.accept_reconciliation(
                terminal_snapshot,
                advanced_authority.clone(),
                &mut terminal_budget,
            )?,
            ReconnectProjectionDecisionV2::Terminal {
                disposition: ReconnectDurableTerminalDispositionV1::TransportRefCollision,
            }
        );
        assert!(
            terminal_budget
                .replacement_allowed_after_collision(record.identity().reconnect_attempt_ref())
        );

        let (mut committed_flow, mut committed_budget) = reconciliation_flow()?;
        let committed_snapshot = ReconnectDurableReconciliationSnapshotV2::new(
            record.clone(),
            ReconnectDurableOutcomeV2::Committed {
                current_generation: record.connection().candidate(),
                current_transport_ref: record.connection().transport_ref(),
            },
        );
        assert_eq!(
            committed_flow.accept_reconciliation(
                committed_snapshot,
                advanced_authority,
                &mut committed_budget,
            ),
            Err(ReconnectDurabilityErrorV1::ReconciliationMismatch)
        );
        Ok(())
    }

    #[test]
    fn terminal_replacement_rejects_same_world_generation_different_current_runtime_scope()
    -> Result<(), ReconnectDurabilityErrorV1> {
        let candidate = candidate_record()?;
        let result = TerminalGameSessionReplacementAuthorizationV1::from_current_authority(
            ACCOUNT,
            Some(&AccountPresenceClaimV1::new(ACCOUNT, character(11)?)?),
            game_session(10)?,
            game_session(20)?,
            predecessor_snapshot(14)?,
            &candidate,
        );
        assert_eq!(result, Err(ReconnectDurabilityErrorV1::StaleAuthority));
        Ok(())
    }

    #[test]
    fn final_revalidation_requires_complete_current_authority_facts()
    -> Result<(), ReconnectDurabilityErrorV1> {
        let record = candidate_record()?;
        let exact_scope = record.identity().runtime_scope();
        let exact_connection = record.connection().predecessor();
        let exact_epoch = record.continuity().control_loss_epoch();
        let exact_proof = record.proof().clone();

        let mut exact_flow = prepared_flow(&record)?;
        let exact = current_authority(
            &record,
            exact_scope,
            exact_connection,
            exact_epoch,
            exact_proof.clone(),
        )?;
        assert!(exact_flow.authorize_commit(exact, 104).is_ok());

        let mut scope_flow = prepared_flow(&record)?;
        let changed_scope = current_authority(
            &record,
            RuntimeScopeRefV1::channel(record.identity().world_id(), channel(14)?),
            exact_connection,
            exact_epoch,
            exact_proof.clone(),
        )?;
        assert_eq!(
            scope_flow.authorize_commit(changed_scope, 104),
            Err(ReconnectDurabilityErrorV1::StaleAuthority)
        );

        let mut connection_flow = prepared_flow(&record)?;
        let changed_connection = current_authority(
            &record,
            exact_scope,
            ConnectionGeneration::new(9).map_err(|_| ReconnectDurabilityErrorV1::InvalidRecord)?,
            exact_epoch,
            exact_proof.clone(),
        )?;
        assert_eq!(
            connection_flow.authorize_commit(changed_connection, 104),
            Err(ReconnectDurabilityErrorV1::StaleAuthority)
        );

        let mut epoch_flow = prepared_flow(&record)?;
        let changed_epoch = current_authority(
            &record,
            exact_scope,
            exact_connection,
            ControlLossEpochRefV1::new(4)?,
            exact_proof,
        )?;
        assert_eq!(
            epoch_flow.authorize_commit(changed_epoch, 104),
            Err(ReconnectDurabilityErrorV1::StaleAuthority)
        );

        let mut proof_flow = prepared_flow(&record)?;
        let changed_proof = current_authority(
            &record,
            exact_scope,
            exact_connection,
            exact_epoch,
            ReconnectProofV1::ReauthenticatedRecovery {
                recovery_grant_nonce: [0x56; 32],
            },
        )?;
        assert_eq!(
            proof_flow.authorize_commit(changed_proof, 104),
            Err(ReconnectDurabilityErrorV1::StaleAuthority)
        );
        Ok(())
    }

    #[test]
    fn final_revalidation_requires_current_original_grace_deadline()
    -> Result<(), ReconnectDurabilityErrorV1> {
        let record = candidate_record()?;
        let current = |original_grace_deadline: i64| {
            ReconnectCurrentAuthorityV1::from_current_facts(
                &record,
                Some(AccountPresenceClaimV1::new(
                    record.identity().account_id(),
                    record.identity().character_id(),
                )?),
                Some(CharacterWorldEligibilityClaimV1::new(
                    record.identity().character_id(),
                    record.identity().world_id(),
                )),
                Some(ReconnectCandidateBindingV1::new(
                    record.identity().game_session_id(),
                    record.identity().reconnect_attempt_ref(),
                    record.connection().candidate(),
                    record.connection().transport_ref(),
                    record.continuity().prepared_deadline(),
                )?),
                record.identity().runtime_scope(),
                record.connection().predecessor(),
                record.authority(),
                record.continuity().control_loss_epoch(),
                original_grace_deadline,
                record.proof().clone(),
                record.fnd02().clone(),
                record.compatibility().clone(),
                GameSessionState::Reconnectable,
                false,
                105,
            )
        };

        let exact_deadline = record.continuity().original_grace_deadline();
        let mut exact_flow = prepared_flow(&record)?;
        assert!(
            exact_flow
                .authorize_commit(current(exact_deadline)?, 104)
                .is_ok()
        );

        let mut shortened_flow = prepared_flow(&record)?;
        assert_eq!(
            shortened_flow.authorize_commit(current(exact_deadline - 1)?, 104),
            Err(ReconnectDurabilityErrorV1::StaleAuthority)
        );

        let snapshot = ReconnectDurableReconciliationSnapshotV2::new(
            record.clone(),
            ReconnectDurableOutcomeV2::Committed {
                current_generation: record.connection().candidate(),
                current_transport_ref: record.connection().transport_ref(),
            },
        );
        let mut budget = ReconnectAttemptBudgetV1::new(record.continuity().control_loss_epoch());
        budget.reserve(
            record.identity().reconnect_attempt_ref(),
            record.connection().transport_ref(),
        )?;
        let (mut reconciliation_flow, request) =
            ReconnectDurabilityFlowV2::begin(record.clone(), None);
        reconciliation_flow.accept_prepare_completion(
            ReconnectPrepareCompletionV2::for_request(
                &request,
                ReconnectPrepareDispositionV2::Ambiguous,
            ),
            &mut budget,
        )?;
        assert!(matches!(
            reconciliation_flow.accept_reconciliation(
                snapshot,
                current(exact_deadline)?,
                &mut budget,
            )?,
            ReconnectProjectionDecisionV2::InstallController { .. }
        ));
        Ok(())
    }

    #[test]
    fn replacement_authorization_rejects_runtime_scope_record_substitution()
    -> Result<(), ReconnectDurabilityErrorV1> {
        let candidate = candidate_record()?;
        let authorization = TerminalGameSessionReplacementAuthorizationV1::from_current_authority(
            ACCOUNT,
            Some(&AccountPresenceClaimV1::new(ACCOUNT, character(11)?)?),
            game_session(10)?,
            game_session(20)?,
            predecessor_snapshot(13)?,
            &candidate,
        )?;
        let substituted = candidate_record_for_channel(14)?;

        assert!(
            !crate::durability::replacement_authorization_matches_record(
                &authorization,
                &substituted,
            )
        );
        Ok(())
    }

    #[test]
    fn replacement_reconciliation_requires_receipt_authorization_when_request_omits_it()
    -> TestResult {
        run_postgres_test(async {
            let (database, database_url) =
                migrated_database("unsigned_replacement_reconcile").await?;
            let now = unix_now()?;
            seed_current_actor_anchor(&database_url, 10, now).await?;
            let journal = AdmissionReconnectJournalV2::connect_runtime(&database_url).await?;
            let candidate = postgres_record(now, 20, 1, 0xa1, 7, ProtectionEntitlementV1::unused())
                .map_err(|_| "candidate record")?;
            let authorization = replacement_authorization(&candidate, 10, 7)
                .map_err(|_| "replacement authorization")?;
            let signed_request =
                ReconnectDurabilityFlowV2::begin(candidate.clone(), Some(authorization)).1;
            assert_eq!(
                journal.prepare(&signed_request).await?,
                ReconnectPrepareDispositionV2::Prepared
            );

            let unsigned_request = ReconnectDurabilityFlowV2::begin(candidate, None).1;
            assert!(matches!(
                journal.reconcile(&unsigned_request).await,
                Err(crate::durability::DurabilityError::InvalidStoredState)
            ));

            drop(journal);
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn replacement_prepare_requires_receipt_authorization_when_request_omits_it() -> TestResult {
        run_postgres_test(async {
            let (database, database_url) =
                migrated_database("unsigned_replacement_prepare").await?;
            let now = unix_now()?;
            seed_current_actor_anchor(&database_url, 10, now).await?;
            let journal = AdmissionReconnectJournalV2::connect_runtime(&database_url).await?;
            let candidate = postgres_record(now, 20, 1, 0xa2, 7, ProtectionEntitlementV1::unused())
                .map_err(|_| "candidate record")?;
            let authorization = replacement_authorization(&candidate, 10, 7)
                .map_err(|_| "replacement authorization")?;
            let signed_request =
                ReconnectDurabilityFlowV2::begin(candidate.clone(), Some(authorization)).1;
            assert_eq!(
                journal.prepare(&signed_request).await?,
                ReconnectPrepareDispositionV2::Prepared
            );

            let unsigned_request = ReconnectDurabilityFlowV2::begin(candidate, None).1;
            assert!(matches!(
                journal.prepare(&unsigned_request).await,
                Err(crate::durability::DurabilityError::InvalidStoredState)
            ));

            drop(journal);
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn legacy_prepare_rejects_receipt_backed_replacement_candidate() -> TestResult {
        run_postgres_test(async {
            let (database, database_url) = migrated_database("legacy_replacement_prepare").await?;
            let now = unix_now()?;
            seed_current_actor_anchor(&database_url, 10, now).await?;
            let journal = AdmissionReconnectJournalV2::connect_runtime(&database_url).await?;
            let candidate = postgres_record(now, 20, 1, 0xa3, 7, ProtectionEntitlementV1::unused())
                .map_err(|_| "candidate record")?;
            let authorization = replacement_authorization(&candidate, 10, 7)
                .map_err(|_| "replacement authorization")?;
            let signed_request =
                ReconnectDurabilityFlowV2::begin(candidate.clone(), Some(authorization)).1;
            assert_eq!(
                journal.prepare(&signed_request).await?,
                ReconnectPrepareDispositionV2::Prepared
            );

            let (_, legacy_request) = ReconnectDurabilityFlowV1::begin(candidate);
            assert!(matches!(
                journal.legacy().prepare(&legacy_request).await,
                Err(crate::durability::DurabilityError::InvalidStoredState)
            ));

            drop(journal);
            database.cleanup().await?;
            Ok(())
        })
    }

    #[test]
    fn terminal_replacement_preserves_committed_predecessor_reconciliation() -> TestResult {
        run_postgres_test(async {
            let (database, database_url) =
                migrated_database("committed_predecessor_reconcile").await?;
            let now = unix_now()?;
            let journal = AdmissionReconnectJournalV2::connect_runtime(&database_url).await?;
            let fenced = ProtectionEntitlementV1::fenced(42).map_err(|_| "fenced protection")?;
            let predecessor_record =
                postgres_record(now, 10, 1, 0xb1, 7, fenced).map_err(|_| "predecessor record")?;
            let (mut predecessor_flow, predecessor_prepare) =
                ReconnectDurabilityFlowV1::begin(predecessor_record.clone());
            assert_eq!(
                journal.legacy().prepare(&predecessor_prepare).await?,
                ReconnectPrepareDispositionV1::Prepared
            );
            predecessor_flow
                .accept_prepare_completion(ReconnectPrepareCompletionV1::for_request(
                    &predecessor_prepare,
                    ReconnectPrepareDispositionV1::Prepared,
                ))
                .map_err(|_| "predecessor prepare completion")?;
            let current = exact_current_authority(&predecessor_record, now)
                .map_err(|_| "predecessor current authority")?;
            let predecessor_commit = predecessor_flow
                .authorize_commit(current, now)
                .map_err(|_| "predecessor commit authorization")?;
            assert_eq!(
                journal.legacy().commit(&predecessor_commit).await?,
                ReconnectCommitDispositionV1::Committed
            );

            let candidate = postgres_record(
                now,
                20,
                2,
                0xb2,
                8,
                ProtectionEntitlementV1::fenced(42).map_err(|_| "candidate protection")?,
            )
            .map_err(|_| "candidate record")?;
            let authorization = replacement_authorization(&candidate, 10, 8)
                .map_err(|_| "replacement authorization")?;
            let replacement_request =
                ReconnectDurabilityFlowV2::begin(candidate, Some(authorization)).1;
            assert_eq!(
                journal.prepare(&replacement_request).await?,
                ReconnectPrepareDispositionV2::Prepared
            );

            assert_eq!(
                journal.legacy().reconcile(&predecessor_prepare).await?,
                ReconnectDurableReconciliationSnapshotV1::terminal(predecessor_record)
            );

            drop(journal);
            database.cleanup().await?;
            Ok(())
        })
    }
}

/// Conservative first implementation: all journal writers and row-locking
/// readers serialize before any semantic clock sample. EXCLUSIVE still permits
/// ordinary snapshot readers. This intentionally makes no concurrency claim.
pub(super) async fn lock_admission_domain(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    record: &oteryn_game_server::foundation::ReconnectDurabilityRecordV1,
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
    if let oteryn_game_server::foundation::ReconnectProofV1::ReauthenticatedRecovery {
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
