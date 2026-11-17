/// Keep failure evidence tied to observed combat, rather than an assumed corpse policy.
pub(super) fn newhaven_hunt_cause(health: u32, at_temple: bool, killed: bool) -> &'static str {
    if health == 0 || at_temple {
        "died"
    } else if killed {
        "kill_observed_no_corpse"
    } else {
        "no_kill"
    }
}

#[test]
fn qualification_newhaven_hunt_classifies_death_before_corpse_failure() {
    assert_eq!(newhaven_hunt_cause(0, false, false), "died");
    assert_eq!(newhaven_hunt_cause(150, true, true), "died");
    assert_eq!(newhaven_hunt_cause(70, false, false), "no_kill");
    assert_eq!(
        newhaven_hunt_cause(70, false, true),
        "kill_observed_no_corpse"
    );
}

fn newhaven_at_temple(client: &oteryn_dev_client::DevClientSession) -> bool {
    tile(client.world_spatial().actor_position)
        .ok()
        .is_some_and(|at| {
            at.floor == NEWHAVEN_ISLET_TEMPLE.floor
                && at
                    .x
                    .abs_diff(NEWHAVEN_ISLET_TEMPLE.x)
                    .max(at.y.abs_diff(NEWHAVEN_ISLET_TEMPLE.y))
                    <= 2
        })
}

pub(super) fn newhaven_hunt_died(client: &oteryn_dev_client::DevClientSession) -> bool {
    client
        .actor_vitals()
        .is_some_and(|vitals| vitals.health == 0)
        || newhaven_at_temple(client)
}

async fn newhaven_wait_for_respawn(client: &mut oteryn_dev_client::DevClientSession) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    loop {
        if newhaven_at_temple(client)
            && client
                .actor_vitals()
                .is_some_and(|v| v.health > 0 && v.health == v.max_health)
        {
            client.take_events();
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("Newhaven hunt death did not respawn alive on the islet".into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
    }
}

async fn newhaven_equip_weapon(client: &mut oteryn_dev_client::DevClientSession) -> TestResult {
    use crate::content::{ReferenceEquipmentSlot, ReferenceItemField};
    let input = native_input()?;
    let index = provisioning::item_index()?;
    let profiles: crate::content::native_gameplay::ItemProfilesDocument = serde_json::from_slice(
        &input
            .item_profiles
            .as_ref()
            .ok_or("Newhaven Item profiles absent")?
            .bytes,
    )?;
    let is_weapon = |reference| {
        index.definition_at(reference).is_some_and(|(key, _)| {
            profiles.records.iter().any(|record| {
                record.production_definition.production_key == key
                    && matches!(&record.semantics.equipment, ReferenceItemField::Known(equipment)
                        if matches!(&equipment.patterns, ReferenceItemField::Known(patterns)
                            if patterns.iter().any(|pattern| pattern.primary_slot == ReferenceItemField::Known(ReferenceEquipmentSlot::Weapon))))
            })
        })
    };
    let inventory = client.inventory().ok_or("Newhaven inventory absent")?;
    // An already worn match must not be hotkey-toggled back into the backpack.
    if inventory
        .equipment
        .iter()
        .any(|entry| is_weapon(entry.item.item_definition_ref))
    {
        return Ok(());
    }
    let reference = inventory.entries.iter().find(|entry| is_weapon(entry.item_definition_ref))
        .map(|entry| entry.item_definition_ref)
        .ok_or("Newhaven no-vocation starter has no carried or equipped weapon; normal vocation kits exclude none")?;
    let reply = client
        .session_mut()
        .equip_item_definition(reference)
        .await?;
    if reply.status != CommandStatus::Accepted
        || reply.outcome != oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved
    {
        return Err(format!("Newhaven starter weapon equip refused: {reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !client.inventory().is_some_and(|inventory| {
        inventory
            .equipment
            .iter()
            .any(|entry| entry.item.item_definition_ref == reference)
    }) {
        if tokio::time::Instant::now() >= deadline {
            return Err("Newhaven starter weapon equipment delta missing".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    Ok(())
}

pub(super) fn newhaven_adjacent_creatures(client: &oteryn_dev_client::DevClientSession) -> usize {
    let here = client.world_spatial().actor_position;
    client.world_entities().map_or(0, |entities| {
        entities
            .iter()
            .filter(|entry| {
                entry.kind == oteryn_dev_client::EntityKind::Creature
                    && entry.position.floor == here.floor
                    && entry
                        .position
                        .x
                        .abs_diff(here.x)
                        .max(entry.position.y.abs_diff(here.y))
                        <= 1
            })
            .count()
    })
}

pub(super) async fn newhaven_retreat(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
) -> TestResult {
    client.session_mut().attack_target(None).await?;
    // Move to protection, rather than waiting for regeneration under melee attacks.
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| tile.protection_zone())
    })
    .ok_or("Newhaven retreat has no reachable protection zone")?;
    for direction in path {
        if newhaven_hunt_died(client) {
            return Err("newhaven hunt died".into());
        }
        if let Err(error) = walk(client, world, &[direction]).await {
            if newhaven_hunt_died(client) {
                return Err("newhaven hunt died".into());
            }
            return Err(error);
        }
    }
    let index = provisioning::item_index()?;
    // Use only actual starter supplies. Health potion266 and food3578 are in
    // the Crystal Newhaven kit (starter_grant.rs:137); no fixture is provisioned.
    for id in [266, 3578] {
        if id == 266
            && client
                .actor_vitals()
                .is_some_and(|v| v.health == v.max_health)
        {
            continue;
        }
        let reference = provisioning::item_reference(&index, id)?;
        let handle = client.inventory().and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|entry| entry.item_definition_ref == reference)
                .map(|entry| entry.handle)
        });
        if let Some(handle) = handle {
            let reply = client.use_item(handle).await?;
            if reply.status != CommandStatus::Accepted
                || reply.disposition
                    != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
            {
                return Err(format!("Newhaven starter supply USE refused: {reply:?}").into());
            }
        }
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    while client
        .actor_vitals()
        .is_none_or(|v| u64::from(v.health) * 5 < u64::from(v.max_health) * 4)
    {
        if newhaven_hunt_died(client) {
            return Err("newhaven hunt died".into());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("Newhaven recovery did not reach 80 percent health in protection".into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
    }
    Ok(())
}
