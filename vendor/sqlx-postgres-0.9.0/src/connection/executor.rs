use crate::error::Error;
use crate::executor::{Execute, Executor};
use crate::io::{PortalId, StatementId};
use crate::logger::QueryLogger;
use crate::message::{
    self, BackendMessageFormat, Bind, Close, CommandComplete, DataRow, ParameterDescription, Parse,
    ParseComplete, RowDescription,
};
use crate::statement::Metadata;
use crate::{
    statement::PgStatement, PgArguments, PgConnection, PgQueryResult, PgRow, PgTypeInfo,
    PgValueFormat, Postgres,
};
use futures_core::future::BoxFuture;
use futures_core::stream::BoxStream;
use futures_core::Stream;
use futures_util::TryStreamExt;
use sqlx_core::arguments::Arguments;
use sqlx_core::sql_str::SqlStr;
use sqlx_core::Either;
use std::pin::pin;

// A failed or cancelled preparation may leave an executable prefix, or may
// already have submitted bytes. Fence owned connections without discarding the
// socket, pending response count, or backing debits. Successful submission hands
// response handling back to the existing executor; poisoning is not a rollback.
struct PendingRequest<'c>(Option<&'c mut PgConnection>);

impl<'c> PendingRequest<'c> {
    fn new(conn: &'c mut PgConnection) -> Self {
        Self(Some(conn))
    }

    fn finish(mut self) -> &'c mut PgConnection {
        self.0.take().expect("pending request owns its connection")
    }
}

impl std::ops::Deref for PendingRequest<'_> {
    type Target = PgConnection;

    fn deref(&self) -> &Self::Target {
        self.0
            .as_deref()
            .expect("pending request owns its connection")
    }
}

impl std::ops::DerefMut for PendingRequest<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
            .as_deref_mut()
            .expect("pending request owns its connection")
    }
}

impl Drop for PendingRequest<'_> {
    fn drop(&mut self) {
        if let Some(conn) = self.0.as_deref_mut() {
            if conn.inner.stream.resource_budget().is_some() {
                conn.inner.stream.poison();
            }
        }
    }
}

async fn prepare(
    conn: &mut PgConnection,
    sql: &str,
    arg_types: &[PgTypeInfo],
    metadata: Option<Metadata>,
    persistent: bool,
    resolve_column_origin: bool,
) -> Result<(StatementId, Metadata), Error> {
    let id = if persistent {
        let id = conn.inner.next_statement_id;
        conn.inner.next_statement_id = id.next();
        id
    } else {
        StatementId::UNNAMED
    };

    // build a list of type OIDs to send to the database in the PARSE command
    // we have not yet started the query sequence, so we are *safe* to cleanly make
    // additional queries here to get any missing OIDs
    let param_types = conn.resolve_types(arg_types).await?;

    // flush and wait until we are re-ready
    conn.wait_until_ready().await?;

    // next we send the PARSE command to the server
    conn.inner.stream.write_msg(Parse {
        param_types: &param_types,
        query: sql,
        statement: id,
    })?;

    if metadata.is_none() {
        // get the statement columns and parameters
        conn.inner
            .stream
            .write_msg(message::Describe::Statement(id))?;
    }

    // we ask for the server to immediately send us the result of the PARSE command
    conn.write_sync();
    conn.inner.stream.flush().await?;

    // indicates that the SQL query string is now successfully parsed and has semantic validity
    conn.inner.stream.recv_expect::<ParseComplete>().await?;

    let metadata = if let Some(metadata) = metadata {
        // each SYNC produces one READY FOR QUERY
        conn.recv_ready_for_query().await?;

        // we already have metadata
        metadata
    } else {
        let parameters = recv_desc_params(conn).await?;

        let row_desc = recv_desc_rows(conn).await?;

        // each SYNC produces one READY FOR QUERY
        conn.recv_ready_for_query().await?;

        let metadata = conn
            .resolve_statement_metadata::<true>(Some(parameters), row_desc, resolve_column_origin)
            .await?;

        // ensure that if we did fetch custom data, we wait until we are fully ready before
        // continuing
        conn.wait_until_ready().await?;

        metadata
    };

    Ok((id, metadata))
}

