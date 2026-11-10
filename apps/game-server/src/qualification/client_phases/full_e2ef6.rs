//! Starter-backpack, temple-respawn and first-vocation journeys on the ordinary Character.
//! No provisioning: all assertions consume native admission, command or delta frames.
use super::starter_inventory::check_starter;
use super::*;
use oteryn_dev_client::{CharacterInventory, DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::{item_view as items, logout::LogoutResult};
use std::collections::BTreeSet;
use std::num::NonZeroU32;

const STARTER: &[&str] = &[
    "first-entry-backpack",
    "invalid-equip-refused",
    "relog-custody-no-second-grant",
    "second-relog-no-second-grant",
];
const TEMPLE: &[&str] = &[
    "away-from-temple",
    "incoming-damage",
    "in-fight-logout-refused",
    "lethal-damage-receipt",
    "exact-temple-full-health",
    "relog-saved-temple-full-health",
];
const VOCATION: &[&str] = &[
    "oressa-reachable",
    "unquoted-confirmation-refused",
    "underlevel-choice-refused",
    "earned-level-eight",
    "declined-choice-unchanged",
    "confirmed-first-choice",
    "second-choice-refused",
    "relog-vocation-xp-level",
];

struct Row {
    id: &'static str,
    required: &'static [&'static str],
    passed: BTreeSet<&'static str>,
    invalid_action: bool,
    started: std::time::Instant,
}

impl Row {
    fn new(id: &'static str, required: &'static [&'static str]) -> Self {
        Self {
            id,
            required,
            passed: BTreeSet::new(),
            invalid_action: false,
            started: std::time::Instant::now(),
        }
    }

    fn pass(&mut self, action: &'static str, observed: impl std::fmt::Debug) {
        if !self.required.contains(&action) || !self.passed.insert(action) {
            self.invalid_action = true;
            return;
        }
        evidence(&format!(
            "E2EF6_CHECK {} {action} PASS observed={observed:?}",
            self.id
        ));
    }

    fn complete(&self) -> bool {
        !self.required.is_empty()
            && !self.invalid_action
            && self
                .required
                .iter()
                .all(|action| self.passed.contains(action))
    }

    fn finish(self, result: TestResult) {
        for action in self.required {
            if !self.passed.contains(action) {
                evidence(&format!(
                    "E2EF6_CHECK {} {action} FAIL assertion-not-completed",
                    self.id
                ));
            }
        }
        if let Err(error) = &result {
            evidence(&format!("E2EF6_GAP {} {error}", self.id));
        }
        evidence(&format!(
            "e2e_all scenario={}.full status={} seconds={:.3}",
            self.id,
            if result.is_ok() && self.complete() {
                "PASS"
            } else {
                "FAIL"
            },
            self.started.elapsed().as_secs_f64(),
        ));
    }
}

pub(super) async fn run_starter(clients: &SeamClients<'_>) {
    let mut starter = Row::new("C19a", STARTER);
    let result = tokio::time::timeout(
        Duration::from_secs(100),
        starter_backpack(&mut starter, clients),
    )
    .await
    .unwrap_or_else(|_| Err("starter journey deadline; remaining actions unproven".into()));
    starter.finish(result);
}

pub(super) async fn run(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    npcs: &oteryn_world_bundle::npc::Table,
) {
    // Own a connection per row: an error/timeout cannot prevent the next row's verdict.
    let mut temple = Row::new("C20d", TEMPLE);
    let result = tokio::time::timeout(
        Duration::from_secs(330),
        temple_respawn(&mut temple, clients, world, spawns),
    )
    .await
    .unwrap_or_else(|_| Err("temple journey deadline; remaining actions unproven".into()));
    temple.finish(result);

    let mut vocation = Row::new("S16a", VOCATION);
    let result = tokio::time::timeout(
        Duration::from_secs(420),
        first_vocation(&mut vocation, clients, world, spawns, npcs),
    )
    .await
    .unwrap_or_else(|_| Err("Oressa journey deadline; remaining actions unproven".into()));
    vocation.finish(result);

    let mut loss = Row::new("C20a", EXPERIENCE_LOSS);
    let result = tokio::time::timeout(
        Duration::from_secs(1100),
        experience_loss(&mut loss, clients, world, spawns, npcs),
    )
    .await
    .unwrap_or_else(|_| Err("experience-loss journey deadline; remaining actions unproven".into()));
    match result {
        Ok(Some(source)) => finish_missing_receipt(loss, source),
        other => loss.finish(other.map(|_| ())),
    }

    let mut custody = Row::new("C20b", ITEM_LOSS);
    let result = tokio::time::timeout(
        Duration::from_secs(850),
        item_loss(&mut custody, clients, world, spawns, npcs),
    )
    .await
    .unwrap_or_else(|_| Err("item-loss journey deadline; remaining actions unproven".into()));
    custody.finish(result);

    // Purchase qualification precedes consumption: Character 0 must retain its
    // never-aggressed low-level Adventurer eligibility for both protected deaths.
    let mut services = Row::new("P16", BLESSING_SERVICES);
    let result = tokio::time::timeout(
        Duration::from_secs(1800),
        blessing_services(&mut services, clients, world, spawns, npcs),
    )
    .await
    .unwrap_or_else(|_| {
        Err("blessing services journey deadline; remaining actions unproven".into())
    });
    finish_blessing_row(services, result);

    let mut consumption = Row::new("C20c", BLESSING_DEATH);
    let result = tokio::time::timeout(
        Duration::from_secs(1500),
        blessing_death(&mut consumption, clients, world, spawns, npcs),
    )
    .await
    .unwrap_or_else(|_| {
        Err("blessing consumption journey deadline; remaining actions unproven".into())
    });
    finish_blessing_row(consumption, result);

    let mut resting = Row::new("P10", RESTING_BONUSES);
    let result = tokio::time::timeout(
        Duration::from_secs(1800),
        resting_bonuses(&mut resting, clients, world, spawns),
    )
    .await
    .unwrap_or_else(|_| Err("P10 resting journey deadline; remaining actions unproven".into()));
    finish_resting_row(resting, result);
}

// P10 owns its connection and never borrows the fixture-provisioned character.
const RESTING_BONUSES: &[&str] = &[
    "unopened-claim-refused",
    "shrine-open-and-claim",
    "duplicate-claim-refused",
    "relog-daily-streak",
    "earned-seven-day-streak",
    "player-earned-deficits",
    "resting-double-health-mana",
    "resting-soul-and-stamina",
    "relog-bonus-state",
    "outside-bonuses-stop",
    "reentry-restarts-cadence",
    "resting-area-classification",
];

fn resting_stamina(payload: &[u8]) -> TestResult<u16> {
    progress_frame(payload)?;
    let minutes = u16::from_le_bytes(payload[29..31].try_into()?);
    if minutes > 2520 {
        return Err(format!("native stamina exceeds 2520: {minutes}").into());
    }
    Ok(minutes)
}

// Independent integer oracle for the packet's earned knight/elite-knight build.
// Carries are private: one completed period can credit floor or ceil of the
// percentage/legacy floor. Doubling applies to that integer credit, before cap.
fn resting_credit(before: u32, after: u32, maximum: u32, units: u64, multiplier: u64) -> bool {
    if after <= before || after >= maximum {
        return false; // A capped or unchanged frame cannot prove the multiplier.
    }
    let gain = u64::from(after - before);
    gain == multiplier * (units / 10_000) || gain == multiplier * units.div_ceil(10_000)
}

async fn daily_wall(
    client: &mut DevClientSession,
) -> TestResult<oteryn_protocol_oteryn::daily_reward::DailyRewardWall> {
    use oteryn_dev_client::client_runtime::daily_reward::{
        DailyRewardCommand, DailyRewardReply, decode_reply,
    };
    use oteryn_protocol_oteryn::daily_reward as wire;
    // Unlike the UI convenience Query, this raw Query does not implicitly USE.
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_DAILY_REWARD_V2,
            wire::COMMAND_TYPE_DAILY_REWARD_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("P10 daily wall query refused: {reply:?}").into());
    }
    match decode_reply(DailyRewardCommand::Query, &reply.payload)? {
        DailyRewardReply::Wall(wall) => Ok(wall),
        other => Err(format!("P10 expected native daily wall: {other:?}").into()),
    }
}

async fn daily_claim(client: &mut DevClientSession, disposition: u8) -> TestResult {
    use oteryn_dev_client::client_runtime::daily_reward::{
        DailyRewardCommand, DailyRewardReply, send,
    };
    let reply = send(client.session_mut(), &DailyRewardCommand::Claim).await?;
    let expected_status = if disposition <= 2 {
        CommandStatus::Accepted
    } else {
        CommandStatus::Rejected
    };
    if reply.status != expected_status
        || !matches!(reply.reply,
        Some(DailyRewardReply::Result(result)) if result.disposition == disposition
            && result.transaction_id.is_some() == (disposition == 1))
    {
        return Err(format!(
            "P10 daily claim expected disposition={disposition}, observed={reply:?}"
        )
        .into());
    }
    Ok(())
}

fn resting_tiles(world: &BundleWorld, start: TilePos) -> TestResult<(TilePos, TilePos)> {
    use crate::map::view::MapFacts;
    use oteryn_protocol_oteryn::world_map::MapDefinition;
    let content = crate::content::daily_reward::DailyRewardContent::catalogue()
        .ok_or("P10 production daily reward catalogue unavailable")?;
    let index = provisioning::item_index()?;
    let references = content
        .shrine_source_ids
        .iter()
        .map(|id| provisioning::item_reference(&index, *id))
        .collect::<TestResult<Vec<_>>>()?;
    let facts = world.facts().pinned();
    let mut shrines = Vec::new();
    for (floor, x, y, tile) in world.base().tiles() {
        if start.x.abs_diff(x) > 160 || start.y.abs_diff(y) > 160 {
            continue;
        }
        let at = TilePos { floor, x, y };
        if tile.ids().iter().zip(tile.depths()).filter(|(_, depth)| **depth == 0)
            .enumerate().any(|(ordinal, (id, _))| {
                u8::try_from(ordinal).ok().and_then(|ordinal| facts.base_entry(at, ordinal, *id))
                    .is_some_and(|entry| matches!(entry.definition, MapDefinition::Item(reference) if references.contains(&reference)))
            }) {
            shrines.push(at);
        }
    }
    shrines.sort_by_key(|at| (start.x.abs_diff(at.x).max(start.y.abs_diff(at.y)), *at));
    for shrine in shrines {
        let Some(path) = route(world, start, |at| {
            at.floor == shrine.floor
                && at.x.abs_diff(shrine.x).max(at.y.abs_diff(shrine.y)) <= 1
                && world
                    .base()
                    .tile(at.x, at.y, at.floor)
                    .is_some_and(|tile| tile.protection_zone())
        }) else {
            continue;
        };
        // Resolve the exact endpoint through the same bundle movement oracle.
        let mut inside = start;
        for direction in path {
            inside = destination(world, inside, direction)
                .ok_or("P10 shrine path endpoint unavailable")?;
        }
        let outside_path = route(world, inside, |at| {
            at.floor == inside.floor
                && at.x.abs_diff(inside.x).max(at.y.abs_diff(inside.y)) >= 4
                && world
                    .base()
                    .tile(at.x, at.y, at.floor)
                    .is_some_and(|tile| !tile.protection_zone())
        });
        if let Some(path) = outside_path {
            let mut outside = inside;
            for direction in path {
                outside = destination(world, outside, direction)
                    .ok_or("P10 outside path endpoint unavailable")?;
            }
            return Ok((inside, outside));
        }
    }
    Err("P10 player route blocked: no reachable production reward shrine with a PZ approach and outside return route in the 160-tile town window".into())
}

async fn resting_walk(
    client: &mut DevClientSession,
    world: &BundleWorld,
    to: TilePos,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let path = route(world, from, |at| at == to)
        .ok_or_else(|| format!("P10 player walking route blocked: {from:?}->{to:?}"))?;
    walk(client, world, &path).await?;
    if tile(client.world_spatial().actor_position)? != to {
        return Err(format!("P10 server STEP did not reach {to:?}").into());
    }
    Ok(())
}

// Time the observed outside->PZ STEP, rather than the final shrine approach.
async fn resting_reentry(
    client: &mut DevClientSession,
    world: &BundleWorld,
    inside: TilePos,
) -> TestResult<tokio::time::Instant> {
    let mut at = tile(client.world_spatial().actor_position)?;
    let path = route(world, at, |at| at == inside).ok_or("P10 reentry walking route blocked")?;
    let mut entered = None;
    for direction in path {
        let to = destination(world, at, direction).ok_or("P10 reentry destination absent")?;
        let is_pz = |at: TilePos| {
            world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|tile| tile.protection_zone())
        };
        if is_pz(to) && !is_pz(at) {
            entered = Some(tokio::time::Instant::now());
        } else if !is_pz(to) {
            entered = None;
        }
        walk(client, world, std::slice::from_ref(&direction)).await?;
        at = tile(client.world_spatial().actor_position)?;
        if at != to {
            return Err("P10 reentry STEP did not reach the native bundle destination".into());
        }
    }
    entered.ok_or_else(|| "P10 reentry did not observe an outside-to-PZ boundary crossing".into())
}

