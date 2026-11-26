//! Capture the real capability-64 receipts consumed by the native M01 client adapter.
use super::*;
use oteryn_protocol_oteryn::target_move::*;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::observe(journey(clients)).await {
        Ok(()) => evidence("e2e_all scenario=M01.client status=PASS evidence=real-target-move-replies"),
        Err(error) => evidence(&format!("M01_CLIENT_CAPTURE_GAP {error}")),
    }
}

async fn command(
    client: &mut oteryn_dev_client::DevClientSession,
    payload: &[u8],
) -> TestResult<Vec<u8>> {
    let result = client
        .session_mut()
        .gated_command(CAPABILITY_TARGET_MOVE_V1, COMMAND_TYPE_TARGET_MOVE_INTENT, payload)
        .await?;
    if result.status != CommandStatus::Accepted {
        return Err(format!("target-move command refused: {:?}", result.status).into());
    }
    Ok(result.payload)
}

fn capture(name: &str, payload: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else { return };
    if let Err(error) = std::fs::write(std::path::Path::new(&root).join(format!("m01-{name}.bin")), payload) {
        evidence(&format!("M01_CLIENT_CAPTURE_WRITE name={name} error={error}"));
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
    let mut client = connect_world(clients, 0x6d).await?;
    let start = tile(client.world_spatial().actor_position)?;
    let path = route(world, start, |at| at != start).ok_or("M01 capture movement lane")?;
    let direction = *path.first().ok_or("M01 capture empty route")?;
    let _ = client.step(direction).await?;
    let next = destination(world, tile(client.world_spatial().actor_position)?, direction)
        .ok_or("M01 capture autowalk destination")?;
    let walk_intent = encode_target_move_intent(TargetMoveIntent::Walk(ActorPosition { x: i32::from(next.x), y: i32::from(next.y), floor: -i16::from(next.floor) }))
        .map_err(|error| format!("M01 walk encode: {error:?}"))?;
    capture("autowalk", &command(&mut client, &walk_intent).await?);
    capture("stop", &command(&mut client, &[16, 1]).await?);
    capture("turn", &command(&mut client, &[26, 2, 8, StepDirection::North as u8]).await?);

    let mut peer =
        provisioning::with_peer_liveness(&mut client, connect_character(clients, 0x6e, 0))
            .await?;
    let follower = tile(client.world_spatial().actor_position)?;
    let peer_at = tile(peer.world_spatial().actor_position)?;
    let path = route(world, peer_at, |at| {
        at.floor == follower.floor && at.x.abs_diff(follower.x).max(at.y.abs_diff(follower.y)) == 3
    })
    .ok_or("M01 capture follow peer has no approach")?;
    provisioning::with_peer_liveness(&mut client, walk(&mut peer, world, &path)).await?;

    let peer_entities = peer.world_entities().ok_or("M01 capture peer entity view")?;
    let target = peer_entities
        .iter()
        .find(|entity| &entity.entity.identity == peer_entities.own_identity())
        .ok_or("M01 capture peer identity absent")?
        .entity;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !client
        .world_entities()
        .is_some_and(|entities| entities.get(&target).is_some())
    {
        if tokio::time::Instant::now() >= deadline {
            return Err("M01 capture follower did not observe its peer".into());
        }
        peer.service_liveness(Duration::from_millis(25)).await?;
        client.service_liveness(Duration::from_millis(25)).await?;
    }
    let follow = encode_target_move_intent(TargetMoveIntent::Follow(target))
        .map_err(|error| format!("M01 follow encode: {error:?}"))?;
    capture("follow", &command(&mut client, &follow).await?);
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR")
        && let Err(error) = std::fs::write(std::path::Path::new(&root).join("m01-expected.txt"), "autowalk=accepted\nstop=accepted\nturn=accepted\nfollow=accepted\n")
    {
        evidence(&format!("M01_CLIENT_CAPTURE_WRITE name=expected error={error}"));
    }
    Ok(())
}
