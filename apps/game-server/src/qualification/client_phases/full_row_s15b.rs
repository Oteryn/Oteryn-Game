//! S15b — login protection: monster targeting protection after reconnect.
//!
//! Concrete player scenario, derived from the S15b matrix/audit row and its runtime owner:
//! 1. Admit the node-boot character, walk from town to a compiled rat spawn, wait through the
//!    fresh-login window, and require a monster-attributed damage presentation plus a lower HP
//!    delta. This proves that the live monster can perceive and target this character.
//! 2. Lose the real TLS connection and resume it with the native reconnect command. Require the
//!    same session/actor continuity supplied by the shared recovery helper.
//! 3. During the four-second reentry window, send SET_FIGHT_MODES, ATTACK_TARGET and
//!    ATTACK_TARGET(None). Require their accepted replies and exact combat-state deltas (login
//!    protection is a monster-selection rule, not a refusal of the player's commands), while
//!    requiring no monster-attributed damage or HP loss.
//! 4. After the window, require a new monster-attributed damage frame and HP loss. Then send
//!    LOGOUT while in fight and require the server's refusal; retry only until the ordinary
//!    combat lock expires and require an accepted logout.
//! 5. Perform a fresh admission, walk back to a live rat using player STEP commands, require the
//!    fresh-login protection interval to suppress monster damage, then require damage after its
//!    expiry. This re-check proves that logout did not durably retain or extend the reconnect
//!    epoch; only the separately owned fresh-admission window applies.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityDetail, EntityKind, EntityRef, SessionEvent};
use oteryn_protocol_oteryn::attack::{AttackIntentDisposition, ChaseMode, FightMode, FightModes};

const ROW: &str = "S15b";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients))).await;
    let result = match result {
        Ok(result) => result,
        Err(_) => Err(
            "step=journey observed=timeout-240s need=responsive monster AI and combat-lock expiry"
                .into(),
        ),
    };
    if let Err(error) = &result {
        evidence(&format!("S15B_GAP {error}"));
    }
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario={ROW}.full status={} seconds={:.3}{}",
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

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0x8b).await?;
    travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
    let (monster, own) = actors(&client)?;
    require_damage(
        &mut client,
        monster,
        own,
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
    client = client_disconnect_resume::recover(client, clients, expected)
        .await
        .map_err(|error| {
            format!("step=2 reconnect observed={error} need=committed CompleteReconnect")
        })?;
    let (monster, own) = actors(&client)?;
    evidence("S15B_CHECK step=2 committed-reconnect PASS");

    player_commands(&mut client, monster).await?;
    client.take_events();
    require_quiet(
        &mut client,
        monster,
        own,
        Duration::from_millis(2_500),
        "step=3 four-second protection",
    )
    .await?;
    evidence("S15B_CHECK step=3 commands-accepted-and-monster-suppressed PASS");
    require_damage(
        &mut client,
        monster,
        own,
        Duration::from_secs(12),
        "step=4 reentry expiry",
    )
    .await?;

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
    logout_after_combat(&mut client).await?;
    evidence("S15B_CHECK step=4 expiry-damage-and-logout-refusal PASS");
    drop(client);

    let mut fresh = connect_world(clients, 0x8c)
        .await
        .map_err(|error| format!("step=5 fresh admission observed={error}"))?;
    travel_to_spawn(&mut fresh, world, &metadata.spawns, "oteryn:creature.rat").await?;
    let (monster, own) = actors(&fresh)?;
    fresh.take_events();
    require_quiet(
        &mut fresh,
        monster,
        own,
        Duration::from_secs(3),
        "step=5 fresh-login protection",
    )
    .await?;
    require_damage(
        &mut fresh,
        monster,
        own,
        Duration::from_secs(12),
        "step=5 fresh-login expiry",
    )
    .await?;
    evidence("S15B_CHECK step=5 fresh-admission-window-not-reentry-epoch PASS");
    Ok(())
}

fn actor_position(at: TilePos) -> ActorPosition {
    ActorPosition {
        x: i32::from(at.x),
        y: i32::from(at.y),
        floor: -i16::from(at.floor),
    }
}

