//! Hunt through network attack, visibility, corpse USE and item-move replies.
use super::*;
pub(super) async fn hunt(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<()> {
    hunt_with_corpse_stage(client, None, None).await
}

#[derive(Clone, Copy)]
pub(super) struct CorpseStage {
    pub fresh_ref: u32,
    pub mature_ref: u32,
}

fn corpse_definition(entity: &oteryn_dev_client::WorldSpatialEntity) -> Option<u32> {
    match entity.detail {
        oteryn_dev_client::EntityDetail::Object {
            item_definition_ref,
            ..
        }
        | oteryn_dev_client::EntityDetail::ObjectWithAppearance {
            item_definition_ref,
            ..
        } => Some(item_definition_ref),
        _ => None,
    }
}

pub(super) async fn wait_for_corpse_stage(
    client: &mut oteryn_dev_client::DevClientSession,
    initial: oteryn_dev_client::WorldSpatialEntity,
    stage: CorpseStage,
) -> TestResult<oteryn_dev_client::WorldSpatialEntity> {
    if corpse_definition(&initial) != Some(stage.fresh_ref) {
        return Err(format!(
            "fresh corpse did not have expected definition {}: {initial:?}",
            stage.fresh_ref
        )
        .into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(45);
    loop {
        let current = client
            .world_entities()
            .and_then(|entities| entities.get(&initial.entity))
            .filter(|entity| {
                entity.position == initial.position
                    && entity.kind == oteryn_dev_client::EntityKind::Corpse
            })
            .copied();
        let sequence = client.last_server_sequence();
        match current.and_then(|entity| corpse_definition(&entity).map(|reference| (reference, entity))) {
            Some((reference, corpse)) if reference == stage.mature_ref => {
                evidence(&format!("P01_CHECK rat-corpse-decay PASS server_sequence={sequence}"));
                return Ok(corpse);
            }
            Some((reference, _)) if reference == stage.fresh_ref => {}
            _ => return Err(format!(
                "Rat corpse disappeared or changed to the wrong stage: {current:?} server_sequence={sequence}"
            ).into()),
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "Rat corpse did not reach the expected decay stage: {current:?} server_sequence={sequence}"
            ).into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
    }
}

#[expect(
    dead_code,
    reason = "retained for qualification lanes; P01 now verifies the committed death receipt"
)]
pub(super) async fn hunt_named_target(
    client: &mut oteryn_dev_client::DevClientSession,
    target: oteryn_dev_client::WorldSpatialEntity,
    stage: CorpseStage,
) -> TestResult {
    hunt_with_corpse_stage(client, Some(target), Some(stage)).await
}

async fn hunt_with_corpse_stage(
    client: &mut oteryn_dev_client::DevClientSession,
    target: Option<oteryn_dev_client::WorldSpatialEntity>,
    stage: Option<CorpseStage>,
) -> TestResult {
    use oteryn_dev_client::{EntityDetail, EntityKind};
    if !client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1)
    {
        if stage.is_some() {
            return Err("Rat hunt did not negotiate native world map capability".into());
        }
        evidence("mechanic=hunt skipped=entry_room_has_no_native_combat_and_loot_content");
        return Ok(());
    }
    let position = client.world_spatial().actor_position;
    let target = target
        .or_else(|| {
            client
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
        })
        .ok_or("hunt route has no visible monster")?;
    let mut old_corpses = Vec::new();
    if stage.is_some() {
        old_corpses.extend(client.world_entities().into_iter().flat_map(|entities| {
            entities
                .iter()
                .filter(|entity| entity.kind == EntityKind::Corpse)
                .map(|entity| entity.entity)
        }));
    }
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
                        && !old_corpses.contains(&entry.entity)
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
    let detail = match stage {
        Some(stage) => wait_for_corpse_stage(client, corpse, stage).await?.detail,
        None => corpse.detail,
    };
    let handle = match detail {
        EntityDetail::Object { item_handle, .. } => item_handle,
        EntityDetail::ObjectWithAppearance { item_handle, .. } if stage.is_some() => item_handle,
        _ => None,
    }
    .ok_or("hunt corpse has no server-issued item handle")?;
    let used = if stage.is_some() {
        use_rat_corpse(client, handle).await?
    } else {
        client.use_item(handle).await?
    };
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

pub(super) async fn use_rat_corpse(
    client: &mut oteryn_dev_client::DevClientSession,
    handle: oteryn_dev_client::ItemHandle,
) -> TestResult<oteryn_dev_client::UseOutcome> {
    let command_id = client.next_command_id();
    let sequence = client.last_server_sequence();
    let result = client.use_item(handle).await;
    match result {
        Ok(reply) => {
            if reply.status != CommandStatus::Accepted
                || reply.disposition
                    != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
            {
                evidence(&format!(
                    "P01_CORPSE_USE command_id={command_id} before_sequence={sequence} after_sequence={} reply={reply:?}",
                    client.last_server_sequence()
                ));
            }
            Ok(reply)
        }
        Err(error) => {
            evidence(&format!(
                "P01_CORPSE_USE command_id={command_id} before_sequence={sequence} after_sequence={} error={error}",
                client.last_server_sequence()
            ));
            Err(error.into())
        }
    }
}
