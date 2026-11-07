//! Real native handoff recovery: registered S2 credentials, actual stream
//! ownership, retained protection, and return-before-release on cancellation.
use super::{BOSS, Harness, TestResult, debug, fixture};
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{
    AuthenticatedTransportRefV1, ControlLossHistoryV1, GameSessionId, GameSessionState,
    MessageType, RecoveryProtectionContinuityV1, RecoveryProtectionUseV1, RuntimeScopeRefV1,
    decode_wire_envelope,
};
use crate::gameplay_transport::activity_recovery_admission::{
    RECOVERY_COMMITTED_TEST_BARRIER, RecoveryCommittedTestBarrier,
};
use crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES;
use crate::gameplay_transport::connection::{
    AdmissionRefusal, AdmittedSession, ConnectionEnd, ConnectionIdentifiers, ControlLossResult,
    FreshAdmissionAuthority, IDLE_LIVENESS, ResumeAttempt, serve_admitted,
};
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use crate::gameplay_transport::{ComposedFreshAdmission, item_fence_of};
use oteryn_protocol_oteryn::activity_scope::{STATE_DOMAIN_ACTIVITY_SCOPE, decode_activity_scope};
use oteryn_protocol_oteryn::boss_encounter as wire;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn token(nonce: u8) -> TestResult<String> {
    super::super::house_transfer::RecoverySource::token(nonce)
}
fn transport(seed: u8) -> TestResult<AuthenticatedTransportRefV1> {
    AuthenticatedTransportRefV1::decode(&[seed; 16]).map_err(debug)
}

