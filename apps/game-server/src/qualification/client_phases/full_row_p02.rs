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
use oteryn_protocol_oteryn::attack::{
    ActorCombatState, AttackIntentDisposition, ChaseMode, FightMode, FightModes,
};
use oteryn_protocol_oteryn::bosstiary_slots::{
    BossSlotReply, BossSlotRequest, COMMAND_TYPE_BOSSTIARY_SLOTS,
};
use std::collections::BTreeSet;
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
                && delta.server_sequence > after_sequence
                && matches(&delta.value)
            {
                return Ok(());
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "P02 missing combat state delta after sequence {after_sequence}: {:?}",
                client.session_mut().combat_state()
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

async fn fight(
    c: &mut DevClientSession,
    key: &str,
) -> TestResult<(oteryn_dev_client::EntityRef, tokio::time::Instant)> {
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
    observe_combat(c, r.result_server_sequence, |state| {
        state.target == Some(target)
    })
    .await?;
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
                    return Err(format!(
                        "step boss-kill {key}: player not alive when boss left visibility"
                    )
                    .into());
                }
                return Ok((target, until));
            }
        }
        if tokio::time::Instant::now() >= until {
            return Err(
                format!("step boss-kill {key}: death frame timeout target={target:?} last_seen={last_seen:?}").into(),
            );
        }
    }
}
const BOOSTED_BOSS: &str = "oteryn:creature.ekatrix";
const ARCHFOE_PROWESS: u64 = 5;

// The Channel reward occurrence seeds its UUID suffix from the exact death
// key, Character and millisecond carried in the UUID prefix. The statistics
// ledger records physical deaths even when the XP settlement has no receipt.
fn credit_belongs_to_death(death_key: &[u8], principal: &[u8; 16], occurrence: &[u8; 16]) -> bool {
    use sha2::{Digest, Sha256};
    let mut millis = [0; 8];
    millis[2..].copy_from_slice(&occurrence[..6]);
    let digest = Sha256::new()
        .chain_update(b"oteryn:combat-death-xp-occurrence:v1")
        .chain_update(death_key)
        .chain_update(principal)
        .chain_update(millis)
        .finalize();
    occurrence[6] == 0x70 | (digest[0] & 0x0f)
        && occurrence[7] == digest[1]
        && occurrence[8] == 0x80 | (digest[2] & 0x3f)
        && occurrence[9..] == digest[3..10]
}

pub(super) async fn credited_death(
    c: &mut DevClientSession,
    clients: &SeamClients<'_>,
    key: &str,
    credit: i32,
) -> TestResult {
    let (target, deadline) = fight(c, key).await?;
    let principal = clients.characters[1];
    let prefix = [
        clients.world.as_slice(),
        clients.channel.as_slice(),
        &clients.scope_generation.to_be_bytes(),
    ]
    .concat();
    verdict::Harness::global().bounded_wait(deadline, "p02-exact-death-proof");
    let (occurrence, received) = tokio::time::timeout_at(deadline, async {
        let mut database = sqlx::PgConnection::connect(clients.url).await?;
        loop {
            let death: Option<Vec<u8>> = sqlx::query_scalar(
                "SELECT death_key FROM game_statistics_creature_deaths
                 WHERE world_id=encode($1,'hex')::uuid AND creature_key=$2 AND boss
                   AND octet_length(death_key)=52 AND substring(death_key FROM 1 FOR 40)=$3
                   AND substring(death_key FROM 45 FOR 8)=$4
                   AND substring(sha256(convert_to('oteryn:runtime-player-placement:v1','UTF8')
                       || death_key) FROM 1 FOR 16)=$5",
            )
            .bind(clients.world.as_slice())
            .bind(key)
            .bind(&prefix)
            .bind(target.generation.to_be_bytes().as_slice())
            .bind(target.identity.as_slice())
            .fetch_optional(&mut database)
            .await?;
            if let Some(death) = death {
                let receipts: Vec<(Vec<u8>, i32)> = sqlx::query_as(
                    "SELECT uuid_send(occurrence_id), kill_credit::integer
                     FROM game_character_bosstiary_kill_receipts
                     WHERE character_id=encode($1,'hex')::uuid AND boss_key=$2 LIMIT 3",
                )
                .bind(principal.as_slice())
                .bind(key)
                .fetch_all(&mut database)
                .await?;
                if receipts.len() > 2 {
                    return Err(format!("P02 too many credit receipts for {key}").into());
                }
                for (occurrence, received) in receipts {
                    let occurrence: [u8; 16] = occurrence.as_slice().try_into()?;
                    if credit_belongs_to_death(&death, &principal, &occurrence) {
                        return TestResult::Ok((occurrence, received));
                    }
                }
            }
            c.service_liveness(Duration::from_millis(100)).await?;
            c.take_events();
        }
    })
    .await
    .map_err(|_| {
        format!("P02 no matching physical death and Bosstiary credit for {key} {target:?}")
    })??;
    if received != credit {
        return Err(
            format!("P02 exact death {target:?} expected credit {credit}, got {received}").into(),
        );
    }
    if c.actor_vitals().is_none_or(|vitals| vitals.health == 0) {
        return Err("P02 player died before the exact death proof committed".into());
    }
    if c.session_mut()
        .combat_state()
        .is_some_and(|state| state.target.is_some())
    {
        let clear = c.session_mut().attack_target(None).await?;
        if clear.disposition != AttackIntentDisposition::Ok {
            return Err(format!("step boss-kill {key}: target clear reply={clear:?}").into());
        }
        observe_combat(c, clear.result_server_sequence, |state| {
            state.target.is_none()
        })
        .await?;
    }
    evidence(&format!(
        "P02_DEATH entity={target:?} occurrence={} boss={key} credit={credit}",
        uuid_text(&occurrence)
    ));
    Ok(())
}

