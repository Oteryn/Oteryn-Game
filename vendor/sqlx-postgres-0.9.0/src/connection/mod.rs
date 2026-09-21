use std::collections::BTreeMap;
use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::sync::Arc;

use crate::HashMap;

use crate::common::StatementCache;
use crate::error::Error;
use crate::ext::ustr::UStr;
use crate::io::StatementId;
use crate::message::{
    BackendMessageFormat, Close, Query, ReadyForQuery, ReceivedMessage, Terminate,
    TransactionStatus,
};
use crate::statement::PgStatementMetadata;
use crate::transaction::Transaction;
use sqlx_core::net::{ResourceBudget, ResourceReservation};
use crate::types::Oid;
use crate::{PgConnectOptions, PgTypeInfo, Postgres};

pub(crate) use sqlx_core::connection::*;
use sqlx_core::sql_str::SqlSafeStr;

pub use self::stream::PgStream;

#[cfg(feature = "offline")]
mod describe;
mod establish;
mod executor;
mod resolve;
mod sasl;
mod stream;
mod tls;

/// A connection to a PostgreSQL database.
///
/// See [`PgConnectOptions`] for connection URL reference.
pub struct PgConnection {
    pub(crate) inner: Box<PgConnectionInner>,
}

pub struct PgConnectionInner {
    // underlying TCP or UDS stream,
    // wrapped in a potentially TLS stream,
    // wrapped in a buffered stream
    pub(crate) stream: PgStream,

    // process id of this backend
    // used to send cancel requests
    #[allow(dead_code)]
    process_id: u32,

    // secret key of this backend
    // used to send cancel requests
    #[allow(dead_code)]
    secret_key: u32,

    // sequence of statement IDs for use in preparing statements
    // in PostgreSQL, the statement is prepared to a user-supplied identifier
    next_statement_id: StatementId,

    // cache statement by query string to the id and columns
    cache_statement: PgStatementCache,

    // cache user-defined types by id <-> info
    cache_type_info: HashMap<Oid, PgTypeInfo>,
    cache_type_oid: HashMap<UStr, Oid>,
    cache_elem_type_to_array: HashMap<Oid, Oid>,
    cache_table_data: HashMap<Oid, TableData>,

    // number of ReadyForQuery messages that we are currently expecting
    pub(crate) pending_ready_for_query_count: usize,

    // current transaction status
    transaction_status: TransactionStatus,
    pub(crate) transaction_depth: usize,

    log_settings: LogSettings,
}

type CachedStatement = (StatementId, Arc<PgStatementMetadata>);

struct Wp3CacheEntry {
    key: String,
    value: CachedStatement,
    _allocation: ResourceReservation,
}

struct Wp3StatementCache {
    entries: Vec<Wp3CacheEntry>,
    _backing: ResourceReservation,
    budget: Arc<dyn ResourceBudget>,
    capacity: usize,
}

enum PgStatementCache {
    Ordinary(StatementCache<CachedStatement>),
    FirstSlice(Wp3StatementCache),
}

impl PgStatementCache {
    fn new(
        capacity: usize,
        first_slice: bool,
        budget: Option<Arc<dyn ResourceBudget>>,
    ) -> Result<Self, Error> {
        if !first_slice {
            return Ok(Self::Ordinary(StatementCache::new(capacity)));
        }
        if capacity != 100 {
            return Err(Error::Io(std::io::ErrorKind::InvalidInput.into()));
        }
        let budget = budget.ok_or_else(|| Error::Io(std::io::ErrorKind::OutOfMemory.into()))?;
        let bytes = capacity
            .checked_mul(std::mem::size_of::<Wp3CacheEntry>())
            .ok_or_else(|| Error::Io(std::io::ErrorKind::OutOfMemory.into()))?;
        let backing = ResourceReservation::try_new(budget.clone(), bytes)
            .map_err(|_| Error::Io(std::io::ErrorKind::OutOfMemory.into()))?;
        Ok(Self::FirstSlice(Wp3StatementCache {
            entries: Vec::with_capacity(capacity),
            _backing: backing,
            budget,
            capacity,
        }))
    }

