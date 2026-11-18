//! Capture the real capability-64 receipts consumed by the native M01 client adapter.
use super::*;
use oteryn_protocol_oteryn::target_move::*;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = verdict::indexed_timeout(Duration::from_secs(75), verdict::observe(journey(clients)))
        .await.unwrap_or_else(|_| Err("M01 movement journey deadline".into()));
    match result {
        Ok(()) => evidence("e2e_all scenario=M01.client status=PASS evidence=step-walk-stop-turn-peer-follow"),
        Err(error) => evidence(&format!("e2e_all scenario=M01.client status=FAIL evidence={error}")),
    }
}

fn movement_accepted(status: CommandStatus, payload: &[u8]) -> bool {
    status == CommandStatus::Accepted && payload == [8, 1]
}

async fn command(
    client: &mut oteryn_dev_client::DevClientSession,
    intent: TargetMoveIntent,
) -> TestResult<Vec<u8>> {
    let payload = encode_target_move_intent(intent).map_err(|error| format!("M01 {intent:?} encode: {error:?}"))?;
    let result = client
        .session_mut()
        .gated_command(CAPABILITY_TARGET_MOVE_V1, COMMAND_TYPE_TARGET_MOVE_INTENT, &payload)
        .await?;
    if !movement_accepted(result.status, &result.payload) {
        return Err(format!("M01 {intent:?} command refused: {result:?}").into());
    }
    Ok(result.payload)
}

fn capture(name: &str, payload: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else { return };
    if let Err(error) = std::fs::write(std::path::Path::new(&root).join(format!("m01-{name}.bin")), payload) {
        evidence(&format!("M01_CLIENT_CAPTURE_WRITE name={name} error={error}"));
    }
}

pub(super) fn own_direction(client: &oteryn_dev_client::DevClientSession) -> Option<StepDirection> {
    use oteryn_dev_client::EntityDetail;
    let entities = client.world_entities()?;
    let actor = entities.iter().find(|entity| &entity.entity.identity == entities.own_identity())?;
    match actor.detail {
        EntityDetail::Actor { direction, .. } | EntityDetail::ActorWithIcons { direction, .. } => Some(direction),
        _ => None,
    }
}

async fn wait_movement(
    client: &mut oteryn_dev_client::DevClientSession,
    observed: impl Fn(&oteryn_dev_client::DevClientSession) -> bool,
    step: &str,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    while !observed(client) {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("M01 {step} did not publish the expected actor state").into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
    let mut client = connect_world(clients, 0x6d).await?;
    let start = tile(client.world_spatial().actor_position)?;
    let path = route(world, start, |at| at != start).ok_or("M01 capture movement lane")?;
    let direction = *path.first().ok_or("M01 capture empty route")?;
    let expected_step = destination(world, start, direction).ok_or("M01 invalid first step plan")?;
    let step = client.step_retrying(direction).await?;
    if step.disposition != StepDisposition::Moved || step.world_spatial_delta.is_none()
        || tile(client.world_spatial().actor_position)? != expected_step
    {
        return Err(format!("M01 first step did not land at {expected_step:?}: {step:?}").into());
    }

    let here = tile(client.world_spatial().actor_position)?;
    let path = route(world, here, |at| at.floor == here.floor
        && at.x.abs_diff(here.x).max(at.y.abs_diff(here.y)) >= 5).ok_or("M01 no multi-step movement lane")?;
    let destination_tile = path.iter().try_fold(here, |from, direction| destination(world, from, *direction))
        .ok_or("M01 invalid multi-step route")?;
    let goal = ActorPosition { x: i32::from(destination_tile.x), y: i32::from(destination_tile.y), floor: -i16::from(destination_tile.floor) };
    let before_walk = client.world_spatial().actor_position;
    let autowalk = command(&mut client, TargetMoveIntent::Walk(goal)).await?;
    wait_movement(&mut client, |client| client.world_spatial().actor_position != before_walk, "walk").await?;
    let stop = command(&mut client, TargetMoveIntent::Stop).await?;
    let stopped = client.world_spatial().actor_position;
    if stopped == goal {
        return Err("M01 autowalk finished before stop could be tested".into());
    }
    client.service_liveness(Duration::from_secs(2)).await?;
    if client.world_spatial().actor_position != stopped {
        return Err("M01 stopped movement still changed position".into());
    }
    client.take_events();
    let facing = own_direction(&client).ok_or("M01 own actor direction missing")?;
    let direction = if facing == StepDirection::North { StepDirection::South } else { StepDirection::North };
    let turn = command(&mut client, TargetMoveIntent::Turn(direction)).await?;
    wait_movement(&mut client, |client| own_direction(client) == Some(direction), "turn").await?;
    if client.world_spatial().actor_position != stopped {
        return Err("M01 turn translated the actor".into());
    }

    let mut peer = with_peer(&mut client, connect_character(clients, 0x6e, 0)).await?;
    let follow_from = tile(stopped)?;
    let path = route(world, tile(peer.world_spatial().actor_position)?, |at| {
        at.floor == follow_from.floor
            && (3..=5).contains(&at.x.abs_diff(follow_from.x).max(at.y.abs_diff(follow_from.y)))
    }).ok_or("M01 no reachable peer follow lane")?;
    with_peer(&mut client, super::walk(&mut peer, world, &path)).await?;
    let (peer_ref, peer_position) = {
        let entities = peer.world_entities().ok_or("M01 peer entity view missing")?;
        let entity = entities.iter().find(|entity| &entity.entity.identity == entities.own_identity())
            .ok_or("M01 peer actor missing")?;
        (entity.entity, entity.position)
    };
    wait_movement(&mut client, |client| client.world_entities().and_then(|entities| entities.get(&peer_ref)).is_some(), "peer visibility").await?;
    let follow = command(&mut client, TargetMoveIntent::Follow(peer_ref)).await?;
    with_peer(&mut peer, wait_movement(&mut client, |client| {
        let at = client.world_spatial().actor_position;
        at != stopped && at.floor == peer_position.floor
            && at.x.abs_diff(peer_position.x).max(at.y.abs_diff(peer_position.y)) <= 1
    }, "follow admitted peer")).await?;
    command(&mut client, TargetMoveIntent::Stop).await?;
    for (label, reply) in [("autowalk", autowalk), ("stop", stop), ("turn", turn), ("follow", follow)] {
        capture(label, &reply);
    }
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR")
        && let Err(error) = std::fs::write(std::path::Path::new(&root).join("m01-expected.txt"), "autowalk=accepted\nstop=accepted\nturn=accepted\nfollow=accepted\n")
    {
        evidence(&format!("M01_CLIENT_CAPTURE_WRITE name=expected error={error}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movement_ack_requires_disposition_one() {
        assert!(!movement_accepted(CommandStatus::Accepted, &[8, 2]));
        assert!(!movement_accepted(CommandStatus::Accepted, &[]));
        assert!(movement_accepted(CommandStatus::Accepted, &[8, 1]));
    }
}