async fn resting_spend(client: &mut DevClientSession, soul: bool) -> TestResult {
    use oteryn_protocol_oteryn::{
        actor_spell::{SpellCastDisposition, SpellTarget},
        actor_spell_catalog as wire,
    };
    let snapshot = client
        .session_mut()
        .gated_snapshot(wire::STATE_DOMAIN_ACTOR_SPELL_CATALOG)
        .ok_or("P10 player spell prerequisite blocked: native own-actor spell catalogue absent")?;
    let entries = wire::decode_catalog(&snapshot.payload)
        .map_err(|error| format!("P10 served spell catalogue: {error:?}"))?;
    let spell = entries.iter().find(|entry| entry.available && entry.rune_item.is_none()
        && if soul { entry.soul > 0 } else { entry.words == "exura" })
        .ok_or_else(|| format!("P10 player {} deficit blocked: no available learned {} spell in the admitted native catalogue; no vocation, learning, Premium or reagent grant allowed", if soul { "soul" } else { "mana" }, if soul { "soul-consuming" } else { "exura" }))?;
    let before = client
        .actor_vitals()
        .copied()
        .ok_or("P10 pre-cast vitals absent")?;
    let result = client
        .cast_spell(
            NonZeroU32::new(spell.cast_index).ok_or("P10 served spell index zero")?,
            SpellTarget::None,
            false,
        )
        .await?;
    if result.disposition != SpellCastDisposition::Cast {
        return Err(format!(
            "P10 real player {} deficit cast {} refused: {result:?}; ordinary items={:?}",
            if soul { "soul" } else { "mana" },
            spell.words,
            client.inventory()
        )
        .into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let after = client
            .actor_vitals()
            .copied()
            .ok_or("P10 post-cast vitals absent")?;
        if if soul {
            after.soul < before.soul
        } else {
            after.mana < before.mana
        } {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "P10 accepted cast supplied no {} debit frame: {before:?}->{after:?}",
                if soul { "soul" } else { "mana" }
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
}

async fn resting_deficits(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    outside: TilePos,
) -> TestResult {
    // A real credited kill debits stamina. No combat/minute override is used.
    let before = progress(client).await?;
    travel_to_spawn(client, world, spawns, "oteryn:creature.rat").await?;
    client_hunt::hunt(client).await.map_err(|error| {
        format!("P10 earned stamina/HP deficit prerequisite: real rat hunt: {error}")
    })?;
    if progress(client).await?.0 <= before.0 {
        return Err("P10 stamina prerequisite blocked: real rat hunt produced no earned XP".into());
    }
    resting_walk(client, world, outside).await?;
    relog(client, clients, 0xf3).await?; // Clears the unrelated kill-triggered soul window.
    let stamina = resting_stamina(&tab(client, 1).await?)?;
    if stamina >= 2520 {
        return Err(format!("P10 stamina deficit blocked after earned kill and fresh admission: observed={stamina}, need <2520").into());
    }
    resting_spend(client, false).await?;
    resting_spend(client, true).await?;
    // Exura can heal: obtain HP loss after resource spending, without another
    // kill that could reopen the unrelated soul window.
    // Hunt closes its target; return to an actual spawn for nonlethal damage.
    travel_to_spawn(client, world, spawns, "oteryn:creature.rat").await?;
    // Wait for nonlethal native damage, then physically retreat. A death fails this setup.
    let health = client
        .actor_vitals()
        .ok_or("P10 hunt vitals missing")?
        .health;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(40);
    while client
        .actor_vitals()
        .is_none_or(|v| v.health >= health || v.max_health - v.health < 8)
    {
        if tokio::time::Instant::now() >= deadline {
            return Err("P10 HP deficit prerequisite blocked: no nonlethal native rat damage with at least eight missing HP".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    if client.actor_vitals().is_none_or(|v| v.health == 0) {
        return Err("P10 deficit setup died instead of retaining nonlethal HP loss".into());
    }
    resting_walk(client, world, outside).await?;
    let vitals = client
        .actor_vitals()
        .ok_or("P10 final deficit vitals absent")?;
    if vitals.health == 0
        || vitals.max_health - vitals.health < 8
        || vitals.max_mana - vitals.mana < 2
    {
        return Err(format!("P10 actual travel/regeneration exhausted the earned HP/mana deficits before resting: {vitals:?}").into());
    }
    Ok(())
}

fn finish_resting_row(row: Row, result: TestResult<Option<&'static str>>) {
    if let Ok(Some(source)) = result {
        for action in row.required {
            if !row.passed.contains(action) {
                evidence(&format!(
                    "E2EF6_CHECK {} {action} FAIL assertion-not-completed",
                    row.id
                ));
            }
        }
        evidence(&format!(
            "E2EF6_GAP {} server ticks qualify all PZ tiles, without an admitted narrower resting-area classification; no native player route can prove the area distinction source:{source}",
            row.id
        ));
        evidence(&format!(
            "e2e_all scenario={}.full status=MISSING-SERVER seconds={:.3} evidence=source:{source}",
            row.id,
            row.started.elapsed().as_secs_f64()
        ));
    } else {
        row.finish(result.map(|_| ()));
    }
}

async fn resting_bonuses(
    row: &mut Row,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult<Option<&'static str>> {
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    let mut client = connect_world(clients, 0xf2).await?;
    let result: TestResult<Option<&'static str>> = async {
        let initial = daily_wall(&mut client).await?;
        if initial.claimed || initial.claim_enabled {
            return Err(format!("P10 fresh-day prerequisite blocked: expected an unclaimed, unopened wall on this fresh GameSession; observed={initial:?}").into());
        }
        let held = custody(client.inventory().ok_or("P10 initial custody missing")?);
        daily_claim(&mut client, 4).await?;
        if daily_wall(&mut client).await? != initial || custody(client.inventory().ok_or("P10 refused-claim custody missing")?) != held {
            return Err("P10 unopened refusal changed daily state or item custody".into());
        }
        row.pass("unopened-claim-refused", initial);
        let (inside, outside) = resting_tiles(world, tile(client.world_spatial().actor_position)?)?;
        resting_walk(&mut client, world, inside).await?;
        let opened = client.session_mut().use_object(b"oteryn:placement/daily-reward-shrine", 0).await?;
        if opened.disposition != UseDisposition::Committed {
            return Err(format!("P10 real shrine USE refused at {inside:?}: {opened:?}").into());
        }
        let ready = daily_wall(&mut client).await?;
        if !ready.claim_enabled || ready.claimed || ready.streak != initial.streak || ready.lane_day != initial.lane_day {
            return Err(format!("P10 shrine USE failed to open current daily claim: {ready:?}").into());
        }
        daily_claim(&mut client, 1).await?;
        let claimed = daily_wall(&mut client).await?;
        if !claimed.claimed || claimed.claim_enabled || claimed.streak != initial.streak.saturating_add(1).min(100_000) {
            return Err(format!("P10 real daily claim did not advance consecutive streak once: {initial:?}->{claimed:?}").into());
        }
        row.pass("shrine-open-and-claim", claimed);
        let awarded = custody(client.inventory().ok_or("P10 reward custody missing")?);
        daily_claim(&mut client, 2).await?;
        if daily_wall(&mut client).await? != claimed || custody(client.inventory().ok_or("P10 duplicate custody missing")?) != awarded {
            return Err("P10 duplicate claim changed daily state or item custody".into());
        }
        row.pass("duplicate-claim-refused", claimed);
        relog(&mut client, clients, 0xf3).await?;
        if daily_wall(&mut client).await? != claimed || custody(client.inventory().ok_or("P10 saved reward custody missing")?) != awarded {
            return Err("P10 daily claim/streak/custody did not persist through fresh admission".into());
        }
        row.pass("relog-daily-streak", claimed);
        if claimed.streak < 7 {
            return Err(format!("P10 seven-day streak prerequisite blocked: native claimed wall streak={} lane_day={} claimed={}; HP/mana unlock at 2/3, Premium stamina at 4, doubled HP/mana at 5/6 and soul at 7. Real player route permits one claim per committed World server-save epoch; {} further consecutive daily claims are required. This bounded run cannot advance server time/epochs or mint a streak", claimed.streak, claimed.lane_day, claimed.claimed, 7 - claimed.streak).into());
        }
        row.pass("earned-seven-day-streak", claimed);
        resting_deficits(&mut client, clients, world, spawns, outside).await?;
        let look = self_look(&mut client).await?;
        let promoted = look.ends_with(", elite knight).");
        if !promoted && !look.ends_with(", knight).") {
            return Err(format!("P10 ordinary knight prerequisite blocked: native build={look}").into());
        }
        let initial_vitals = client.actor_vitals().copied().ok_or("P10 deficit vitals missing")?;
        row.pass("player-earned-deficits", (initial_vitals, resting_stamina(&tab(&mut client, 1).await?)?));
        resting_walk(&mut client, world, inside).await?;
        daily_wall(&mut client).await?; // Install the saved streak; do not reset cadence during observation.
        client.take_events();
        let mut previous = client.actor_vitals().copied().ok_or("P10 resting vitals missing")?;
        let stamina_before = resting_stamina(&tab(&mut client, 1).await?)?;
        let soul_before = previous.soul;
        let mut health = false;
        let mut mana = false;
        let mut soul = false;
        let started = tokio::time::Instant::now();
        let deadline = started + Duration::from_secs(390);
        let mut next_stamina_read = started;
        let mut stamina = stamina_before;
        let stamina_after = loop {
            client.service_liveness(Duration::from_millis(50)).await?;
            for event in client.take_events() {
                if let SessionEvent::ActorVitals(delta) = event {
                    let v = delta.value;
                    if v.health == 0 || v.health < previous.health || v.mana < previous.mana || v.soul < previous.soul
                        || v.health > v.max_health || v.mana > v.max_mana {
                        return Err(format!("P10 unrelated damage/debit/death or cap violation during resting: {previous:?}->{v:?}").into());
                    }
                    if v.health > previous.health && v.health < v.max_health {
                        let units = (u64::from(v.max_health) * if promoted { 15 } else { 10 }).max(if promoted { 7500 } else { 5000 });
                        if !resting_credit(previous.health, v.health, v.max_health, units, 2) {
                            return Err(format!("P10 resting HP gain is not the doubled integer credit: {previous:?}->{v:?}").into());
                        }
                        // Where base/double floor/ceil sets overlap, wait for a credit
                        // that actually distinguishes the multiplier from a base tick.
                        health |= u64::from(v.health - previous.health) > units.div_ceil(10_000);
                    }
                    if v.mana > previous.mana && v.mana < v.max_mana {
                        let units = (u64::from(v.max_mana) * if promoted { 10 } else { 8 }).max(10_000);
                        if !resting_credit(previous.mana, v.mana, v.max_mana, units, 2) {
                            return Err(format!("P10 resting mana gain is not the doubled integer credit: {previous:?}->{v:?}").into());
                        }
                        mana |= u64::from(v.mana - previous.mana) > units.div_ceil(10_000);
                    }
                    if v.soul > previous.soul {
                        if v.soul != previous.soul + 1 || started.elapsed() < Duration::from_secs(if promoted { 14 } else { 119 }) {
                            return Err(format!("P10 soul cadence/credit differs: {previous:?}->{v:?} elapsed={:?}", started.elapsed()).into());
                        }
                        soul = true;
                    }
                    previous = v;
                }
            }
            if tokio::time::Instant::now() >= next_stamina_read {
                stamina = resting_stamina(&tab(&mut client, 1).await?)?;
                next_stamina_read = tokio::time::Instant::now() + Duration::from_secs(1);
            }
            if stamina < stamina_before {
                return Err(format!("P10 stamina unexpectedly debited while resting: {stamina_before}->{stamina}").into());
            }
            if stamina > stamina_before {
                let interval = if stamina_before >= 2340 { 360 } else { 180 };
                if stamina != stamina_before + 1 || started.elapsed() < Duration::from_secs(interval - 1) {
                    return Err(format!("P10 stamina cadence/credit differs: {stamina_before}->{stamina} elapsed={:?}", started.elapsed()).into());
                }
                break stamina;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("P10 actual resting bonus deadline: doubled_hp={health} doubled_mana={mana} soul={soul} soul_before={soul_before} observed={previous:?} stamina={stamina_before}->{stamina}; Premium tiers require current membership and real deficits; no membership grant permitted").into());
            }
        };
        if !health || !mana || !soul {
            return Err(format!("P10 real resting observations incomplete: doubled_hp={health} doubled_mana={mana} soul={soul}, last={previous:?}").into());
        }
        row.pass("resting-double-health-mana", previous);
        row.pass("resting-soul-and-stamina", (soul_before, previous.soul, stamina_before, stamina_after));
        relog(&mut client, clients, 0xf3).await?;
        if daily_wall(&mut client).await? != claimed || resting_stamina(&tab(&mut client, 1).await?)? != stamina_after
            || client.actor_vitals().is_none_or(|v| v.soul != previous.soul || v.health < previous.health || v.mana < previous.mana) {
            return Err("P10 earned bonuses/streak did not persist through fresh admission".into());
        }
        row.pass("relog-bonus-state", (client.actor_vitals(), stamina_after));
        resting_deficits(&mut client, clients, world, spawns, outside).await?;
        let mut previous_outside = client.actor_vitals().copied().ok_or("P10 outside vitals absent")?;
        let mut base_health = false;
        let mut base_mana = false;
        let outside_soul = client.actor_vitals().ok_or("P10 outside vitals absent")?.soul;
        let outside_stamina = resting_stamina(&tab(&mut client, 1).await?)?;
        if outside_stamina >= 2520 {
            return Err("P10 outside negative-path stamina prerequisite capped; absence of a gain would be vacuous".into());
        }
        let mut next_stamina_read = tokio::time::Instant::now();
        let mut observed_stamina = outside_stamina;
        let until = tokio::time::Instant::now() + Duration::from_secs(370);
        while tokio::time::Instant::now() < until {
            client.service_liveness(Duration::from_millis(100)).await?;
            for event in client.take_events() {
                if let SessionEvent::ActorVitals(delta) = event {
                    let v = delta.value;
                    if v.health < previous_outside.health || v.mana < previous_outside.mana {
                        return Err(format!("P10 outside sample invalidated by damage/debit: {previous_outside:?}->{v:?}").into());
                    }
                    if v.health > previous_outside.health && v.health < v.max_health {
                        if !resting_credit(previous_outside.health, v.health, v.max_health,
                            (u64::from(v.max_health) * if promoted { 15 } else { 10 }).max(if promoted { 7500 } else { 5000 }), 1) {
                            return Err(format!("P10 resting HP multiplier leaked outside: {previous_outside:?}->{v:?}").into());
                        }
                        base_health = true;
                    }
                    if v.mana > previous_outside.mana && v.mana < v.max_mana {
                        if !resting_credit(previous_outside.mana, v.mana, v.max_mana,
                            (u64::from(v.max_mana) * if promoted { 10 } else { 8 }).max(10_000), 1) {
                            return Err(format!("P10 resting mana multiplier leaked outside: {previous_outside:?}->{v:?}").into());
                        }
                        base_mana = true;
                    }
                    previous_outside = v;
                }
            }
            if tokio::time::Instant::now() >= next_stamina_read {
                observed_stamina = resting_stamina(&tab(&mut client, 1).await?)?;
                next_stamina_read = tokio::time::Instant::now() + Duration::from_secs(1);
            }
            if tile(client.world_spatial().actor_position)? != outside
                || client.actor_vitals().is_none_or(|v| v.soul != outside_soul || v.health == 0)
                || observed_stamina != outside_stamina {
                return Err("P10 resting soul/stamina leaked outside PZ, or outside encounter invalidated isolation".into());
            }
        }
        if !base_health || !base_mana {
            return Err(format!("P10 outside base regeneration unproven: HP={base_health} mana={base_mana}; cap observations do not prove absence of a multiplier").into());
        }
        row.pass("outside-bonuses-stop", (outside, outside_soul, outside_stamina));
        let started = resting_reentry(&mut client, world, inside).await?;
        let interval = if promoted { 15 } else { 120 };
        let deadline = tokio::time::Instant::now() + Duration::from_secs(interval + 10);
        loop {
            client.service_liveness(Duration::from_millis(50)).await?;
            client.take_events();
            let soul = client.actor_vitals().ok_or("P10 reentry soul frame missing")?.soul;
            if soul != outside_soul {
                if soul != outside_soul + 1 || started.elapsed() < Duration::from_secs(interval - 1) {
                    return Err(format!("P10 reentry credited old/outside cadence: soul {outside_soul}->{soul}, elapsed={:?}", started.elapsed()).into());
                }
                row.pass("reentry-restarts-cadence", (soul, started.elapsed()));
                break;
            }
            if tokio::time::Instant::now() > deadline {
                return Err(format!("P10 no renewed resting soul credit after reentry: soul={outside_soul} interval={interval}").into());
            }
        }
        // The current server has only a PZ proxy, so even all accessible lifecycle
        // observations cannot prove correct membership in the narrower resting area.
        Ok(Some("crates/gameplay/dispatch/src/gameplay_transport/composed_admission/vitals.rs:137"))
    }.await;
    let cleanup = return_and_release(&mut client, world).await;
    match (result, cleanup) {
        (Err(error), _) | (_, Err(error)) => Err(error),
        (Ok(source), Ok(())) => Ok(source),
    }
}

#[cfg(test)]
mod resting_tests {
    use super::*;

    #[test]
    #[allow(
        dead_code,
        reason = "Function-local mount requires the isolated helper unit-test harness."
    )]
    fn resting_credit_distinguishes_multipliers_and_rejects_caps() {
        assert!(resting_credit(10, 12, 100, 5000, 2));
        assert!(!resting_credit(10, 11, 100, 5000, 2));
        assert!(resting_credit(10, 11, 100, 5000, 1));
        assert!(!resting_credit(10, 12, 100, 5000, 1));
        assert!(resting_credit(10, 12, 100, 15_001, 2));
        assert!(resting_credit(10, 14, 100, 15_001, 2));
        assert!(!resting_credit(10, 13, 100, 15_001, 2));
        assert!(!resting_credit(98, 100, 100, 10_000, 2));
        assert!(!resting_credit(100, 100, 100, 10_000, 2));
        assert!(!resting_credit(10, 9, 100, 10_000, 2));
    }

    #[test]
    #[allow(
        dead_code,
        reason = "Function-local mount requires the isolated helper unit-test harness."
    )]
    fn resting_stamina_requires_current_layout_and_bounded_minutes() -> TestResult {
        use oteryn_protocol_oteryn::cyclopedia_character::{GeneralStats, TabView};
        for minutes in [0, 2339, 2340, 2519, 2520] {
            let payload = TabView::GeneralStats(GeneralStats {
                level: 8,
                stamina_minutes: minutes,
                ..Default::default()
            })
            .encode()
            .map_err(|error| format!("stats encode: {error:?}"))?;
            assert_eq!(resting_stamina(&payload)?, minutes);
            assert!(resting_stamina(&payload[..41]).is_err());
        }
        let payload = TabView::GeneralStats(GeneralStats {
            level: 8,
            stamina_minutes: 2521,
            ..Default::default()
        })
        .encode()
        .map_err(|error| format!("stats encode: {error:?}"))?;
        assert!(resting_stamina(&payload).is_err());
        Ok(())
    }
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    loop {
        let logout = client.session_mut().logout().await?;
        match logout.result {
            Some(LogoutResult::Accepted) => return Ok(()),
            Some(LogoutResult::Busy | LogoutResult::InFight { .. })
                if tokio::time::Instant::now() < deadline => {}
            _ => return Err(format!("E2EF6 logout did not settle: {logout:?}").into()),
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
    }
}

async fn relog(client: &mut DevClientSession, clients: &SeamClients<'_>, tag: u8) -> TestResult {
    let previous = client.join_snapshot().game_session_id;
    logout(client).await?;
    *client = connect_world(clients, tag).await?;
    if client.join_snapshot().game_session_id == previous {
        return Err("fresh admission reused the preceding GameSession".into());
    }
    Ok(())
}

async fn starter_backpack(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0xe0).await?;
    let result: TestResult = async {
        let index = provisioning::item_index()?;
        let definition = provisioning::item_reference(&index, 2854)?;
        let entries = [
            provisioning::item_reference(&index, 3003)?,
            provisioning::item_reference(&index, 3457)?,
        ];
        check_starter(
            client
                .inventory()
                .ok_or("starter inventory snapshot missing")?,
            definition,
            entries,
        )?;
        row.pass("first-entry-backpack", client.inventory());

        // A container cannot become a weapon. Require the native refusal, then re-admit
        // to check both rollback and the first-entry grant's occupied-slot replay path.
        let source = client
            .inventory()
            .and_then(|inventory| inventory.main_backpack)
            .ok_or("starter main backpack missing")?
            .handle;
        let selection = items::ItemMoveSelection {
            equip_drop: client
                .selected_capabilities()
                .contains(&items::CAPABILITY_ITEM_EQUIP_DROP_V1),
            container_tree: false,
            world_interaction: false,
        };
        let payload = items::encode_item_move_intent_for(
            &items::ItemMoveIntent {
                source,
                destination: items::ItemMoveDestination::Equipment(items::EquipmentSlot::RightHand),
            },
            selection,
        )
        .map_err(|error| format!("starter equip intent: {error:?}"))?;
        let reply = client
            .session_mut()
            .gated_command(
                items::CAPABILITY_ITEM_VIEW_MOVE_V1,
                items::COMMAND_TYPE_ITEM_MOVE_INTENT,
                &payload,
            )
            .await?;
        let (payload, _) =
            oteryn_protocol_oteryn::refusal_detail::split_refusal_detail(&reply.payload)
                .map_err(|error| format!("starter equip refusal detail: {error:?}"))?;
        if reply.status != CommandStatus::Accepted
            || items::decode_item_move_result_for(payload, selection)
                .map_err(|error| format!("starter equip result: {error:?}"))?
                != items::ItemMoveOutcome::SlotMismatch
        {
            return Err(format!(
                "starter invalid equip was not refused as SlotMismatch: {reply:?}"
            )
            .into());
        }
        row.pass("invalid-equip-refused", &reply);

        relog(&mut client, clients, 0xe1).await?;
        check_starter(
            client
                .inventory()
                .ok_or("starter relog inventory missing")?,
            definition,
            entries,
        )?;
        row.pass("relog-custody-no-second-grant", client.inventory());
        relog(&mut client, clients, 0xe2).await?;
        check_starter(
            client
                .inventory()
                .ok_or("second starter relog inventory missing")?,
            definition,
            entries,
        )?;
        row.pass("second-relog-no-second-grant", client.inventory());
        Ok(())
    }
    .await;
    // Release this row's session even on an assertion failure; still run the next row.
    let released = logout(&mut client).await;
    result.and(released)
}

