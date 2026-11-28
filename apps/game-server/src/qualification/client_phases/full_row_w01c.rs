//! W01c full-row journey: admitted creature attacks.
//!
//! Concrete player scenario derived from the W01c matrix/audit rows and the native melee owner:
//! 1. Admit the provisioned node-boot character and walk, using ordinary STEP commands and their
//!    `Moved` replies plus spatial deltas, to an authored `oteryn:creature.poison_spider` spawn.
//! 2. Send the ordinary attack-cancel command so the character does not kill the attacker; require
//!    its accepted reply, then require a native ActorVitals delta that reduces nonzero health. This
//!    proves the admitted hostile creature selected and committed a real attack against the player.
//! 3. Send LOGOUT while the attack's fight lock is active and require the server's `InFight`
//!    refusal, with the character and damaged vitals still present.
//! 4. Escape by ordinary STEP commands, then require the first LOGOUT to be `Accepted`; freshly
//!    admit the same character and require a new game session plus the exact server-saved position,
//!    current/max health, mana and soul observed immediately before logout.
//! 5. Return to the poison spider, observe lethal damage and the granted respawn,
//!    and verify the new durable death receipt and respawn state after re-admission.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityRef, SessionEvent};
use oteryn_protocol_oteryn::analyser::{
    AnalyserDamageSource, AnalyserFact, CAPABILITY_ANALYSER_HIT_ATTRIBUTION_V1,
    CAPABILITY_ANALYSER_V1,
};
use oteryn_protocol_oteryn::damage_element::DamageElement;
use oteryn_protocol_oteryn::logout::LogoutResult;
use std::collections::BTreeSet;

pub(super) const ENCOUNTER: TilePos = TilePos {
    x: 32462,
    y: 32225,
    floor: -7,
};
const NEW_HAVEN_RESPAWN: TilePos = TilePos {
    x: 32536,
    y: 32514,
    floor: -7,
};

const LETHAL_ACTION: &str = "lethal-attack-and-respawn";

// The live town route takes about 150 seconds per traversal. Budget for all
// three walks, the World load and admission, and the existing combat and
// logout deadlines. The wire and no-progress deadlines still apply.
const JOURNEY_BUDGET: Duration = Duration::from_secs(900);

const ACTIONS: &[&str] = &[
    "admitted-and-walked-to-spawn",
    "attack-command-and-vitals-delta",
    "in-fight-logout-refused",
    "escaped-and-logout-accepted",
    "fresh-admission-retains-state",
    LETHAL_ACTION,
];

struct Row {
    passed: BTreeSet<&'static str>,
    invalid: bool,
    started: std::time::Instant,
}

impl Row {
    fn new() -> Self {
        Self {
            passed: BTreeSet::new(),
            invalid: false,
            started: std::time::Instant::now(),
        }
    }

    fn pass(&mut self, action: &'static str, observed: impl std::fmt::Debug) {
        if !ACTIONS.contains(&action) || !self.passed.insert(action) {
            self.invalid = true;
            return;
        }
        evidence(&format!("W01C_CHECK {action} PASS observed={observed:?}"));
    }

    fn is_complete(&self) -> bool {
        !self.invalid && ACTIONS.iter().all(|action| self.passed.contains(action))
    }

