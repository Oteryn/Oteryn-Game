//! C09 client replay capture: create real HP/mana deficits, purchase and consume real food, then
//! capture the ACTOR_VITALS payload applied by `crates/session/src/domain_dispatch.rs` through
//! `Session::apply_domain_delta` and exposed by `Session::actor_vitals` in `session_access.rs`.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::actor_spell::ActorVitals;
use oteryn_protocol_oteryn::world_object::UseDisposition;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result =
        verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients)))
            .await;
    match result {
        Ok(Ok(())) => evidence(
            "e2e_all scenario=C09.client status=PASS evidence=food-funded-HP-mana-payloads",
        ),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=C09.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=C09.client status=FAIL evidence=journey-timeout"),
    }
}

fn quantity(client: &DevClientSession, reference: std::num::NonZeroU32) -> u64 {
    client.inventory().map_or(0, |inventory| {
        inventory
            .entries
            .iter()
            .filter(|item| item.item_definition_ref == reference)
            .map(|item| u64::from(item.count.get()))
            .sum()
    })
}

async fn acquire_meat(
    client: &mut DevClientSession,
    world: &BundleWorld,
) -> TestResult<std::num::NonZeroU32> {
    let index = provisioning::item_index()?;
    let reference = provisioning::item_reference(&index, 3577)?;
    if quantity(client, reference) == 0 {
        super::full_row_c09::buy_meat(client, world, reference, 1).await?;
    }
    Ok(reference)
}

fn capture(before: ActorVitals, after: ActorVitals) {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    let write = || -> TestResult {
        let path = Path::new(&path);
        std::fs::write(
            path.join("c09-before.bin"),
            oteryn_protocol_oteryn::actor_spell::encode_actor_vitals(&before)
                .map_err(|error| format!("encode before: {error:?}"))?,
        )?;
        std::fs::write(
            path.join("c09-regenerated.bin"),
            oteryn_protocol_oteryn::actor_spell::encode_actor_vitals(&after)
                .map_err(|error| format!("encode regenerated: {error:?}"))?,
        )?;
        std::fs::write(
            path.join("c09-expected.txt"),
            format!(
                "{} {} {} {} {} {} {}\n",
                after.health,
                after.max_health,
                after.mana,
                after.max_mana,
                after.soul,
                after.harmony,
                u8::from(after.serene)
            ),
        )?;
        Ok(())
    };
    if let Err(error) = write() {
        evidence(&format!("C09_CAPTURE_WRITE_ERROR {error}"));
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0xc9).await?;
    let meat = acquire_meat(&mut client, world).await?;
    travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
    let full = *client.actor_vitals().ok_or("C09 initial vitals absent")?;
    super::full_row_c09::spend_mana(&mut client).await?;
    for _ in 0..3 {
        client_hunt::hunt(&mut client).await.map_err(|error| {
            verdict::record_error(verdict::IndexedError::as_error(&error));
            verdict::context(format!("C09 combat: {error}"), error)
        })?;
        if client
            .actor_vitals()
            .is_some_and(|v| v.health < full.health)
        {
            break;
        }
    }
    let before = *client.actor_vitals().ok_or("deficit vitals absent")?;
    if before.health >= full.health || before.mana >= full.mana {
        return Err(format!("HP/mana deficits missing full={full:?} before={before:?}").into());
    }
    let handle = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|item| item.item_definition_ref == meat)
        })
        .map(|item| item.handle)
        .ok_or("meat handle absent")?;
    if client.use_item(handle).await?.disposition != UseDisposition::Committed {
        return Err("food use was not committed".into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        for event in client.take_events() {
            if let SessionEvent::ActorVitals(delta) = event
                && delta.value.health > before.health
                && delta.value.mana > before.mana
            {
                capture(before, delta.value);
                return Ok(());
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "food regeneration timeout before={before:?} current={:?}",
                client.actor_vitals()
            )
            .into());
        }
    }
}
