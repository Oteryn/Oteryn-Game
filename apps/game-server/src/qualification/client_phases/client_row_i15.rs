//! Captures the real actor-vitals payload consumed by the native session reducer for I15.
use super::*;
use oteryn_dev_client::{SessionEvent, client_runtime::inventory};
use oteryn_protocol_oteryn::{actor_spell, item_view as items};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = verdict::observe(journey(clients)).await;
    match result {
        Ok(()) => evidence("e2e_all scenario=I15.client status=PASS evidence=actor-vitals-2-to-1-mana-shield"),
        Err(error) => evidence(&format!("e2e_all scenario=I15.client status=FAIL evidence={error}")),
    }
}

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else { return Ok(()); };
    std::fs::write(Path::new(&path).join(name), bytes)?;
    Ok(())
}

fn shield_ratio(health_unchanged: bool, mana_debit: u32, incoming_damage: u32) -> bool {
    health_unchanged && incoming_damage > 0 && incoming_damage.checked_mul(2) == Some(mana_debit)
}

async fn move_ring(client: &mut oteryn_dev_client::DevClientSession, handle: items::ItemHandle, worn: bool) -> TestResult {
    let destination = if worn { items::ItemMoveDestination::Equipment(items::EquipmentSlot::Ring) } else { items::ItemMoveDestination::MainBackpack };
    let cutoff = client.last_server_sequence();
    client.take_events();
    let reply = inventory::send(client.session_mut(), inventory::MoveRequest { source: handle, destination, count: None })
        .await?.ok_or("I15 equipment move unavailable")?;
    if reply.outcome != items::ItemMoveOutcome::Moved {
        return Err(format!("I15 ring move failed: {reply:?}").into());
    }
    let until = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        if client.take_events().into_iter().any(|event| matches!(event, SessionEvent::Inventory(delta)
            if delta.server_sequence > cutoff
                && if worn {
                    delta.value.equipment.iter().any(|entry| entry.slot == items::EquipmentSlot::Ring && entry.item.handle == handle)
                } else {
                    delta.value.entries.iter().any(|entry| entry.handle == handle)
                    && !delta.value.equipment.iter().any(|entry| entry.slot == items::EquipmentSlot::Ring)
                })) {
            return Ok(());
        }
        if tokio::time::Instant::now() >= until { return Err("I15 ring move inventory frame absent".into()); }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
}

async fn restore_after_reconnect(
    clients: &SeamClients<'_>, active: std::num::NonZeroU32, inactive: std::num::NonZeroU32,
) -> TestResult {
    verdict::Harness::global().bounded_wait(tokio::time::Instant::now() + Duration::from_secs(200), "I15 ring recovery");
    let mut fresh = connect_world(clients, 0xb4).await?;
    let view = fresh.inventory().ok_or("I15 recovery inventory absent")?;
    if !view.equipment.iter().any(|entry| entry.slot == items::EquipmentSlot::Ring && entry.item.item_definition_ref == active) {
        let handle = view.entries.iter()
            .find(|entry| entry.item_definition_ref == inactive || entry.item_definition_ref == active)
            .ok_or("I15 original Energy Ring missing after re-admission")?.handle;
        move_ring(&mut fresh, handle, true).await?;
    }
    client_row_c08::require_logout(&mut fresh, Duration::from_secs(15)).await
}