    fn finish(self, result: TestResult) {
        for action in ACTIONS {
            if !self.passed.contains(action) {
                evidence(&format!("W01C_CHECK {action} FAIL assertion-not-completed"));
            }
        }
        if let Err(error) = &result {
            evidence(&format!(
                "W01C_GAP step={} observed={error}",
                self.passed.len() + 1
            ));
        }
        let pass = result.is_ok() && self.is_complete();
        evidence(&format!(
            "e2e_all scenario=W01c.full status={} seconds={:.3}{}",
            if pass { "PASS" } else { "FAIL" },
            self.started.elapsed().as_secs_f64(),
            if pass {
                ""
            } else {
                " evidence=runtime-observation"
            },
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result =
        verdict::indexed_timeout(JOURNEY_BUDGET, verdict::observe(journey(&mut row, clients)))
            .await;
    row.finish(match result {
        Ok(result) => result,
        Err(_) => Err(format!("W01c full journey exceeded {JOURNEY_BUDGET:?}").into()),
    });
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    if !metadata.spawns.sources.iter().any(|source| {
        source.floor == ENCOUNTER.floor
            && source.points.iter().any(|point| {
                point
                    .x
                    .abs_diff(ENCOUNTER.x)
                    .max(point.y.abs_diff(ENCOUNTER.y))
                    == 1
                    && metadata
                        .spawns
                        .creatures
                        .get(point.creature as usize)
                        .is_some_and(|creature| creature.key == "oteryn:creature.poison_spider")
            })
    }) {
        return Err("step 1: fixed encounter has no adjacent authored Poison Spider spawn".into());
    }
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0xc1).await?;
    let first_session = client.join_snapshot().game_session_id;
    if world
        .base()
        .tile(ENCOUNTER.x, ENCOUNTER.y, ENCOUNTER.floor)
        .is_none_or(|tile| tile.protection_zone())
    {
        return Err("step 1: fixed encounter is missing or protected".into());
    }
    if tile(client.world_spatial().actor_position)? != TOWN {
        return Err("step 1: character did not enter at the configured town".into());
    }
    walk_to(&mut client, world, ENCOUNTER).await?;
    let spiders = adjacent_spiders(&client)?;
    row.pass(
        "admitted-and-walked-to-spawn",
        client.world_spatial().actor_position,
    );

    let before = *client
        .actor_vitals()
        .ok_or("step 2: admission supplied no ActorVitals snapshot")?;
    let cancel = client.session_mut().attack_target(None).await?;
    if cancel.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(format!("step 2: attack-cancel command refused: {cancel:?}").into());
    }
    if !client
        .session_mut()
        .combat_state()
        .is_some_and(|state| state.target.is_none())
    {
        return Err("step 2: attack cancel did not leave an empty target".into());
    }
    let (damaged, attacker, amount) = wait_for_damage(&mut client, before.health, &spiders).await?;
    row.pass(
        "attack-command-and-vitals-delta",
        (cancel, before.health, damaged, attacker, amount),
    );

    let refusal_position = client.world_spatial().actor_position;
    let refusal_vitals = client
        .actor_vitals()
        .copied()
        .ok_or("step 3: vitals absent")?;
    let refusal = client.session_mut().logout().await?;
    if !matches!(refusal.result, Some(LogoutResult::InFight { .. })) {
        return Err(
            format!("step 3: expected InFight logout refusal, observed {refusal:?}").into(),
        );
    }
    if refusal_vitals.health == 0
        || refusal_vitals.health >= before.health
        || client.actor_vitals() != Some(&refusal_vitals)
        || client.world_spatial().actor_position != refusal_position
        || client.join_snapshot().game_session_id != first_session
    {
        return Err("step 3: InFight changed the actor's position, vitals or session".into());
    }
    row.pass("in-fight-logout-refused", refusal);

    walk_to(&mut client, world, TOWN).await?;
    let health_before_logout = client
        .actor_vitals()
        .ok_or("step 4: pre-logout vitals absent")?
        .health;
    let saved_position = client.world_spatial().actor_position;
    // The session can apply pushed vitals before it reads the Accepted result.
    // Sample the last state on that same wire at the acceptance boundary.
    logout_once(&mut client).await?;
    let saved = *client.actor_vitals().ok_or("step 4: vitals absent")?;
    if saved.health == 0 || saved.health >= before.health {
        return Err(
            format!("step 4: expected saved damage from {before:?}, observed {saved:?}").into(),
        );
    }
    if client.world_spatial().actor_position != saved_position {
        return Err("step 4: town position changed while logging out".into());
    }
    row.pass(
        "escaped-and-logout-accepted",
        (
            saved_position,
            health_before_logout,
            saved,
            client.last_server_sequence(),
        ),
    );
    drop(client);

    let mut client = connect_world(clients, 0xc2).await?;
    let observed = *client
        .actor_vitals()
        .ok_or("step 5: fresh admission supplied no ActorVitals snapshot")?;
    if client.join_snapshot().game_session_id == first_session
        || client.world_spatial().actor_position != saved_position
        || observed != saved
    {
        return Err(format!(
            "step 5: fresh admission state mismatch: session={} position={:?} vitals={observed:?}; expected position={saved_position:?} vitals={saved:?}",
            client.join_snapshot().game_session_id == first_session,
            client.world_spatial().actor_position,
        )
        .into());
    }
    row.pass("fresh-admission-retains-state", (saved_position, observed));
    verdict::Harness::global().progress(LETHAL_ACTION);
    if !client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1)
    {
        return Err("step=lethal-attack-and-respawn observed=no-world-map-capability source=apps/game-server/src/qualification/client_phases/death.rs:7".into());
    }
    let second_session = client.join_snapshot().game_session_id;
    walk_to(&mut client, world, ENCOUNTER).await?;
    adjacent_spiders(&client)?;
    let encounter_vitals = *client
        .actor_vitals()
        .ok_or("step 6: pre-death vitals absent")?;
    if encounter_vitals.health >= encounter_vitals.max_health {
        return Err("step 6: pre-death vitals did not retain damage".into());
    }
    verdict::Harness::global().bounded_wait(
        tokio::time::Instant::now() + Duration::from_secs(180),
        LETHAL_ACTION,
    );
    client_death::death(&mut client, clients).await?;
    let respawn_position = client.world_spatial().actor_position;
    let respawn_vitals = *client
        .actor_vitals()
        .ok_or("step 6: respawn vitals absent")?;
    if tile(respawn_position)? != NEW_HAVEN_RESPAWN
        || respawn_vitals.health != encounter_vitals.max_health
        || respawn_vitals.mana != encounter_vitals.max_mana
    {
        return Err(format!(
            "step 6: wrong respawn position or vitals: {respawn_position:?} {respawn_vitals:?}"
        )
        .into());
    }

