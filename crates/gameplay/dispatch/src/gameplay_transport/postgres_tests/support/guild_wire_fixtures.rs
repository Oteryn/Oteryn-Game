use tokio::io::AsyncWriteExt;
use super::fixture::{self, debug};

use super::TestResult;

use crate::foundation::{MessageType, decode_wire_envelope};

use crate::gameplay_transport::connection::{
    AdmittedSession, IDLE_LIVENESS, serve_admitted,
};

use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};

use crate::gameplay_transport::ComposedFreshAdmission;


use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};

pub(super) async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    kind: u32,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let before = *admitted;
    let (mut server, mut client) = tokio::io::duplex(1024);
    let serving = async move {
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
                    command_id: id,
                    command_type: kind,
                    payload,
                },
            )
            .map_err(debug)?,
        )
        .await?;
        let reply = loop {
            let frame = read_frame(&mut client).await?;
            let env = decode_wire_envelope(&frame).map_err(debug)?;
            if env.message_type() == MessageType::CommandResult {
                let r = decode_command_result(env.payload()).map_err(debug)?;
                assert_eq!(r.command_id, id);
                break (r.status, r.payload.to_vec());
            }
        };
        client.shutdown().await?;
        Ok(reply)
    };
    let (ended, result): (_, TestResult<(CommandStatus, Vec<u8>)>) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        fixture::join_two(serving, driving),
    )
    .await?;
    match &ended {
        crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(current)
        | crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenClosed(current, _) => {
            *admitted = *current
        }
        _ => {}
    }
    let reply =
        result.map_err(|error| format!("guild command {id}/{kind} ended {ended:?}: {error}"))?;
    assert_eq!(
        admitted.continuity.next_command_id,
        before.continuity.next_command_id + u64::from(id == before.continuity.next_command_id)
    );
    assert!(admitted.continuity.server_sequence > before.continuity.server_sequence);
    Ok(reply)
}

/// One real transport epoch, retaining every server frame across an optional
/// activity barrier. No sequence, session binding or snapshot is reconstructed.
pub(super) async fn exchange_capture(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    kind: u32,
    build_payload: impl FnOnce(&[Vec<u8>]) -> TestResult<Vec<u8>>,
    expect_switch: bool,
    capture: &mut Vec<Vec<u8>>,
) -> TestResult<(
    CommandStatus,
    Vec<u8>,
    Option<oteryn_protocol_oteryn::activity_scope::ActivityScopeSnapshot>,
    Vec<u8>,
)> {
    exchange_capture_goal(
        owner,
        admitted,
        id,
        kind,
        build_payload,
        if expect_switch {
            CaptureGoal::Switch
        } else {
            CaptureGoal::Reply
        },
        capture,
    )
    .await
}

#[derive(Clone, Copy)]
enum CaptureGoal {
    Reply,
    Switch,
    Effect(u16),
    SwitchEffect(u16),
}

pub(super) async fn exchange_capture_effect(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    kind: u32,
    build_payload: impl FnOnce(&[Vec<u8>]) -> TestResult<Vec<u8>>,
    effect: u16,
    capture: &mut Vec<Vec<u8>>,
) -> TestResult<(
    CommandStatus,
    Vec<u8>,
    Option<oteryn_protocol_oteryn::activity_scope::ActivityScopeSnapshot>,
    Vec<u8>,
)> {
    assert!(
        admitted
            .continuity
            .selected_capabilities
            .contains(oteryn_protocol_oteryn::presentation::CAPABILITY_PRESENTATION_V1),
        "actual native presentation23 selected before starting effect capture"
    );
    exchange_capture_goal(
        owner,
        admitted,
        id,
        kind,
        build_payload,
        CaptureGoal::Effect(effect),
        capture,
    )
    .await
}

