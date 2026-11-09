//! One live transport: wrap, unwrap, select the owned House, notice, then cancel.
//! Disconnecting between actions is deliberately avoided: it loses actor control.
use super::*;
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use oteryn_protocol_oteryn::house_sale as sale;

pub(super) async fn roundtrip(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    admitted: &mut AdmittedSession,
) -> TestResult {
    admitted.continuity.next_command_id = 1;
    let before = *admitted;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let driving = async {
        loop {
            let frame = read_frame(&mut client).await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        let timed = timed(h).await?;
        for (number, action, revision) in [(1, wire::Action::Wrap, 1), (2, wire::Action::Unwrap, 2)]
        {
            let body = wire::Request {
                action,
                item: id(80),
                expected_revision: revision,
                acl_revision: 1,
                position: POSITION,
            }
            .encode();
            let (status, payload) =
                exchange(&mut client, before, number, wire::COMMAND_WRAP, &body).await?;
            assert_eq!(
                (status, wire::Reply::decode(&payload).ok_or("wrap reply")?),
                (
                    CommandStatus::Accepted,
                    wire::Reply::Committed(revision + 1)
                ),
                "partly used cask retains its contents through the real wrapping caller"
            );
            assert_eq!(
                self::timed(h).await?,
                timed,
                "wrapping cannot refill or discard charges"
            );
        }
        assert_eq!(sqlx::query_scalar::<_,String>("SELECT definition_production_key FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(80).as_slice()).fetch_one(&h.pool).await?,"oteryn:item.tibia.i25880");
        let session:serde_json::Value=sqlx::query_scalar("SELECT jsonb_build_object('state',session_state,'transport',current_transport_ref IS NOT NULL,'epoch',control_loss_epoch,'scope_generation',scope_ownership_generation,'connection',current_generation,'lease',character_lease_generation) FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid").bind(before.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        eprintln!("T135 live House session {session}");
        let mut request = sale::HouseSaleRequest::catalogue(String::new());
        for (number, action) in [(3, 0), (4, 1), (5, 2)] {
            request.action = action;
            let body = sale::encode_request(&request).map_err(debug)?;
            let (status, payload) =
                exchange(&mut client, before, number, sale::COMMAND, &body).await?;
            assert_eq!(
                status,
                CommandStatus::Accepted,
                "named-House ownership action {action} uses its live assignment and session fence"
            );
            let reply = sale::decode_reply(&payload).map_err(debug)?;
            let row = reply
                .rows
                .iter()
                .find(|row| row.house == HOUSE)
                .ok_or("owned House selection")?;
            assert_eq!(row.move_out_state, if action == 1 { 1 } else { 0 });
            request.house = row.house.clone();
            request.expected_revision = row.revision;
            request.due_at_unix_ms = if action == 0 {
                reply.now_unix_ms + 2 * 86_400_000
            } else {
                0
            };
        }
        client.shutdown().await?;
        Ok(())
    };
    let (end, result): (_, TestResult) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        super::super::fixture::join_two(serving, driving),
    )
    .await?;
    if let ConnectionEnd::AdmittedThenDisconnected(current)
    | ConnectionEnd::AdmittedThenClosed(current, _) = end
    {
        *admitted = current;
    }
    result
}
async fn timed(h: &Harness) -> TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar("SELECT to_jsonb(t) FROM game_item_timed_states t WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(80).as_slice()).fetch_one(&h.pool).await?)
}
async fn exchange(
    client: &mut tokio::io::DuplexStream,
    before: AdmittedSession,
    number: u64,
    kind: u32,
    body: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let frame = encode_client_command(
        before.continuity.connection_generation,
        &ClientCommandValue {
            command_id: number,
            command_type: kind,
            payload: body,
        },
    )
    .map_err(debug)?;
    write_frame(client, &frame).await?;
    loop {
        let frame = read_frame(client).await?;
        let envelope = decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(result.command_id, number);
            return Ok((result.status, result.payload.to_vec()));
        }
    }
}
