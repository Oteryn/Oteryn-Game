use crate::error::Error;
use crate::net::tls::{self, CertificateInput, TlsConfig};
use crate::net::{Socket, SocketIntoBox, WithSocket};
use sqlx_core::net::{ConnectionOwner, OwnedSocket, SocketIntoOwnedBox};

use crate::message::SslRequest;
use crate::{PgConnectOptions, PgSslMode};
use std::net::IpAddr;

pub struct MaybeUpgradeTls<'a>(pub &'a PgConnectOptions);

pub struct MaybeUpgradeTlsOwned<'a>(pub &'a PgConnectOptions, pub ConnectionOwner);

impl WithSocket for MaybeUpgradeTlsOwned<'_> {
    type Output = crate::Result<OwnedSocket>;

    async fn with_socket<S: Socket>(self, socket: S) -> Self::Output {
        maybe_upgrade_owned(socket, self.0, self.1).await
    }
}

impl WithSocket for MaybeUpgradeTls<'_> {
    type Output = crate::Result<Box<dyn Socket>>;

    async fn with_socket<S: Socket>(self, socket: S) -> Self::Output {
        maybe_upgrade(socket, self.0).await
    }
}

async fn maybe_upgrade<S: Socket>(
    mut socket: S,
    options: &PgConnectOptions,
) -> Result<Box<dyn Socket>, Error> {
    // https://www.postgresql.org/docs/12/libpq-ssl.html#LIBPQ-SSL-SSLMODE-STATEMENTS
    match options.ssl_mode {
        // FIXME: Implement ALLOW
        PgSslMode::Allow | PgSslMode::Disable => return Ok(Box::new(socket)),

        PgSslMode::Prefer => {
            if !tls::available() {
                return Ok(Box::new(socket));
            }

            // try upgrade, but its okay if we fail
            if !request_upgrade(&mut socket, options).await? {
                return Ok(Box::new(socket));
            }
        }

        PgSslMode::Require | PgSslMode::VerifyFull | PgSslMode::VerifyCa => {
            tls::error_if_unavailable()?;

            if !request_upgrade(&mut socket, options).await? {
                // upgrade failed, die
                return Err(Error::Tls("server does not support TLS".into()));
            }
        }
    }

    let accept_invalid_certs = !matches!(
        options.ssl_mode,
        PgSslMode::VerifyCa | PgSslMode::VerifyFull
    );
    let accept_invalid_hostnames = !matches!(options.ssl_mode, PgSslMode::VerifyFull);

    let config = TlsConfig {
        accept_invalid_certs,
        accept_invalid_hostnames,
        hostname: &options.host,
        oteryn_root_profile: false,
        root_cert_path: options.ssl_root_cert.as_ref(),
        client_cert_path: options.ssl_client_cert.as_ref(),
        client_key_path: options.ssl_client_key.as_ref(),
    };

    tls::handshake(socket, config, SocketIntoBox).await
}