/// Retain the actual post-barrier idle cue on the same transport epoch.
#[allow(clippy::too_many_arguments)]
pub(super) async fn exchange_capture_switch_effect(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    kind: u32,
    build_payload: impl FnOnce(&[Vec<u8>]) -> TestResult<Vec<u8>>,
    effect: u16,
    capture: &mut Vec<Vec<u8>>,
) -> TestResult<(
    CommandStatus,
    Vec<u8>,
    Option<oteryn_protocol_oteryn::activity_scope::ActivityScopeSnapshot>,
    Vec<u8>,
)> {
    assert!(
        admitted
            .continuity
            .selected_capabilities
            .contains(oteryn_protocol_oteryn::presentation::CAPABILITY_PRESENTATION_V1),
        "actual presentation23 selected before switching cue capture"
    );
    exchange_capture_goal(
        owner,
        admitted,
        id,
        kind,
        build_payload,
        CaptureGoal::SwitchEffect(effect),
        capture,
    )
    .await
}

async fn exchange_capture_goal(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    kind: u32,
    build_payload: impl FnOnce(&[Vec<u8>]) -> TestResult<Vec<u8>>,
    goal: CaptureGoal,
    capture: &mut Vec<Vec<u8>>,
) -> TestResult<(
    CommandStatus,
    Vec<u8>,
    Option<oteryn_protocol_oteryn::activity_scope::ActivityScopeSnapshot>,
    Vec<u8>,
)> {
    let expect_switch = matches!(goal, CaptureGoal::Switch | CaptureGoal::SwitchEffect(_));
    use oteryn_protocol_oteryn::activity_scope::{
        STATE_DOMAIN_ACTIVITY_SCOPE, decode_activity_scope,
    };
    let before = *admitted;
    let (mut server, mut client) = tokio::io::duplex(16384);
    let serving = async {
        let ended = serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await;
        drop(server);
        ended
    };
    let driving = async {
        let mut native_effect_basis = false;
        loop {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            let complete = envelope.message_type() == MessageType::SnapshotCommit;
            if envelope.message_type() == MessageType::SnapshotChunk {
                let (_, domains) =
                    oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())
                        .map_err(debug)?;
                for domain in domains {
                    if domain.domain_id
                        == oteryn_protocol_oteryn::presentation::STATE_DOMAIN_WORLD_PRESENTATION
                    {
                        native_effect_basis = domain.snapshot_type == oteryn_protocol_oteryn::native_map_cues::SNAPSHOT_TYPE_NATIVE_MAP_CUE_BASIS_V1;
                        if native_effect_basis {
                            assert!(before.continuity.selected_capabilities.contains(385));
                        }
                    }
                }
            }
            capture.push(frame);
            if complete {
                break;
            }
        }
        let command_payload = build_payload(capture)?;
        write_frame(
            &mut client,
            &oteryn_protocol_oteryn::encode_client_command_for_session(
                before.continuity.connection_generation,
                &ClientCommandValue {
                    command_id: id,
                    command_type: kind,
                    payload: &command_payload,
                },
                before.game_session_id.as_bytes(),
            )
            .map_err(debug)?,
        )
        .await?;
        let mut reply = None;
        let mut barrier: Option<oteryn_protocol_oteryn::activity_scope::ActivityScopeSnapshot> =
            None;
        let mut barrier_commits = 0;
        let mut effect_seen = false;
        let mut acknowledged = false;
        loop {
            let frame = read_frame(&mut client).await?;
            let env = decode_wire_envelope(&frame).map_err(debug)?;
            match env.message_type() {
                MessageType::CommandResult => {
                    let result = decode_command_result(env.payload()).map_err(debug)?;
                    assert_eq!(result.command_id, id);
                    if expect_switch && result.status != CommandStatus::Accepted {
                        return Err(format!(
                            "actual handoff command {id}/{kind} returned {:?}: {:?}",
                            result.status, result.payload
                        )
                        .into());
                    }
                    if matches!(goal, CaptureGoal::Effect(_) | CaptureGoal::SwitchEffect(_)) {
                        assert_eq!(result.status, CommandStatus::Accepted);
                        if kind == oteryn_protocol_oteryn::world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT {
                            assert_eq!(
                                oteryn_protocol_oteryn::world_spatial::decode_step_result(result.payload).map_err(debug)?,
                                oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved,
                                "the actual native step commits before awaiting its source exit effect",
                            );
                        } else {
                            let disposition = oteryn_protocol_oteryn::world_object::decode_use_result(result.payload).map_err(debug)?;
                            assert_eq!(disposition,oteryn_protocol_oteryn::world_object::UseDisposition::Committed,
                                "the real effect producer must commit before awaiting native egress");
                        }
                    }
                    reply = Some((result.status, result.payload.to_vec()));
                }
                MessageType::SnapshotChunk => {
                    let (_, domains) = oteryn_protocol_oteryn::decode_snapshot_chunk(env.payload())
                        .map_err(debug)?;
                    for domain in domains {
                        if domain.domain_id
                            == oteryn_protocol_oteryn::presentation::STATE_DOMAIN_WORLD_PRESENTATION
                        {
                            native_effect_basis = domain.snapshot_type == oteryn_protocol_oteryn::native_map_cues::SNAPSHOT_TYPE_NATIVE_MAP_CUE_BASIS_V1;
                            if native_effect_basis {
                                assert!(before.continuity.selected_capabilities.contains(385));
                            }
                        }
                        if domain.domain_id == STATE_DOMAIN_ACTIVITY_SCOPE {
                            assert!(reply.is_some(), "initiator result precedes scope barrier");
                            let scope = decode_activity_scope(domain.payload).map_err(debug)?;
                            if let Some(previous) = barrier.as_ref() {
                                assert!(barrier_commits >= 1, "one source-stream switch barrier");
                                // The prior source barrier requires ACK; the cold projection
                                // never requests a second transfer. This is the exact native
                                // Session::activity_handoff projected-receipt comparison.
                                let mut projected = previous.clone();
                                projected.requires_ack = false;
                                assert_eq!(projected, scope, "new owner's initial scope agrees");
                            } else {
                                barrier = Some(scope);
                            }
                        }
                    }
                }
                MessageType::StateDelta => {
                    if let CaptureGoal::Effect(wanted) | CaptureGoal::SwitchEffect(wanted) = goal {
                        let delta = oteryn_protocol_oteryn::decode_state_delta(env.payload())
                            .map_err(debug)?;
                        if delta.domain_id
                            == oteryn_protocol_oteryn::presentation::STATE_DOMAIN_WORLD_PRESENTATION
                        {
                            let events = if native_effect_basis {
                                assert_eq!(delta.delta_type,oteryn_protocol_oteryn::native_map_cues::DELTA_TYPE_NATIVE_MAP_PRESENTATION_V1);
                                oteryn_protocol_oteryn::native_map_cues::decode_native_map_presentation_batch(delta.payload)
                            } else {
                                assert_eq!(delta.delta_type,1);
                                oteryn_protocol_oteryn::presentation::decode_presentation_batch(delta.payload)
                            }.map_err(debug)?;
                            effect_seen |= events.iter().any(|event| matches!(&event.kind,
                                    oteryn_protocol_oteryn::presentation::PresentationEventKind::MagicEffect { effect_id, .. }
                                        if *effect_id == u32::from(wanted)));
                        }
                    }
                }
                MessageType::SnapshotCommit if barrier.is_some() => {
                    barrier_commits += 1;
                }
                _ => {}
            }
            capture.push(frame);
            if !acknowledged
                && barrier_commits == 2
                && let Some(receipt) = barrier.as_ref().filter(|receipt| receipt.requires_ack)
            {
                acknowledge_captured_activity(&mut client, receipt, capture).await?;
                acknowledged = true;
            }
            if reply.is_some()
                && match goal {
                    CaptureGoal::Reply => true,
                    CaptureGoal::Switch => barrier_commits == 2,
                    CaptureGoal::Effect(_) => effect_seen,
                    CaptureGoal::SwitchEffect(_) => barrier_commits == 2 && effect_seen,
                }
            {
                break;
            }
        }
        client.shutdown().await?;
        let (status, payload) = reply.ok_or("real command result")?;
        Ok((status, payload, barrier, command_payload))
    };
    let (ended, result): (_, TestResult<_>) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        fixture::join_two(serving, driving),
    )
    .await?;
    match ended {
        crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(current)
        | crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenClosed(current, _) => {
            *admitted = current;
        }
        _ => {
            return Err(format!(
                "captured command {id}/{kind} ended {}",
                capture_end_kind(&ended)
            )
            .into());
        }
    }
    let result = result.map_err(|error| {
        format!(
            "captured command {id}/{kind}, ended {}: {error}",
            capture_end_kind(&ended)
        )
    })?;
    if expect_switch {
        let scope = result.2.as_ref().ok_or("actual activity scope barrier")?;
        assert_eq!(scope.source_session, *before.game_session_id.as_bytes());
        assert_eq!(scope.session, *admitted.game_session_id.as_bytes());
        assert_ne!(before.game_session_id, admitted.game_session_id);
        if scope.requires_ack {
            assert_eq!(admitted.continuity.next_command_id, 2);
        }
    }
    Ok(result)
}

