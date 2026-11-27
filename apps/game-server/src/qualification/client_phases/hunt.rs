//! Hunt through network attack, visibility, corpse USE and item-move replies.
use super::*;
pub(super) async fn hunt(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    use oteryn_dev_client::EntityKind;
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
            })
        })
        .copied()
        .ok_or("hunt route has no visible monster")?;
    hunt_target(client, target, None).await
}

pub(super) async fn hunt_target(
    client: &mut oteryn_dev_client::DevClientSession,
    target: oteryn_dev_client::WorldSpatialEntity,
    bounded_corpse_wait: Option<Duration>,
) -> TestResult<()> {
    use oteryn_dev_client::{EntityDetail, EntityKind};

    let previous_corpses = bounded_corpse_wait.map(|_| {
        client
            .world_entities()
            .into_iter()
            .flat_map(|view| view.iter())
            .filter(|entry| entry.kind == EntityKind::Corpse)
            .map(|entry| entry.entity)
            .collect::<std::collections::BTreeSet<_>>()
    });
    let started = tokio::time::Instant::now();
    let mut last_health = None;
    if bounded_corpse_wait.is_some()
        && let EntityDetail::Actor { health_percent, .. } = target.detail
    {
        last_health = Some(health_percent);
        evidence(&format!(
            "W13_HUNT_HP target={:?} elapsed_ms=0 hp_percent={health_percent}",
            target.entity
        ));
    }
    let attack = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    if attack.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(format!("hunt attack: {attack:?}").into());
    }
    let deadline =
        tokio::time::Instant::now() + bounded_corpse_wait.unwrap_or(Duration::from_secs(120));
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
                    if bounded_corpse_wait.is_some()
                        && let EntityDetail::Actor { health_percent, .. } = updated.detail
                        && last_health != Some(health_percent)
                    {
                        last_health = Some(health_percent);
                        evidence(&format!(
                            "W13_HUNT_HP target={:?} elapsed_ms={} hp_percent={health_percent}",
                            target.entity,
                            started.elapsed().as_millis()
                        ));
                    }
                }
                damaged |= delta.value.update.iter().any(|entry| entry.entity == target.entity && matches!(entry.detail, EntityDetail::Actor { health_percent, .. } if health_percent < 100));
            }
        }
        if bounded_corpse_wait.is_some()
            && let Some(live_target) = client
                .world_entities()
                .and_then(|entities| entities.get(&target.entity))
        {
            last_position = live_target.position;
        }
        if let Some(corpse) = client
            .world_entities()
            .and_then(|entities| {
                entities.iter().find(|entry| {
                    entry.kind == EntityKind::Corpse
                        && entry.position == last_position
                        && previous_corpses
                            .as_ref()
                            .is_none_or(|previous| !previous.contains(&entry.entity))
                })
            })
            .copied()
        {
            break corpse;
        }
        if tokio::time::Instant::now() >= deadline {
            if bounded_corpse_wait.is_none() {
                return Err("hunt produced no corpse before deadline".into());
            }
            let observed = client
                .world_entities()
                .and_then(|entities| entities.get(&target.entity));
            return Err(format!(
                "hunt deadline waiting for target removal and corpse: target={:?} last_position={last_position:?} damaged={damaged} observed={observed:?}",
                target.entity
            ).into());
        }
    };
    if !damaged
        || client
            .world_entities()
            .is_some_and(|entities| entities.get(&target.entity).is_some())
    {
        return Err("hunt did not observe damage and target removal".into());
    }
    let (variant, handle) = corpse_item_handle(corpse.detail);
    evidence(&format!(
        "hunt corpse entity={:?} detail={variant}",
        corpse.entity
    ));
    let handle = handle.ok_or("hunt corpse has no server-issued item handle")?;
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

fn corpse_item_handle(
    detail: oteryn_dev_client::EntityDetail,
) -> (
    &'static str,
    Option<oteryn_protocol_oteryn::item_view::ItemHandle>,
) {
    use oteryn_dev_client::EntityDetail;
    match detail {
        EntityDetail::Object { item_handle, .. } => ("Object", item_handle),
        EntityDetail::ObjectWithAppearance { item_handle, .. } => {
            ("ObjectWithAppearance", item_handle)
        }
        _ => ("non-object", None),
    }
}

#[test]
fn hunt_corpse_handle_accepts_both_wire_variants() {
    use oteryn_dev_client::EntityDetail;
    let handle = oteryn_protocol_oteryn::item_view::ItemHandle::new(17);
    for item_handle in [handle, None] {
        assert_eq!(
            corpse_item_handle(EntityDetail::Object {
                item_definition_ref: 42,
                quantity: 1,
                item_handle
            }),
            ("Object", item_handle)
        );
        assert_eq!(
            corpse_item_handle(EntityDetail::ObjectWithAppearance {
                item_definition_ref: 42,
                quantity: 1,
                item_handle,
                appearance_id: std::num::NonZeroU16::MIN,
            }),
            ("ObjectWithAppearance", item_handle)
        );
    }
}