async fn recv_desc_params(conn: &mut PgConnection) -> Result<ParameterDescription, Error> {
    conn.inner.stream.recv_expect().await
}

async fn recv_desc_rows(conn: &mut PgConnection) -> Result<Option<RowDescription>, Error> {
    let rows: Option<RowDescription> = match conn.inner.stream.recv().await? {
        // describes the rows that will be returned when the statement is eventually executed
        message if message.format == BackendMessageFormat::RowDescription => {
            Some(message.decode()?)
        }

        // no data would be returned if this statement was executed
        message if message.format == BackendMessageFormat::NoData => None,

        message => {
            return Err(err_protocol!(
                "expecting RowDescription or NoData but received {:?}",
                message.format
            ));
        }
    };

    Ok(rows)
}

impl PgConnection {
    // wait for CloseComplete to indicate a statement was closed
    pub(super) async fn wait_for_close_complete(&mut self, mut count: usize) -> Result<(), Error> {
        // we need to wait for the [CloseComplete] to be returned from the server
        while count > 0 {
            match self.inner.stream.recv().await? {
                message if message.format == BackendMessageFormat::PortalSuspended => {
                    // there was an open portal
                    // this can happen if the last time a statement was used it was not fully executed
                }

                message if message.format == BackendMessageFormat::CloseComplete => {
                    // successfully closed the statement (and freed up the server resources)
                    count -= 1;
                }

                message => {
                    return Err(err_protocol!(
                        "expecting PortalSuspended or CloseComplete but received {:?}",
                        message.format
                    ));
                }
            }
        }

        Ok(())
    }

    #[inline(always)]
    pub(crate) fn write_sync(&mut self) {
        if self.inner.stream.write_msg(message::Sync).is_err() {
            self.inner.stream.poison();
            return;
        }
        // all SYNC messages will return a ReadyForQuery
        if let Some(count) = self.inner.pending_ready_for_query_count.checked_add(1) {
            self.inner.pending_ready_for_query_count = count;
        } else {
            self.inner.stream.poison();
        }
    }

    async fn get_or_prepare(
        &mut self,
        sql: &str,
        parameters: &[PgTypeInfo],
        persistent: bool,
        // optional metadata that was provided by the user, this means they are reusing
        // a statement object
        metadata: Option<Metadata>,
        resolve_column_origin: bool,
    ) -> Result<(StatementId, Metadata), Error> {
        let mut request = PendingRequest::new(self);
        let result = request
            .get_or_prepare_inner(sql, parameters, persistent, metadata, resolve_column_origin)
            .await?;
        request.finish();
        Ok(result)
    }

    async fn get_or_prepare_inner(
        &mut self,
        sql: &str,
        parameters: &[PgTypeInfo],
        persistent: bool,
        metadata: Option<Metadata>,
        resolve_column_origin: bool,
    ) -> Result<(StatementId, Metadata), Error> {
        if let Some(statement) = self.inner.cache_statement.get_mut(sql) {
            return Ok((*statement).clone());
        }

        let statement = prepare(
            self,
            sql,
            parameters,
            metadata,
            persistent,
            resolve_column_origin,
        )
        .await?;

        if persistent && self.inner.cache_statement.is_enabled() {
            let replaced = match self.inner.cache_statement.insert(sql, statement.clone()) {
                Ok(replaced) => replaced,
                Err(error) => {
                    self.inner.stream.poison();
                    return Err(error);
                }
            };
            if let Some((id, _)) = replaced {
                self.inner.stream.write_msg(Close::Statement(id))?;
                self.write_sync();

                self.inner.stream.flush().await?;

                self.wait_for_close_complete(1).await?;
                self.recv_ready_for_query().await?;
            }
        }

        Ok(statement)
    }

