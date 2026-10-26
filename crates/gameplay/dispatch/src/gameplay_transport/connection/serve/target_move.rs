//! Explicit walks and FOLLOW share the existing actor step clock and ordinary step owner.
use super::*;
use crate::gameplay_transport::{AutoWalkRoute, AutoWalkTarget};
use oteryn_protocol_oteryn::target_move::{TargetMoveIntent, decode_target_move_intent};
use std::time::Duration;

pub(super) struct PendingWalk {
    target: AutoWalkTarget,
    route: AutoWalkRoute,
    pub(super) due: tokio::time::Instant,
}

impl PendingWalk {
    fn schedule(&mut self, due: tokio::time::Instant) {
        // Target loss must be noticed while a slow step clock is waiting too.
        self.due = if matches!(self.target, AutoWalkTarget::Follow(_)) {
            due.min(tokio::time::Instant::now() + Duration::from_millis(100))
        } else {
            due
        };
    }
}

fn reply(disposition: u8) -> Dispatch {
    Dispatch::Reply(
        if disposition == 3 {
            CommandStatus::Rejected
        } else {
            CommandStatus::Accepted
        },
        vec![8, disposition],
    )
}

pub(super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    session: &mut ServeState,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
) -> Result<Dispatch, ServeEnd> {
    if !session
        .target_move_window
        .admit(tokio::time::Instant::now())
    {
        return Ok(reply(3));
    }
    let Ok(intent) = decode_target_move_intent(command.payload) else {
        return Ok(reply(3));
    };
    session.target_walk = None;
    session.chase_due = None;
    session.chase_yielded = false;
    let actor = session.actor;
    let id = session.admitted.game_session_id;
    let reply = tokio::time::timeout(Duration::from_secs(10), async {
        authority.yield_chase_to_client(actor, id).await;
        match intent {
            TargetMoveIntent::Turn(direction) => {
                reply(if authority.turn_actor(actor, id, direction).await {
                    1
                } else {
                    3
                })
            }
            TargetMoveIntent::Stop => {
                let valid = authority.turn_actor_valid(actor, id).await;
                reply(if valid { 1 } else { 3 })
            }
            intent => {
                let target = match intent {
                    TargetMoveIntent::Walk(cell) => AutoWalkTarget::Cell(cell),
                    TargetMoveIntent::Follow(target) => AutoWalkTarget::Follow(target),
                    TargetMoveIntent::Stop | TargetMoveIntent::Turn(_) => unreachable!(),
                };
                match authority.item_walk_route(actor, id, target.clone()).await {
                    Ok(Some(route)) => {
                        // Canary FOLLOW clears the attack target; walking uses one Movement clock.
                        let _ = authority
                            .attack_target(actor, id, command.command_id, None)
                            .await;
                        session.target_walk = Some(PendingWalk {
                            target,
                            route,
                            due: tokio::time::Instant::now(),
                        });
                        reply(1)
                    }
                    Ok(None) => reply(3),
                    Err(()) => reply(2),
                }
            }
        }
    })
    .await
    .map_err(|_| ServeEnd::Disconnected)?;
    Ok(reply)
}

pub(super) async fn tick<S: AsyncRead + AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
) -> Result<(), ServeEnd> {
    session.chase_yielded = false;
    let Some(mut walk) = session.target_walk.take() else {
        return Ok(());
    };
    let Some(input) = authority
        .enter_input(session.admitted.game_session_id)
        .await
    else {
        return Ok(());
    };
    let actor = session.actor;
    let id = session.admitted.game_session_id;
    let result = tokio::time::timeout(Duration::from_secs(10), async {
        if !authority
            .item_walk_valid(actor, id, walk.route.expected)
            .await
        {
            return None;
        }
        if matches!(walk.target, AutoWalkTarget::Follow(_)) || walk.route.deferred {
            walk.route = authority
                .item_walk_route(actor, id, walk.target.clone())
                .await
                .ok()??;
        }
        let now = tokio::time::Instant::now();
        if walk.route.deferred || walk.route.steps.is_empty() {
            if matches!(walk.target, AutoWalkTarget::Follow(_)) || walk.route.deferred {
                walk.due = now + Duration::from_millis(100);
                session.target_walk = Some(walk);
            }
            return None;
        }
        if let Some(due) = session.pacer.wait_until(now) {
            walk.schedule(due);
            session.target_walk = Some(walk);
            return None;
        }
        if let Some(due) =
            begin_actor_step(&mut session.pacer, authority, actor, &session.step_holder).await
        {
            walk.schedule(due);
            session.target_walk = Some(walk);
            return None;
        }
        let direction = walk.route.steps.pop_front()?;
        let (outcome, duration, current) = authority
            .item_walk_step(actor, id, walk.route.expected, direction, walk.route.follow)
            .await;
        record_actor_step(
            &mut session.pacer,
            authority,
            actor,
            &session.step_holder,
            duration,
        )
        .await;
        if let Some(current) = current.filter(|_| outcome.disposition == StepDisposition::Moved) {
            walk.route.expected = current;
            let now = tokio::time::Instant::now();
            walk.schedule(session.pacer.wait_until(now).unwrap_or(now));
            session.target_walk = Some(walk);
        } else if matches!(walk.target, AutoWalkTarget::Follow(_)) {
            // A target can move during equipment/quest reads. Re-path on the next check;
            // the owner snapshot gate prevented this obsolete step from committing.
            walk.due = tokio::time::Instant::now() + Duration::from_millis(100);
            session.target_walk = Some(walk);
        }
        Some(outcome)
    })
    .await
    .map_err(|_| ServeEnd::Disconnected)?;
    drop(input);
    if let Some(outcome) = result {
        effects::step(stream, authority, session, outcome).await?;
    }
    Ok(())
}