    logout_once(&mut client).await?;
    drop(client);
    let mut client = connect_world(clients, 0xc3).await?;
    let third_session = client.join_snapshot().game_session_id;
    if third_session == first_session
        || third_session == second_session
        || client.world_spatial().actor_position != respawn_position
        || client.actor_vitals() != Some(&respawn_vitals)
    {
        return Err(format!(
            "step 6: fresh admission did not retain respawn: {:?} {:?}",
            client.world_spatial().actor_position,
            client.actor_vitals()
        )
        .into());
    }
    logout_once(&mut client).await?;
    row.pass(LETHAL_ACTION, (respawn_position, respawn_vitals));
    Ok(())
}

async fn wait_for_damage(
    client: &mut DevClientSession,
    before: u32,
    spiders: &[EntityRef],
) -> TestResult<(
    oteryn_protocol_oteryn::actor_spell::ActorVitals,
    EntityRef,
    u32,
)> {
    let capabilities = client.selected_capabilities();
    if !capabilities.contains(&CAPABILITY_ANALYSER_V1)
        || !capabilities.contains(&CAPABILITY_ANALYSER_HIT_ATTRIBUTION_V1)
    {
        return Err("step 2: attributed damage evidence was not negotiated".into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(45);
    let mut damaged = None;
    let mut hit = None;
    loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        for event in client.take_events() {
            match event {
                SessionEvent::ActorVitals(delta)
                    if damaged.is_none()
                        && delta.value.health > 0
                        && delta.value.health < before =>
                {
                    damaged = Some(delta.value);
                }
                SessionEvent::AnalyserAttributedFacts(delta) if hit.is_none() => {
                    for fact in delta.value {
                        let AnalyserFact::DamageInput {
                            value,
                            element,
                            source,
                        } = &fact.fact
                        else {
                            continue;
                        };
                        let attacker = fact.attacker.filter(|attacker| spiders.contains(attacker));
                        if *element != DamageElement::Physical
                            || !matches!(source, AnalyserDamageSource::Creature(_))
                            || attacker.is_none()
                        {
                            return Err(format!(
                                "step 2: damage did not come from adjacent Poison Spider: {fact:?}"
                            )
                            .into());
                        }
                        hit = attacker.map(|attacker| (attacker, value.get()));
                        break;
                    }
                }
                _ => {}
            }
        }
        if let (Some(damaged), Some((attacker, amount))) = (damaged, hit) {
            if before.checked_sub(amount) != Some(damaged.health) {
                return Err(format!(
                    "step 2: attributed damage {amount} does not explain HP {before} -> {}",
                    damaged.health
                )
                .into());
            }
            return Ok((damaged, attacker, amount));
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "step 2: no matching Poison Spider damage and ActorVitals in 45s: before={before} vitals={damaged:?} hit={hit:?}"
            )
            .into());
        }
    }
}

