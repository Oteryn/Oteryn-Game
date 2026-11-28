//! C03 full-row journey (Critical, life/mana leech, reflect, cleave, perfect shot and fatal).
//! Every mechanic is judged on its measured effect in attributed server frames, never on the
//! projection alone: a projection only says an effect may occur, so a zero projection means the
//! player holds no source and the step FAILs naming what is needed.
//!
//! Confirm fresh admission, then observe the following steps.
//! 1. `tabs`: request the native Offence and Defence tabs; require revisioned projections.
//! 2. `baseline`: attack a live compiled-world monster; require attributed player hits, monster
//!    hits on the player, and an idle vitals regeneration rate to compare against.
//! 3. `critical`: a hit the server marks as critical while the critical projection is positive;
//!    the attributed frames carry no such mark yet, so the step fails naming it.
//! 4. `leech`: attacker health and mana rise beyond the idle regeneration while hitting.
//! 5. `reflect`: a monster hits the player and the same decision damages that monster without a
//!    player hit.
//! 6. `cleave`: a second adjacent monster takes player damage after one target command.
//! 7. `perfect-shot`: a hit at the projected distance that the server marks as a perfect shot.
//! 8. `fatal`: a hit the server marks as fatal while the onslaught projection is positive.
//! 9. `refusal-relog`: self-target is refused with no damage; after logout and fresh admission the
//!    tabs and equipment are unchanged.

use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, EntityRef, SessionEvent};
use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
use oteryn_protocol_oteryn::cyclopedia_character::{CharacterTab, combat};
use oteryn_protocol_oteryn::item_view as items;
use oteryn_protocol_oteryn::presentation::{PresentationEventKind, PresentationValueKind};
use std::collections::{BTreeMap, BTreeSet};

use combat::CombatStat as Stat;

const STEPS: [&str; 10] = [
    "admission", "tabs", "baseline", "critical", "leech", "reflect", "cleave", "perfect-shot", "fatal", "refusal-relog",
];
const FIGHT_SECONDS: u64 = 25;
/// The attributed damage frames carry no critical, fatal or perfect-shot mark, so the client
/// cannot tell those hits from ordinary damage.
const UNMARKED: &str = "needed=server-mark-on-the-hit;ValueText-has-no-critical-fatal-or-perfect-shot-mark;source=crates/protocol-oteryn/src/presentation.rs:101";

struct Row {
    started: std::time::Instant,
    passed: Vec<&'static str>,
    failures: Vec<(&'static str, String)>,
}

impl Row {
    fn new() -> Self {
        Self { started: std::time::Instant::now(), passed: Vec::new(), failures: Vec::new() }
    }

    fn pass(&mut self, step: &'static str, observed: impl std::fmt::Debug) {
        if STEPS.contains(&step) && !self.passed.contains(&step) && !self.failures.iter().any(|(s, _)| *s == step) {
            self.passed.push(step);
            evidence(&format!("C03_CHECK {step} PASS observed={observed:?}"));
        }
    }

    fn fail(&mut self, step: &'static str, observed: impl std::fmt::Display) {
        if STEPS.contains(&step) && !self.passed.contains(&step) && !self.failures.iter().any(|(s, _)| *s == step) {
            verdict::record_assertion(step);
            let observed = observed.to_string();
            evidence(&format!("C03_CHECK {step} FAIL observed={observed}"));
            self.failures.push((step, observed));
        }
    }

    fn complete(&self) -> bool {
        self.failures.is_empty() && STEPS.iter().all(|step| self.passed.contains(step))
    }

