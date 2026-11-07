//! A historical depot page must not establish a view for a displaced connection.
use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub(super) async fn exercise(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    pool: &sqlx::PgPool,
    deposit: DepotIntent,
) -> TestResult {
    let session = admitted.game_session_id;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = crate::gameplay_transport::connection::serve_admitted(
        &mut server,
        admitted,
        owner,
        SCENARIO_LIVENESS,
    );
    let exchange = async {
        loop {
            let frame = read_frame(&mut client).await?;
            if oteryn_protocol_oteryn::decode_wire_envelope(&frame)?.message_type()
                == oteryn_protocol_oteryn::MessageType::SnapshotCommit
            {
                break;
            }
        }
        let browse = DepotIntent {
            action: DepotAction::Browse,
            ..deposit
        };
        // A fresh browse must fail closed; a terminal replay must preserve
        // MOVED but cannot reestablish a current page for the displaced session.
        for (command_id, intent, expected, next_browse) in [
            (1, browse, DepotOutcome::Rejected, 2),
            (3, deposit, DepotOutcome::Moved, 4),
        ] {
            // Block only the page SELECT after admission has checked this session.
            // This is privileged fixture scheduling, not a production lock strategy.
            let mut blocker = pool.begin().await?;
            sqlx::query("LOCK TABLE game_item_character_depot_locations IN ACCESS EXCLUSIVE MODE")
                .execute(&mut *blocker)
                .await?;
            let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
                .fetch_one(&mut *blocker)
                .await?;
            send(&mut client, command_id, intent).await?;
            tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let blocked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid)))")
                    .bind(pid).fetch_one(pool).await?;
                if blocked { return Ok::<(), sqlx::Error>(()); }
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        }).await.map_err(|_| format!("depot command {command_id} did not block on its page read"))??;
            sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation=current_generation+1 WHERE game_session_id=encode($1,'hex')::uuid")
            .bind(session.as_bytes().as_slice()).execute(pool).await?;
            blocker.commit().await?;
            let reply = read_reply(&mut client, command_id).await?;
            assert_eq!(reply.outcome, expected);
            assert_eq!(reply.town, None);
            assert_eq!(reply.total, 0);
            assert!(reply.entries.is_empty());

            // Restore fixture generation and qualify the same connection's next
            // current page, rather than accepting a route that refuses every read.
            sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation=current_generation-1 WHERE game_session_id=encode($1,'hex')::uuid")
            .bind(session.as_bytes().as_slice()).execute(pool).await?;
            send(&mut client, next_browse, browse).await?;
            let reply = read_reply(&mut client, next_browse).await?;
            assert_eq!(reply.outcome, DepotOutcome::Opened);
            assert_eq!(reply.town, Some(0));
            assert_eq!(reply.box_number, 17);
            assert_eq!(reply.page, 0);
            assert_eq!(reply.total, 1);
            assert_eq!(reply.entries.len(), 1);
            assert_eq!(reply.entries[0].ordinal, 1);
            assert_eq!(reply.entries[0].count, 100);
        }
        // Search also loses disclosure authority if reconnect wins while its SELECT waits.
        for displaced in [true, false] {
            let mut blocker = pool.begin().await?;
            sqlx::query("LOCK TABLE game_item_character_depot_locations IN ACCESS EXCLUSIVE MODE")
                .execute(&mut *blocker)
                .await?;
            let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
                .fetch_one(&mut *blocker)
                .await?;
            let payload = oteryn_protocol_oteryn::depot_search::encode_request(
                oteryn_protocol_oteryn::depot_search::Request::Search { client_id: 3031 },
            )
            .map_err(debug)?;
            let frame = oteryn_protocol_oteryn::encode_client_command(
                1,
                &oteryn_protocol_oteryn::ClientCommandValue {
                    command_id: 6 - u64::from(displaced),
                    command_type: oteryn_protocol_oteryn::depot_search::COMMAND_TYPE_DEPOT_SEARCH,
                    payload: &payload,
                },
            )?;
            client.write_u32(u32::try_from(frame.len())?).await?;
            client.write_all(&frame).await?;
            tokio::time::timeout(std::time::Duration::from_secs(5), async {
                loop {
                    let blocked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid)))")
                        .bind(pid).fetch_one(pool).await?;
                    if blocked {return Ok::<(),sqlx::Error>(());}
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                }
            }).await??;
            if displaced {
                sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation=current_generation+1 WHERE game_session_id=encode($1,'hex')::uuid")
                    .bind(session.as_bytes().as_slice()).execute(pool).await?;
            }
            blocker.commit().await?;
            loop {
                let frame = read_frame(&mut client).await?;
                let envelope = oteryn_protocol_oteryn::decode_wire_envelope(&frame)?;
                if envelope.message_type() == oteryn_protocol_oteryn::MessageType::CommandResult {
                    let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                    assert_eq!(result.command_id, 6 - u64::from(displaced));
                    assert_eq!(
                        result.status,
                        oteryn_protocol_oteryn::CommandStatus::Accepted
                    );
                    let reply = oteryn_protocol_oteryn::depot_search::decode_reply(result.payload)
                        .map_err(debug)?;
                    if displaced {
                        assert_eq!(
                            reply.outcome,
                            oteryn_protocol_oteryn::depot_search::Outcome::Rejected
                        );
                        assert!(reply.results.is_empty());
                    } else {
                        assert_eq!(
                            reply.outcome,
                            oteryn_protocol_oteryn::depot_search::Outcome::Found
                        );
                        assert_eq!(reply.results[0].depot, 100);
                    }
                    break;
                }
            }
            if displaced {
                sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation=current_generation-1 WHERE game_session_id=encode($1,'hex')::uuid")
                    .bind(session.as_bytes().as_slice()).execute(pool).await?;
            }
        }
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_item_loop_move_receipts WHERE depot_town=0 AND depot_box=17",
        )
        .fetch_one(pool)
        .await?;
        assert_eq!(receipts, 1, "terminal replay must not repeat the deposit");
        client.shutdown().await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let mut serving = std::pin::pin!(serving);
    let mut exchange = std::pin::pin!(exchange);
    let mut end = None;
    let mut result = None;
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        std::future::poll_fn(|cx| {
            use std::future::Future;
            use std::task::Poll;
            if end.is_none()
                && let Poll::Ready(value) = serving.as_mut().poll(cx)
            {
                end = Some(value);
            }
            if result.is_none()
                && let Poll::Ready(value) = exchange.as_mut().poll(cx)
            {
                result = Some(value);
            }
            if end.is_some() && result.is_some() {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }),
    )
    .await?;
    result.ok_or("depot client did not finish")?
}

