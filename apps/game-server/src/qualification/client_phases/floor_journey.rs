// One real automatic floor transition, distinct from tool-operated ladders/ropes/holes.
async fn floor_journey(
    mut client: oteryn_dev_client::DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult<oteryn_dev_client::DevClientSession> {
    let started = std::time::Instant::now();
    let from = tile(client.world_spatial().actor_position)?;
    let path = route(world, from, |at| at.floor != from.floor)
        .ok_or("no reachable automatic floor transition within the town route budget")?;
    walk(&mut client, world, &path).await?;
    let landed = client.world_spatial().actor_position;
    if tile(landed)?.floor == from.floor {
        return Err("floor route did not produce a protocol floor change".into());
    }
    let old_session = client.join_snapshot().game_session_id;
    let (result, logout, attempts, busy_timed_out) = logout_retrying_busy(&mut client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("floor journey logout refused: {logout}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
    }
    drop(client);
    client = connect_world(clients, 0x77).await?;
    if client.join_snapshot().game_session_id == old_session
        || client.world_spatial().actor_position != landed
    {
        return Err("floor journey relog did not retain the landing position".into());
    }
    evidence(&format!(
        "e2e_all scenario=M03.floor-step-relogin status=PASS seconds={:.3}",
        started.elapsed().as_secs_f64()
    ));
    let back =
        route(world, tile(landed)?, |at| at == from).ok_or("floor journey has no return route")?;
    walk(&mut client, world, &back).await?;
    Ok(client)
}