async fn temple_respawn(
    row: &mut Row,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult {
    let mut client = connect_world(clients, 0xe3).await?;
    let result: TestResult = async {
        // death() has a route-less no-op path; that path cannot qualify this row.
        if !client
            .selected_capabilities()
            .contains(&oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1)
        {
            return Err("temple journey requires the native bundle map capability".into());
        }
        // This unprovisioned Character has made no citizenship change. The bundle's
        // configured hometown fixes the expected temple BEFORE observing the death.
        let towns = &world.base().subareas;
        let home = towns
            .default_town
            .and_then(|id| towns.town(id))
            .and_then(|town| town.temple)
            .ok_or("compiled world has no default home temple")?;
        let temple = ActorPosition {
            x: i32::from(home.x),
            y: i32::from(home.y),
            floor: -i16::from(home.floor),
        };
        travel_to_spawn(&mut client, world, spawns, "oteryn:creature.poison_spider").await?;
        let encounter = client.world_spatial().actor_position;
        if encounter.floor == temple.floor
            && encounter
                .x
                .abs_diff(temple.x)
                .max(encounter.y.abs_diff(temple.y))
                <= 8
        {
            return Err("lethal encounter is not away from the configured temple".into());
        }
        row.pass("away-from-temple", (encounter, temple));
        client.session_mut().attack_target(None).await?;
        client.take_events();
        let before = client
            .actor_vitals()
            .ok_or("pre-death vitals absent")?
            .health;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(40);
        loop {
            client.service_liveness(Duration::from_millis(50)).await?;
            let damage = client
                .take_events()
                .into_iter()
                .find_map(|event| match event {
                    SessionEvent::ActorVitals(delta)
                        if delta.value.health > 0 && delta.value.health < before =>
                    {
                        Some(delta.value)
                    }
                    _ => None,
                });
            if let Some(vitals) = damage {
                row.pass("incoming-damage", vitals);
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err("no native incoming damage frame before death deadline".into());
            }
        }
        let refused = client.session_mut().logout().await?;
        if !matches!(refused.result, Some(LogoutResult::InFight { .. })) {
            return Err(format!("in-fight logout did not refuse: {refused:?}").into());
        }
        row.pass("in-fight-logout-refused", refused);

        // The existing helper observes further damage, full-health spatial respawn and
        // a new committed server death receipt. It never issues a debug kill.
        client_death::death(&mut client, clients).await?;
        row.pass(
            "lethal-damage-receipt",
            client.world_spatial().actor_position,
        );
        let vitals = *client.actor_vitals().ok_or("respawn vitals absent")?;
        if client.world_spatial().actor_position != temple
            || vitals.health == 0
            || vitals.health != vitals.max_health
        {
            return Err(format!(
                "death did not restore exact home temple and full HP: {:?} {vitals:?}",
                client.world_spatial()
            )
            .into());
        }
        row.pass("exact-temple-full-health", (temple, vitals));
        relog(&mut client, clients, 0xe4).await?;
        let saved = client.actor_vitals().ok_or("respawn relog vitals absent")?;
        if client.world_spatial().actor_position != temple
            || saved.health != vitals.max_health
            || saved.max_health != vitals.max_health
        {
            return Err("fresh admission did not retain the saved temple and full HP".into());
        }
        row.pass(
            "relog-saved-temple-full-health",
            (client.world_spatial(), saved),
        );
        Ok(())
    }
    .await;
    // Preserve the entry position required by the pre-existing phases, even on failure.
    let restore: TestResult = async {
        let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
            at == TOWN
        })
        .ok_or("E2EF6 return route unavailable")?;
        walk(&mut client, world, &path).await
    }
    .await;
    let released = logout(&mut client).await;
    result.and(restore).and(released)
}

/// GeneralStats has a fixed native layout. Do not substitute a ranking snapshot:
/// its scheduled cut can predate the kill or the vocation confirmation.
fn progress_frame(payload: &[u8]) -> TestResult<(u64, u32)> {
    if payload.len() != 42 || payload[0] != 1 || payload[33] > 1 {
        return Err("malformed native GeneralStats frame".into());
    }
    let xp = u64::from_le_bytes(payload[1..9].try_into()?);
    let level = u32::from_le_bytes(payload[9..13].try_into()?);
    if level == 0 {
        return Err("native GeneralStats returned level zero".into());
    }
    Ok((xp, level))
}

async fn progress(client: &mut DevClientSession) -> TestResult<(u64, u32)> {
    use oteryn_protocol_oteryn::cyclopedia_character as wire;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            wire::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &[1, 0, 0, 1],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("current progression query refused: {reply:?}").into());
    }
    progress_frame(&reply.payload)
}

async fn self_look(client: &mut DevClientSession) -> TestResult<String> {
    use oteryn_protocol_oteryn::item_look as wire;
    let entities = client
        .world_entities()
        .ok_or("native own actor view absent")?;
    let own = entities
        .iter()
        .find(|entry| &entry.entity.identity == entities.own_identity())
        .ok_or("native own actor identity absent")?
        .entity;
    let selection = wire::ItemLookSelection { extended: true };
    let payload = wire::encode_item_look_intent_for(
        wire::ItemLookIntent {
            target: wire::ItemLookTarget::Creature(own),
        },
        selection,
    )
    .map_err(|error| format!("self look intent: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            wire::ITEM_LOOK_CAPABILITY_ID.ok_or("native look capability unallocated")?,
            wire::ITEM_LOOK_COMMAND_ID.ok_or("native look command unallocated")?,
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("native self look refused: {reply:?}").into());
    }
    match wire::decode_item_look_result_for(&reply.payload, selection)
        .map_err(|error| format!("self look result: {error:?}"))?
    {
        wire::ItemLookResult::Text(text) => Ok(text),
        result => Err(format!("native self look unavailable: {result:?}").into()),
    }
}

async fn assert_build(
    client: &mut DevClientSession,
    expected: (u64, u32),
    vocation: &str,
) -> TestResult {
    let observed = progress(client).await?;
    let look = self_look(client).await?;
    if observed != expected
        || look != format!("You see yourself (Level {}, {vocation}).", expected.1)
    {
        return Err(format!(
            "native build differs: expected={expected:?}/{vocation} observed={observed:?}/{look}"
        )
        .into());
    }
    Ok(())
}

async fn approach_oressa(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult<oteryn_dev_client::EntityRef> {
    let npc = npcs
        .npcs
        .iter()
        .find(|npc| npc.key == "oteryn:npc.oressa")
        .ok_or("Oressa prerequisite blocked: compiled world has no oteryn:npc.oressa placement")?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        npc.placements.iter().any(|placement| placement.floor == at.floor
            && at.x.abs_diff(placement.x).max(at.y.abs_diff(placement.y)) <= 3)
    }).ok_or("Oressa prerequisite blocked: no player walking route to oteryn:npc.oressa within the existing town route budget")?;
    walk(client, world, &path).await?;
    client.world_entities().and_then(|entities| entities.iter().find(|entry| {
        entry.kind == oteryn_dev_client::EntityKind::Npc && npc.placements.iter().any(|placement| {
            entry.position.x == i32::from(placement.x)
                && entry.position.y == i32::from(placement.y)
                && entry.position.floor == -i16::from(placement.floor)
        })
    })).map(|entry| entry.entity)
        .ok_or_else(|| "Oressa prerequisite blocked: no native NPC actor at the compiled Oressa placement after walking there".into())
}