pub(super) async fn prepare_source_reentry(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult<RecoveryProtectionContinuityV1> {
    owner.states.command_ingress.admit(admitted.game_session_id);
    assert_eq!(
        owner
            .lose_control(*admitted, std::time::Duration::ZERO, None)
            .await,
        ControlLossResult::Recorded
    );
    *admitted = resume_wire(owner, admitted.game_session_id, &token(201)?, 201).await?;
    let Some(ControlLossHistoryV1::Resumed { protection, .. }) =
        FreshAdmissionStore::from_root(h.root.clone())
            .resumed_history(admitted.game_session_id)
            .await?
    else {
        return Err("real source CompleteReconnect history".into());
    };
    assert!(matches!(
        protection.usage,
        RecoveryProtectionUseV1::Activated { .. }
    ));
    Ok(protection)
}

pub(super) async fn assert_recovery(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    source: AdmittedSession,
    admitted: AdmittedSession,
) -> TestResult {
    let supported: Vec<u32> = BUNDLE_WORLD_OFFERED_CAPABILITIES
        .iter()
        .map(|offer| offer.id)
        .collect();
    let store = FreshAdmissionStore::from_root(h.root.clone());
    let destination = owner
        .resolve_runtime_view(admitted)
        .await
        .map_err(|_| "Instance view")?
        .ok_or("Instance owner")?;
    let actor = admitted.runtime_actor.ok_or("actual actor")?;
    let before = {
        let runtime = destination.runtime.lock().await;
        let states = destination.states.spell_states.lock().await;
        let state = states
            .get(&runtime, actor, admitted.game_session_id)
            .ok_or("retained live spell state")?;
        (
            runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            state.revision(),
            state.vitals(),
            state.saved_vitals(),
        )
    };
    let rejected = owner
        .resume_session(ResumeAttempt {
            game_session_id: source.game_session_id,
            recovery_material: b"not-a-credential",
            transport: transport(202)?,
            last_applied_server_sequence: source.continuity.server_sequence,
            supported_capabilities: &supported,
        })
        .await;
    assert!(matches!(rejected, Err(AdmissionRefusal::Rejected)));
    assert_eq!(
        item_fence_of(store.current_session_at(admitted.game_session_id).await?.0),
        admitted.item_fence
    );
    let mut resumed = resume_wire(owner, source.game_session_id, &token(203)?, 203).await?;
    assert_eq!(resumed.game_session_id, admitted.game_session_id);
    assert_eq!(resumed.runtime_actor, admitted.runtime_actor);
    assert_eq!(resumed.continuity.connection_generation, 2);
    assert_eq!(resumed.continuity.next_command_id, 1);
    assert_eq!(
        resumed.continuity.selected_capabilities,
        admitted.continuity.selected_capabilities
    );
    let current = store.current_session_at(resumed.game_session_id).await?.0;
    assert_eq!(
        current.current_control_loss_epoch(),
        None,
        "lost acknowledgement is not a fabricated loss"
    );
    assert_eq!(current.current_original_grace_deadline(), None);
    {
        let runtime = destination.runtime.lock().await;
        let states = destination.states.spell_states.lock().await;
        let state = states
            .get(&runtime, actor, resumed.game_session_id)
            .ok_or("same live state after adoption")?;
        assert_eq!(
            (
                runtime
                    .read_actor_position(actor)
                    .map_err(debug)?
                    .position(),
                state.revision(),
                state.vitals(),
                state.saved_vitals()
            ),
            before
        );
        assert!(
            !runtime
                .current_player_reentry_protection(
                    actor,
                    resumed.game_session_id,
                    destination.owner_now().get()
                )
                .map_err(debug)?,
            "zero-loss adoption never grants CompleteReconnect protection"
        );
    }
    let projected = decode_activity_scope(
        &owner
            .project_activity_scope(resumed)
            .await
            .map_err(|_| "project scope")?
            .ok_or("scope99")?,
    )
    .map_err(debug)?;
    assert_eq!(projected.source_session, *source.game_session_id.as_bytes());
    assert_eq!(projected.session, *resumed.game_session_id.as_bytes());
    assert_eq!(projected.connection_generation, 2);
    resumed = first_command_with_scope_snapshot(owner, resumed).await?;
    assert_eq!(resumed.continuity.next_command_id, 2);
    let credential = token(204)?;
    assert!(
        matches!(
            owner
                .resume_session(ResumeAttempt {
                    game_session_id: source.game_session_id,
                    recovery_material: credential.as_bytes(),
                    transport: transport(204)?,
                    last_applied_server_sequence: 0,
                    supported_capabilities: &supported,
                })
                .await,
            Err(AdmissionRefusal::Rejected)
        ),
        "first real input closes historical lost-ack recovery"
    );
    assert_eq!(
        owner
            .lose_control(resumed, std::time::Duration::ZERO, None)
            .await,
        ControlLossResult::Recorded
    );
    let after = resume_wire(owner, resumed.game_session_id, &token(205)?, 205).await?;
    assert_eq!(after.runtime_actor, Some(actor));
    assert_eq!(
        after.item_fence.ok_or("resumed fence")?.runtime_scope,
        admitted.item_fence.ok_or("entry fence")?.runtime_scope
    );
    assert_eq!(after.continuity.connection_generation, 3);
    assert_eq!(
        after.continuity.next_command_id, 2,
        "ordinary reconnect keeps command continuity"
    );
    let counts:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_activity_transfer_recoveries),(SELECT count(*) FROM game_activity_runtime_entries)")
        .fetch_one(&h.pool).await?;
    assert_eq!(counts, (1, 1));
    eprintln!("BOSS_ACTIVITY_AUTHENTICATED_RECOVERY POSTGRES_CASE_COMPLETED");
    eprintln!("BOSS_ACTIVITY_SAME_SESSION_RECONNECT POSTGRES_CASE_COMPLETED");
    Ok(())
}

