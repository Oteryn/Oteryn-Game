//! The scope barrier belongs to the connection; the owner decides the transfer.
use super::*;
use crate::gameplay_transport::house_podium::Approach;
use crate::gameplay_transport::raid_notices::ServerMessageConnection;
use oteryn_protocol_oteryn::house_podium::{COMMAND_PROJECTION, COMMAND_QUERY, decode_item};
use oteryn_protocol_oteryn::house_transfer::{
    COMMAND_TYPE_HOUSE_TRANSFER, HouseTransferRefusal, STATE_DOMAIN_HOUSE_SCOPE,
};
use oteryn_protocol_oteryn::zones_nologout::{
    CAPABILITY_ZONES_NOLOGOUT_V1, SERVER_MESSAGES_EMPTY_SNAPSHOT, STATE_DOMAIN_SERVER_MESSAGES,
};
use std::collections::VecDeque;
use std::time::Duration;
use tokio::time::Instant;

/// Fill rejections produced before the transfer owner runs, or by an authority
/// that still returns the original empty refusal.
pub(super) fn refusal_payload(
    command_type: u32,
    selected: bool,
    status: CommandStatus,
    payload: &[u8],
) -> Option<[u8; 2]> {
    (command_type == COMMAND_TYPE_HOUSE_TRANSFER
        && status == CommandStatus::Rejected
        && payload.is_empty())
    .then(|| {
        if selected {
            HouseTransferRefusal::Unavailable
        } else {
            HouseTransferRefusal::CapabilityRequired
        }
        .encode()
    })
}

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
    let status = if next.is_some() {
        CommandStatus::Accepted
    } else {
        CommandStatus::Rejected
    };
    let fallback = refusal_payload(command.command_type, true, status, &payload);
    let payload = fallback
        .as_ref()
        .map_or(payload.as_slice(), |bytes| bytes.as_slice());
    let result = encode_command_result(
        session.generation,
        sequence,
        command.command_id,
        status,
        payload,
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
    if next.is_some()
        && session
            .selected_capabilities
            .contains(oteryn_protocol_oteryn::presentation::CAPABILITY_AMBIENT_SOUND_V1)
    {
        // A House has no Channel cadence: leaving the sound zone must stop its loop now.
        refresh::ambient_sound(stream, session, 0).await?;
    }
    match next {
        Some(_) => Err(ServeEnd::Transferred(Box::new(session.admitted))),
        None => Ok(()),
    }
}

