//! Bounded aliases preserve real actor/fence identities and unresolved callbacks.
use super::*;
use super::switch_tests::{native_switch, native_switch_with_pin};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::{AuthenticatedTransportRefV1, ConnectionGeneration, NodeId};
use crate::gameplay_transport::connection::{
    AdmissionRefusal, ControlLossResult, FreshAdmissionAttempt, FreshAdmissionAuthority,
    GraceExpiryResult, SessionContinuity,
};
use crate::gameplay_transport::session_ingress::{CommandAction, SessionCommandStream, SessionCommands};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use oteryn_foundation::CancellationToken;

fn debug(error: impl std::fmt::Debug) -> String { format!("{error:?}") }
fn id(tag: u8) -> [u8; 16] {
    let mut value = [tag; 16];
    value[6] = 0x70;
    value[8] = 0x80;
    value
}

/// Every destination uses an actor actually reserved and committed in its own
/// tagged carrier. No Channel actor is relabelled as an Instance actor.
fn subsequent_switch(
    source: AdmittedSession,
    tag: u8,
    pin: &crate::content::channel_content_pin::ChannelContentPin,
) -> Result<ActivityConnectionSwitch, String> {
    let old = source.item_fence.ok_or("source fence")?;
    let scope = match old.runtime_scope {
        RuntimeScopeRefV1::Channel { world_id, .. } => RuntimeScopeRefV1::instance(world_id, id(tag)).map_err(debug)?,
        RuntimeScopeRefV1::Instance { world_id, .. } => RuntimeScopeRefV1::channel(world_id, source.channel_id),
    };
    let session = GameSessionId::decode(&id(tag)).map_err(debug)?;
    // Reuse only immutable qualified content. Every hop still creates its own
    // carrier and reserves/commits the exact new scoped actor and session.
    let mut runtime = ChannelRuntimeV1::from_committed_scope_assignment(
        scope, NodeId::decode(&id(4)).map_err(debug)?, 1, 1, 1,
        "runtime-scope-assignment:1", 1, pin.clone(),
    ).map_err(debug)?;
    let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
    let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    let mut controller = source.controller.ok_or("controlled source")?;
    controller.transport = AuthenticatedTransportRefV1::decode(&id(tag + 1)).map_err(debug)?;
    let destination = AdmittedSession {
        game_session_id: session,
        runtime_actor: Some(actor),
        controller: Some(controller),
        item_fence: Some(CurrentCharacterItemFence {
            game_session_id: session,
            connection_generation: ConnectionGeneration::new(1).map_err(debug)?,
            character_lease_generation: old.character_lease_generation + 1,
            runtime_scope: scope,
            ..old
        }),
        continuity: SessionContinuity {
            selected_capabilities: source.continuity.selected_capabilities,
            ..SessionContinuity::FRESH
        },
        ..source
    };
    use oteryn_protocol_oteryn::activity_scope::{ActivityScope, ActivityScopeSnapshot, encode_activity_scope};
    let wire_scope = match scope {
        RuntimeScopeRefV1::Channel { world_id, channel_id } => ActivityScope::Channel {
            world: *world_id.as_bytes(), channel: *channel_id.as_bytes(),
        },
        RuntimeScopeRefV1::Instance { world_id, instance_id } => ActivityScope::Instance {
            world: *world_id.as_bytes(), instance: instance_id,
        },
    };
    let payload = encode_activity_scope(&ActivityScopeSnapshot {
        handoff: id(tag + 2),
        source_session: *source.game_session_id.as_bytes(),
        session: *session.as_bytes(),
        scope: wire_scope,
        scope_generation: 1,
        connection_generation: 1,
        lease_generation: old.character_lease_generation + 1,
        selection: source.continuity.selected_capabilities.as_slice().to_vec(),
    }).map_err(debug)?;
    Ok(ActivityConnectionSwitch { source, destination, payload })
}

#[test]
fn activity_lineage_many_delivered_hops_keep_only_the_original_socket_root() -> Result<(), String> {
    let registry = ActivityRuntimeRegistry::default();
    let (first, pin) = native_switch_with_pin()?;
    let original = first.source;
    registry.remember(original).map_err(debug)?;
    registry.stage_switch(first.clone()).map_err(debug)?;
    registry.finish_switch(original, first.destination).map_err(debug)?;
    let mut current = first.destination;
    let mut delivered = vec![first];
    for index in 0..48_u8 {
        let switch = subsequent_switch(current, 20 + index * 2, &pin)?;
        registry.stage_switch(switch.clone()).map_err(debug)?;
        assert_eq!(registry.successors.lock().map_err(debug)?.len(), 2, "one socket root plus the in-flight hop");
        assert_eq!(registry.current_connection(original), switch.destination);
        registry.finish_switch(current, switch.destination).map_err(debug)?;
        current = switch.destination;
        delivered.push(switch);
        assert_eq!(registry.successors.lock().map_err(debug)?.len(), 1, "delivered visits do not accumulate");
        assert!(registry.pending.lock().map_err(debug)?.is_empty());
        assert_eq!(registry.connections.lock().map_err(debug)?.len(), 1);
    }
    assert_eq!(registry.current_connection(original), current, "shutdown still resolves the physical socket's initial snapshot");
    registry.forget_terminal_connection(current.game_session_id, current.runtime_actor.ok_or("actor")?);
    assert_eq!(registry.successors.lock().map_err(debug)?.len(), 1, "terminal reconciliation still owns the root until its listener completes");
    registry.finish_connection_lineage(original).map_err(debug)?;
    assert!(registry.successors.lock().map_err(debug)?.is_empty());
    for switch in delivered { registry.stage_switch(switch).map_err(debug)?; }
    assert!(registry.connections.lock().map_err(debug)?.is_empty());
    assert!(registry.pending.lock().map_err(debug)?.is_empty());
    assert!(registry.successors.lock().map_err(debug)?.is_empty(), "cached committed results cannot resurrect a pruned lineage");
    Ok(())
}