pub(super) async fn assert_cancelled_recovery(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    source: AdmittedSession,
    admitted: AdmittedSession,
) -> TestResult {
    let supported: Vec<u32> = BUNDLE_WORLD_OFFERED_CAPABILITIES
        .iter()
        .map(|offer| offer.id)
        .collect();
    let barrier = std::sync::Arc::new(RecoveryCommittedTestBarrier::default());
    let credential = token(206)?;
    let recovering = RECOVERY_COMMITTED_TEST_BARRIER.scope(
        std::sync::Arc::clone(&barrier),
        owner.resume_session(ResumeAttempt {
            game_session_id: source.game_session_id,
            recovery_material: credential.as_bytes(),
            transport: transport(206)?,
            last_applied_server_sequence: 0,
            supported_capabilities: &supported,
        }),
    );
    // Drop the real socket future after PostgreSQL committed but before any
    // native installation/acceptance, exactly the former stranded-ACTIVE gap.
    let outcome = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        crate::gameplay_transport::first(recovering, barrier.entered.notified()),
    )
    .await
    .map_err(|error| format!("activity recovery durable-commit barrier: {error}"))?;
    assert!(outcome.is_none(), "reached committed cancellation barrier");
    eprintln!("ACTIVITY_RECOVERY_CANCELLED after_durable_commit");
    // The committed recovery advanced the real connection generation. Reading
    // retained combat locks must work in that exact Instance before release;
    // each negative changes only one authority field of this current fence.
    let current = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "current recovered Instance gameplay fence")?
        .ok_or("live recovered Instance gameplay fence")?;
    let old = admitted.item_fence.ok_or("pre-recovery Instance fence")?;
    assert_eq!(current.runtime_scope, old.runtime_scope);
    assert!(matches!(
        current.runtime_scope,
        RuntimeScopeRefV1::Instance { .. }
    ));
    let _stored = h
        .root
        .read_pvp_state(owner.character, &h.node, current)
        .await
        .map_err(|error| format!("current Instance combat-lock read: {error:?}"))?;
    let mut borrowed_channel = current;
    borrowed_channel.runtime_scope = source
        .item_fence
        .ok_or("origin Channel fence")?
        .runtime_scope;
    assert!(matches!(
        borrowed_channel.runtime_scope,
        RuntimeScopeRefV1::Channel { .. }
    ));
    assert!(matches!(
        h.root
            .read_pvp_state(owner.character, &h.node, borrowed_channel)
            .await,
        Err(CharacterProgressionError::AuthorityRejected)
    ));
    let mut stale_connection = current;
    stale_connection.connection_generation = old.connection_generation;
    assert_ne!(
        stale_connection.connection_generation,
        current.connection_generation
    );
    assert!(matches!(
        h.root
            .read_pvp_state(owner.character, &h.node, stale_connection)
            .await,
        Err(CharacterProgressionError::AuthorityRejected)
    ));
    let mut pending = owner.abandoned_activity_recoveries();
    assert_eq!(pending.len(), 1);
    let (_, pending) = pending.pop().ok_or("retained cancellation")?;
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        owner.reconcile_abandoned_activity_recovery(pending),
    )
    .await
    .map_err(|error| format!("activity recovery abandoned return/release: {error}"))?;
    assert!(owner.abandoned_activity_recoveries().is_empty());
    assert_terminal_return(h, owner, admitted, 8).await?;
    eprintln!("BOSS_ACTIVITY_CANCELLED_RECOVERY_RELEASE POSTGRES_CASE_COMPLETED");
    Ok(())
}

pub(super) async fn assert_capability_mismatch(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    assert_eq!(
        owner
            .lose_control(admitted, std::time::Duration::ZERO, None)
            .await,
        ControlLossResult::Recorded
    );
    let supported: Vec<_> = BUNDLE_WORLD_OFFERED_CAPABILITIES
        .iter()
        .map(|offer| offer.id)
        .filter(|cap| {
            *cap != oteryn_protocol_oteryn::boss_difficulty::CAPABILITY_BOSS_DIFFICULTY_V1
        })
        .collect();
    let credential = token(207)?;
    let refused = owner
        .resume_session(ResumeAttempt {
            game_session_id: admitted.game_session_id,
            recovery_material: credential.as_bytes(),
            transport: transport(207)?,
            last_applied_server_sequence: 0,
            supported_capabilities: &supported,
        })
        .await;
    assert!(
        matches!(refused, Err(AdmissionRefusal::Rejected)),
        "authenticated D449 path, got {refused:?}"
    );
    assert_terminal_return(h, owner, admitted, 9).await?;
    eprintln!("BOSS_ACTIVITY_CAPABILITY_MISMATCH_RELEASE POSTGRES_CASE_COMPLETED");
    Ok(())
}

