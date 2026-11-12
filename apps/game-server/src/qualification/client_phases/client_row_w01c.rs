//! W01c client replay capture: an admitted poison spider's melee attack reaches the player as an
//! `ACTOR_VITALS` health decrease. The session retains typed vitals only, so the captured payload
//! is the server-delivered `ActorVitals` encoded with the production encoder (the same convention
//! as M11a). The consumer is `Session::apply_state_delta` and `Session::actor_vitals` in
//! `crates/session`.
use super::*;
use oteryn_dev_client::SessionEvent;
use oteryn_protocol_oteryn::actor_spell;
use oteryn_protocol_oteryn::attack::AttackIntentDisposition;

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("W01C_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
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
    let mut client = connect_world(clients, 0x0d).await?;
    travel_to_spawn(
        &mut client,
        world,
        &metadata.spawns,
        "oteryn:creature.poison_spider",
    )
    .await?;
    let before = *client
        .actor_vitals()
        .ok_or("W01c client capture: admission supplied no ActorVitals")?;
    let cancel = client.session_mut().attack_target(None).await?;
    if cancel.disposition != AttackIntentDisposition::Ok {
        return Err(format!("W01c client capture: attack-cancel refused: {cancel:?}").into());
    }
    client.take_events();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(45);
    let damaged = 'wait: loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        for event in client.take_events() {
            if let SessionEvent::ActorVitals(delta) = event
                && delta.value.health > 0
                && delta.value.health < before.health
            {
                break 'wait delta.value;
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("W01c client capture: no spider damage delta in 45s".into());
        }
    };
    capture(
        "w01c-vitals.bin",
        &actor_spell::encode_actor_vitals(&damaged)
            .map_err(|error| format!("encode vitals: {error:?}"))?,
    );
    let expected = format!(
        "health_before={}\nhealth={}\nmax_health={}\nmana={}\nmax_mana={}\nsoul={}\n",
        before.health, damaged.health, damaged.max_health, damaged.mana, damaged.max_mana, damaged.soul
    );
    capture("w01c-expected.txt", expected.as_bytes());
    Ok(expected)
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    match verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients))).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=W01c.client status=CAPTURED evidence={:?}",
            expected.replace('\n', ";")
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W01c.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=W01c.client status=FAIL evidence=timeout"),
    }
}