    fn finish(mut self, result: TestResult) {
        if let Err(error) = result {
            verdict::record_error(error.as_ref());
            match STEPS.iter().copied().find(|s| !self.passed.contains(s) && !self.failures.iter().any(|(f, _)| f == s)) {
                Some(step) => self.fail(step, error),
                // An error after every step was judged (the final logout, say) still fails the row.
                None => {
                    let observed = error.to_string();
                    evidence(&format!("C03_CHECK journey FAIL observed={observed}"));
                    self.failures.push(("journey", observed));
                }
            }
        }
        let detail = if self.complete() {
            String::new()
        } else {
            let failed: Vec<String> = self.failures.iter().map(|(step, why)| format!("{step}:{}", token(why))).collect();
            let missing: Vec<_> = STEPS
                .iter()
                .filter(|s| !self.passed.contains(s) && !self.failures.iter().any(|(f, _)| f == *s))
                .copied()
                .collect();
            evidence(&format!(
                "C03_GAP row=C03 failed={} unexecuted={}",
                failed.join("|"),
                missing.join(","),
            ));
            format!(
                " evidence=failed:{};not-run:{}",
                failed.join("|"),
                missing.join(",")
            )
        };
        evidence(&format!(
            "e2e_all scenario=C03.full status={} seconds={:.3}{detail}",
            if self.complete() { "PASS" } else { "FAIL" },
            self.started.elapsed().as_secs_f64()
        ));
    }
}

fn token(value: &str) -> String {
    value
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || "-_.:/<>=;,".contains(c) { c } else { '-' })
        .collect()
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = match verdict::indexed_timeout(Duration::from_secs(420), verdict::observe(journey(&mut row, clients))).await {
        Ok(result) => result,
        Err(_) => Err("journey-timeout-420s-without-server-reply".into()),
    };
    row.finish(result);
}

async fn tab(client: &mut DevClientSession, tab: CharacterTab) -> TestResult<combat::CombatStats> {
    use oteryn_dev_client::client_ui::cyclopedia_character::{
        CyclopediaCharacterPanel, Intent, request,
    };
    let mut panel = CyclopediaCharacterPanel {
        selected: true,
        tab,
        ..Default::default()
    };
    let command = request(&mut panel, Intent::Tab).ok_or("C03 combat page request unavailable")?;
    let response = client
        .session_mut()
        .gated_command(command.capability, command.kind, &command.payload)
        .await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!("character-tab-{tab:?}-status:{:?}", response.status).into());
    }
    let stats = combat::CombatStats::decode(&response.payload)
        .map_err(|error| format!("tab-{tab:?}-decode:{error:?}"))?;
    if stats.tab() != tab {
        return Err(format!("C03 expected {tab:?} but received {:?}", stats.tab()).into());
    }
    Ok(stats)
}

struct Stats {
    offence: combat::CombatStats,
    defence: combat::CombatStats,
}

impl Stats {
    async fn read(client: &mut DevClientSession) -> TestResult<Self> {
        Ok(Self { offence: tab(client, CharacterTab::Offence).await?, defence: tab(client, CharacterTab::Defence).await? })
    }

    fn off(&self, key: Stat) -> i64 {
        self.offence.get(key).unwrap_or(0)
    }

    fn def(&self, key: Stat) -> i64 {
        self.defence.get(key).unwrap_or(0)
    }

    fn perfect_shots(&self) -> Vec<(u32, i64)> {
        [Stat::PerfectShot1, Stat::PerfectShot2, Stat::PerfectShot3, Stat::PerfectShot4, Stat::PerfectShot5, Stat::PerfectShot6, Stat::PerfectShot7]
            .into_iter()
            .zip(1_u32..)
            .filter_map(|(key, distance)| self.offence.get(key).filter(|v| *v > 0).map(|v| (distance, v)))
            .collect()
    }

    /// The equipment-sourced offence projections that a clean baseline requires to be zero.
    fn offence_sources(&self) -> i64 {
        [Stat::CriticalChance, Stat::LifeLeech, Stat::ManaLeech, Stat::Onslaught, Stat::Cleave].iter().map(|k| self.off(*k)).sum::<i64>()
            + self.perfect_shots().len() as i64
            + self.def(Stat::PhysicalReflection)
    }

    fn same(&self, other: &Self) -> bool {
        let tabs = [
            (&self.offence, &other.offence, CharacterTab::Offence),
            (&self.defence, &other.defence, CharacterTab::Defence),
        ];
        tabs.iter()
            .all(|(a, b, tab)| Stat::for_tab(*tab).iter().all(|k| a.get(*k) == b.get(*k)))
    }
}

#[derive(PartialEq, Eq)]
struct Custody {
    main_backpack: (u32, u32, u32),
    items: BTreeMap<(Option<items::EquipmentSlot>, u32, u32), u64>,
}

fn custody(client: &DevClientSession) -> TestResult<Custody> {
    let inventory = client
        .inventory()
        .ok_or("C03 inventory projection absent")?;
    let pack = inventory
        .main_backpack
        .ok_or("C03 main backpack projection absent")?;
    let mut items = BTreeMap::new();
    for (slot, item) in inventory
        .equipment
        .iter()
        .map(|equipped| (Some(equipped.slot), &equipped.item))
        .chain(inventory.entries.iter().map(|item| (None, item)))
    {
        *items
            .entry((slot, item.item_definition_ref.get(), item.sub_type))
            .or_default() += u64::from(item.count.get());
    }
    Ok(Custody {
        main_backpack: (
            pack.item_definition_ref.get(),
            pack.count.get(),
            pack.sub_type,
        ),
        items,
    })
}

