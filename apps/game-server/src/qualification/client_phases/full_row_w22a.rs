//! W22a (monster base-loot chance and count), derived from the matrix, reaudit,
//! and the production `loot_plan` owner:
//!
//! 1. Walk from the provisioned character's saved position to an authored
//!    `oteryn:creature.rat`; expect every STEP reply to move by the compiled map.
//! 2. Send ATTACK_TARGET and expect acceptance, damage deltas, target removal,
//!    and a corpse projection. An empty roll cannot qualify pickup.
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

pub(super) fn quantity(client: &DevClientSession, definition: u32) -> TestResult<u64> {
    Ok(client
        .inventory()
        .ok_or("inventory-snapshot-missing")?
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref.get() == definition)
        .map(|entry| u64::from(entry.count.get()))
        .sum())
}

pub(super) async fn logout(client: &mut DevClientSession) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    verdict::Harness::global().bounded_wait(deadline, "logout-combat-lock");
    loop {
        let combat = client
            .session_mut()
            .combat_state()
            .ok_or("step-6-missing-combat-state-before-logout")?;
        if !combat.in_fight {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step-6-combat-lock-did-not-expire".into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
    }
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step-6-logout-refused-observed={reply:?}").into());
    }
    Ok(())
}

pub(super) async fn prepare_rat_attack(client: &mut DevClientSession) -> TestResult {
    use oteryn_protocol_oteryn::attack::{AttackIntentDisposition, ChaseMode, FightMode, FightModes};

    let wanted = FightModes {
        fight_mode: FightMode::Offensive,
        chase: ChaseMode::Chase,
        secure: true,
    };
    let before = client
        .session_mut()
        .combat_state()
        .ok_or("step-2-missing-combat-state-before-mode-change")?
        .modes;
    if before == wanted {
        return Err("step-2-combat-modes-already-match".into());
    }
    let reply = client.session_mut().set_fight_modes(&wanted).await?;
    if reply.disposition != AttackIntentDisposition::Ok {
        return Err(format!("step-2-offensive-chase-mode-refused observed={reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while client
        .session_mut()
        .combat_state()
        .is_none_or(|state| state.modes != wanted)
    {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step-2-offensive-chase-mode-missing observed={reply:?}").into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    Ok(())
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
    Ok(kill_and_open_tracked(client, name, None).await?.entries)
}

struct OpenedCorpse {
    target: oteryn_dev_client::EntityRef,
    corpse: oteryn_dev_client::EntityRef,
    entries: Vec<ItemEntry>,
}

async fn kill_and_open_tracked(
    client: &mut DevClientSession,
    name: &str,
    selected: Option<oteryn_dev_client::EntityRef>,
) -> TestResult<OpenedCorpse> {
    let position = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == EntityKind::Creature
                    && selected.is_none_or(|target| entry.entity == target)
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
            return Ok(OpenedCorpse {
                target: target.entity,
                corpse: corpse.entity,
                entries: view.entries.clone(),
            });
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
    travel_to_spawn_with_steps(client, world, spawns, creature, name, false).await.map(drop)
}

pub(super) async fn travel_to_rat(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult<oteryn_dev_client::EntityRef> {
    travel_to_spawn_with_steps(client, world, spawns, "oteryn:creature.rat", "Rat", true).await
}

async fn travel_to_spawn_with_steps(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    creature: &str,
    name: &str,
    strict: bool,
) -> TestResult<oteryn_dev_client::EntityRef> {
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
    if strict {
        walk_rat_route(client, world, &path).await?;
    } else {
        walk(client, world, &path).await?;
    }
    let mut selected = None;
    for _ in 0..32 {
        let here = client.world_spatial().actor_position;
        let target = client
            .world_entities()
            .and_then(|entities| {
                entities
                    .iter()
                    .filter(|entry| {
                        entry.kind == oteryn_dev_client::EntityKind::Creature
                            && selected.is_none_or(|target| entry.entity == target)
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
        if strict {
            selected = Some(target.entity);
        }
        let target_id = target.entity;
        let target = tile(target.position)?;
        let path = route(world, tile(here)?, |at| {
            at.floor == target.floor && at.x.abs_diff(target.x).max(at.y.abs_diff(target.y)) <= 1
        })
        .ok_or("visible creature unreachable")?;
        let Some(direction) = path.first() else {
            return Ok(target_id);
        };
        if strict {
            walk_rat_route(client, world, &[*direction]).await?;
        } else {
            walk(client, world, &[*direction]).await?;
        }
    }
    Err("could not approach a live creature within the route budget".into())
}

async fn walk_rat_route(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    let mut remaining = VecDeque::from(path.to_vec());
    let mut detours = 0;
    while let Some(direction) = remaining.pop_front() {
        let before = client.world_spatial().actor_position;
        let from = tile(before)?;
        let expected = destination(world, from, direction).ok_or("rat route step has no landing")?;
        let outcome = client.step(direction).await?;
        if outcome.disposition == StepDisposition::Blocked && client.world_spatial().actor_position == before {
            let occupant = client.world_entities().and_then(|entities| {
                entities.iter().find(|entry| entry.kind == EntityKind::Creature && tile(entry.position).ok() == Some(expected))
            }).copied();
            let Some(occupant) = occupant else {
                return Err(format!("BLOCKER-SERVER rat-route blocked with no creature frame from={from:?} to={expected:?} result={outcome:?}").into());
            };
            if detours >= 4 {
                return Err("rat route exhausted its four tracked-occupant detours".into());
            }
            detours += 1;
            client.service_liveness(Duration::from_millis(100)).await?;
            let current = client.world_entities().and_then(|entities| entities.get(&occupant.entity)).map(|entry| entry.position);
            evidence(&format!("W22A_MOVE_BLOCKED creature={:?} before={:?} current={current:?} reply={outcome:?}", occupant.entity, occupant.position));
            if current != Some(occupant.position) {
                remaining.push_front(direction);
                continue;
            }
            remaining = route_around_occupant(
                from,
                expected,
                &remaining,
                |at| client.world_entities().is_some_and(|entities| entities.iter().all(|entry| tile(entry.position).ok() != Some(at))),
                |at, step| destination(world, at, step),
            )
            .ok_or("rat route has no detour around the tracked creature")?
            .into();
            continue;
        }
        if outcome.disposition != StepDisposition::Moved
            || outcome.world_spatial_delta.is_none()
            || tile(client.world_spatial().actor_position)? != expected
        {
            return Err(format!("rat route step mismatch from={from:?} expected={expected:?} reply={outcome:?}").into());
        }
        verdict::Harness::global().progress("walk");
        client.take_events();
    }
    Ok(())
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

async fn verify_rat_roll(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    rewards: &crate::content::creature_reward::CreatureRewardTable,
    opened: &OpenedCorpse,
) -> TestResult {
    const RAT: &str = "oteryn:creature.rat";
    let death = committed_hunt_death(
        client,
        clients,
        Some(clients.characters[1]),
        RAT,
        opened.target,
        tokio::time::Instant::now() + Duration::from_secs(5),
    )
    .await?;
    let actor = death.actor;
    let channel = actor.channel_id().ok_or("step-3-rat-death-outside-channel")?;
    if opened.corpse.identity != actor.corpse_identity() {
        return Err("step-3-opened-corpse-does-not-belong-to-attacked-rat".into());
    }
    let seed = crate::combat::LootPlanDeathKey::new(
        *actor.world_id().as_bytes(),
        *channel.as_bytes(),
        actor.scope_generation().get(),
        actor.actor_local_id(),
        actor.actor_local_generation(),
    );
    let row = rewards.row(RAT).map_err(|error| format!("step-3-rat-loot-definition: {error:?}"))?;
    let reference = row.loot_table_ref.as_ref().ok_or("step-3-rat-loot-table-missing")?;
    // These are the admitted estimates from the retained Global Rat loot sample.
    // Source: https://tibia.fandom.com/wiki/Loot_Statistics:Rat
    let rates = BTreeMap::from([
        ("oteryn:item.tibia.i3031", (1, 4, Some(903_036))),
        ("oteryn:item.tibia.i3607", (1, 1, Some(356_433))),
    ]);
    for entry in &row.loot_table.entries {
        if rates.get(entry.item.production_key.as_str())
            != Some(&(entry.min_count, entry.max_count, entry.probability_ppm))
        {
            return Err(format!("step-3-wrong-base-loot-entry: {entry:?}").into());
        }
    }
    if row.loot_table.entries.len() != rates.len() {
        return Err("step-3-wrong-base-loot-table-size".into());
    }
    let plan = crate::combat::plan_creature_loot(seed, reference, &row.loot_table)
        .map_err(|error| format!("step-3-rat-loot-plan: {error:?}"))?;
    let index = provisioning::item_index()?;
    let mut expected = BTreeMap::new();
    for entry in plan.entries {
        let reference = index
            .definition_ref(&entry.item.family, &entry.item.production_key, &entry.item.revision_ref)
            .ok_or("step-3-rat-loot-plan-item-missing")?;
        *expected.entry(reference).or_insert(0_u64) += u64::from(entry.quantity);
    }
    let mut observed = BTreeMap::new();
    for entry in &opened.entries {
        *observed.entry(entry.item_definition_ref).or_insert(0_u64) += u64::from(entry.count.get());
    }
    if observed != expected {
        return Err(format!("step-3-rat-roll-mismatch expected={expected:?} observed={observed:?}").into());
    }
    Ok(())
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let generation = load_world_generation(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let world = &generation.bundle;
    let mut client = connect_world(clients, 0xb2).await?;
    let start = client.world_spatial().actor_position;
    let target = travel_to_rat(&mut client, world, &metadata.spawns).await?;
    let arrived = client.world_spatial().actor_position;
    if arrived == start {
        return Err("step-1-rat-route-did-not-move-the-player".into());
    }
    row.pass("walk-to-rat", format!("from={start:?} to={arrived:?} rat={target:?}"));
    prepare_rat_attack(&mut client).await?;
    let opened = kill_and_open_tracked(&mut client, "Rat", Some(target)).await?;
    row.pass(
        "attack-and-death",
        "ATTACK accepted and damage/removal/corpse frames received",
    );
    verify_rat_roll(&mut client, clients, &generation.rewards, &opened).await?;
    let entries = opened.entries;
    let item = *entries
        .first()
        .ok_or("step-3-empty-first-roll-cannot-qualify-pickup")?;
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
    let definition = item.item_definition_ref.get();
    let before = quantity(&client, definition)?;
    let expected_quantity = before + u64::from(item.count.get());
    let mut expected_corpse = client
        .open_container()
        .ok_or("step-4-pre-move-corpse-view-missing")?
        .clone();
    if expected_corpse.entries != entries {
        return Err("step-4-pre-move-corpse-view-changed".into());
    }
    expected_corpse.entries.remove(0);
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
    while quantity(&client, definition)? != expected_quantity
        || client.open_container() != Some(&expected_corpse)
    {
        if tokio::time::Instant::now() >= deadline {
            return Err("step-4-observed-inventory-or-corpse-mismatch".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    row.pass(
        "loot-pickup",
        format!("definition={definition} count={}", item.count),
    );
    let inventory_after_pickup = client
        .inventory()
        .ok_or("step-5-inventory-missing")?
        .clone();
    let refused = client
        .move_item(&oteryn_dev_client::ItemMoveIntent {
            source: item.handle,
            destination: ItemMoveDestination::MainBackpack,
        })
        .await?;
    if refused.outcome != ItemMoveOutcome::Stale
        || client.inventory() != Some(&inventory_after_pickup)
        || client.open_container() != Some(&expected_corpse)
    {
        return Err(
            format!("step-5-expected-stale-with-no-duplication-observed={refused:?}").into(),
        );
    }
    row.pass(
        "stale-move-refusal",
        format!("outcome={:?}", refused.outcome),
    );
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
