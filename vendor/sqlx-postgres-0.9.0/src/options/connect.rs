use crate::connection::ConnectOptions;
use crate::error::Error;
use crate::{PgConnectOptions, PgConnection};
use log::LevelFilter;
use sqlx_core::Url;
use std::future::Future;
use std::time::Duration;

impl ConnectOptions for PgConnectOptions {
    type Connection = PgConnection;

    fn from_url(url: &Url) -> Result<Self, Error> {
        Self::parse_from_url(url)
    }

    fn to_url_lossy(&self) -> Url {
        self.build_url()
    }

    fn connect(&self) -> impl Future<Output = Result<Self::Connection, Error>> + Send + '_
    where
        Self::Connection: Sized,
    {
        async move {
            match self.resource_budget() {
                Some(resource_budget) => {
                    PgConnection::establish_with_resource_budget(self, resource_budget).await
                }
                None => PgConnection::establish(self).await,
            }
        }
    }

    fn log_statements(mut self, level: LevelFilter) -> Self {
        self.log_settings.log_statements(level);
        self
    }

    fn log_slow_statements(mut self, level: LevelFilter, duration: Duration) -> Self {
        self.log_settings.log_slow_statements(level, duration);
        self
    }
}

#[cfg(test)]
mod resource_owner_tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Weak};

    use sqlx_core::net::resource_budget::{BudgetError, ResourceBudget};

    use crate::{PgConnectOptions, PgSslMode};

    use super::ConnectOptions;

    #[derive(Debug)]
    struct TestBudget;

    impl ResourceBudget for TestBudget {
        fn try_reserve(&self, _bytes: usize) -> Result<(), BudgetError> {
            Ok(())
        }

        fn release(&self, _bytes: usize) {}
    }

    fn server() -> (u16, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
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

    fn options(port: u16) -> PgConnectOptions {
        PgConnectOptions::new()
            .host("127.0.0.1")
            .port(port)
            .username("root-owner-test")
            .ssl_mode(PgSslMode::Disable)
    }

    #[test]
    fn dedicated_resource_budget_profile_routes_connect_through_owner_aware_establish() {
        let (port, server) = server();
        let concrete = Arc::new(TestBudget);
        let weak: Weak<TestBudget> = Arc::downgrade(&concrete);
        let owner: Arc<dyn ResourceBudget> = concrete;
        let expected = Arc::clone(&owner);
        let options = options(port).with_resource_budget(owner);

        let connection = sqlx_core::rt::test_block_on(options.connect()).unwrap();
        assert!(Arc::ptr_eq(
            connection.inner.stream.resource_budget().unwrap(),
            &expected
        ));

        drop(expected);
        assert!(weak.upgrade().is_some());
        drop(connection);
        drop(options);
        assert!(weak.upgrade().is_none());
        server.join().unwrap();
    }

    #[test]
    fn ordinary_connect_options_remain_owner_free() {
        let (port, server) = server();
        let options = options(port);
        let connection = sqlx_core::rt::test_block_on(options.connect()).unwrap();
        assert!(connection.inner.stream.resource_budget().is_none());
        drop(connection);
        server.join().unwrap();
    }
}
