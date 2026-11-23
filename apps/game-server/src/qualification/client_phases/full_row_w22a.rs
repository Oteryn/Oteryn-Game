//! W22a (monster base-loot chance and count), derived from the matrix, reaudit,
//! and the production `loot_plan` owner:
//!
//! 1. Walk from the provisioned character's saved position to an authored
//!    `oteryn:creature.rat`; expect every STEP reply to move by the compiled map.
//! 2. Send ATTACK_TARGET and expect acceptance, damage deltas, target removal,
//!    and a corpse projection. Repeat by ordinary hunting when the chance roll
//!    legitimately produces an empty corpse, until there is loot.
//! 3. Send USE for the corpse handle; expect a committed reply and an open-
//!    container delta. Assert every rolled definition belongs to the production
//!    rat table and every count is within that entry's admitted min..=max range.
//! 4. Send ITEM_MOVE for one rolled stack into the main backpack; expect MOVED
//!    plus conserving corpse and inventory deltas.
//! 5. Send the same ITEM_MOVE again; expect the stale-handle refusal and no
//!    inventory change (the negative path must not duplicate loot).
//! 6. Logout, make a fresh admission, and re-check that the picked definition's
//!    count and the character position are durable under a new game session.

use super::*;
use oteryn_dev_client::{
    DevClientSession, EntityDetail, EntityKind, ItemMoveDestination, SessionEvent,
};
use oteryn_protocol_oteryn::item_view::{ItemEntry, ItemMoveOutcome};
use std::collections::{BTreeMap, BTreeSet};

const ACTIONS: &[&str] = &[
    "walk-to-rat",
    "attack-and-death",
    "base-roll-bounds",
    "loot-pickup",
    "stale-move-refusal",
    "fresh-admission",
];

struct Row {
    passed: BTreeSet<&'static str>,
    started: std::time::Instant,
}

impl Row {
    fn new() -> Self {
        Self {
            passed: BTreeSet::new(),
            started: std::time::Instant::now(),
        }
    }

    fn pass(&mut self, action: &'static str, detail: impl std::fmt::Display) {
        if ACTIONS.contains(&action) && self.passed.insert(action) {
            evidence(&format!(
                "W22A_CHECK action={action} status=PASS observed={detail}"
            ));
        }
    }

    fn finish(self, result: TestResult) {
        let complete = ACTIONS.iter().all(|action| self.passed.contains(action));
        let error = result.as_ref().err().map(ToString::to_string);
        if let Some(error) = &error {
            evidence(&format!("W22A_GAP observed={error}"));
        }
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=W22a.full status={} seconds={:.3}{}",
            if result.is_ok() && complete { "PASS" } else { "FAIL" },
            self.started.elapsed().as_secs_f64(),
            error
                .as_deref()
                .map(|error| format!(" evidence={}", evidence_token(error)))
                .unwrap_or_default(),
        ));
    }
}

