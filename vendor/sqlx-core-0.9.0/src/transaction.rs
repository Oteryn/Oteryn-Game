use std::fmt::{self, Debug, Formatter};
use std::future::{self, Future};
use std::ops::{Deref, DerefMut};

use futures_core::future::BoxFuture;

use crate::database::Database;
use crate::error::Error;
use crate::pool::MaybePoolConnection;
use crate::sql_str::{AssertSqlSafe, SqlSafeStr, SqlStr};

/// Generic management of database transactions.
///
/// This trait should not be used, except when implementing [`Connection`].
pub trait TransactionManager {
    type Database: Database;

    /// Begin a new transaction or establish a savepoint within the active transaction.
    ///
    /// If this is a new transaction, `statement` may be used instead of the
    /// default "BEGIN" statement.
    ///
    /// If we are already inside a transaction and `statement.is_some()`, then
    /// `Error::InvalidSavePoint` is returned without running any statements.
    fn begin(
        conn: &mut <Self::Database as Database>::Connection,
        statement: Option<SqlStr>,
    ) -> impl Future<Output = Result<(), Error>> + Send + '_;

    /// Commit the active transaction or release the most recent savepoint.
    fn commit(
        conn: &mut <Self::Database as Database>::Connection,
    ) -> impl Future<Output = Result<(), Error>> + Send + '_;

    /// Abort the active transaction or restore from the most recent savepoint.
    fn rollback(
        conn: &mut <Self::Database as Database>::Connection,
    ) -> impl Future<Output = Result<(), Error>> + Send + '_;

    /// Starts to abort the active transaction or restore from the most recent snapshot.
    fn start_rollback(conn: &mut <Self::Database as Database>::Connection);

    /// Returns the current transaction depth.
    ///
    /// Transaction depth indicates the level of nested transactions:
    /// - Level 0: No active transaction.
    /// - Level 1: A transaction is active.
    /// - Level 2 or higher: A transaction is active and one or more SAVEPOINTs have been created within it.
    fn get_transaction_depth(conn: &<Self::Database as Database>::Connection) -> usize;
}

/// An in-progress database transaction or savepoint.
///
/// A transaction starts with a call to [`Pool::begin`] or [`Connection::begin`].
///
/// A transaction should end with a call to [`commit`] or [`rollback`]. If neither are called
/// before the transaction goes out-of-scope, [`rollback`] is called. In other
/// words, [`rollback`] is called on `drop` if the transaction is still in-progress.
///
/// A savepoint is a special mark inside a transaction that allows all commands that are
/// executed after it was established to be rolled back, restoring the transaction state to
/// what it was at the time of the savepoint.
///
/// A transaction can be used as an [`Executor`] when performing queries:
/// ```rust,no_run
/// # use sqlx_core::acquire::Acquire;
/// # async fn example() -> sqlx::Result<()> {
/// # let id = 1;
/// # let mut conn: sqlx::PgConnection = unimplemented!();
/// let mut tx = conn.begin().await?;
///
/// let result = sqlx::query("DELETE FROM \"testcases\" WHERE id = $1")
///     .bind(id)
///     .execute(&mut *tx)
///     .await?
///     .rows_affected();
///
/// tx.commit().await
/// # }
/// ```
/// [`Executor`]: crate::executor::Executor
/// [`Connection::begin`]: crate::connection::Connection::begin()
/// [`Pool::begin`]: crate::pool::Pool::begin()
/// [`commit`]: Self::commit()
/// [`rollback`]: Self::rollback()
pub struct Transaction<'c, DB>
where
    DB: Database,
{
    // Kept takeable so the M05 owned-root finalizer can transfer custody out of
    // this `Drop` type without unsafe field moves.
    connection: Option<MaybePoolConnection<'c, DB>>,
    open: bool,
}