fn actors(client: &DevClientSession) -> TestResult<(EntityRef, EntityRef)> {
    let entities = client
        .world_entities()
        .ok_or("step=setup observed=missing entity snapshot")?;
    let own = entities
        .iter()
        .find(|entry| entry.entity.identity == *entities.own_identity())
        .map(|entry| entry.entity)
        .ok_or("step=setup observed=own actor absent")?;
    let here = client.world_spatial().actor_position;
    let monster = entities
        .iter()
        .filter(|entry| entry.kind == EntityKind::Creature && entry.position.floor == here.floor)
        .min_by_key(|entry| {
            entry
                .position
                .x
                .abs_diff(here.x)
                .max(entry.position.y.abs_diff(here.y))
        })
        .map(|entry| entry.entity)
        .ok_or("step=setup observed=no visible monster")?;
    Ok((monster, own))
}

async fn player_commands(client: &mut DevClientSession, monster: EntityRef) -> TestResult {
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
    if attack.disposition != AttackIntentDisposition::Ok {
        return Err(format!("step=3 command=ATTACK_TARGET observed={attack:?} need=Ok").into());
    }
    if client
        .session_mut()
        .combat_state()
        .is_none_or(|state| state.target != Some(monster) || state.modes != modes)
    {
        return Err(
            "step=3 attack observed=missing exact combat-state delta need=selected monster".into(),
        );
    }
    let clear = client.session_mut().attack_target(None).await?;
    if clear.disposition != AttackIntentDisposition::Ok
        || client
            .session_mut()
            .combat_state()
            .is_none_or(|state| state.target.is_some())
    {
        return Err(format!(
            "step=3 command=ATTACK_TARGET(None) observed={clear:?} need=Ok plus cleared delta"
        )
        .into());
    }
    Ok(())
}

fn monster_damage(event: &SessionEvent, monster: EntityRef, own: EntityRef) -> bool {
    use oteryn_protocol_oteryn::presentation::PresentationEventKind;
    matches!(event, SessionEvent::Presentation(delta) if delta.value.iter().any(|event|
        matches!(&event.kind, PresentationEventKind::ValueText { target, attacker: Some(attacker), primary, .. }
            if *target == own && *attacker == monster && primary.value > 0)))
}

fn health(client: &DevClientSession, own: EntityRef) -> TestResult<u8> {
    let entry = client
        .world_entities()
        .and_then(|entities| entities.iter().find(|entry| entry.entity == own))
        .ok_or("observed=own actor left entity snapshot")?;
    match entry.detail {
        EntityDetail::Actor { health_percent, .. } => Ok(health_percent),
        _ => Err("observed=own actor has no health delta".into()),
    }
}

async fn require_quiet(
    client: &mut DevClientSession,
    monster: EntityRef,
    own: EntityRef,
    duration: Duration,
    step: &str,
) -> TestResult {
    let before = health(client, own)?;
    let deadline = tokio::time::Instant::now() + duration;
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(25)).await?;
        if client
            .take_events()
            .iter()
            .any(|event| monster_damage(event, monster, own))
            || health(client, own)? < before
        {
            return Err(format!("{step} observed=monster damage/HP loss before protection deadline need=no eligible monster target").into());
        }
    }
    Ok(())
}

async fn require_damage(
    client: &mut DevClientSession,
    monster: EntityRef,
    own: EntityRef,
    duration: Duration,
    step: &str,
) -> TestResult {
    let before = health(client, own)?;
    client.take_events();
    let deadline = tokio::time::Instant::now() + duration;
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    let mut frame = false;
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(25)).await?;
        frame |= client
            .take_events()
            .iter()
            .any(|event| monster_damage(event, monster, own));
        if frame && health(client, own)? < before {
            return Ok(());
        }
    }
    Err(format!("{step} observed=damage-frame:{frame},hp-before:{before},hp-after:{} need=monster-attributed damage frame and lower HP", health(client, own)?).into())
}

async fn logout_after_combat(client: &mut DevClientSession) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    loop {
        let reply = client.session_mut().logout().await?;
        match reply.result {
            Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) => return Ok(()),
            Some(
                oteryn_protocol_oteryn::logout::LogoutResult::InFight { .. }
                | oteryn_protocol_oteryn::logout::LogoutResult::Busy,
            ) if tokio::time::Instant::now() < deadline => {
                client.service_liveness(Duration::from_millis(250)).await?;
            }
            _ => {
                return Err(format!(
                    "step=4 eventual logout observed={reply:?} need=Accepted after combat expiry"
                )
                .into());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::compact;

    #[test]
    fn evidence_is_one_token() {
        assert_eq!(compact("step=3 missing delta"), "step=3-missing-delta");
    }
}
