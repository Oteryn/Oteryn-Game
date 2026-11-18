use tokio::io::AsyncWriteExt;
use super::fixture::{self, Harness, debug};

use super::TestResult;

use crate::foundation::{
    CommandStatus, MessageType,
    RuntimeScopeRefV1, decode_wire_envelope,
};

use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, IDLE_LIVENESS, serve_admitted,
};

use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};

use crate::gameplay_transport::ComposedFreshAdmission;

use oteryn_protocol_oteryn::activity_scope::{
    ActivityScopeSnapshot, STATE_DOMAIN_ACTIVITY_SCOPE, decode_activity_scope,
};

use oteryn_protocol_oteryn::boss_encounter::{
    self as wire, BossActivityReply,
};

use oteryn_protocol_oteryn::{
    ClientCommandValue, decode_command_result, decode_snapshot_chunk,
    encode_client_command_for_session,
};

pub(super) async fn issue_content_and_reset(
    h: &Harness,
    pin: &oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin,
) -> TestResult {
    issue_content_and_reset_with_runtime_pin(h, pin, pin, "activity-fixture").await
}

pub(super) async fn issue_content_and_reset_with_runtime_pin(
    h: &Harness,
    pin: &oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin,
    runtime_pin: &oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin,
    runtime_map_revision: &str,
) -> TestResult {
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fixture::fence(1)?.runtime_scope
    else {
        return Err("fixture Channel".into());
    };
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
        .bind(pin.server_artifact_digest().as_slice()).bind(pin.client_artifact_digest().as_slice())
        .bind(pin.frame_binding_digest().as_slice()).execute(&h.pool).await?;
    // Initial reset fixture follows the existing kill-reward PG harness. Runtime
    // entry never writes a control grant, activation or world-reset row.
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,0,$2,$5,$2,$3,$4,'TIBIA_SERVER_SAVE_BERLIN_06',1,clock_timestamp(),'2026-10')")
        .bind(world_id.as_bytes().as_slice()).bind(runtime_pin.server_artifact_digest().as_slice())
        .bind(runtime_pin.client_artifact_digest().as_slice()).bind(runtime_pin.frame_binding_digest().as_slice()).bind(runtime_map_revision).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
        .bind(world_id.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    Ok(())
}

pub(super) async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    boss: &str,
    expect_switch: bool,
) -> TestResult<(u64, BossActivityReply, Option<ActivityScopeSnapshot>)> {
    exchange_command(
        owner,
        admitted,
        boss,
        expect_switch,
        wire::COMMAND_TYPE_BOSS_LEVER,
    )
    .await
}

pub(super) async fn exchange_capture(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    boss: &str,
    expect_switch: bool,
    capture: &mut Vec<Vec<u8>>,
) -> TestResult<(u64, BossActivityReply, Option<ActivityScopeSnapshot>)> {
    exchange_command_capture(
        owner,
        admitted,
        boss,
        expect_switch,
        wire::COMMAND_TYPE_BOSS_LEVER,
        capture,
    )
    .await
}

pub(super) async fn exchange_command(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    boss: &str,
    expect_switch: bool,
    command_type: u32,
) -> TestResult<(u64, BossActivityReply, Option<ActivityScopeSnapshot>)> {
    exchange_command_capture(
        owner,
        admitted,
        boss,
        expect_switch,
        command_type,
        &mut Vec::new(),
    )
    .await
}

