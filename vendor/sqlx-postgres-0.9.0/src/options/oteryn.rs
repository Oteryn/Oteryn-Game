use std::net::IpAddr;
use std::sync::Arc;

use sqlx_core::net::resource_budget::ResourceBudget;

use super::{PgConnectOptions, PgResourceBudget, PgSslMode};
use crate::connection::LogSettings;
use crate::net::tls::CertificateInput;

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
    ) -> Self {
        Self {
            host: transport_ip.to_string(),
            port,
            socket: None,
            username: username.to_owned(),
            password: Some(password.to_owned()),
            database: Some(database.to_owned()),
            ssl_mode: PgSslMode::VerifyFull,
            ssl_root_cert: Some(CertificateInput::Inline(root_ca_pem)),
            ssl_client_cert: None,
            ssl_client_key: None,
            statement_cache_capacity: 100,
            application_name: None,
            log_settings: LogSettings::default(),
            extra_float_digits: Some("2".into()),
            options: None,
            resource_budget: Some(PgResourceBudget(resource_budget)),
            oteryn_root_profile: true,
            oteryn_tls_server_name: Some(tls_server_name.to_owned()),
        }
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

    use sqlx_core::net::resource_budget::{BudgetError, ResourceBudget};

    use super::*;

    #[derive(Debug)]
    struct Budget;

    impl ResourceBudget for Budget {
        fn try_reserve(&self, _bytes: usize) -> Result<(), BudgetError> {
            Ok(())
        }

        fn release(&self, _bytes: usize) {}
    }

    #[test]
    fn root_profile_is_no_ambient_and_separates_transport_from_tls_identity() {
        let owner: Arc<dyn ResourceBudget> = Arc::new(Budget);
        let options = PgConnectOptions::new_oteryn_root_profile(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            5432,
            "postgres.internal.example",
            "oteryn",
            "oteryn_runtime",
            "secret",
            b"-----BEGIN CERTIFICATE-----\nfixture\n-----END CERTIFICATE-----\n".to_vec(),
            owner,
        );

        assert_eq!(options.host, "127.0.0.1");
        assert_eq!(options.tls_server_name(), "postgres.internal.example");
        assert!(options.socket.is_none());
        assert_eq!(options.ssl_mode, PgSslMode::VerifyFull);
        assert!(options.ssl_client_cert.is_none());
        assert!(options.ssl_client_key.is_none());
        assert!(options.application_name.is_none());
        assert!(options.options.is_none());
        assert_eq!(options.statement_cache_capacity, 100);
        assert!(options.oteryn_root_profile());
        assert!(options.resource_budget().is_some());
    }
}
