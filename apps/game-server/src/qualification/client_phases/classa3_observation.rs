use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind};
use oteryn_protocol_oteryn::logout::LogoutResult;

pub(super) async fn logout(client: &mut DevClientSession) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    let mut waiting = false;
    while client
        .session_mut()
        .combat_state()
        .is_some_and(|state| state.in_fight)
    {
        if !waiting {
            evidence("classa3 logout waiting for live combat state to clear");
            verdict::Harness::global().bounded_wait(deadline, "logout-combat-lock");
            waiting = true;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("combat state remained in fight for the PvE lock duration".into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
    }
    let reply = client.session_mut().logout().await?;
    evidence(&format!("classa3 logout first_reply={reply:?}"));
    if reply.result != Some(LogoutResult::Accepted) {
        return Err(format!("first logout refused: {reply:?}").into());
    }
    Ok(())
}

pub(super) async fn walk(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    for &direction in path {
        let before = client.world_spatial().actor_position;
        let from = tile(before)?;
        let expected = destination(world, from, direction).ok_or("missing route landing")?;
        let mut outcome = client.step_retrying(direction).await?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        while outcome.disposition == StepDisposition::Blocked
            && client.world_spatial().actor_position == before
        {
            let blocker = client.world_entities().and_then(|entities| {
                entities.iter().find(|entity| {
                    &entity.entity.identity != entities.own_identity()
                        && matches!(entity.kind, EntityKind::Creature | EntityKind::Npc | EntityKind::Player)
                        && tile(entity.position).ok() == Some(expected)
                })
            }).map(|entity| entity.entity)
                .ok_or_else(|| format!("BLOCKER-SERVER step Blocked with no occupant from={from:?} tile={expected:?} frame={outcome:?}"))?;
            evidence(&format!(
                "classa3 movement Blocked entity={blocker:?} tile={expected:?} frame={outcome:?}"
            ));
            verdict::Harness::global().bounded_wait(deadline, "movement-occupant");
            loop {
                if tokio::time::Instant::now() >= deadline {
                    return Err(format!(
                        "movement wait expired entity={blocker:?} tile={expected:?}"
                    )
                    .into());
                }
                client.service_liveness(Duration::from_millis(100)).await?;
                let position = client
                    .world_entities()
                    .and_then(|entities| entities.get(&blocker))
                    .map(|entity| entity.position);
                if position.and_then(|at| tile(at).ok()) != Some(expected) {
                    evidence(&format!(
                        "classa3 movement blocker moved entity={blocker:?} position={position:?}"
                    ));
                    break;
                }
            }
            outcome = client.step_retrying(direction).await?;
        }
        if outcome.disposition != StepDisposition::Moved
            || outcome.world_spatial_delta.is_none()
            || tile(client.world_spatial().actor_position)? != expected
        {
            return Err(format!(
                "route step failed from={from:?} tile={expected:?} frame={outcome:?}"
            )
            .into());
        }
        verdict::Harness::global().progress("walk");
        client.take_events();
    }
    Ok(())
}

pub(super) async fn approach(
    client: &mut DevClientSession,
    world: &BundleWorld,
    target: ActorPosition,
) -> TestResult {
    let target = tile(target)?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at.floor == target.floor && at.x.abs_diff(target.x).max(at.y.abs_diff(target.y)) == 1
    })
    .ok_or("no adjacent route to the recorded target")?;
    walk(client, world, &path).await
}
