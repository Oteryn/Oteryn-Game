//! One pending item command drives a bounded queue on the existing actor pacing clock.
use super::*;
use crate::gameplay_transport::AutoWalkTarget;
use oteryn_protocol_oteryn::world_object::COMMAND_TYPE_USE_INTENT;
use std::time::Duration;

pub(super) enum WalkOutcome {
    Arrived,
    Cancelled,
    NoPath,
}

pub(super) fn too_far(dispatch: &Dispatch) -> bool {
    matches!(
        dispatch,
        Dispatch::Use(UseOutcome {
            disposition: UseDisposition::TooFar,
            ..
        }) | Dispatch::UseItem(OpenDecision::TooFar, _)
            | Dispatch::ItemMove(ItemMoveOutcome::TooFar, _)
    )
}

pub(super) fn target(
    session: &ServeState,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
) -> Option<AutoWalkTarget> {
    let items = session.item_view.as_ref();
    if command.command_type == COMMAND_TYPE_USE_INTENT {
        use oteryn_protocol_oteryn::world_object::MapUseTarget;
        let intent =
            decode_use_intent_target(session.selected_capabilities.as_slice(), command.payload)
                .ok()?;
        if let Some(target) = intent.use_with_map_item {
            return match target {
                MapUseTarget::Item(handle) => Some(AutoWalkTarget::Item(items?.resolve(handle)?)),
                MapUseTarget::Object(target) => Some(AutoWalkTarget::Object(target)),
            };
        }
        return match intent.target {
            UseTarget::WorldObject(target) => Some(AutoWalkTarget::Object(target)),
            UseTarget::Item(handle) => Some(AutoWalkTarget::Item(items?.resolve(handle)?)),
            UseTarget::ItemByDefinition(_) => None,
        };
    }
    if command.command_type == oteryn_protocol_oteryn::item_view::COMMAND_TYPE_ITEM_MOVE_INTENT {
        let selection = ItemMoveSelection {
            equip_drop: session
                .selected_capabilities
                .contains(CAPABILITY_ITEM_EQUIP_DROP_V1),
            container_tree: session
                .selected_capabilities
                .contains(CAPABILITY_CONTAINER_TREE_V1),
            world_interaction: session
                .selected_capabilities
                .contains(CAPABILITY_WORLD_INTERACTION_V1),
        };
        let intent = if session
            .selected_capabilities
            .contains(oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_STACK_COUNT_V1)
        {
            oteryn_protocol_oteryn::item_view::decode_item_move_intent_with_count_for(
                command.payload,
                selection,
            )
            .ok()?
            .0
        } else {
            oteryn_protocol_oteryn::item_view::decode_item_move_intent_for(
                command.payload,
                selection,
            )
            .ok()?
        };
        let items = items?;
        let source = items.resolve(intent.source)?;
        return Some(AutoWalkTarget::Item(
            items.open_corpse_of(&source).unwrap_or(source),
        ));
    }
    None
}