    pub(crate) async fn run<'e, 'c: 'e, 'q: 'e>(
        &'c mut self,
        query: SqlStr,
        arguments: Option<PgArguments>,
        persistent: bool,
        metadata_opt: Option<Metadata>,
    ) -> Result<impl Stream<Item = Result<Either<PgQueryResult, PgRow>, Error>> + 'e, Error> {
        let mut logger = QueryLogger::new(query, self.inner.log_settings.clone());
        let sql = logger.sql().as_str();

        let mut request = PendingRequest::new(self);

        // before we continue, wait until we are "ready" to accept more queries
        request.wait_until_ready().await?;

        let mut metadata: Metadata;

        let format = if let Some(mut arguments) = arguments {
            // Check this before we write anything to the stream.
            //
            // Note: Postgres actually interprets this value as unsigned,
            // making the max number of parameters 65535, not 32767
            // https://github.com/launchbadge/sqlx/issues/3464
            // https://www.postgresql.org/docs/current/limits.html
            let num_params = u16::try_from(arguments.len()).map_err(|_| {
                err_protocol!(
                    "PgConnection::run(): too many arguments for query: {}",
                    arguments.len()
                )
            })?;

            // prepare the statement if this our first time executing it
            // always return the statement ID here
            let (statement, metadata_) = request
                .get_or_prepare(sql, &arguments.types, persistent, metadata_opt, false)
                .await?;

            metadata = metadata_;

            // patch holes created during encoding
            arguments
                .apply_patches(&mut request, &metadata.parameters)
                .await?;

            // consume messages till `ReadyForQuery` before bind and execute
            request.wait_until_ready().await?;

            // bind to attach the arguments to the statement and create a portal
            request.inner.stream.write_msg(Bind {
                portal: PortalId::UNNAMED,
                statement,
                formats: &[PgValueFormat::Binary],
                num_params,
                params: &arguments.buffer,
                result_formats: &[PgValueFormat::Binary],
            })?;

            // executes the portal up to the passed limit
            // the protocol-level limit acts nearly identically to the `LIMIT` in SQL
            request.inner.stream.write_msg(message::Execute {
                portal: PortalId::UNNAMED,
                // Non-zero limits cause query plan pessimization by disabling parallel workers:
                // https://github.com/launchbadge/sqlx/issues/3673
                limit: 0,
            })?;
            // From https://www.postgresql.org/docs/current/protocol-flow.html:
            //
            // "An unnamed portal is destroyed at the end of the transaction, or as
            // soon as the next Bind statement specifying the unnamed portal as
            // destination is issued. (Note that a simple Query message also
            // destroys the unnamed portal."

            // we ask the database server to close the unnamed portal and free the associated resources
            // earlier - after the execution of the current query.
            request
                .inner
                .stream
                .write_msg(Close::Portal(PortalId::UNNAMED))?;

            // finally, [Sync] asks postgres to process the messages that we sent and respond with
            // a [ReadyForQuery] message when it's completely done. Theoretically, we could send
            // dozens of queries before a [Sync] and postgres can handle that. Execution on the server
            // is still serial but it would reduce round-trips. Some kind of builder pattern that is
            // termed batching might suit this.
            request.write_sync();

            // prepared statements are binary
            PgValueFormat::Binary
        } else {
            // Query will trigger a ReadyForQuery
            request.queue_simple_query(sql)?;

            // metadata starts out as "nothing"
            metadata = Metadata::empty(request.inner.stream.resource_budget().cloned())?;

            // and unprepared statements are text
            PgValueFormat::Text
        };

        request.inner.stream.flush().await?;

        let connection = request.finish();

        Ok(try_stream! {
            loop {
                let message = connection.inner.stream.recv().await?;

                match message.format {
                    BackendMessageFormat::BindComplete
                    | BackendMessageFormat::ParseComplete
                    | BackendMessageFormat::ParameterDescription
                    | BackendMessageFormat::NoData
                    // unnamed portal has been closed
                    | BackendMessageFormat::CloseComplete
                    => {
                        // harmless messages to ignore
                    }

                    // "Execute phase is always terminated by the appearance of
                    // exactly one of these messages: CommandComplete,
                    // EmptyQueryResponse (if the portal was created from an
                    // empty query string), ErrorResponse, or PortalSuspended"
                    BackendMessageFormat::CommandComplete => {
                        // a SQL command completed normally
                        let cc: CommandComplete = message.decode()?;

                        let rows_affected = cc.rows_affected();
                        logger.increase_rows_affected(rows_affected);
                        r#yield!(Either::Left(PgQueryResult {
                            rows_affected,
                        }));
                    }

                    BackendMessageFormat::EmptyQueryResponse => {
                        // empty query string passed to an unprepared execute
                    }

                    // Message::ErrorResponse is handled in connection.stream.recv()

                    // incomplete query execution has finished
                    BackendMessageFormat::PortalSuspended => {}

                    // indicates that a *new* set of rows are about to be returned
                    BackendMessageFormat::RowDescription => {
                        let new_metadata = connection.resolve_statement_metadata::<false>(
                            None,
                            Some(message.decode()?),
                            false,
                        ).await?;

                        metadata = new_metadata;
                    }

                    BackendMessageFormat::DataRow => {
                        logger.increment_rows_returned();

                        // one of the set of rows returned by a SELECT, FETCH, etc query
                        let count = message.contents.get(..2)
                            .map(|bytes| usize::from(u16::from_be_bytes([bytes[0], bytes[1]])))
                            .ok_or_else(|| Error::Io(std::io::ErrorKind::InvalidData.into()))?;
                        if count != metadata.columns.len() {
                            Err(Error::Io(std::io::ErrorKind::InvalidData.into()))?;
                        }
                        let data: DataRow = message.decode()?;
                        let row = PgRow {
                            data,
                            format,
                            metadata: metadata.clone(),
                        };

                        r#yield!(Either::Right(row));
                    }

                    BackendMessageFormat::ReadyForQuery => {
                        // processing of the query string is complete
                        connection.handle_ready_for_query(message)?;
                        break;
                    }

                    _ => {
                        return Err(err_protocol!(
                            "execute: unexpected message: {:?}",
                            message.format
                        ));
                    }
                }
            }

            Ok(())
        })
    }
}

