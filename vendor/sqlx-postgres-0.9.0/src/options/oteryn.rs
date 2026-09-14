use std::net::IpAddr;
use std::sync::Arc;

use sqlx_core::net::resource_budget::{BudgetError, ResourceBudget};

use super::{PgConnectOptions, PgResourceBudget, PgSslMode};
use crate::connection::LogSettings;
use crate::net::tls::CertificateInput;

/// Same-ledger custody for the retained backing owned by an Oteryn root profile.
///
/// This wrapper is shared by option clones and physical connections, so the
/// charge is returned exactly once after the last holder has destroyed the
/// corresponding profile backing.
struct OterynProfileBudget {
    root: Arc<dyn ResourceBudget>,
    retained_bytes: usize,
}

impl ResourceBudget for OterynProfileBudget {
    fn try_reserve(&self, bytes: usize) -> Result<(), BudgetError> {
        self.root.try_reserve(bytes)
    }

    fn release(&self, bytes: usize) {
        self.root.release(bytes);
    }

    fn try_reserve_provider_shared(&self, bytes: usize) -> Result<(), BudgetError> {
        self.root.try_reserve_provider_shared(bytes)
    }
}

impl Drop for OterynProfileBudget {
    fn drop(&mut self) {
        self.root.release(self.retained_bytes);
    }
}

impl std::fmt::Debug for OterynProfileBudget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OterynProfileBudget(..)")
    }
}

impl PgConnectOptions {
    /// Builds the exact no-ambient PostgreSQL profile used by the Oteryn
    /// Durability root. Unlike `new()`/`new_without_pgpass()`, this constructor
    /// does not read PG* variables, the OS username, `.pgpass`, Unix sockets,
    /// application options, client credentials, or filesystem CA defaults.
    ///
    /// The transport address is already typed as an IP address and remains
    /// independent from the DNS identity used by TLS certificate verification.
    /// The supplied resource budget is the process-scoped Durability root
    /// ledger carried into pool-created physical connections.
    #[doc(hidden)]
    #[allow(clippy::too_many_arguments)]
    pub fn new_oteryn_root_profile(
        transport_ip: IpAddr,
        port: u16,
        tls_server_name: &str,
        database: &str,
        username: &str,
        password: &str,
        root_ca_pem: Vec<u8>,
        resource_budget: Arc<dyn ResourceBudget>,
    ) -> Result<Self, BudgetError> {
        // IPv6 text is at most 39 bytes; reserve that conservative prospective
        // capacity before formatting the transport address. Every other owned
        // value is built with exactly the capacity charged below. The Arc
        // allocation holding this destruction-bound charge is included too.
        const MAX_IP_TEXT_BYTES: usize = 39;
        let retained_bytes = MAX_IP_TEXT_BYTES
            .checked_add(tls_server_name.len())
            .and_then(|n| n.checked_add(database.len()))
            .and_then(|n| n.checked_add(username.len()))
            .and_then(|n| n.checked_add(password.len()))
            .and_then(|n| n.checked_add(root_ca_pem.capacity()))
            .and_then(|n| n.checked_add(std::mem::size_of::<OterynProfileBudget>()))
            .and_then(|n| n.checked_add(2 * std::mem::size_of::<usize>()))
            .ok_or(BudgetError::Overflow)?;
        resource_budget.try_reserve(retained_bytes)?;
        let profile_budget: Arc<dyn ResourceBudget> = Arc::new(OterynProfileBudget {
            root: resource_budget,
            retained_bytes,
        });

        fn copy_with_capacity(value: &str) -> String {
            let mut owned = String::with_capacity(value.len());
            owned.push_str(value);
            owned
        }

        let mut host = String::with_capacity(MAX_IP_TEXT_BYTES);
        use std::fmt::Write as _;
        write!(&mut host, "{transport_ip}").expect("formatting an IP address cannot fail");
        let mut log_settings = LogSettings::default();
        log_settings.statements_level = log::LevelFilter::Off;
        log_settings.slow_statements_level = log::LevelFilter::Off;
        Ok(Self {
            host,
            port,
            socket: None,
            username: copy_with_capacity(username),
            password: Some(copy_with_capacity(password)),
            database: Some(copy_with_capacity(database)),
            ssl_mode: PgSslMode::VerifyFull,
            ssl_root_cert: Some(CertificateInput::OterynInline(root_ca_pem)),
            ssl_client_cert: None,
            ssl_client_key: None,
            statement_cache_capacity: 100,
            application_name: None,
            log_settings,
            extra_float_digits: Some("2".into()),
            options: None,
            resource_budget: Some(PgResourceBudget(profile_budget)),
            oteryn_root_profile: true,
            oteryn_tls_server_name: Some(copy_with_capacity(tls_server_name)),
        })
    }