async fn observe_drowning(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<u32> {
    let mut previous = *client.actor_vitals().ok_or("I15 unshielded vitals absent")?;
    let until = tokio::time::Instant::now() + Duration::from_secs(7);
    loop {
        if client.world_entities().is_none_or(|entities| entities.others().any(|entity|
            matches!(entity.kind, oteryn_dev_client::EntityKind::Creature | oteryn_dev_client::EntityKind::Player)
        )) {
            return Err("I15 unshielded sample may include another attacker".into());
        }
        for event in client.take_events() {
            if let SessionEvent::ActorVitals(delta) = event {
                let current = delta.value;
                let loss = previous.health.saturating_sub(current.health);
                if loss > 0 && current.mana == previous.mana { return Ok(loss); }
                previous = current;
            }
        }
        if tokio::time::Instant::now() >= until { return Err("I15 unshielded drowning hit absent".into()); }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
    let index = provisioning::item_index()?;
    let active = provisioning::item_reference(&index, 3088)?;
    let inactive = provisioning::item_reference(&index, 3051)?;
    let mut client = verdict::indexed_timeout(Duration::from_secs(190), connect_world(clients, 0xb2)).await??;
    let mut recover_ring = false;
    let sample_deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    let result: TestResult = tokio::time::timeout_at(sample_deadline, async {
        let handle = client.inventory().ok_or("I15 inventory absent")?.equipment.iter()
            .find(|entry| entry.slot == items::EquipmentSlot::Ring && entry.item.item_definition_ref == active)
            .ok_or("I15 active Energy Ring 3088 absent after I05 lifecycle")?.item.handle;
        if client.inventory().is_some_and(|view| view.entries.iter().any(|entry| entry.item_definition_ref == active || entry.item_definition_ref == inactive)) {
            return Err("I15 recovery requires a unique Energy Ring".into());
        }
        let from = tile(client.world_spatial().actor_position)?;
        let mut path = route(world, from, |at| client_row_m10::is_underwater(world, at))
            .ok_or("I15 no reachable underwater tile for a known incoming hit")?;
        let enter = path.pop().ok_or("I15 started underwater")?;
        walk(&mut client, world, &path).await?;
        let baseline = client_row_c17a::query(&mut client).await?.0;
        if baseline.conditions.iter().any(|condition| (1..=8).contains(&condition.icon)) {
            return Err("I15 unexpected pre-existing damage condition".into());
        }
        client.take_events();
        let moved = client.step_retrying(enter).await?;
        if moved.disposition != StepDisposition::Moved || moved.world_spatial_delta.is_none() {
            return Err("I15 entering underwater did not move".into());
        }
        let observed = client_row_c17a::query(&mut client).await?.0;
        let drowning = observed.conditions.iter().any(|condition| condition.icon == 5);
        let other_harm = observed.conditions.iter().any(|condition| condition.icon != 5 && (1..=8).contains(&condition.icon));
        if !drowning || other_harm {
            return Err("I15 expected drowning as the only damage condition".into());
        }
        verdict::Harness::global().bounded_wait(sample_deadline, "I15 ring control");
        recover_ring = true;
        let removal = move_ring(&mut client, handle, false).await;
        let incoming_damage = async {
            removal?;
            observe_drowning(&mut client).await
        }.await;
        // When the removal reply/delta is missing the original stream and
        // inventory may disagree. Recovery re-admits before reading custody.
        let incoming_damage = incoming_damage?;
        move_ring(&mut client, handle, true).await?;
        recover_ring = false;
        let mut previous = *client.actor_vitals().ok_or("I15 shielded vitals absent")?;
        let needed = incoming_damage.checked_mul(2).ok_or("I15 incoming hit overflow")?;
        if previous.mana < needed {
            return Err("I15 insufficient mana for the observed hit".into());
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(7);
        loop {
            if client.world_entities().is_some_and(|entities| entities.iter().any(|entity|
                matches!(entity.kind, oteryn_dev_client::EntityKind::Creature | oteryn_dev_client::EntityKind::Player)
                && &entity.entity.identity != entities.own_identity()
            )) {
                return Err("I15 another visible attacker could supply the observed hit".into());
            }
            for event in client.take_events() {
                if let SessionEvent::ActorVitals(delta) = event {
                    let current = delta.value;
                    if current.health < previous.health || previous.mana > current.mana {
                        let paid = previous.mana.saturating_sub(current.mana);
                        if !shield_ratio(current.health == previous.health, paid, incoming_damage) { return Err(format!("I15 mana debit is not an accepted 2:1 cost: {paid}").into()); }
                        let before = actor_spell::encode_actor_vitals(&previous).map_err(|error| format!("I15 before encode: {error:?}"))?;
                        let after = actor_spell::encode_actor_vitals(&current).map_err(|error| format!("I15 after encode: {error:?}"))?;
                        capture("i15-before.bin", &before)?;
                        capture("i15-after.bin", &after)?;
                        capture("i15-expected.txt", format!("{} {}\n", current.health, current.mana).as_bytes())?;
                        return Ok(());
                    }
                    previous = current;
                }
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("I15 no shielded damage payload; last_vitals={previous:?}").into());
            }
            client.service_liveness(Duration::from_millis(50)).await?;
        }
    }).await.unwrap_or_else(|error| Err(verdict::context("I15 sample deadline".into(), error)));
    if recover_ring {
        drop(client);
        let recovery = verdict::indexed_timeout(
            Duration::from_secs(200), restore_after_reconnect(clients, active, inactive),
        ).await.unwrap_or_else(|error| Err(verdict::context("I15 recovery deadline".into(), error)));
        if let Err(error) = &recovery {
            evidence(&format!("I15 ring recovery failed: {error}"));
        }
        return result.and(recovery);
    }
    let logout = verdict::indexed_timeout(Duration::from_secs(75), client_row_c08::require_logout(&mut client, Duration::from_secs(70))).await?;
    result.and(logout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shield_ratio_requires_twice_the_observed_hit() {
        assert!(!shield_ratio(true, 2, 20));
        assert!(!shield_ratio(true, 20, 20));
        assert!(!shield_ratio(false, 40, 20));
        assert!(shield_ratio(true, 40, 20));
    }
}
