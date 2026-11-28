//! S15b — login protection: monster targeting protection after reconnect.
//!
//! Concrete player scenario, derived from the S15b matrix/audit row and its runtime owner:
//! 1. Admit the node-boot character, walk out of town protection and place a Rat with the
//!    qualification spawn fixture. After fresh-login protection expires, require a damage
//!    frame attributed to that Rat and an HP loss.
//! 2. Lose the real TLS connection and resume it with the native reconnect command. Require the
//!    same session/actor continuity supplied by the shared recovery helper.
//! 3. During the four-second reentry window, send SET_FIGHT_MODES, ATTACK_TARGET and
//!    ATTACK_TARGET(None). Require accepted mode/clear replies, the exact reentry refusal
//!    for a new attack, and no monster-attributed damage or HP loss. After expiry, require
//!    that the same target can be selected and cleared.
//! 4. After the window, require a new monster-attributed damage frame and HP loss. Then send
//!    LOGOUT while in fight and require the server's refusal; wait for the ordinary combat
//!    lock to expire and require an accepted logout on the first request.
//! 5. Freshly admit, return to the same Rat approach with STEP, and check that the
//!    fresh-login protection suppresses monster damage, then require damage after its
//!    expiry. This re-check proves that logout did not durably retain or extend the reconnect
//!    epoch; only the separately owned fresh-admission window applies.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityRef, SessionEvent};
use oteryn_protocol_oteryn::attack::{AttackIntentDisposition, ChaseMode, FightMode, FightModes};

const ROW: &str = "S15b";
const JOURNEY_TIMEOUT: Duration = Duration::from_secs(240);

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(JOURNEY_TIMEOUT, verdict::observe(journey(clients))).await;
    let result = match result {
        Ok(result) => result,
        Err(_) => Err(format!(
            "step=journey observed=timeout-{}s need=responsive monster AI and combat-lock expiry",
            JOURNEY_TIMEOUT.as_secs()
        )
        .into()),
    };
    if let Err(error) = &result {
        evidence(&format!("S15B_GAP {error}"));
    }
    evidence(&format!(
        "e2e_all scenario={ROW}.full status={} seconds={:.3}{}",
        if result.is_ok() { "PASS" } else { "FAIL" },
        started.elapsed().as_secs_f64(),
        result
            .err()
            .map(|error| format!(" evidence={}", compact(&error.to_string())))
            .unwrap_or_default(),
    ));
}

fn compact(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_whitespace() {
                '-'
            } else {
                character
            }
        })
        .collect()
}

const COMBAT_STATE_OBSERVATION: Duration = Duration::from_secs(2);
// The live two-second expiry probe ended at unchanged HP despite cardinal adjacency.
// Allow the Rat to retarget and complete three authored two-second swing intervals.
const EXPIRY_OBSERVATION: Duration = Duration::from_secs(6);
fn reconnect_quiet_window() -> Duration {
    Duration::from_secs(4)
}

