//! GUILD-WIRE-1: real admitted connection bytes and native Guild persistence.
#[path = "guild_wire/paging.rs"]
mod paging;

use super::fixture::{self, Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::domain::CharacterId;
use crate::durability::guild_chat_fixture as seed;
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
};
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use crate::gameplay_transport::{ComposedFreshAdmission, item_fence_of};
use oteryn_protocol_oteryn::guild::*;
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use tokio::io::AsyncWriteExt;

#[test]
fn guild_wire_postgres_real_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "guild wire e2e panicked")??;
    Ok(())
}

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
        action.await?;
        let mut barrier: Option<oteryn_protocol_oteryn::activity_scope::ActivityScopeSnapshot> =
            None;
        let mut commits = 0;
        loop {
            let frame = read_frame(&mut client).await?;
            let env = decode_wire_envelope(&frame).map_err(debug)?;
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

async fn intent(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    action: GuildIntent,
    code: u8,
) -> TestResult<GuildIntentResult> {
    let id = admitted.continuity.next_command_id;
    let (status, bytes) = exchange(
        owner,
        admitted,
        id,
        COMMAND_TYPE_GUILD_INTENT,
        &encode_guild_intent(&action).map_err(debug)?,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    let r = decode_guild_intent_result(&bytes).map_err(debug)?;
    assert_eq!(r.code, code);
    Ok(r)
}
async fn query(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    q: GuildQuery,
) -> TestResult<GuildQueryResult> {
    let id = admitted.continuity.next_command_id;
    let (status, bytes) = exchange(
        owner,
        admitted,
        id,
        COMMAND_TYPE_GUILD_QUERY,
        &encode_guild_query(&q).map_err(debug)?,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    decode_guild_query_result(&bytes).map_err(debug)
}
fn scenario() -> TestResult {
    run(|admin| async move {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "guild_wire_dispatch", true, false).await?;
        prepare_current_session(&h).await?;
        seed::prepare(&h.pool).await?;
        sqlx::raw_sql(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../apps/game-server/migrations/0161_guild_roster_keyset.sql"
        )))
        .execute(&h.pool)
        .await?;
        let base = super::fixture::fence(1)?;
        let base = crate::durability::item_transfer::CurrentCharacterItemFence {
            character_id: base.character_id,
            game_session_id: base.game_session_id,
            connection_generation: base.connection_generation,
            character_lease_generation: 1,
            runtime_scope: base.runtime_scope,
            scope_ownership_generation: base.scope_ownership_generation,
        };
        let mut guest_fences = Vec::new();
        for (n, name) in [(90, "Guest One"), (91, "Guest Two")] {
            seed::historical_character(
                &h.pool,
                n,
                CharacterId::from_bytes(seed::sid(n, 1)).map_err(debug)?,
                seed::sid(n, 2),
                id(42),
                name,
            )
            .await?;
            guest_fences.push(seed::historical_session(&h.pool, n, base).await?);
        }
        seed::guard(&h.pool).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        // This scenario has three simultaneous native sessions; the shared fixture defaults to two.
        let mut runtime = crate::foundation::ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            3,
            crate::foundation::ChannelContentPin::test(deps.admitted.world_id),
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        deps.admitted.runtime_actor = Some(deps.actor);
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let coordinator =
            crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
                Default::default(),
                None,
                None,
                Some(h.root.clone()),
            );
        let mut owner = deps.owner(&h, &authority);
        owner.premium_coordinator = Some(&coordinator);
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let (current, _) =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?;
        let mut leader = deps.admitted;
        leader.first_entry = FirstEntryOutcome::Positioned;
        leader.item_fence = item_fence_of(current);
        leader.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[40])
                .ok_or("guild selection")?;
        let initial_leader = leader;
        let mut guests = Vec::new();
        for fence in guest_fences {
            let actor = {
                let mut runtime = owner.runtime.lock().await;
                let reservation = runtime
                    .reserve_fresh_session(fence.game_session_id)
                    .map_err(debug)?;
                let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
                runtime
                    .initialize_first_entry_position(actor)
                    .map_err(debug)?;
                actor
            };
            deps.initialize_player(&owner, actor, fence.game_session_id)
                .await;
            guests.push(AdmittedSession {
                game_session_id: fence.game_session_id,
                runtime_actor: Some(actor),
                item_fence: Some(fence),
                controller: None,
                ..leader
            });
        }
        let mut one = guests[0];
        let mut two = guests[1];
        let character = *one.item_fence.ok_or("guest fence")?.character_id.as_bytes();
        let second = *two.item_fence.ok_or("guest fence")?.character_id.as_bytes();
        // Prove the Guild mutation fence on a fresh command, before reconnect's independent
        // Bank reconciliation prelude can reject the same stale lease first.
        let mut stale = leader;
        stale
            .item_fence
            .as_mut()
            .ok_or("fence")?
            .character_lease_generation += 1;
        let stale_command = stale.continuity.next_command_id;
        assert_eq!(
            exchange(
                &owner,
                &mut stale,
                stale_command,
                COMMAND_TYPE_GUILD_INTENT,
                &encode_guild_intent(&GuildIntent::Found {
                    name: "Wire Guild".into()
                })
                .map_err(debug)?
            )
            .await?
            .0,
            CommandStatus::Rejected
        );
        // Even a rejected command advances this GameSession's ingress. Retain its ended
        // continuity while restoring the current lease for the next fresh command.
        leader.continuity = stale.continuity;
        let found_command = leader.continuity.next_command_id;
        let found = encode_guild_intent(&GuildIntent::Found {
            name: "Wire Guild".into(),
        })
        .map_err(debug)?;
        let (status, receipt) = exchange(
            &owner,
            &mut leader,
            found_command,
            COMMAND_TYPE_GUILD_INTENT,
            &found,
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        let r = decode_guild_intent_result(&receipt).map_err(debug)?;
        assert_eq!(r.code, 1);
        let guild = r.guild;
        assert_eq!(r.revision, Some(1));
        // Reconnect before acknowledgement replaces only the connection: the ended
        // high-water mark and server sequence survive, and the reserved ID replays.
        let (status, replayed) = exchange(
            &owner,
            &mut leader,
            found_command,
            COMMAND_TYPE_GUILD_INTENT,
            &found,
        )
        .await?;
        assert_eq!(status, CommandStatus::DuplicateReplay);
        assert_eq!(replayed, receipt);
        assert_eq!(decode_guild_intent_result(&replayed).map_err(debug)?, r);
        let changed = encode_guild_intent(&GuildIntent::Found {
            name: "Other Guild".into(),
        })
        .map_err(debug)?;
        let (status, replayed) = exchange(
            &owner,
            &mut leader,
            found_command,
            COMMAND_TYPE_GUILD_INTENT,
            &changed,
        )
        .await?;
        // FND-02 §13.4: replacement bytes are not interpreted as a new Guild intent.
        assert_eq!(status, CommandStatus::DuplicateReplay);
        assert_eq!(replayed, receipt);
        assert_eq!(decode_guild_intent_result(&replayed).map_err(debug)?, r);
        intent(&owner, &mut leader, GuildIntent::Leave { guild }, 2).await?; // leader cannot leave
        intent(&owner, &mut one, GuildIntent::Join { guild }, 5).await?; // not invited
        intent(
            &owner,
            &mut leader,
            GuildIntent::Invite {
                guild,
                name: "Guest One".into(),
            },
            1,
        )
        .await?;
        let invitations = query(&owner, &mut one, GuildQuery::Invitations).await?;
        assert!(
            matches!(invitations,GuildQueryResult::Invitations(ref list) if list.len()==1 && list[0].guild==guild)
        );
        intent(&owner, &mut one, GuildIntent::Join { guild }, 1).await?;
        assert_eq!(
            query(&owner, &mut one, GuildQuery::Invitations).await?,
            GuildQueryResult::Invitations(vec![])
        );
        let GuildQueryResult::Own(Some(summary)) =
            query(&owner, &mut leader, GuildQuery::Own).await?
        else {
            return Err("missing own guild".into());
        };
        assert_eq!(summary.name, "Wire Guild");
        assert_eq!(summary.own_rank, 1);
        assert_eq!(summary.ranks, vec!["Leader", "Vice Leader", "Member"]);
        intent(
            &owner,
            &mut leader,
            GuildIntent::SetRank {
                guild,
                character,
                level: 2,
                expected_revision: summary.revision - 1,
            },
            13,
        )
        .await?;
        intent(
            &owner,
            &mut leader,
            GuildIntent::SetRank {
                guild,
                character,
                level: 2,
                expected_revision: summary.revision,
            },
            1,
        )
        .await?;
        intent(
            &owner,
            &mut one,
            GuildIntent::Kick {
                guild,
                character: *leader.item_fence.ok_or("leader")?.character_id.as_bytes(),
            },
            2,
        )
        .await?;
        intent(
            &owner,
            &mut leader,
            GuildIntent::Invite {
                guild,
                name: "Guest Two".into(),
            },
            1,
        )
        .await?;
        intent(
            &owner,
            &mut leader,
            GuildIntent::RevokeInvite {
                guild,
                character: second,
            },
            1,
        )
        .await?;
        intent(&owner, &mut two, GuildIntent::Join { guild }, 5).await?;
        intent(
            &owner,
            &mut leader,
            GuildIntent::Invite {
                guild,
                name: "Guest Two".into(),
            },
            1,
        )
        .await?;
        intent(&owner, &mut two, GuildIntent::Join { guild }, 1).await?;
        intent(
            &owner,
            &mut leader,
            GuildIntent::Kick {
                guild,
                character: second,
            },
            1,
        )
        .await?;
        assert_eq!(
            query(&owner, &mut two, GuildQuery::Own).await?,
            GuildQueryResult::Own(None)
        );
        let GuildQueryResult::Roster { members, next, .. } =
            query(&owner, &mut one, GuildQuery::Roster { guild, after: None }).await?
        else {
            return Err("missing roster".into());
        };
        assert_eq!(members.len(), 2);
        assert_eq!(members[1].character, character);
        assert_eq!(members[1].level, 2);
        assert!(next.is_none());
        intent(&owner, &mut one, GuildIntent::Leave { guild }, 1).await?;
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_guild_members WHERE guild_id=encode($1,'hex')::uuid",
        )
        .bind(guild.as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(count, 1);
        let revision:String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(revision, "1");
        let mut old = leader;
        old.continuity.selected_capabilities = SelectedCapabilities::NONE;
        let command = old.continuity.next_command_id;
        assert_eq!(
            exchange(
                &owner,
                &mut old,
                command,
                COMMAND_TYPE_GUILD_QUERY,
                &encode_guild_query(&GuildQuery::Own).map_err(debug)?
            )
            .await?
            .0,
            CommandStatus::Rejected
        );
        let command = old.continuity.next_command_id;
        assert_eq!(
            exchange(
                &owner,
                &mut old,
                command,
                COMMAND_TYPE_GUILD_INTENT,
                &encode_guild_intent(&GuildIntent::Found {
                    name: "No Capability".into()
                })
                .map_err(debug)?
            )
            .await?
            .0,
            CommandStatus::Rejected
        );
        // Capability-negative replacements also consume fresh IDs on the same session.
        let selected = leader.continuity.selected_capabilities;
        leader.continuity = old.continuity;
        leader.continuity.selected_capabilities = selected;
        let command = leader.continuity.next_command_id;
        assert_eq!(
            exchange(
                &owner,
                &mut leader,
                command,
                COMMAND_TYPE_GUILD_INTENT,
                &[0xff]
            )
            .await?
            .0,
            CommandStatus::Rejected
        );
        // No in-memory Guild cache: new composed owner and reopened authority read committed data.
        drop(owner);
        drop(authority);
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut restarted = deps.owner(&h, &authority);
        restarted.premium_coordinator = Some(&coordinator);
        deps.initialize(&restarted).await;
        // deps.owner builds a fresh ChannelSessionStates/SessionCommands directory. These
        // synthetic connections start at its initial continuity to exercise the durable
        // Guild journal, not to claim production same-GameSession recovery after truth loss.
        let mut reloaded = initial_leader;
        let GuildQueryResult::Own(Some(summary)) =
            query(&restarted, &mut reloaded, GuildQuery::Own).await?
        else {
            return Err("guild lost on reload".into());
        };
        assert_eq!(summary.name, "Wire Guild");
        assert_eq!(summary.revision, 10);
        assert_eq!(reloaded.continuity.next_command_id, found_command);
        // Here ingress has no retained founding result, so the changed input reaches the
        // native Guild receipt conflict check under the original durable command identity.
        let (status, bytes) = exchange(
            &restarted,
            &mut reloaded,
            found_command,
            COMMAND_TYPE_GUILD_INTENT,
            &changed,
        )
        .await?;
        assert_eq!(status, CommandStatus::Rejected);
        assert!(bytes.is_empty());
        drop(restarted);
        // A second fresh directory avoids replaying the conflict rejection retained above.
        // Reserve the preceding ID with a read, then replay the original durable receipt.
        let mut restarted = deps.owner(&h, &authority);
        restarted.premium_coordinator = Some(&coordinator);
        deps.initialize(&restarted).await;
        let mut replay = initial_leader;
        assert_eq!(
            query(&restarted, &mut replay, GuildQuery::Own).await?,
            GuildQueryResult::Own(Some(summary))
        );
        assert_eq!(replay.continuity.next_command_id, found_command);
        let (status, replayed) = exchange(
            &restarted,
            &mut replay,
            found_command,
            COMMAND_TYPE_GUILD_INTENT,
            &found,
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(replayed, receipt);
        assert_eq!(decode_guild_intent_result(&replayed).map_err(debug)?, r);
        let rows: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_guild_operations WHERE result=0")
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(rows, 10);
        let events: i64 = sqlx::query_scalar("SELECT count(*) FROM game_guild_event_outbox")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(events, 10);
        drop(restarted);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
