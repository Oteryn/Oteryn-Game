use super::TestResult;

use super::fixture::debug;

use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RecoveryPoint {
    BeforeNames,
    BeforeWriteWithLoss,
    AfterWrite,
    AfterWriteWithLoss,
    Handoff,
}

/// Cancel the actual family writer while the original ingress owns its command.
/// No substitute command, synthetic durable receipt or connected native tick is
/// needed to recover the same occurrence and replay its ordinary wire result.
pub(super) async fn recover_source_use(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    command_id: u64,
    intent: &UseIntent,
    point: RecoveryPoint,
    expected: UseDisposition,
) -> TestResult<Vec<u8>> {
    use crate::foundation::{CommandId, CommandRef, MessageType, decode_wire_envelope};
    use crate::gameplay_transport::connection::UseCommand;
    use crate::gameplay_transport::session_ingress::{CommandAction, SessionCommandStream};
    let actor = admitted.runtime_actor.ok_or("source recovery actor")?;
    let session = admitted.game_session_id;
    let reference = CommandRef::new(session, CommandId::new(command_id).map_err(debug)?);
    let command = UseCommand {
        game_session_id: session,
        command_id,
        item_fence: admitted.item_fence,
    };
    let sequence = admitted.continuity.server_sequence;
    let generation = admitted.continuity.connection_generation;
    let commands = &owner.states.command_ingress;
    assert_eq!(admitted.continuity.next_command_id, command_id);
    let mut stream = SessionCommandStream::open(Some(commands), admitted)
        .ok_or("original source ingress stream")?;
    assert!(matches!(
        stream.begin(command_id, generation, sequence, false),
        Ok(CommandAction::Execute)
    ));
    if point == RecoveryPoint::Handoff {
        owner.refuse_soul_war_handoff_once_for_test(reference);
        assert!(
            owner
                .dispatch_soul_war_source_use(actor, command, intent, None, None)
                .await
                .is_err(),
            "a failed ingress handoff retains the completed source occurrence"
        );
    } else {
        let reached = match point {
            RecoveryPoint::BeforeNames => owner.pause_soul_war_before_names_for_test(reference),
            RecoveryPoint::BeforeWriteWithLoss => {
                owner.pause_soul_war_before_write_for_test(reference)
            }
            RecoveryPoint::AfterWrite | RecoveryPoint::AfterWriteWithLoss => {
                owner.pause_soul_war_after_write_for_test(reference)
            }
            RecoveryPoint::Handoff => unreachable!(),
        };
        let pending = owner.dispatch_soul_war_source_use(actor, command, intent, None, None);
        tokio::pin!(pending);
        tokio::select! {
            _ = reached.notified() => {}
            _ = &mut pending => return Err("source USE missed its exact cancellation point".into()),
        }
        // Dropping this future releases its lane/SQL cursor, never its source
        // occurrence or its unresolved original ingress reservation.
    }
    drop(stream);
    assert!(commands.pending_reconciliation(reference));
    assert!(!commands.original_terminalized(reference));
    assert!(!commands.resumable(admitted));
    let _ = commands.recover_trade_continuity(admitted);
    assert!(commands.pending_reconciliation(reference));
    assert!(!commands.resumable(admitted));
    assert_eq!(admitted.continuity.next_command_id, command_id);
    assert_eq!(admitted.continuity.server_sequence, sequence);
    let lost = matches!(
        point,
        RecoveryPoint::BeforeWriteWithLoss | RecoveryPoint::AfterWriteWithLoss
    );
    if lost {
        let mut runtime = owner.runtime.lock().await;
        assert!(!runtime.actor_spell_reserved(actor));
        runtime
            .record_control_loss(
                actor,
                session,
                crate::foundation::ControlLossMark {
                    epoch: 1,
                    grace_deadline: 100,
                },
            )
            .map_err(debug)?;
    }
    if point == RecoveryPoint::BeforeNames {
        let permit = owner
            .spell_lane_permit()
            .await
            .ok_or("source recovery lane")?;
        owner
            .drain_retained_soul_war_uses(&permit)
            .await
            .map_err(|_| "native drain must include the all-none preparation phase")?;
    } else {
        assert!(
            owner
                .recover_pending_soul_war_source_use(actor, session)
                .await,
            "the lifecycle hook must recover the last disconnected actor without a native tick"
        );
    }
    assert!(commands.original_terminalized(reference));
    assert!(!commands.pending_reconciliation(reference));
    commands
        .recover_trade_continuity(admitted)
        .ok_or("known source result must complete the original ingress reservation")?;
    assert_eq!(admitted.continuity.next_command_id, command_id + 1);
    assert_eq!(admitted.continuity.server_sequence, sequence + 1);
    assert!(commands.resumable(admitted));
    let mut resumed = SessionCommandStream::open(Some(commands), admitted)
        .ok_or("reopen after retained source recovery")?;
    let CommandAction::Replay(frame) = resumed
        .begin(command_id, generation, sequence + 1, false)
        .map_err(debug)?
    else {
        return Err("the original source command must replay instead of executing again".into());
    };
    let envelope = decode_wire_envelope(&frame).map_err(debug)?;
    assert_eq!(envelope.message_type(), MessageType::CommandResult);
    assert_eq!(envelope.connection_generation(), generation);
    assert_eq!(envelope.server_sequence(), sequence + 2);
    let result =
        oteryn_protocol_oteryn::decode_command_result(envelope.payload()).map_err(debug)?;
    assert_eq!(result.command_id, command_id);
    assert_eq!(
        result.status,
        oteryn_protocol_oteryn::CommandStatus::DuplicateReplay
    );
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(result.payload).map_err(debug)?,
        expected
    );
    let payload = result.payload.to_vec();
    admitted.continuity.server_sequence = sequence + 2;
    drop(resumed);
    assert!(
        owner
            .recover_pending_soul_war_source_use(actor, session)
            .await
    );
    commands
        .recover_trade_continuity(admitted)
        .ok_or("idempotent source recovery")?;
    assert_eq!(admitted.continuity.next_command_id, command_id + 1);
    assert_eq!(admitted.continuity.server_sequence, sequence + 2);
    assert!(commands.resumable(admitted));
    if lost {
        owner
            .runtime
            .lock()
            .await
            .restore_control(actor, session, 1)
            .map_err(debug)?;
    }
    Ok(payload)
}
