//! The scope barrier belongs to the connection; the owner decides the transfer.
use super::*;
use crate::gameplay_transport::raid_notices::ServerMessageConnection;
use oteryn_protocol_oteryn::house_transfer::{
    COMMAND_TYPE_HOUSE_TRANSFER, STATE_DOMAIN_HOUSE_SCOPE,
};
use oteryn_protocol_oteryn::zones_nologout::{
    CAPABILITY_ZONES_NOLOGOUT_V1, SERVER_MESSAGES_EMPTY_SNAPSHOT, STATE_DOMAIN_SERVER_MESSAGES,
};

pub(super) async fn execute<S: AsyncRead + AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
    permit: Option<InputPermit<'_>>,
) -> Result<(), ServeEnd> {
    let Some((next, payload)) = authority
        .house_transfer(session.admitted, command.command_id, command.payload)
        .await
    else {
        return Err(ServeEnd::Disconnected);
    };
    drop(permit);
    let sequence = session
        .sequence
        .checked_add(1)
        .ok_or(ServeEnd::Disconnected)?;
    let result = encode_command_result(
        session.generation,
        sequence,
        command.command_id,
        if next.is_some() {
            CommandStatus::Accepted
        } else {
            CommandStatus::Rejected
        },
        &payload,
    )
    .map_err(|_| ServeEnd::Disconnected)?;
    session
        .commands
        .terminalize_with(command.command_id, &result, || {})
        .ok_or(ServeEnd::Disconnected)?;
    session.admitted.continuity.server_sequence = sequence;
    session.admitted.continuity.next_command_id = session.commands.next_command_id();
    session.sequence = sequence;
    if let Some(next) = next {
        session.admitted = next;
    }
    write_frame(stream, &result)
        .await
        .map_err(|_| ServeEnd::Disconnected)?;
    match next {
        Some(next) => Err(ServeEnd::Transferred(Box::new(next))),
        None => Ok(()),
    }
}

