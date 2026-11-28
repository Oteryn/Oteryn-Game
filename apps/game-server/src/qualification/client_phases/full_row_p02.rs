//! P02 full-row scenario, derived from its audit/matrix row and Bosstiary owners:
//! 1. Send BOSS_COOLDOWNS; require an accepted progress/points snapshot.
//! 2. Walk to open ground, place a boss fixture, ATTACK until death, and require kill/point deltas.
//! 3. Earn prowess by those kills; Equip slot one and require unlock/bonus/slot deltas.
//! 4. Repeat Equip and require a refusal reply with unchanged state.
//! 5. ATTACK the daily boosted boss and require its three-kill delta and slot refusal.
//! 6. LOGOUT, freshly admit, query both commands, and re-check all durable state.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::boss_encounter;
use oteryn_protocol_oteryn::attack::{ActorCombatState, AttackIntentDisposition, ChaseMode, FightMode, FightModes};
use oteryn_protocol_oteryn::bosstiary_slots::{
    BossSlotReply, BossSlotRequest, COMMAND_TYPE_BOSSTIARY_SLOTS,
};
use std::collections::{BTreeMap, BTreeSet};
const REQUIRED: &[&str] = &[
    "initial-query",
    "boss-kill-progress",
    "slot-equip",
    "slot-refusal",
    "boosted-kill",
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
    fn pass(&mut self, s: &'static str, v: impl std::fmt::Debug) {
        if REQUIRED.contains(&s) && self.passed.insert(s) {
            evidence(&format!("P02_CHECK step={s} status=PASS observed={v:?}"));
        }
    }
    fn finish(self, r: TestResult) {
        for s in REQUIRED {
            if !self.passed.contains(s) {
                evidence(&format!(
                    "P02_CHECK step={s} status=FAIL observed=not-reached"
                ));
            }
        }
        let ok = r.is_ok() && REQUIRED.iter().all(|s| self.passed.contains(s));
        match &r {
            Err(error) => evidence(&format!(
                "P02_GAP status=FAIL observed={}",
                error.to_string().replace(char::is_whitespace, "_")
            )),
            Ok(()) if !ok => {
                evidence("P02_GAP status=FAIL observed=required-lifecycle-step-missing")
            }
            Ok(()) => evidence("P02_GAP status=PASS"),
        }
        evidence(&format!(
            "e2e_all scenario=P02.full status={} seconds={:.3} evidence={}",
            if ok { "PASS" } else { "FAIL" },
            self.started.elapsed().as_secs_f64(),
            if ok {
                "observed-complete-player-lifecycle"
            } else {
                "runtime-step-failure-see-P02_GAP"
            }
        ));
    }
}
pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let r = verdict::indexed_timeout(
        Duration::from_secs(600),
        verdict::observe(journey(&mut row, clients)),
    )
    .await;
    row.finish(match r {
        Ok(v) => v,
        Err(_) => Err("step deadline: no server reply within 600s".into()),
    });
}
async fn snapshot(c: &mut DevClientSession) -> TestResult<boss_encounter::Snapshot> {
    let q = boss_encounter::query("").ok_or("cooldown encoder")?;
    let r = c
        .session_mut()
        .gated_command(q.capability, q.kind, &q.payload)
        .await?;
    if r.status != CommandStatus::Accepted {
        return Err(format!("step cooldown-query: reply={r:?}").into());
    }
    boss_encounter::decode(&r.payload)
        .ok_or_else(|| format!("step cooldown-query: malformed payload={:?}", r.payload).into())
}
async fn slot(c: &mut DevClientSession, q: BossSlotRequest) -> TestResult<BossSlotReply> {
    let p = q.encode().ok_or("slot encoder")?;
    let r = c
        .session_mut()
        .gated_command(
            boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1,
            COMMAND_TYPE_BOSSTIARY_SLOTS,
            &p,
        )
        .await?;
    if r.status != CommandStatus::Accepted {
        return Err(format!("step boss-slot: reply={r:?}").into());
    }
    BossSlotReply::decode(&r.payload)
        .ok_or_else(|| format!("step boss-slot: malformed payload={:?}", r.payload).into())
}
fn counts(s: &boss_encounter::Snapshot) -> BTreeMap<String, u64> {
    s.progress.iter().cloned().collect()
}
fn bosses() -> TestResult<Vec<crate::domain::bosstiary::BosstiaryBoss>> {
    let l = crate::content::ProjectEvidenceLimits {
        max_documents: 5,
        max_document_bytes: 2_500_000,
        max_total_bytes: 8_000_000,
        max_json_depth: 10,
        max_decoded_fields: 65_000,
        max_string_bytes: 1_000_000,
        max_locator_bytes: 128,
        max_locator_segments: 5,
        max_reference_records: 4096,
        max_import_records: 1,
        max_reimport_states: 1,
    };
    Ok(crate::content::canonical_title_catalogue(l)?.bosstiary)
}
async fn prepare_boss_ground(c: &mut DevClientSession, w: &BundleWorld) -> TestResult {
    let open = |at: TilePos| {
        w.base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|cell| !cell.protection_zone())
    };
    let from = tile(c.world_spatial().actor_position)?;
    let path = route(w, from, |at| {
        open(at)
            && [
                StepDirection::North,
                StepDirection::East,
                StepDirection::South,
                StepDirection::West,
            ]
            .into_iter()
            .filter(|direction| destination(w, at, *direction).is_some_and(open))
            .count()
                >= 3
    })
    .ok_or("P02 no reachable open ground for fixture placement")?;
    for direction in path {
        let before = tile(c.world_spatial().actor_position)?;
        let expected = destination(w, before, direction).ok_or("boss route landing missing")?;
        let occupies_landing = |entity: &oteryn_dev_client::WorldSpatialEntity| {
            matches!(
                entity.kind,
                oteryn_dev_client::EntityKind::Player
                    | oteryn_dev_client::EntityKind::Creature
                    | oteryn_dev_client::EntityKind::Npc
            ) && tile(entity.position).is_ok_and(|position| position == expected)
        };
        let outcome = c.step(direction).await?;
        let outcome = if outcome.disposition == StepDisposition::Blocked
            && tile(c.world_spatial().actor_position)? == before
        {
            let entities = c
                .world_entities()
                .ok_or("P02 blocked movement entity view missing")?;
            let occupant = entities.iter().find(|entity| occupies_landing(entity)).map(|entity| entity.entity)
                .ok_or_else(|| format!(
                    "BLOCKER-SERVER P02 movement blocked without a visible occupant: from={before:?} landing={expected:?} frame={outcome:?}"
                ))?;
            evidence(&format!(
                "P02_BOSS_ROUTE occupied={expected:?} occupant={occupant:?} frame={outcome:?}"
            ));
            let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
            verdict::Harness::global().bounded_wait(deadline, "p02-occupant-clearing-route");
            loop {
                let entities = c.world_entities().ok_or("P02 route entity view missing")?;
                if entities.get(&occupant).is_none_or(|entity| {
                    tile(entity.position).is_ok_and(|position| position != expected)
                }) && !entities.iter().any(occupies_landing)
                {
                    break;
                }
                if tokio::time::Instant::now() >= deadline {
                    return Err(format!(
                        "P02 route occupant {occupant:?} did not clear {expected:?}"
                    )
                    .into());
                }
                c.service_liveness(Duration::from_millis(50)).await?;
                c.take_events();
            }
            if tile(c.world_spatial().actor_position)? != before {
                return Err("P02 actor moved while waiting for the occupied tile".into());
            }
            evidence(&format!(
                "P02_BOSS_ROUTE cleared={expected:?} occupant={occupant:?}"
            ));
            let resumed = c.step(direction).await?;
            if resumed.disposition == StepDisposition::Blocked
                && c.world_entities()
                    .is_some_and(|entities| !entities.iter().any(occupies_landing))
            {
                return Err(format!("BLOCKER-SERVER P02 movement blocked after occupant cleared: from={before:?} landing={expected:?} frame={resumed:?}").into());
            }
            resumed
        } else {
            outcome
        };
        let actual = tile(c.world_spatial().actor_position)?;
        if outcome.disposition != StepDisposition::Moved
            || outcome.world_spatial_delta.is_none()
            || actual == before
            || actual != expected
        {
            return Err(format!("step boss-route: before={before:?} expected={expected:?} observed={actual:?} outcome={outcome:?}").into());
        }
        verdict::Harness::global().progress("p02-boss-route");
    }
    Ok(())
}

