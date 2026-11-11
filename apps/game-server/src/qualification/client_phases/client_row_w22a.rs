//! W22a client replay capture: a player kills a rat, opens its corpse and moves the first rolled
//! stack into the backpack. The corpse view the client holds (`Session::open_container`) before and
//! after the move is session-owned, so (the C17c practice) both views are re-encoded with the
//! production `item_view::encode_open_container`; `crates/session/src/tests/e2e_w22a.rs` replays
//! them through `Session::admit` and `Session::apply_state_delta`. The expected file lists the
//! server rat table range for each rolled definition.
use super::*;
use oteryn_dev_client::{
    DevClientSession, EntityDetail, EntityKind, ItemMoveDestination, SessionEvent,
};
use oteryn_protocol_oteryn::item_view::{self, ItemEntry, ItemMoveOutcome, OpenContainer};

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

fn rat_ranges() -> TestResult<BTreeMap<u32, (u32, u32)>> {
    let index = provisioning::item_index()?;
    [
        ("oteryn:item.tibia.i3607", 1, 1),
        ("oteryn:item.tibia.i3031", 1, 4),
    ]
    .into_iter()
    .map(|(key, min, max)| {
        let revision = index
            .revision(key)
            .ok_or("step=table rat item revision missing")?;
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

async fn kill_and_open(client: &mut DevClientSession) -> TestResult<OpenContainer> {
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
        .ok_or("step=attack no adjacent rat after the walk")?;
    let reply = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    if reply.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(format!("step=attack refused {reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let mut last_position = target.position;
    let corpse = loop {
        client.service_liveness(Duration::from_millis(100)).await?;
        for event in client.take_events() {
            if let SessionEvent::WorldSpatialEntities(delta) = event
                && let Some(update) = delta
                    .value
                    .update
                    .iter()
                    .find(|entry| entry.entity == target.entity)
            {
                last_position = update.position;
            }
        }
        if let Some(corpse) = client.world_entities().and_then(|entities| {
            entities
                .iter()
                .find(|entry| entry.kind == EntityKind::Corpse && entry.position == last_position)
        }) {
            break *corpse;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step=death no corpse frame".into());
        }
    };
    let EntityDetail::Object {
        item_handle: Some(handle),
        ..
    } = corpse.detail
    else {
        return Err("step=corpse the corpse has no item handle".into());
    };
    let used = client.use_item(handle).await?;
    if used.status != CommandStatus::Accepted
        || used.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    {
        return Err(format!("step=open corpse USE refused {used:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(view) = client
            .open_container()
            .filter(|view| view.container_handle == Some(handle))
        {
            return Ok(view.clone());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step=open no container delta".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
}

async fn journey(clients: &SeamClients<'_>, world: &BundleWorld) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(300);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let ranges = rat_ranges()?;
    let mut client = connect_character(clients, 0x98, 1).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        let mut before = OpenContainer::default();
        for _attempt in 0..4 {
            travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
            before = kill_and_open(&mut client).await?;
            if !before.entries.is_empty() {
                break;
            }
        }
        let first = *before
            .entries
            .first()
            .ok_or("step=roll every rat corpse was empty after four kills")?;
        for entry in &before.entries {
            let (min, max) = ranges
                .get(&entry.item_definition_ref.get())
                .ok_or_else(|| {
                    format!(
                        "step=table unexpected definition {}",
                        entry.item_definition_ref
                    )
                })?;
            if !(*min..=*max).contains(&entry.count.get()) {
                return Err(format!(
                    "step=table count {}:{} outside {min}..={max}",
                    entry.item_definition_ref, entry.count
                )
                .into());
            }
        }
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
            if let Some(view) = client.open_container()
                && view
                    .entries
                    .iter()
                    .all(|entry| entry.handle != first.handle)
            {
                break view.clone();
            }
            if tokio::time::Instant::now() >= deadline {
                return Err("step=move no corpse removal delta".into());
            }
            client.service_liveness(Duration::from_millis(50)).await?;
            client.take_events();
        };
        let before_payload = item_view::encode_open_container(&before)
            .map_err(|error| format!("encode {error:?}"))?;
        let after_payload = item_view::encode_open_container(&after)
            .map_err(|error| format!("encode {error:?}"))?;
        let table = ranges
            .iter()
            .map(|(reference, (min, max))| format!("{reference}:{min}:{max}"))
            .collect::<Vec<_>>()
            .join(";");
        capture("w22a-before.bin", &before_payload)?;
        capture("w22a-after.bin", &after_payload)?;
        capture(
            "w22a-expected.txt",
            format!(
                "before={}\nafter={}\ntable={table}\n",
                listed(&before.entries),
                listed(&after.entries)
            )
            .as_bytes(),
        )?;

        Ok(format!(
            "before={} after={}",
            listed(&before.entries),
            listed(&after.entries)
        ))
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>, world: &BundleWorld) {
    match row_complete(journey(clients, world)).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W22a.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W22a.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=W22a.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn loot_evidence_has_no_rows_for_empty_inventory() {
        assert_eq!(listed(&[]), "");
    }
}
