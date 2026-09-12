use std::sync::Arc;

use crate::HashMap;

use crate::common::StatementCache;
use crate::connection::{sasl, stream::PgStream};
use crate::error::Error;
use crate::io::StatementId;
use crate::message::{
    Authentication, BackendKeyData, BackendMessageFormat, Password, ReadyForQuery, Startup,
};
use crate::{PgConnectOptions, PgConnection};

use super::PgConnectionInner;
use sqlx_core::net::resource_budget::ResourceBudget;

// https://www.postgresql.org/docs/current/protocol-flow.html#id-1.10.5.7.3
// https://www.postgresql.org/docs/current/protocol-flow.html#id-1.10.5.7.11

impl PgConnection {
    pub(crate) async fn establish(options: &PgConnectOptions) -> Result<Self, Error> {
        let stream = PgStream::connect(options).await?;
        Self::finish_establish(options, stream).await
    }

    /// Establishes an operation-owned connection without changing ordinary
    /// `ConnectOptions` or pool behavior.
    #[doc(hidden)]
    pub async fn establish_with_resource_budget(
        options: &PgConnectOptions,
        resource_budget: Arc<dyn ResourceBudget>,
    ) -> Result<Self, Error> {
        let stream = PgStream::connect_with_resource_budget(options, resource_budget).await?;
        Self::finish_establish(options, stream).await
    }

    async fn finish_establish(
        options: &PgConnectOptions,
        mut stream: PgStream,
    ) -> Result<Self, Error> {
        // To begin a session, a frontend opens a connection to the server
        // and sends a startup message.

        let mut params = vec![
            // Sets the display format for date and time values,
            // as well as the rules for interpreting ambiguous date input values.
            ("DateStyle", "ISO, MDY"),
            // Sets the client-side encoding (character set).
            // <https://www.postgresql.org/docs/devel/multibyte.html#MULTIBYTE-CHARSET-SUPPORTED>
            ("client_encoding", "UTF8"),
            // Sets the time zone for displaying and interpreting time stamps.
            ("TimeZone", "UTC"),
        ];

        if let Some(ref extra_float_digits) = options.extra_float_digits {
            params.push(("extra_float_digits", extra_float_digits));
        }

        if let Some(ref application_name) = options.application_name {
            params.push(("application_name", application_name));
        }

        if let Some(ref options) = options.options {
            params.push(("options", options));
        }

        stream.write(Startup {
            username: Some(&options.username),
            database: options.database.as_deref(),
            params: &params,
        })?;

        stream.flush().await?;

        // The server then uses this information and the contents of
        // its configuration files (such as pg_hba.conf) to determine whether the connection is
        // provisionally acceptable, and what additional
        // authentication is required (if any).

        let mut process_id = 0;
        let mut secret_key = 0;
        let transaction_status;

        loop {
            let message = stream.recv().await?;
            match message.format {
                BackendMessageFormat::Authentication => match message.decode()? {
                    Authentication::Ok => {
                        // the authentication exchange is successfully completed
                        // do nothing; no more information is required to continue
                    }

                    Authentication::CleartextPassword => {
                        // The frontend must now send a [PasswordMessage] containing the
                        // password in clear-text form.

                        stream
                            .send(Password::Cleartext(
                                options.password.as_deref().unwrap_or_default(),
                            ))
                            .await?;
                    }

                    Authentication::Md5Password(body) => {
                        // The frontend must now send a [PasswordMessage] containing the
                        // password (with user name) encrypted via MD5, then encrypted again
                        // using the 4-byte random salt specified in the
                        // [AuthenticationMD5Password] message.

                        stream
                            .send(Password::Md5 {
                                username: &options.username,
                                password: options.password.as_deref().unwrap_or_default(),
                                salt: body.salt,
                            })
                            .await?;
                    }

                    Authentication::Sasl(body) => {
                        sasl::authenticate(&mut stream, options, body).await?;
                    }

                    method => {
                        return Err(err_protocol!(
                            "unsupported authentication method: {:?}",
                            method
                        ));
                    }
                },

                BackendMessageFormat::BackendKeyData => {
                    // provides secret-key data that the frontend must save if it wants to be
                    // able to issue cancel requests later

                    let data: BackendKeyData = message.decode()?;

                    process_id = data.process_id;
                    secret_key = data.secret_key;
                }

                BackendMessageFormat::ReadyForQuery => {
                    // start-up is completed. The frontend can now issue commands
                    transaction_status = message.decode::<ReadyForQuery>()?.transaction_status;

                    break;
                }

                _ => {
                    return Err(err_protocol!(
                        "establish: unexpected message: {:?}",
                        message.format
                    ))
                }
            }
        }

        Ok(PgConnection {
            inner: Box::new(PgConnectionInner {
                stream,
                process_id,
                secret_key,
                transaction_status,
                transaction_depth: 0,
                pending_ready_for_query_count: 0,
                next_statement_id: StatementId::NAMED_START,
                cache_statement: StatementCache::new(options.statement_cache_capacity),
                cache_type_oid: HashMap::new(),
                cache_type_info: HashMap::new(),
                cache_elem_type_to_array: HashMap::new(),
                cache_table_data: HashMap::new(),
                log_settings: options.log_settings.clone(),
            }),
        })
    }
}

