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
//! 6. Walk back to a live creature, send ATTACK, and require a second real
//!    kill while the assignment is active, proving the production combat hook
//!    is traversed without losing the assignment.
//! 7. Send LOGOUT, obtain a fresh admission, then send QUERY; expect the kill
//!    counts, charm-point balance, unlocked stage, and assigned race to equal
//!    the pre-logout durable view.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::charm::{CharmAssignDisposition, CharmUnlockDisposition};
use oteryn_protocol_oteryn::charm_progression::{
    self as wire, CharmProgressionAction, CharmProgressionOutcome, CharmProgressionReply,
};
use std::collections::BTreeSet;
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
        match &result {
            Ok(()) => evidence("P01_GAP status=PASS"),
            Err(error) => evidence(&format!(
                "P01_GAP status=FAIL step={} observed={}",
                self.step,
                compact(&error.to_string())
            )),
        }
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
    let result =
        verdict::indexed_timeout(DEADLINE, verdict::observe(journey(&mut row, clients))).await;
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
    require_mechanic_acceptance(response.status, &format!("CHARM_PROGRESSION {action:?}"))?;
    wire::decode_charm_progression_reply(&response.payload)
        .map_err(|error| format!("decode {action:?}: {error:?}").into())
}

pub(super) async fn logout_when_ready(client: &mut DevClientSession) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    verdict::Harness::global().bounded_wait(deadline, "logout-combat-lock");
    while client
        .session_mut()
        .combat_state()
        .is_none_or(|state| state.in_fight)
    {
        if tokio::time::Instant::now() >= deadline {
            return Err("combat state did not clear before logout".into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
    }
    let (result, detail, _, _) = logout_once(client).await?;
    if result != Some(LogoutResult::Accepted) {
        return Err(format!("logout refused: {detail}").into());
    }
    Ok(())
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

fn mastery_plan(
    current: &CharmProgressionReply,
    reachable: impl Fn(&str) -> bool,
) -> TestResult<Vec<(NonZeroU32, &'static str, u32, u32)>> {
    let mut plan = Vec::new();
    for (index, (definition, points)) in full_row_p05::canonical_races()?.iter().enumerate() {
        let race = NonZeroU32::new(u32::try_from(index)? + 1).ok_or("zero race reference")?;
        let remaining = definition
            .final_kill_threshold()
            .saturating_sub(count(current, race));
        if remaining > 0 && *points > 0 && reachable(definition.key()) {
            plan.push((race, definition.key(), remaining, *points));
        }
    }
    plan.sort_by(|left, right| {
        (u64::from(left.2) * u64::from(right.3)).cmp(&(u64::from(right.2) * u64::from(left.3)))
    });
    Ok(plan)
}

fn reachable_mastery_races<'a>(
    world: &BundleWorld,
    from: TilePos,
    spawns: &'a oteryn_world_bundle::spawn::Table,
) -> BTreeSet<&'a str> {
    let reachable = std::cell::RefCell::new(BTreeSet::new());
    // Use the route planner's normal landings and visit the whole reachable area once.
    let _ = route(world, from, |at| {
        if world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone())
        {
            reachable.borrow_mut().insert(at);
        }
        false
    });
    let reachable = reachable.into_inner();
    spawns
        .sources
        .iter()
        .flat_map(|source| {
            source.points.iter().filter_map(|point| {
                let approachable = (-1_i16..=1).any(|dx| {
                    (-1_i16..=1).any(|dy| {
                        (dx != 0 || dy != 0)
                            && point
                                .x
                                .checked_add_signed(dx)
                                .zip(point.y.checked_add_signed(dy))
                                .is_some_and(|(x, y)| {
                                    reachable.contains(&TilePos {
                                        x,
                                        y,
                                        floor: source.floor,
                                    })
                                })
                    })
                });
                approachable
                    .then(|| spawns.creatures.get(point.creature as usize))
                    .flatten()
                    .map(|entry| entry.key.as_str())
            })
        })
        .collect()
}