fn validate_oteryn_root_profile(options: &PgConnectOptions) -> Result<(), Error> {
    if !options.oteryn_root_profile() {
        return Ok(());
    }
    if !matches!(options.ssl_mode, PgSslMode::VerifyFull) {
        return Err(Error::Configuration(
            "Oteryn PostgreSQL root profile requires VerifyFull".into(),
        ));
    }
    if options.host.parse::<IpAddr>().is_err() || options.fetch_socket().is_some() {
        return Err(Error::Configuration(
            "Oteryn PostgreSQL root profile requires literal-IP TCP".into(),
        ));
    }
    if options.tls_server_name().parse::<IpAddr>().is_ok() || options.tls_server_name().is_empty() {
        return Err(Error::Configuration(
            "Oteryn PostgreSQL root profile requires a separate DNS TLS identity".into(),
        ));
    }
    if !matches!(
        options.ssl_root_cert,
        Some(CertificateInput::OterynInline(_))
    ) {
        return Err(Error::Configuration(
            "Oteryn PostgreSQL root profile requires an inline root CA".into(),
        ));
    }
    if options.ssl_client_cert.is_some() || options.ssl_client_key.is_some() {
        return Err(Error::Configuration(
            "Oteryn PostgreSQL root profile forbids client certificates".into(),
        ));
    }
    if options.application_name.is_some()
        || !options.has_oteryn_root_startup_options()
        || options.statement_cache_capacity != 100
    {
        return Err(Error::Configuration(
            "Oteryn PostgreSQL root profile forbids ambient startup options".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod oteryn_root_startup_options_tests {
    use std::net::{IpAddr, Ipv4Addr};
    use std::sync::Arc;

    use sqlx_core::net::resource_budget::{BudgetError, ResourceBudget};

    use super::*;

    #[derive(Debug)]
    struct UnlimitedBudget;

    impl ResourceBudget for UnlimitedBudget {
        fn try_reserve(&self, _bytes: usize) -> Result<(), BudgetError> {
            Ok(())
        }

        fn release(&self, _bytes: usize) {}
    }

    fn root_profile(options: [(&str, &str); 3]) -> PgConnectOptions {
        PgConnectOptions::new_oteryn_root_profile(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            5432,
            "localhost",
            "oteryn",
            "oteryn_runtime",
            "fixture-password",
            b"-----BEGIN CERTIFICATE-----\nfixture\n-----END CERTIFICATE-----\n",
            Arc::new(UnlimitedBudget),
        )
        .expect("root profile")
        .options(options)
        .expect("funded startup options")
        .into_connect_options()
    }

    #[test]
    fn exact_sanctioned_startup_options_are_accepted() {
        let options = root_profile([
            ("transaction_timeout", "2000ms"),
            ("statement_timeout", "2000ms"),
            ("lock_timeout", "2000ms"),
        ]);
        assert!(validate_oteryn_root_profile(&options).is_ok());
    }

    #[test]
    fn extra_ambient_startup_option_is_rejected() {
        let mut options = root_profile([
            ("transaction_timeout", "2000ms"),
            ("statement_timeout", "2000ms"),
            ("lock_timeout", "2000ms"),
        ]);
        options.options.as_mut().unwrap().push_str(" -c search_path=public");
        assert!(matches!(
            validate_oteryn_root_profile(&options),
            Err(Error::Configuration(_))
        ));
    }

    #[test]
    fn mutated_sanctioned_startup_option_is_rejected() {
        let options = root_profile([
            ("transaction_timeout", "2001ms"),
            ("statement_timeout", "2000ms"),
            ("lock_timeout", "2000ms"),
        ]);
        assert!(matches!(
            validate_oteryn_root_profile(&options),
            Err(Error::Configuration(_))
        ));
    }

    #[test]
    fn ordinary_options_builder_cannot_forge_precharged_startup_options() {
        let options = PgConnectOptions::new_oteryn_root_profile(
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            5432,
            "localhost",
            "oteryn",
            "oteryn_runtime",
            "fixture-password",
            b"-----BEGIN CERTIFICATE-----\nfixture\n-----END CERTIFICATE-----\n",
            Arc::new(UnlimitedBudget),
        )
        .expect("root profile")
        .into_connect_options()
        .options([
            ("transaction_timeout", "2000ms"),
            ("statement_timeout", "2000ms"),
            ("lock_timeout", "2000ms"),
        ]);

        assert!(matches!(
            validate_oteryn_root_profile(&options),
            Err(Error::Configuration(_))
        ));
    }

    #[test]
    fn ordinary_options_clone_cannot_copy_precharge_proof() {
        let options = root_profile([
            ("transaction_timeout", "2000ms"),
            ("statement_timeout", "2000ms"),
            ("lock_timeout", "2000ms"),
        ]);
        let cloned = options.clone();

        assert!(validate_oteryn_root_profile(&options).is_ok());
        assert!(matches!(
            validate_oteryn_root_profile(&cloned),
            Err(Error::Configuration(_))
        ));
    }

    #[test]
    fn ordinary_options_builder_invalidates_an_existing_precharge_proof() {
        let options = root_profile([
            ("transaction_timeout", "2000ms"),
            ("statement_timeout", "2000ms"),
            ("lock_timeout", "2000ms"),
        ]);
        let original = options.get_options().unwrap().to_owned();
        let mutated = options.options(std::iter::empty::<(&str, &str)>());

        assert_eq!(mutated.get_options(), Some(original.as_str()));
        assert!(matches!(
            validate_oteryn_root_profile(&mutated),
            Err(Error::Configuration(_))
        ));
    }
}

async fn maybe_upgrade_owned<S: Socket>(
    mut socket: S,
    options: &PgConnectOptions,
    owner: ConnectionOwner,
) -> Result<OwnedSocket, Error> {
    validate_oteryn_root_profile(options)?;
    match options.ssl_mode {
        PgSslMode::Allow | PgSslMode::Disable => {
            return SocketIntoOwnedBox(owner.budget()).with_socket(socket).await
        }
        PgSslMode::Prefer => {
            if !tls::available() || !request_upgrade(&mut socket, options).await? {
                return SocketIntoOwnedBox(owner.budget()).with_socket(socket).await;
            }
        }
        PgSslMode::Require | PgSslMode::VerifyFull | PgSslMode::VerifyCa => {
            tls::error_if_unavailable()?;
            if !request_upgrade(&mut socket, options).await? {
                return Err(Error::Tls("server does not support TLS".into()));
            }
        }
    }
    let config = TlsConfig {
        accept_invalid_certs: !matches!(
            options.ssl_mode,
            PgSslMode::VerifyCa | PgSslMode::VerifyFull
        ),
        accept_invalid_hostnames: !matches!(options.ssl_mode, PgSslMode::VerifyFull),
        hostname: options.tls_server_name(),
        oteryn_root_profile: options.oteryn_root_profile(),
        root_cert_path: options.ssl_root_cert.as_ref(),
        client_cert_path: options.ssl_client_cert.as_ref(),
        client_key_path: options.ssl_client_key.as_ref(),
    };
    tls::handshake_with_connection_owner(socket, config, SocketIntoOwnedBox(owner.budget()), owner)
        .await?
}

async fn request_upgrade(
    socket: &mut impl Socket,
    _options: &PgConnectOptions,
) -> Result<bool, Error> {
    // https://www.postgresql.org/docs/current/protocol-flow.html#id-1.10.5.7.11

    // To initiate an SSL-encrypted connection, the frontend initially sends an
    // SSLRequest message rather than a StartupMessage

    socket.write(SslRequest::BYTES).await?;

    // The server then responds with a single byte containing S or N, indicating that
    // it is willing or unwilling to perform SSL, respectively.

    let mut response = [0u8];

    socket.read(&mut &mut response[..]).await?;

    match response[0] {
        b'S' => {
            // The server is ready and willing to accept an SSL connection
            Ok(true)
        }

        b'N' => {
            // The server is _unwilling_ to perform an SSL connection
            Ok(false)
        }

        other => Err(err_protocol!(
            "unexpected response from SSLRequest: 0x{:02x}",
            other
        )),
    }
}