/// Emit the actual ACK only after the complete destination snapshot, then
/// retain the untouched server result in the server-frame transcript. Client
/// command bytes are sent verbatim, never inserted into that server-only stream.
async fn acknowledge_captured_activity<S>(
    client: &mut S,
    receipt: &oteryn_protocol_oteryn::activity_scope::ActivityScopeSnapshot,
    capture: &mut Vec<Vec<u8>>,
) -> TestResult
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    use oteryn_protocol_oteryn::activity_scope::{
        ActivityScopeAck, COMMAND_TYPE_ACTIVITY_SCOPE_ACK, encode_activity_scope_ack,
    };
    let payload = encode_activity_scope_ack(&ActivityScopeAck {
        handoff: receipt.handoff,
        session: receipt.session,
    })
    .map_err(debug)?;
    let ack = oteryn_protocol_oteryn::encode_client_command_for_session(
        receipt.connection_generation,
        &ClientCommandValue {
            command_id: 1,
            command_type: COMMAND_TYPE_ACTIVITY_SCOPE_ACK,
            payload: &payload,
        },
        &receipt.session,
    )
    .map_err(debug)?;
    let command_envelope = decode_wire_envelope(&ack).map_err(debug)?;
    assert_eq!(
        command_envelope
            .client_command_session_id()
            .map_err(debug)?,
        Some(receipt.session.as_slice())
    );
    let command = command_envelope
        .client_command(receipt.connection_generation)
        .map_err(debug)?;
    assert_eq!(command.command_id, 1);
    assert_eq!(command.command_type, COMMAND_TYPE_ACTIVITY_SCOPE_ACK);
    assert_eq!(command.payload, payload);
    eprintln!("SRX3_ACTIVITY_ACK actual_client_frame={ack:02x?}");
    write_frame(client, &ack).await?;
    let frame = read_frame(client).await?;
    let envelope = decode_wire_envelope(&frame).map_err(debug)?;
    envelope
        .validate(crate::foundation::Direction::ServerToClient, true)
        .map_err(debug)?;
    assert_eq!(envelope.message_type(), MessageType::CommandResult);
    assert_eq!(
        envelope.connection_generation(),
        receipt.connection_generation
    );
    assert_eq!(envelope.server_sequence(), 1);
    let result = decode_command_result(envelope.payload()).map_err(debug)?;
    assert_eq!(result.command_id, 1);
    assert_eq!(result.status, CommandStatus::Accepted);
    assert!(result.payload.is_empty());
    capture.push(frame);
    Ok(())
}

