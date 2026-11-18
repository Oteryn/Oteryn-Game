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
    use oteryn_dev_client::EntityKind;
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
    let mut observed = HuntEvidence {
        damaged: false,
        last_position: target.position,
        removed: false,
        prior_corpses,
        corpses_during_absence: BTreeSet::new(),
    };
    let corpse = loop {
        client.service_liveness(Duration::from_millis(100)).await?;
        for event in client.take_events() {
            if let oteryn_dev_client::SessionEvent::WorldSpatialEntities(delta) = event {
                observed.apply(target.entity, &delta.value);
            }
        }
        if name.is_some()
            && let Some(entities) = client.world_entities()
            && let Some(corpse) = observed.corpse(entities.iter(), target.entity)
        {
            break corpse;
        }
        if name.is_none()
            && let Some(corpse) = client
                .world_entities()
                .and_then(|entities| {
                    entities.iter().find(|entry| {
                        entry.kind == EntityKind::Corpse && entry.position == observed.last_position
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
    if !observed.damaged
        || client
            .world_entities()
            .is_some_and(|entities| entities.get(&target.entity).is_some())
    {
        return Err("hunt did not observe damage and target removal".into());
    }
    let handle =
        corpse_handle(corpse.detail).ok_or("hunt corpse has no server-issued item handle")?;
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

fn corpse_handle(detail: oteryn_dev_client::EntityDetail) -> Option<oteryn_dev_client::ItemHandle> {
    match detail {
        oteryn_dev_client::EntityDetail::Object { item_handle, .. }
        | oteryn_dev_client::EntityDetail::ObjectWithAppearance { item_handle, .. } => item_handle,
        _ => None,
    }
}

#[test]
fn round2_appearance_corpse_retains_its_item_handle() {
    use oteryn_dev_client::{EntityDetail, ItemHandle};
    assert_eq!(
        corpse_handle(EntityDetail::ObjectWithAppearance {
            item_definition_ref: 1,
            quantity: 1,
            item_handle: Some(ItemHandle::MIN),
            appearance_id: std::num::NonZeroU16::MIN,
        }),
        Some(ItemHandle::MIN)
    );
}

struct HuntEvidence {
    damaged: bool,
    last_position: oteryn_protocol_oteryn::world_spatial::ActorPosition,
    removed: bool,
    prior_corpses: BTreeSet<oteryn_dev_client::EntityRef>,
    corpses_during_absence: BTreeSet<oteryn_dev_client::EntityRef>,
}

impl HuntEvidence {
    fn apply(
        &mut self,
        target: oteryn_dev_client::EntityRef,
        delta: &oteryn_dev_client::WorldSpatialEntitiesDelta,
    ) {
        use oteryn_dev_client::{EntityDetail, EntityKind};
        if let Some(actor) = delta
            .update
            .iter()
            .chain(&delta.enter)
            .find(|entity| entity.entity == target)
        {
            // A live return invalidates corpses that entered during the previous absence.
            self.prior_corpses.append(&mut self.corpses_during_absence);
            self.last_position = actor.position;
            self.removed = false;
            self.damaged |= matches!(actor.detail, EntityDetail::Actor { health_percent, .. }
                | EntityDetail::ActorWithIcons { health_percent, .. } if health_percent < 100);
        }
        self.removed |= delta.leave.contains(&target);
        let corpses = if self.removed {
            &mut self.corpses_during_absence
        } else {
            &mut self.prior_corpses
        };
        corpses.extend(
            delta
                .enter
                .iter()
                .filter(|entry| entry.kind == EntityKind::Corpse)
                .map(|entry| entry.entity),
        );
    }

    fn corpse<'a>(
        &self,
        entities: impl Iterator<Item = &'a oteryn_dev_client::WorldSpatialEntity>,
        target: oteryn_dev_client::EntityRef,
    ) -> Option<oteryn_dev_client::WorldSpatialEntity> {
        if !self.removed || !self.damaged {
            return None;
        }
        named_corpse(entities, target, self.last_position, &self.prior_corpses)
    }
}

fn named_corpse<'a>(
    entities: impl Iterator<Item = &'a oteryn_dev_client::WorldSpatialEntity>,
    target: oteryn_dev_client::EntityRef,
    last_position: oteryn_protocol_oteryn::world_spatial::ActorPosition,
    prior_corpses: &BTreeSet<oteryn_dev_client::EntityRef>,
) -> Option<oteryn_dev_client::WorldSpatialEntity> {
    use oteryn_dev_client::EntityKind;
    let mut found = None;
    for entry in entities {
        if entry.entity == target {
            return None;
        }
        if entry.kind == EntityKind::Corpse
            && entry.position == last_position
            && !prior_corpses.contains(&entry.entity)
            && corpse_handle(entry.detail).is_some()
            && found.replace(*entry).is_some()
        {
            return None;
        }
    }
    found
}

#[test]
fn round5_live_warlock_reentry_excludes_corpse_from_separate_delta() {
    use oteryn_dev_client::{EntityDetail, EntityKind, EntityRef, ItemHandle, WorldSpatialEntity};
    use oteryn_protocol_oteryn::world_spatial::ActorPosition;
    let at = ActorPosition {
        x: 20,
        y: 30,
        floor: 7,
    };
    let id = EntityRef {
        identity: [1; 16],
        generation: 1,
    };
    let actor = WorldSpatialEntity {
        display_name: None,
        entity: id,
        kind: EntityKind::Creature,
        position: at,
        detail: EntityDetail::Actor {
            direction: StepDirection::North,
            appearance_ref: 1,
            health_percent: 35,
        },
    };
    let corpse = WorldSpatialEntity {
        display_name: None,
        entity: EntityRef {
            identity: [2; 16],
            generation: 1,
        },
        kind: EntityKind::Corpse,
        position: at,
        detail: EntityDetail::Object {
            item_definition_ref: 1,
            quantity: 1,
            item_handle: Some(ItemHandle::MIN),
        },
    };
    let mut observed = HuntEvidence {
        damaged: false,
        last_position: at,
        removed: false,
        prior_corpses: BTreeSet::new(),
        corpses_during_absence: BTreeSet::new(),
    };
    let mut delta = oteryn_dev_client::WorldSpatialEntitiesDelta {
        content_generation: [0; 32],
        actor_position: at,
        enter: vec![],
        update: vec![],
        leave: vec![id],
    };
    observed.apply(id, &delta);
    delta.leave.clear();
    delta.enter = vec![corpse];
    observed.apply(id, &delta);
    delta.enter = vec![actor];
    observed.apply(id, &delta);
    assert_eq!(observed.corpse([actor, corpse].iter(), id), None);
    delta.enter.clear();
    delta.leave.push(id);
    observed.apply(id, &delta);
    assert_eq!(observed.corpse([corpse].iter(), id), None);
    let final_corpse = WorldSpatialEntity {
        entity: EntityRef {
            identity: [3; 16],
            generation: 1,
        },
        ..corpse
    };
    delta.leave.clear();
    delta.enter.push(final_corpse);
    observed.apply(id, &delta);
    assert_eq!(
        observed.corpse([corpse, final_corpse].iter(), id),
        Some(final_corpse)
    );
}
