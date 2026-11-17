//! Survival decisions for an ordinary, unprovisioned rat hunter.
pub(super) fn ready_to_fight(health: u32, maximum: u32) -> bool {
    maximum > 0 && health == maximum
}
pub(super) fn should_retreat(health: u32, maximum: u32, other_threats: usize) -> bool {
    maximum == 0 || u64::from(health) * 4 <= u64::from(maximum) * 3 || other_threats > 0
}

use super::*;
pub(super) fn other_threats(
    client: &DevClientSession,
    target: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
) -> usize {
    let at = client.world_spatial().actor_position;
    let prey = client
        .world_entities()
        .and_then(|entities| entities.get(&target))
        .map(|entry| entry.position)
        .unwrap_or(at);
    client
        .world_entities()
        .into_iter()
        .flat_map(|entities| entities.iter())
        .filter(|entry| {
            entry.kind == EntityKind::Creature
                && entry.entity != target
                && entry.position.floor == at.floor
                && (entry
                    .position
                    .x
                    .abs_diff(at.x)
                    .max(entry.position.y.abs_diff(at.y))
                    <= 4
                    || entry
                        .position
                        .x
                        .abs_diff(prey.x)
                        .max(entry.position.y.abs_diff(prey.y))
                        <= 4)
        })
        .count()
}
pub(crate) async fn recover_rat_health(
    client: &mut DevClientSession,
    world: &crate::map::boot::BundleWorld,
    until: tokio::time::Instant,
) -> TestResult {
    let before = *client.actor_vitals().ok_or("rat recovery vitals absent")?;
    if ready_to_fight(before.health, before.max_health) {
        return Ok(());
    }
    if before.health == 0 {
        return Err("rat recovery cannot heal a dead player".into());
    }
    client_world::rat_retreat(client, world).await?;
    let foods = client_world::native_food_references()?;
    let food = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|item| foods.contains(&item.item_definition_ref))
        })
        .map(|item| item.handle);
    if let Some(food) = food {
        let reply = client.use_item(food).await?;
        evidence(&format!(
            "rat_care phase=food handle={food:?} reply={reply:?}"
        ));
    }
    // Native PZ holds food/hungry clocks without a daily streak. Stand at a
    // clear border tile with a one-step refuge, never manufacture a resting bonus.
    client_world::rat_recovery_edge(client, world).await?;
    let deadline = until.min(tokio::time::Instant::now() + Duration::from_secs(180));
    client_world::rat_recovery_wait(deadline);
    let mut previous = before.health;
    loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        let vitals = *client.actor_vitals().ok_or("rat recovery lost vitals")?;
        client.take_events();
        if vitals.health == 0 {
            return Err("rat recovery observed player death".into());
        }
        if !client_world::rat_recovery_clear(client)? || vitals.health < previous {
            client_world::rat_retreat(client, world).await?;
            client_world::rat_recovery_edge(client, world).await?;
        }
        if vitals.health > previous {
            evidence(&format!(
                "rat_care phase=healing before={previous} after={} maximum={}",
                vitals.health, vitals.max_health
            ));
        }
        previous = vitals.health;
        if ready_to_fight(vitals.health, vitals.max_health) {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            client_world::rat_retreat(client, world).await?;
            return Err(format!("rat recovery budget exhausted safely in PZ; initial={} observed={} maximum={}; no HP override",before.health,vitals.health,vitals.max_health).into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn qualification_nbroute_care_requires_full_health() {
        assert!(!ready_to_fight(149, 150));
        assert!(ready_to_fight(150, 150));
        assert!(!ready_to_fight(0, 150));
    }
    #[test]
    fn qualification_nbroute_care_preserves_retreat_reserve_and_refuses_a_pack() {
        assert!(should_retreat(112, 150, 0));
        assert!(!should_retreat(113, 150, 0));
        assert!(should_retreat(150, 150, 1));
    }
}
