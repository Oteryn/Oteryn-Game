//! Hunt through network attack, visibility, corpse USE and item-move replies.
use super::*;
pub(super) async fn hunt(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    hunt_creature(client, None).await
}

/// Hunts an adjacent creature; with `creature` (a content key) only that creature.
pub(super) async fn hunt_creature(
    client: &mut oteryn_dev_client::DevClientSession,
    creature: Option<&str>,
) -> TestResult<()> {
    use oteryn_dev_client::{EntityDetail, EntityKind};
    if !client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1)
    {
        evidence("mechanic=hunt skipped=entry_room_has_no_native_combat_and_loot_content");
        return Ok(());
    }
    let position = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == EntityKind::Creature
                    && entry.position.floor == position.floor
                    && entry
                        .position
                        .x
                        .abs_diff(position.x)
                        .max(entry.position.y.abs_diff(position.y))
                        <= 1
                    && creature.is_none_or(|key| super::client_world::is_spawn_creature(entry, key))
            })
        })
        .copied()
        .ok_or("hunt route has no visible monster")?;
    // A corpse that was already on the ground is not this kill's corpse.
    let old_corpses: Vec<_> = client
        .world_entities()
        .map(|entities| {
            entities
                .iter()
                .filter(|entry| entry.kind == EntityKind::Corpse)
                .map(|entry| entry.entity)
                .collect()
        })
        .unwrap_or_default();
    // A real client hunts with chase on: the rat flees at low health and a standing attacker
    // loses it out of melee range.
    let modes = oteryn_protocol_oteryn::attack::FightModes {
        fight_mode: oteryn_protocol_oteryn::attack::FightMode::Offensive,
        chase: oteryn_protocol_oteryn::attack::ChaseMode::Chase,
        secure: true,
    };
    let chase = client.session_mut().set_fight_modes(&modes).await?;
    if chase.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(format!("hunt chase mode: {chase:?}").into());
    }
    let attack = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    eprintln!(
        "[DBG-hunt] attack={attack:?} kind={:?} name={:?} at={:?} me={position:?}",
        target.kind,
        target
            .display_name
            .as_ref()
            .map(|n| n.as_str().map(str::to_owned)),
        target.position
    );
    if attack.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(format!("hunt attack: {attack:?}").into());
    }
    eprintln!(
        "[DBG-hunt] equip={:?} vitals={:?}",
        client.inventory().map(|i| i
            .equipment
            .iter()
            .map(|e| format!("{e:?}").chars().take(160).collect::<String>())
            .collect::<Vec<_>>()),
        client.actor_vitals()
    ); // [DBG-hunt]
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    // This existing combat observation budget can exceed the row's idle
    // watchdog. Preserve it without extending the hunt or outer row deadline.
    client_world::hunt_observation_wait(deadline);
    let mut damaged = false;
    let mut last_position = target.position;
    let mut dbg_tick = tokio::time::Instant::now(); // [DBG-hunt]
    let corpse = loop {
        client.service_liveness(Duration::from_millis(100)).await?;
        let me = client.world_spatial().actor_position;
        if me.floor != position.floor
            || me.x.abs_diff(position.x).max(me.y.abs_diff(position.y)) > 50
        {
            let seen: Vec<String> = client
                .world_entities()
                .map(|e| {
                    e.iter()
                        .filter(|x| x.kind == EntityKind::Creature)
                        .map(|x| {
                            format!(
                                "{:?}@{},{},{}",
                                x.display_name
                                    .as_ref()
                                    .map(|n| n.as_str().map(str::to_owned)),
                                x.position.x,
                                x.position.y,
                                x.position.floor
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            eprintln!("[DBG-hunt] died damaged={damaged} last={last_position:?} seen={seen:?}");
            return Err(
                format!("hunt character died or was moved away before the kill: {me:?}").into(),
            );
        }
        if !client
            .world_entities()
            .is_some_and(|e| e.get(&target.entity).is_some())
        {
            let corpses: Vec<_> = client
                .world_entities()
                .map(|e| {
                    e.iter()
                        .filter(|x| x.kind == EntityKind::Corpse)
                        .map(|x| x.position)
                        .collect()
                })
                .unwrap_or_default();
            eprintln!(
                "[DBG-hunt] target gone last={last_position:?} me={me:?} corpses={corpses:?}"
            );
        }
        for event in client.take_events() {
            if let oteryn_dev_client::SessionEvent::WorldSpatialEntities(delta) = event {
                if let Some(updated) = delta
                    .value
                    .update
                    .iter()
                    .find(|entry| entry.entity == target.entity)
                {
                    last_position = updated.position;
                }
                damaged |= delta.value.update.iter().any(|entry| entry.entity == target.entity && matches!(entry.detail, EntityDetail::Actor { health_percent, .. } if health_percent < 100));
            }
        }
        // A replacement snapshot carries the target too, not only update deltas.
        let current = client
            .world_entities()
            .and_then(|entities| entities.get(&target.entity))
            .copied();
        if dbg_tick.elapsed() >= Duration::from_secs(3) {
            dbg_tick = tokio::time::Instant::now(); // [DBG-hunt]
            eprintln!(
                "[DBG-hunt] tick me={me:?} hp={:?} near={:?} target={:?}",
                client
                    .actor_vitals()
                    .map(|v| (v.health, v.max_health, v.mana, v.max_mana)),
                client.world_entities().map(|e| e
                    .iter()
                    .filter(|x| x.kind != EntityKind::Player
                        && x.position.floor == me.floor
                        && x.position.x.abs_diff(me.x).max(x.position.y.abs_diff(me.y)) <= 3)
                    .map(|x| format!(
                        "{:?}/{:?}@{},{}",
                        x.kind,
                        x.display_name
                            .as_ref()
                            .map(|n| n.as_str().map(str::to_owned)),
                        x.position.x,
                        x.position.y
                    ))
                    .collect::<Vec<_>>()),
                current.map(|c| (
                    c.position,
                    match c.detail {
                        EntityDetail::Actor { health_percent, .. } => Some(health_percent),
                        _ => None,
                    }
                ))
            ); // [DBG-hunt]
        } // [DBG-hunt]
        if let Some(current) = current {
            last_position = current.position;
            damaged |= matches!(current.detail, EntityDetail::Actor { health_percent, .. } if health_percent < 100);
        }
        if let Some(corpse) = client
            .world_entities()
            .and_then(|entities| {
                entities.iter().find(|entry| {
                    entry.kind == EntityKind::Corpse
                        && entry.position == last_position
                        && !old_corpses.contains(&entry.entity)
                })
            })
            .copied()
            .filter(|_| current.is_none())
        {
            break corpse;
        }
        if tokio::time::Instant::now() >= deadline {
            let me = client.world_spatial().actor_position;
            let seen: Vec<String> = client
                .world_entities()
                .map(|e| {
                    e.iter()
                        .filter(|x| x.kind != EntityKind::Player)
                        .map(|x| {
                            format!(
                                "{:?}@{},{},{}",
                                x.kind, x.position.x, x.position.y, x.position.floor
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            let still = client
                .world_entities()
                .is_some_and(|e| e.get(&target.entity).is_some());
            eprintln!(
                "[DBG-hunt] damaged={damaged} me={me:?} start={:?} last={last_position:?} still={still:?} seen={seen:?}",
                target.position
            );
            return Err("hunt produced no corpse before deadline".into());
        }
    };
    if !damaged
        || client
            .world_entities()
            .is_some_and(|entities| entities.get(&target.entity).is_some())
    {
        return Err("hunt did not observe damage and target removal".into());
    }
    loot_corpse(client, target, corpse).await
}

async fn loot_corpse(
    client: &mut oteryn_dev_client::DevClientSession,
    target: oteryn_protocol_oteryn::world_spatial_entities::WorldSpatialEntity,
    corpse: oteryn_protocol_oteryn::world_spatial_entities::WorldSpatialEntity,
) -> TestResult {
    use oteryn_dev_client::EntityDetail;
    let EntityDetail::Object {
        item_handle: Some(handle),
        ..
    } = corpse.detail
    else {
        return Err("hunt corpse has no server-issued item handle".into());
    };
    let used = client.use_item(handle).await?;
    if used.status != CommandStatus::Accepted
        || used.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    {
        return Err(format!(
            "hunt corpse USE refused status={:?} disposition={:?} handle={handle:?} type={:?}",
            used.status, used.disposition, corpse.detail
        )
        .into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while client
        .open_container()
        .is_none_or(|view| view.container_handle != Some(handle))
    {
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
        if tokio::time::Instant::now() >= deadline {
            return Err("hunt corpse container delta missing".into());
        }
    }
    let entries = client
        .open_container()
        .ok_or("corpse view")?
        .entries
        .clone();
    let mut picked = 0;
    if let Some(item) = entries.first() {
        let before = client
            .inventory()
            .ok_or("hunt backpack")?
            .entries
            .iter()
            .filter(|entry| entry.item_definition_ref == item.item_definition_ref)
            .map(|entry| u64::from(entry.count.get()))
            .sum::<u64>();
        let corpse_before = entries
            .iter()
            .filter(|entry| entry.item_definition_ref == item.item_definition_ref)
            .map(|entry| u64::from(entry.count.get()))
            .sum::<u64>();
        let moved = client
            .move_item(&oteryn_dev_client::ItemMoveIntent {
                source: item.handle,
                destination: oteryn_dev_client::ItemMoveDestination::MainBackpack,
            })
            .await?;
        if moved.outcome != oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved {
            return Err(format!("hunt loot move: {moved:?}").into());
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            let after = client
                .inventory()
                .ok_or("hunt inventory delta")?
                .entries
                .iter()
                .filter(|entry| entry.item_definition_ref == item.item_definition_ref)
                .map(|entry| u64::from(entry.count.get()))
                .sum::<u64>();
            let corpse_after = client
                .open_container()
                .ok_or("loot corpse delta")?
                .entries
                .iter()
                .filter(|entry| entry.item_definition_ref == item.item_definition_ref)
                .map(|entry| u64::from(entry.count.get()))
                .sum::<u64>();
            if after == before + u64::from(item.count.get())
                && corpse_after + u64::from(item.count.get()) == corpse_before
            {
                picked = item.count.get();
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err("hunt loot inventory conservation failed".into());
            }
            client.service_liveness(Duration::from_millis(50)).await?;
            client.take_events();
        }
    }
    evidence(&format!(
        "mechanic=hunt state=killed target={:?} corpse={:?} loot_entries={} picked={picked}",
        target.entity,
        corpse.entity,
        entries.len()
    ));
    Ok(())
}

#[path = "rat_hunt.rs"]
mod rat;
pub(super) use rat::{hunt_rat, recover_rat_health};
