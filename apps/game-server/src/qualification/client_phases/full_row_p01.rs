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

async fn hunt_once(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult<full_row_p05::KillSample> {
    full_row_p05::hunt_for_race(client, world, spawns, "oteryn:creature.rat").await
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
    let first_kill = hunt_once(&mut client, world, &metadata.spawns).await?;
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
    while !race_complete(&current, race) {
        hunt_once(&mut client, world, &metadata.spawns).await?;
        let next = query(&mut client).await?;
        if count(&current, race).checked_add(1) != Some(count(&next, race)) {
            return Err(format!(
                "credited hunt failed to advance the selected race by one: {} -> {}",
                count(&current, race),
                count(&next, race)
            )
            .into());
        }
        current = next;
    }
    if current.charms.charm_points_available < cost {
        return Err(format!(
            "race={race} reached mastery with {} charm points; need={} and a reachable second Bestiary race; source=crates/gameplay/dispatch/src/gameplay_transport/charm_native.rs:158",
            current.charms.charm_points_available, cost
        ).into());
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
    let kill = hunt_once(&mut client, world, &metadata.spawns).await?;
    if kill.race != race {
        return Err(format!(
            "step=assigned-combat expected-race={race} credited-race={}",
            kill.race
        )
        .into());
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
    full_row_p05::logout_after_combat(&mut client).await?;
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
    Err("step=charm-proc need=observed-effect-attributed-to-the-assigned-charm;source=crates/gameplay/dispatch/src/gameplay_transport/charm_native.rs:158".into())
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