fn own(client: &DevClientSession) -> TestResult<EntityRef> {
    let entities = client.world_entities().ok_or("own-entity-snapshot-missing")?;
    entities
        .iter()
        .find(|entry| entry.entity.identity == *entities.own_identity())
        .map(|entry| entry.entity)
        .ok_or_else(|| "own-entity-missing".into())
}

fn monsters_near(client: &DevClientSession, within: i32) -> Vec<EntityRef> {
    let at = client.world_spatial().actor_position;
    client
        .world_entities()
        .map(|entities| {
            entities
                .iter()
                .filter(|e| {
                    e.kind == EntityKind::Creature
                        && e.position.floor == at.floor
                        && e.position.x.abs_diff(at.x).max(e.position.y.abs_diff(at.y)) <= within as u32
                })
                .map(|e| e.entity)
                .collect()
        })
        .unwrap_or_default()
}

fn outside_protection(world: &BundleWorld, at: TilePos) -> bool {
    world
        .base()
        .tile(at.x, at.y, at.floor)
        .is_some_and(|ground| !ground.protection_zone())
}

fn baseline_route(
    world: &BundleWorld,
    from: TilePos,
    spawns: &oteryn_world_bundle::spawn::Table,
    available: impl Fn(TilePos) -> bool,
    searched: &[TilePos],
) -> Option<Vec<StepDirection>> {
    let mut approaches = BTreeSet::new();
    for (spawn, _) in full_e2ef3::ordinary_spawn_points(spawns) {
        if !outside_protection(world, spawn) {
            continue;
        }
        for dx in -1..=1 {
            for dy in -1..=1 {
                if let (Some(x), Some(y)) = (
                    spawn.x.checked_add_signed(dx),
                    spawn.y.checked_add_signed(dy),
                ) {
                    let at = TilePos { x, y, ..spawn };
                    if at != spawn
                        && outside_protection(world, at)
                        && !searched.iter().any(|prior| {
                            prior.floor == at.floor
                                && prior.x.abs_diff(at.x).max(prior.y.abs_diff(at.y)) <= 8
                        })
                    {
                        approaches.insert(at);
                    }
                }
            }
        }
    }
    // The compiled town includes reachable underground floors beyond the
    // shared two-floor radius. Search its finite movement graph.
    find_route(
        from,
        |at| approaches.contains(&at) && available(at),
        |at, direction| destination(world, at, direction).filter(|next| available(*next)),
    )
    .ok()
}