fn exact_progress(
    observed: &boss_encounter::Snapshot,
    entries: &[(&str, u64)],
    points: u64,
) -> TestResult {
    if observed.points != points
        || observed.progress_next.is_some()
        || observed.progress.len() != entries.len()
        || entries.iter().any(|(key, kills)| {
            !observed
                .progress
                .iter()
                .any(|(actual_key, actual_kills)| actual_key == key && actual_kills == kills)
        })
    {
        return Err(format!(
            "P02 progress expected {entries:?} points={points}, observed {observed:?}"
        )
        .into());
    }
    Ok(())
}

fn exact_slots(
    reply: &BossSlotReply,
    status: u8,
    unlocked: u8,
    bonus: u16,
    points: u32,
    bosses: [&str; 2],
) -> TestResult {
    if reply.status != status
        || reply.unlocked != unlocked
        || reply.bonus != bonus
        || reply.points != points
        || reply.bosses != bosses
        || reply.remove_fee != 0
    {
        return Err(format!("P02 slots expected status={status} unlocked={unlocked} bonus={bonus} points={points} bosses={bosses:?}, observed {reply:?}").into());
    }
    Ok(())
}

fn boosted_exclusion_slot(reply: &BossSlotReply, kills: u64) -> TestResult<u8> {
    exact_slots(reply, 0, 1, 27, 20, ["", ""])?;
    let details = reply
        .details
        .as_ref()
        .ok_or("P02 missing detailed slot state")?;
    if kills < ARCHFOE_PROWESS
        || details.boosted != BOOSTED_BOSS
        || details.base_bonus != 27
        || details.bonuses != [0, 0]
        || details.now < reply.ready_at_ms
    {
        return Err(format!("step=boosted-eligibility kills={kills} observed={reply:?}").into());
    }
    Ok(1)
}

pub(super) const ORDINARY_BOSS: &str = "oteryn:creature.rotworm_queen";
const DEEPLING_AXE: u32 = 13991;

async fn equip_boss_axe(
    mut client: DevClientSession,
    clients: &SeamClients<'_>,
    funded: bool,
) -> TestResult<DevClientSession> {
    let reference = provisioning::item_reference(&provisioning::item_index()?, DEEPLING_AXE)?;
    let session = client.join_snapshot().game_session_id;
    request_fixture_control(
        uuid_text(session.as_bytes()),
        "--profile",
        format!("item-{DEEPLING_AXE}-1"),
    )
    .await?;
    evidence(&format!(
        "P02 FIXTURE input=weapon item={DEEPLING_AXE} quantity=1 result=accepted"
    ));
    if funded {
        request_fixture_control(
            uuid_text(session.as_bytes()),
            "--profile",
            "item-3031-20".to_owned(),
        )
        .await?;
        evidence("P02 FIXTURE input=gold item=3031 quantity=20 result=accepted");
    }
    logout(&mut client).await?;
    drop(client);
    let mut client = connect_world(clients, 0x71).await?;
    let inventory = client.inventory().ok_or("P02 weapon inventory missing")?;
    let equipped = |inventory: &oteryn_dev_client::CharacterInventory| {
        inventory
            .equipment
            .iter()
            .any(|slot| slot.item.item_definition_ref == reference && slot.item.count.get() == 1)
    };
    if client.join_snapshot().game_session_id == session
        || equipped(inventory)
        || !inventory
            .entries
            .iter()
            .any(|item| item.item_definition_ref == reference && item.count.get() == 1)
    {
        return Err("P02 Deepling Axe must start unequipped in the fresh admission".into());
    }
    let reply = client
        .session_mut()
        .equip_item_definition(reference)
        .await?;
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
    evidence(&format!(
        "P02_COMBAT item={DEEPLING_AXE} equipped=confirmed"
    ));
    Ok(client)
}

