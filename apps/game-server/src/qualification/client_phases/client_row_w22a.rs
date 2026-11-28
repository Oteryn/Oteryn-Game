//! Capture the live open-container state deltas for W22a session replay.
use super::*;
use oteryn_dev_client::{
    DevClientSession, EntityDetail, EntityKind, ItemMoveDestination, SessionEvent,
};
use oteryn_protocol_oteryn::item_view::{self, ItemEntry, ItemMoveOutcome, OpenContainer};
use std::io::Write;

fn capture(root: &Path, name: &str, bytes: &[u8]) -> TestResult {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(name))?
        .write_all(bytes)?;
    Ok(())
}

fn rat_ranges() -> TestResult<BTreeMap<u32, (u32, u32)>> {
    let index = provisioning::item_index()?;
    [("oteryn:item.tibia.i3607", 1, 1), ("oteryn:item.tibia.i3031", 1, 4)]
        .into_iter()
        .map(|(key, min, max)| {
            let revision = index.revision(key).ok_or("step=table rat item revision missing")?;
            let reference = index
                .definition_ref("Item", key, revision)
                .ok_or("step=table rat item reference missing")?;
            Ok((reference.get(), (min, max)))
        })
        .collect()
}

fn listed(entries: &[ItemEntry]) -> String {
    entries
        .iter()
        .map(|entry| format!("{}:{}", entry.item_definition_ref, entry.count))
        .collect::<Vec<_>>()
        .join(";")
}