async fn assert_terminal_return(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    source: AdmittedSession,
    reason: i16,
) -> TestResult {
    type ActivityReturnRow = (
        Vec<u8>,
        i16,
        Option<String>,
        Option<Vec<u8>>,
        Option<String>,
    );
    let row: ActivityReturnRow = sqlx::query_as(
        "SELECT uuid_send(destination_game_session_id),reason,command_id::text,release_transport_ref,release_loss_epoch::text FROM game_activity_runtime_returns WHERE source_game_session_id=encode($1,'hex')::uuid")
        .bind(source.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(row.1, reason);
    assert_eq!(row.2, None, "no synthetic client command");
    assert_eq!(row.3.is_some(), reason == 8);
    assert_eq!(row.4.is_some(), reason == 9);
    let session = GameSessionId::decode(&row.0).map_err(debug)?;
    let store = FreshAdmissionStore::from_root(h.root.clone());
    assert_eq!(
        store
            .current_session_at(source.game_session_id)
            .await?
            .0
            .session_state(),
        GameSessionState::Terminal
    );
    assert_eq!(
        store.current_session_at(session).await?.0.session_state(),
        GameSessionState::Terminal
    );
    let instance = owner.resolve_runtime_view(source).await;
    if let Ok(Some(instance)) = instance {
        assert!(
            instance
                .runtime
                .lock()
                .await
                .activity_participant_sessions()
                .map_err(debug)?
                .is_empty()
        );
    }
    assert!(
        owner
            .runtime
            .lock()
            .await
            .activity_participant_sessions()
            .map_err(debug)?
            .is_empty()
    );
    let unfinished: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_boss_room_participants WHERE returned_at IS NULL",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        unfinished, 0,
        "terminal release cannot orphan a room participant"
    );
    Ok(())
}

pub(super) async fn assert_inherited_protection(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    inherited: RecoveryProtectionContinuityV1,
) -> TestResult {
    let store = FreshAdmissionStore::from_root(h.root.clone());
    assert!(
        matches!(store.activity_loss_origin(admitted.game_session_id).await?,
        Some(ControlLossHistoryV1::ActivityOrigin {protection,..}) if protection==inherited)
    );
    assert_eq!(
        owner
            .lose_control(admitted, std::time::Duration::ZERO, None)
            .await,
        ControlLossResult::Recorded
    );
    let resumed = resume_wire(owner, admitted.game_session_id, &token(208)?, 208).await?;
    let Some(ControlLossHistoryV1::Resumed { protection, .. }) =
        store.resumed_history(resumed.game_session_id).await?
    else {
        return Err("Activity real resumed history".into());
    };
    assert_eq!(
        protection, inherited,
        "entry and another real loss do not renew the durable entitlement/deadline"
    );
    let destination = owner
        .resolve_runtime_view(resumed)
        .await
        .map_err(|_| "Instance owner")?
        .ok_or("Instance owner")?;
    // Exercise the shared native Exit owner with a command actually reserved
    // in its retained stream; no client result or arbitrary command zero exists.
    let mut stream = crate::gameplay_transport::session_ingress::SessionCommandStream::open(
        Some(&destination.states.command_ingress),
        &resumed,
    )
    .ok_or("return command stream")?;
    let id = resumed.continuity.next_command_id;
    assert!(matches!(
        stream.begin(
            id,
            resumed.continuity.connection_generation,
            resumed.continuity.server_sequence,
            false
        ),
        Ok(crate::gameplay_transport::session_ingress::CommandAction::Execute)
    ));
    assert!(destination.note_activity_input(resumed).await);
    let result = destination
        .return_native_activity(
            resumed,
            id,
            crate::durability::activity_handoff::ActivityReturnReason::Exit,
        )
        .await
        .map_err(|_| "native return")?;
    let crate::gameplay_transport::native_activity::NativeActivityOutcome::Committed(commit) =
        result
    else {
        return Err(format!("protection return refused: {result:?}").into());
    };
    owner
        .publish_activity_commit(&commit)
        .map_err(|_| "publish return")?;
    let next = owner.activity_runtimes.current_connection(resumed);
    assert_ne!(next.game_session_id, resumed.game_session_id);
    assert!(matches!(
        next.item_fence.ok_or("origin fence")?.runtime_scope,
        RuntimeScopeRefV1::Channel { .. }
    ));
    assert!(
        matches!(store.activity_loss_origin(next.game_session_id).await?,
        Some(ControlLossHistoryV1::ActivityOrigin {protection,..}) if protection==inherited)
    );
    eprintln!("BOSS_ACTIVITY_INHERITED_RECOVERY_PROTECTION POSTGRES_CASE_COMPLETED");
    Ok(())
}