fn fresh_quiet_budget(elapsed: Duration) -> Duration {
    Duration::from_micros(
        oteryn_gameplay_entities::carrier::runtime_actor_reentry::FRESH_LOGIN_PROTECTION_MICROS,
    )
    .saturating_sub(elapsed)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0x8b).await?;
    stow_equipment(&mut client).await?;
    let (monster, approach) = approach_rat(&mut client, world).await?;
    require_attribution(&client)?;
    client
        .service_liveness(fresh_quiet_budget(Duration::ZERO))
        .await?;
    require_damage(
        &mut client,
        monster,
        Duration::from_secs(20),
        "step=1 baseline targeting",
    )
    .await?;
    evidence("S15B_CHECK step=1 baseline-monster-target PASS");

    let expected = destination(
        world,
        tile(client.world_spatial().actor_position)?,
        StepDirection::West,
    )
    .map(actor_position);
    let (recovered, resume_sent, resume_joined) =
        client_disconnect_resume::recover_with_resume_bounds(client, clients, expected)
            .await
            .map_err(|error| {
                verdict::record_error(verdict::IndexedError::as_error(&error));
                verdict::context(
                    format!("step=2 reconnect observed={error} need=committed CompleteReconnect"),
                    error,
                )
            })?;
    client = recovered;
    if resume_sent.elapsed() >= reconnect_quiet_window() {
        return Err(
            "step=3 reentry need=observable-recovery-commit-window-before-protection-expires"
                .into(),
        );
    }
    require_attribution(&client)?;
    evidence("S15B_CHECK step=2 committed-reconnect PASS");

    let hp_before_commands = health(&client)?;
    player_commands(
        &mut client,
        monster,
        AttackIntentDisposition::ReentryProtected,
    )
    .await?;
    if health(&client)? < hp_before_commands {
        return Err("step=3 reentry need=no-monster-damage-during-protected-commands".into());
    }
    let remaining = reconnect_quiet_window().saturating_sub(resume_sent.elapsed());
    if remaining.is_zero() {
        return Err("step=3 reentry need=protected-observation-after-native-commands".into());
    }
    require_quiet(
        &mut client,
        monster,
        remaining,
        "step=3 four-second protection",
    )
    .await?;
    evidence("S15B_CHECK step=3 reentry-refusal-and-monster-suppression PASS");
    require_expiry_damage(
        &mut client,
        world,
        monster,
        resume_joined + reconnect_quiet_window() + EXPIRY_OBSERVATION,
        "step=4 reentry expiry",
    )
    .await?;

    player_commands(&mut client, monster, AttackIntentDisposition::Ok).await?;
    let refused = client.session_mut().logout().await?;
    if !matches!(
        refused.result,
        Some(oteryn_protocol_oteryn::logout::LogoutResult::InFight { .. })
    ) {
        return Err(format!(
            "step=4 combat logout refusal observed={refused:?} need=Refused while in-fight"
        )
        .into());
    }
    retreat_and_logout(&mut client, world).await?;
    evidence("S15B_CHECK step=4 expiry-damage-and-logout-refusal PASS");
    drop(client);

    let admission_started = tokio::time::Instant::now();
    let mut fresh = connect_world(clients, 0x8c).await.map_err(|error| {
        verdict::record_error(verdict::IndexedError::as_error(&error));
        verdict::context(format!("step=5 fresh admission observed={error}"), error)
    })?;
    let admitted_at = tokio::time::Instant::now();
    if !fresh
        .inventory()
        .is_some_and(|view| view.equipment.is_empty())
    {
        return Err("step=5 fresh-login need=unarmoured-fixture-retained-on-relog".into());
    }
    // The commit is between the request start and the joined-session receipt.
    // Demand silence only through the earliest possible expiry.
    if admitted_at.duration_since(admission_started) >= fresh_quiet_budget(Duration::ZERO) {
        return Err(
            "step=5 fresh-login need=observable-admission-commit-window-before-protection-expires"
                .into(),
        );
    }
    require_attribution(&fresh)?;
    walk_during_fresh_protection(&mut fresh, world, approach, admission_started).await?;
    let remaining = fresh_quiet_budget(admission_started.elapsed());
    if remaining.is_zero() {
        return Err(
            "step=5 fresh-login need=reachable-rat-within-admission-protection-window".into(),
        );
    }
    require_quiet(
        &mut fresh,
        monster,
        remaining,
        "step=5 fresh-login protection",
    )
    .await?;
    require_expiry_damage(
        &mut fresh,
        world,
        monster,
        admitted_at + fresh_quiet_budget(Duration::ZERO) + EXPIRY_OBSERVATION,
        "step=5 fresh-login expiry",
    )
    .await?;
    evidence("S15B_CHECK step=5 fresh-admission-window-not-reentry-epoch PASS");
    retreat_and_logout(&mut fresh, world).await
}