/// Consume a new conversation frame after this particular command result. A
/// greeting cached before the confirmation cannot prove a successful choice.
async fn oressa_talk(
    client: &mut DevClientSession,
    npc: oteryn_dev_client::EntityRef,
    text: &str,
    expected: oteryn_protocol_oteryn::npc_service::NpcIntentDisposition,
    expected_line: &str,
) -> TestResult {
    use oteryn_protocol_oteryn::npc_service as wire;
    let payload = wire::encode_npc_talk_intent(&wire::NpcTalkIntent {
        npc_actor: npc,
        text: text.into(),
    })
    .map_err(|error| format!("Oressa talk intent: {error:?}"))?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let reply = loop {
        let reply = client
            .session_mut()
            .gated_command(
                wire::CAPABILITY_NPC_SERVICE_V1,
                wire::COMMAND_TYPE_NPC_TALK_INTENT,
                &payload,
            )
            .await?;
        if reply.status != CommandStatus::Accepted {
            return Err(format!("Oressa {text} command rejected: {reply:?}").into());
        }
        let disposition = wire::decode_npc_intent_result(&reply.payload)
            .map_err(|error| format!("Oressa talk result: {error:?}"))?;
        if disposition == wire::NpcIntentDisposition::RateLimited
            && tokio::time::Instant::now() < deadline
        {
            client.service_liveness(Duration::from_millis(50)).await?;
            client.take_events();
            continue;
        }
        if disposition != expected {
            return Err(format!("Oressa {text}: expected {expected:?}, observed {disposition:?}; required reply={expected_line}").into());
        }
        break reply;
    };
    loop {
        let delta = client
            .session_mut()
            .read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1)
            .await?;
        // Walk-away farewells can have been queued before this command.
        if delta.server_sequence <= reply.server_sequence {
            continue;
        }
        let conversation = wire::decode_npc_conversation(&delta.payload)
            .map_err(|error| format!("Oressa conversation frame: {error:?}"))?;
        if conversation.open.map(|(actor, _)| actor) != Some(npc)
            || conversation.lines != [expected_line]
        {
            return Err(format!(
                "Oressa {text} returned a different conversation: {conversation:?}"
            )
            .into());
        }
        return Ok(());
    }
}

/// Only ATTACK against an actual adjacent creature can earn the prerequisite.
/// No fixture, clock override, stored build write or debug level command is used.
async fn earn_level_eight(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult<(u64, u32)> {
    use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(240);
    let mut before = progress(client).await?;
    while before.1 < 8 {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("level-8 prerequisite blocked: real rat hunting budget exhausted at XP={} level={}; no player progression override allowed", before.0, before.1).into());
        }
        travel_to_spawn(client, world, spawns, "oteryn:creature.rat")
            .await
            .map_err(|error| {
                format!(
                    "level-8 prerequisite blocked at XP={} level={}: rat player route: {error}",
                    before.0, before.1
                )
            })?;
        let here = client.world_spatial().actor_position;
        let target = client
            .world_entities()
            .and_then(|entities| {
                entities.iter().find(|entry| {
                    entry.kind == oteryn_dev_client::EntityKind::Creature
                        && entry.position.floor == here.floor
                        && entry
                            .position
                            .x
                            .abs_diff(here.x)
                            .max(entry.position.y.abs_diff(here.y))
                            <= 1
                })
            })
            .copied()
            .ok_or("level-8 prerequisite blocked: no adjacent creature after real rat route")?;
        client.take_events();
        let attack = client
            .session_mut()
            .attack_target(Some(&target.entity))
            .await?;
        if attack.disposition != AttackIntentDisposition::Ok {
            return Err(format!("level-8 prerequisite blocked at XP={} level={}: native ATTACK refused {attack:?}; ordinary inventory={:?}", before.0, before.1, client.inventory()).into());
        }
        let kill_deadline = (tokio::time::Instant::now() + Duration::from_secs(60)).min(deadline);
        let mut damaged = false;
        loop {
            client.service_liveness(Duration::from_millis(50)).await?;
            for event in client.take_events() {
                if let SessionEvent::WorldSpatialEntities(delta) = event {
                    damaged |= delta.value.update.iter().any(|entry| entry.entity == target.entity && matches!(entry.detail,
                        oteryn_dev_client::EntityDetail::Actor { health_percent, .. } if health_percent < 100));
                }
            }
            let removed = client
                .world_entities()
                .is_some_and(|entities| entities.get(&target.entity).is_none());
            if removed {
                let settled = tokio::time::Instant::now() + Duration::from_secs(5);
                let after = loop {
                    let after = progress(client).await?;
                    if after.0 > before.0 || tokio::time::Instant::now() >= settled {
                        break after;
                    }
                    client.service_liveness(Duration::from_millis(50)).await?;
                    client.take_events();
                };
                // A single lethal hit can remove the target without an intermediate
                // health-percent frame. Credit requires the server's actual XP increase.
                if after.0 <= before.0 || after.1 < before.1 {
                    return Err(format!("level-8 prerequisite blocked: attacked target removed without earned XP; before={before:?} after={after:?} damaged={damaged}").into());
                }
                evidence(&format!(
                    "E2EF6_OBSERVATION S16a player-kill XP-level {before:?}->{after:?} intermediate-damage={damaged}"
                ));
                before = after;
                break;
            }
            if tokio::time::Instant::now() >= kill_deadline {
                client.session_mut().attack_target(None).await?;
                return Err(format!("level-8 prerequisite blocked: native ATTACK produced no credited kill within budget at XP={} level={} damaged={damaged}; ordinary inventory={:?}", before.0, before.1, client.inventory()).into());
            }
        }
    }
    client.session_mut().attack_target(None).await?;
    if before.1 != 8 {
        return Err(format!(
            "level-8 prerequisite blocked: observed level {} instead of exactly 8",
            before.1
        )
        .into());
    }
    Ok(before)
}

async fn first_vocation(
    row: &mut Row,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult {
    use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::{Ok as Accepted, Rejected};
    let mut client = connect_world(clients, 0xe5).await?;
    let result: TestResult = async {
        let initial = progress(&mut client).await?;
        assert_build(&mut client, initial, "no vocation").await?;
        if initial.1 >= 8 {
            return Err(format!("underlevel refusal prerequisite blocked: ordinary character already level {}; no level reset permitted", initial.1).into());
        }
        let npc = approach_oressa(&mut client, world, npcs).await?;
        oressa_talk(&mut client, npc, "hi", Accepted, "Welcome. Ask about a vocation when you are ready to choose.").await?;
        row.pass("oressa-reachable", npc);
        oressa_talk(&mut client, npc, "yes", Rejected, "Choose a vocation before confirming.").await?;
        assert_build(&mut client, initial, "no vocation").await?;
        row.pass("unquoted-confirmation-refused", initial);
        oressa_talk(&mut client, npc, "knight", Accepted, "Do you wish to become a knight? Say yes to confirm, or no to reconsider.").await?;
        oressa_talk(&mut client, npc, "yes", Rejected, "Reach level 8 before making your first vocation choice.").await?;
        assert_build(&mut client, initial, "no vocation").await?;
        row.pass("underlevel-choice-refused", initial);

        // A refused confirmed choice remains in the per-session conversation.
        // Fresh admission clears it and also verifies the refusal was durable.
        relog(&mut client, clients, 0xe6).await?;
        assert_build(&mut client, initial, "no vocation").await?;

        let earned = earn_level_eight(&mut client, world, spawns).await?;
        assert_build(&mut client, earned, "no vocation").await?;
        row.pass("earned-level-eight", earned);
        let npc = approach_oressa(&mut client, world, npcs).await?;
        oressa_talk(&mut client, npc, "hi", Accepted, "Welcome. Ask about a vocation when you are ready to choose.").await?;
        oressa_talk(&mut client, npc, "knight", Accepted, "Do you wish to become a knight? Say yes to confirm, or no to reconsider.").await?;
        assert_build(&mut client, earned, "no vocation").await?;
        oressa_talk(&mut client, npc, "no", Accepted, "Take your time before choosing your vocation.").await?;
        oressa_talk(&mut client, npc, "yes", Rejected, "Choose a vocation before confirming.").await?;
        assert_build(&mut client, earned, "no vocation").await?;
        row.pass("declined-choice-unchanged", earned);
        oressa_talk(&mut client, npc, "knight", Accepted, "Do you wish to become a knight? Say yes to confirm, or no to reconsider.").await?;
        oressa_talk(&mut client, npc, "yes", Accepted, "Your vocation is now knight.").await?;
        assert_build(&mut client, earned, "knight").await?;
        row.pass("confirmed-first-choice", earned);
        oressa_talk(&mut client, npc, "hi", Accepted, "Welcome. Ask about a vocation when you are ready to choose.").await?;
        oressa_talk(&mut client, npc, "druid", Accepted, "Do you wish to become a druid? Say yes to confirm, or no to reconsider.").await?;
        oressa_talk(&mut client, npc, "yes", Rejected, "Reach level 8 before making your first vocation choice.").await?;
        assert_build(&mut client, earned, "knight").await?;
        row.pass("second-choice-refused", earned);
        relog(&mut client, clients, 0xe7).await?;
        assert_build(&mut client, earned, "knight").await?;
        row.pass("relog-vocation-xp-level", earned);
        Ok(())
    }.await;
    let restore: TestResult = async {
        client.session_mut().attack_target(None).await?;
        let back = route(world, tile(client.world_spatial().actor_position)?, |at| {
            at == TOWN
        })
        .ok_or("Oressa journey return route unavailable")?;
        walk(&mut client, world, &back).await
    }
    .await;
    let released = logout(&mut client).await;
    result.and(restore).and(released)
}

const EXPERIENCE_LOSS: &[&str] = &[
    "earned-nonzero-xp",
    "unblessed-death-xp-level",
    "unblessed-relog",
    "blessing-purchased",
    "duplicate-blessing-refused",
    "blessed-death-xp-level",
    "blessed-relog-consumed",
    "promotion-purchased",
    "promoted-death-xp-level",
    "promoted-relog",
    "receipt-loss-fields",
];
const ITEM_LOSS: &[&str] = &[
    "earned-loss-eligible-level",
    "unprotected-bag-and-contents",
    "unprotected-death-loss",
    "ground-original-bag-contents",
    "unprotected-relog-custody",
    "amulet-bought-equipped",
    "amulet-invalid-slot-refused",
    "protected-death-consumes-amulet",
    "protected-ground-unchanged",
    "protected-relog-custody",
];

async fn tab(client: &mut DevClientSession, id: u8) -> TestResult<Vec<u8>> {
    use oteryn_protocol_oteryn::cyclopedia_character as wire;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            wire::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &[id, 0, 0, 50],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("native character tab {id} refused: {reply:?}").into());
    }
    Ok(reply.payload)
}

fn read_text(payload: &[u8], cursor: &mut usize) -> TestResult<String> {
    let len = usize::from(*payload.get(*cursor).ok_or("missing text length")?);
    *cursor += 1;
    let text = std::str::from_utf8(
        payload
            .get(*cursor..*cursor + len)
            .ok_or("truncated text")?,
    )?
    .to_owned();
    *cursor += len;
    Ok(text)
}

fn blessing_frame(payload: &[u8]) -> TestResult<Vec<String>> {
    if payload.first() != Some(&6) {
        return Err("expected StoreSummary frame".into());
    }
    let count = usize::from(*payload.get(1).ok_or("missing blessing count")?);
    let mut cursor = 2;
    let mut keys = Vec::new();
    for _ in 0..count {
        keys.push(read_text(payload, &mut cursor)?);
    }
    if payload.len() != cursor + 3 || keys.iter().collect::<BTreeSet<_>>().len() != keys.len() {
        return Err("malformed or duplicate StoreSummary blessings".into());
    }
    keys.sort();
    Ok(keys)
}

async fn blessings(client: &mut DevClientSession) -> TestResult<Vec<String>> {
    blessing_frame(&tab(client, 6).await?)
}

fn death_frame(payload: &[u8]) -> TestResult<Vec<(i64, String)>> {
    if payload.first() != Some(&2) {
        return Err("expected RecentDeaths frame".into());
    }
    let count = usize::from(u16::from_le_bytes(
        payload.get(1..3).ok_or("missing death count")?.try_into()?,
    ));
    if count > 50 {
        return Err("RecentDeaths exceeds requested page size".into());
    }
    let mut cursor = 3;
    let mut entries = Vec::new();
    for _ in 0..count {
        let at = i64::from_le_bytes(
            payload
                .get(cursor..cursor + 8)
                .ok_or("truncated death time")?
                .try_into()?,
        );
        cursor += 8;
        entries.push((at, read_text(payload, &mut cursor)?));
    }
    if cursor != payload.len() {
        return Err("trailing RecentDeaths bytes".into());
    }
    Ok(entries)
}

// Handles are admission-local. Compare exact definitions/counts/subtypes and slots.
fn custody(inventory: &CharacterInventory) -> Vec<(u8, u32, u32, u32)> {
    let item = |slot, entry: &items::ItemEntry| {
        (
            slot,
            entry.item_definition_ref.get(),
            entry.count.get(),
            entry.sub_type,
        )
    };
    let mut result = Vec::new();
    if let Some(bag) = &inventory.main_backpack {
        result.push(item(0, bag));
    }
    result.extend(inventory.entries.iter().map(|entry| item(255, entry)));
    result.extend(
        inventory
            .equipment
            .iter()
            .map(|entry| item(entry.slot as u8, &entry.item)),
    );
    result.sort();
    result
}

async fn wait_inventory(
    client: &mut DevClientSession,
    matches: impl Fn(&CharacterInventory) -> bool,
    blocker: &str,
) -> TestResult<CharacterInventory> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(view) = client.inventory().filter(|view| matches(view)) {
            return Ok(view.clone());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("{blocker}; observed={:?}", client.inventory()).into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

async fn return_and_release(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    let restore: TestResult = async {
        client.session_mut().attack_target(None).await?;
        let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
            at == TOWN
        })
        .ok_or("death row cleanup: no walking route back to entry")?;
        walk(client, world, &path).await
    }
    .await;
    let released = logout(client).await;
    restore.and(released)
}

