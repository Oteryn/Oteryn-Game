//! P02 full-row scenario, derived from its audit/matrix row and Bosstiary owners:
//! 1. Send BOSS_COOLDOWNS; require an accepted progress/points snapshot.
//! 2. Walk to a compiled-world boss, ATTACK until death, and require kill/point deltas.
//! 3. Earn prowess by those kills; Equip slot one and require unlock/bonus/slot deltas.
//! 4. Repeat Equip and require a refusal reply with unchanged state.
//! 5. ATTACK the daily boosted boss and require its three-kill delta and slot refusal.
//! 6. LOGOUT, freshly admit, query both commands, and re-check all durable state.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::boss_encounter;
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
async fn kill(
    c: &mut DevClientSession,
    w: &BundleWorld,
    s: &oteryn_world_bundle::spawn::Table,
    key: &str,
) -> TestResult {
    let points: Vec<_> = s
        .sources
        .iter()
        .flat_map(|source| {
            source.points.iter().filter_map(move |point| {
                s.creatures
                    .get(point.creature as usize)
                    .filter(|creature| creature.key == key)
                    .map(|_| TilePos {
                        x: point.x,
                        y: point.y,
                        floor: source.floor,
                    })
            })
        })
        .collect();
    let adjacent = |at: TilePos, point: TilePos| {
        at != point
            && at.floor == point.floor
            && at.x.abs_diff(point.x).max(at.y.abs_diff(point.y)) == 1
    };
    let from = tile(c.world_spatial().actor_position)?;
    let path = route(w, from, |at| {
        w.base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|cell| !cell.protection_zone())
            && points.iter().any(|&point| adjacent(at, point))
    })
    .ok_or_else(|| format!("step boss-route {key}: no reachable authored spawn"))?;
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
            return Err(format!("step boss-route {key}: before={before:?} expected={expected:?} observed={actual:?} outcome={outcome:?}").into());
        }
        verdict::Harness::global().progress("p02-boss-route");
    }
    let at = tile(c.world_spatial().actor_position)?;
    let point = points
        .into_iter()
        .find(|&point| adjacent(at, point))
        .ok_or("boss spawn missing at planned approach")?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(95);
    verdict::Harness::global().bounded_wait(deadline, "p02-authored-boss-spawn");
    let target = loop {
        if let Some(target) = c.world_entities().and_then(|entities| {
            entities
                .iter()
                .find(|entity| {
                    entity.kind == oteryn_dev_client::EntityKind::Creature
                        && tile(entity.position).is_ok_and(|position| position == point)
                })
                .copied()
        }) {
            break target;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step boss-spawn {key}: no creature at {point:?}").into());
        }
        c.service_liveness(Duration::from_millis(50)).await?;
        c.take_events();
    };
    let r = c.session_mut().attack_target(Some(&target.entity)).await?;
    if r.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(format!("step boss-kill {key}: ATTACK reply={r:?}").into());
    }
    let until = tokio::time::Instant::now() + Duration::from_secs(90);
    verdict::Harness::global().bounded_wait(until, "bounded-observation");
    loop {
        c.service_liveness(Duration::from_millis(50)).await?;
        c.take_events();
        if c.world_entities()
            .is_some_and(|es| es.get(&target.entity).is_none())
        {
            c.session_mut().attack_target(None).await?;
            return Ok(());
        }
        if tokio::time::Instant::now() >= until {
            c.session_mut().attack_target(None).await?;
            return Err(
                format!("step boss-kill {key}: ATTACK accepted, death frame timeout").into(),
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

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let meta = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut c = connect_world(clients, 0xb2).await?;
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
    if initial_slot.status != 0 {
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
    let from = tile(c.world_spatial().actor_position)?;
    let spawned = catalog
        .iter()
        .filter(|boss| full_row_p05::has_reachable_spawn(world, from, &meta.spawns, &boss.key));
    let mut ordinary = None;
    let mut boosted = None;
    for b in spawned {
        kill(&mut c, world, &meta.spawns, &b.key).await?;
        let after = snapshot(&mut c).await?;
        let d = counts(&after)
            .get(&b.key)
            .copied()
            .unwrap_or(0)
            .saturating_sub(counts(&before).get(&b.key).copied().unwrap_or(0));
        match (b.key == daily_boss, d) {
            (false, 1) => {
                let prowess = crate::domain::bosstiary::kill_thresholds(&b.category)
                    .ok_or_else(|| format!("step=prowess unknown boss category={}", b.category))?
                    [0];
                if ordinary
                    .as_ref()
                    .is_none_or(|(_, _, _, goal)| prowess < *goal)
                {
                    ordinary = Some((b.key.clone(), b.category.clone(), after.clone(), prowess));
                }
            }
            (true, 3) => boosted = Some((b.key.clone(), after.clone())),
            _ => {
                return Err(format!(
                    "step boss-kill {}: progress delta={d}, reply={after:?}",
                    b.key
                )
                .into());
            }
        }
        before = after;
        if ordinary.is_some() && boosted.is_some() {
            break;
        }
    }
    let (boss, _, credited, prowess) =
        ordinary.ok_or("step boss-kill: no reachable compiled Bosstiary boss produced delta=1")?;
    row.pass(
        "boss-kill-progress",
        (&boss, &credited.progress, credited.points),
    );
    while counts(&before).get(&boss).copied().unwrap_or(0) < prowess {
        let previous = counts(&before).get(&boss).copied().unwrap_or(0);
        kill(&mut c, world, &meta.spawns, &boss).await.map_err(|error| {
            verdict::record_error(error.as_ref());
            verdict::context(format!("step=prowess boss={boss} kills={previous}/{prowess} need=another-live-boss {error}"), error)
        })?;
        before = snapshot(&mut c).await?;
        let current = counts(&before).get(&boss).copied().unwrap_or(0);
        if previous.checked_add(1) != Some(current) {
            return Err(format!(
                "step=prowess boss={boss} observed={previous}->{current} need={prowess}"
            )
            .into());
        }
    }
    let (boosted_key, boosted_state) =
        boosted.ok_or("step=boosted-kill no reachable boss produced delta=3")?;
    let boosted_category = catalog
        .iter()
        .find(|entry| entry.key == boosted_key)
        .ok_or("step=boosted-exclusion boosted boss catalogue entry missing")?;
    let threshold = crate::domain::bosstiary::kill_thresholds(&boosted_category.category)
        .ok_or("step=boosted-exclusion boosted boss prowess threshold missing")?[0];
    let mut boosted_kills = counts(&before).get(&boosted_key).copied().unwrap_or(0);
    while boosted_kills < threshold {
        kill(&mut c, world, &meta.spawns, &boosted_key).await?;
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
