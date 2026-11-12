//! S15b client replay capture: the in-fight logout refusal after a real rat fight. Character one
//! walks to a compiled rat spawn, attacks an adjacent rat, and sends the native LOGOUT while in
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
        evidence(&format!("S15B_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_character(clients, 0x64, 1).await?;
    super::travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
    let here = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == EntityKind::Creature
                    && entry.position.floor == here.floor
                    && entry.position.x.abs_diff(here.x).max(entry.position.y.abs_diff(here.y)) <= 1
            })
        })
        .copied()
        .ok_or("step=attack no adjacent creature after walking")?;
    client.take_events();
    let attack = client.session_mut().attack_target(Some(&target.entity)).await?;
    if attack.disposition != AttackIntentDisposition::Ok {
        return Err(format!("step=attack reply={attack:?}").into());
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
    let accepted = loop {
        client.session_mut().attack_target(None).await?;
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
    match verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients))).await {
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