async fn first_command_with_scope_snapshot(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult<AdmittedSession> {
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let driving = async {
        let mut saw_scope = false;
        loop {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == MessageType::SnapshotChunk {
                let (_, domains) =
                    oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())
                        .map_err(debug)?;
                for domain in domains {
                    if domain.domain_id == STATE_DOMAIN_ACTIVITY_SCOPE {
                        let scope = decode_activity_scope(domain.payload).map_err(debug)?;
                        assert_eq!(scope.session, *admitted.game_session_id.as_bytes());
                        assert_eq!(
                            scope.connection_generation,
                            admitted.continuity.connection_generation
                        );
                        saw_scope = true;
                    }
                }
            }
            if envelope.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        assert!(
            saw_scope,
            "resumed Instance initial snapshot carries its exact scope"
        );
        let body = wire::encode_boss_key(BOSS).map_err(debug)?;
        write_frame(
            &mut client,
            &oteryn_protocol_oteryn::encode_client_command(
                admitted.continuity.connection_generation,
                &oteryn_protocol_oteryn::ClientCommandValue {
                    command_id: 1,
                    command_type: wire::COMMAND_TYPE_BOSS_TIMEOUT,
                    payload: &body,
                },
            )
            .map_err(debug)?,
        )
        .await?;
        loop {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == MessageType::CommandResult {
                let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())
                    .map_err(debug)?;
                assert_eq!(result.command_id, 1);
                break;
            }
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (end, result) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        fixture::join_two(serving, driving),
    )
    .await?;
    result?;
    match end {
        ConnectionEnd::AdmittedThenDisconnected(next)
        | ConnectionEnd::AdmittedThenClosed(next, _) => Ok(next),
        end => Err(format!("recovered command stream ended: {end:?}").into()),
    }
}

struct Identifiers(u8);
impl ConnectionIdentifiers for Identifiers {
    fn game_session_id(&self) -> Option<GameSessionId> {
        None
    }
    fn transport_ref(&self) -> Option<AuthenticatedTransportRefV1> {
        AuthenticatedTransportRefV1::decode(&[self.0; 16]).ok()
    }
}
async fn resume_wire(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    credential: &str,
    seed: u8,
) -> TestResult<AdmittedSession> {
    let mut body = Vec::new();
    bytes(&mut body, 1, session.as_bytes());
    bytes(&mut body, 2, credential.as_bytes());
    body.extend([24, 0]);
    scalar(&mut body, 4, 1);
    scalar(&mut body, 5, 1);
    scalar(&mut body, 6, 1);
    for capability in BUNDLE_WORLD_OFFERED_CAPABILITIES {
        scalar(&mut body, 7, u64::from(capability.id));
    }
    bytes(&mut body, 8, b"native-activity-recovery-test");
    let mut frame = vec![8, MessageType::ClientResume as u8];
    bytes(&mut frame, 4, &body);
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let admitted = crate::gameplay_transport::connection::admit_frame(
        &mut server,
        &frame,
        owner,
        &Identifiers(seed),
    )
    .await
    .map_err(|error| format!("activity ClientResume: {error:?}"))?;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    let n = u32::from_be_bytes(output.get(..4).ok_or("resume reply")?.try_into()?) as usize;
    let envelope =
        decode_wire_envelope(output.get(4..4 + n).ok_or("resume frame")?).map_err(debug)?;
    assert_eq!(envelope.message_type(), MessageType::ServerResumeAccepted);
    Ok(admitted)
}
fn scalar(out: &mut Vec<u8>, tag: u8, mut value: u64) {
    out.push(tag << 3);
    while value >= 128 {
        out.push(value as u8 | 128);
        value >>= 7;
    }
    out.push(value as u8);
}
fn bytes(out: &mut Vec<u8>, tag: u8, value: &[u8]) {
    out.push(tag << 3 | 2);
    let mut n = value.len();
    while n >= 128 {
        out.push(n as u8 | 128);
        n >>= 7;
    }
    out.push(n as u8);
    out.extend_from_slice(value);
}