    fn get_mut(&mut self, key: &str) -> Option<&mut CachedStatement> {
        match self {
            Self::Ordinary(cache) => cache.get_mut(key),
            Self::FirstSlice(cache) => {
                let index = cache.entries.iter().position(|entry| entry.key == key)?;
                cache.entries[index..].rotate_left(1);
                cache.entries.last_mut().map(|entry| &mut entry.value)
            }
        }
    }

    fn insert(
        &mut self,
        key: &str,
        value: CachedStatement,
    ) -> Result<Option<CachedStatement>, Error> {
        match self {
            Self::Ordinary(cache) => Ok(cache.insert(key, value)),
            Self::FirstSlice(cache) => {
                let allocation = ResourceReservation::try_new(cache.budget.clone(), key.len())
                    .map_err(|_| Error::Io(std::io::ErrorKind::OutOfMemory.into()))?;
                let entry = Wp3CacheEntry {
                    key: key.to_owned(),
                    value,
                    _allocation: allocation,
                };
                let replaced = if let Some(index) =
                    cache.entries.iter().position(|entry| entry.key == key)
                {
                    Some(cache.entries.remove(index).value)
                } else if cache.entries.len() == cache.capacity {
                    Some(cache.entries.remove(0).value)
                } else {
                    None
                };
                cache.entries.push(entry);
                Ok(replaced)
            }
        }
    }

    fn len(&self) -> usize {
        match self {
            Self::Ordinary(cache) => cache.len(),
            Self::FirstSlice(cache) => cache.entries.len(),
        }
    }

    fn is_enabled(&self) -> bool {
        match self {
            Self::Ordinary(cache) => cache.is_enabled(),
            Self::FirstSlice(cache) => cache.capacity > 0,
        }
    }

    fn remove_lru(&mut self) -> Option<CachedStatement> {
        match self {
            Self::Ordinary(cache) => cache.remove_lru(),
            Self::FirstSlice(cache) => {
                (!cache.entries.is_empty()).then(|| cache.entries.remove(0).value)
            }
        }
    }
}

pub(crate) struct TableData {
    table_name: Arc<str>,
    /// Attribute number -> name.
    columns: BTreeMap<i16, Arc<str>>,
}

impl PgConnection {
    /// the version number of the server in `libpq` format
    pub fn server_version_num(&self) -> Option<u32> {
        self.inner.stream.server_version_num
    }

    // will return when the connection is ready for another query
    pub(crate) async fn wait_until_ready(&mut self) -> Result<(), Error> {
        if !self.inner.stream.write_buffer_mut().is_empty() {
            self.inner.stream.flush().await?;
        }

        while self.inner.pending_ready_for_query_count > 0 {
            let message = self.inner.stream.recv().await?;

            if let BackendMessageFormat::ReadyForQuery = message.format {
                self.handle_ready_for_query(message)?;
            }
        }

        Ok(())
    }

    async fn recv_ready_for_query(&mut self) -> Result<(), Error> {
        let r: ReadyForQuery = self.inner.stream.recv_expect().await?;

        self.inner.pending_ready_for_query_count -= 1;
        self.inner.transaction_status = r.transaction_status;

        Ok(())
    }

    #[inline(always)]
    fn handle_ready_for_query(&mut self, message: ReceivedMessage) -> Result<(), Error> {
        self.inner.pending_ready_for_query_count = self
            .inner
            .pending_ready_for_query_count
            .checked_sub(1)
            .ok_or_else(|| err_protocol!("received more ReadyForQuery messages than expected"))?;

        self.inner.transaction_status = message.decode::<ReadyForQuery>()?.transaction_status;

        Ok(())
    }

    /// Queue a simple query (not prepared) to execute the next time this connection is used.
    ///
    /// Used for rolling back transactions and releasing advisory locks.
    #[inline(always)]
    pub(crate) fn queue_simple_query(&mut self, query: &str) -> Result<(), Error> {
        self.inner.stream.write_msg(Query(query))?;
        self.inner.pending_ready_for_query_count += 1;

        Ok(())
    }

    pub(crate) fn in_transaction(&self) -> bool {
        match self.inner.transaction_status {
            TransactionStatus::Transaction => true,
            TransactionStatus::Error | TransactionStatus::Idle => false,
        }
    }
}

impl Debug for PgConnection {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("PgConnection").finish()
    }
}

impl Connection for PgConnection {
    type Database = Postgres;

