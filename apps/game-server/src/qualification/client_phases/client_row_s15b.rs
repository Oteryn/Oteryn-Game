//! S15b client replay capture: the in-fight logout refusal after a real rat fight. Character one
//! walks out of town protection, attacks a Rat placed by the qualification spawn fixture,
//! and sends the native LOGOUT while in
//! fight; the server refuses it (`InFight`), and an ordinary retry after the combat lock expires
//! is accepted. `Session::logout` decodes the result into `LogoutOutcome` and does not retain the
//! raw bytes, so the capture records the server-observed outcomes in `s15b-expected.txt`; the
//! replay encodes them with the production logout encoder. No `.bin` file is written.
use super::*;
use oteryn_dev_client::EntityKind;
use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
use oteryn_protocol_oteryn::logout::LogoutResult;

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "S15B_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_character(clients, 0x64, 1).await?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone())
    })
    .ok_or("step=setup need=walkable-unprotected-tile")?;
    walk(&mut client, world, &path).await?;
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
    walk(&mut client, world, &approach).await?;
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
    let refused = client.session_mut().logout().await?;
    let Some(LogoutResult::InFight { retry_after_ms }) = refused.result else {
        return Err(format!("step=logout-refusal observed {refused:?}, expected InFight").into());
    };
    if retry_after_ms == 0 {
        return Err("step=logout-refusal InFight carried no retry wait".into());
    }
    let mut expected = format!(
        "logout-refusal\tstatus={:?}\tresult=InFight\tretry_after_ms={retry_after_ms}\n",
        refused.status
    );
    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
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
    walk(&mut client, world, &shelter).await?;
    let accepted = loop {
        client.service_liveness(Duration::from_millis(250)).await?;
        let reply = client.session_mut().logout().await?;
        match reply.result {
            Some(LogoutResult::Accepted) => break reply,
            Some(LogoutResult::InFight { .. } | LogoutResult::Busy)
                if tokio::time::Instant::now() < deadline => {}
            _ => return Err(format!("step=logout-retry observed {reply:?}").into()),
        }
    };
    expected.push_str(&format!(
        "logout-accepted\tstatus={:?}\tresult=Accepted\n",
        accepted.status
    ));
    capture("s15b-expected.txt", expected.as_bytes());
    Ok(format!("refused_retry_after_ms={retry_after_ms}"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    match verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients)))
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