#[test]
fn activity_lineage_lost_barrier_and_offline_return_keep_exact_pending_obligations() -> Result<(), String> {
    let registry = ActivityRuntimeRegistry::default();
    let (first, pin) = native_switch_with_pin()?;
    registry.remember(first.source).map_err(debug)?;
    registry.stage_switch(first.clone()).map_err(debug)?;
    let mut offline = subsequent_switch(first.destination, 20, &pin)?.destination;
    offline.controller = None;
    registry.remember_offline_return(first.destination, offline).map_err(debug)?;
    assert_eq!(registry.current_connection(first.source), offline);
    assert_eq!(registry.current_connection(first.destination), offline);
    assert_eq!(registry.account_for(offline), first.source.controller.map(|value| value.account_id));
    let mut foreign = offline;
    foreign.item_fence.as_mut().ok_or("fence")?.character_lease_generation += 1;
    assert!(registry.account_for(foreign).is_none(), "a current offline account cache is fenced by the complete lease");
    assert!(registry.pending_switch(first.source).map_err(debug)?.is_some(), "no acknowledgement means no pending-barrier cleanup");
    assert_eq!(registry.successors.lock().map_err(debug)?.len(), 2);

    registry.forget_terminal_connection(offline.game_session_id, offline.runtime_actor.ok_or("actor")?);
    assert!(registry.account_for(offline).is_none());
    // A delayed source write completing now must not reinsert the earlier B
    // destination, which has already returned and reached terminal C.
    registry.finish_switch(first.source, first.destination).map_err(debug)?;
    assert!(registry.connection(first.destination.game_session_id).is_none());
    assert!(registry.connection(offline.game_session_id).is_none());
    assert_eq!(registry.current_connection(first.source), offline);
    registry.finish_connection_lineage(first.destination).map_err(debug)?;
    assert!(registry.successors.lock().map_err(debug)?.is_empty());
    assert!(registry.pending.lock().map_err(debug)?.is_empty());
    registry.stage_switch(first.clone()).map_err(debug)?;
    registry.remember_offline_return(first.destination, offline).map_err(debug)?;
    assert!(registry.connections.lock().map_err(debug)?.is_empty());
    assert!(registry.offline_accounts.lock().map_err(debug)?.is_empty());
    Ok(())
}

#[test]
fn activity_lineage_old_socket_cleanup_never_follows_a_recovered_controller() -> Result<(), String> {
    let registry = ActivityRuntimeRegistry::default();
    let (first, pin) = native_switch_with_pin()?;
    registry.remember(first.source).map_err(debug)?;
    registry.stage_switch(first.clone()).map_err(debug)?;
    registry.finish_switch(first.source, first.destination).map_err(debug)?;
    let mut recovered = first.destination;
    recovered.continuity.connection_generation = 2;
    recovered.item_fence.as_mut().ok_or("fence")?.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
    recovered.controller.as_mut().ok_or("controller")?.transport = AuthenticatedTransportRefV1::decode(&id(18)).map_err(debug)?;
    registry.remember(recovered).map_err(debug)?;
    let next = subsequent_switch(recovered, 20, &pin)?;
    registry.stage_switch(next.clone()).map_err(debug)?;
    registry.finish_switch(recovered, next.destination).map_err(debug)?;
    assert_eq!(registry.current_connection(first.source), first.destination, "old physical transport stops before the later controller");
    assert_eq!(registry.current_connection(recovered), next.destination);
    registry.finish_connection_lineage(first.source).map_err(debug)?;
    assert_eq!(registry.successors.lock().map_err(debug)?.len(), 1);
    assert_eq!(registry.current_connection(recovered), next.destination);
    assert_eq!(registry.connection(next.destination.game_session_id), Some(next.destination));
    registry.stage_switch(first).map_err(debug)?;
    assert_eq!(registry.connection(next.destination.game_session_id), Some(next.destination));
    assert!(registry.pending.lock().map_err(debug)?.is_empty());
    Ok(())
}