impl<'c> Executor<'c> for &'c mut PgConnection {
    type Database = Postgres;

    fn fetch_many<'e, 'q, E>(
        self,
        mut query: E,
    ) -> BoxStream<'e, Result<Either<PgQueryResult, PgRow>, Error>>
    where
        'c: 'e,
        E: Execute<'q, Self::Database>,
        'q: 'e,
        E: 'q,
    {
        // False positive: https://github.com/rust-lang/rust-clippy/issues/12560
        #[allow(clippy::map_clone)]
        let metadata = query.statement().map(|s| s.metadata.clone());
        let arguments = query.take_arguments().map_err(Error::Encode);
        let persistent = query.persistent();
        let sql = query.sql();

        Box::pin(try_stream! {
            let arguments = arguments?;
            let mut s = pin!(self.run(sql, arguments, persistent, metadata).await?);

            while let Some(v) = s.try_next().await? {
                r#yield!(v);
            }

            Ok(())
        })
    }

    fn fetch_optional<'e, 'q, E>(self, mut query: E) -> BoxFuture<'e, Result<Option<PgRow>, Error>>
    where
        'c: 'e,
        E: Execute<'q, Self::Database>,
        'q: 'e,
        E: 'q,
    {
        // False positive: https://github.com/rust-lang/rust-clippy/issues/12560
        #[allow(clippy::map_clone)]
        let metadata = query.statement().map(|s| s.metadata.clone());
        let arguments = query.take_arguments().map_err(Error::Encode);
        let persistent = query.persistent();

        Box::pin(async move {
            let sql = query.sql();
            let arguments = arguments?;
            let mut s = pin!(self.run(sql, arguments, persistent, metadata).await?);

            // With deferred constraints we need to check all responses as we
            // could get a OK response (with uncommitted data), only to get an
            // error response after (when the deferred constraint is actually
            // checked).
            let mut ret = None;
            while let Some(result) = s.try_next().await? {
                match result {
                    Either::Right(r) if ret.is_none() => ret = Some(r),
                    _ => {}
                }
            }
            Ok(ret)
        })
    }

    fn prepare_with<'e>(
        self,
        sql: SqlStr,
        parameters: &'e [PgTypeInfo],
    ) -> BoxFuture<'e, Result<PgStatement, Error>>
    where
        'c: 'e,
    {
        Box::pin(async move {
            self.wait_until_ready().await?;

            let (_, metadata) = self
                .get_or_prepare(sql.as_str(), parameters, true, None, true)
                .await?;

            Ok(PgStatement { sql, metadata })
        })
    }

    #[cfg(feature = "offline")]
    fn describe<'e>(
        self,
        sql: SqlStr,
    ) -> BoxFuture<'e, Result<crate::describe::Describe<Self::Database>, Error>>
    where
        'c: 'e,
    {
        Box::pin(async move {
            self.wait_until_ready().await?;

            let (stmt_id, metadata) = self
                .get_or_prepare(sql.as_str(), &[], true, None, true)
                .await?;

            let nullable = self.get_nullable_for_columns(stmt_id, &metadata).await?;

            Ok(crate::describe::Describe {
                columns: metadata.columns.clone(),
                nullable,
                parameters: Some(Either::Left(metadata.parameters.clone())),
            })
        })
    }
}