    type Options = PgConnectOptions;

    async fn close(mut self) -> Result<(), Error> {
        // The normal, graceful termination procedure is that the frontend sends a Terminate
        // message and immediately closes the connection.

        // On receipt of this message, the backend closes the
        // connection and terminates.
        self.inner.stream.send(Terminate).await?;
        self.inner.stream.shutdown().await?;

        Ok(())
    }

    async fn close_hard(mut self) -> Result<(), Error> {
        self.inner.stream.shutdown().await?;

        Ok(())
    }

    async fn ping(&mut self) -> Result<(), Error> {
        // Users were complaining about this showing up in query statistics on the server.
        // By sending a comment we avoid an error if the connection was in the middle of a rowset
        // self.execute("/* SQLx ping */").map_ok(|_| ()).boxed()

        // The simplest call-and-response that's possible.
        self.write_sync();
        self.wait_until_ready().await
    }

    fn begin(
        &mut self,
    ) -> impl Future<Output = Result<Transaction<'_, Self::Database>, Error>> + Send + '_ {
        Transaction::begin(self, None)
    }

    fn begin_with(
        &mut self,
        statement: impl SqlSafeStr,
    ) -> impl Future<Output = Result<Transaction<'_, Self::Database>, Error>> + Send + '_
    where
        Self: Sized,
    {
        Transaction::begin(self, Some(statement.into_sql_str()))
    }

    fn cached_statements_size(&self) -> usize {
        self.inner.cache_statement.len()
    }

    async fn clear_cached_statements(&mut self) -> Result<(), Error> {
        self.inner.cache_type_oid.clear();

        let mut cleared = 0_usize;

        self.wait_until_ready().await?;

        while let Some((id, _)) = self.inner.cache_statement.remove_lru() {
            self.inner.stream.write_msg(Close::Statement(id))?;
            cleared += 1;
        }

        if cleared > 0 {
            self.write_sync();
            self.inner.stream.flush().await?;

            self.wait_for_close_complete(cleared).await?;
            self.recv_ready_for_query().await?;
        }

        Ok(())
    }

    fn shrink_buffers(&mut self) {
        self.inner.stream.shrink_buffers();
    }

    #[doc(hidden)]
    fn flush(&mut self) -> impl Future<Output = Result<(), Error>> + Send + '_ {
        self.wait_until_ready()
    }

    #[doc(hidden)]
    fn should_flush(&self) -> bool {
        !self.inner.stream.write_buffer().is_empty()
    }
}

// Implement `AsMut<Self>` so that `PgConnection` can be wrapped in
// a `PgAdvisoryLockGuard`.
//
// See: https://github.com/launchbadge/sqlx/issues/2520
impl AsMut<PgConnection> for PgConnection {
    fn as_mut(&mut self) -> &mut PgConnection {
        self
    }
}


#[cfg(test)]
mod wp3_statement_cache_tests {
    use super::*;
    use sqlx_core::net::BudgetError;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Ledger(AtomicUsize);

    impl ResourceBudget for Ledger {
        fn try_reserve(&self, bytes: usize) -> Result<(), BudgetError> {
            self.0.fetch_add(bytes, Ordering::AcqRel);
            Ok(())
        }

        fn release(&self, bytes: usize) {
            self.0.fetch_sub(bytes, Ordering::AcqRel);
        }
    }

    #[test]
    fn wp3_statement_cache_is_exactly_100_and_charged_through_drop() {
        let ledger = Arc::new(Ledger(AtomicUsize::new(0)));
        let owner: Arc<dyn ResourceBudget> = ledger.clone();
        let mut cache = PgStatementCache::new(100, true, Some(owner)).unwrap();
        let base = ledger.0.load(Ordering::Acquire);
        assert!(base > 0);
        let denied_owner: Arc<dyn ResourceBudget> = ledger.clone();
        assert!(PgStatementCache::new(99, true, Some(denied_owner)).is_err());
        let metadata = Arc::new(PgStatementMetadata::default());
        assert!(cache
            .insert("SELECT 1", (StatementId::UNNAMED, metadata))
            .unwrap()
            .is_none());
        assert!(ledger.0.load(Ordering::Acquire) > base);
        drop(cache);
        assert_eq!(ledger.0.load(Ordering::Acquire), 0);
    }
}