/// House custody uses the same command ingress and framing, with no Channel
/// snapshot, combat cadence or Channel mutations while the House owns the slot.
#[allow(
    clippy::too_many_arguments,
    reason = "the same transport carries its activity session fence into House ingress"
)]
pub(super) async fn serve<'a, S: AsyncRead + AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
    stream: &mut S,
    mut admitted: AdmittedSession,
    authority: &A,
    root_authority: &'a A,
    server_messages: &mut Option<ServerMessageConnection<'a, A>>,
    policy: LivenessPolicy,
    integrity: &mut integrity::Integrity,
    require_session_binding: bool,
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
        let (mut services, mut item_domains) =
            house_hireling::State::prepare(authority, actor, &mut admitted).await?;
        super::cooldowns::snapshot(
            authority,
            &admitted,
            services.cooldowns.as_mut(),
            &mut item_domains,
        )?;
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
        services.appearance_domain(&mut domains);
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
        let mut queue = house_hireling::step::Queue::default();
        let mut podium = None::<PodiumWalk>;
        let mut liveness = Liveness::new(policy);
        let mut cadence = tokio::time::interval_at(
            tokio::time::Instant::now() + policy.interval,
            policy.interval,
        );
        cadence.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut service_cadence = tokio::time::interval_at(
            tokio::time::Instant::now() + std::time::Duration::from_secs(1),
            std::time::Duration::from_secs(1),
        );
        service_cadence.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut next_staff_refresh = tokio::time::Instant::now() + VISIBILITY_REFRESH;
        loop {
            let event = {
                let mut sequence = admitted.continuity.server_sequence;
                zones_nologout_transport::drain(stream, authority, &mut admitted, &mut sequence)
                    .await
                    .ok_or(ServeEnd::Disconnected)?;
                let mut input = std::pin::pin!(queue.next(
                    stream,
                    generation,
                    &mut cadence,
                    &mut service_cadence,
                ));
                let mut notices = std::pin::pin!(authority.server_messages_changed(admitted));
                match std::future::poll_fn(|cx| {
                    if let std::task::Poll::Ready(event) = input.as_mut().poll(cx) {
                        return std::task::Poll::Ready(Some(event));
                    }
                    notices.as_mut().poll(cx).map(|()| None)
                })
                .await
                {
                    Some(event) => event,
                    None => continue,
                }
            };
            let event = match event {
                Ok(event) => event,
                Err(end) => {
                    if let Some(walk) = podium.take() {
                        // The pending query never reached its owner, so its reservation
                        // can retain a cancellation even if the client socket is gone.
                        let _ = walk
                            .finish(stream, &mut admitted, &mut commands, refused())
                            .await;
                    }
                    return Err(end);
                }
            };
            // Refresh before ingress/probes, not immediately after a reply: stash and
            // receipt readers need the same holder while we wait for the next command.
            if !matches!(&event, house_hireling::step::Event::Services)
                && tokio::time::Instant::now() >= next_staff_refresh
            {
                if !authority.staff_refresh(admitted).await {
                    return Err(ServeEnd::Disconnected);
                }
                next_staff_refresh = tokio::time::Instant::now() + VISIBILITY_REFRESH;
            }
            let inbound::Input {
                frame,
                arrived_while_buffered,
                buffered_original,
                ..
            } = match event {
                house_hireling::step::Event::Services => {
                    services.tick(stream, authority, &mut admitted).await?;
                    house_hireling::step::services_completed(&mut service_cadence);
                    continue;
                }
                house_hireling::step::Event::Probe => {
                    if tokio::time::timeout(
                        crate::durability::DB_PASS_DEADLINE,
                        authority.account_closure_check(admitted),
                    )
                    .await
                    .map_err(|_| ServeEnd::Disconnected)?
                    {
                        return Err(ServeEnd::Disconnected);
                    }
                    let probe = liveness
                        .tick()
                        .map_err(|_| ServeEnd::ControlLost("house_transfer"))?;
                    let frame = encode_liveness_probe(generation, probe)
                        .map_err(|_| ServeEnd::Disconnected)?;
                    write_frame(stream, &frame)
                        .await
                        .map_err(|_| ServeEnd::Disconnected)?;
                    continue;
                }
                house_hireling::step::Event::PodiumWalk => {
                    let mut walk = podium.take().ok_or(ServeEnd::Disconnected)?;
                    match walk
                        .advance(stream, authority, &mut services, &mut admitted)
                        .await?
                    {
                        PodiumTurn::Wait(due) => {
                            queue.set_podium_due(Some(due));
                            podium = Some(walk);
                        }
                        PodiumTurn::Finish(dispatch) => {
                            integrity.observe(&dispatch);
                            walk.finish(stream, &mut admitted, &mut commands, dispatch)
                                .await?;
                        }
                    }
                    continue;
                }
                house_hireling::step::Event::Input(input) => input,
            };
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
            if !command_session_matches(&frame, admitted, require_session_binding)
                .map_err(ServeEnd::Closed)?
            {
                continue;
            }
            let command = envelope
                .client_command(generation)
                .map_err(ServeEnd::Closed)?;
            // Ingress permits one outstanding command. As with Channel item
            // walks, a new action (including a manual STEP) first cancels the
            // reserved automatic action. Retransmissions stay attached to it.
            if let Some(walk) = podium.take() {
                if walk.command_id.checked_add(1) == Some(command.command_id) {
                    queue.set_podium_due(None);
                    walk.finish(stream, &mut admitted, &mut commands, refused())
                        .await?;
                } else {
                    podium = Some(walk);
                }
            }
            let limited = if buffered_original {
                false
            } else {
                integrity.admit().await
            };
            match commands
                .begin(
                    command.command_id,
                    generation,
                    admitted.continuity.server_sequence,
                    buffered_original,
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
            let mut permit = if limited {
                None
            } else {
                authority.enter_input(admitted.game_session_id).await
            };
            let guard_deferred = permit.is_some()
                && !super::command::guards(authority, &mut admitted, command).await?;
            if guard_deferred {
                drop(permit.take());
                if command.command_type == COMMAND_PROJECTION
                    && selected.command_selected(command.command_type)
                {
                    let _ = commands.cancel_undispatched(
                        command.command_id,
                        generation,
                        admitted.continuity.server_sequence,
                        &[],
                    );
                    return Err(ServeEnd::Disconnected);
                }
            }
            let podium_query = command.command_type == COMMAND_QUERY
                && selected.command_selected(command.command_type);
            let activity_at = authority.gameplay_idle_now();
            let gameplay_activity = permit.is_some()
                && selected.command_selected(command.command_type)
                && matches!(
                    command.command_type,
                    COMMAND_TYPE_HOUSE_TRANSFER
                        | oteryn_protocol_oteryn::house_wrap::COMMAND_WRAP
                        | oteryn_protocol_oteryn::world_object::COMMAND_TYPE_USE_INTENT
                        | oteryn_protocol_oteryn::item_view::COMMAND_TYPE_ITEM_MOVE_INTENT
                )
                && super::super::gameplay_idle::meaningful(command, selected.as_slice());
            let mut effects = None;
            let mut second_logout = false;
            let mut lost_while_logging_out = false;
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
            } else if permit.is_some() || (guard_deferred && podium_query) {
                let dispatch = if command.command_type == COMMAND_TYPE_LOGOUT_INTENT
                    && selected.command_selected(command.command_type)
                {
                    if decode_logout_intent(command.payload).is_err() {
                        Dispatch::Unregistered
                    } else {
                        let outcome = queue.logout(stream, authority, admitted).await?;
                        second_logout = outcome.second;
                        lost_while_logging_out = outcome.lost;
                        outcome.dispatch
                    }
                } else if command.command_type == COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT
                    && selected.command_selected(command.command_type)
                {
                    match services
                        .step(authority, &admitted, command, arrived_while_buffered)
                        .await
                    {
                        route::StepAction::Wait(due) => {
                            commands
                                .buffer_step(command)
                                .ok_or(ServeEnd::Disconnected)?;
                            integrity.observe(&Dispatch::Step(StepOutcome::too_early(), 0));
                            // The reserved original runs once when due. Release the input
                            // permit while the bounded queue keeps later frames in order.
                            queue.buffer(frame, due);
                            continue;
                        }
                        route::StepAction::Dispatch(dispatch) => dispatch,
                    }
                } else if podium_query {
                    let Some(item) = decode_item(command.payload) else {
                        drop(permit.take());
                        return Err(close(
                            stream,
                            admitted,
                            FoundationProtocolError::MalformedEnvelope,
                        )
                        .await);
                    };
                    let approach = if guard_deferred {
                        Approach::Unavailable
                    } else {
                        authority.house_podium_approach(admitted, item).await
                    };
                    match approach {
                        Approach::Ready => services
                            .route(authority, &mut admitted, command)
                            .await
                            .ok_or(ServeEnd::Disconnected)?,
                        Approach::Walk(steps) => {
                            podium = Some(PodiumWalk {
                                command_id: command.command_id,
                                item,
                                steps: steps.into(),
                            });
                            queue.set_podium_due(Some(services.step_due()));
                            continue;
                        }
                        Approach::NoPath => refused(),
                        Approach::Unavailable => {
                            let _ = commands.cancel_undispatched(
                                command.command_id,
                                generation,
                                admitted.continuity.server_sequence,
                                &[],
                            );
                            return Err(ServeEnd::Disconnected);
                        }
                    }
                } else {
                    services
                        .route(authority, &mut admitted, command)
                        .await
                        .ok_or(ServeEnd::Disconnected)?
                };
                integrity.observe(&dispatch);
                let (status, payload) = house_hireling::result(selected, &dispatch)?;
                effects = Some(dispatch);
                (None, payload, status)
            } else if command.command_type == COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT {
                let (status, payload) =
                    house_hireling::result(selected, &Dispatch::Step(StepOutcome::rejected(), 0))?;
                (None, payload, status)
            } else {
                (None, Vec::new(), CommandStatus::Rejected)
            };
            drop(permit);
            let cooldown_frames = if next.is_none() {
                super::cooldowns::frames(
                    authority,
                    services.cooldowns.as_ref(),
                    &mut admitted.continuity,
                    None,
                )?
            } else {
                Vec::new()
            };
            let sequence = admitted
                .continuity
                .server_sequence
                .checked_add(1)
                .ok_or(ServeEnd::Disconnected)?;
            let fallback = refusal_payload(
                command.command_type,
                selected.command_selected(command.command_type),
                status,
                &payload,
            );
            let reply = fallback
                .as_ref()
                .map_or(payload.as_slice(), |bytes| bytes.as_slice());
            let result =
                encode_command_result(generation, sequence, command.command_id, status, reply)
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
            if matches!(
                effects,
                Some(
                    Dispatch::Logout(LogoutResult::Accepted)
                        | Dispatch::ServerModal(
                            crate::gameplay_transport::server_modal::ServerModalOutcome::LoggedOut(
                                _
                            )
                        )
                )
            ) {
                // The native release already committed. A failed final write
                // remains a logout, never a second control-loss transition.
                if super::cooldowns::write_batch(stream, &cooldown_frames, Some(&result))
                    .await
                    .is_ok()
                    && (!second_logout
                        || send_error(
                            stream,
                            FoundationProtocolError::TooManyOutstandingCommands,
                            generation,
                        )
                        .await
                        .is_ok())
                {
                    let _ = tokio::time::timeout(
                        policy.interval,
                        tokio::io::AsyncWriteExt::shutdown(stream),
                    )
                    .await;
                }
                return Err(ServeEnd::LoggedOut);
            }
            super::cooldowns::write_batch(stream, &cooldown_frames, Some(&result)).await?;
            if second_logout {
                return Err(close(
                    stream,
                    admitted,
                    FoundationProtocolError::TooManyOutstandingCommands,
                )
                .await);
            }
            if lost_while_logging_out {
                return Err(ServeEnd::Disconnected);
            }
            if routing::store::is_committed_claim(command.command_type, status, &payload) {
                services.invalidate_capacity();
                services
                    .refresh_inventory(stream, authority, &mut admitted)
                    .await?;
            } else if command.command_type == oteryn_protocol_oteryn::house_wrap::COMMAND_WRAP
                && matches!(
                    oteryn_protocol_oteryn::house_wrap::Reply::decode(&payload),
                    Some(oteryn_protocol_oteryn::house_wrap::Reply::Committed(_))
                )
            {
                services.invalidate_capacity();
                services.inventory(stream, authority, &mut admitted).await?;
            } else if command.command_type
                == u32::from(oteryn_protocol_oteryn::mail::COMMAND_TYPE_MAIL)
                && payload == [oteryn_protocol_oteryn::mail::MailOutcome::TextWritten as u8]
            {
                services.inventory(stream, authority, &mut admitted).await?;
            }
            // House Store replies use the claim-specific inventory refresh above.
            if let Some(effects) =
                effects.filter(|_| !routing::store::handles_command(command.command_type))
            {
                services
                    .effects(stream, authority, &mut admitted, effects)
                    .await?;
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
#[path = "house_transfer_authority_tests.rs"]
mod authority_tests;

#[cfg(test)]
#[path = "house_transfer_raid_tests.rs"]
mod raid_tests;

// Crystal game.cpp:4609-4625 and player.cpp:6190-6192 retry after walking finishes.
const PODIUM_RETRY_DELAY: Duration = Duration::from_millis(400);

struct PodiumWalk {
    command_id: u64,
    item: [u8; 16],
    steps: VecDeque<StepDirection>,
}

enum PodiumTurn {
    Wait(Instant),
    Finish(Dispatch),
}

fn refused() -> Dispatch {
    Dispatch::Reply(CommandStatus::Rejected, Vec::new())
}

impl PodiumWalk {
    async fn advance<S: AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
        &mut self,
        stream: &mut S,
        authority: &A,
        services: &mut house_hireling::State,
        admitted: &mut AdmittedSession,
    ) -> Result<PodiumTurn, ServeEnd> {
        let Some(permit) = authority.enter_input(admitted.game_session_id).await else {
            return Ok(PodiumTurn::Finish(refused()));
        };
        let Some(&direction) = self.steps.front() else {
            let dispatch = match authority.house_podium_approach(*admitted, self.item).await {
                Approach::Ready => {
                    let command = oteryn_protocol_oteryn::ClientCommandView {
                        command_id: self.command_id,
                        command_type: COMMAND_QUERY,
                        payload: &self.item,
                    };
                    services
                        .route(authority, admitted, command)
                        .await
                        .ok_or(ServeEnd::Disconnected)?
                }
                Approach::Walk(_) | Approach::NoPath => refused(),
                Approach::Unavailable => return Err(ServeEnd::Disconnected),
            };
            return Ok(PodiumTurn::Finish(dispatch));
        };
        let payload = oteryn_protocol_oteryn::world_spatial::encode_step_intent(direction);
        let command = oteryn_protocol_oteryn::ClientCommandView {
            command_id: self.command_id,
            command_type: COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            payload: &payload,
        };
        let outcome = services.step(authority, admitted, command, false).await;
        drop(permit);
        match outcome {
            route::StepAction::Wait(due) => Ok(PodiumTurn::Wait(due)),
            route::StepAction::Dispatch(
                dispatch @ Dispatch::Step(
                    StepOutcome {
                        disposition: StepDisposition::Moved,
                        ..
                    },
                    _,
                ),
            ) => {
                self.steps.pop_front();
                services
                    .effects(stream, authority, admitted, dispatch)
                    .await?;
                let due = services.step_due();
                Ok(PodiumTurn::Wait(if self.steps.is_empty() {
                    due + PODIUM_RETRY_DELAY
                } else {
                    due
                }))
            }
            route::StepAction::Dispatch(_) => Ok(PodiumTurn::Finish(refused())),
        }
    }

    async fn finish(
        self,
        stream: &mut (impl AsyncWrite + Unpin),
        admitted: &mut AdmittedSession,
        commands: &mut crate::gameplay_transport::session_ingress::SessionCommandStream,
        dispatch: Dispatch,
    ) -> Result<(), ServeEnd> {
        let (status, payload) =
            house_hireling::result(admitted.continuity.selected_capabilities, &dispatch)?;
        let sequence = admitted
            .continuity
            .server_sequence
            .checked_add(1)
            .ok_or(ServeEnd::Disconnected)?;
        let frame = encode_command_result(
            admitted.continuity.connection_generation,
            sequence,
            self.command_id,
            status,
            &payload,
        )
        .map_err(|_| ServeEnd::Disconnected)?;
        commands
            .terminalize_with(self.command_id, &frame, || {})
            .ok_or(ServeEnd::Disconnected)?;
        admitted.continuity.server_sequence = sequence;
        admitted.continuity.next_command_id = commands.next_command_id();
        write_frame(stream, &frame)
            .await
            .map_err(|_| ServeEnd::Disconnected)
    }
}