async fn send(
    client: &mut tokio::io::DuplexStream,
    command_id: u64,
    intent: DepotIntent,
) -> TestResult {
    let payload = encode_depot_intent(intent).map_err(debug)?;
    let frame = oteryn_protocol_oteryn::encode_client_command(
        1,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id,
            command_type: COMMAND_TYPE_DEPOT_INTENT,
            payload: &payload,
        },
    )?;
    client.write_u32(u32::try_from(frame.len())?).await?;
    client.write_all(&frame).await?;
    Ok(())
}

async fn read_frame(client: &mut tokio::io::DuplexStream) -> TestResult<Vec<u8>> {
    let length = usize::try_from(client.read_u32().await?)?;
    assert!(length <= 1 << 20);
    let mut frame = vec![0; length];
    client.read_exact(&mut frame).await?;
    Ok(frame)
}

async fn read_reply(
    client: &mut tokio::io::DuplexStream,
    command_id: u64,
) -> TestResult<DepotReply> {
    loop {
        let frame = read_frame(client).await?;
        let envelope = oteryn_protocol_oteryn::decode_wire_envelope(&frame)?;
        if envelope.message_type() == oteryn_protocol_oteryn::MessageType::CommandResult {
            let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
            assert_eq!(result.command_id, command_id);
            assert_eq!(
                result.status,
                oteryn_protocol_oteryn::CommandStatus::Accepted
            );
            return decode_depot_reply(result.payload).map_err(debug);
        }
    }
}
