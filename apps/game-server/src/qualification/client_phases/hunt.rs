//! Hunt through network attack, visibility, corpse USE and item-move replies.
use super::*;
use std::collections::BTreeSet;
pub(super) async fn hunt(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    hunt_named(client, None).await
}

pub(super) async fn hunt_named(
    client: &mut oteryn_dev_client::DevClientSession,
    name: Option<&str>,
) -> TestResult {
    use oteryn_dev_client::{EntityDetail, EntityKind};
    if !client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1)
    {
        if let Some(name) = name {
            return Err(format!("hunt for {name} needs native combat and loot").into());
        }
        evidence("mechanic=hunt skipped=entry_room_has_no_native_combat_and_loot_content");
        return Ok(());
    }
    let position = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == EntityKind::Creature
                    && matches_target_name(
                        name,
                        entry
                            .display_name
                            .as_ref()
                            .and_then(|label| label.as_str().ok()),
                    )
                    && entry.position.floor == position.floor
                    && entry
                        .position
                        .x
                        .abs_diff(position.x)
                        .max(entry.position.y.abs_diff(position.y))
                        <= 1
            })
        })
        .copied()
        .ok_or_else(|| format!("hunt route has no visible target matching {name:?}"))?;
    let prior_corpses: BTreeSet<_> = client
        .world_entities()
        .into_iter()
        .flat_map(|entities| entities.iter())
        .filter(|entity| entity.kind == EntityKind::Corpse)
        .map(|entity| entity.entity)
        .collect();
    let attack = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    if attack.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(format!("hunt attack: {attack:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let mut damaged = false;
    let mut last_position = target.position;
    let corpse = loop {
        client.service_liveness(Duration::from_millis(100)).await?;
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
        if let Some(corpse) = client
            .world_entities()
            .and_then(|entities| {
                entities.iter().find(|entry| {
                    entry.kind == EntityKind::Corpse
                        && entry.position == last_position
                        && (name.is_none() || !prior_corpses.contains(&entry.entity))
                })
            })
            .copied()
        {
            break corpse;
        }
        if tokio::time::Instant::now() >= deadline {
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
        return Err("hunt corpse USE refused".into());
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

fn matches_target_name(wanted: Option<&str>, observed: Option<&str>) -> bool {
    wanted.is_none_or(|name| observed.is_some_and(|label| label.eq_ignore_ascii_case(name)))
}

#[test]
fn a_nearby_other_creature_cannot_count_as_court_warlock() {
    assert!(matches_target_name(
        Some("Court Warlock"),
        Some("Court Warlock")
    ));
    assert!(!matches_target_name(Some("Court Warlock"), Some("Rat")));
    assert!(!matches_target_name(Some("Court Warlock"), None));
    assert!(matches_target_name(None, Some("Rat")));
}