async fn logout_once(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(LogoutResult::Accepted) {
        return Err(format!("W01c first logout refused: {reply:?}").into());
    }
    Ok(())
}

pub(super) fn adjacent_spiders(client: &DevClientSession) -> TestResult<Vec<EntityRef>> {
    let here = client.world_spatial().actor_position;
    let spiders = client
        .world_entities()
        .ok_or("W01c encounter: entities absent")?
        .iter()
        .filter(|entity| {
            entity.kind == oteryn_dev_client::EntityKind::Creature
                && entity.position.floor == here.floor
                && here
                    .x
                    .abs_diff(entity.position.x)
                    .max(here.y.abs_diff(entity.position.y))
                    <= 1
        })
        .map(|entity| {
            if entity
                .display_name
                .is_some_and(|name| name.as_str() == Ok("Poison Spider"))
            {
                Ok(entity.entity)
            } else {
                Err(format!("W01c encounter: unexpected adjacent creature {entity:?}").into())
            }
        })
        .collect::<TestResult<Vec<_>>>()?;
    if spiders.is_empty() {
        return Err("W01c encounter: no adjacent Poison Spider".into());
    }
    Ok(spiders)
}

fn step_landed(
    disposition: StepDisposition,
    has_delta: bool,
    previous: TilePos,
    expected: TilePos,
    observed: TilePos,
) -> bool {
    disposition == StepDisposition::Moved && has_delta && observed == expected && observed != previous
}

pub(super) async fn walk_to(client: &mut DevClientSession, world: &BundleWorld, goal: TilePos) -> TestResult {
    let mut position = tile(client.world_spatial().actor_position)?;
    let route = route(world, position, |at| at == goal)
        .filter(|steps| !steps.is_empty())
        .ok_or_else(|| format!("W01c no nonempty route from {position:?} to {goal:?}"))?;
    for direction in route {
        let expected = destination(world, position, direction)
            .ok_or_else(|| format!("W01c step {direction:?} has no landing from {position:?}"))?;
        let outcome = client.step(direction).await?;
        let observed = tile(client.world_spatial().actor_position)?;
        if !step_landed(outcome.disposition, outcome.world_spatial_delta.is_some(), position, expected, observed) {
            return Err(format!(
                "W01c first step {direction:?} from {position:?}: expected {expected:?}, observed {observed:?} {outcome:?}"
            ).into());
        }
        position = observed;
        verdict::Harness::global().progress("walk");
        client
            .service_liveness(Duration::from_millis(u64::from(outcome.step_wait_ms)))
            .await?;
        client.take_events();
    }
    if position != goal {
        return Err(format!("W01c walk ended at {position:?}, expected {goal:?}").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movement_requires_first_attempt_and_exact_landing() {
        for refused in [StepDisposition::TooEarly, StepDisposition::Rejected, StepDisposition::Blocked] {
            assert!(!step_landed(refused, true, TOWN, ENCOUNTER, ENCOUNTER));
        }
        assert!(!step_landed(StepDisposition::Moved, false, TOWN, ENCOUNTER, ENCOUNTER));
        assert!(!step_landed(StepDisposition::Moved, true, TOWN, ENCOUNTER, TOWN));
        assert!(step_landed(StepDisposition::Moved, true, TOWN, ENCOUNTER, ENCOUNTER));
    }

    #[test]
    fn surviving_an_attack_does_not_qualify_lethal_attack_and_respawn() {
        let mut row = Row::new();
        for action in ACTIONS
            .iter()
            .copied()
            .filter(|action| *action != LETHAL_ACTION)
        {
            row.pass(action, ());
        }
        assert!(!row.is_complete());
        row.pass(LETHAL_ACTION, ());
        assert!(row.is_complete());
    }

    #[test]
    fn row_requires_every_lifecycle_action_once() {
        let mut row = Row::new();
        for action in ACTIONS {
            row.pass(action, ());
        }
        assert!(ACTIONS.iter().all(|action| row.passed.contains(action)));
        row.pass(ACTIONS[0], ());
        assert!(row.invalid);
    }
}