async fn reachable_monster(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult<EntityRef> {
    let mut searched = Vec::new();
    for _ in 0..3 {
        let from = tile(client.world_spatial().actor_position)?;
        let occupied = full_e2ef3::occupied_tiles(client);
        let path = baseline_route(
            world,
            from,
            spawns,
            |at| full_e2ef3::route_available(at) && !occupied.contains(&at),
            &searched,
        )
        .ok_or_else(|| format!("no reachable compiled monster approach from {from:?}; prior approaches={searched:?}"))?;
        walk(client, world, &path).await?;
        searched.push(tile(client.world_spatial().actor_position)?);
        if let Some(target) = visible_monster(client, world).await? {
            return Ok(target);
        }
    }
    Err(format!("no live adjacent spawn at compiled approaches {searched:?}").into())
}

fn live_approach_route(
    world: &BundleWorld,
    from: TilePos,
    target: TilePos,
    occupied: &BTreeSet<TilePos>,
) -> Option<Vec<StepDirection>> {
    route_avoiding(
        world,
        from,
        |cell| {
            cell != target
                && cell.floor == target.floor
                && outside_protection(world, cell)
                && cell.x.abs_diff(target.x).max(cell.y.abs_diff(target.y)) <= 1
        },
        |cell| full_e2ef3::route_available(cell) && cell != target && !occupied.contains(&cell),
    )
}

async fn visible_monster(
    client: &mut DevClientSession,
    world: &BundleWorld,
) -> TestResult<Option<EntityRef>> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(7);
    while tokio::time::Instant::now() < deadline {
        let here = tile(client.world_spatial().actor_position)?;
        if outside_protection(world, here) {
            let target = client
                .world_entities()
                .into_iter()
                .flat_map(|entities| entities.others())
                .filter(|entity| entity.kind == EntityKind::Creature)
                .filter_map(|entity| {
                    let at = tile(entity.position).ok()?;
                    (here.floor == at.floor && outside_protection(world, at)).then_some((
                        entity.entity,
                        at,
                        here.x.abs_diff(at.x).max(here.y.abs_diff(at.y)),
                    ))
                })
                .min_by_key(|(_, _, distance)| *distance);
            if let Some((entity, at, distance)) = target {
                if distance <= 1 {
                    return Ok(Some(entity));
                }
                let occupied = full_e2ef3::occupied_tiles(client);
                let path = live_approach_route(world, here, at, &occupied);
                if let Some(direction) = path.and_then(|path| path.first().copied()) {
                    walk(client, world, &[direction]).await?;
                }
            }
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
    }
    Ok(None)
}

/// One attributed ValueText observation: decision ordinal, target, attacker, value.
type Seen = (u32, EntityRef, Option<EntityRef>, u64);

#[derive(Default)]
struct Fight {
    seen: Vec<Seen>,
    health_up: u64,
    mana_up: u64,
    seconds: f64,
    flank: Option<EntityRef>,
}

impl Fight {
    fn own_hits(&self, me: EntityRef) -> Vec<u64> {
        self.seen.iter().filter(|(_, t, a, _)| *a == Some(me) && *t != me).map(|s| s.3).collect()
    }
}

fn collect(event: &SessionEvent, out: &mut Vec<Seen>) {
    let SessionEvent::Presentation(delta) = event else { return };
    for e in &delta.value {
        if let PresentationEventKind::ValueText { kind, target, attacker, primary, .. } = &e.kind
            && matches!(kind, PresentationValueKind::DamageDealt | PresentationValueKind::DamageReceived | PresentationValueKind::DamageOthers)
            && primary.value > 0
        {
            out.push((e.decision_ordinal, *target, *attacker, primary.value));
        }
    }
}

/// Idle regeneration: no attack, vitals polled for the window; returns (health/s, mana/s).
async fn idle_rate(client: &mut DevClientSession, seconds: u64) -> TestResult<(f64, f64)> {
    let first = *client.actor_vitals().ok_or("vitals-absent")?;
    let (mut hp, mut mp, mut last) = (0_u64, 0_u64, first);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(seconds);
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
        let now = *client.actor_vitals().ok_or("vitals-absent")?;
        hp += u64::from(now.health.saturating_sub(last.health));
        mp += u64::from(now.mana.saturating_sub(last.mana));
        last = now;
    }
    Ok((hp as f64 / seconds as f64, mp as f64 / seconds as f64))
}

async fn fight(client: &mut DevClientSession, target: EntityRef, seconds: u64, hits_wanted: usize) -> TestResult<Fight> {
    let me = own(client)?;
    client.take_events();
    let reply = client.session_mut().attack_target(Some(&target)).await?;
    if reply.disposition != AttackIntentDisposition::Ok {
        return Err(format!("attack-target-reply:{reply:?}").into());
    }
    let mut log = Fight { flank: monsters_near(client, 1).into_iter().find(|m| *m != target), ..Fight::default() };
    let began = tokio::time::Instant::now();
    let mut last = *client.actor_vitals().ok_or("vitals-absent")?;
    while began.elapsed() < Duration::from_secs(seconds) && log.own_hits(me).len() < hits_wanted {
        client.service_liveness(Duration::from_millis(50)).await?;
        let now = *client.actor_vitals().ok_or("vitals-absent")?;
        log.health_up += u64::from(now.health.saturating_sub(last.health));
        log.mana_up += u64::from(now.mana.saturating_sub(last.mana));
        last = now;
        for event in client.take_events() {
            collect(&event, &mut log.seen);
        }
        if log.flank.is_none() {
            log.flank = monsters_near(client, 1).into_iter().find(|m| *m != target);
        }
        if client
            .world_entities()
            .is_some_and(|entities| entities.get(&target).is_none())
        {
            break;
        }
    }
    log.seconds = began.elapsed().as_secs_f64();
    let stop = client.session_mut().attack_target(None).await?;
    if stop.disposition != AttackIntentDisposition::Ok {
        return Err(format!("attack-stop-reply:{stop:?}").into());
    }
    Ok(log)
}