fn evidence_token(error: &str) -> String {
    error
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "-._:/".contains(character) {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn rat_ranges() -> TestResult<BTreeMap<u32, (u32, u32)>> {
    let index = provisioning::item_index()?;
    [("oteryn:item.tibia.i3607", 1, 1), ("oteryn:item.tibia.i3031", 1, 4)]
        .into_iter()
        .map(|(key, min, max)| {
            let revision = index.revision(key).ok_or("step-3-rat-item-revision-missing")?;
            let reference = index
                .definition_ref("Item", key, revision)
                .ok_or("step-3-rat-item-reference-missing")?;
            Ok((reference.get(), (min, max)))
        })
        .collect()
}

fn quantity(client: &DevClientSession, definition: u32) -> TestResult<u64> {
    Ok(client
        .inventory()
        .ok_or("inventory-snapshot-missing")?
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref.get() == definition)
        .map(|entry| u64::from(entry.count.get()))
        .sum())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    loop {
        let reply = client.session_mut().logout().await?;
        if reply.result == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step-6-logout-timeout-observed={reply:?}").into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
    }
}

fn shows_damage(detail: EntityDetail) -> bool {
    matches!(detail,
        EntityDetail::Actor { health_percent, .. } | EntityDetail::ActorWithIcons { health_percent, .. }
        if health_percent < 100
    )
}

pub(super) async fn kill_and_open(
    client: &mut DevClientSession,
    name: &str,
) -> TestResult<Vec<ItemEntry>> {
    let position = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == EntityKind::Creature
                    && entry.display_name.is_some_and(|label| label.as_str().is_ok_and(|shown| shown.eq_ignore_ascii_case(name)))
                    && entry.position.floor == position.floor
                    && entry.position.x.abs_diff(position.x).max(entry.position.y.abs_diff(position.y)) <= 1
            })
        })
        .copied()
        .ok_or_else(|| format!("step-2-no-adjacent-creature-after-walk need={name}"))?;
    let old_corpses: BTreeSet<_> = client.world_entities().into_iter()
        .flat_map(|entities| entities.iter())
        .filter(|entry| entry.kind == EntityKind::Corpse)
        .map(|entry| entry.entity)
        .collect();
    let reply = client.session_mut().attack_target(Some(&target.entity)).await?;
    if reply.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(format!("step-2-attack-refused-observed={reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    let mut damaged = false;
    let mut last_position = target.position;
    let corpse = loop {
        client.service_liveness(Duration::from_millis(100)).await?;
        for event in client.take_events() {
            if let SessionEvent::WorldSpatialEntities(delta) = event {
                if let Some(update) = delta.value.update.iter().find(|entry| entry.entity == target.entity) {
                    last_position = update.position;
                }
                damaged |= delta.value.update.iter().any(|entry| {
                    entry.entity == target.entity
                        && shows_damage(entry.detail)
                });
            }
        }
        if let Some(corpse) = client.world_entities().and_then(|entities| {
            entities
                .iter()
                .find(|entry| entry.kind == EntityKind::Corpse && entry.position == last_position && !old_corpses.contains(&entry.entity))
        }) {
            break *corpse;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step-2-death-timeout-observed=no-corpse-frame".into());
        }
    };
    if !damaged || client.world_entities().is_some_and(|entities| entities.get(&target.entity).is_some()) {
        return Err("step-2-observed-missing-damage-or-target-removal-delta".into());
    }
    let (EntityDetail::Object { item_handle: Some(handle), .. } | EntityDetail::ObjectWithAppearance { item_handle: Some(handle), .. }) = corpse.detail else {
        return Err("step-2-observed-corpse-without-item-handle".into());
    };
    let used = client.use_item(handle).await?;
    if used.status != CommandStatus::Accepted
        || used.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    {
        return Err(format!("step-3-corpse-use-refused-observed={used:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(view) = client.open_container().filter(|view| view.container_handle == Some(handle)) {
            return Ok(view.entries.clone());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step-3-open-timeout-observed=no-container-delta".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
}

pub(super) async fn travel_to_named_spawn(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    creature: &str,
    name: &str,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let points: Vec<_> = spawns
        .sources
        .iter()
        .flat_map(|source| source.points.iter().map(move |point| (source.floor, point)))
        .filter(|(_, point)| {
            spawns
                .creatures
                .get(point.creature as usize)
                .is_some_and(|entry| entry.key == creature)
        })
        .collect();
    let path = route(world, from, |at| {
        !world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_none_or(|tile| tile.protection_zone())
            && points.iter().any(|(floor, point)| {
                at.floor == *floor && at.x.abs_diff(point.x) <= 1 && at.y.abs_diff(point.y) <= 1
            })
    })
    .ok_or_else(|| format!("no reachable {creature} spawn within town route budget"))?;
    walk(client, world, &path).await?;
    for _ in 0..32 {
        let here = client.world_spatial().actor_position;
        let target = client
            .world_entities()
            .and_then(|entities| {
                entities
                    .iter()
                    .filter(|entry| {
                        entry.kind == oteryn_dev_client::EntityKind::Creature
                            && entry.position.floor == here.floor
                            && entry.display_name.is_some_and(|label| label.as_str().is_ok_and(|shown| shown.eq_ignore_ascii_case(name)))
                    })
                    .min_by_key(|entry| {
                        entry
                            .position
                            .x
                            .abs_diff(here.x)
                            .max(entry.position.y.abs_diff(here.y))
                    })
            })
            .copied()
            .ok_or_else(|| format!("spawn route has no visible {name}"))?;
        let target = tile(target.position)?;
        let path = route(world, tile(here)?, |at| {
            at.floor == target.floor && at.x.abs_diff(target.x).max(at.y.abs_diff(target.y)) <= 1
        })
        .ok_or("visible creature unreachable")?;
        let Some(direction) = path.first() else {
            return Ok(());
        };
        walk(client, world, &[*direction]).await?;
    }
    Err("could not approach a live creature within the route budget".into())
}

pub(super) async fn move_loot(client: &mut DevClientSession, entries: &[oteryn_protocol_oteryn::item_view::ItemEntry]) -> TestResult {
    for entry in entries {
        let definition = entry.item_definition_ref.get();
        let expected = quantity(client, definition)? + u64::from(entry.count.get());
        let moved = client.move_item(&oteryn_dev_client::ItemMoveIntent {
            source: entry.handle,
            destination: ItemMoveDestination::MainBackpack,
        }).await?;
        if moved.outcome != ItemMoveOutcome::Moved {
            return Err(format!("corpse-to-backpack move refused: {moved:?}").into());
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while quantity(client, definition)? != expected {
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("corpse-to-backpack custody missing: definition={definition} expected={expected}").into());
            }
            client.service_liveness(Duration::from_millis(50)).await?;
            client.take_events();
        }
    }
    Ok(())
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0xb2).await?;
    travel_to_named_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat", "Rat").await?;
    row.pass("walk-to-rat", "STEP replies moved to authored rat spawn");
    let mut attempts = 1_u8;
    let mut entries = kill_and_open(&mut client, "Rat").await?;
    row.pass(
        "attack-and-death",
        "ATTACK accepted and damage/removal/corpse frames received",
    );
    while entries.is_empty() && attempts < 4 {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
        loop {
            client.service_liveness(Duration::from_millis(250)).await?;
            client.take_events();
            if travel_to_named_spawn(
                &mut client,
                world,
                &metadata.spawns,
                "oteryn:creature.rat",
                "Rat",
            )
            .await
            .is_ok()
            {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!(
                    "step-3-empty-chance-roll-attempts={attempts}-observed=no-rat-respawn-within-70s"
                )
                .into());
            }
        }
        attempts += 1;
        entries = kill_and_open(&mut client, "Rat").await?;
    }
    if entries.is_empty() {
        return Err(format!("step-3-empty-chance-roll-observed-after-attempts={attempts}").into());
    }
    let ranges = rat_ranges()?;
    for entry in &entries {
        let (min, max) = ranges
            .get(&entry.item_definition_ref.get())
            .ok_or_else(|| format!("step-3-unexpected-loot-definition-observed={}", entry.item_definition_ref))?;
        if !(*min..=*max).contains(&entry.count.get()) {
            return Err(format!(
                "step-3-count-out-of-range-observed={}:{}-expected={min}..={max}",
                entry.item_definition_ref, entry.count
            )
            .into());
        }
    }
    row.pass("base-roll-bounds", format!("entries={} all definitions/counts admitted", entries.len()));
    let item = entries[0];
    let definition = item.item_definition_ref.get();
    let before = quantity(&client, definition)?;
    let moved = client
        .move_item(&oteryn_dev_client::ItemMoveIntent {
            source: item.handle,
            destination: ItemMoveDestination::MainBackpack,
        })
        .await?;
    if moved.outcome != ItemMoveOutcome::Moved {
        return Err(format!("step-4-item-move-refused-observed={moved:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while quantity(&client, definition)? != before + u64::from(item.count.get()) {
        if tokio::time::Instant::now() >= deadline {
            return Err("step-4-observed-missing-inventory-delta".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    if client.open_container().is_none_or(|view| view.entries.iter().any(|entry| entry.handle == item.handle)) {
        return Err("step-4-observed-missing-corpse-removal-delta".into());
    }
    row.pass("loot-pickup", format!("definition={definition} count={}", item.count));
    let refused = client
        .move_item(&oteryn_dev_client::ItemMoveIntent {
            source: item.handle,
            destination: ItemMoveDestination::MainBackpack,
        })
        .await?;
    if refused.outcome != ItemMoveOutcome::Stale || quantity(&client, definition)? != before + u64::from(item.count.get()) {
        return Err(format!("step-5-expected-stale-with-no-duplication-observed={refused:?}").into());
    }
    row.pass("stale-move-refusal", format!("outcome={:?}", refused.outcome));
    let saved_position = client.world_spatial().actor_position;
    let old_session = client.join_snapshot().game_session_id;
    let saved_quantity = quantity(&client, definition)?;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_world(clients, 0xb3).await?;
    if fresh.join_snapshot().game_session_id == old_session
        || fresh.world_spatial().actor_position != saved_position
        || quantity(&fresh, definition)? != saved_quantity
    {
        return Err("step-6-fresh-admission-observed-session-position-or-loot-mismatch".into());
    }
    row.pass("fresh-admission", format!("definition={definition} durable-count={saved_quantity}"));
    logout(&mut fresh).await
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = match verdict::indexed_timeout(Duration::from_secs(480), verdict::observe(journey(&mut row, clients))).await {
        Ok(result) => result,
        Err(_) => Err("journey-timeout-observed-before-full-lifecycle".into()),
    };
    row.finish(result);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journey_regression_recognizes_damage_in_actor_icons_projection() {
        assert!(shows_damage(EntityDetail::ActorWithIcons {
            direction: StepDirection::North,
            appearance_ref: 1,
            health_percent: 50,
            icons: Default::default(),
        }));
    }

    #[test]
    fn evidence_is_one_report_safe_token() {
        assert_eq!(evidence_token("step 3: Rejected { x }"), "step-3:-Rejected---x--");
    }

    #[test]
    fn verdict_requires_every_declared_action() {
        let mut row = Row::new();
        for action in &ACTIONS[..ACTIONS.len() - 1] {
            row.pass(action, "test");
        }
        assert!(!ACTIONS.iter().all(|action| row.passed.contains(action)));
        row.pass(ACTIONS[ACTIONS.len() - 1], "test");
        assert!(ACTIONS.iter().all(|action| row.passed.contains(action)));
    }
}