#[cfg(test)]
mod resource_owner_tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Weak};

    use sqlx_core::net::resource_budget::{BudgetError, ResourceBudget};

    use crate::{PgConnectOptions, PgConnection, PgSslMode};

    #[derive(Debug)]
    struct TestBudget {
        funded: bool,
    }

    impl ResourceBudget for TestBudget {
        fn try_reserve(&self, _bytes: usize) -> Result<(), BudgetError> {
            self.funded.then_some(()).ok_or(BudgetError::Unavailable)
        }

        fn release(&self, _bytes: usize) {}
    }

    fn server(ssl_response: Option<u8>) -> (u16, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            if let Some(response) = ssl_response {
                let mut request = [0; 8];
                socket.read_exact(&mut request).unwrap();
                assert_eq!(&request, &[0, 0, 0, 8, 4, 210, 22, 47]);
                socket.write_all(&[response]).unwrap();
                if response == b'S' {
                    return;
                }
            }
            let mut length = [0; 4];
            socket.read_exact(&mut length).unwrap();
            let remaining = u32::from_be_bytes(length) as usize - 4;
            let mut startup = vec![0; remaining];
            socket.read_exact(&mut startup).unwrap();
            socket
                .write_all(&[
                    b'R', 0, 0, 0, 8, 0, 0, 0, 0, // AuthenticationOk
                    b'K', 0, 0, 0, 12, 0, 0, 0, 1, 0, 0, 0, 2, // BackendKeyData
                    b'Z', 0, 0, 0, 5, b'I', // ReadyForQuery
                ])
                .unwrap();
        });
        (port, handle)
    }

    fn options(port: u16, ssl_mode: PgSslMode) -> PgConnectOptions {
        PgConnectOptions::new()
            .host("127.0.0.1")
            .port(port)
            .username("owner-test")
            .ssl_mode(ssl_mode)
    }

    #[test]
    fn owner_aware_establish_retains_exact_owner_and_preserves_ssl_refusal() {
        let (port, server) = server(Some(b'N'));
        let concrete = Arc::new(TestBudget { funded: true });
        let weak: Weak<TestBudget> = Arc::downgrade(&concrete);
        let owner: Arc<dyn ResourceBudget> = concrete;
        let expected = owner.clone();
        let connection = sqlx_core::rt::test_block_on(PgConnection::establish_with_resource_budget(
                &options(port, PgSslMode::Prefer),
                owner,
            ))
            .unwrap();
        assert!(Arc::ptr_eq(
            connection.inner.stream.resource_budget().unwrap(),
            &expected
        ));
        drop(expected);
        assert!(weak.upgrade().is_some());
        drop(connection);
        assert!(weak.upgrade().is_none());
        server.join().unwrap();
    }

    #[test]
    fn accepted_tls_budget_denial_is_terminal_and_ordinary_path_is_owner_free() {
        let (port, tls_server) = server(Some(b'S'));
        let denied: Arc<dyn ResourceBudget> = Arc::new(TestBudget { funded: false });
        assert!(sqlx_core::rt::test_block_on(PgConnection::establish_with_resource_budget(
                &options(port, PgSslMode::Require),
                denied,
            ))
            .is_err());
        tls_server.join().unwrap();

        let (port, plain_server) = server(None);
        let connection = sqlx_core::rt::test_block_on(PgConnection::establish(&options(port, PgSslMode::Disable)))
            .unwrap();
        assert!(connection.inner.stream.resource_budget().is_none());
        drop(connection);
        plain_server.join().unwrap();
    }
}