async fn kill_and_open(
    client: &mut DevClientSession,
    selected: oteryn_dev_client::EntityRef,
) -> TestResult<OpenContainer> {
    let position = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == EntityKind::Creature
                    && entry.entity == selected
                    && entry.display_name.is_some_and(|name| name.as_str().is_ok_and(|name| name == "Rat"))
                    && entry.position.floor == position.floor
                    && entry.position.x.abs_diff(position.x).max(entry.position.y.abs_diff(position.y)) <= 1
            })
        })
        .copied()
        .ok_or("step=attack no adjacent rat after the walk")?;
    let old_corpses: std::collections::BTreeSet<_> = client
        .world_entities()
        .into_iter()
        .flat_map(|entities| entities.iter())
        .filter(|entry| entry.kind == EntityKind::Corpse)
        .map(|entry| entry.entity)
        .collect();
    let reply = client.session_mut().attack_target(Some(&target.entity)).await?;
    if reply.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(format!("step=attack refused {reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    let mut last_position = target.position;
    let corpse = loop {
        client.service_liveness(Duration::from_millis(100)).await?;
        for event in client.take_events() {
            if let SessionEvent::WorldSpatialEntities(delta) = event
                && let Some(update) = delta.value.update.iter().find(|entry| entry.entity == target.entity)
            {
                last_position = update.position;
            }
        }
        if let Some(corpse) = client.world_entities().and_then(|entities| {
            entities
                .iter()
                .find(|entry| {
                    entry.kind == EntityKind::Corpse
                        && entry.position == last_position
                        && !old_corpses.contains(&entry.entity)
                })
        }) {
            break *corpse;
        }
        if tokio::time::Instant::now() >= deadline {
            let seen = client.world_entities().and_then(|entities| entities.get(&target.entity)).copied();
            let combat = client.session_mut().combat_state();
            return Err(format!("step=death no corpse frame target={:?} entity={seen:?} combat={combat:?}", target.entity).into());
        }
    };
    if client.world_entities().is_some_and(|entities| entities.get(&target.entity).is_some()) {
        return Err("step=death attacked rat remains visible".into());
    }
    let (EntityDetail::Object { item_handle: Some(handle), .. }
        | EntityDetail::ObjectWithAppearance { item_handle: Some(handle), .. }) = corpse.detail else {
        return Err("step=corpse the corpse has no item handle".into());
    };
    if client.open_container().is_none_or(|view| view.container_handle.is_some()) {
        return Err("step=open expected no corpse open before USE".into());
    }
    client.session_mut().begin_open_container_delta_capture();
    let used = client.use_item(handle).await?;
    if used.status != CommandStatus::Accepted
        || used.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    {
        return Err(format!("step=open corpse USE refused {used:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(view) = client.open_container().filter(|view| view.container_handle == Some(handle)) {
            return Ok(view.clone());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step=open no container delta".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
}

async fn spawn_capture_rat(
    client: &mut DevClientSession,
    world: &BundleWorld,
) -> TestResult<oteryn_dev_client::EntityRef> {
    let open = |at: TilePos| {
        world.base().tile(at.x, at.y, at.floor).is_some_and(|tile| !tile.protection_zone())
    };
    let origin = tile(client.world_spatial().actor_position)?;
    let path = route(world, origin, |at| {
        open(at)
            && [StepDirection::North, StepDirection::East, StepDirection::South, StepDirection::West]
                .into_iter()
                .filter(|direction| destination(world, at, *direction).is_some_and(open))
                .count() >= 3
    }).ok_or("step=spawn no open non-protection clearing")?;
    super::full_row_w22a::walk_rat_route(client, world, &path).await?;
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let answer = request_spawn(session, "oteryn:creature.rat".into()).await?;
    let rat = parse_spawned_entity(&answer)?;
    evidence(&format!("W22A_FIXTURE account=q_w22a purpose=replay-setup rat={rat:?} placement=adjacent"));
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(entity) = client.world_entities().and_then(|entities| entities.get(&rat)) {
            let here = client.world_spatial().actor_position;
            if entity.kind == EntityKind::Creature
                && entity.display_name.is_some_and(|name| name.as_str().is_ok_and(|name| name == "Rat"))
                && entity.position.floor == here.floor
                && entity.position.x.abs_diff(here.x).max(entity.position.y.abs_diff(here.y)) <= 1
            {
                return Ok(rat);
            }
            return Err(format!("step=spawn wrong Rat frame {entity:?}").into());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step=spawn exact spawned Rat was not projected".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
}

async fn journey(clients: &SeamClients<'_>, world: &BundleWorld) -> TestResult<String> {
    let ranges = rat_ranges()?;
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_W22A_ACCOUNT").await?;
    let clients = &dedicated.clients(clients);
    let mut client = super::full_row_w22a::prepare_rat_hunter(clients).await?;
    super::full_row_w22a::prepare_rat_attack(&mut client).await?;
    let target = spawn_capture_rat(&mut client, world).await?;
    let before = kill_and_open(&mut client, target).await?;
    let first = *before
        .entries
        .first()
        .ok_or("step=roll empty first corpse cannot qualify pickup")?;
    for entry in &before.entries {
        let (min, max) = ranges
            .get(&entry.item_definition_ref.get())
            .ok_or_else(|| format!("step=table unexpected definition {}", entry.item_definition_ref))?;
        if !(*min..=*max).contains(&entry.count.get()) {
            return Err(format!("step=table count {}:{} outside {min}..={max}", entry.item_definition_ref, entry.count).into());
        }
    }
    let definition = first.item_definition_ref.get();
    let expected_quantity = super::full_row_w22a::quantity(&client, definition)?
        + u64::from(first.count.get());
    let mut expected_after = before.clone();
    expected_after.entries.remove(0);
    let moved = client
        .move_item(&oteryn_dev_client::ItemMoveIntent {
            source: first.handle,
            destination: ItemMoveDestination::MainBackpack,
        })
        .await?;
    if moved.outcome != ItemMoveOutcome::Moved {
        return Err(format!("step=move {moved:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let after = loop {
        if let Some(view) = client.open_container().filter(|view| *view == &expected_after)
            && super::full_row_w22a::quantity(&client, definition)? == expected_quantity
        {
            break view.clone();
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step=move no corpse removal delta".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    };
    let caps = client.session_mut().selected_capabilities();
    let appearances = caps.contains(&item_view::CAPABILITY_ITEM_APPEARANCE_V1);
    let expanded = caps.contains(&item_view::CAPABILITY_CORPSE_CONTAINER_48_V1);
    let deltas = client.session_mut().take_open_container_delta_capture();
    let [opened, picked_up] = deltas.as_slice() else {
        return Err("step=capture expected two live open-container deltas".into());
    };
    let session = client.join_snapshot().game_session_id;
    super::full_row_w22a::logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(clients, 0x99, 0).await?;
    if fresh.join_snapshot().game_session_id == session
        || super::full_row_w22a::quantity(&fresh, definition)? != expected_quantity
    {
        return Err("step=relog picked loot did not persist under fresh admission".into());
    }
    super::full_row_w22a::logout(&mut fresh).await?;
    let root = std::path::PathBuf::from(required("W22A_CAPTURE_DIR")?);
    let table = ranges
        .iter()
        .map(|(reference, (min, max))| format!("{reference}:{min}:{max}"))
        .collect::<Vec<_>>()
        .join(";");
    capture(&root, "w22a-before.bin", &opened.payload)?;
    capture(&root, "w22a-after.bin", &picked_up.payload)?;
    capture(
        &root,
        "w22a-expected.txt",
        format!(
            "session={}\nopen_sequence={}\npickup_sequence={}\nappearances={appearances}\nexpanded={expanded}\nbefore={}\nafter={}\ntable={table}\n",
            uuid_text(session.as_bytes()),
            opened.server_sequence,
            picked_up.server_sequence,
            listed(&before.entries),
            listed(&after.entries),
        )
        .as_bytes(),
    )?;
    Ok(format!(
        "session={} open_sequence={} pickup_sequence={}",
        uuid_text(session.as_bytes()),
        opened.server_sequence,
        picked_up.server_sequence,
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>, world: &BundleWorld) {
    match verdict::indexed_timeout(Duration::from_secs(300), verdict::observe(journey(clients, world))).await {
        Ok(Ok(observed)) => evidence(&format!("e2e_all scenario=W22a.client status=CAPTURED evidence={observed}")),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W22a.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=W22a.client status=FAIL evidence=timeout"),
    }
}
