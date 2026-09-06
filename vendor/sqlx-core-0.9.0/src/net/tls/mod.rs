#![allow(dead_code)]

use std::path::PathBuf;

use crate::error::Error;
use crate::net::socket::WithSocket;
use crate::net::Socket;

#[cfg(feature = "_tls-rustls")]
mod tls_rustls;

#[cfg(feature = "_tls-native-tls")]
mod tls_native_tls;

mod util;

/// X.509 Certificate input, either a file path or a PEM encoded inline certificate(s).
#[derive(Clone, Debug)]
pub enum CertificateInput {
    /// PEM encoded certificate(s)
    Inline(Vec<u8>),
    /// Path to a file containing PEM encoded certificate(s)
    File(PathBuf),
}

impl From<String> for CertificateInput {
    fn from(value: String) -> Self {
        // Leading and trailing whitespace/newlines
        let trimmed = value.trim();

        // Heuristic for PEM encoded inputs:
        // https://tools.ietf.org/html/rfc7468
        if trimmed.starts_with("-----BEGIN") && trimmed.ends_with("-----") {
            CertificateInput::Inline(value.as_bytes().to_vec())
        } else {
            CertificateInput::File(PathBuf::from(value))
        }
    }
}

impl CertificateInput {
    async fn data(&self) -> Result<Vec<u8>, std::io::Error> {
        use crate::fs;
        match self {
            CertificateInput::Inline(v) => Ok(v.clone()),
            CertificateInput::File(path) => fs::read(path).await,
        }
    }
}

impl std::fmt::Display for CertificateInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CertificateInput::Inline(v) => write!(f, "{}", String::from_utf8_lossy(v.as_slice())),
            CertificateInput::File(path) => write!(f, "file: {}", path.display()),
        }
    }
}

pub struct TlsConfig<'a> {
    pub accept_invalid_certs: bool,
    pub accept_invalid_hostnames: bool,
    pub hostname: &'a str,
    pub root_cert_path: Option<&'a CertificateInput>,
    pub client_cert_path: Option<&'a CertificateInput>,
    pub client_key_path: Option<&'a CertificateInput>,
}

pub async fn handshake<S, Ws>(
    socket: S,
    config: TlsConfig<'_>,
    with_socket: Ws,
) -> crate::Result<Ws::Output>
where
    S: Socket,
    Ws: WithSocket,
{
    #[cfg(feature = "_tls-native-tls")]
    return Ok(with_socket
        .with_socket(tls_native_tls::handshake(socket, config).await?)
        .await);

    #[cfg(all(feature = "_tls-rustls", not(feature = "_tls-native-tls")))]
    return Ok(with_socket
        .with_socket(tls_rustls::handshake(socket, config).await?)
        .await);

    #[cfg(not(any(feature = "_tls-native-tls", feature = "_tls-rustls")))]
    {
        drop((socket, config, with_socket));
        panic!("one of the `runtime-*-native-tls` or `runtime-*-rustls` features must be enabled")
    }
}

pub fn available() -> bool {
    cfg!(any(feature = "_tls-native-tls", feature = "_tls-rustls"))
}

pub fn error_if_unavailable() -> crate::Result<()> {
    if !available() {
        return Err(Error::tls(
            "TLS upgrade required by connect options \
                    but SQLx was built without TLS support enabled",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod resource_budget_tests;

/// Data-owner failure only; no rejected bytes or path are copied into a message.
#[derive(Debug)]
pub(super) enum CertificateReadError {
    Budget(crate::net::resource_budget::BudgetError),
    Io(std::io::Error),
}

impl From<crate::net::resource_budget::BudgetError> for CertificateReadError {
    fn from(value: crate::net::resource_budget::BudgetError) -> Self {
        Self::Budget(value)
    }
}

/// Read complete certificate/key data with capacity charged before each growth.
///
/// This synchronous data-owner primitive does not launch a task. A future
/// blocking-job adapter must separately charge its closure, scheduler and result
/// custody and must move this returned owner into the completed job result.
/// The reader's own storage/behavior and enclosing result metadata belong to
/// that caller. No file length snapshot, truncation or separate byte cap is used.
fn read_certificate_data_accounted(
    reader: &mut impl std::io::Read,
    budget: std::sync::Arc<dyn crate::net::resource_budget::ResourceBudget>,
) -> Result<crate::net::resource_budget::Charged<Vec<u8>>, CertificateReadError> {
    use crate::net::resource_budget::{BudgetError, ResourceReservation};
    // Read granularity only: every iteration continues to EOF while the same
    // owner can fund actual capacity. This is not a certificate-size maximum.
    const READ_CHUNK: usize = 4096;
    let mut data_charge = ResourceReservation::try_new(budget.clone(), 0)?;
    let mut data = Vec::new();
    let scratch_charge = ResourceReservation::try_new(budget, READ_CHUNK)?;
    let mut scratch = Vec::new();
    scratch
        .try_reserve_exact(READ_CHUNK)
        .map_err(|_| BudgetError::Unavailable)?;
    scratch.resize(READ_CHUNK, 0);

    loop {
        let count = match reader.read(&mut scratch) {
            Ok(0) => break,
            Ok(count) => count,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(CertificateReadError::Io(error)),
        };
        let required = data.len().checked_add(count).ok_or(BudgetError::Overflow)?;
        if required > data.capacity() {
            let old_capacity = data.capacity();
            let next_capacity = old_capacity
                .checked_mul(2)
                .ok_or(BudgetError::Overflow)?
                .max(required);
            // Hold old + new until the allocation call returns. The pinned
            // Global allocator exposes the requested exact capacity to Vec;
            // allocator bookkeeping/RSS is a different measurement.
            data_charge.try_grow(next_capacity)?;
            data.try_reserve_exact(next_capacity - data.len())
                .map_err(|_| BudgetError::Unavailable)?;
            // Old backing has now either been replaced/freed or reused.
            drop(data_charge.split_off(old_capacity)?);
        }
        data.extend_from_slice(&scratch[..count]);
    }
    drop(scratch);
    drop(scratch_charge);
    Ok(data_charge.bind(data))
}

/// Linux/Rust1.94 file-data primitive, awaiting a qualified execution owner.
/// This does not replace the existing async `CertificateInput::data` path.
#[cfg(target_os = "linux")]
fn read_certificate_file_accounted(
    path: &std::path::Path,
    budget: std::sync::Arc<dyn crate::net::resource_budget::ResourceBudget>,
) -> Result<crate::net::resource_budget::Charged<Vec<u8>>, CertificateReadError> {
    use crate::net::resource_budget::{BudgetError, ResourceReservation};
    // std's run_path_with_cstr uses stack storage for short paths and an exact
    // len+1 CString allocation otherwise (small_c_string.rs, c_str.rs273–295).
    let path_capacity = path
        .as_os_str()
        .as_encoded_bytes()
        .len()
        .checked_add(1)
        .ok_or(BudgetError::Overflow)?;
    let path_charge = ResourceReservation::try_new(budget.clone(), path_capacity)?;
    let mut file = std::fs::File::open(path).map_err(CertificateReadError::Io)?;
    drop(path_charge);
    read_certificate_data_accounted(&mut file, budget)
}