/// Real kills and corpse loot fund the prerequisites; no fixture or bank read is used.
async fn earn_death_prerequisites(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    level: u32,
    gold: u64,
) -> TestResult {
    let index = provisioning::item_index()?;
    let coins = [
        (provisioning::item_reference(&index, 3031)?, 1),
        (provisioning::item_reference(&index, 3035)?, 100),
        (provisioning::item_reference(&index, 3043)?, 10_000),
    ];
    let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    loop {
        let build = progress(client).await?;
        let money: u64 = client
            .inventory()
            .ok_or("prerequisite inventory missing")?
            .entries
            .iter()
            .map(|entry| {
                coins
                    .iter()
                    .find(|(key, _)| *key == entry.item_definition_ref)
                    .map_or(0, |(_, value)| u64::from(entry.count.get()) * value)
            })
            .sum();
        if build.1 >= level && money >= gold {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("player prerequisite blocked: rat hunting/loot budget exhausted; required level={level} carried_gold={gold}; observed XP={} level={} carried_gold={money}", build.0, build.1).into());
        }
        travel_to_spawn(client, world, spawns, "oteryn:creature.rat").await
            .map_err(|error| format!("player prerequisite blocked: required level={level} carried_gold={gold}, observed XP={} level={} carried_gold={money}; rat route: {error}", build.0, build.1))?;
        client_hunt::hunt(client).await.map_err(|error| format!("player prerequisite blocked: real rat kill/loot: {error}; required level={level} carried_gold={gold}; before XP={} level={} carried_gold={money}", build.0, build.1))?;
    }
}

async fn approach_service(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    key: &str,
) -> TestResult<oteryn_dev_client::EntityRef> {
    let npc = npcs.npcs.iter().find(|npc| npc.key == key).ok_or_else(|| {
        format!("player prerequisite blocked: compiled world has no {key} placement")
    })?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        npc.placements.iter().any(|place| {
            place.floor == at.floor && at.x.abs_diff(place.x).max(at.y.abs_diff(place.y)) <= 3
        })
    })
    .ok_or_else(|| {
        format!("player prerequisite blocked: no walking route to {key} within town route budget")
    })?;
    walk(client, world, &path).await?;
    client
        .world_entities()
        .and_then(|view| {
            view.iter().find(|entry| {
                entry.kind == oteryn_dev_client::EntityKind::Npc
                    && npc.placements.iter().any(|place| {
                        entry.position
                            == ActorPosition {
                                x: i32::from(place.x),
                                y: i32::from(place.y),
                                floor: -i16::from(place.floor),
                            }
                    })
            })
        })
        .map(|entry| entry.entity)
        .ok_or_else(|| {
            format!("player prerequisite blocked: no native {key} actor after walking to placement")
                .into()
        })
}

async fn service_talk(
    client: &mut DevClientSession,
    npc: oteryn_dev_client::EntityRef,
    text: &str,
) -> TestResult<Vec<String>> {
    use oteryn_protocol_oteryn::npc_service as wire;
    let payload = wire::encode_npc_talk_intent(&wire::NpcTalkIntent {
        npc_actor: npc,
        text: text.into(),
    })
    .map_err(|error| format!("service talk encode: {error:?}"))?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let reply = loop {
        let reply = client
            .session_mut()
            .gated_command(
                wire::CAPABILITY_NPC_SERVICE_V1,
                wire::COMMAND_TYPE_NPC_TALK_INTENT,
                &payload,
            )
            .await?;
        let disposition = wire::decode_npc_intent_result(&reply.payload)
            .map_err(|error| format!("service reply: {error:?}"))?;
        if disposition == wire::NpcIntentDisposition::RateLimited
            && tokio::time::Instant::now() < deadline
        {
            client.service_liveness(Duration::from_millis(50)).await?;
            continue;
        }
        if reply.status != CommandStatus::Accepted || disposition != wire::NpcIntentDisposition::Ok
        {
            return Err(format!(
                "player service prerequisite blocked: {text}: {reply:?} disposition={disposition:?}"
            )
            .into());
        }
        break reply;
    };
    loop {
        let delta = client
            .session_mut()
            .read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1)
            .await?;
        if delta.server_sequence <= reply.server_sequence {
            continue;
        }
        let conversation = wire::decode_npc_conversation(&delta.payload)
            .map_err(|error| format!("service conversation: {error:?}"))?;
        if conversation.open.map(|(actor, _)| actor) != Some(npc) {
            return Err("service response belongs to another NPC".into());
        }
        return Ok(conversation.lines);
    }
}

async fn purchase_service(
    client: &mut DevClientSession,
    npc: oteryn_dev_client::EntityRef,
    request: &str,
    quote: &str,
    success: &str,
) -> TestResult {
    service_talk(client, npc, "hi").await?;
    let lines = service_talk(client, npc, request).await?;
    if lines != [quote] {
        return Err(
            format!("player service prerequisite blocked: {request} quote={lines:?}").into(),
        );
    }
    let lines = service_talk(client, npc, "yes").await?;
    if lines != [success] {
        return Err(format!(
            "player service prerequisite blocked: {request} confirmed server reply={lines:?}"
        )
        .into());
    }
    Ok(())
}

/// No SQL receipt reads: new native history plus actual HP loss and temple deltas.
async fn observed_death(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult<(ActorPosition, (u64, u32), (u64, u32))> {
    if !client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1)
    {
        return Err("death journey requires native production map and lethal encounter".into());
    }
    let history = death_frame(&tab(client, 2).await?)?;
    let before = progress(client).await?;
    let home = world
        .base()
        .subareas
        .default_town
        .and_then(|id| world.base().subareas.town(id))
        .and_then(|town| town.temple)
        .ok_or("compiled default temple unavailable")?;
    let temple = ActorPosition {
        x: i32::from(home.x),
        y: i32::from(home.y),
        floor: -i16::from(home.floor),
    };
    travel_to_spawn(client, world, spawns, "oteryn:creature.poison_spider").await?;
    let encounter = client.world_spatial().actor_position;
    if encounter == temple {
        return Err("lethal encounter coincides with temple".into());
    }
    client.session_mut().attack_target(None).await?;
    client.take_events();
    let hp = client
        .actor_vitals()
        .ok_or("death entry vitals missing")?
        .health;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    let mut damaged = false;
    loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        for event in client.take_events() {
            if let SessionEvent::ActorVitals(delta) = event {
                damaged |= delta.value.health < hp;
            }
        }
        if damaged
            && client.world_spatial().actor_position == temple
            && client
                .actor_vitals()
                .is_some_and(|v| v.health > 0 && v.health == v.max_health)
        {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("native death deadline: incoming_damage={damaged} encounter={encounter:?} current={:?}", client.world_spatial().actor_position).into());
        }
    }
    let latest = death_frame(&tab(client, 2).await?)?;
    if latest.first() == history.first()
        || latest
            .first()
            .is_none_or(|(_, cause)| cause != &format!("Died at Level {}", before.1))
    {
        return Err(format!("respawn lacks a new matching native death-history receipt: before={history:?} after={latest:?}").into());
    }
    let after = progress(client).await?;
    Ok((encounter, before, after))
}

/// Independent oracle from the accepted finite experience table, including delevelling.
fn expected_death_progress(
    before: (u64, u32),
    blessings: u8,
    promoted: bool,
) -> TestResult<(u64, u32)> {
    let section: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../rulesets/character/experience/native-section.json"
    ))?;
    let table = section["experience_table"]["levels"]
        .as_array()
        .ok_or("canonical XP levels absent")?;
    let thresholds: Vec<(u32, u64)> = table
        .iter()
        .map(|entry| -> TestResult<_> {
            Ok((
                u32::try_from(entry["level"].as_u64().ok_or("canonical level absent")?)?,
                entry["minimum_experience"]
                    .as_str()
                    .ok_or("canonical threshold absent")?
                    .parse()?,
            ))
        })
        .collect::<TestResult<_>>()?;
    if blessings > 7
        || thresholds
            .iter()
            .rev()
            .find(|(_, xp)| *xp <= before.0)
            .map(|(level, _)| *level)
            != Some(before.1)
    {
        return Err("invalid death baseline XP/level/blessings".into());
    }
    let current = thresholds
        .iter()
        .position(|(level, _)| *level == before.1)
        .ok_or("death level outside canonical table")?;
    let next = thresholds
        .get(current + 1)
        .map(|(_, xp)| *xp)
        .ok_or("death test requires a next canonical level")?;
    let percent = 100_u128 - 8 * u128::from(blessings) - if promoted { 30 } else { 0 };
    let lost = (u128::from(next - thresholds[current].1) * percent / 100).min(u128::from(before.0));
    let xp = before.0 - u64::try_from(lost)?;
    let level = thresholds
        .iter()
        .rev()
        .find(|(_, minimum)| *minimum <= xp)
        .map(|(level, _)| *level)
        .ok_or("post-death XP outside canonical table")?;
    Ok((xp, level))
}

async fn experience_loss(
    row: &mut Row,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult<Option<&'static str>> {
    let mut client = connect_world(clients, 0xc0).await?;
    let result = async {
        if !blessings(&mut client).await?.is_empty() { return Err("unblessed baseline prerequisite blocked: ordinary Character already holds blessings".into()); }
        let look = self_look(&mut client).await?;
        if !look.contains(", knight)") { return Err(format!("promotion prerequisite blocked: first-vocation journey has not produced a knight; self LOOK={look}").into()); }
        // Promotion needs level 20; earn it and the later service fee through real combat.
        earn_death_prerequisites(&mut client, world, spawns, 20, 0).await?;
        row.pass("earned-nonzero-xp", progress(&mut client).await?);
        for (blessed, promoted, check, saved, tag) in [
            (false, false, "unblessed-death-xp-level", "unblessed-relog", 0xc1),
            (true, false, "blessed-death-xp-level", "blessed-relog-consumed", 0xc2),
            (false, true, "promoted-death-xp-level", "promoted-relog", 0xc3),
        ] {
            if blessed {
                earn_death_prerequisites(&mut client, world, spawns, 20, 20_000).await?;
                let npc = approach_service(&mut client, world, npcs, "oteryn:npc.eremo").await?;
                let quote = "Do you want to buy The Wisdom of Solitude at the price for your current level?";
                purchase_service(&mut client, npc, "The Wisdom of Solitude", quote, "You have received the blessing.").await?;
                let held = blessings(&mut client).await?;
                if held != ["oteryn:blessing.the_wisdom_of_solitude"] { return Err(format!("blessing purchase did not change native ownership: {held:?}").into()); }
                row.pass("blessing-purchased", &held);
                service_talk(&mut client, npc, "The Wisdom of Solitude").await?;
                let duplicate = service_talk(&mut client, npc, "yes").await?;
                if duplicate != ["You already have this blessing."] || blessings(&mut client).await? != held {
                    return Err(format!("duplicate blessing purchase did not refuse without ownership change: {duplicate:?}").into());
                }
                row.pass("duplicate-blessing-refused", duplicate);
            }
            if promoted {
                earn_death_prerequisites(&mut client, world, spawns, 20, 20_000).await?;
                let npc = approach_service(&mut client, world, npcs, "oteryn:npc.king_tibianus").await?;
                purchase_service(&mut client, npc, "promotion", "Do you want to be promoted for 20000 gold coins?", "You are now promoted.").await?;
                row.pass("promotion-purchased", self_look(&mut client).await?);
            }
            let (_, before, after) = observed_death(&mut client, world, spawns).await?;
            let expected = expected_death_progress(before, u8::from(blessed), promoted)?;
            if after != expected {
                return Err(format!("death reduction differs: blessed={blessed} promoted={promoted} before={before:?} expected={expected:?} after={after:?}").into());
            }
            row.pass(check, (before, after, before.0 - after.0));
            relog(&mut client, clients, tag).await?;
            let persisted = progress(&mut client).await?;
            let held = blessings(&mut client).await?;
            if persisted != after || !held.is_empty() { return Err(format!("death XP/level or consumed blessings differ after fresh admission: {persisted:?} {held:?}").into()); }
            row.pass(saved, (persisted, held));
        }
        // The only native death-history route exposes (time, cause), not loss receipt
        // fields. Never pretend that an XP delta proves the receipt's stored XP loss.
        Ok(Some("crates/gameplay/durability/src/durability/cyclopedia_character.rs:118"))
    }.await;
    let cleanup = return_and_release(&mut client, world).await;
    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), _) | (_, Err(error)) => Err(error),
    }
}

fn finish_missing_receipt(row: Row, source: &str) {
    for action in row.required {
        if !row.passed.contains(action) {
            evidence(&format!(
                "E2EF6_CHECK {} {action} FAIL assertion-not-completed",
                row.id
            ));
        }
    }
    evidence(&format!(
        "E2EF6_GAP {} native RecentDeaths exposes only timestamp/cause; no player route to loss receipt XP/blessing/promotion fields source:{source}",
        row.id
    ));
    evidence(&format!(
        "e2e_all scenario={}.full status=MISSING-SERVER seconds={:.3} evidence=source:{source}",
        row.id,
        row.started.elapsed().as_secs_f64()
    ));
}