fn capture_end_kind(end: &crate::gameplay_transport::connection::ConnectionEnd) -> String {
    match end {
        crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenControlLost(_) => {
            "AdmittedThenControlLost".into()
        }
        crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(_) => {
            "AdmittedThenDisconnected".into()
        }
        crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenClosed(_, _) => {
            "AdmittedThenClosed".into()
        }
        _ => format!("{:?}", std::mem::discriminant(end)),
    }
}

/// Observe a real autonomous owner action inside one continuously served
/// Instance transport, through its barrier and the Channel's initial snapshot.
pub(super) async fn capture_activity_return(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    capture: &mut Vec<Vec<u8>>,
    action: impl std::future::Future<Output = TestResult>,
) -> TestResult<oteryn_protocol_oteryn::activity_scope::ActivityScopeSnapshot> {
    use oteryn_protocol_oteryn::activity_scope::{
        STATE_DOMAIN_ACTIVITY_SCOPE, decode_activity_scope,
    };
    let before = *admitted;
    let (mut server, mut client) = tokio::io::duplex(16384);
    let serving = async {
        let ended = serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await;
        drop(server);
        ended
    };
    // The action may drive the owner for seconds; the client keeps answering liveness probes
    // meanwhile (as a real client does), or IDLE_LIVENESS ends the session as control lost.
    let generation = before.continuity.connection_generation;
    let (snapshot_tx, snapshot_rx) = tokio::sync::oneshot::channel::<()>();
    let acting = async {
        snapshot_rx
            .await
            .map_err(|_| "initial snapshot before the return action")?;
        action.await
    };
    let reading = async {
        let mut snapshot_tx = Some(snapshot_tx);
        let mut barrier: Option<oteryn_protocol_oteryn::activity_scope::ActivityScopeSnapshot> =
            None;
        let mut commits = 0;
        loop {
            let frame = read_frame(&mut client).await?;
            let env = decode_wire_envelope(&frame).map_err(debug)?;
            if env.message_type() == MessageType::LivenessProbe {
                let probe =
                    oteryn_protocol_oteryn::decode_liveness_probe(env.payload()).map_err(debug)?;
                let ack = oteryn_protocol_oteryn::encode_liveness_ack(generation, probe, 0)
                    .map_err(debug)?;
                write_frame(&mut client, &ack).await?;
                continue;
            }
            if let Some(tx) = snapshot_tx.take() {
                let complete = env.message_type() == MessageType::SnapshotCommit;
                capture.push(frame);
                if complete {
                    tx.send(()).map_err(|_| "return action dropped")?;
                } else {
                    snapshot_tx = Some(tx);
                }
                continue;
            }
            if env.message_type() == MessageType::SnapshotChunk {
                let (_, domains) =
                    oteryn_protocol_oteryn::decode_snapshot_chunk(env.payload()).map_err(debug)?;
                for domain in domains {
                    if domain.domain_id == STATE_DOMAIN_ACTIVITY_SCOPE {
                        let projected = decode_activity_scope(domain.payload).map_err(debug)?;
                        if let Some(previous) = barrier.as_ref() {
                            assert!(commits >= 1, "one autonomous source barrier");
                            let mut expected = previous.clone();
                            expected.requires_ack = false;
                            assert_eq!(
                                expected, projected,
                                "autonomous destination projection agrees"
                            );
                        } else {
                            barrier = Some(projected);
                        }
                    }
                }
            } else if env.message_type() == MessageType::SnapshotCommit && barrier.is_some() {
                commits += 1;
            }
            capture.push(frame);
            if commits == 2 {
                if let Some(receipt) = barrier.as_ref().filter(|receipt| receipt.requires_ack) {
                    acknowledge_captured_activity(&mut client, receipt, capture).await?;
                }
                break;
            }
        }
        client.shutdown().await?;
        barrier.ok_or("autonomous scope barrier".into())
    };
    let driving = async {
        let ((), barrier) = tokio::try_join!(acting, reading)?;
        Ok(barrier)
    };
    let (ended, result): (_, TestResult<_>) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        fixture::join_two(serving, driving),
    )
    .await?;
    match ended {
        crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(current)
        | crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenClosed(current, _) => {
            *admitted = current
        }
        _ => return Err(format!("autonomous capture ended {ended:?}").into()),
    }
    let barrier = result?;
    assert_eq!(barrier.source_session, *before.game_session_id.as_bytes());
    assert_eq!(barrier.session, *admitted.game_session_id.as_bytes());
    if barrier.requires_ack {
        assert_eq!(admitted.continuity.next_command_id, 2);
    }
    Ok(barrier)
}