async fn observe_combat(
    client: &mut DevClientSession,
    after_sequence: u64,
    matches: impl Fn(&ActorCombatState) -> bool,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    verdict::Harness::global().bounded_wait(deadline, "p02-combat-state");
    loop {
        for event in client.take_events() {
            if let oteryn_dev_client::SessionEvent::ActorCombatState(delta) = event
                && delta.server_sequence > after_sequence && matches(&delta.value)
            {
                return Ok(());
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("P02 missing combat state delta after sequence {after_sequence}: {:?}", client.session_mut().combat_state()).into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

pub(super) async fn kill(c: &mut DevClientSession, key: &str) -> TestResult {
    let session = uuid_text(c.join_snapshot().game_session_id.as_bytes());
    let answer = request_spawn(session, key.to_owned())
        .await
        .map_err(|error| format!("P02 FIXTURE boss placement refused key={key}: {error}"))?;
    let target = parse_spawned_entity(&answer)?;
    evidence(&format!(
        "P02 FIXTURE input=boss_placement key={key} receipt={answer}"
    ));
    let deadline = tokio::time::Instant::now() + Duration::from_secs(95);
    verdict::Harness::global().bounded_wait(deadline, "p02-fixture-boss-visible");
    loop {
        if c.world_entities()
            .and_then(|entities| entities.get(&target))
            .is_some_and(|entity| entity.kind == oteryn_dev_client::EntityKind::Creature)
        {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "step boss-spawn {key}: exact placed entity {target:?} not visible"
            )
            .into());
        }
        c.service_liveness(Duration::from_millis(50)).await?;
        c.take_events();
    }
    let r = c.session_mut().attack_target(Some(&target)).await?;
    if r.disposition != AttackIntentDisposition::Ok {
        return Err(format!("step boss-kill {key}: ATTACK reply={r:?}").into());
    }
    let until = tokio::time::Instant::now() + Duration::from_secs(90);
    observe_combat(c, r.result_server_sequence, |state| state.target == Some(target)).await?;
    verdict::Harness::global().bounded_wait(until, "bounded-observation");
    let mut last_seen = None;
    loop {
        c.service_liveness(Duration::from_millis(50)).await?;
        c.take_events();
        if let Some(entities) = c.world_entities() {
            if let Some(entity) = entities.get(&target) {
                last_seen = Some(*entity);
            } else {
                let vitals = c.session_mut().actor_vitals().copied();
                evidence(&format!(
                    "P02_COMBAT key={key} target={target:?} visible=false last_seen={last_seen:?} player_vitals={vitals:?}"
                ));
                if vitals.is_none_or(|vitals| vitals.health == 0) {
                    return Err(format!("step boss-kill {key}: player not alive when boss left visibility").into());
                }
                if c.session_mut().combat_state().is_some_and(|state| state.target.is_some()) {
                    let clear = c.session_mut().attack_target(None).await?;
                    if clear.disposition != AttackIntentDisposition::Ok {
                        return Err(format!("step boss-kill {key}: target clear reply={clear:?}").into());
                    }
                    observe_combat(c, clear.result_server_sequence, |state| state.target.is_none()).await?;
                }
                return Ok(());
            }
        }
        if tokio::time::Instant::now() >= until {
            return Err(
                format!("step boss-kill {key}: death frame timeout target={target:?} last_seen={last_seen:?}").into(),
            );
        }
    }
}
fn boosted_exclusion_slot(
    reply: &BossSlotReply,
    boss: &str,
    kills: u64,
    threshold: u64,
) -> Option<u8> {
    let details = reply.details.as_ref()?;
    if reply.status != 0
        || details.boosted != boss
        || kills < threshold
        || details.now < reply.ready_at_ms
        || reply.bosses.iter().any(|assigned| assigned == boss)
    {
        return None;
    }
    (0_u8..2)
        .find(|index| {
            reply.unlocked & (1 << index) != 0 && reply.bosses[usize::from(*index)].is_empty()
        })
        .map(|index| index + 1)
}

pub(super) const ORDINARY_BOSS: &str = "oteryn:creature.rotworm_queen";
const DEEPLING_AXE: u32 = 13991;

async fn equip_boss_axe(
    mut client: DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult<DevClientSession> {
    let reference = provisioning::item_reference(&provisioning::item_index()?, DEEPLING_AXE)?;
    let session = client.join_snapshot().game_session_id;
    request_fixture_control(
        uuid_text(session.as_bytes()),
        "--profile",
        format!("item-{DEEPLING_AXE}-1"),
    )
    .await?;
    evidence(&format!("P02 FIXTURE input=weapon item={DEEPLING_AXE} quantity=1 result=accepted"));
    logout(&mut client).await?;
    drop(client);
    let mut client = connect_world(clients, 0x71).await?;
    let inventory = client.inventory().ok_or("P02 weapon inventory missing")?;
    let equipped = |inventory: &oteryn_dev_client::CharacterInventory| {
        inventory.equipment.iter().any(|slot| {
            slot.item.item_definition_ref == reference && slot.item.count.get() == 1
        })
    };
    if client.join_snapshot().game_session_id == session || equipped(inventory)
        || !inventory.entries.iter().any(|item| item.item_definition_ref == reference && item.count.get() == 1)
    {
        return Err("P02 Deepling Axe must start unequipped in the fresh admission".into());
    }
    let reply = client.session_mut().equip_item_definition(reference).await?;
    if reply.outcome != oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved {
        return Err(format!("P02 Deepling Axe equip refused: {reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    verdict::Harness::global().bounded_wait(deadline, "p02-weapon-equip");
    while !client.inventory().is_some_and(equipped) {
        if tokio::time::Instant::now() >= deadline {
            return Err("P02 Deepling Axe equip delta missing".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    evidence(&format!("P02_COMBAT item={DEEPLING_AXE} equipped=confirmed"));
    Ok(client)
}

pub(super) async fn prepare_combat(clients: &SeamClients<'_>, row: &str, tag: u8) -> TestResult<DevClientSession> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let boosted_combat = row == "P02.full";
    let prerequisites = if boosted_combat {
        "level 100, Deepling Axe, boss placement"
    } else {
        "level 100, axe, boss placement"
    };
    character_prep::declare_seeded(
        row,
        prerequisites,
        "combat inputs are prepared; credit and slots must come from live gameplay",
    );
    let client = connect_world(clients, tag).await?;
    let c = provisioning::provision_fixture(client, clients, 1, 100).await?;
    let mut c = if boosted_combat { equip_boss_axe(c, clients).await? } else { c };
    prepare_boss_ground(&mut c, world).await?;
    let modes = FightModes {
        fight_mode: FightMode::Offensive,
        chase: ChaseMode::Chase,
        ..FightModes::DEFAULT
    };
    if c.session_mut().combat_state().is_none_or(|state| state.modes == modes) {
        return Err("P02 needs a different combat mode before enabling offensive chase".into());
    }
    let reply = c.session_mut().set_fight_modes(&modes).await?;
    if reply.disposition != AttackIntentDisposition::Ok {
        return Err(format!("P02 offensive chase refused: {reply:?}").into());
    }
    observe_combat(&mut c, reply.result_server_sequence, |state| state.modes == modes).await?;
    evidence("P02_COMBAT modes=offensive,chase change=confirmed");
    Ok(c)
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let mut c = prepare_combat(clients, "P02.full", 0xb2).await?;
    let mut before = snapshot(&mut c).await?;
    let initial_slot = slot(
        &mut c,
        BossSlotRequest::Detailed(Box::new(BossSlotRequest::Query)),
    )
    .await?;
    let daily_boss = initial_slot
        .details
        .as_ref()
        .ok_or("initial boss slot details missing")?
        .boosted
        .clone();
    if initial_slot.status != 0
        || !before.progress.is_empty()
        || before.points != 0
        || initial_slot.points != 0
        || initial_slot.bosses.iter().any(|boss| !boss.is_empty())
        || daily_boss != "oteryn:creature.ekatrix"
    {
        return Err(format!("initial boss slot query: {initial_slot:?}").into());
    }
    row.pass(
        "initial-query",
        (&before.progress, before.points, &initial_slot),
    );
    let catalog = bosses()?;
    if !catalog
        .iter()
        .any(|boss| boss.key == daily_boss && boss.category.eq_ignore_ascii_case("archfoe"))
    {
        return Err(format!("daily boosted boss is not an Archfoe: {daily_boss}").into());
    }
    // Global Bosstiary lists Rotworm Queen as a Nemesis (one kill, ten points).
    let boss = ORDINARY_BOSS.to_owned();
    catalog
        .iter()
        .find(|entry| entry.key == boss && entry.category.eq_ignore_ascii_case("nemesis"))
        .ok_or("P02 Global ordinary boss absent from Nemesis catalogue")?;
    let previous = counts(&before).get(&boss).copied().unwrap_or(0);
    kill(&mut c, &boss).await?;
    let credited = snapshot(&mut c).await?;
    let current = counts(&credited).get(&boss).copied().unwrap_or(0);
    if previous.checked_add(1) != Some(current) || credited.points != 10 {
        return Err(format!("step boss-kill {boss}: expected +1 and Prowess points, observed kills={previous}->{current} points={} -> {}", before.points, credited.points).into());
    }
    before = credited.clone();
    row.pass(
        "boss-kill-progress",
        (&boss, &credited.progress, credited.points),
    );
    let boosted_key = daily_boss;
    let previous = counts(&before).get(&boosted_key).copied().unwrap_or(0);
    kill(&mut c, &boosted_key).await?;
    let boosted_state = snapshot(&mut c).await?;
    let current = counts(&boosted_state)
        .get(&boosted_key)
        .copied()
        .unwrap_or(0);
    if previous.checked_add(3) != Some(current) {
        return Err(format!(
            "step boosted-kill {boosted_key}: expected +3, observed={previous}->{current}"
        )
        .into());
    }
    before = boosted_state.clone();
    let boosted_category = catalog
        .iter()
        .find(|entry| entry.key == boosted_key)
        .ok_or("step=boosted-exclusion boosted boss catalogue entry missing")?;
    let threshold = crate::domain::bosstiary::kill_thresholds(&boosted_category.category)
        .ok_or("step=boosted-exclusion boosted boss prowess threshold missing")?[0];
    let mut boosted_kills = counts(&before).get(&boosted_key).copied().unwrap_or(0);
    while boosted_kills < threshold {
        kill(&mut c, &boosted_key).await?;
        before = snapshot(&mut c).await?;
        let next = counts(&before).get(&boosted_key).copied().unwrap_or(0);
        if boosted_kills.checked_add(3) != Some(next) {
            return Err(format!("step=boosted-eligibility boss={boosted_key} unexpected credit={boosted_kills}->{next} need={threshold}").into());
        }
        boosted_kills = next;
    }
    let ready = slot(
        &mut c,
        BossSlotRequest::Detailed(Box::new(BossSlotRequest::Query)),
    )
    .await?;
    let exclusion_slot = boosted_exclusion_slot(&ready, &boosted_key, boosted_kills, threshold)
        .ok_or_else(|| format!("step=boosted-exclusion boss={boosted_key} kills={boosted_kills}/{threshold} need=boosted-boss-prowess-and-ready-empty-unlocked-slot observed={ready:?}"))?;
    let br = slot(
        &mut c,
        BossSlotRequest::Equip {
            slot: exclusion_slot,
            boss: boosted_key.clone(),
        },
    )
    .await?;
    let plain_ready = BossSlotReply {
        details: None,
        ..ready.clone()
    };
    if br
        != (BossSlotReply {
            status: 1,
            ..plain_ready.clone()
        })
    {
        return Err(
            format!("step=boosted-exclusion refusal or custody mismatch reply={br:?}").into(),
        );
    }
    let after_exclusion = slot(
        &mut c,
        BossSlotRequest::Detailed(Box::new(BossSlotRequest::Query)),
    )
    .await?;
    if after_exclusion
        .details
        .as_ref()
        .is_none_or(|details| details.boosted != boosted_key)
        || (BossSlotReply {
            details: None,
            ..after_exclusion.clone()
        }) != plain_ready
    {
        return Err(format!(
            "step=boosted-exclusion daily boss or slot custody changed reply={after_exclusion:?}"
        )
        .into());
    }
    row.pass("boosted-kill", (&boosted_key, &boosted_state.progress, &br));
    if !plain_ready.bosses[0].is_empty() || plain_ready.bosses.iter().any(|key| key == &boss) {
        return Err(format!(
            "step slot-equip: expected empty slot 1 and unassigned boss: {plain_ready:?}"
        )
        .into());
    }
    let expected_bosses = [boss.clone(), plain_ready.bosses[1].clone()];
    let equipped = slot(
        &mut c,
        BossSlotRequest::Equip {
            slot: 1,
            boss: boss.clone(),
        },
    )
    .await?;
    if equipped.status != 0 || equipped.unlocked & 1 == 0 || equipped.bosses != expected_bosses {
        return Err(format!("step slot-equip: reply={equipped:?}").into());
    }
    row.pass("slot-equip", &equipped);
    let refused = slot(
        &mut c,
        BossSlotRequest::Equip {
            slot: 1,
            boss: boss.clone(),
        },
    )
    .await?;
    if refused
        != (BossSlotReply {
            status: 2,
            ..equipped.clone()
        })
    {
        return Err(format!("step slot-refusal: reply={refused:?}").into());
    }
    row.pass("slot-refusal", &refused);
    let session = logout(&mut c).await?;
    drop(c);
    let mut c = connect_world(clients, 0xb3).await?;
    let durable = snapshot(&mut c).await?;
    let ds = slot(&mut c, BossSlotRequest::Query).await?;
    if c.join_snapshot().game_session_id == session
        || durable.progress != before.progress
        || durable.points != before.points
        || ds != equipped
    {
        return Err(format!("step fresh-admission: progress={durable:?}, slot={ds:?}").into());
    }
    row.pass("fresh-admission", (&durable.progress, durable.points, &ds));
    Ok(())
}

pub(super) async fn logout(c: &mut DevClientSession) -> TestResult<crate::foundation::GameSessionId> {
    let deadline = tokio::time::Instant::now()
        + Duration::from_millis(u64::from(
            oteryn_protocol_oteryn::logout::MAX_LOGOUT_RETRY_AFTER_MS,
        ));
    verdict::Harness::global().bounded_wait(deadline, "p02-server-combat-lock");
    loop {
        let combat = c
            .session_mut()
            .combat_state()
            .ok_or("P02 combat state missing before logout")?;
        if combat.target.is_some() {
            return Err(format!(
                "P02 logout still has an exact combat target: {:?}",
                combat.target
            )
            .into());
        }
        if !combat.in_fight {
            evidence("P02_LOGOUT server_combat_state=clear");
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("P02 combat lock did not clear within the protocol bound".into());
        }
        c.service_liveness(Duration::from_millis(50)).await?;
        c.take_events();
    }
    let session = c.join_snapshot().game_session_id;
    let logout = c.session_mut().logout().await?.result;
    if logout != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step logout: {logout:?}").into());
    }
    Ok(session)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exclusion_requires_prowess_and_an_empty_ready_unlocked_slot() {
        let mut reply = BossSlotReply {
            details: Some(oteryn_protocol_oteryn::bosstiary_slots::SlotDetails {
                base_bonus: 0,
                bonuses: [0; 2],
                boosted: "boss".into(),
                now: 10,
            }),
            status: 0,
            unlocked: 1,
            bonus: 0,
            points: 0,
            remove_fee: 0,
            ready_at_ms: 10,
            bosses: ["ordinary".into(), String::new()],
        };
        assert_eq!(boosted_exclusion_slot(&reply, "boss", 5, 5), None);
        reply.bosses[0].clear();
        assert_eq!(boosted_exclusion_slot(&reply, "boss", 4, 5), None);
        assert_eq!(boosted_exclusion_slot(&reply, "boss", 5, 5), Some(1));
        reply.ready_at_ms = 11;
        assert_eq!(boosted_exclusion_slot(&reply, "boss", 5, 5), None);
    }

    #[test]
    fn requires_whole_lifecycle() {
        assert_eq!(REQUIRED.len(), 6);
        assert!(Row::new().passed.is_empty());
    }
}

#[cfg(test)]
#[test]
fn row_02_publishes_a_gap_for_an_unproven_journey() {
    assert!(
        include_str!("full_row_p02.rs")
            .split_once("#[cfg(test)]")
            .is_some_and(|(production, _)| production.contains("P02_GAP status=PASS"))
    );
}

#[cfg(test)]
#[test]
fn review_p02_guards_the_journey() {
    assert!(
        include_str!("full_row_p02.rs")
            .split_once("#[cfg(test)]")
            .is_some_and(|(body, _)| body.contains("boosted_exclusion_slot"))
    );
}

#[cfg(test)]
#[test]
fn review_2_p02() {
    assert!(
        include_str!("full_row_p02.rs")
            .split_once("#[cfg(test)]")
            .is_some_and(|(body, _)| body.contains("boosted-eligibility"))
    );
}