async fn ground_entries(
    client: &mut DevClientSession,
    position: ActorPosition,
) -> TestResult<Vec<items::ItemEntry>> {
    use oteryn_protocol_oteryn::{browse_field as wire, container_tree as tree};
    let payload = wire::encode_browse_field_intent(&wire::BrowseFieldIntent::Open {
        position: items::WorldTilePosition {
            x: position.x,
            y: position.y,
            floor: position.floor,
        },
    })
    .map_err(|error| format!("death ground browse encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_BROWSE_FIELD_V1,
            wire::COMMAND_TYPE_BROWSE_FIELD_INTENT,
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted
        || tree::decode_container_view_result(&reply.payload)
            .map_err(|error| format!("ground browse result: {error:?}"))?
            != tree::ContainerViewOutcome::Opened
    {
        return Err(format!("death ground browse refused: {reply:?}").into());
    }
    loop {
        let delta = client
            .session_mut()
            .read_gated_delta(
                tree::STATE_DOMAIN_CONTAINER_VIEWS,
                tree::DELTA_TYPE_CONTAINER_VIEWS_V1,
            )
            .await?;
        if delta.server_sequence <= reply.server_sequence {
            continue;
        }
        let views = tree::decode_container_views_for(&delta.payload, true)
            .map_err(|error| format!("death ground view: {error:?}"))?;
        let view = views
            .views
            .iter()
            .find(|view| {
                view.browse.is_some_and(|page| {
                    page.position.x == position.x
                        && page.position.y == position.y
                        && page.position.floor == position.floor
                })
            })
            .ok_or("matching ground browse delta missing")?;
        if view
            .browse
            .is_none_or(|page| usize::from(page.total) > view.entries.len())
        {
            return Err(
                "death ground assertions require all entries; ground page exceeds capacity".into(),
            );
        }
        return Ok(view.entries.clone());
    }
}

async fn buy_amulet(
    client: &mut DevClientSession,
    npc: oteryn_dev_client::EntityRef,
    definition: NonZeroU32,
) -> TestResult {
    use oteryn_dev_client::client_runtime::npc_shop;
    use oteryn_protocol_oteryn::npc_service as wire;
    service_talk(client, npc, "hi").await?;
    // trade opens its own domain; do not wait for a conversation on this command.
    let disposition = npc_shop::talk(
        client.session_mut(),
        &wire::NpcTalkIntent {
            npc_actor: npc,
            text: "trade".into(),
        },
    )
    .await?;
    if disposition != wire::NpcIntentDisposition::Ok {
        return Err(
            format!("Amulet of Loss prerequisite blocked: trade refused {disposition:?}").into(),
        );
    }
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let window = wire::decode_npc_trade_window(&delta.payload)
        .map_err(|error| format!("amulet shop window: {error:?}"))?;
    let (_, revision) = window
        .open
        .filter(|(actor, _)| *actor == npc)
        .ok_or("Amulet of Loss shop did not open for approached NPC")?;
    let (offer_index, offer) = window.offers.iter().enumerate().find(|(_, offer)| offer.side == wire::NpcTradeSide::Buy && offer.item_definition_ref == definition.get())
        .ok_or("Amulet of Loss prerequisite blocked: Eremo's real trade window has no oteryn:item.tibia.i3057 buy offer")?;
    let disposition = npc_shop::buy(
        client.session_mut(),
        &wire::NpcTradeIntent {
            npc_actor: npc,
            catalogue_revision: revision,
            offer_index: u32::try_from(offer_index)?,
            side: offer.side,
            quantity: 1,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    if disposition != wire::NpcIntentDisposition::Ok {
        return Err(format!(
            "Amulet of Loss prerequisite blocked: actual purchase price={} server={disposition:?}",
            offer.unit_price
        )
        .into());
    }
    Ok(())
}

async fn item_loss(
    row: &mut Row,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult {
    let mut client = connect_world(clients, 0xc4).await?;
    let result: TestResult = async {
        if !blessings(&mut client).await?.is_empty() { return Err("unprotected item-loss prerequisite blocked: ordinary Character has held blessings".into()); }
        earn_death_prerequisites(&mut client, world, spawns, 9, 1).await?;
        row.pass("earned-loss-eligible-level", progress(&mut client).await?);
        let index = provisioning::item_index()?;
        // Exercise AoL first so the unprotected death can be terminal: no invented
        // replacement-backpack grant is required to fund the protected case.
        earn_death_prerequisites(&mut client, world, spawns, 9, 50_000).await?;
        let npc = approach_service(&mut client, world, npcs, "oteryn:npc.eremo").await?;
        let amulet = provisioning::item_reference(&index, 3057)?;
        buy_amulet(&mut client, npc, amulet).await?;
        let purchased = wait_inventory(&mut client, |view| view.entries.iter().any(|item| item.item_definition_ref == amulet), "purchased Amulet of Loss inventory delta absent").await?;
        let owned = purchased.entries.iter().find(|item| item.item_definition_ref == amulet).ok_or("purchased amulet disappeared")?;
        let refused = client.move_item(&items::ItemMoveIntent { source: owned.handle, destination: items::ItemMoveDestination::Equipment(items::EquipmentSlot::RightHand) }).await?;
        if refused.outcome != items::ItemMoveOutcome::SlotMismatch { return Err(format!("amulet invalid slot did not refuse: {refused:?}").into()); }
        let unchanged = custody(&purchased);
        wait_inventory(&mut client, |view| custody(view) == unchanged, "invalid amulet equip changed custody").await?;
        row.pass("amulet-invalid-slot-refused", refused);
        let equipped = client.move_item(&items::ItemMoveIntent { source: owned.handle, destination: items::ItemMoveDestination::Equipment(items::EquipmentSlot::Necklace) }).await?;
        if equipped.outcome != items::ItemMoveOutcome::Moved { return Err(format!("amulet equip refused: {equipped:?}").into()); }
        wait_inventory(&mut client, |view| view.equipment.iter().any(|entry| entry.slot == items::EquipmentSlot::Necklace && entry.item.item_definition_ref == amulet), "amulet necklace native delta absent").await?;
        row.pass("amulet-bought-equipped", client.inventory());
        travel_to_spawn(&mut client, world, spawns, "oteryn:creature.poison_spider").await?;
        let protected_position = client.world_spatial().actor_position;
        let ground_before = ground_entries(&mut client, protected_position).await?;
        let ground_signature = custody(&CharacterInventory { entries: ground_before, ..Default::default() });
        let mut expected = client.inventory().ok_or("protected inventory absent")?.clone();
        expected.equipment.retain(|entry| entry.slot != items::EquipmentSlot::Necklace);
        let expected = custody(&expected);
        let (actual_encounter, _, _) = observed_death(&mut client, world, spawns).await?;
        if actual_encounter != protected_position { return Err("protected death encounter moved after ground baseline".into()); }
        wait_inventory(&mut client, |view| custody(view) == expected, "Amulet of Loss death did not consume only amulet and preserve backpack/contents/equipment").await?;
        row.pass("protected-death-consumes-amulet", &expected);
        relog(&mut client, clients, 0xc6).await?;
        if custody(client.inventory().ok_or("protected relog inventory absent")?) != expected { return Err("protected death custody/amulet consumption differs after fresh admission".into()); }
        row.pass("protected-relog-custody", &expected);
        let destination = tile(protected_position)?;
        let path = route(world, tile(client.world_spatial().actor_position)?, |at| at == destination).ok_or("protected death ground return route unavailable")?;
        walk(&mut client, world, &path).await?;
        let after_ground = custody(&CharacterInventory { entries: ground_entries(&mut client, protected_position).await?, ..Default::default() });
        if after_ground != ground_signature { return Err(format!("protected death changed ground custody: before={ground_signature:?} after={after_ground:?}").into()); }
        row.pass("protected-ground-unchanged", after_ground);
        earn_death_prerequisites(&mut client, world, spawns, 9, 1).await?;
        let inventory = client.inventory().ok_or("unprotected inventory missing")?.clone();
        let original = inventory.main_backpack.ok_or("unprotected death requires a real main backpack")?;
        if inventory.entries.is_empty() { return Err("unprotected death requires actual earned loot in backpack".into()); }
        row.pass("unprotected-bag-and-contents", &inventory);
        let (encounter, _, _) = observed_death(&mut client, world, spawns).await?;
        let after = wait_inventory(&mut client, |view| view.main_backpack.is_none_or(|bag| bag.handle != original.handle) && view.entries.is_empty(),
            "unprotected death did not remove the original backpack and its contents").await?;
        row.pass("unprotected-death-loss", &after);
        let saved = custody(&after);
        relog(&mut client, clients, 0xc5).await?;
        if custody(client.inventory().ok_or("unprotected relog inventory absent")?) != saved { return Err("unprotected death custody differs after fresh admission".into()); }
        row.pass("unprotected-relog-custody", &saved);
        let destination = tile(encounter)?;
        let path = route(world, tile(client.world_spatial().actor_position)?, |at| at == destination)
            .ok_or("no real return route to death ground")?;
        walk(&mut client, world, &path).await?;
        let ground = ground_entries(&mut client, encounter).await?;
        let bag = ground.iter().find(|entry| entry.item_definition_ref == original.item_definition_ref && entry.count == original.count && entry.sub_type == original.sub_type)
            .ok_or("lost original backpack not observed in native ground view")?;
        use oteryn_protocol_oteryn::container_tree as tree;
        let payload = tree::encode_container_view_intent(&tree::ContainerViewIntent::Open { handle: bag.handle, replace_view: None }).map_err(|error| format!("lost bag open: {error:?}"))?;
        let reply = client.session_mut().gated_command(tree::CAPABILITY_CONTAINER_TREE_V1, tree::COMMAND_TYPE_CONTAINER_VIEW_INTENT, &payload).await?;
        if reply.status != CommandStatus::Accepted || tree::decode_container_view_result(&reply.payload).map_err(|error| format!("lost bag result: {error:?}"))? != tree::ContainerViewOutcome::Opened { return Err("lost bag native open refused".into()); }
        loop {
            let delta = client.session_mut().read_gated_delta(tree::STATE_DOMAIN_CONTAINER_VIEWS, tree::DELTA_TYPE_CONTAINER_VIEWS_V1).await?;
            if delta.server_sequence <= reply.server_sequence { continue; }
            let views = tree::decode_container_views_for(&delta.payload, true).map_err(|error| format!("lost bag view: {error:?}"))?;
            let view = views.views.iter().find(|view| view.container_handle == bag.handle).ok_or("lost bag container delta absent")?;
            let observed = CharacterInventory { entries: view.entries.clone(), ..Default::default() };
            let expected = CharacterInventory { entries: inventory.entries.clone(), ..Default::default() };
            if custody(&observed) != custody(&expected) { return Err("ground backpack did not retain exact earned contents".into()); }
            row.pass("ground-original-bag-contents", &view.entries);
            break;
        }
        Ok(())
    }.await;
    let cleanup = return_and_release(&mut client, world).await;
    result.and(cleanup)
}

// Consumption is a death outcome, not an NPC command acknowledgement.
const BLESSING_DEATH: &[&str] = &[
    "adventurer-earned-prerequisites",
    "adventurer-pvp-keeps-xp-blessings-custody",
    "adventurer-relog",
    "regular-and-twist-purchased",
    "duplicate-purchase-refused",
    "pve-consumes-regular-keeps-twist",
    "pve-relog",
    "twist-pvp-consumes-only-twist",
    "twist-relog",
];
const BLESSING_SERVICES: &[&str] = &[
    "adventurer-earned-prerequisites",
    "adventurer-pvp-keeps-xp-blessings-custody",
    "adventurer-relog",
    "decline-and-unquoted-confirmation-unchanged",
    "seven-blessings-purchased",
    "duplicate-purchases-refused",
    "purchases-relog",
    "henricus-complete-set",
    "henricus-duplicate-refused",
    "henricus-partial-refused",
    "henricus-relog",
];
const WISDOM: (&str, &str, &str) = (
    "oteryn:npc.eremo",
    "The Wisdom of Solitude",
    "oteryn:blessing.the_wisdom_of_solitude",
);
const TWIST: (&str, &str, &str) = (
    "oteryn:npc.temple_npc",
    "Twist of Fate",
    "oteryn:blessing.twist_of_fate",
);
const PILGRIMAGE: &[(&str, &str, &str)] = &[
    (
        "oteryn:npc.norf",
        "The Spiritual Shielding",
        "oteryn:blessing.the_spiritual_shielding",
    ),
    (
        "oteryn:npc.humphrey",
        "The Embrace of Tibia",
        "oteryn:blessing.the_embrace_of_tibia",
    ),
    (
        "oteryn:npc.edala",
        "The Fire of the Suns",
        "oteryn:blessing.the_fire_of_the_suns",
    ),
    (
        "oteryn:npc.kawill",
        "The Spark of the Phoenix",
        "oteryn:blessing.the_spark_of_the_phoenix",
    ),
    WISDOM,
    (
        "oteryn:npc.kais",
        "Blood of the Mountain",
        "oteryn:blessing.blood_of_the_mountain",
    ),
    (
        "oteryn:npc.nomad",
        "Heart of the Mountain",
        "oteryn:blessing.heart_of_the_mountain",
    ),
];

fn blessing_price(level: u32, key: &str) -> u64 {
    let level = u64::from(level);
    if key == TWIST.2 {
        2_000 + 200 * level.saturating_sub(30).min(240)
    } else if matches!(
        key,
        "oteryn:blessing.blood_of_the_mountain" | "oteryn:blessing.heart_of_the_mountain"
    ) {
        2_600 + 260 * level.saturating_sub(30).min(90) + 100 * level.saturating_sub(120)
    } else {
        2_000 + 200 * level.saturating_sub(30).min(90) + 75 * level.saturating_sub(120)
    }
}

fn held_with(held: &[String], keys: &[&str]) -> Vec<String> {
    held.iter()
        .map(String::as_str)
        .chain(keys.iter().copied())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(str::to_owned)
        .collect()
}

#[derive(Debug)]
struct MissingBlessingRoute {
    detail: String,
}
impl std::fmt::Display for MissingBlessingRoute {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.detail)
    }
}
impl std::error::Error for MissingBlessingRoute {}

fn finish_blessing_row(row: Row, result: TestResult) {
    if let Err(error) = &result
        && error.downcast_ref::<MissingBlessingRoute>().is_some()
    {
        for action in row.required {
            if !row.passed.contains(action) {
                evidence(&format!(
                    "E2EF6_CHECK {} {action} FAIL assertion-not-completed",
                    row.id
                ));
            }
        }
        let source = "crates/gameplay/content/src/content/npc_catalogue/premium_services.rs:95";
        evidence(&format!("E2EF6_GAP {} {error} source:{source}", row.id));
        evidence(&format!(
            "e2e_all scenario={}.full status=MISSING-SERVER seconds={:.3} evidence=source:{source}",
            row.id,
            row.started.elapsed().as_secs_f64()
        ));
    } else {
        row.finish(result);
    }
}

async fn buy_blessing(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    npcs: &oteryn_world_bundle::npc::Table,
    service: (&str, &str, &str),
) -> TestResult {
    let before = blessings(client).await?;
    if before.iter().any(|key| key == service.2) {
        return Err(format!(
            "blessing purchase prerequisite: {} already held: {before:?}",
            service.2
        )
        .into());
    }
    // Hunting can cross a price boundary. Reprice after each earned level, rather
    // than assuming a fee computed before real combat is still current.
    loop {
        let (_, level) = progress(client).await?;
        earn_death_prerequisites(
            client,
            world,
            spawns,
            level,
            blessing_price(level, service.2),
        )
        .await?;
        if progress(client).await?.1 == level {
            break;
        }
    }
    let npc = approach_service(client, world, npcs, service.0).await?;
    service_talk(client, npc, "hi").await?;
    let quote = format!(
        "Do you want to buy {} at the price for your current level?",
        service.1
    );
    let lines = service_talk(client, npc, service.1).await?;
    if lines != [quote] {
        // Runtime NPC response, plus the traced resolver, establish this gap.
        // Placement, funding and transport errors above remain ordinary FAILs.
        if ![WISDOM.2, PILGRIMAGE[5].2, PILGRIMAGE[6].2].contains(&service.2) {
            return Err(Box::new(MissingBlessingRoute {
                detail: format!(
                    "{} has no enabled player purchase service; observed quote reply={lines:?}",
                    service.2
                ),
            }));
        }
        return Err(format!("blessing quote unavailable for {}: {lines:?}", service.2).into());
    }
    let lines = service_talk(client, npc, "yes").await?;
    if lines != ["You have received the blessing."] {
        return Err(format!(
            "blessing player prerequisite/purchase blocked: {} server={lines:?}",
            service.2
        )
        .into());
    }
    let expected = held_with(&before, &[service.2]);
    let after = blessings(client).await?;
    if after != expected {
        return Err(format!(
            "purchase {} ownership differs: expected={expected:?} observed={after:?}",
            service.2
        )
        .into());
    }
    Ok(())
}

async fn duplicate_blessing(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    service: (&str, &str, &str),
) -> TestResult {
    let npc = approach_service(client, world, npcs, service.0).await?;
    let held = blessings(client).await?;
    let inventory = custody(
        client
            .inventory()
            .ok_or("duplicate purchase inventory absent")?,
    );
    service_talk(client, npc, "hi").await?;
    service_talk(client, npc, service.1).await?;
    let lines = service_talk(client, npc, "yes").await?;
    if lines != ["You already have this blessing."] || blessings(client).await? != held {
        return Err(format!(
            "duplicate {} was not refused unchanged: {lines:?}",
            service.2
        )
        .into());
    }
    wait_inventory(
        client,
        |view| custody(view) == inventory,
        "duplicate purchase changed custody",
    )
    .await?;
    Ok(())
}

async fn saved_blessings(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    index: usize,
    tag: u8,
    expected: &[String],
    build: (u64, u32),
) -> TestResult {
    let owned = custody(
        client
            .inventory()
            .ok_or("fresh blessing admission inventory absent")?,
    );
    let previous = client.join_snapshot().game_session_id;
    logout(client).await?;
    *client = connect_character(clients, tag, index).await?;
    let observed = blessings(client).await?;
    let saved = progress(client).await?;
    if client.join_snapshot().game_session_id == previous || observed != expected || saved != build
    {
        return Err(format!("fresh admission blessing/build mismatch: expected={expected:?}/{build:?} observed={observed:?}/{saved:?}").into());
    }
    wait_inventory(
        client,
        |view| custody(view) == owned,
        "blessing fresh admission custody differs",
    )
    .await?;
    Ok(())
}

// Find an ordinary walkable PvP site away from every authored creature spawn.
// Actual visibility and ATTACK results still decide whether it can be used.
async fn pvp_meet(
    victim: &mut DevClientSession,
    attacker: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult<oteryn_dev_client::EntityRef> {
    victim.session_mut().attack_target(None).await?;
    attacker.session_mut().attack_target(None).await?;
    let path = route(world, tile(victim.world_spatial().actor_position)?, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone())
            && spawns.sources.iter().all(|source| {
                source.floor != at.floor
                    || source
                        .points
                        .iter()
                        .all(|point| at.x.abs_diff(point.x).max(at.y.abs_diff(point.y)) > 10)
            })
    })
    .ok_or("PvP player route blocked: no reachable non-PZ tile clear of creature spawns")?;
    walk(victim, world, &path).await?;
    let target = tile(victim.world_spatial().actor_position)?;
    let path = route(
        world,
        tile(attacker.world_spatial().actor_position)?,
        |at| {
            at.floor == target.floor
                && at.x.abs_diff(target.x) + at.y.abs_diff(target.y) == 1
                && world
                    .base()
                    .tile(at.x, at.y, at.floor)
                    .is_some_and(|tile| !tile.protection_zone())
        },
    )
    .ok_or("PvP player route blocked: no adjacent non-PZ attacker tile")?;
    walk(attacker, world, &path).await?;
    let identity = *victim
        .world_entities()
        .ok_or("victim visibility absent")?
        .own_identity();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        attacker.service_liveness(Duration::from_millis(50)).await?;
        if let Some(entity) = attacker
            .world_entities()
            .and_then(|view| view.iter().find(|entry| entry.entity.identity == identity))
        {
            return Ok(entity.entity);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("PvP victim not observed in attacker visibility".into());
        }
    }
}

async fn observed_pvp_death(
    victim: &mut DevClientSession,
    attacker: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult<((u64, u32), (u64, u32))> {
    use oteryn_protocol_oteryn::{attack, pvp_expert as wire};
    let target = pvp_meet(victim, attacker, world, spawns).await?;
    let view = attacker
        .world_entities()
        .ok_or("PvP attacker visibility missing")?;
    let source = view
        .iter()
        .find(|entry| entry.entity.identity == *view.own_identity())
        .ok_or("PvP own attacker actor absent")?
        .entity;
    let before = progress(victim).await?;
    let history = death_frame(&tab(victim, 2).await?)?;
    let home = world
        .base()
        .subareas
        .default_town
        .and_then(|id| world.base().subareas.town(id))
        .and_then(|town| town.temple)
        .ok_or("PvP default temple missing")?;
    let temple = ActorPosition {
        x: i32::from(home.x),
        y: i32::from(home.y),
        floor: -i16::from(home.floor),
    };
    if victim.world_spatial().actor_position == temple {
        return Err("PvP encounter is at temple".into());
    }
    let hp = victim
        .actor_vitals()
        .ok_or("PvP initial vitals missing")?
        .health;
    victim.take_events();
    let modes = wire::ExpertFightModes {
        modes: attack::FightModes {
            fight_mode: attack::FightMode::Offensive,
            secure: false,
            ..attack::FightModes::DEFAULT
        },
        expert: wire::ExpertMode::RedFist,
    };
    let reply = attacker
        .session_mut()
        .gated_command(
            wire::CAPABILITY_PVP_EXPERT_V1,
            wire::COMMAND_TYPE_EXPERT_FIGHT_MODES_INTENT,
            &wire::encode_expert_fight_modes(&modes),
        )
        .await?;
    if reply.status != CommandStatus::Accepted
        || attack::decode_attack_intent_result(&reply.payload)
            .map_err(|error| format!("expert mode reply: {error:?}"))?
            != attack::AttackIntentDisposition::Ok
    {
        return Err(format!("PvP expert controls unavailable/refused: {reply:?}").into());
    }
    loop {
        let delta = attacker
            .session_mut()
            .read_gated_delta(wire::STATE_DOMAIN_EXPERT_FIGHT_MODES, 1)
            .await?;
        if delta.server_sequence <= reply.server_sequence {
            continue;
        }
        let actual = wire::decode_expert_fight_modes(&delta.payload)
            .map_err(|error| format!("expert frame: {error:?}"))?;
        if actual != modes {
            return Err(format!(
                "PvP world prerequisite blocked: RedFist/secure-off not served: {actual:?}"
            )
            .into());
        }
        break;
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    let mut damaged = false;
    let mut player_hit = false;
    let mut last_attack = String::new();
    loop {
        let attack = attacker.session_mut().attack_target(Some(&target)).await?;
        last_attack.clear();
        last_attack.push_str(&format!("{attack:?}"));
        victim.service_liveness(Duration::from_millis(50)).await?;
        attacker.service_liveness(Duration::from_millis(50)).await?;
        if victim.world_entities().is_some_and(|view| {
            view.others()
                .any(|entry| entry.kind == oteryn_dev_client::EntityKind::Creature)
        }) {
            return Err("PvP outcome contaminated: creature entered victim visibility".into());
        }
        for event in victim.take_events() {
            match event {
                SessionEvent::ActorVitals(delta) => {
                    damaged |= attack.status == CommandStatus::Accepted
                        && attack.disposition == attack::AttackIntentDisposition::Ok
                        && delta.value.health < hp;
                }
                SessionEvent::Presentation(delta) => {
                    use oteryn_protocol_oteryn::presentation::{
                        PresentationEventKind, PresentationValueKind,
                    };
                    for event in delta.value {
                        if let PresentationEventKind::ValueText {
                            kind: PresentationValueKind::DamageReceived,
                            target: damaged_actor,
                            attacker: dealer,
                            primary,
                            ..
                        } = event.kind
                            && damaged_actor == target
                            && primary.value > 0
                        {
                            if dealer != Some(source) {
                                return Err(format!(
                                    "PvP death contaminated by another damage source: {dealer:?}"
                                )
                                .into());
                            }
                            player_hit = true;
                        }
                    }
                }
                _ => {}
            }
        }
        attacker.take_events();
        if damaged
            && player_hit
            && victim.world_spatial().actor_position == temple
            && victim
                .actor_vitals()
                .is_some_and(|v| v.health > 0 && v.health == v.max_health)
        {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("PvP player prerequisite/encounter blocked: required both level>=8, Open policy, legal damage and lethal hit; incoming_damage={damaged} matching_player_damage_frame={player_hit} last={last_attack} victim_build={before:?}").into());
        }
    }
    attacker.session_mut().attack_target(None).await?;
    let latest = death_frame(&tab(victim, 2).await?)?;
    if latest.first() == history.first()
        || latest
            .first()
            .is_none_or(|(_, cause)| cause != &format!("Died at Level {}", before.1))
    {
        return Err(format!(
            "PvP respawn lacks new matching native history: before={history:?} after={latest:?}"
        )
        .into());
    }
    Ok((before, progress(victim).await?))
}

async fn release_fighter(client: &mut DevClientSession) -> TestResult {
    let stopped = client.session_mut().attack_target(None).await;
    let released = logout(client).await;
    stopped?;
    released
}

async fn adventurer_journey(
    row: &mut Row,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    npcs: &oteryn_world_bundle::npc::Table,
    tag: u8,
) -> TestResult {
    let mut victim = connect_character(clients, tag, 0).await?;
    let result: TestResult = async {
        // Character 0 is still ordinary here: this mount precedes fixture provisioning.
        earn_death_prerequisites(&mut victim, world, spawns, 8, 0).await?;
        if !blessings(&mut victim).await?.iter().any(|key| key == WISDOM.2) {
            buy_blessing(&mut victim, world, spawns, npcs, WISDOM).await?;
        }
        let build = progress(&mut victim).await?;
        if !(8..=20).contains(&build.1) {
            return Err(format!("Adventurer player prerequisite blocked: earned victim level={} outside 8..20; previous level>=21 or aggression cannot be reset by a player", build.1).into());
        }
        let held = blessings(&mut victim).await?;
        let owned = custody(victim.inventory().ok_or("Adventurer inventory missing")?);
        let mut attacker = connect_world(clients, tag + 1).await?;
        let mut earning_cancelled = false;
        let fight: TestResult = async {
            {
                let earning = earn_death_prerequisites(&mut attacker, world, spawns, 8, 0);
                tokio::pin!(earning);
                loop {
                    match tokio::time::timeout(Duration::from_millis(100), earning.as_mut()).await {
                        Ok(result) => { result?; break; }
                        Err(_) => {
                            if let Err(error) = victim.service_liveness(Duration::from_millis(1)).await {
                                earning_cancelled = true;
                                return Err(error.into());
                            }
                            victim.take_events();
                        }
                    }
                }
            }
            row.pass("adventurer-earned-prerequisites", (build, progress(&mut attacker).await?, &held));
            let (before, after) = observed_pvp_death(&mut victim, &mut attacker, world, spawns).await?;
            if before != build || after != build || blessings(&mut victim).await? != held {
                return Err(format!("Adventurer protection not observed: before={before:?} after={after:?} expected={build:?}; requires Open policy, never reached 21 and no forfeiture/skull").into());
            }
            wait_inventory(&mut victim, |view| custody(view) == owned, "Adventurer death changed custody").await?;
            row.pass("adventurer-pvp-keeps-xp-blessings-custody", (after, &held, &owned));
            saved_blessings(&mut victim, clients, 0, tag + 2, &held, after).await?;
            wait_inventory(&mut victim, |view| custody(view) == owned, "Adventurer relog changed custody").await?;
            row.pass("adventurer-relog", (&held, &owned));
            Ok(())
        }.await;
        if earning_cancelled {
            // The observer error dropped a pending attacker exchange. Close its
            // connection without sending cleanup on an uncertain reply/frame.
            drop(attacker);
            return fight;
        }
        let cleanup = release_fighter(&mut attacker).await;
        fight.and(cleanup)
    }.await;
    let cleanup = return_and_release(&mut victim, world).await;
    result.and(cleanup)
}

async fn blessing_death(
    row: &mut Row,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult {
    adventurer_journey(row, clients, world, spawns, npcs, 0xa0).await?;
    let mut client = connect_world(clients, 0xa3).await?;
    let result: TestResult = async {
        // Level 21 permanently excludes Adventurer, so the Twist case cannot
        // accidentally PASS by observing KeepAll from the low-level exemption.
        earn_death_prerequisites(&mut client, world, spawns, 21, 0).await?;
        if !blessings(&mut client).await?.is_empty() {
            observed_death(&mut client, world, spawns).await?;
            if blessings(&mut client).await? != [TWIST.2] && !blessings(&mut client).await?.is_empty() {
                return Err("blessing baseline prerequisite: PvE did not clear regular ownership".into());
            }
        }
        buy_blessing(&mut client, world, spawns, npcs, WISDOM).await?;
        buy_blessing(&mut client, world, spawns, npcs, TWIST).await?;
        let both = held_with(&[], &[WISDOM.2, TWIST.2]);
        if blessings(&mut client).await? != both { return Err("regular/Twist ownership baseline differs".into()); }
        row.pass("regular-and-twist-purchased", &both);
        duplicate_blessing(&mut client, world, npcs, TWIST).await?;
        row.pass("duplicate-purchase-refused", blessings(&mut client).await?);
        let (_, _, after) = observed_death(&mut client, world, spawns).await?;
        let twist = vec![TWIST.2.to_owned()];
        if blessings(&mut client).await? != twist { return Err("PvE must consume regular blessing and retain Twist".into()); }
        row.pass("pve-consumes-regular-keeps-twist", &twist);
        saved_blessings(&mut client, clients, 1, 0xa4, &twist, after).await?;
        row.pass("pve-relog", (&twist, after));
        buy_blessing(&mut client, world, spawns, npcs, WISDOM).await?;
        let mut attacker = connect_character(clients, 0xa5, 0).await?;
        let fight: TestResult = async {
            earn_death_prerequisites(&mut attacker, world, spawns, 8, 0).await?;
            let (before, after) = observed_pvp_death(&mut client, &mut attacker, world, spawns).await?;
            let regular = vec![WISDOM.2.to_owned()];
            if after.0 >= before.0 || blessings(&mut client).await? != regular {
                return Err(format!("Twist PvP must lose XP, consume only Twist and retain Wisdom: before={before:?} after={after:?}").into());
            }
            row.pass("twist-pvp-consumes-only-twist", (&regular, before, after));
            saved_blessings(&mut client, clients, 1, 0xa6, &regular, after).await?;
            row.pass("twist-relog", (&regular, after));
            Ok(())
        }.await;
        let cleanup = release_fighter(&mut attacker).await;
        fight.and(cleanup)
    }.await;
    let cleanup = return_and_release(&mut client, world).await;
    result.and(cleanup)
}

async fn blessing_services(
    row: &mut Row,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult {
    adventurer_journey(row, clients, world, spawns, npcs, 0xb0).await?;
    let mut client = connect_world(clients, 0xb3).await?;
    let result: TestResult = async {
        // A real PvE death clears previous regular ownership; never delete it in SQL.
        if !blessings(&mut client).await?.is_empty() { observed_death(&mut client, world, spawns).await?; }
        let baseline = blessings(&mut client).await?;
        if baseline.iter().any(|key| key != TWIST.2) { return Err(format!("purchase baseline still has regular blessings: {baseline:?}").into()); }
        let npc = approach_service(&mut client, world, npcs, WISDOM.0).await?;
        let owned = custody(client.inventory().ok_or("decline inventory missing")?);
        service_talk(&mut client, npc, "hi").await?;
        let quote = service_talk(&mut client, npc, WISDOM.1).await?;
        if quote != [format!("Do you want to buy {} at the price for your current level?", WISDOM.1)] { return Err(format!("blessing quote unavailable: {quote:?}").into()); }
        for text in ["no", "yes"] {
            let lines = service_talk(&mut client, npc, text).await?;
            if lines.iter().any(|line| line == "You have received the blessing.") || blessings(&mut client).await? != baseline {
                return Err(format!("declined/unquoted blessing confirmation changed ownership: {text} {lines:?}").into());
            }
            wait_inventory(&mut client, |view| custody(view) == owned, "decline/unquoted confirmation changed custody").await?;
        }
        row.pass("decline-and-unquoted-confirmation-unchanged", &baseline);
        for service in PILGRIMAGE.iter().copied().chain(std::iter::once(TWIST)) {
            buy_blessing(&mut client, world, spawns, npcs, service).await?;
            duplicate_blessing(&mut client, world, npcs, service).await?;
        }
        let expected = held_with(&[], &PILGRIMAGE.iter().map(|service| service.2).chain(std::iter::once(TWIST.2)).collect::<Vec<_>>());
        if blessings(&mut client).await? != expected { return Err("seven regular/enhanced blessings plus Twist not observed".into()); }
        row.pass("seven-blessings-purchased", &expected);
        row.pass("duplicate-purchases-refused", &expected);
        let build = progress(&mut client).await?;
        saved_blessings(&mut client, clients, 1, 0xb4, &expected, build).await?;
        row.pass("purchases-relog", (&expected, build));
        observed_death(&mut client, world, spawns).await?;
        let before = blessings(&mut client).await?;
        if before != [TWIST.2] { return Err(format!("Henricus prerequisite: PvE did not clear regular ownership: {before:?}").into()); }
        // Henricus charges the five regular prices plus 10%. Only real combat funds it.
        loop {
            let (_, level) = progress(&mut client).await?;
            let fee = blessing_price(level, WISDOM.2) * 5 * 11 / 10;
            earn_death_prerequisites(&mut client, world, spawns, level, fee).await?;
            if progress(&mut client).await?.1 == level { break; }
        }
        let npc = approach_service(&mut client, world, npcs, "oteryn:npc.henricus").await?;
        // No quest flag is fabricated. The actual server refusal names the exact
        // player prerequisite if this ordinary character has not earned completion.
        purchase_service(&mut client, npc, "blessings",
            "Do you want all five regular blessings at your current level price plus 10%? You must have completed The Inquisition Quest and hold none of these blessings.",
            "You have received the blessings of the Inquisition.").await
            .map_err(|error| format!("Henricus player prerequisite/purchase blocked: requires real Inquisition questline=25; {error}"))?;
        let five = held_with(&before, &PILGRIMAGE[..5].iter().map(|service| service.2).collect::<Vec<_>>());
        if blessings(&mut client).await? != five { return Err("Henricus did not grant exactly five regular blessings".into()); }
        row.pass("henricus-complete-set", &five);
        let owned = custody(client.inventory().ok_or("Henricus inventory missing")?);
        service_talk(&mut client, npc, "blessings").await?;
        let lines = service_talk(&mut client, npc, "yes").await?;
        if lines != ["You already have all five regular blessings."] || blessings(&mut client).await? != five { return Err(format!("Henricus duplicate not refused unchanged: {lines:?}").into()); }
        wait_inventory(&mut client, |view| custody(view) == owned, "Henricus duplicate charged money").await?;
        row.pass("henricus-duplicate-refused", lines);
        let build = progress(&mut client).await?;
        saved_blessings(&mut client, clients, 1, 0xb5, &five, build).await?;
        row.pass("henricus-relog", (&five, build));
        observed_death(&mut client, world, spawns).await?;
        buy_blessing(&mut client, world, spawns, npcs, WISDOM).await?;
        let partial = blessings(&mut client).await?;
        let npc = approach_service(&mut client, world, npcs, "oteryn:npc.henricus").await?;
        let owned = custody(client.inventory().ok_or("partial inventory missing")?);
        service_talk(&mut client, npc, "hi").await?;
        service_talk(&mut client, npc, "blessings").await?;
        let lines = service_talk(&mut client, npc, "yes").await?;
        if lines != ["You already have a regular blessing. I can only provide the complete set."] || blessings(&mut client).await? != partial { return Err(format!("Henricus partial set not refused unchanged: {lines:?}").into()); }
        wait_inventory(&mut client, |view| custody(view) == owned, "Henricus partial-set refusal charged money").await?;
        row.pass("henricus-partial-refused", lines);
        let build = progress(&mut client).await?;
        saved_blessings(&mut client, clients, 1, 0xb6, &partial, build).await?;
        Ok(())
    }.await;
    let cleanup = return_and_release(&mut client, world).await;
    result.and(cleanup)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(
        dead_code,
        reason = "Function-local mount requires the isolated helper unit-test harness."
    )]
    fn blessing_price_checks_boundaries_and_twist_cap() {
        for (level, regular, enhanced, twist) in [
            (1, 2000, 2600, 2000),
            (30, 2000, 2600, 2000),
            (31, 2200, 2860, 2200),
            (119, 19800, 25740, 19800),
            (120, 20000, 26000, 20000),
            (121, 20075, 26100, 20200),
            (270, 31250, 41000, 50000),
            (271, 31325, 41100, 50000),
        ] {
            assert_eq!(blessing_price(level, WISDOM.2), regular);
            assert_eq!(blessing_price(level, PILGRIMAGE[5].2), enhanced);
            assert_eq!(blessing_price(level, TWIST.2), twist);
        }
    }

    #[test]
    #[allow(
        dead_code,
        reason = "Function-local mount requires the isolated helper unit-test harness."
    )]
    fn blessing_union_preserves_keys_and_compares_order_independently() {
        let held = vec![TWIST.2.to_owned()];
        let both = held_with(&held, &[WISDOM.2, TWIST.2]);
        assert_eq!(both, [WISDOM.2, TWIST.2]);
        assert_eq!(held, [TWIST.2]);
        assert_eq!(held_with(&both, &[]), both);
        assert!(held_with(&[], &[]).is_empty());
    }

    #[test]
    #[allow(
        dead_code,
        reason = "Function-local mount requires the isolated helper unit-test harness."
    )]
    fn native_blessing_and_death_frames_fail_closed() -> TestResult {
        use oteryn_protocol_oteryn::cyclopedia_character::{StoreSummary, TabView};
        let frame = TabView::StoreSummary(StoreSummary {
            blessings: vec!["oteryn:blessing.the_wisdom_of_solitude".into()],
            permanent_prey_slots: 0,
            prey_wildcards: 0,
            hirelings: 0,
        })
        .encode()
        .map_err(|error| format!("summary encode: {error:?}"))?;
        assert_eq!(
            blessing_frame(&frame)?,
            ["oteryn:blessing.the_wisdom_of_solitude"]
        );
        for length in 0..frame.len() {
            assert!(blessing_frame(&frame[..length]).is_err());
        }
        assert!(blessing_frame(&[6, 2, 1, b'a', 1, b'a', 0, 0, 0]).is_err());
        assert!(blessing_frame(&[6, 1, 1, 255, 0, 0, 0]).is_err());
        let deaths = vec![(1234, "Died at Level 20".into())];
        let frame = TabView::RecentDeaths(deaths.clone())
            .encode()
            .map_err(|error| format!("history encode: {error:?}"))?;
        assert_eq!(death_frame(&frame)?, deaths);
        for length in 0..frame.len() {
            assert!(death_frame(&frame[..length]).is_err());
        }
        let mut trailing = frame;
        trailing.push(0);
        assert!(death_frame(&trailing).is_err());
        assert_eq!(death_frame(&[2, 0, 0])?, []);
        assert!(death_frame(&[2, 51, 0]).is_err());
        Ok(())
    }

    #[test]
    #[allow(
        dead_code,
        reason = "Function-local mount requires the isolated helper unit-test harness."
    )]
    fn custody_ignores_only_admission_handles_and_entry_order() -> TestResult {
        let first = items::ItemEntry {
            handle: std::num::NonZeroU64::new(1).ok_or("handle")?,
            item_definition_ref: NonZeroU32::new(1).ok_or("definition")?,
            count: NonZeroU32::new(3).ok_or("count")?,
            sub_type: 0,
        };
        let second = items::ItemEntry {
            item_definition_ref: NonZeroU32::new(2).ok_or("other definition")?,
            ..first
        };
        let before = CharacterInventory {
            main_backpack: Some(first),
            entries: vec![first, second],
            ..Default::default()
        };
        let mut after = before.clone();
        after.entries.reverse();
        after.entries[0].handle = std::num::NonZeroU64::new(5).ok_or("new handle")?;
        assert_eq!(custody(&before), custody(&after));
        after.entries[0].count = NonZeroU32::new(2).ok_or("changed count")?;
        assert_ne!(custody(&before), custody(&after));
        after = before.clone();
        after.main_backpack = None;
        assert_ne!(custody(&before), custody(&after));
        after = before.clone();
        after.entries[0].sub_type = 1;
        assert_ne!(custody(&before), custody(&after));
        after = before.clone();
        after.entries.remove(0);
        after.equipment.push(items::EquippedItem {
            slot: items::EquipmentSlot::Necklace,
            item: first,
        });
        assert_ne!(custody(&before), custody(&after));
        Ok(())
    }

    #[test]
    #[allow(
        dead_code,
        reason = "Function-local mount requires the isolated helper unit-test harness."
    )]
    fn death_projection_checks_reductions_floor_cap_and_delevelling() -> TestResult {
        assert_eq!(expected_death_progress((4200, 8), 0, false)?, (2000, 6));
        assert_eq!(expected_death_progress((4200, 8), 1, false)?, (2176, 6));
        assert_eq!(expected_death_progress((4200, 8), 0, true)?, (2660, 7));
        assert_eq!(expected_death_progress((99, 1), 0, false)?, (0, 1));
        assert_eq!(expected_death_progress((100, 2), 7, true)?, (86, 1));
        assert!(expected_death_progress((4200, 9), 0, false).is_err());
        assert!(expected_death_progress((4200, 8), 8, false).is_err());
        Ok(())
    }

    #[test]
    #[allow(
        dead_code,
        reason = "Function-local mount requires the isolated helper unit-test harness."
    )]
    fn progression_requires_exact_current_stats_layout() -> TestResult {
        use oteryn_protocol_oteryn::cyclopedia_character::{GeneralStats, TabView};
        let mut payload = TabView::GeneralStats(GeneralStats {
            experience: 4200,
            level: 8,
            last_vitals: Some((150, 30)),
            ..Default::default()
        })
        .encode()
        .map_err(|error| format!("stats encode: {error:?}"))?;
        assert_eq!(progress_frame(&payload)?, (4200, 8));
        assert!(progress_frame(&payload[..41]).is_err());
        payload[0] = 2;
        assert!(progress_frame(&payload).is_err());
        payload[0] = 1;
        payload[33] = 2;
        assert!(progress_frame(&payload).is_err());
        payload[33] = 0;
        payload[9..13].fill(0);
        assert!(progress_frame(&payload).is_err());
        Ok(())
    }

    #[test]
    #[allow(
        dead_code,
        reason = "The required function-local mount is not discoverable by libtest; run this exact helper/test source in the isolated unit-test harness."
    )]
    fn row_requires_every_action_and_rejects_duplicate_or_unknown_actions() {
        let mut row = Row::new("C19a", STARTER);
        assert!(!row.complete());
        for action in STARTER {
            row.pass(action, "unit observation");
        }
        assert!(row.complete());
        row.pass(STARTER[0], "duplicate");
        assert!(!row.complete());
        let mut unknown = Row::new("C19a", STARTER);
        unknown.pass("unregistered", ());
        for action in STARTER {
            unknown.pass(action, "unit observation");
        }
        assert!(!unknown.complete());
        assert!(!Row::new("C19a", &[]).complete());
    }
}
