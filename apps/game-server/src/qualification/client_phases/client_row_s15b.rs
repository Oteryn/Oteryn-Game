//! S15b client replay capture: the in-fight logout refusal after a real rat fight. Character one
//! walks out of town protection, attacks a Rat placed by the qualification spawn fixture,
//! and sends the native LOGOUT while in
//! fight; the server refuses it (`InFight`), and logout on a protection tile
//! is accepted. The capture preserves both original server response envelopes.
use super::*;
use oteryn_dev_client::EntityKind;
use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
use oteryn_protocol_oteryn::logout::LogoutResult;

async fn walk_s15b(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    for &direction in path {
        let from = tile(client.world_spatial().actor_position)?;
        let expected = destination(world, from, direction).ok_or("S15b missing step landing")?;
        let outcome = client.step(direction).await?;
        if outcome.disposition != StepDisposition::Moved
            || outcome.world_spatial_delta.is_none()
            || tile(client.world_spatial().actor_position)? != expected
        {
            return Err(format!(
                "S15b step {direction:?} from={from:?} expected={expected:?} observed={outcome:?}"
            )
            .into());
        }
        verdict::Harness::global().progress("walk");
        client.take_events();
        if outcome.step_wait_ms != 0 {
            client
                .service_liveness(Duration::from_millis(u64::from(outcome.step_wait_ms)))
                .await?;
        }
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>, capture_dir: &Path) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_character(clients, 0x64, 1).await?;
    let initial_position = client.world_spatial().actor_position;
    let session_id = client.join_snapshot().game_session_id;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone())
    })
    .ok_or("step=setup need=walkable-unprotected-tile")?;
    walk_s15b(&mut client, world, &path).await?;
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let spawn = request_spawn(session, "oteryn:creature.rat".into())
        .await
        .map_err(|error| format!("step=setup Rat fixture refused: {error}"))?;
    let rat_ref = parse_spawned_entity(&spawn)?;
    let visible_by = tokio::time::Instant::now() + Duration::from_secs(5);
    let target = loop {
        if let Some(rat) = client
            .world_entities()
            .and_then(|view| view.get(&rat_ref))
            .copied()
        {
            if rat.kind != EntityKind::Creature {
                return Err("step=setup spawned Rat did not have creature kind".into());
            }
            break rat;
        }
        if tokio::time::Instant::now() >= visible_by {
            return Err("step=setup need=exact-spawned-Rat-visible".into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    };
    let here = client.world_spatial().actor_position;
    let rat = tile(target.position)?;
    let approach = route_avoiding(
        world,
        tile(here)?,
        |at| {
            at.floor == rat.floor
                && u32::from(at.x.abs_diff(rat.x)) + u32::from(at.y.abs_diff(rat.y)) == 1
                && world
                    .base()
                    .tile(at.x, at.y, at.floor)
                    .is_some_and(|tile| !tile.protection_zone())
        },
        |at| at != rat,
    )
    .ok_or("step=attack no cardinal, unprotected approach to the Rat")?;
    walk_s15b(&mut client, world, &approach).await?;
    let current = client
        .world_entities()
        .and_then(|view| view.get(&target.entity))
        .ok_or("step=attack target Rat left the entity view")?;
    let here = client.world_spatial().actor_position;
    if current.position.floor != here.floor
        || u64::from(current.position.x.abs_diff(here.x))
            + u64::from(current.position.y.abs_diff(here.y))
            != 1
    {
        return Err(format!(
            "step=attack Rat moved away from cardinal adjacency: actor={here:?} rat={:?}",
            current.position
        )
        .into());
    }
    evidence(&format!(
        "S15B_CLIENT_TARGET actor={here:?} rat={:?}",
        current.position
    ));
    let attack = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    if attack.disposition != AttackIntentDisposition::Ok {
        return Err(format!("step=attack reply={attack:?}").into());
    }
    let combat_deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    while !client
        .session_mut()
        .combat_state()
        .is_some_and(|state| state.in_fight && state.target == Some(target.entity))
    {
        let remaining = combat_deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(
                "step=attack accepted Rat target never acquired a served combat logout lock".into(),
            );
        }
        client
            .service_liveness(remaining.min(Duration::from_millis(50)))
            .await?;
    }
    let refusal_position = client.world_spatial().actor_position;
    let refusal_combat = client.session_mut().combat_state().copied();
    let (refused, refusal_frame) = client.session_mut().logout_with_frame().await?;
    let Some(LogoutResult::InFight { retry_after_ms }) = refused.result else {
        return Err(format!("step=logout-refusal observed {refused:?}, expected InFight").into());
    };
    if retry_after_ms == 0 {
        return Err("step=logout-refusal InFight carried no retry wait".into());
    }
    if client.world_spatial().actor_position != refusal_position
        || client.join_snapshot().game_session_id != session_id
        || client.session_mut().combat_state() != refusal_combat.as_ref()
    {
        return Err("step=logout-refusal changed position, session, or combat state".into());
    }
    let cleared = client.session_mut().attack_target(None).await?;
    if cleared.disposition != AttackIntentDisposition::Ok {
        return Err(format!("step=logout-cleanup could not clear Rat target: {cleared:?}").into());
    }
    let shelter = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| tile.protection_zone() && !tile.no_logout())
    })
    .ok_or("step=logout-cleanup no walkable protection tile outside Rat aggro")?;
    walk_s15b(&mut client, world, &shelter).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    while !client
        .session_mut()
        .combat_state()
        .is_some_and(|state| !state.in_fight && state.target.is_none())
    {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err("step=logout-cleanup combat lock did not clear within its duration".into());
        }
        client
            .service_liveness(remaining.min(Duration::from_millis(100)))
            .await?;
    }
    let saved_position = client.world_spatial().actor_position;
    if saved_position == initial_position {
        return Err("step=logout-cleanup saved position equals the admission position".into());
    }
    let (accepted, acceptance_frame) = client.session_mut().logout_with_frame().await?;
    if accepted.result != Some(LogoutResult::Accepted) {
        return Err(format!("step=logout-in-protection-zone observed {accepted:?}").into());
    }
    drop(client);
    let mut fresh = connect_character(clients, 0x65, 1).await?;
    if fresh.join_snapshot().game_session_id == session_id
        || fresh.world_spatial().actor_position != saved_position
    {
        return Err("step=relog expected a new session at the exact saved position".into());
    }
    let cleanup = fresh.session_mut().logout().await?;
    if cleanup.result != Some(LogoutResult::Accepted) {
        return Err(format!("step=relog-cleanup observed {cleanup:?}").into());
    }
    let manifest = format!(
        "{} {} {} {} {} {}\n",
        uuid_text(session_id.as_bytes()),
        refused.command_id,
        refused.result_server_sequence,
        accepted.command_id,
        accepted.result_server_sequence,
        retry_after_ms
    );
    use std::io::Write as _;
    for (name, bytes) in [
        ("s15b-refused.bin", refusal_frame.as_slice()),
        ("s15b-accepted.bin", acceptance_frame.as_slice()),
        ("s15b-expected.txt", manifest.as_bytes()),
    ] {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(capture_dir.join(name))?
            .write_all(bytes)?;
    }
    Ok(manifest.split_whitespace().collect::<Vec<_>>().join(","))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let Some(capture_dir) = std::env::var_os("OTERYN_S15B_CAPTURE_DIR")
        .or_else(|| std::env::var_os("E2E_ALL_CAPTURE_DIR"))
    else {
        evidence("e2e_all scenario=S15b.client status=FAIL evidence=missing-capture-directory");
        return;
    };
    match verdict::indexed_timeout(
        Duration::from_secs(240),
        verdict::observe(journey(clients, Path::new(&capture_dir))),
    )
    .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=S15b.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=S15b.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=S15b.client status=FAIL evidence=timeout"),
    }
}