/// Equipment entries as (slot, definition) so they can be restored exactly.
fn worn_slots(client: &DevClientSession) -> Vec<items::EquipmentSlot> {
    client
        .inventory()
        .map(|inv| inv.equipment.iter().map(|e| e.slot).collect())
        .unwrap_or_default()
}

async fn settle(client: &mut DevClientSession, what: &str, ready: impl Fn(&DevClientSession) -> bool) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !ready(client) {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("no inventory delta for {what}").into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
    Ok(())
}

async fn move_to_backpack(client: &mut DevClientSession, slot: items::EquipmentSlot) -> TestResult<items::ItemHandle> {
    use oteryn_dev_client::client_runtime::inventory::MoveRequest;
    let inventory = client.inventory().ok_or("inventory absent before unequip")?;
    let handle = inventory.equipment.iter()
        .find(|entry| entry.slot == slot)
        .map(|entry| entry.item.handle)
        .ok_or("equipment slot already empty")?;
    if inventory.entries.iter().any(|entry| entry.handle == handle) {
        return Err("equipped item already appears in the backpack".into());
    }
    let moved = client.session_mut().move_item_counted(MoveRequest {
        source: handle,
        destination: items::ItemMoveDestination::MainBackpack,
        count: None,
    }).await?;
    if moved.outcome != items::ItemMoveOutcome::Moved {
        return Err(format!("unequip ITEM_MOVE reply={moved:?}").into());
    }
    settle(client, "unequip", |c| c.inventory().is_some_and(|inv|
        !inv.equipment.iter().any(|entry| entry.slot == slot)
            && inv.entries.iter().any(|entry| entry.handle == handle)
    )).await?;
    Ok(handle)
}

async fn restore(client: &mut DevClientSession, slot: items::EquipmentSlot, handle: items::ItemHandle) -> TestResult {
    use oteryn_dev_client::client_runtime::inventory::MoveRequest;
    let inventory = client.inventory().ok_or("inventory absent before re-equip")?;
    if inventory.equipment.iter().any(|entry| entry.slot == slot)
        || !inventory.entries.iter().any(|entry| entry.handle == handle)
    {
        return Err("re-equip requires the stripped item in the backpack and its original slot empty".into());
    }
    let moved = client.session_mut().move_item_counted(MoveRequest {
        source: handle,
        destination: items::ItemMoveDestination::Equipment(slot),
        count: None,
    }).await?;
    if moved.outcome != items::ItemMoveOutcome::Moved {
        return Err(format!("re-equip ITEM_MOVE reply={moved:?}").into());
    }
    settle(client, "re-equip", |c| c.inventory().is_some_and(|inv|
        !inv.entries.iter().any(|entry| entry.handle == handle)
            && inv.equipment.iter().any(|entry| entry.slot == slot && entry.item.handle == handle)
    )).await
}

/// Strip worn items one at a time until every offence source projection reads zero; returns the
/// stripped items and whether the clean state was reached (imbuements cannot be stripped).
async fn strip(client: &mut DevClientSession) -> TestResult<(Vec<(items::EquipmentSlot, items::ItemHandle)>, bool)> {
    let mut stripped = Vec::new();
    for slot in worn_slots(client) {
        if Stats::read(client).await?.offence_sources() == 0 {
            break;
        }
        let handle = move_to_backpack(client, slot).await?;
        stripped.push((slot, handle));
    }
    let clean = Stats::read(client).await?.offence_sources() == 0;
    Ok((stripped, clean))
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    // The active combat constants give an ordinary hit a 60-second fight lock.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    while client.session_mut().combat_state().is_none_or(|state| state.in_fight) {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err("C03 combat lock did not clear before logout".into());
        }
        client.service_liveness(remaining.min(Duration::from_millis(250))).await?;
    }
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("logout reply={:?}", reply.result).into());
    }
    Ok(())
}