impl<'c, DB> Transaction<'c, DB>
where
    DB: Database,
{
    #[doc(hidden)]
    pub fn begin(
        conn: impl Into<MaybePoolConnection<'c, DB>>,
        statement: Option<SqlStr>,
    ) -> BoxFuture<'c, Result<Self, Error>> {
        let conn = conn.into();

        Box::pin(async move {
            let mut tx = Self {
                connection: Some(conn),

                // If the call to `begin` fails or doesn't complete we want to attempt a rollback in case the transaction was started.
                open: true,
            };

            DB::TransactionManager::begin(tx.connection_mut(), statement).await?;

            Ok(tx)
        })
    }

    /// Commits this transaction or savepoint.
    pub async fn commit(mut self) -> Result<(), Error> {
        DB::TransactionManager::commit(self.connection_mut()).await?;
        self.open = false;

        Ok(())
    }

    /// Aborts this transaction or savepoint.
    pub async fn rollback(mut self) -> Result<(), Error> {
        DB::TransactionManager::rollback(self.connection_mut()).await?;
        self.open = false;

        Ok(())
    }

    #[inline]
    fn connection(&self) -> &MaybePoolConnection<'c, DB> {
        // The connection is absent only after the consuming M05 seam has made
        // the transaction inert; safe callers cannot observe that value again.
        self.connection
            .as_ref()
            .expect("transaction connection is present while transaction is observable")
    }

    #[inline]
    fn connection_mut(&mut self) -> &mut MaybePoolConnection<'c, DB> {
        self.connection
            .as_mut()
            .expect("transaction connection is present while transaction is observable")
    }
}

impl<DB> Transaction<'static, DB>
where
    DB: Database,
{
    /// Finalizes an owned M05 root transaction and returns its exact connection.
    ///
    /// This is an Oteryn-internal custody seam, not a stable SQLx API.
    #[doc(hidden)]
    pub async fn oteryn_m05_commit(
        mut self,
    ) -> (MaybePoolConnection<'static, DB>, Result<(), Error>) {
        let result = DB::TransactionManager::commit(self.connection_mut()).await;
        self.finish_oteryn_m05(result)
    }

    /// Rolls back an owned M05 root transaction and returns its exact connection.
    ///
    /// This is an Oteryn-internal custody seam, not a stable SQLx API.
    #[doc(hidden)]
    pub async fn oteryn_m05_rollback(
        mut self,
    ) -> (MaybePoolConnection<'static, DB>, Result<(), Error>) {
        let result = DB::TransactionManager::rollback(self.connection_mut()).await;
        self.finish_oteryn_m05(result)
    }

    fn finish_oteryn_m05(
        mut self,
        result: Result<(), Error>,
    ) -> (MaybePoolConnection<'static, DB>, Result<(), Error>) {
        if result.is_err() {
            // Match ordinary Transaction Drop exactly once before transferring
            // custody; making the wrapper inert prevents a second invocation.
            DB::TransactionManager::start_rollback(self.connection_mut());
        }
        self.open = false;
        let connection = self
            .connection
            .take()
            .expect("owned M05 transaction retains its connection until finalization");
        (connection, result)
    }
}

