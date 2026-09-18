use std::sync::Arc;

use crate::HashMap;

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

        // The protocol has three fixed and at most three optional parameters.
        // Keep the outer parameter array inline through authentication.
        let mut params = [("", ""); 6];
        params[..3].copy_from_slice(&[
            ("DateStyle", "ISO, MDY"),
            ("client_encoding", "UTF8"),
            ("TimeZone", "UTC"),
        ]);
        let mut count = 3;
        for (key, value) in [
            ("extra_float_digits", options.extra_float_digits.as_deref()),
            ("application_name", options.application_name.as_deref()),
            ("options", options.options.as_deref()),
        ] {
            if let Some(value) = value {
                params[count] = (key, value);
                count += 1;
            }
        }
        let startup = Startup {
            username: Some(&options.username),
            database: options.database.as_deref(),
            params: &params[..count],
        };
        use sqlx_core::io::ProtocolEncode;
        stream.write_precharged(startup.encoded_size()?, |buf| startup.encode(buf))?;

        stream.flush().await?;

        // The server then uses this information and the contents of
        // its configuration files (such as pg_hba.conf) to determine whether the connection is
        // provisionally acceptable, and what additional
        // authentication is required (if any).

        let mut process_id = 0;
        let mut secret_key = 0;
        let transaction_status;
        let strict_scram = options.oteryn_root_profile();
        let mut scram_authenticated = false;

        loop {
            let message = stream.recv().await?;
            match message.format {
                BackendMessageFormat::Authentication => match message.decode()? {
                    Authentication::Ok => {
                        if strict_scram && !scram_authenticated {
                            return Err(Error::Configuration(
                                "Oteryn PostgreSQL root profile rejects passwordless authentication"
                                    .into(),
                            ));
                        }
                    }

                    Authentication::CleartextPassword => {
                        if strict_scram {
                            return Err(Error::Configuration(
                                "Oteryn PostgreSQL root profile requires SCRAM-SHA-256".into(),
                            ));
                        }

                        // The frontend must now send a [PasswordMessage] containing the
                        // password in clear-text form.
                        stream
                            .send(Password::Cleartext(
                                options.password.as_deref().unwrap_or_default(),
                            ))
                            .await?;
                    }

                    Authentication::Md5Password(body) => {
                        if strict_scram {
                            return Err(Error::Configuration(
                                "Oteryn PostgreSQL root profile requires SCRAM-SHA-256".into(),
                            ));
                        }

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
                        if strict_scram
                            && !body
                                .mechanisms()
                                .any(|mechanism| mechanism == "SCRAM-SHA-256")
                        {
                            return Err(Error::Configuration(
                                "Oteryn PostgreSQL root profile requires SCRAM-SHA-256".into(),
                            ));
                        }
                        sasl::authenticate(&mut stream, options, body).await?;
                        if strict_scram {
                            scram_authenticated = true;
                        }
                    }

                    method => {
                        if stream.resource_budget().is_some() {
                            return Err(Error::Io(std::io::ErrorKind::InvalidData.into()));
                        }
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
                    if strict_scram && !scram_authenticated {
                        return Err(Error::Configuration(
                            "Oteryn PostgreSQL root profile requires completed SCRAM-SHA-256"
                                .into(),
                        ));
                    }
                    // start-up is completed. The frontend can now issue commands
                    transaction_status = message.decode::<ReadyForQuery>()?.transaction_status;

                    break;
                }

                _ => {
                    if stream.resource_budget().is_some() {
                        return Err(Error::Io(std::io::ErrorKind::InvalidData.into()));
                    }
                    return Err(err_protocol!(
                        "establish: unexpected message: {:?}",
                        message.format
                    ));
                }
            }
        }

        let budget = stream.resource_budget().cloned();
        let allocation = budget
            .as_ref()
            .map(|budget| {
                sqlx_core::net::resource_budget::ResourceReservation::try_new(
                    budget.clone(),
                    std::mem::size_of::<PgConnectionInner>(),
                )
            })
            .transpose()
            .map_err(|_| crate::statement::allocation_denied())?;
        let cache_statement =
            super::PgStatementCache::new(options.statement_cache_capacity, budget)?;
        Ok(PgConnection {
            inner: Box::new(PgConnectionInner {
                stream,
                process_id,
                secret_key,
                transaction_status,
                transaction_depth: 0,
                pending_ready_for_query_count: 0,
                next_statement_id: StatementId::NAMED_START,
                cache_statement,
                cache_type_oid: HashMap::new(),
                cache_type_info: HashMap::new(),
                cache_elem_type_to_array: HashMap::new(),
                cache_table_data: HashMap::new(),
                log_settings: options.log_settings.clone(),
                root_profile: options.oteryn_root_profile(),
            }),
            _allocation: allocation,
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
        server_with_gate(ssl_response, None)
    }

    fn server_with_gate(
        ssl_response: Option<u8>,
        gate: Option<Arc<std::sync::atomic::AtomicBool>>,
    ) -> (u16, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            if let Some(response) = ssl_response {
                let mut request = [0; 8];
                socket.read_exact(&mut request).unwrap();
                assert_eq!(&request, &[0, 0, 0, 8, 4, 210, 22, 47]);
                if let Some(gate) = gate {
                    gate.store(true, std::sync::atomic::Ordering::SeqCst);
                }
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
    fn denied_sync_and_drop_rollback_fence_connection_without_panicking() {
        use crate::statement::custody_test_support::Ledger;
        use crate::transaction::{PgTransactionManager, TransactionManager};
        for rollback in [false, true] {
            let (port, server) = server(None);
            let budget = Ledger::new(usize::MAX);
            sqlx_core::rt::test_block_on(async {
                let mut connection = PgConnection::establish_with_resource_budget(
                    &options(port, PgSslMode::Disable),
                    budget.clone(),
                )
                .await
                .unwrap();
                connection
                    .inner
                    .stream
                    .write_precharged(8192, |buf| {
                        buf.resize(8192, 0);
                        Ok(())
                    })
                    .unwrap();
                budget.limit(budget.held());
                if rollback {
                    connection.inner.transaction_depth = 1;
                    PgTransactionManager::start_rollback(&mut connection);
                } else {
                    connection.write_sync();
                }
                assert_eq!(
                    connection.inner.stream.flush().await.unwrap_err().kind(),
                    std::io::ErrorKind::ConnectionAborted
                );
                drop(connection);
            });
            server.join().unwrap();
            assert_eq!(budget.held(), 0);
        }
    }

    #[test]
    fn owner_aware_establish_retains_exact_owner_and_preserves_ssl_refusal() {
        let (port, server) = server(Some(b'N'));
        let concrete = Arc::new(TestBudget { funded: true });
        let weak: Weak<TestBudget> = Arc::downgrade(&concrete);
        let owner: Arc<dyn ResourceBudget> = concrete;
        let expected = owner.clone();
        let connection = sqlx_core::rt::test_block_on(
            PgConnection::establish_with_resource_budget(&options(port, PgSslMode::Prefer), owner),
        )
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
        struct TlsGateBudget(Arc<std::sync::atomic::AtomicBool>);
        impl ResourceBudget for TlsGateBudget {
            fn try_reserve(&self, _: usize) -> Result<(), BudgetError> {
                if self.0.load(std::sync::atomic::Ordering::SeqCst) {
                    Err(BudgetError::Unavailable)
                } else {
                    Ok(())
                }
            }
            fn release(&self, _: usize) {}
        }
        let gate = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (port, tls_server) = server_with_gate(Some(b'S'), Some(gate.clone()));
        let denied: Arc<dyn ResourceBudget> = Arc::new(TlsGateBudget(gate));
        assert!(
            sqlx_core::rt::test_block_on(PgConnection::establish_with_resource_budget(
                &options(port, PgSslMode::Require),
                denied,
            ))
            .is_err()
        );
        tls_server.join().unwrap();

        let (port, plain_server) = server(None);
        let connection = sqlx_core::rt::test_block_on(PgConnection::establish(&options(
            port,
            PgSslMode::Disable,
        )))
        .unwrap();
        assert!(connection.inner.stream.resource_budget().is_none());
        drop(connection);
        plain_server.join().unwrap();
    }
}

#[cfg(test)]
mod custody_tests {
    use super::*;
    use crate::statement::custody_test_support::Ledger;
    use crate::PgSslMode;
    use sqlx_core::{
        row::Row,
        value::{Value, ValueRef},
    };
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};

    fn frame(socket: &mut TcpStream, tag: u8, body: &[u8]) {
        socket.write_all(&[tag]).unwrap();
        socket
            .write_all(&u32::try_from(body.len() + 4).unwrap().to_be_bytes())
            .unwrap();
        socket.write_all(body).unwrap();
    }
    fn read_body(socket: &mut TcpStream) -> Vec<u8> {
        let mut size = [0; 4];
        socket.read_exact(&mut size).unwrap();
        let size = usize::try_from(u32::from_be_bytes(size)).unwrap();
        assert!((4..1024).contains(&size));
        let mut body = vec![0; size - 4];
        socket.read_exact(&mut body).unwrap();
        body
    }
    #[test]
    fn owned_query_row_value_and_column_outlive_connection_and_runtime() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(10)))
                .unwrap();
            let _startup = read_body(&mut socket);
            frame(&mut socket, b'R', &[0, 0, 0, 0]);
            frame(&mut socket, b'K', &[0, 0, 0, 1, 0, 0, 0, 2]);
            frame(&mut socket, b'Z', b"I");
            let mut tag = [0];
            socket.read_exact(&mut tag).unwrap();
            assert_eq!(tag, [b'Q']);
            assert_eq!(read_body(&mut socket), b"SELECT 42\0");
            let mut description = b"\0\x01number\0".to_vec();
            description.extend_from_slice(&0u32.to_be_bytes());
            description.extend_from_slice(&0i16.to_be_bytes());
            description.extend_from_slice(&23u32.to_be_bytes());
            description.extend_from_slice(&4i16.to_be_bytes());
            description.extend_from_slice(&(-1i32).to_be_bytes());
            description.extend_from_slice(&0i16.to_be_bytes());
            frame(&mut socket, b'T', &description);
            frame(&mut socket, b'D', b"\0\x01\0\0\0\x0242");
            frame(&mut socket, b'C', b"SELECT 1\0");
            frame(&mut socket, b'Z', b"I");
        });
        let budget = Ledger::new(usize::MAX);
        let (row, value, column) = sqlx_core::rt::test_block_on(async {
            let options = PgConnectOptions::new_without_pgpass()
                .host("127.0.0.1")
                .port(port)
                .username("test")
                .ssl_mode(PgSslMode::Disable);
            let mut connection =
                PgConnection::establish_with_resource_budget(&options, budget.clone())
                    .await
                    .unwrap();
            let row = sqlx_core::raw_sql::raw_sql(sqlx_core::sql_str::AssertSqlSafe("SELECT 42"))
                .fetch_one(&mut connection)
                .await
                .unwrap();
            assert_eq!(row.try_get::<i32, _>("number").unwrap(), 42);
            let value = ValueRef::to_owned(&row.try_get_raw(0).unwrap());
            let column = row.columns()[0].clone();
            drop(connection);
            (row, value, column)
        });
        server.join().unwrap();
        assert!(budget.held() > 0);
        drop(row);
        assert_eq!(value.as_ref().as_str().unwrap(), "42");
        drop(value);
        assert!(budget.held() > 0);
        drop(column);
        assert_eq!(budget.held(), 0);
    }
}