fn needed(stat: &str, projection: i64) -> String {
    format!("needed=player-obtainable-source-of-{stat};projection={projection};observed-after-ordinary-admission-and-no-equipment-or-imbuement-route-taken")
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
    let mut client = connect_world(clients, 0xc3).await?;
    let first_session = client.join_snapshot().game_session_id;
    row.pass("admission", first_session);
    let me = own(&client)?;

    let with = Stats::read(&mut client).await?;
    row.pass("tabs", (with.offence.character_revision, with.offence.equipment_revision, with.defence.equipment_revision));

    // Clean baseline: the same fight with every strippable source out of the equipment.
    let (stripped, clean) = if with.offence_sources() > 0 { strip(&mut client).await? } else { (Vec::new(), true) };
    let (idle_hp, idle_mp) = idle_rate(&mut client, 8).await?;
    let mut base_hits = Vec::new();
    let mut base_taken = 0;
    for _ in 0..3 {
        let target = reachable_monster(&mut client, world, &metadata.spawns).await?;
        let base = fight(&mut client, target, FIGHT_SECONDS, 12).await?;
        base_hits.extend(base.own_hits(me));
        base_taken += base
            .seen
            .iter()
            .filter(|seen| seen.1 == me && seen.2 == Some(target))
            .count();
        if base_hits.len() >= 6 && base_taken > 0 {
            break;
        }
    }
    for (slot, handle) in &stripped {
        restore(&mut client, *slot, *handle).await?;
    }
    let after_restore = Stats::read(&mut client).await?;
    if !after_restore.same(&with) {
        return Err("re-equipping the stripped items did not restore the projections".into());
    }
    if base_hits.len() >= 6 && base_taken > 0 {
        row.pass("baseline", (&base_hits, base_taken, idle_hp, idle_mp, clean));
    } else {
        row.fail("baseline", format!("attributed-player-hits={};monster-hits-on-player={base_taken};needed>=6-and>=1", base_hits.len()));
    }
    let base_max = base_hits.iter().copied().max().unwrap_or(0);

    let engaged = reachable_monster(&mut client, world, &metadata.spawns).await?;
    let hunt = fight(&mut client, engaged, FIGHT_SECONDS * 2, 40).await?;
    let hits = hunt.own_hits(me);

    // critical: a hit marked critical by the server while the projection is positive.
    let (chance, damage) = (with.off(Stat::CriticalChance), with.off(Stat::CriticalDamage));
    if chance <= 0 || damage <= 0 {
        row.fail("critical", needed("critical-chance-and-damage", chance.min(damage)));
    } else if !clean {
        row.fail("critical", "needed=clean-non-critical-baseline;a-source-that-cannot-be-unequipped-remains");
    } else {
        // A hit above a sampled clean maximum can still be an ordinary roll; only a server mark
        // on the hit itself separates a critical from ordinary damage.
        row.fail("critical", format!("{UNMARKED};clean-max={base_max};hunt-max={:?}", hits.iter().max()));
    }

    // leech: attacker vitals must rise beyond idle regeneration, with the expected leech non-zero.
    let (life, mana) = (with.off(Stat::LifeLeech), with.off(Stat::ManaLeech));
    let dealt: u64 = hits.iter().sum();
    let vitals = *client.actor_vitals().ok_or("vitals-absent")?;
    if life <= 0 || mana <= 0 {
        row.fail(
            "leech",
            format!("needed=both-leech-sources;life={life};mana={mana}"),
        );
    } else {
        let life_ok = life > 0
            && dealt.saturating_mul(life as u64) / 1_000_000 >= 1
            && hunt.health_up as f64 > idle_hp * hunt.seconds;
        let mana_ok = mana > 0
            && dealt.saturating_mul(mana as u64) / 1_000_000 >= 1
            && hunt.mana_up as f64 > idle_mp * hunt.seconds;
        if life_ok && mana_ok {
            row.pass(
                "leech",
                (life, mana, hunt.health_up, hunt.mana_up, idle_hp, idle_mp),
            );
        } else {
            row.fail("leech", format!(
                "life-projection={life} health-up={} idle={:.2}/s;mana-projection={mana} mana-up={} idle={:.2}/s;dealt={dealt};vitals={vitals:?}",
                hunt.health_up, idle_hp, hunt.mana_up, idle_mp
            ));
        }
    }

    // reflect: a monster hits the player and the same decision damages that monster, not by the player.
    let reflection = with.def(Stat::PhysicalReflection);
    if reflection <= 0 {
        row.fail("reflect", needed("physical-reflection", reflection));
    } else {
        let reflected = hunt.seen.iter().any(|(ordinal, victim, attacker, _)| {
            *victim != me
                && *attacker != Some(me)
                && hunt.seen.iter().any(|(o, t, a, _)| o == ordinal && *t == me && *a == Some(*victim))
        });
        let taken = hunt.seen.iter().filter(|s| s.1 == me).count();
        if reflected {
            row.pass("reflect", (reflection, taken));
        } else {
            row.fail("reflect", format!("projection={reflection};monster-hits-on-player={taken};no reflected damage on an attacking monster"));
        }
    }

    // cleave: a second adjacent monster takes player damage after one target command.
    let cleave = with.off(Stat::Cleave);
    if cleave <= 0 {
        row.fail("cleave", needed("cleave", cleave));
    } else if let Some(flank) = hunt.flank {
        let second: Vec<u64> = hunt.seen.iter().filter(|(_, t, a, _)| *t == flank && *a == Some(me)).map(|s| s.3).collect();
        if second.is_empty() {
            row.fail("cleave", format!("projection={cleave};second-monster-adjacent-but-took-no-player-damage"));
        } else {
            row.pass("cleave", (cleave, second));
        }
    } else {
        row.fail("cleave", format!("projection={cleave};needed=second-monster-adjacent-to-the-target"));
    }

    // perfect shot: a hit at the projected distance marked as a perfect shot.
    let shots = with.perfect_shots();
    if shots.is_empty() {
        row.fail("perfect-shot", needed("perfect-shot", 0));
    } else if let Some((distance, bonus)) = shots.iter().copied().find(|(d, _)| *d == 1) {
        // An ordinary hit can exceed the flat bonus, so the lowest hit proves nothing on its own.
        let floor = hits.iter().copied().min().unwrap_or(0);
        row.fail("perfect-shot", format!("{UNMARKED};distance={distance};bonus={bonus};lowest-hit={floor}"));
    } else {
        row.fail("perfect-shot", format!("projected={shots:?};needed=ranged-attack-at-the-projected-distance-that-this-adjacent-fight-cannot-produce"));
    }

    // fatal: a hit marked fatal by the server while the onslaught projection is positive.
    let fatal = with.off(Stat::OnslaughtEquipment);
    if fatal <= 0 {
        row.fail("fatal", needed("onslaught-from-a-tiered-weapon", fatal));
    } else if !clean {
        row.fail("fatal", "needed=clean-baseline;a-source-that-cannot-be-unequipped-remains");
    } else {
        row.fail("fatal", format!("{UNMARKED};projection={fatal};clean-max={base_max};hunt-max={:?}", hits.iter().max()));
    }

    // refusal and durability.
    let before_vitals = *client.actor_vitals().ok_or("vitals-absent")?;
    let before_combat = client.session_mut().combat_state().copied()
        .ok_or("combat-state-absent-before-self-target")?;
    let before_inventory = custody(&client)?;
    let refusal = client.session_mut().attack_target(Some(&me)).await?;
    let refusal_inert = client.actor_vitals() == Some(&before_vitals)
        && client.session_mut().combat_state() == Some(&before_combat)
        && custody(&client)? == before_inventory;
    client.service_liveness(Duration::from_millis(300)).await?;
    let self_damage: Vec<_> = client
        .take_events()
        .iter()
        .fold(Vec::new(), |mut seen, e| {
            collect(e, &mut seen);
            seen
        })
        .into_iter()
        .filter(|s| s.1 == me && s.2 == Some(me))
        .collect();
    let inventory = custody(&client)?;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_world(clients, 0xc4).await?;
    let after = Stats::read(&mut fresh).await?;
    let durable = fresh.join_snapshot().game_session_id != first_session
        && custody(&fresh)? == inventory
        && after.same(&after_restore)
        && after.offence.equipment_revision == after_restore.offence.equipment_revision
        && after.defence.equipment_revision == after_restore.defence.equipment_revision;
    if refusal.disposition == AttackIntentDisposition::TargetNotACreature
        && refusal_inert && self_damage.is_empty() && durable
    {
        row.pass("refusal-relog", (refusal, before_vitals.health));
    } else {
        row.fail("refusal-relog", format!("self-target-reply={refusal:?};self-damage={self_damage:?};inert={refusal_inert};durable={durable}"));
    }
    logout(&mut fresh).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pass_requires_every_step_and_no_failure() {
        let mut row = Row::new();
        for step in STEPS {
            row.pass(step, "ok");
        }
        assert!(row.complete());
        let mut row = Row::new();
        for step in STEPS {
            row.pass(step, "ok");
        }
        row.failures.push(("cleave", "x".into()));
        assert!(!row.complete());
        let mut row = Row::new();
        row.pass("tabs", "ok");
        assert!(!row.complete());
    }

    #[test]
    fn a_failed_step_cannot_later_pass() {
        let mut row = Row::new();
        row.fail("leech", "no vitals increase");
        row.pass("leech", "late");
        assert!(row.passed.is_empty() && row.failures.len() == 1);
    }

    #[test]
    fn baseline_route_reaches_compiled_town_spawn() -> TestResult {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| check_compiled_town_route().map_err(|error| error.to_string()))?
            .join()
            .map_err(|_| "compiled town route test thread panicked")??;
        Ok(())
    }

    fn check_compiled_town_route() -> TestResult {
        use oteryn_world_bundle_compiler::{bundle::BuildClass, compile, source};

        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let regions: Vec<_> =
            include_str!("../../../../../tools/qualification/node_boot/town-regions.txt")
                .trim()
                .split(';')
                .map(|region| -> TestResult<_> {
                    let (x, y) = region.split_once(',').ok_or("invalid town region")?;
                    Ok((x.parse()?, y.parse()?))
                })
                .collect::<TestResult<_>>()?;
        let project = source::project_regions(&root, &regions)?;
        let (registry, palette) = source::registry(&root, &project)?;
        let input = compile::Input {
            regions: &project.regions,
            palette: &palette,
            identity: serde_json::from_slice(&std::fs::read(
                root.join("content/world/pins/oteryn.identity.json"),
            )?)?,
            world: source::world_extent(&root)?,
            build_class: BuildClass::NonProduction,
            draft_areas: oteryn_world_bundle_compiler::project::draft_areas(&project.index)?,
            families: &project.families,
        };
        let compiled = compile::compile(&input, &registry)?;
        let metadata = oteryn_world_bundle::bundle::visit(&compiled.bytes, |_, _| Ok(()))?;
        let mut pin: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("content/world/pins/oteryn.json"))?)?;
        pin["digest"] = compiled
            .digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
            .into();
        let world = &boot_world(
            compiled.bytes,
            &pin,
            WorldId::decode(&v7(1, 2))?,
            ChannelId::decode(&v7(1, 3))?,
        )?
        .bundle;
        let initial = baseline_route(world, TOWN, &metadata.spawns, |_| true, &[])
            .ok_or("C03 baseline could not reach a compiled town spawn")?;
        let first = *initial.first().ok_or("C03 route must leave the temple")?;
        let occupied = destination(world, TOWN, first).ok_or("C03 route has invalid first step")?;
        let path = baseline_route(world, TOWN, &metadata.spawns, |at| at != occupied, &[])
            .ok_or("C03 baseline could not route around an occupied temple exit")?;
        let mut at = TOWN;
        for step in path {
            at = destination(world, at, step).ok_or("C03 route has invalid step")?;
            assert_ne!(at, occupied);
        }
        assert_ne!(at, TOWN);
        let target = full_e2ef3::ordinary_spawn_points(&metadata.spawns)
            .map(|(spawn, _)| spawn)
            .find(|spawn| {
                spawn.floor == at.floor
                    && at.x.abs_diff(spawn.x).max(at.y.abs_diff(spawn.y)) <= 1
                    && outside_protection(world, *spawn)
            })
            .ok_or("C03 compiled route ended beside no spawn")?;
        let open = live_approach_route(world, TOWN, target, &BTreeSet::from([target]))
            .ok_or("C03 live approach has no route from town")?;
        let blocked = destination(world, TOWN, *open.first().ok_or("C03 approach must move")?)
            .ok_or("C03 approach first step invalid")?;
        let alternate = live_approach_route(world, TOWN, target, &BTreeSet::from([target, blocked]))
            .ok_or("C03 live approach must detour around the other actor")?;
        let mut cell = TOWN;
        for step in alternate {
            cell = destination(world, cell, step).ok_or("C03 live approach step invalid")?;
            assert_ne!(cell, blocked);
        }

        Ok(())
    }

    #[test]
    fn failed_admission_remains_separate_from_unexecuted_tabs() {
        let mut row = Row::new();
        assert_eq!(STEPS.first(), Some(&"admission"));
        row.fail("admission", "fixture-release closed the connection");
        assert!(!row.complete());
        assert_eq!(
            row.failures,
            [("admission", "fixture-release closed the connection".to_owned())]
        );
        assert!(!row.passed.contains(&"tabs"));
    }

    #[test]
    fn evidence_token_has_no_whitespace() {
        assert!(!token("a b\"c\n").contains(char::is_whitespace));
    }
}