// NOTE: fails to compile due to lack of lazy normalization
// impl<'c, 't, DB: Database> crate::executor::Executor<'t>
//     for &'t mut crate::transaction::Transaction<'c, DB>
// where
//     &'c mut DB::Connection: Executor<'c, Database = DB>,
// {
//     type Database = DB;
//
//
//
//     fn fetch_many<'e, 'q: 'e, E: 'q>(
//         self,
//         query: E,
//     ) -> futures_core::stream::BoxStream<
//         'e,
//         Result<
//             crate::Either<<DB as crate::database::Database>::QueryResult, DB::Row>,
//             crate::error::Error,
//         >,
//     >
//     where
//         't: 'e,
//         E: crate::executor::Execute<'q, Self::Database>,
//     {
//         (&mut **self).fetch_many(query)
//     }
//
//     fn fetch_optional<'e, 'q: 'e, E: 'q>(
//         self,
//         query: E,
//     ) -> futures_core::future::BoxFuture<'e, Result<Option<DB::Row>, crate::error::Error>>
//     where
//         't: 'e,
//         E: crate::executor::Execute<'q, Self::Database>,
//     {
//         (&mut **self).fetch_optional(query)
//     }
//
//     fn prepare_with<'e, 'q: 'e>(
//         self,
//         sql: &'q str,
//         parameters: &'e [<Self::Database as crate::database::Database>::TypeInfo],
//     ) -> futures_core::future::BoxFuture<
//         'e,
//         Result<
//             <Self::Database as crate::database::Database>::Statement<'q>,
//             crate::error::Error,
//         >,
//     >
//     where
//         't: 'e,
//     {
//         (&mut **self).prepare_with(sql, parameters)
//     }
//
//     #[doc(hidden)]
//     #[cfg(feature = "offline")]
//     fn describe<'e, 'q: 'e>(
//         self,
//         query: &'q str,
//     ) -> futures_core::future::BoxFuture<
//         'e,
//         Result<crate::describe::Describe<Self::Database>, crate::error::Error>,
//     >
//     where
//         't: 'e,
//     {
//         (&mut **self).describe(query)
//     }
// }

impl<DB> Debug for Transaction<'_, DB>
where
    DB: Database,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // TODO: Show the full type <..<..<..
        f.debug_struct("Transaction").finish()
    }
}

impl<DB> Deref for Transaction<'_, DB>
where
    DB: Database,
{
    type Target = DB::Connection;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.connection()
    }
}

impl<DB> DerefMut for Transaction<'_, DB>
where
    DB: Database,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.connection_mut()
    }
}

// Implement `AsMut<DB::Connection>` so `Transaction` can be given to a
// `PgAdvisoryLockGuard`.
//
// See: https://github.com/launchbadge/sqlx/issues/2520
impl<DB: Database> AsMut<DB::Connection> for Transaction<'_, DB> {
    fn as_mut(&mut self) -> &mut DB::Connection {
        self.connection_mut()
    }
}

impl<'t, DB: Database> crate::acquire::Acquire<'t> for &'t mut Transaction<'_, DB> {
    type Database = DB;

    type Connection = &'t mut <DB as Database>::Connection;

    #[inline]
    fn acquire(self) -> BoxFuture<'t, Result<Self::Connection, Error>> {
        Box::pin(future::ready(Ok(&mut **self)))
    }

    #[inline]
    fn begin(self) -> BoxFuture<'t, Result<Transaction<'t, DB>, Error>> {
        Transaction::begin(&mut **self, None)
    }
}

impl<DB> Drop for Transaction<'_, DB>
where
    DB: Database,
{
    fn drop(&mut self) {
        if self.open {
            // starts a rollback operation

            // what this does depends on the database but generally this means we queue a rollback
            // operation that will happen on the next asynchronous invocation of the underlying
            // connection (including if the connection is returned to a pool)

            if let Some(connection) = self.connection.as_mut() {
                DB::TransactionManager::start_rollback(connection);
            }
        }
    }
}

pub fn begin_ansi_transaction_sql(depth: usize) -> SqlStr {
    if depth == 0 {
        "BEGIN".into_sql_str()
    } else {
        AssertSqlSafe(format!("SAVEPOINT _sqlx_savepoint_{depth}")).into_sql_str()
    }
}

pub fn commit_ansi_transaction_sql(depth: usize) -> SqlStr {
    if depth == 1 {
        "COMMIT".into_sql_str()
    } else {
        AssertSqlSafe(format!("RELEASE SAVEPOINT _sqlx_savepoint_{}", depth - 1)).into_sql_str()
    }
}

pub fn rollback_ansi_transaction_sql(depth: usize) -> SqlStr {
    if depth == 1 {
        "ROLLBACK".into_sql_str()
    } else {
        AssertSqlSafe(format!(
            "ROLLBACK TO SAVEPOINT _sqlx_savepoint_{}",
            depth - 1
        ))
        .into_sql_str()
    }
}

