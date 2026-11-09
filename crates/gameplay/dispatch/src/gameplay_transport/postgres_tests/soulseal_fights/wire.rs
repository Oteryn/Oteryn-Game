//! A real source stream receives the result, scope barrier and destination snapshot,
//! then acknowledges the new Soulpit controller before any gameplay can proceed.
use super::*;
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::{
    connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted},
    tcp_tls::{read_frame, write_frame},
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, activity_scope::*, decode_command_result, decode_snapshot_chunk,
    encode_client_command,
};
use tokio::io::AsyncWriteExt;
pub(super) async fn enter(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    body: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let before = *admitted;
    let (mut server, mut client) = tokio::io::duplex(128 * 1024);
    let serving = async {
        let ended = serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await;
        drop(server);
        ended
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
        write_frame(
            &mut client,
            &encode_client_command(
                before.continuity.connection_generation,
                &ClientCommandValue {
                    command_id: 2,
                    command_type: wire::COMMAND_TYPE,
                    payload: body,
                },
            )
            .map_err(debug)?,
        )
        .await?;
        let mut result = None;
        let mut barrier: Option<ActivityScopeSnapshot> = None;
        let mut commits = 0;
        loop {
            let frame = read_frame(&mut client).await?;
            let env = decode_wire_envelope(&frame).map_err(debug)?;
            match env.message_type() {
                MessageType::CommandResult => {
                    let reply = decode_command_result(env.payload()).map_err(debug)?;
                    assert_eq!(reply.command_id, 2);
                    assert_eq!(reply.status, CommandStatus::Accepted);
                    assert_eq!(
                        wire::Reply::decode(reply.payload).map_err(debug)?.status,
                        wire::Status::Started
                    );
                    result = Some((reply.status, reply.payload.to_vec()));
                }
                MessageType::SnapshotChunk => {
                    let (_, domains) = decode_snapshot_chunk(env.payload()).map_err(debug)?;
                    for domain in domains {
                        if domain.domain_id == STATE_DOMAIN_ACTIVITY_SCOPE {
                            assert!(result.is_some(), "result precedes barrier");
                            let scope = decode_activity_scope(domain.payload).map_err(debug)?;
                            if let Some(prior) = &barrier {
                                assert_eq!(
                                    commits, 1,
                                    "only the ordinary destination snapshot repeats the binding"
                                );
                                let mut expected: ActivityScopeSnapshot = prior.clone();
                                expected.requires_ack = false;
                                assert_eq!(scope, expected);
                            } else {
                                assert_eq!(
                                    scope.source_session,
                                    *before.game_session_id.as_bytes()
                                );
                                assert!(matches!(scope.scope, ActivityScope::Instance { .. }));
                                assert!(scope.requires_ack);
                                barrier = Some(scope);
                            }
                        }
                    }
                }
                MessageType::SnapshotCommit if barrier.is_some() => {
                    commits += 1;
                    if commits == 2 {
                        break;
                    }
                }
                _ => {}
            }
        }
        let scope = barrier.ok_or("scope barrier")?;
        let ack = encode_activity_scope_ack(&ActivityScopeAck {
            handoff: scope.handoff,
            session: scope.session,
        })
        .map_err(debug)?;
        write_frame(
            &mut client,
            &encode_client_command(
                scope.connection_generation,
                &ClientCommandValue {
                    command_id: 1,
                    command_type: COMMAND_TYPE_ACTIVITY_SCOPE_ACK,
                    payload: &ack,
                },
            )
            .map_err(debug)?,
        )
        .await?;
        loop {
            let frame = read_frame(&mut client).await?;
            let env = decode_wire_envelope(&frame).map_err(debug)?;
            if env.message_type() == MessageType::CommandResult {
                let reply = decode_command_result(env.payload()).map_err(debug)?;
                assert_eq!(reply.command_id, 1);
                assert_eq!(reply.status, CommandStatus::Accepted);
                break;
            }
        }
        client.shutdown().await?;
        Ok((result.ok_or("entry result")?, scope.session))
    };
    let (ended, result): (_, TestResult<_>) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        super::super::fixture::join_two(serving, driving),
    )
    .await?;
    match ended {
        ConnectionEnd::AdmittedThenDisconnected(next)
        | ConnectionEnd::AdmittedThenClosed(next, _) => *admitted = next,
        _ => return Err("entry stream did not retain destination".into()),
    }
    let (result, session) = result?;
    assert_eq!(admitted.game_session_id.as_bytes(), &session);
    assert_ne!(admitted.game_session_id, before.game_session_id);
    Ok(result)
}
