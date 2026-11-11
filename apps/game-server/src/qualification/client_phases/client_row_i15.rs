//! Captures the real actor-vitals payload consumed by the native session reducer for I15.
use super::*;
use oteryn_dev_client::SessionEvent;
use oteryn_protocol_oteryn::{actor_spell, item_view as items};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = row_complete(journey(clients)).await;
    match result {
        Ok(Ok(())) => evidence(
            "e2e_all scenario=I15.client status=PASS evidence=actor-vitals-2-to-1-mana-shield",
        ),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=I15.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=I15.client status=FAIL evidence=damage-delta-timeout"),
    }
}

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&path).join(name), bytes)?;

    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let active = provisioning::item_reference(&provisioning::item_index()?, 3088)?;
    let mut client = connect_world(clients, 0xb2).await?;
    let row_needs_logout_0 = true;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        let result: TestResult = async {
            let equipped = client
                .inventory()
                .ok_or("I15 inventory absent")?
                .equipment
                .iter()
                .any(|entry| {
                    entry.slot == items::EquipmentSlot::Ring
                        && entry.item.item_definition_ref == active
                });
            if !equipped {
                return Err("I15 active Energy Ring 3088 absent after I05 lifecycle".into());
            }

            let mut previous = *client.actor_vitals().ok_or("I15 admission vitals absent")?;
            client.take_events();
            travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
            let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
            loop {
                for event in client.take_events() {
                    if let SessionEvent::ActorVitals(delta) = event {
                        let current = delta.value;
                        if current.health == previous.health && previous.mana > current.mana {
                            let paid = previous.mana - current.mana;
                            if paid % 2 != 0 {
                                return Err(format!(
                                    "I15 mana debit is not an accepted 2:1 cost: {paid}"
                                )
                                .into());
                            }
                            let before = actor_spell::encode_actor_vitals(&previous)
                                .map_err(|error| format!("I15 before encode: {error:?}"))?;
                            let after = actor_spell::encode_actor_vitals(&current)
                                .map_err(|error| format!("I15 after encode: {error:?}"))?;
                            capture("i15-before.bin", &before)?;
                            capture("i15-after.bin", &after)?;
                            capture(
                                "i15-expected.txt",
                                format!("{} {}\n", current.health, current.mana).as_bytes(),
                            )?;
                            return Ok(());
                        }
                        previous = current;
                    }
                }
                if tokio::time::Instant::now() >= deadline {
                    return Err(format!(
                        "I15 no shielded damage payload; last_vitals={previous:?}"
                    )
                    .into());
                }
                client.service_liveness(Duration::from_millis(50)).await?;
            }
        }
        .await;
        let logout = client.session_mut().logout().await;
        result.and(logout.map(|_| ()).map_err(Into::into))
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    if row_needs_logout_0 {
        row_logout(&mut client).await?;
    }
    row_result
}