#[test]
fn activity_lineage_source_stream_can_finish_its_result_after_directory_release() -> Result<(), String> {
    let source = native_switch()?.source;
    let commands = SessionCommands::default();
    commands.admit(source.game_session_id);
    let mut stream = SessionCommandStream::open(Some(&commands), &source).ok_or("source stream")?;
    assert!(matches!(stream.begin(1, 1, 0, false), Ok(CommandAction::Execute)));
    commands.forget(source.game_session_id);
    assert!(commands.reserve_activity_recovery(&source).is_none(), "a released source directory cannot authorize a fresh recovery reservation");
    let result = oteryn_protocol_oteryn::encode_command_result(1, 1, 1, oteryn_protocol_oteryn::CommandStatus::Accepted, &[9]).map_err(debug)?;
    stream.terminalize(1, &result).ok_or("held source result still completes")?;
    drop(stream);
    let mut old = source;
    old.continuity.next_command_id = 2;
    old.continuity.server_sequence = 1;
    assert!(SessionCommandStream::open(Some(&commands), &old).is_none(), "no retired high-water can be reconstructed from a client counter");
    Ok(())
}

struct LifecycleAuthority {
    registry: ActivityRuntimeRegistry<'static>,
    loss: ControlLossResult,
    expiry: GraceExpiryResult,
    shutdown: CancellationToken,
    closed: AtomicUsize,
}
impl FreshAdmissionAuthority for LifecycleAuthority {
    async fn admit(&self, _attempt: FreshAdmissionAttempt<'_>) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Unavailable)
    }
    async fn settle_activity_connection(&self, admitted: AdmittedSession) -> Result<AdmittedSession, ()> {
        Ok(self.registry.current_connection(admitted))
    }
    async fn lose_control(&self, _admitted: AdmittedSession, _wait: Duration, _reset: Option<&crate::durability::world_reset::RetiringWorldReset>) -> ControlLossResult {
        if self.loss == ControlLossResult::Unknown { self.shutdown.cancel(); }
        self.loss
    }
    async fn expire_control_loss(&self, _admitted: AdmittedSession) -> GraceExpiryResult {
        if self.expiry == GraceExpiryResult::Unknown { self.shutdown.cancel(); }
        self.expiry
    }
    async fn reconcile_terminal(&self, _admitted: AdmittedSession) -> GraceExpiryResult { self.expiry }
    fn finish_activity_connection_lineage(&self, admitted: AdmittedSession) {
        if self.registry.finish_connection_lineage(admitted).is_ok() {
            self.closed.fetch_add(1, Ordering::SeqCst);
        }
    }
}

#[test]
fn activity_lineage_listener_completion_retains_only_unknown_and_cancelled_work() -> Result<(), String> {
    eprintln!("ACTIVITY_LINEAGE_LIFECYCLE fixture_begin");
    let switch = native_switch()?;
    eprintln!("ACTIVITY_LINEAGE_LIFECYCLE fixture_complete");
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .map_err(debug)?
        .block_on(async {
    for (loss, expiry, completes) in [
        (ControlLossResult::Recorded, GraceExpiryResult::Released, true),
        (ControlLossResult::Terminal, GraceExpiryResult::Released, true),
        (ControlLossResult::NotApplicable, GraceExpiryResult::NotApplicable, true),
        (ControlLossResult::Recorded, GraceExpiryResult::Unknown, false),
        (ControlLossResult::Unknown, GraceExpiryResult::Unknown, false),
        (ControlLossResult::Refused, GraceExpiryResult::NotApplicable, true),
    ] {
        let authority = LifecycleAuthority {
            registry: ActivityRuntimeRegistry::default(), loss, expiry,
            shutdown: CancellationToken::new(), closed: AtomicUsize::new(0),
        };
        eprintln!("ACTIVITY_LINEAGE_LIFECYCLE case_begin loss={loss:?} expiry={expiry:?}");
        authority.registry.remember(switch.source).map_err(debug)?;
        authority.registry.stage_switch(switch.clone()).map_err(debug)?;
        tokio::time::timeout(
            Duration::from_secs(5),
            crate::gameplay_transport::listener::control_loss_lifecycle(
                &authority,
                crate::gameplay_transport::listener::LostControl {
                    session: switch.source, wait: Duration::ZERO, reset: None,
                },
                &authority.shutdown,
            ),
        ).await.map_err(|_| format!("lineage lifecycle timeout: loss={loss:?}, expiry={expiry:?}"))?;
        eprintln!("ACTIVITY_LINEAGE_LIFECYCLE case_complete loss={loss:?} expiry={expiry:?}");
        assert_eq!(authority.closed.load(Ordering::SeqCst), usize::from(completes));
        assert_eq!(authority.registry.successors.lock().map_err(debug)?.is_empty(), completes);
        assert_eq!(authority.registry.pending.lock().map_err(debug)?.is_empty(), completes);
        assert_eq!(authority.registry.connection(switch.destination.game_session_id), Some(switch.destination), "lineage cleanup never changes current playable control");
    }
    Ok(())
        })
}