pub(super) async fn walk<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
    command_id: u64,
    target: AutoWalkTarget,
) -> Result<WalkOutcome, ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    let Some(input) = authority.enter_input().await else {
        return Ok(WalkOutcome::Cancelled);
    };
    let route = tokio::time::timeout(
        Duration::from_secs(10),
        authority.item_walk_route(session.actor, session.admitted.game_session_id, target),
    )
    .await
    .map_err(|_| ServeEnd::Disconnected)?;
    drop(input);
    let mut queue = match route {
        Ok(Some(queue)) => queue,
        Ok(None) => return Ok(WalkOutcome::Cancelled),
        Err(()) => return Ok(WalkOutcome::NoPath),
    };
    session.chase_due = None;
    session.chase_yielded = false;
    authority
        .yield_chase_to_client(session.actor, session.admitted.game_session_id)
        .await;
    while let Some(direction) = queue.steps.pop_front() {
        // A ready client action wins even if this step is already due. Liveness and owner
        // refreshes remain live while waiting; no second command reserves an ingress slot.
        if !wait(stream, authority, session, command_id).await? {
            return Ok(WalkOutcome::Cancelled);
        }
        loop {
            if begin_actor_step(
                &mut session.pacer,
                authority,
                session.actor,
                &session.step_holder,
            )
            .await
            .is_some()
            {
                if !wait(stream, authority, session, command_id).await? {
                    return Ok(WalkOutcome::Cancelled);
                }
            } else {
                break;
            }
        }
        let Some(step_input) = authority.enter_input().await else {
            record_actor_step(
                &mut session.pacer,
                authority,
                session.actor,
                &session.step_holder,
                None,
            )
            .await;
            return Ok(WalkOutcome::Cancelled);
        };
        let (outcome, duration, current) = tokio::time::timeout(
            Duration::from_secs(10),
            authority.item_walk_step(
                session.actor,
                session.admitted.game_session_id,
                queue.expected,
                direction,
                None,
            ),
        )
        .await
        .map_err(|_| ServeEnd::Disconnected)?;
        record_actor_step(
            &mut session.pacer,
            authority,
            session.actor,
            &session.step_holder,
            duration,
        )
        .await;
        drop(step_input);
        let moved = outcome.disposition == StepDisposition::Moved;
        effects::step(stream, authority, session, outcome).await?;
        let Some(current) = current.filter(|_| moved) else {
            return Ok(if outcome.disposition == StepDisposition::Blocked {
                WalkOutcome::NoPath
            } else {
                WalkOutcome::Cancelled
            });
        };
        queue.expected = current;
    }
    Ok(
        if wait(stream, authority, session, command_id).await?
            && authority
                .item_walk_valid(
                    session.actor,
                    session.admitted.game_session_id,
                    queue.expected,
                )
                .await
        {
            WalkOutcome::Arrived
        } else {
            WalkOutcome::Cancelled
        },
    )
}

async fn wait<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
    command_id: u64,
) -> Result<bool, ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    let due = session
        .pacer
        .wait_until(tokio::time::Instant::now())
        .unwrap_or_else(tokio::time::Instant::now);
    let mut yielded_to_client = false;
    loop {
        let event = {
            let mut next = std::pin::pin!(poll::next(stream, session));
            let mut timer = std::pin::pin!(tokio::time::sleep_until(due));
            std::future::poll_fn(|context| {
                // Like chase pacing, give ready input one turn ahead of a due step.
                // Retransmissions or ACK traffic cannot postpone that step indefinitely.
                if yielded_to_client && timer.as_mut().poll(context).is_ready() {
                    return std::task::Poll::Ready(None);
                }
                if let std::task::Poll::Ready(event) = next.as_mut().poll(context) {
                    return std::task::Poll::Ready(Some(event));
                }
                timer.as_mut().poll(context).map(|()| None)
            })
            .await
        };
        let Some(event) = event else {
            return Ok(true);
        };
        if let poll::Next::Frame(Err(_)) = event {
            return Ok(false);
        }
        if let Some(input) = inbound::receive(stream, authority, session, event).await? {
            // Do not consume another player action on behalf of this pending item command.
            // The normal command decoder handles stop, logout and malformed/stale frames too.
            if let Ok(envelope) = decode_wire_envelope(&input.frame)
                && envelope.connection_generation() == session.generation
                && envelope.message_type() == MessageType::LivenessAck
            {
                let ack = envelope
                    .liveness_ack(session.generation)
                    .and_then(|ack| session.liveness.ack(ack.probe_id));
                if let Err(error) = ack {
                    return Err(close(stream, session.admitted, error).await);
                }
                yielded_to_client = true;
                continue;
            }
            // FND-02: a retransmission remains the same pending operation, even when its
            // replacement bytes differ. It neither cancels nor queues a second item action.
            if let Ok(envelope) = decode_wire_envelope(&input.frame)
                && envelope.connection_generation() == session.generation
                && let Ok(command) = envelope.client_command(session.generation)
                && command.command_id == command_id
            {
                yielded_to_client = true;
                continue;
            }
            session
                .held
                .push_front((input.frame, input.arrived_while_buffered));
            return Ok(false);
        }
        // A combat tick may arm chase; item walks retain the single Movement clock.
        session.chase_due = None;
    }
}
