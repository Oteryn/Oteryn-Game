//! Crystal 201189f game.cpp:3718-3732 / player.cpp:3038-3054 replaces the
//! pending ordinary action. Keep CommandIds ordered: a superseded press receives
//! the existing rejection result before its successor enters command ingress.
use super::*;
use oteryn_protocol_oteryn::hotkey_equip::{
    CAPABILITY_HOTKEY_EQUIP_UPGRADE_TIER_V1, COMMAND_TYPE_HOTKEY_EQUIP, TieredRequest,
};

pub(super) async fn replaced<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
    queued_for: std::time::Duration,
) -> Result<bool, ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    if session.reconcile_original_bank
        || command.command_type != COMMAND_TYPE_HOTKEY_EQUIP
        || !session
            .selected_capabilities
            .command_selected(command.command_type)
    {
        return Ok(false);
    }
    let tiered = session
        .selected_capabilities
        .contains(CAPABILITY_HOTKEY_EQUIP_UPGRADE_TIER_V1);
    let Some(TieredRequest { request, .. }) =
        TieredRequest::decode_selected(tiered, command.payload)
    else {
        return Ok(false);
    };
    let Some(delay) = authority
        .hotkey_equip_wait(
            session.actor,
            use_command(session, command.command_id),
            request,
            queued_for,
        )
        .await
    else {
        return Ok(false);
    };
    if delay.is_zero() {
        return Ok(false);
    }
    let due = tokio::time::Instant::now() + delay.saturating_sub(queued_for);
    // A recovered command may already have committed. Never replace durable truth.
    if !matches!(
        authority
            .committed_equipment_move(session.actor, use_command(session, command.command_id))
            .await,
        Some(crate::gameplay_transport::item_move::EquipmentMoveReplay::Absent)
    ) {
        return Ok(false);
    }
    // Ammunition hotkeys commit through the existing container move owner.
    // Its receipt is equally final and must survive pending-action replacement.
    if !matches!(
        authority
            .committed_loop_move(session.actor, use_command(session, command.command_id))
            .await,
        Some(crate::gameplay_transport::item_move::LoopMoveReplay::Absent)
    ) {
        return Ok(false);
    }
    loop {
        if let Some((frame, _, _)) = session.held.front() {
            let replaced = tokio::time::Instant::now() < due
                && is_replacement(authority, session, command.command_id, frame).await;
            if !replaced {
                tokio::time::sleep_until(due).await;
            }
            return Ok(replaced);
        }
        let Ok(frame) = tokio::time::timeout_at(due, session.frames.next(stream)).await else {
            return Ok(false);
        };
        let Ok(frame) = frame else {
            // Preserve the existing execution-before-EOF behavior of a queued command.
            tokio::time::sleep_until(due).await;
            return Ok(false);
        };
        if let Ok(envelope) = decode_wire_envelope(&frame)
            && envelope.connection_generation() == session.generation
            && envelope.message_type() == MessageType::LivenessAck
        {
            if let Err(error) = envelope
                .liveness_ack(session.generation)
                .and_then(|ack| session.liveness.ack(ack.probe_id))
            {
                return Err(close(stream, session.admitted, error).await);
            }
            continue;
        }
        let received_at = tokio::time::Instant::now();
        let replaced = received_at < due
            && is_replacement(authority, session, command.command_id, &frame).await;
        // One lookahead, using the existing bounded held-frame queue. Other commands,
        // invalid frames and jewelry retain their order and cannot replace this action.
        session.held.push_back((frame, false, received_at));
        if !replaced {
            tokio::time::sleep_until(due).await;
        }
        return Ok(replaced);
    }
}

fn use_command(session: &ServeState, command_id: u64) -> UseCommand {
    UseCommand {
        game_session_id: session.admitted.game_session_id,
        command_id,
        item_fence: session.admitted.item_fence,
    }
}

async fn is_replacement<A: FreshAdmissionAuthority>(
    authority: &A,
    session: &ServeState,
    current: u64,
    frame: &[u8],
) -> bool {
    let Ok(envelope) = decode_wire_envelope(frame) else {
        return false;
    };
    let Ok(command) = envelope.client_command(session.generation) else {
        return false;
    };
    if Some(command.command_id) != current.checked_add(1)
        || command.command_type != COMMAND_TYPE_HOTKEY_EQUIP
    {
        return false;
    }
    let tiered = session
        .selected_capabilities
        .contains(CAPABILITY_HOTKEY_EQUIP_UPGRADE_TIER_V1);
    let Some(TieredRequest { request, .. }) =
        TieredRequest::decode_selected(tiered, command.payload)
    else {
        return false;
    };
    authority
        .hotkey_equip_wait(
            session.actor,
            use_command(session, command.command_id),
            request,
            std::time::Duration::ZERO,
        )
        .await
        .is_some()
}