#[cfg(all(test, feature = "any"))]
mod oteryn_m05_finality_tests {
    use super::*;
    use crate::any::{
        Any, AnyArguments, AnyConnection, AnyConnectionBackend, AnyQueryResult, AnyRow,
        AnyStatement, AnyTypeInfo,
    };
    use crate::pool::MaybePoolConnection;
    use crate::sql_str::SqlStr;
    use either::Either;
    use futures_core::future::BoxFuture;
    use futures_core::stream::BoxStream;
    use futures_util::stream;
    use std::fmt;
    use std::pin::pin;
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll, Waker};

    #[derive(Clone, Copy, Debug)]
    enum Finish {
        Ok,
        Error(&'static str),
        Pending,
    }

    #[derive(Debug)]
    struct State {
        identity: usize,
        commit: Finish,
        rollback: Finish,
        commit_calls: usize,
        rollback_calls: usize,
        start_rollback_calls: usize,
        waiter: Option<Waker>,
    }

    #[derive(Clone)]
    struct Backend(Arc<Mutex<State>>);

    impl fmt::Debug for Backend {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Backend").finish_non_exhaustive()
        }
    }

    fn finish<'a>(
        state: &'a Arc<Mutex<State>>,
        operation: bool,
    ) -> BoxFuture<'a, Result<(), Error>> {
        Box::pin(std::future::poll_fn(move |cx| {
            let mut state = state.lock().unwrap();
            let finish = if operation {
                state.commit_calls += 1;
                state.commit
            } else {
                state.rollback_calls += 1;
                state.rollback
            };
            match finish {
                Finish::Ok => Poll::Ready(Ok(())),
                Finish::Error(message) => Poll::Ready(Err(Error::Protocol(message.into()))),
                Finish::Pending => {
                    state.waiter = Some(cx.waker().clone());
                    Poll::Pending
                }
            }
        }))
    }

    impl AnyConnectionBackend for Backend {
        fn name(&self) -> &str {
            "oteryn-m05-test"
        }
        fn close(self: Box<Self>) -> BoxFuture<'static, crate::Result<()>> {
            Box::pin(async { Ok(()) })
        }
        fn close_hard(self: Box<Self>) -> BoxFuture<'static, crate::Result<()>> {
            Box::pin(async { Ok(()) })
        }
        fn ping(&mut self) -> BoxFuture<'_, crate::Result<()>> {
            Box::pin(async { Ok(()) })
        }
        fn begin(&mut self, _: Option<SqlStr>) -> BoxFuture<'_, crate::Result<()>> {
            Box::pin(async { Ok(()) })
        }
        fn commit(&mut self) -> BoxFuture<'_, crate::Result<()>> {
            finish(&self.0, true)
        }
        fn rollback(&mut self) -> BoxFuture<'_, crate::Result<()>> {
            finish(&self.0, false)
        }
        fn start_rollback(&mut self) {
            self.0.lock().unwrap().start_rollback_calls += 1;
        }
        fn get_transaction_depth(&self) -> usize {
            1
        }
        fn shrink_buffers(&mut self) {}
        fn flush(&mut self) -> BoxFuture<'_, crate::Result<()>> {
            Box::pin(async { Ok(()) })
        }
        fn should_flush(&self) -> bool {
            false
        }
        fn fetch_many(
            &mut self,
            _: SqlStr,
            _: bool,
            _: Option<AnyArguments>,
        ) -> BoxStream<'_, crate::Result<Either<AnyQueryResult, AnyRow>>> {
            Box::pin(stream::empty())
        }
        fn fetch_optional(
            &mut self,
            _: SqlStr,
            _: bool,
            _: Option<AnyArguments>,
        ) -> BoxFuture<'_, crate::Result<Option<AnyRow>>> {
            Box::pin(async { Ok(None) })
        }
        fn prepare_with<'c, 'q: 'c>(
            &'c mut self,
            _: SqlStr,
            _: &[AnyTypeInfo],
        ) -> BoxFuture<'c, crate::Result<AnyStatement>> {
            Box::pin(async { Err(Error::Protocol("unused test prepare".into())) })
        }
    }

    fn transaction(
        commit: Finish,
        rollback: Finish,
    ) -> (Transaction<'static, Any>, Arc<Mutex<State>>) {
        let state = Arc::new(Mutex::new(State {
            identity: 0,
            commit,
            rollback,
            commit_calls: 0,
            rollback_calls: 0,
            start_rollback_calls: 0,
            waiter: None,
        }));
        let connection = Box::leak(Box::new(AnyConnection {
            backend: Box::new(Backend(state.clone())),
        }));
        state.lock().unwrap().identity = connection as *const AnyConnection as usize;
        (
            Transaction {
                connection: Some(MaybePoolConnection::Connection(connection)),
                open: true,
            },
            state,
        )
    }

    fn assert_connection_identity(
        connection: &MaybePoolConnection<'static, Any>,
        state: &Arc<Mutex<State>>,
    ) {
        match connection {
            MaybePoolConnection::Connection(connection) => {
                assert_eq!(
                    *connection as *const AnyConnection as usize,
                    state.lock().unwrap().identity
                );
            }
            MaybePoolConnection::PoolConnection(_) => {
                panic!("instrumented connection changed custody variant")
            }
        }
    }

    fn run_ready<F: Future>(future: F) -> F::Output {
        let mut future = pin!(future);
        let waker = futures_util::task::noop_waker();
        let mut context = Context::from_waker(&waker);
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("instrumented operation unexpectedly remained pending"),
        }
    }

    #[test]
    fn commit_and_rollback_success_retain_exact_connection_without_drop_rollback() {
        let (tx, commit_state) = transaction(Finish::Ok, Finish::Ok);
        let (connection, result) = run_ready(tx.oteryn_m05_commit());
        assert!(result.is_ok());
        assert_connection_identity(&connection, &commit_state);
        assert_eq!(commit_state.lock().unwrap().start_rollback_calls, 0);

        let (tx, rollback_state) = transaction(Finish::Ok, Finish::Ok);
        let (connection, result) = run_ready(tx.oteryn_m05_rollback());
        assert!(result.is_ok());
        assert_connection_identity(&connection, &rollback_state);
        assert_eq!(rollback_state.lock().unwrap().start_rollback_calls, 0);
    }

    #[test]
    fn finalization_errors_preserve_original_error_and_start_rollback_once() {
        for commit in [true, false] {
            let (tx, state) = transaction(
                Finish::Error("commit-original"),
                Finish::Error("rollback-original"),
            );
            let (connection, result) = if commit {
                run_ready(tx.oteryn_m05_commit())
            } else {
                run_ready(tx.oteryn_m05_rollback())
            };
            let expected = if commit {
                "commit-original"
            } else {
                "rollback-original"
            };
            assert!(matches!(result, Err(Error::Protocol(message)) if message == expected));
            assert_connection_identity(&connection, &state);
            drop(connection);
            let state = state.lock().unwrap();
            assert_eq!(state.start_rollback_calls, 1);
            assert_eq!(
                (state.commit_calls, state.rollback_calls),
                if commit { (1, 0) } else { (0, 1) }
            );
        }
    }

    #[test]
    fn cancelling_pending_finalization_has_no_success_and_uses_ordinary_drop_rollback() {
        for commit in [true, false] {
            let (tx, state) = transaction(Finish::Pending, Finish::Pending);
            let mut future: std::pin::Pin<Box<dyn Future<Output = _>>> = if commit {
                Box::pin(tx.oteryn_m05_commit())
            } else {
                Box::pin(tx.oteryn_m05_rollback())
            };
            let waker = futures_util::task::noop_waker();
            let mut context = Context::from_waker(&waker);
            assert!(matches!(future.as_mut().poll(&mut context), Poll::Pending));
            drop(future);
            let state = state.lock().unwrap();
            assert_eq!(state.start_rollback_calls, 1);
            assert_eq!(
                (state.commit_calls, state.rollback_calls),
                if commit { (1, 0) } else { (0, 1) }
            );
        }
    }
}