async fn stow_equipment(client: &mut DevClientSession) -> TestResult {
    use oteryn_dev_client::{ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome};

    // Remove mitigation as a possible explanation for an otherwise legal zero-damage swing.
    let equipment = client
        .inventory()
        .ok_or("step=setup need=equipment-view")?
        .equipment
        .clone();
    for entry in equipment {
        let moved = client
            .move_item(&ItemMoveIntent {
                source: entry.item.handle,
                destination: ItemMoveDestination::MainBackpack,
            })
            .await?;
        if moved.outcome != ItemMoveOutcome::Moved {
            return Err(format!("step=setup could not stow {:?}: {moved:?}", entry.slot).into());
        }
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !client
        .inventory()
        .is_some_and(|view| view.equipment.is_empty())
    {
        if tokio::time::Instant::now() >= deadline {
            return Err("step=setup need=served-unarmoured-combat-character".into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
    Ok(())
}

async fn approach_rat(
    client: &mut DevClientSession,
    world: &BundleWorld,
) -> TestResult<(EntityRef, TilePos)> {
    let started = tokio::time::Instant::now();
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        unprotected(world, at)
    })
    .ok_or("step=setup need=walkable-unprotected-tile")?;
    walk(client, world, &path).await?;
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let spawn = request_spawn(session, "oteryn:creature.rat".into())
        .await
        .map_err(|error| format!("step=setup Rat fixture refused: {error}"))?;
    let rat = parse_spawned_entity(&spawn)?;
    let visible_by = tokio::time::Instant::now() + Duration::from_secs(5);
    while !client
        .world_entities()
        .is_some_and(|view| view.get(&rat).is_some())
    {
        if tokio::time::Instant::now() >= visible_by {
            return Err("step=setup need=exact-spawned-Rat-visible".into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
    let (here, target) = approach_exact_rat(client, world, rat).await?;
    evidence(&format!(
        "S15B_TARGET actor={here:?} rat={target:?} distance=1 approach_seconds={:.3}",
        started.elapsed().as_secs_f64()
    ));
    Ok((rat, here))
}

async fn approach_exact_rat(
    client: &mut DevClientSession,
    world: &BundleWorld,
    rat: EntityRef,
) -> TestResult<(TilePos, TilePos)> {
    for _ in 0..32 {
        let here = tile(client.world_spatial().actor_position)?;
        let target = client
            .world_entities()
            .and_then(|entities| entities.iter().find(|entry| entry.entity == rat))
            .ok_or("S15b rat left the served entity view")?;
        let target = tile(target.position)?;
        let path = route_avoiding(
            world,
            here,
            |at| cardinal_neighbor(at, target.x, target.y, target.floor) && unprotected(world, at),
            |at| at != target,
        )
        .ok_or("S15b has no cardinal, unprotected approach to the visible rat")?;
        let Some(direction) = path.first() else {
            return Ok((here, target));
        };
        walk(client, world, &[*direction]).await?;
    }
    Err("S15b rat moved away from every cardinal approach".into())
}

async fn require_expiry_damage(
    client: &mut DevClientSession,
    world: &BundleWorld,
    rat: EntityRef,
    deadline: tokio::time::Instant,
    step: &str,
) -> TestResult {
    let latest_expiry = deadline - EXPIRY_OBSERVATION;
    client
        .service_liveness(latest_expiry.saturating_duration_since(tokio::time::Instant::now()))
        .await?;
    client.take_events();
    // A consumed or retried STEP can land the resumed actor away from the original
    // melee tile. Recheck the exact live Rat within the same damage deadline.
    let (actor, target) = tokio::time::timeout_at(deadline, approach_exact_rat(client, world, rat))
        .await
        .map_err(|_| format!("{step} need=cardinal-rat-approach-before-damage-deadline"))??;
    evidence(&format!(
        "S15B_TARGET step={step} actor={actor:?} rat={target:?} distance=1"
    ));
    require_damage(
        client,
        rat,
        deadline.saturating_duration_since(tokio::time::Instant::now()),
        step,
    )
    .await
}

fn cardinal_neighbor(at: TilePos, x: u16, y: u16, floor: i8) -> bool {
    at.floor == floor && u32::from(at.x.abs_diff(x)) + u32::from(at.y.abs_diff(y)) == 1
}

fn unprotected(world: &BundleWorld, at: TilePos) -> bool {
    world
        .base()
        .tile(at.x, at.y, at.floor)
        .is_some_and(|tile| !tile.protection_zone())
}

async fn walk_during_fresh_protection(
    client: &mut DevClientSession,
    world: &BundleWorld,
    approach: TilePos,
    admitted_at: tokio::time::Instant,
) -> TestResult {
    let here = tile(client.world_spatial().actor_position)?;
    let path = route(world, here, |at| at == approach)
        .ok_or("step=5 fresh-login need=walkable-route-to-rat")?;
    let before = health(client)?;
    for direction in path {
        if fresh_quiet_budget(admitted_at.elapsed()).is_zero() {
            return Err("step=5 fresh-login route exceeded the protection window".into());
        }
        let expected = destination(
            world,
            tile(client.world_spatial().actor_position)?,
            direction,
        );
        let outcome = client.step_retrying(direction).await?;
        if outcome.disposition != StepDisposition::Moved
            || Some(tile(client.world_spatial().actor_position)?) != expected
        {
            return Err(format!(
                "step=5 fresh-login STEP did not reach the rat route: {outcome:?}"
            )
            .into());
        }
        let lost_health = health(client)? < before;
        let saw_damage = client
            .take_events()
            .iter()
            .any(|event| incoming_damage(event, None));
        if lost_health || saw_damage {
            return Err("step=5 fresh-login observed=damage-during-route-to-rat".into());
        }
    }
    Ok(())
}

async fn retreat_and_logout(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let refuge = route(world, from, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| tile.protection_zone())
    })
    .ok_or("step=logout-lock need=walkable-protection-zone-to-break-rat-aggro")?;
    walk(client, world, &refuge).await?;
    logout_after_combat(client).await
}

fn actor_position(at: TilePos) -> ActorPosition {
    ActorPosition {
        x: i32::from(at.x),
        y: i32::from(at.y),
        floor: -i16::from(at.floor),
    }
}

fn require_attribution(client: &DevClientSession) -> TestResult {
    use oteryn_protocol_oteryn::analyser::CAPABILITY_ANALYSER_HIT_ATTRIBUTION_V1;
    if !client
        .selected_capabilities()
        .contains(&CAPABILITY_ANALYSER_HIT_ATTRIBUTION_V1)
    {
        return Err("step=setup need=negotiated-damage-attribution".into());
    }
    Ok(())
}

async fn player_commands(
    client: &mut DevClientSession,
    monster: EntityRef,
    expected: AttackIntentDisposition,
) -> TestResult {
    let modes = FightModes {
        fight_mode: FightMode::Defensive,
        chase: ChaseMode::Stand,
        secure: true,
    };
    let modes_reply = client.session_mut().set_fight_modes(&modes).await?;
    if modes_reply.disposition != AttackIntentDisposition::Ok {
        return Err(
            format!("step=3 command=SET_FIGHT_MODES observed={modes_reply:?} need=Ok").into(),
        );
    }
    let attack = client.session_mut().attack_target(Some(&monster)).await?;
    if attack.disposition != expected {
        return Err(format!("step=attack-target observed={attack:?} need={expected:?}").into());
    }
    let target = (expected == AttackIntentDisposition::Ok).then_some(monster);
    wait_combat_state(client, target, &modes, "step=attack-target").await?;
    let clear = client.session_mut().attack_target(None).await?;
    if clear.disposition != AttackIntentDisposition::Ok {
        return Err(format!(
            "step=3 command=ATTACK_TARGET(None) observed={clear:?} need=Ok plus cleared delta"
        )
        .into());
    }
    wait_combat_state(client, None, &modes, "step=clear-attack-target").await
}

async fn wait_combat_state(
    client: &mut DevClientSession,
    target: Option<EntityRef>,
    modes: &FightModes,
    step: &str,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + COMBAT_STATE_OBSERVATION;
    loop {
        let observed = client.session_mut().combat_state();
        if observed.is_some_and(|state| state.target == target && &state.modes == modes) {
            return Ok(());
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(format!(
                "{step} observed={observed:?} need=target-{target:?}-and-modes-{modes:?}"
            )
            .into());
        }
        client
            .service_liveness(remaining.min(Duration::from_millis(25)))
            .await?;
    }
}

fn incoming_damage(event: &SessionEvent, attacker_filter: Option<EntityRef>) -> bool {
    use oteryn_protocol_oteryn::analyser::{AnalyserDamageSource, AnalyserFact};
    matches!(event, SessionEvent::AnalyserAttributedFacts(delta) if delta.value.iter().any(|hit|
        attacker_filter.is_none_or(|attacker| hit.attacker == Some(attacker))
            && matches!(&hit.fact, AnalyserFact::DamageInput { source: AnalyserDamageSource::Creature(_), .. })))
}

fn health(client: &DevClientSession) -> TestResult<u32> {
    client
        .actor_vitals()
        .map(|vitals| vitals.health)
        .ok_or_else(|| "observed=own-actor-vitals-unavailable".into())
}

async fn require_quiet(
    client: &mut DevClientSession,
    monster: EntityRef,
    duration: Duration,
    step: &str,
) -> TestResult {
    let before = health(client)?;
    let deadline = tokio::time::Instant::now() + duration;
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    loop {
        if client
            .take_events()
            .iter()
            .any(|event| incoming_damage(event, Some(monster)))
            || health(client)? < before
        {
            return Err(format!("{step} observed=monster damage/HP loss before protection deadline need=no eligible monster target").into());
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Ok(());
        }
        client
            .service_liveness(remaining.min(Duration::from_millis(25)))
            .await?;
    }
}

async fn require_damage(
    client: &mut DevClientSession,
    monster: EntityRef,
    duration: Duration,
    step: &str,
) -> TestResult {
    let before = health(client)?;
    let deadline = tokio::time::Instant::now() + duration;
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    let mut frame = false;
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(25)).await?;
        frame |= client
            .take_events()
            .iter()
            .any(|event| incoming_damage(event, Some(monster)));
        if frame && health(client)? < before {
            return Ok(());
        }
    }
    Err(format!("{step} observed=damage-frame:{frame},hp-before:{before},hp-after:{} need=monster-attributed damage frame and lower HP", health(client)?).into())
}

async fn logout_after_combat(client: &mut DevClientSession) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    while !client
        .session_mut()
        .combat_state()
        .is_some_and(|state| !state.in_fight)
    {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err("step=logout-lock need=served-combat-clearance".into());
        }
        client
            .service_liveness(remaining.min(Duration::from_millis(100)))
            .await?;
    }
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=logout observed={reply:?} need=Accepted after combat expiry").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walking_time_does_not_extend_fresh_login_protection() {
        assert_eq!(
            fresh_quiet_budget(Duration::from_secs(8)),
            Duration::from_secs(2)
        );
        assert_eq!(fresh_quiet_budget(Duration::from_secs(10)), Duration::ZERO);
    }

    #[test]
    fn evidence_is_one_token() {
        assert_eq!(compact("step=3 missing delta"), "step=3-missing-delta");
    }
}

#[test]
fn reconnect_observation_covers_the_last_second() {
    assert_eq!(reconnect_quiet_window(), Duration::from_secs(4));
}

#[test]
fn expiry_observation_is_bounded_to_three_rat_attack_intervals() {
    assert_eq!(EXPIRY_OBSERVATION, Duration::from_secs(6));
}
