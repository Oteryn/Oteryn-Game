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
//!    kill while the assignment is active, proving the production combat hook
//!    is traversed without losing the assignment.
//! 7. Send LOGOUT, obtain a fresh admission, then send QUERY; expect the kill
//!    counts, charm-point balance, unlocked stage, and assigned race to equal
//!    the pre-logout durable view.

use super::*;
use oteryn_dev_client::{DevClientSession, EntityDetail, EntityKind, SessionEvent};
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

async fn kill_adjacent(client: &mut DevClientSession) -> TestResult {
    use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
    let before = query(client).await?;
    let here = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == EntityKind::Creature
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
    let mut damaged = false;
    loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        for event in client.take_events() {
            if let SessionEvent::WorldSpatialEntities(delta) = event {
                damaged |= delta.value.update.iter().any(|entry| {
                    entry.entity == target.entity
                        && matches!(
                            entry.detail,
                            EntityDetail::Actor { health_percent, .. } if health_percent < 100
                        )
                });
            }
        }
        if client
            .world_entities()
            .is_some_and(|entities| entities.get(&target.entity).is_none())
        {
            client.session_mut().attack_target(None).await?;
            let after = query(client).await?;
            let before_total = before
                .bestiary
                .iter()
                .try_fold(0_u32, |total, entry| total.checked_add(entry.kill_count))
                .ok_or("Bestiary kill total overflow before ATTACK")?;
            let after_total = after
                .bestiary
                .iter()
                .try_fold(0_u32, |total, entry| total.checked_add(entry.kill_count))
                .ok_or("Bestiary kill total overflow after ATTACK")?;
            if before_total
                .checked_add(1)
                .ok_or("Bestiary kill total overflow")?
                != after_total
                || before
                    .bestiary
                    .iter()
                    .any(|entry| count(&after, entry.race) < entry.kill_count)
            {
                return Err(format!(
                    "target vanished without exactly one credited Bestiary kill: before={:?} after={:?}",
                    before.bestiary, after.bestiary
                )
                .into());
            }
            evidence(&format!(
                "P01_CHECK combat-kill PASS target={:?} damaged={damaged} bestiary_delta=1",
                target.entity
            ));
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            client.session_mut().attack_target(None).await?;
            return Err(format!(
                "ATTACK timeout target={:?} damaged={damaged}",
                target.entity
            )
            .into());
        }
    }
}

async fn hunt_once(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult {
    travel_to_spawn(client, world, spawns, "oteryn:creature.rat").await?;
    kill_adjacent(client).await
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
    hunt_once(&mut client, world, &metadata.spawns).await?;
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
    if count(&current, race) != count(&initial, race) + 1 {
        return Err("credited kill did not increment Bestiary by exactly one".into());
    }
    evidence(&format!(
        "P01_CHECK kill-count PASS race={} count={}",
        race,
        count(&current, race)
    ));

    let (charm, stage, cost) = current
        .charms
        .charms
        .iter()
        .enumerate()
        .filter(|(_, charm)| charm.unlocked_stage == 0 && charm.next_stage_cost > 0)
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
    {
        return Err(format!("locked ASSIGN reply/delta mismatch: {refused:?}").into());
    }
    evidence("P01_CHECK locked-assignment-refusal PASS disposition=CharmLocked");

    row.at("earn-unlock-currency");
    while current.charms.charm_points_available < cost || !race_complete(&current, race) {
        hunt_once(&mut client, world, &metadata.spawns).await?;
        let next = query(&mut client).await?;
        if count(&next, race) <= count(&current, race) {
            return Err(format!(
                "credited hunt produced no increasing QUERY delta: {} -> {}",
                count(&current, race),
                count(&next, race)
            )
            .into());
        }
        current = next;
    }
    if !race_complete(&current, race) {
        return Err("hunted race did not reach its final threshold".into());
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
        || unlocked.charms.charm_points_available + cost != current.charms.charm_points_available
    {
        return Err(format!("UNLOCK reply/delta mismatch: {unlocked:?}").into());
    }
    evidence("P01_CHECK unlock-stage PASS stage-and-balance-delta");

    row.at("assign-unlocked");
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
    hunt_once(&mut client, world, &metadata.spawns).await?;
    let after_hook = query(&mut client).await?;
    let after_state = &after_hook.charms.charms[usize::try_from(charm.get() - 1)?];
    if count(&after_hook, race) != before_hook.checked_add(1).ok_or("kill count overflow")?
        || after_state.assigned_race != Some(race)
        || !after_state.effect_active
    {
        return Err(format!("assigned combat hook lost durable state: {after_hook:?}").into());
    }
    evidence("P01_CHECK assigned-combat-hook PASS native-attack-retained-active-assignment");

    row.at("logout-fresh-admission");
    let old_session = client.join_snapshot().game_session_id;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("LOGOUT refused after charm combat".into());
    }
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
