//! P01 full-row player scenario, derived from the matrix/audit and the native
//! `charm_progression` owner:
//!
//! 1. Admit the node-boot Character and send QUERY; expect an accepted reply
//!    containing the complete charm view and its current durable Bestiary rows.
//! 2. Send ASSIGN for a still-locked major charm and a real Bestiary race;
//!    expect the explicit `CharmLocked` refusal and no assignment/balance delta.
//! 3. Walk to an authored creature spawn, send ATTACK, and wait for a credited
//!    kill; send QUERY and expect that race's kill count to increase by exactly
//!    one (repeating ordinary hunts until the final threshold awards enough
//!    charm points for the selected charm).
//! 4. Send UNLOCK with the stage observed by QUERY; expect `Unlocked`, a
//!    one-stage increase, and exactly the advertised charm-point debit.
//! 5. Send ASSIGN for that unlocked charm and the completed race; expect
//!    `Assigned`, the race assignment delta, and `effect_active=true`.
//! 6. Walk back to a live creature, send ATTACK, and require a second credited
//!    kill and Zap's energy impact from an otherwise physical attack, with the
//!    assignment still active.
//! 7. Send LOGOUT, obtain a fresh admission, then send QUERY; expect the kill
//!    counts, charm-point balance, unlocked stage, and assigned race to equal
//!    the pre-logout durable view.

use super::*;
use oteryn_dev_client::{DevClientSession, EntityDetail, SessionEvent};
use oteryn_protocol_oteryn::charm::{CharmAssignDisposition, CharmUnlockDisposition};
use oteryn_protocol_oteryn::charm_progression::{
    self as wire, CharmProgressionAction, CharmProgressionOutcome, CharmProgressionReply,
};
use std::num::NonZeroU32;

const DEADLINE: Duration = Duration::from_secs(1_800);

struct Row {
    started: std::time::Instant,
    step: &'static str,
}

impl Row {
    fn new() -> Self {
        Self {
            started: std::time::Instant::now(),
            step: "admission-query",
        }
    }

    fn at(&mut self, step: &'static str) {
        self.step = step;
    }

    fn finish(self, result: TestResult) {
        match result {
            Ok(()) => evidence(&format!(
                "SEAM_EVIDENCE e2e_all scenario=P01.full status=PASS seconds={:.3}",
                self.started.elapsed().as_secs_f64()
            )),
            Err(error) => evidence(&format!(
                "SEAM_EVIDENCE e2e_all scenario=P01.full status=FAIL seconds={:.3} evidence=step:{};observed:{};need:reachable-player-earned-charm-points-and-native-reply",
                self.started.elapsed().as_secs_f64(),
                self.step,
                compact(&error.to_string())
            )),
        }
    }
}

fn compact(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "-_:.,=".contains(character) {
                character
            } else {
                '_'
            }
        })
        .take(180)
        .collect()
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = verdict::indexed_timeout(DEADLINE, verdict::observe(journey(&mut row, clients))).await;
    let result = match result {
        Ok(result) => result,
        Err(_) => Err(format!("deadline while executing {}", row.step).into()),
    };
    row.finish(result);
}

async fn command(
    client: &mut DevClientSession,
    action: CharmProgressionAction,
) -> TestResult<CharmProgressionReply> {
    let payload = wire::encode_charm_progression_request(&action)
        .map_err(|error| format!("encode {action:?}: {error:?}"))?;
    let response = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_BESTIARY_CHARM_PROGRESSION_V1,
            wire::COMMAND_TYPE_CHARM_PROGRESSION,
            &payload,
        )
        .await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!(
            "CHARM_PROGRESSION {action:?} status={:?} payload_len={}",
            response.status,
            response.payload.len()
        )
        .into());
    }
    wire::decode_charm_progression_reply(&response.payload)
        .map_err(|error| format!("decode {action:?}: {error:?}").into())
}

async fn query(client: &mut DevClientSession) -> TestResult<CharmProgressionReply> {
    let reply = command(client, CharmProgressionAction::Query).await?;
    if reply.outcome != CharmProgressionOutcome::Query || reply.charms.charms.is_empty() {
        return Err(format!("query reply missing native views: {reply:?}").into());
    }
    Ok(reply)
}