    pub(crate) fn oteryn_root_profile(&self) -> bool {
        self.oteryn_root_profile
    }

    pub(crate) fn tls_server_name(&self) -> &str {
        self.oteryn_tls_server_name
            .as_deref()
            .unwrap_or(self.host.as_str())
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use sqlx_core::net::resource_budget::{BudgetError, ResourceBudget};

    use super::*;

    #[derive(Debug)]
    struct Budget {
        limit: usize,
        acquired: AtomicUsize,
        released: AtomicUsize,
    }

    impl Budget {
        fn new(limit: usize) -> Self {
            Self {
                limit,
                acquired: AtomicUsize::new(0),
                released: AtomicUsize::new(0),
            }
        }
        fn retained(&self) -> usize {
            self.acquired.load(Ordering::SeqCst) - self.released.load(Ordering::SeqCst)
        }
    }

    impl ResourceBudget for Budget {
        fn try_reserve(&self, bytes: usize) -> Result<(), BudgetError> {
            let mut current = self.acquired.load(Ordering::SeqCst);
            loop {
                let next = current.checked_add(bytes).ok_or(BudgetError::Overflow)?;
                if next > self.limit {
                    return Err(BudgetError::Unavailable);
                }
                match self.acquired.compare_exchange(
                    current,
                    next,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                ) {
                    Ok(_) => return Ok(()),
                    Err(observed) => current = observed,
                }
            }
        }
        fn release(&self, bytes: usize) {
            self.released.fetch_add(bytes, Ordering::SeqCst);
        }
    }

    fn profile(owner: Arc<dyn ResourceBudget>) -> Result<PgConnectOptions, BudgetError> {
        PgConnectOptions::new_oteryn_root_profile(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            5432,
            "postgres.internal.example",
            "oteryn",
            "oteryn_runtime",
            "secret",
            b"-----BEGIN CERTIFICATE-----\nfixture\n-----END CERTIFICATE-----\n".to_vec(),
            owner,
        )
    }

    #[test]
    fn root_profile_is_no_ambient_and_separates_transport_from_tls_identity() {
        let ledger = Arc::new(Budget::new(usize::MAX));
        let owner: Arc<dyn ResourceBudget> = ledger.clone();
        let options = profile(owner).expect("funded root profile");
        assert!(ledger.retained() > 0);
        assert_eq!(options.host, "127.0.0.1");
        assert_eq!(options.tls_server_name(), "postgres.internal.example");
        assert!(options.socket.is_none());
        assert!(matches!(options.ssl_mode, PgSslMode::VerifyFull));
        assert!(options.ssl_client_cert.is_none());
        assert!(options.ssl_client_key.is_none());
        assert!(options.application_name.is_none());
        assert!(options.options.is_none());
        assert_eq!(options.statement_cache_capacity, 100);
        assert!(options.oteryn_root_profile());
        assert!(options.resource_budget().is_some());
        assert!(matches!(
            options.ssl_root_cert,
            Some(CertificateInput::OterynInline(_))
        ));
        drop(options);
        assert_eq!(ledger.retained(), 0);
    }

    #[test]
    fn root_profile_denies_before_retained_copy_and_leaves_no_charge() {
        let ledger = Arc::new(Budget::new(0));
        let owner: Arc<dyn ResourceBudget> = ledger.clone();
        assert!(matches!(profile(owner), Err(BudgetError::Unavailable)));
        assert_eq!(ledger.retained(), 0);
    }

    #[test]
    fn root_profile_clones_share_one_charge_until_last_drop() {
        let ledger = Arc::new(Budget::new(usize::MAX));
        let owner: Arc<dyn ResourceBudget> = ledger.clone();
        let options = profile(owner).expect("funded root profile");
        let retained = ledger.retained();
        let clone = options.clone();
        assert_eq!(ledger.retained(), retained);
        drop(options);
        assert_eq!(ledger.retained(), retained);
        drop(clone);
        assert_eq!(ledger.retained(), 0);
    }
}