pub(super) async fn prepare_combat(
    clients: &SeamClients<'_>,
    row: &str,
    tag: u8,
) -> TestResult<DevClientSession> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let funded = row == "P02.client";
    let prerequisites = if funded {
        "level 100, Deepling Axe, boss placement, 20 gold"
    } else {
        "level 100, Deepling Axe, boss placement"
    };
    character_prep::declare_seeded(
        row,
        prerequisites,
        "combat inputs are prepared; credit and slots must come from live gameplay",
    );
    let client = connect_world(clients, tag).await?;
    let c = provisioning::provision_once(client, clients, 1, 100).await?;
    let mut c = equip_boss_axe(c, clients, funded).await?;
    prepare_boss_ground(&mut c, world).await?;
    let modes = FightModes {
        fight_mode: FightMode::Offensive,
        chase: ChaseMode::Chase,
        ..FightModes::DEFAULT
    };
    if c.session_mut()
        .combat_state()
        .is_none_or(|state| state.modes == modes)
    {
        return Err("P02 needs a different combat mode before enabling offensive chase".into());
    }
    let reply = c.session_mut().set_fight_modes(&modes).await?;
    if reply.disposition != AttackIntentDisposition::Ok {
        return Err(format!("P02 offensive chase refused: {reply:?}").into());
    }
    observe_combat(&mut c, reply.result_server_sequence, |state| {
        state.modes == modes
    })
    .await?;
    evidence("P02_COMBAT modes=offensive,chase change=confirmed");
    Ok(c)
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let mut c = prepare_combat(clients, "P02.full", 0xb2).await?;
    let before = snapshot(&mut c).await?;
    exact_progress(&before, &[], 0)?;
    let initial_slot = slot(
        &mut c,
        BossSlotRequest::Detailed(Box::new(BossSlotRequest::Query)),
    )
    .await?;
    exact_slots(&initial_slot, 0, 0, 25, 0, ["", ""])?;
    let details = initial_slot
        .details
        .as_ref()
        .ok_or("initial boss slot details missing")?;
    if details.boosted != BOOSTED_BOSS
        || details.base_bonus != 25
        || details.bonuses != [0, 0]
        || initial_slot.ready_at_ms != 0
    {
        return Err(format!("initial boss slot query requires {BOOSTED_BOSS} and pristine slot details: {initial_slot:?}").into());
    }
    row.pass(
        "initial-query",
        (&before.progress, before.points, &initial_slot),
    );
    let catalog = bosses()?;
    for (key, category) in [(ORDINARY_BOSS, "nemesis"), (BOOSTED_BOSS, "archfoe")] {
        if !catalog
            .iter()
            .any(|boss| boss.key == key && boss.category.eq_ignore_ascii_case(category))
        {
            return Err(format!("P02 Global boss {key} absent from {category} catalogue").into());
        }
    }

    credited_death(&mut c, clients, ORDINARY_BOSS, 1).await?;
    let credited = snapshot(&mut c).await?;
    exact_progress(&credited, &[(ORDINARY_BOSS, 1)], 10)?;
    row.pass("boss-kill-progress", (&credited.progress, credited.points));

    let final_progress = [(ORDINARY_BOSS, 1), (BOOSTED_BOSS, 6)];
    for (kills, points) in [(3, 10), (6, 20)] {
        credited_death(&mut c, clients, BOOSTED_BOSS, 3).await?;
        let progress = snapshot(&mut c).await?;
        exact_progress(
            &progress,
            &[(ORDINARY_BOSS, 1), (BOOSTED_BOSS, kills)],
            points,
        )?;
        evidence(&format!(
            "P02_PROGRESS boosted_credits={kills} points={points} progress={:?}",
            progress.progress
        ));
    }
    let ready = slot(
        &mut c,
        BossSlotRequest::Detailed(Box::new(BossSlotRequest::Query)),
    )
    .await?;
    let exclusion_slot = boosted_exclusion_slot(&ready, final_progress[1].1)?;
    let br = slot(
        &mut c,
        BossSlotRequest::Equip {
            slot: exclusion_slot,
            boss: BOOSTED_BOSS.into(),
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
    if after_exclusion.details.as_ref().is_none_or(|details| {
        details.boosted != BOOSTED_BOSS || details.base_bonus != 27 || details.bonuses != [0, 0]
    }) || (BossSlotReply {
        details: None,
        ..after_exclusion.clone()
    }) != plain_ready
    {
        return Err(format!(
            "step=boosted-exclusion daily boss or slot custody changed reply={after_exclusion:?}"
        )
        .into());
    }
    row.pass("boosted-kill", (&final_progress, &br));
    let boss = ORDINARY_BOSS.to_owned();
    let equipped = slot(
        &mut c,
        BossSlotRequest::Equip {
            slot: 1,
            boss: boss.clone(),
        },
    )
    .await?;
    exact_slots(&equipped, 0, 1, 27, 20, [ORDINARY_BOSS, ""])?;
    let equipped_details = detailed_equipment(&mut c, &equipped).await?;
    row.pass("slot-equip", (&equipped, &equipped_details));
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
    exact_progress(&durable, &final_progress, 20)?;
    exact_slots(&ds, 0, 1, 27, 20, [ORDINARY_BOSS, ""])?;
    if c.join_snapshot().game_session_id == session || ds != equipped {
        return Err(format!("step fresh-admission: progress={durable:?}, slot={ds:?}").into());
    }
    let durable_details = detailed_equipment(&mut c, &ds).await?;
    row.pass(
        "fresh-admission",
        (&durable.progress, durable.points, &ds, &durable_details),
    );
    Ok(())
}

async fn detailed_equipment(
    c: &mut DevClientSession,
    plain: &BossSlotReply,
) -> TestResult<oteryn_protocol_oteryn::bosstiary_slots::SlotDetails> {
    let mut reply = slot(
        c,
        BossSlotRequest::Detailed(Box::new(BossSlotRequest::Query)),
    )
    .await?;
    let details = reply
        .details
        .take()
        .ok_or("P02 missing equipped slot details")?;
    if &reply != plain
        || details.base_bonus != 27
        || details.bonuses != [27, 0]
        || details.boosted != BOOSTED_BOSS
    {
        return Err(format!("P02 equipped slot detail mismatch: {reply:?} {details:?}").into());
    }
    Ok(details)
}

pub(super) async fn logout(
    c: &mut DevClientSession,
) -> TestResult<crate::foundation::GameSessionId> {
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
    fn exclusion_requires_prowess_and_exact_slots() {
        let mut reply = BossSlotReply {
            details: Some(oteryn_protocol_oteryn::bosstiary_slots::SlotDetails {
                base_bonus: 27,
                bonuses: [0; 2],
                boosted: BOOSTED_BOSS.into(),
                now: 10,
            }),
            status: 0,
            unlocked: 1,
            bonus: 27,
            points: 20,
            remove_fee: 0,
            ready_at_ms: 10,
            bosses: [String::new(), String::new()],
        };
        assert!(boosted_exclusion_slot(&reply, 4).is_err());
        assert_eq!(boosted_exclusion_slot(&reply, 6).ok(), Some(1));
        for (unlocked, bonus) in [(3, 27), (2, 27), (1, 26)] {
            reply.unlocked = unlocked;
            reply.bonus = bonus;
            assert!(boosted_exclusion_slot(&reply, 6).is_err());
        }
    }

    #[test]
    fn exact_credit_rejects_another_actor_generation_or_principal() {
        let actor = [7; 52];
        let principal = [8; 16];
        let occurrence = [
            0, 0, 0, 0, 0, 1, 123, 59, 128, 14, 209, 32, 138, 219, 123, 110,
        ];
        assert!(credit_belongs_to_death(&actor, &principal, &occurrence));
        let mut wrong_actor = actor;
        wrong_actor[43] ^= 1;
        assert!(!credit_belongs_to_death(
            &wrong_actor,
            &principal,
            &occurrence
        ));
        wrong_actor = actor;
        wrong_actor[51] ^= 1;
        assert!(!credit_belongs_to_death(
            &wrong_actor,
            &principal,
            &occurrence
        ));
        assert!(!credit_belongs_to_death(&actor, &[9; 16], &occurrence));
    }

    #[test]
    fn exact_progress_rejects_unrelated_entries_lost_ordinary_credit_and_wrong_points() {
        let expected = [(ORDINARY_BOSS, 1), (BOOSTED_BOSS, 6)];
        let state = boss_encounter::Snapshot {
            progress: expected
                .map(|(key, kills)| (key.to_owned(), kills))
                .to_vec(),
            points: 20,
            ..Default::default()
        };
        assert!(exact_progress(&state, &expected, 20).is_ok());
        let mut wrong = state.clone();
        wrong.progress.push(("unrelated".into(), 1));
        assert!(exact_progress(&wrong, &expected, 20).is_err());
        wrong = state.clone();
        wrong.progress[0].1 = 2;
        assert!(exact_progress(&wrong, &expected, 20).is_err());
        wrong = state;
        wrong.points = 19;
        assert!(exact_progress(&wrong, &expected, 20).is_err());
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