pub(super) fn rat_corpse_stage() -> TestResult<client_hunt::CorpseStage> {
    let items = provisioning::item_index()?;
    Ok(client_hunt::CorpseStage {
        fresh_ref: provisioning::item_reference(&items, 5964)?.get(),
        mature_ref: provisioning::item_reference(&items, 3994)?.get(),
    })
}

async fn hunt_race(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    key: &str,
    rat_stage: client_hunt::CorpseStage,
) -> TestResult<full_row_p05::KillSample> {
    let stage = (key == "oteryn:creature.rat").then_some(rat_stage);
    full_row_p05::hunt_for_race_with_corpse_stage(client, world, spawns, key, stage).await
}

pub(super) async fn prepare_hunter(
    clients: &SeamClients<'_>,
    scenario: &str,
    tag: u8,
) -> TestResult<DevClientSession> {
    use oteryn_protocol_oteryn::attack::{
        AttackIntentDisposition, ChaseMode, FightMode, FightModes,
    };
    use oteryn_protocol_oteryn::cyclopedia_character as stats;

    let reason = "ordinary leveling does not fit the timed Bestiary hunts";
    character_prep::declare_seeded(scenario, "level-80-sorcerer", reason);
    let mut client = character_prep::Seeded {
        profile: "p01-sorcerer",
        reason,
    }
    .connect(clients, tag, 1)
    .await?;
    let reply = client
        .session_mut()
        .gated_command(
            stats::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            stats::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &[stats::CharacterTab::GeneralStats as u8, 0, 0, 1],
        )
        .await?;
    if reply.status != CommandStatus::Accepted
        || general_stats::validate(&reply.payload, general_stats::selected(&client))?
            != stats::vocation_id("sorcerer")
        || reply.payload.first() != Some(&(stats::CharacterTab::GeneralStats as u8))
    {
        return Err("P01 requires the admitted Sorcerer GeneralStats view".into());
    }
    let xp = u64::from_le_bytes(reply.payload[1..9].try_into()?);
    let level = u32::from_le_bytes(reply.payload[9..13].try_into()?);
    if (xp, level) != (7_915_800, 80) {
        return Err(format!("P01 seeded Sorcerer stats xp={xp} level={level}").into());
    }
    let vitals = client
        .actor_vitals()
        .ok_or("P01 seeded Sorcerer vitals absent")?;
    if (vitals.max_health, vitals.max_mana) != (545, 2250) {
        return Err(format!("P01 seeded Sorcerer max vitals {vitals:?}").into());
    }
    evidence(&format!(
        "P01_CHECK hunter-profile PASS vocation=sorcerer level={level} xp={xp} max_health=545 max_mana=2250"
    ));
    let modes = FightModes {
        fight_mode: FightMode::Offensive,
        chase: ChaseMode::Chase,
        secure: true,
    };
    if client
        .session_mut()
        .combat_state()
        .is_none_or(|state| state.modes == modes)
    {
        return Err("P01 requires a distinct initial fight-mode state".into());
    }
    let reply = client.session_mut().set_fight_modes(&modes).await?;
    require_mechanic_acceptance(reply.status, "P01 offensive chase")?;
    if reply.disposition != AttackIntentDisposition::Ok {
        return Err(format!("P01 offensive chase refused: {reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while client
        .session_mut()
        .combat_state()
        .is_none_or(|state| state.modes != modes)
    {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "P01 accepted offensive chase but observed combat state {:?}",
                client.session_mut().combat_state()
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    evidence("P01_CHECK hunt-modes PASS offensive-chase");
    Ok(client)
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
    let mut client = prepare_hunter(clients, "P01.full", 0xa2).await?;
    if !client
        .selected_capabilities()
        .contains(&wire::CAPABILITY_BESTIARY_CHARM_PROGRESSION_V1)
    {
        return Err("admission omitted capability 67".into());
    }
    let initial = query(&mut client).await?;
    evidence("P01_CHECK admission-query PASS accepted-native-views");

    row.at("player-earned-kill-count");
    let rat_stage = rat_corpse_stage()?;
    let first_kill = hunt_race(
        &mut client,
        world,
        &metadata.spawns,
        "oteryn:creature.rat",
        rat_stage,
    )
    .await?;
    let mut current = query(&mut client).await?;
    let race = current
        .bestiary
        .iter()
        .find(|entry| count(&initial, entry.race) < entry.kill_count)
        .map(|entry| entry.race)
        .ok_or_else(|| {
            format!(
                "QUERY after credited ATTACK has no Bestiary delta; before={:?} after={:?}",
                initial.bestiary, current.bestiary
            )
        })?;
    if first_kill.race != race
        || count(&initial, race).checked_add(1) != Some(count(&current, race))
    {
        return Err("credited kill did not increment Bestiary by exactly one".into());
    }
    evidence(&format!(
        "P01_CHECK kill-count PASS race={} count={}",
        race,
        count(&current, race)
    ));

    let catalogue = crate::content::charm_source::CanonicalCharmCatalogue::embedded()?;
    let fire_charm = catalogue
        .catalogue()
        .definitions()
        .position(|entry| entry.key.as_str() == "oteryn:charm.enflame")
        .ok_or("step=charm-proc canonical Enflame missing")?;
    let (charm, stage, cost) = current
        .charms
        .charms
        .iter()
        .enumerate()
        .filter(|(index, charm)| {
            *index == fire_charm && charm.unlocked_stage == 0 && charm.next_stage_cost > 0
        })
        .min_by_key(|(_, charm)| charm.next_stage_cost)
        .and_then(|(index, charm)| {
            Some((
                NonZeroU32::new(u32::try_from(index).ok()?.checked_add(1)?)?,
                charm.unlocked_stage,
                charm.next_stage_cost,
            ))
        })
        .ok_or("query has no locked charm with a stage cost")?;

    row.at("locked-assignment-refusal");
    let refused = command(&mut client, CharmProgressionAction::Assign { charm, race }).await?;
    if refused.outcome != CharmProgressionOutcome::Assign(CharmAssignDisposition::CharmLocked)
        || refused.charms != current.charms
        || refused.bestiary != current.bestiary
    {
        return Err(format!("locked ASSIGN reply/delta mismatch: {refused:?}").into());
    }
    evidence("P01_CHECK locked-assignment-refusal PASS disposition=CharmLocked");

    row.at("earn-unlock-currency");
    let from = tile(client.world_spatial().actor_position)?;
    let progress = current.clone();
    let spawns = metadata.spawns.clone();
    let planning = tokio::task::spawn_blocking(move || {
        let reachable = reachable_mastery_races(world, from, &spawns);
        mastery_plan(&progress, |key| reachable.contains(key)).map_err(|error| error.to_string())
    });
    while !planning.is_finished() {
        client.service_liveness(Duration::from_millis(100)).await?;
    }
    let plan = planning.await??;
    let reachable_points = plan.iter().map(|entry| u64::from(entry.3)).sum::<u64>();
    if u64::from(current.charms.charm_points_available) + reachable_points < u64::from(cost) {
        return Err(format!("step=earn-unlock-currency need={cost} balance={} reachable-mastery-points={reachable_points}", current.charms.charm_points_available).into());
    }
    let mut mastered = None;
    for (candidate, key, remaining, _) in plan {
        for _ in 0..remaining {
            let kill = hunt_race(&mut client, world, &metadata.spawns, key, rat_stage).await?;
            let next = query(&mut client).await?;
            if kill.race != candidate
                || count(&current, candidate).checked_add(1) != Some(count(&next, candidate))
            {
                return Err(format!(
                    "step=earn-unlock-currency incorrect credited delta for {key}"
                )
                .into());
            }
            current = next;
        }
        mastered = Some((candidate, key));
        if current.charms.charm_points_available >= cost {
            break;
        }
    }
    let (race, assigned_key) = mastered.ok_or("step=assign need=player-mastered-reachable-race")?;

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
        || unlocked.charms.charm_points_available + cost != current.charms.charm_points_available
    {
        return Err(format!("UNLOCK reply/delta mismatch: {unlocked:?}").into());
    }
    evidence("P01_CHECK unlock-stage PASS stage-and-balance-delta");

    row.at("assign-unlocked");
    if unlocked_state.assigned_race.is_some() || unlocked_state.effect_active {
        return Err(format!("UNLOCK unexpectedly assigned the charm: {unlocked_state:?}").into());
    }
    let assigned = command(&mut client, CharmProgressionAction::Assign { charm, race }).await?;
    let assigned_state = &assigned.charms.charms[usize::try_from(charm.get() - 1)?];
    if assigned.outcome != CharmProgressionOutcome::Assign(CharmAssignDisposition::Assigned)
        || assigned_state.assigned_race != Some(race)
        || !assigned_state.effect_active
    {
        return Err(format!("ASSIGN reply/delta mismatch: {assigned:?}").into());
    }
    evidence("P01_CHECK assign-unlocked PASS assignment-and-active-hook");

    row.at("assigned-combat-hook");
    let before_hook = count(&assigned, race);
    if client.inventory().is_none_or(|inventory| {
        inventory.equipment.iter().any(|item| {
            matches!(
                item.slot,
                oteryn_protocol_oteryn::item_view::EquipmentSlot::LeftHand
                    | oteryn_protocol_oteryn::item_view::EquipmentSlot::RightHand
            )
        })
    }) {
        return Err("step=charm-proc need=unarmed-physical-baseline-to-isolate-Enflame".into());
    }
    let mut observed_proc = false;
    for _ in 0..12 {
        let kill = hunt_race(
            &mut client,
            world,
            &metadata.spawns,
            assigned_key,
            rat_stage,
        )
        .await?;
        if kill.race != race {
            return Err(format!(
                "step=assigned-combat expected-race={race} credited-race={}",
                kill.race
            )
            .into());
        }
        if kill.fire_damage {
            observed_proc = true;
            break;
        }
    }
    let after_hook = query(&mut client).await?;
    let after_state = &after_hook.charms.charms[usize::try_from(charm.get() - 1)?];
    if count(&after_hook, race) != before_hook
        || after_state.assigned_race != Some(race)
        || !after_state.effect_active
    {
        return Err(format!("assigned combat hook lost durable state: {after_hook:?}").into());
    }
    evidence("P01_CHECK assigned-race-kill PASS analyser-credit-and-active-assignment");

    row.at("logout-fresh-admission");
    let old_session = client.join_snapshot().game_session_id;
    logout_when_ready(&mut client).await?;
    drop(client);
    let mut fresh = connect_world(clients, 0xa3).await?;
    if fresh.join_snapshot().game_session_id == old_session {
        return Err("fresh admission reused the old game session".into());
    }
    let durable = query(&mut fresh).await?;
    if durable.bestiary != after_hook.bestiary || durable.charms != after_hook.charms {
        return Err(format!("fresh QUERY lost durable state: {durable:?}").into());
    }
    evidence("P01_CHECK logout-fresh-admission PASS exact-durable-views");
    if !observed_proc {
        return Err(
            "step=charm-proc no Enflame fire damage observed during bounded unarmed hunts".into(),
        );
    }
    logout_when_ready(&mut fresh).await
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
#[test]
fn row_01_publishes_a_gap_for_an_unproven_journey() {
    assert!(
        include_str!("full_row_p01.rs")
            .split_once("#[cfg(test)]")
            .is_some_and(|(production, _)| production.contains("P01_GAP status="))
    );
}

#[cfg(test)]
#[test]
fn review_p01_guards_the_journey() {
    assert!(
        include_str!("full_row_p01.rs")
            .split_once("#[cfg(test)]")
            .is_some_and(|(body, _)| body.contains("hunt_for_race"))
    );
}

#[cfg(test)]
#[test]
fn review_2_p01() {
    assert!(
        include_str!("full_row_p01.rs")
            .split_once("#[cfg(test)]")
            .is_some_and(|(body, _)| body.contains("mastery_plan"))
    );
}