pub(super) async fn exchange_command_capture(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    boss: &str,
    expect_switch: bool,
    command_type: u32,
    capture: &mut Vec<Vec<u8>>,
) -> TestResult<(u64, BossActivityReply, Option<ActivityScopeSnapshot>)> {
    let before = *admitted;
    let id = before.continuity.next_command_id;
    let body = wire::encode_boss_key(boss).map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(16384);
    let serving = async {
        let end = serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let mut stage = "source initial snapshot";
    let mut frames = std::collections::VecDeque::with_capacity(16);
    let driving = async {
        loop {
            let frame = read_frame(&mut client).await?;
            capture.push(frame.clone());
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if frames.len() == 16 {
                frames.pop_front();
            }
            frames.push_back(format!(
                "{stage}: {:?}, sequence {}",
                envelope.message_type(),
                envelope.server_sequence()
            ));
            if envelope.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        stage = "write boss command";
        write_frame(
            &mut client,
            &encode_client_command_for_session(
                before.continuity.connection_generation,
                &ClientCommandValue {
                    command_id: id,
                    command_type,
                    payload: &body,
                },
                before.game_session_id.as_bytes(),
            )
            .map_err(debug)?,
        )
        .await?;
        stage = "command result";
        let mut reply = None;
        let mut barrier = None;
        let mut commits_after_barrier = 0;
        loop {
            let frame = read_frame(&mut client).await?;
            capture.push(frame.clone());
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if frames.len() == 16 {
                frames.pop_front();
            }
            frames.push_back(format!(
                "{stage}: {:?}, sequence {}",
                envelope.message_type(),
                envelope.server_sequence()
            ));
            match envelope.message_type() {
                MessageType::CommandResult => {
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(result.command_id, id);
                    let decoded = wire::decode_boss_activity_reply(result.payload);
                    assert_eq!(
                        result.status,
                        if expect_switch {
                            CommandStatus::Accepted
                        } else {
                            CommandStatus::Rejected
                        },
                        "boss command {id}, session {:?}, generation {}: {decoded:?}; raw payload {:?}",
                        before.game_session_id,
                        before.continuity.connection_generation,
                        result.payload
                    );
                    reply = Some(decoded.map_err(debug)?);
                    if !expect_switch {
                        break;
                    }
                    stage = "source scope barrier";
                }
                MessageType::SnapshotChunk => {
                    let (_, domains) = decode_snapshot_chunk(envelope.payload()).map_err(debug)?;
                    for domain in domains {
                        if domain.domain_id == STATE_DOMAIN_ACTIVITY_SCOPE {
                            assert!(reply.is_some(), "initiator result precedes scope barrier");
                            let projected = decode_activity_scope(domain.payload).map_err(debug)?;
                            if let Some(prior) = barrier.as_ref() {
                                // The new owner's full initial snapshot includes
                                // its binding too. It is the same projection, not
                                // another source-stream switch or reset.
                                assert!(commits_after_barrier >= 1, "one source-stream barrier");
                                assert_eq!(
                                    prior, &projected,
                                    "initial scope projection agrees with committed barrier"
                                );
                            } else {
                                barrier = Some(projected);
                                stage = "source scope barrier commit";
                            }
                        }
                    }
                }
                MessageType::SnapshotCommit if barrier.is_some() => {
                    commits_after_barrier += 1;
                    if commits_after_barrier == 2 {
                        break;
                    }
                    stage = "destination initial snapshot";
                }
                _ => {}
            }
        }
        stage = "client shutdown";
        client.shutdown().await?;
        stage = "complete";
        Ok((id, reply.ok_or("command result")?, barrier))
    };
    let (ended, result): (_, TestResult<_>) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        fixture::join_two(serving, driving),
    )
    .await?;
    if let ConnectionEnd::AdmittedThenDisconnected(next)
    | ConnectionEnd::AdmittedThenClosed(next, _) = ended
    {
        *admitted = next;
    }
    result.map_err(|error| {
        format!(
            "boss wire command {id}, expect_switch={expect_switch}, stage={stage}, source={before:?}, ended={ended:?}, frames={frames:?}: {error}"
        )
        .into()
    })
}

pub(super) async fn capture_current_snapshot(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    capture: &mut Vec<Vec<u8>>,
) -> TestResult {
    let before = *admitted;
    let (mut server, mut client) = tokio::io::duplex(16384);
    let serving = async {
        let ended = serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await;
        drop(server);
        ended
    };
    let driving = async {
        loop {
            let frame = read_frame(&mut client).await?;
            let complete = decode_wire_envelope(&frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit;
            capture.push(frame);
            if complete {
                break;
            }
        }
        client.shutdown().await?;
        Ok(())
    };
    let (ended, result): (_, TestResult) = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        fixture::join_two(serving, driving),
    )
    .await?;
    match ended {
        ConnectionEnd::AdmittedThenDisconnected(next)
        | ConnectionEnd::AdmittedThenClosed(next, _) => *admitted = next,
        _ => return Err("actual source observation stream did not retain admission".into()),
    }
    result
}