/// House custody uses the same command ingress and framing, with no Channel
/// snapshot, combat cadence or Channel mutations while the House owns the slot.
pub(super) async fn serve<'a, S: AsyncRead + AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
    stream: &mut S,
    mut admitted: AdmittedSession,
    authority: &A,
    root_authority: &'a A,
    server_messages: &mut Option<ServerMessageConnection<'a, A>>,
    policy: LivenessPolicy,
    integrity: &mut integrity::Integrity,
) -> Result<AdmittedSession, Box<ConnectionEnd>> {
    let run = async {
        let (_, projection) = authority
            .house_scope_ack(admitted)
            .await
            .map_err(|_| ServeEnd::Disconnected)?
            .ok_or(ServeEnd::Disconnected)?;
        let sequence = admitted
            .continuity
            .server_sequence
            .checked_add(1)
            .ok_or(ServeEnd::Disconnected)?;
        let actor = admitted.runtime_actor.ok_or(ServeEnd::Disconnected)?;
        let selected = admitted.continuity.selected_capabilities;
        let mut house_items = if selected
            .contains(oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_VIEW_MOVE_V1)
            && selected
                .contains(oteryn_protocol_oteryn::house_wrap::CAPABILITY_HOUSE_FURNITURE_WRAP_V1)
        {
            let mut view = SessionItemView::resume(admitted.continuity.item_view);
            if selected.contains(oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_EQUIP_DROP_V1) {
                view = view.with_equip_drop();
            }
            if selected
                .contains(oteryn_protocol_oteryn::container_tree::CAPABILITY_CONTAINER_TREE_V1)
            {
                view = view.with_container_tree();
            }
            Some(view)
        } else {
            None
        };
        let mut item_domains = Vec::new();
        if let Some(view) = house_items.as_mut() {
            let inventory =
                item_move::observe_inventory(authority, actor, admitted.game_session_id, view)
                    .await
                    .ok_or(ServeEnd::Disconnected)?;
            item_domains.extend(
                view.snapshot(inventory, None)
                    .map_err(|_| ServeEnd::Disconnected)?,
            );
            if selected
                .contains(oteryn_protocol_oteryn::container_tree::CAPABILITY_CONTAINER_TREE_V1)
            {
                item_domains.push(view.views_snapshot().map_err(|_| ServeEnd::Disconnected)?);
            }
            admitted.continuity.item_view = view.continuity();
        }
        let mut domains = vec![DomainSnapshot {
            domain_id: STATE_DOMAIN_HOUSE_SCOPE,
            revision: 1,
            snapshot_type: 1,
            payload: &projection,
        }];
        if selected.contains(CAPABILITY_ZONES_NOLOGOUT_V1) {
            domains.push(DomainSnapshot {
                domain_id: STATE_DOMAIN_SERVER_MESSAGES,
                revision: admitted.continuity.server_message_revision,
                snapshot_type: 1,
                payload: SERVER_MESSAGES_EMPTY_SNAPSHOT,
            });
        }
        domains.extend(item_domains.iter().map(|domain| DomainSnapshot {
            domain_id: domain.domain_id,
            revision: domain.revision,
            snapshot_type: domain.snapshot_type,
            payload: &domain.payload,
        }));
        let snapshot = encode_single_chunk_snapshot(
            admitted.continuity.connection_generation,
            sequence,
            sequence,
            &domains,
        )
        .map_err(|_| ServeEnd::Disconnected)?;
        for frame in snapshot {
            write_frame(stream, &frame)
                .await
                .map_err(|_| ServeEnd::Disconnected)?;
        }
        admitted.continuity.server_sequence = sequence;
        zones_nologout_transport::delivered(&mut admitted);
        // A retained idle owner survives a House handoff or transport retry.
        // Only a genuinely fresh recovered actor needs its initial clock value.
        if admitted.continuity.gameplay_idle.last_activity().is_none() {
            admitted
                .continuity
                .gameplay_idle
                .activity(authority.gameplay_idle_now());
        }
        authority.observe_gameplay_idle(admitted);
        if server_messages.is_none() {
            *server_messages = Some(
                ServerMessageConnection::join(root_authority, admitted)
                    .ok_or(ServeEnd::Disconnected)?,
            );
        }
        let mut commands = crate::gameplay_transport::session_ingress::SessionCommandStream::open(
            authority.session_commands(),
            &admitted,
        )
        .ok_or(ServeEnd::Disconnected)?;
        let generation = admitted.continuity.connection_generation;
        let mut frames = FrameReader::default();
        let mut liveness = Liveness::new(policy);
        let mut cadence = tokio::time::interval_at(
            tokio::time::Instant::now() + policy.interval,
            policy.interval,
        );
        cadence.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        enum HouseInput {
            Frame(std::io::Result<Vec<u8>>),
            Probe,
            ServerMessage,
        }
        loop {
            let mut sequence = admitted.continuity.server_sequence;
            zones_nologout_transport::drain(stream, authority, &mut admitted, &mut sequence)
                .await
                .ok_or(ServeEnd::Disconnected)?;
            let frame = {
                let mut read = std::pin::pin!(frames.next(stream));
                let mut tick = std::pin::pin!(cadence.tick());
                let mut notices = std::pin::pin!(authority.server_messages_changed(admitted));
                std::future::poll_fn(|cx| {
                    if tick.as_mut().poll(cx).is_ready() {
                        return std::task::Poll::Ready(HouseInput::Probe);
                    }
                    if notices.as_mut().poll(cx).is_ready() {
                        return std::task::Poll::Ready(HouseInput::ServerMessage);
                    }
                    read.as_mut().poll(cx).map(HouseInput::Frame)
                })
                .await
            };
            let frame = match frame {
                HouseInput::Frame(frame) => frame,
                HouseInput::ServerMessage => continue,
                HouseInput::Probe => {
                    let probe = liveness.tick().map_err(|_| ServeEnd::ControlLost)?;
                    let frame = encode_liveness_probe(generation, probe)
                        .map_err(|_| ServeEnd::Disconnected)?;
                    write_frame(stream, &frame)
                        .await
                        .map_err(|_| ServeEnd::Disconnected)?;
                    continue;
                }
            };
            let frame = frame.map_err(|_| ServeEnd::Disconnected)?;
            let envelope = decode_wire_envelope(&frame).map_err(ServeEnd::Closed)?;
            if envelope.connection_generation() != generation {
                return Err(ServeEnd::Closed(
                    FoundationProtocolError::StaleConnectionGeneration,
                ));
            }
            if envelope.message_type() == MessageType::LivenessAck {
                liveness
                    .ack(
                        envelope
                            .liveness_ack(generation)
                            .map_err(ServeEnd::Closed)?
                            .probe_id,
                    )
                    .map_err(ServeEnd::Closed)?;
                continue;
            }
            let command = envelope
                .client_command(generation)
                .map_err(ServeEnd::Closed)?;
            let limited = integrity.admit().await;
            match commands
                .begin(
                    command.command_id,
                    generation,
                    admitted.continuity.server_sequence,
                    false,
                )
                .map_err(|(e, _)| ServeEnd::Closed(e))?
            {
                crate::gameplay_transport::session_ingress::CommandAction::Pending => continue,
                crate::gameplay_transport::session_ingress::CommandAction::Replay(frame) => {
                    admitted.continuity.server_sequence += 1;
                    write_frame(stream, &frame)
                        .await
                        .map_err(|_| ServeEnd::Disconnected)?;
                    continue;
                }
                crate::gameplay_transport::session_ingress::CommandAction::Execute => {}
            }
            let permit = if limited {
                None
            } else {
                authority.enter_input(admitted.game_session_id).await
            };
            if permit.is_some() && !authority.house_transfer_input(admitted).await {
                return Err(ServeEnd::Disconnected);
            }
            let activity_at = authority.gameplay_idle_now();
            let gameplay_activity = permit.is_some()
                && selected.command_selected(command.command_type)
                && matches!(
                    command.command_type,
                    COMMAND_TYPE_HOUSE_TRANSFER | oteryn_protocol_oteryn::house_wrap::COMMAND_WRAP
                )
                && super::super::gameplay_idle::meaningful(command, selected.as_slice());
            let (next, payload, status) = if permit.is_some()
                && command.command_type == COMMAND_TYPE_HOUSE_TRANSFER
                && admitted
                    .continuity
                    .selected_capabilities
                    .command_selected(command.command_type)
            {
                let (next, payload) = authority
                    .house_transfer(admitted, command.command_id, command.payload)
                    .await
                    .ok_or(ServeEnd::Disconnected)?;
                let status = if next.is_some() {
                    CommandStatus::Accepted
                } else {
                    CommandStatus::Rejected
                };
                (next, payload, status)
            } else if permit.is_some()
                && command.command_type == oteryn_protocol_oteryn::house_wrap::COMMAND_WRAP
                && selected.command_selected(command.command_type)
            {
                match routing::house_wrap::route(authority, &admitted, command).await {
                    Some(Dispatch::Reply(status, payload)) => (None, payload, status),
                    _ => return Err(ServeEnd::Disconnected),
                }
            } else {
                (None, Vec::new(), CommandStatus::Rejected)
            };
            drop(permit);
            let sequence = admitted
                .continuity
                .server_sequence
                .checked_add(1)
                .ok_or(ServeEnd::Disconnected)?;
            let result =
                encode_command_result(generation, sequence, command.command_id, status, &payload)
                    .map_err(|_| ServeEnd::Disconnected)?;
            commands
                .terminalize_with(command.command_id, &result, || {})
                .ok_or(ServeEnd::Disconnected)?;
            admitted.continuity.server_sequence = sequence;
            admitted.continuity.next_command_id = commands.next_command_id();
            if let Some(next) = next {
                admitted = next;
            }
            if gameplay_activity {
                admitted.continuity.gameplay_idle.activity(activity_at);
            }
            authority.observe_gameplay_idle(admitted);
            write_frame(stream, &result)
                .await
                .map_err(|_| ServeEnd::Disconnected)?;
            if command.command_type == oteryn_protocol_oteryn::house_wrap::COMMAND_WRAP
                && matches!(
                    oteryn_protocol_oteryn::house_wrap::Reply::decode(&payload),
                    Some(oteryn_protocol_oteryn::house_wrap::Reply::Committed(_))
                )
                && let Some(view) = house_items.as_mut()
            {
                let mut sequence = admitted.continuity.server_sequence;
                item_use_view_refresh::refresh(
                    stream,
                    authority,
                    actor,
                    &mut admitted,
                    view,
                    None,
                    &mut sequence,
                )
                .await
                .ok_or(ServeEnd::Disconnected)?;
            }
            if next.is_some() {
                return Ok(admitted);
            }
        }
    }
    .await;
    run.map_err(|end| Box::new(end.finish(admitted)))
}

#[cfg(test)]
#[path = "house_transfer_raid_tests.rs"]
mod raid_tests;