#[cfg(test)]
mod repair1_tests {
    use super::*;
    use crate::statement::custody_test_support::Ledger;
    use crate::{PgConnectOptions, PgSslMode};
    use sqlx_core::sql_str::SqlSafeStr;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn abandoned_prefix_case(phase: u8) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(10)))
                .unwrap();
            let mut length = [0; 4];
            socket.read_exact(&mut length).unwrap();
            let size = u32::from_be_bytes(length) as usize;
            assert!((4..1024).contains(&size));
            socket.read_exact(&mut vec![0; size - 4]).unwrap();
            for (tag, body) in [
                (b'R', &[0, 0, 0, 0][..]),
                (b'K', &[0, 0, 0, 1, 0, 0, 0, 2][..]),
                (b'Z', &b"I"[..]),
            ] {
                socket.write_all(&[tag]).unwrap();
                socket
                    .write_all(&u32::try_from(body.len() + 4).unwrap().to_be_bytes())
                    .unwrap();
                socket.write_all(body).unwrap();
            }
            let mut received = Vec::new();
            socket.read_to_end(&mut received).unwrap();
            received
        });
        let budget = Ledger::new(usize::MAX);
        let (flush_failed, queued) = sqlx_core::rt::test_block_on(async {
            let options = PgConnectOptions::new_without_pgpass()
                .host("127.0.0.1")
                .port(port)
                .username("test")
                .ssl_mode(PgSslMode::Disable);
            let mut conn = PgConnection::establish_with_resource_budget(&options, budget.clone())
                .await
                .unwrap();
            if phase >= 2 {
                let metadata = Metadata::empty(Some(budget.clone())).unwrap();
                conn.inner
                    .cache_statement
                    .insert("SELECT $1", (StatementId::UNNAMED, metadata))
                    .unwrap();
            }
            let retained = budget.held();
            budget.limit(retained);
            if phase == 1 {
                let query = "x".repeat(8183); // Parse fills the 8192-byte owned write buffer.
                assert!(conn
                    .get_or_prepare(&query, &[], false, None, false)
                    .await
                    .is_err());
            } else {
                let arguments = if phase >= 2 {
                    let mut arguments = PgArguments::default();
                    // Bind fills the buffer, or leaves exactly one Execute's ten bytes.
                    arguments
                        .add("x".repeat(if phase == 2 { 8171 } else { 8161 }))
                        .unwrap();
                    Some(arguments)
                } else {
                    None
                };
                assert!(conn
                    .run("SELECT $1".into_sql_str(), arguments, true, None)
                    .await
                    .is_err());
            }
            let queued = !conn.inner.stream.write_buffer().is_empty();
            let flush_failed = conn.inner.stream.flush().await.is_err();
            assert_eq!(
                budget.held(),
                retained,
                "poison must retain allocation custody"
            );
            drop(conn);
            (flush_failed, queued)
        });
        let received = server.join().unwrap();
        assert_eq!(budget.held(), 0);
        assert!(queued, "fixture must reach an intermediate append");
        assert!(
            flush_failed,
            "reuse flushed an abandoned request: {} bytes",
            received.len()
        );
        assert!(received.is_empty(), "abandoned protocol reached the peer");
    }

    #[test]
    fn simple_metadata_denial_cannot_flush_abandoned_query() {
        abandoned_prefix_case(0);
    }
    #[test]
    fn describe_denial_cannot_flush_abandoned_parse() {
        abandoned_prefix_case(1);
    }
    #[test]
    fn execute_denial_cannot_flush_abandoned_bind() {
        abandoned_prefix_case(2);
    }
    #[test]
    fn close_denial_cannot_flush_abandoned_execute() {
        abandoned_prefix_case(3);
    }
}