fn count(reply: &CharmProgressionReply, race: NonZeroU32) -> u32 {
    reply
        .bestiary
        .iter()
        .find(|entry| entry.race == race)
        .map_or(0, |entry| entry.kill_count)
}

fn race_complete(reply: &CharmProgressionReply, race: NonZeroU32) -> bool {
    reply
        .bestiary
        .iter()
        .find(|entry| entry.race == race)
        .is_some_and(|entry| entry.kill_count == entry.kill_thresholds[2])
}

fn charm_effect_observed(energy_hit: bool, credited_kill: bool) -> bool {
    energy_hit && credited_kill
}

fn energy_damage(fact: &oteryn_protocol_oteryn::analyser::AnalyserFact) -> bool {
    use oteryn_protocol_oteryn::{analyser::{AnalyserFact, AnalyserImpact}, damage_element::DamageElement};
    matches!(fact, AnalyserFact::Impact(AnalyserImpact::Damage { element: DamageElement::Energy, .. }))
}

async fn kill_adjacent(
    client: &mut DevClientSession,
    name: &str,
    expected: oteryn_dev_client::EntityRef,
) -> TestResult<bool> {
    use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
    let here = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                super::full_row_p02::named_creature(entry, name)
                    && entry.entity == expected
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
        .ok_or("hunt route has no adjacent creature")?;
    client.take_events();
    let attack = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    if attack.disposition != AttackIntentDisposition::Ok {
        return Err(format!("ATTACK refused: {attack:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    let mut energy = false;
    let mut damaged = false;
    loop {
        tokio::time::timeout_at(deadline, client.service_liveness(Duration::from_millis(50))).await??;
        for event in client.take_events() {
            match event {
                SessionEvent::WorldSpatialEntities(delta) => {
                    damaged |= delta.value.update.iter().any(|entry| {
                        entry.entity == target.entity && matches!(entry.detail,
                            EntityDetail::Actor { health_percent, .. } |
                            EntityDetail::ActorWithIcons { health_percent, .. } if health_percent < 100)
                    });
                }
                SessionEvent::AnalyserFacts(delta) if delta.server_sequence > attack.result_server_sequence => {
                    energy |= delta.value.iter().any(energy_damage);
                }
                SessionEvent::AnalyserAttributedFacts(delta) if delta.server_sequence > attack.result_server_sequence => {
                    energy |= delta.value.iter().any(|fact| energy_damage(&fact.fact));
                }
                _ => {}
            }
        }
        if client.world_entities().is_some_and(|entities| entities.get(&target.entity).is_none()) {
            client.session_mut().attack_target(None).await?;
            evidence(&format!("P01 combat target={:?} health-update={damaged} energy-impact={energy}", target.entity));
            return Ok(energy);
        }
    }
}

async fn hunt_once(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    key: &str,
    name: &str,
) -> TestResult<bool> {
    let target = travel_to_spawn_target(client, world, spawns, key).await?;
    kill_adjacent(client, name, target).await
}

async fn rat_reward(
    database: &mut sqlx::PgConnection,
    character: &[u8; 16],
) -> TestResult<Option<Vec<u8>>> {
    Ok(sqlx::query_scalar(
        "SELECT m.death_key FROM game_party_kill_xp_members m JOIN game_party_kill_xp_receipts r USING(death_key)
         WHERE m.character_id=encode($1,'hex')::uuid AND r.creature_key='oteryn:creature.rat'
           AND m.xp_awarded > 0 ORDER BY r.committed_at DESC,r.death_key DESC LIMIT 1",
    ).bind(character.as_slice()).fetch_optional(database).await?)
}

#[derive(Clone, Copy)]
struct CurrencyHunt {
    race: NonZeroU32,
    points: u32,
    seconds: u32,
}

fn deadline_plan(current: u32, cost: u32, hunts: &[CurrencyHunt], budget: u32) -> Option<Vec<NonZeroU32>> {
    let needed = usize::try_from(cost.saturating_sub(current)).ok()?;
    // For each reachable balance, retain the cheapest schedule. Iterate backwards so a race
    // completion can be used only once, even when several balances overshoot the target.
    let mut best = vec![None; needed + 1];
    best[0] = Some((0_u32, Vec::new()));
    for hunt in hunts {
        for balance in (0..needed).rev() {
            let Some((seconds, races)) = &best[balance] else { continue };
            let Some(seconds) = seconds.checked_add(hunt.seconds).filter(|seconds| *seconds <= budget) else { continue };
            let next = balance.saturating_add(usize::try_from(hunt.points).ok()?).min(needed);
            if best[next].as_ref().is_none_or(|(current, _)| seconds < *current) {
                let mut races = races.clone();
                races.push(hunt.race);
                best[next] = Some((seconds, races));
            }
        }
    }
    best.pop().flatten().map(|(_, races)| races)
}

fn hunt_seconds(kills: u32, respawns_ms: impl IntoIterator<Item = u32>) -> Option<u32> {
    // Keep the next spawn time for each point. Account for serial combat as well as respawn,
    // and budget one approach per source. The row's hard deadline still bounds live combat.
    use std::{cmp::Reverse, collections::BinaryHeap};
    let mut ready: BinaryHeap<_> = respawns_ms.into_iter().map(|delay| Reverse((0_u64, delay))).collect();
    let mut elapsed = 0_u64;
    for _ in 0..kills {
        let Reverse((next, delay)) = ready.pop()?;
        elapsed = elapsed.max(next).checked_add(2_000)?;
        ready.push(Reverse((elapsed.checked_add(u64::from(delay))?, delay)));
    }
    u32::try_from(elapsed.div_ceil(1000)).ok()?.checked_add(30)
}

fn completion_fixture_prefix(count: u32, threshold: u32, fits: bool) -> u32 {
    if fits { 0 } else { threshold.saturating_sub(1).saturating_sub(count) }
}

/// Qualification-only history setup, after releasing the disposable fixture's Character.
/// Every seeded race remains incomplete so the real combat path still earns each award.
async fn prepare_bestiary_prefix(
    db: &mut sqlx::PgConnection, clients: &SeamClients<'_>,
    races: &[(crate::domain::bestiary::BestiaryRace, u32)], progress: &CharmProgressionReply,
    selected: &[NonZeroU32],
) -> TestResult {
    use sqlx::Connection;
    if clients.url != required("NODE_BOOT_DATABASE_URL")? {
        return Err("step=currency-setup needed=disposable-node-boot-database".into());
    }
    wait_character_released(clients, 1).await?;
    let character = clients.characters[1].as_slice();
    let mut tx = db.begin().await?;
    sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(character).fetch_one(&mut *tx).await?;
    // The dispatch-owned PostgreSQL Charm fixture uses the same receipt-chain shape.
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    let mut offset = 0_i64;
    for id in selected {
        let (race, _) = &races[usize::try_from(id.get() - 1)?];
        let before = count(progress, *id);
        let threshold = race.final_kill_threshold();
        let prefix = completion_fixture_prefix(before, threshold, false);
        if prefix == 0 { continue; }
        let projected: i64 = sqlx::query_scalar(
            "SELECT coalesce((SELECT kill_count FROM game_character_bestiary_progress
                            WHERE character_id=encode($1,'hex')::uuid AND race_key=$2),0)",
        ).bind(character).bind(race.key()).fetch_one(&mut *tx).await?;
        if projected != i64::from(before) {
            return Err(format!("step=currency-setup observed=changed-count race={}", race.key()).into());
        }
        sqlx::query(
            "WITH receipts AS (
                INSERT INTO game_character_bestiary_kill_receipts(
                    bestiary_occurrence_id,command_binding,race_digest,character_id,
                    original_character_revision,committed_character_revision,
                    level_before,level_after,experience_before,experience_after,
                    race_key,race_definition_revision,final_kill_threshold,kill_count_before,kill_count_after,
                    profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,
                    declaration_revision,policy_revision,reward_revision,committed_at)
                SELECT game_character_uuid_v7(),convert_to('P01-fixture-prefix','UTF8'),sha256(convert_to($2,'UTF8')),
                    s.character_id,s.character_revision+$6+k-1,s.character_revision+$6+k,
                    s.level,s.level,s.total_experience,s.total_experience,
                    $2,$3,$4,$5+k-1,$5+k,s.profile_revision,s.ruleset_revision,s.content_revision,
                    s.simulation_revision,s.evidence_revision,s.declaration_revision,s.policy_revision,s.reward_revision,0
                FROM game_character_progression_state s CROSS JOIN generate_series(1::bigint,$7::bigint) k
                WHERE s.character_id=encode($1,'hex')::uuid
                RETURNING character_id,race_key,kill_count_after,committed_character_revision,bestiary_occurrence_id)
             INSERT INTO game_character_bestiary_progress(
                character_id,race_key,kill_count,committed_character_revision,last_bestiary_occurrence_id)
             SELECT character_id,race_key,kill_count_after,committed_character_revision,bestiary_occurrence_id
                FROM receipts WHERE kill_count_after=$5+$7
             ON CONFLICT(character_id,race_key) DO UPDATE SET kill_count=excluded.kill_count,
                committed_character_revision=excluded.committed_character_revision,
                last_bestiary_occurrence_id=excluded.last_bestiary_occurrence_id",
        ).bind(character).bind(race.key()).bind(race.definition_revision()).bind(i64::from(threshold))
            .bind(i64::from(before)).bind(offset).bind(i64::from(prefix)).execute(&mut *tx).await?;
        offset += i64::from(prefix);
    }
    sqlx::query("UPDATE game_character_roots SET character_revision=character_revision+$2 WHERE character_id=encode($1,'hex')::uuid")
        .bind(character).bind(offset).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_character_progression_state SET character_revision=character_revision+$2 WHERE character_id=encode($1,'hex')::uuid")
        .bind(character).bind(offset).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
fn fastest_source(spawns: &oteryn_world_bundle::spawn::Table, creature: &str, kills: u32) -> Option<(usize, u32)> {
    let index = u32::try_from(spawns.creatures.binary_search_by(|entry| entry.key.as_str().cmp(creature)).ok()?).ok()?;
    spawns.sources.iter().enumerate().filter_map(|(source, record)| {
        let respawns = record.points.iter().filter(|point| point.creature == index && point.boss_policy.is_none())
            .map(|point| point.respawn_ms);
        hunt_seconds(kills, respawns).map(|seconds| (source, seconds))
    }).min_by_key(|(_, seconds)| *seconds)
}

fn authored_bestiary() -> TestResult<Vec<(crate::domain::bestiary::BestiaryRace, u32)>> {
    let limits = crate::content::ProjectEvidenceLimits {
        max_documents: 5, max_document_bytes: 2_500_000, max_total_bytes: 8_000_000,
        max_json_depth: 10, max_decoded_fields: 65_000, max_string_bytes: 1_000_000,
        max_locator_bytes: 128, max_locator_segments: 5, max_reference_records: 4096,
        max_import_records: 1, max_reimport_states: 1,
    };
    let mut races = crate::content::canonical_bestiary_rows(limits)?;
    races.sort_unstable_by(|left, right| left.0.key().cmp(right.0.key()));
    Ok(races)
}

pub(super) async fn logout_for_fixture(client: &mut DevClientSession) -> TestResult {
    fixture_logout(client, async |client| Ok(client.session_mut().logout().await?.result), async |client, duration| {
        let deadline = tokio::time::Instant::now() + duration;
        verdict::Harness::global().bounded_wait(deadline, "fixture-logout-combat-lock");
        while let Some(remaining) = deadline.checked_duration_since(tokio::time::Instant::now()).filter(|time| !time.is_zero()) {
            client.service_liveness(remaining.min(Duration::from_millis(250))).await?;
            client.take_events();
        }
        Ok(())
    }).await
}

async fn fixture_logout<S>(
    session: &mut S,
    mut send: impl AsyncFnMut(&mut S) -> TestResult<Option<oteryn_protocol_oteryn::logout::LogoutResult>>,
    mut wait: impl AsyncFnMut(&mut S, Duration) -> TestResult,
) -> TestResult {
    use oteryn_protocol_oteryn::logout::LogoutResult;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    loop {
        let result = tokio::time::timeout_at(deadline, send(session)).await??;
        let delay = match result {
            Some(LogoutResult::Accepted) => return Ok(()),
            Some(LogoutResult::InFight { retry_after_ms }) => Duration::from_millis(u64::from(retry_after_ms).max(1)),
            Some(LogoutResult::Busy) => Duration::from_millis(250),
            _ => return Err(format!("step=fixture-logout observed={result:?}").into()),
        };
        if delay >= deadline.saturating_duration_since(tokio::time::Instant::now()) {
            return Err(format!("step=fixture-logout observed={result:?};retry-exceeds-deadline").into());
        }
        tokio::time::timeout_at(deadline, wait(session, delay)).await??;
    }
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
    let mut client = connect_world(clients, 0xa2).await?;
    if !client
        .selected_capabilities()
        .contains(&wire::CAPABILITY_BESTIARY_CHARM_PROGRESSION_V1)
    {
        return Err("admission omitted capability 67".into());
    }
    let initial = query(&mut client).await?;
    evidence("P01_CHECK admission-query PASS accepted-native-views");

    row.at("player-earned-kill-count");
    const RAT: &str = "oteryn:creature.rat";
    use sqlx::Connection;
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let races = authored_bestiary()?;
    let rat_index = races.iter().position(|(race, _)| race.key() == RAT).ok_or("rat absent from Bestiary")?;
    let race = NonZeroU32::new(u32::try_from(rat_index + 1)?).ok_or("invalid Rat index")?;
    let rat_limit = races[rat_index].0.final_kill_threshold();
    let before_reward = rat_reward(&mut database, &clients.characters[1]).await?;
    hunt_once(&mut client, world, &metadata.spawns, RAT, "Rat").await?;
    let after_reward = rat_reward(&mut database, &clients.characters[1]).await?;
    let mut current = query(&mut client).await?;
    if after_reward.is_none() || after_reward == before_reward
        || count(&initial, race).saturating_add(1).min(rat_limit) != count(&current, race)
    {
        return Err("credited Rat kill did not produce the expected Bestiary count".into());
    }
    evidence(&format!(
        "P01_CHECK kill-count PASS race={} count={}",
        race,
        count(&current, race)
    ));

    let catalogue = crate::content::charm_source::CanonicalCharmCatalogue::embedded()?;
    let zap = catalogue.catalogue().definitions().position(|entry| entry.key.as_str() == "oteryn:charm.zap")
        .and_then(|index| u32::try_from(index + 1).ok()).and_then(NonZeroU32::new)
        .ok_or("Zap absent from authored catalogue")?;
    let locked = current.charms.charms.iter().find(|entry| entry.charm == zap)
        .filter(|entry| entry.unlocked_stage == 0 && entry.next_stage_cost > 0)
        .ok_or("step=locked-assignment needed=locked-Zap")?;
    let (charm, stage, cost) = (locked.charm, locked.unlocked_stage, locked.next_stage_cost);

    row.at("locked-assignment-refusal");
    let refused = command(&mut client, CharmProgressionAction::Assign { charm, race }).await?;
    if refused.outcome != CharmProgressionOutcome::Assign(CharmAssignDisposition::CharmLocked)
        || refused.charms != current.charms
    {
        return Err(format!("locked ASSIGN reply/delta mismatch: {refused:?}").into());
    }
    evidence("P01_CHECK locked-assignment-refusal PASS disposition=CharmLocked");

    row.at("earn-unlock-currency");
    let budget = DEADLINE.saturating_sub(row.started.elapsed()).saturating_sub(Duration::from_secs(180));
    let rat_left = rat_limit.saturating_sub(count(&current, race));
    let mut sources = std::collections::BTreeMap::new();
    let mut hunts = Vec::new();
    for (index, (authored, points)) in races.iter().enumerate() {
        let id = NonZeroU32::new(u32::try_from(index + 1)?).ok_or("invalid Bestiary index")?;
        let left = authored.final_kill_threshold().saturating_sub(count(&current, id));
        if left > 0 && *points > 0
            && let Some((source, seconds)) = fastest_source(&metadata.spawns, authored.key(), left)
        {
            sources.insert(id, (source, seconds));
            if id != race {
                hunts.push(CurrencyHunt { race: id, points: *points, seconds });
            }
        }
    }
    let rat_seconds = if rat_left == 0 { 0 } else {
        sources.get(&race).ok_or("step=earn-unlock-currency needed=repeatable-Rat-source")?.1
    };
    let seconds = u32::try_from(budget.as_secs())?;
    let balance = current.charms.charm_points_available.saturating_add(if rat_left == 0 { 0 } else { races[rat_index].1 });
    let plan = seconds.checked_sub(rat_seconds).and_then(|remaining| deadline_plan(balance, cost, &hunts, remaining));
    let used_prefix = plan.is_none();
    let plan = if let Some(plan) = plan {
        plan
    } else {
        let mut final_hunts = Vec::new();
        for hunt in &hunts {
            let (authored, _) = &races[usize::try_from(hunt.race.get() - 1)?];
            if let Some((source, seconds)) = fastest_source(&metadata.spawns, authored.key(), 1) {
                sources.insert(hunt.race, (source, seconds));
                final_hunts.push(CurrencyHunt { seconds, ..*hunt });
            }
        }
        // Break equal-time ties toward higher-value races to minimize walks and fixture records.
        final_hunts.sort_by_key(|hunt| std::cmp::Reverse(hunt.points));
        let rat_final = if rat_left == 0 { 0 } else {
            let (source, seconds) = fastest_source(&metadata.spawns, RAT, 1)
                .ok_or("step=currency-setup needed=live-Rat-spawn")?;
            sources.insert(race, (source, seconds));
            seconds
        };
        let plan = seconds.checked_sub(rat_final).and_then(|remaining| deadline_plan(balance, cost, &final_hunts, remaining))
            .ok_or("step=currency-setup needed=spawned-races-for-enough-live-completions")?;
        let selected = std::iter::once(race).chain(plan.iter().copied()).collect::<Vec<_>>();
        logout_for_fixture(&mut client).await?;
        drop(client);
        prepare_bestiary_prefix(&mut database, clients, &races, &current, &selected).await?;
        client = connect_world(clients, 0xa4).await?;
        let prepared = query(&mut client).await?;
        if prepared.charms != current.charms || selected.iter().any(|id| {
            let index = id.get() as usize - 1;
            let before = count(&current, *id);
            count(&prepared, *id) != before + completion_fixture_prefix(before, races[index].0.final_kill_threshold(), false)
        }) {
            return Err("step=currency-setup observed=fixture-changed-charm-state-or-wrong-prefix".into());
        }
        current = prepared;
        evidence("P01_CHECK currency-setup fixture=incomplete-bestiary-receipt-prefix");
        plan
    };
    for id in std::iter::once(race).chain(plan) {
        if race_complete(&current, id) { continue; }
        let (authored, points) = &races[usize::try_from(id.get() - 1)?];
        let name = super::full_row_p02::creature_display_name(authored.key())?;
        let source = sources.get(&id).ok_or("step=earn-unlock-currency observed=missing-planned-source")?.0;
        let cluster = oteryn_world_bundle::spawn::Table {
            creatures: metadata.spawns.creatures.clone(),
            sources: vec![metadata.spawns.sources[source].clone()],
        };
        while !race_complete(&current, id) {
            let spawns = if used_prefix { &metadata.spawns } else { &cluster };
            hunt_once(&mut client, world, spawns, authored.key(), &name).await?;
            let next = query(&mut client).await?;
            let next_count = count(&next, id);
            let award = if next_count == authored.final_kill_threshold() { *points } else { 0 };
            if count(&current, id).checked_add(1) != Some(next_count)
                || current.charms.charm_points_available.checked_add(award) != Some(next.charms.charm_points_available)
            {
                return Err(format!("step=earn-unlock-currency observed=unexpected-count-or-award-on-{}", authored.key()).into());
            }
            current = next;
        }
    }
    if current.charms.charm_points_available < cost {
        return Err(format!("step=earn-unlock-currency observed={};needed={cost}-player-earned-major-charm-points",
            current.charms.charm_points_available).into());
    }

    row.at("unlock-stage");
    let unlocked = command(
        &mut client,
        CharmProgressionAction::Unlock {
            charm,
            expected_stage: stage,
        },
    )
    .await?;
    let unlocked_state = unlocked
        .charms
        .charms
        .get(usize::try_from(charm.get() - 1)?)
        .ok_or("unlocked charm vanished")?;
    if unlocked.outcome != CharmProgressionOutcome::Unlock(CharmUnlockDisposition::Unlocked)
        || unlocked_state.unlocked_stage != stage + 1
        || unlocked.charms.charm_points_available.checked_add(cost) != Some(current.charms.charm_points_available)
    {
        return Err(format!("UNLOCK reply/delta mismatch: {unlocked:?}").into());
    }
    evidence("P01_CHECK unlock-stage PASS stage-and-balance-delta");

    row.at("assign-unlocked");
    let assigned = command(&mut client, CharmProgressionAction::Assign { charm, race }).await?;
    let assigned_state = assigned.charms.charms.iter().find(|entry| entry.charm == charm)
        .ok_or("assigned charm absent from mutation reply")?;
    if assigned.outcome != CharmProgressionOutcome::Assign(CharmAssignDisposition::Assigned)
        || assigned_state.assigned_race != Some(race)
        || !assigned_state.effect_active
    {
        return Err(format!("ASSIGN reply/delta mismatch: {assigned:?}").into());
    }
    evidence("P01_CHECK assign-unlocked PASS assignment-and-active-hook");

    row.at("assigned-combat-hook");
    use oteryn_protocol_oteryn::item_view::EquipmentSlot;
    use oteryn_protocol_oteryn::attack::{FightMode, FightModes, AttackIntentDisposition};
    let hands = client.inventory().ok_or("step=combat observed=no-equipment-view")?.equipment.iter()
        .filter(|entry| matches!(entry.slot, EquipmentSlot::LeftHand | EquipmentSlot::RightHand))
        .map(|entry| entry.item.handle).collect::<Vec<_>>();
    for handle in hands {
        let moved = client.move_item(&oteryn_dev_client::ItemMoveIntent {
            source: handle, destination: oteryn_dev_client::ItemMoveDestination::MainBackpack,
        }).await?;
        if moved.outcome != oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved {
            return Err("step=combat needed=unarmed-physical-attacks".into());
        }
    }
    let stance = client.session_mut().set_fight_modes(&FightModes { fight_mode: FightMode::Defensive, ..FightModes::DEFAULT }).await?;
    if stance.disposition != AttackIntentDisposition::Ok {
        return Err("step=combat observed=defensive-stance-refused".into());
    }
    let mut observed = false;
    // Stage-one Zap procs at 5%; 180 eligible trials keep random failure below 0.01%.
    for _ in 0..180 {
        let before = rat_reward(&mut database, &clients.characters[1]).await?;
        let energy = hunt_once(&mut client, world, &metadata.spawns, "oteryn:creature.rat", "Rat").await?;
        let after = rat_reward(&mut database, &clients.characters[1]).await?;
        let credited = after.is_some() && after != before;
        if !credited {
            return Err("step=combat observed=rat-vanished-without-new-kill-receipt".into());
        }
        if charm_effect_observed(energy, credited) {
            observed = true;
            break;
        }
    }
    let after_hook = query(&mut client).await?;
    let after_state = after_hook.charms.charms.iter().find(|entry| entry.charm == charm)
        .ok_or("post-combat charm absent from reply")?;
    if !observed || after_state.assigned_race != Some(race) || !after_state.effect_active {
        return Err(format!("step=combat observed=zap-energy-hit:{observed},assignment:{after_state:?}").into());
    }
    evidence("P01_CHECK assigned-combat-hook PASS unarmed-Zap-energy-impact-and-credited-rat-kill");

    row.at("logout-fresh-admission");
    let old_session = client.join_snapshot().game_session_id;
    logout_for_fixture(&mut client).await?;
    drop(client);
    let mut fresh = connect_world(clients, 0xa3).await?;
    if fresh.join_snapshot().game_session_id == old_session {
        return Err("fresh admission reused the old game session".into());
    }
    let durable = query(&mut fresh).await?;
    if durable.bestiary != after_hook.bestiary || durable.charms != after_hook.charms {
        return Err(format!("fresh QUERY lost durable state: {durable:?}").into());
    }
    if fresh.session_mut().logout().await?.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err("fresh LOGOUT refused after charm readback".into());
    }
    evidence("P01_CHECK logout-fresh-admission PASS exact-durable-views");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_token_is_single_line_and_bounded() {
        let token = compact("a reply with spaces\nand punctuation!?/x");
        assert!(!token.contains([' ', '\n']));
        assert!(token.len() <= 180);
    }
}

#[cfg(test)]
mod combat_proof_tests {
    use super::*;

    #[test]
    fn kill_credit_without_elemental_charm_damage_is_insufficient() {
        assert!(!charm_effect_observed(false, true));
        assert!(!charm_effect_observed(true, false));
        assert!(charm_effect_observed(true, true));
    }
}

#[cfg(test)]
mod delta_tests {
    use super::*;

    #[test]
    fn currency_plan_from_zero_earns_the_full_zap_cost() -> TestResult {
        let race = |n| NonZeroU32::new(n).ok_or("test race index is zero");
        let id = race(1)?;
        let available = [CurrencyHunt { race: id, points: 320, seconds: 80 }];
        assert_eq!(deadline_plan(0, 320, &available, 90), Some(vec![id]));
        assert_eq!(deadline_plan(320, 320, &available, 0), Some(vec![]));
        Ok(())
    }
}

#[cfg(test)]
mod review_regressions {
    use super::*;
    #[test]
    fn spawn_cadence_excludes_unfinishable_bestiary_race() -> TestResult {
        let cat = NonZeroU32::new(1).ok_or("cat index")?;
        let feasible = NonZeroU32::new(2).ok_or("viable index")?;
        // One initial Cat and 24 subsequent spawns, 90 seconds apart.
        let cat_seconds = hunt_seconds(25, [90_000]).ok_or("Cat estimate")?;
        assert!(cat_seconds >= 2160);
        let available = [
            CurrencyHunt { race: cat, points: 10, seconds: cat_seconds },
            CurrencyHunt { race: feasible, points: 10, seconds: 1200 },
        ];
        assert_eq!(deadline_plan(0, 10, &available, 1800), Some(vec![feasible]));
        assert_eq!(deadline_plan(0, 20, &available, 1800), None);
        Ok(())
    }
}

#[cfg(test)]
mod eligibility_review {
    use super::*;

    #[test]
    fn zero_progress_thais_prepares_the_rat_prefix_without_completing_it() {
        assert!(hunt_seconds(249, [90_000; 8]).is_some_and(|seconds| seconds > DEADLINE.as_secs() as u32));
        assert_eq!(completion_fixture_prefix(1, 250, false), 248);
        assert_eq!(completion_fixture_prefix(1, 250, true), 0);
    }

}

#[cfg(test)]
mod final_review {
    use super::*;
    use oteryn_protocol_oteryn::logout::LogoutResult;
    #[tokio::test]
    async fn fixture_logout_waits_through_fight_then_confirms_release() -> TestResult {
        let fight = LogoutResult::InFight { retry_after_ms: 2000 };
        let mut replies = [fight, LogoutResult::Accepted].into_iter();
        let mut waits = Vec::new();
        fixture_logout(&mut replies, async |replies| Ok(replies.next()),
            async |_, delay| { waits.push(delay); Ok(()) }).await?;
        assert_eq!(replies.next(), None);
        assert_eq!(waits, [Duration::from_secs(2)]);
        Ok(())
    }
}
