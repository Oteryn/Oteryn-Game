//! S15a (login protection: monsters ignore a fresh login for 10 s):
//! 1. Walk the node-boot character out of protection, log out and admit at that saved position.
//!    Place an exact Rat with the qualification spawn fixture as soon as the client joins.
//! 2. Require no monster-attributed damage frame and no HP loss through the 10 s window.
//!    The fresh admission must expose the exact Rat early enough to test the window.
//! 3. After the window, require a monster-attributed damage frame and a lower HP, proving the
//!    monster was only held back by the protection, then log out once the combat lock expires.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, EntityRef, SessionEvent};

const WINDOW: Duration = Duration::from_secs(10);
const MARGIN: Duration = Duration::from_millis(750);

async fn journey(clients: &SeamClients<'_>) -> TestResult<Verdict> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut setup = connect_world(clients, 0x8d).await?;
    stow_equipment(&mut setup).await?;
    let path = route(world, tile(setup.world_spatial().actor_position)?, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone() && !tile.no_logout())
    })
    .ok_or("step=setup;needed=reachable-unprotected-logout-tile")?;
    walk(&mut setup, world, &path).await?;
    let staged = setup.world_spatial().actor_position;
    let old_session = setup.join_snapshot().game_session_id;
    logout(&mut setup).await?;
    drop(setup);

    let admission_requested = tokio::time::Instant::now();
    let mut client = connect_world(clients, 0x8e).await?;
    let admitted = tokio::time::Instant::now();
    let quiet_deadline = admission_requested + WINDOW;
    if client.join_snapshot().game_session_id == old_session
        || client.world_spatial().actor_position != staged
        || !client
            .inventory()
            .is_some_and(|view| view.equipment.is_empty())
    {
        return Err(
            "step=setup;needed=fresh-unarmoured-admission-at-saved-unprotected-position".into(),
        );
    }
    require_attribution(&client)?;
    let initial_health = health(&client)?;
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let monster = tokio::time::timeout_at(quiet_deadline - MARGIN, async {
        let spawn = request_spawn(session, "oteryn:creature.rat".into())
            .await
            .map_err(|error| format!("step=window;observed=rat-fixture-refused-{error}"))?;
        let monster = parse_spawned_entity(&spawn)?;
        while !client.world_entities().is_some_and(|entities| {
            entities
                .get(&monster)
                .is_some_and(|entry| entry.kind == EntityKind::Creature)
        }) {
            client.service_liveness(Duration::from_millis(25)).await?;
        }
        TestResult::Ok(monster)
    })
    .await
    .map_err(|_| "step=window;needed=exact-rat-visible-before-login-protection-expires")??;
    evidence(&format!(
        "S15A_TARGET actor={staged:?} rat={monster:?} setup_seconds={:.3}",
        admitted.elapsed().as_secs_f64()
    ));
    // The server commits admission between the request and the joined receipt.
    // Check silence up to the earliest expiry and count damage after the latest.
    require_quiet(
        &mut client,
        monster,
        initial_health,
        quiet_deadline,
        "step=protected-window",
    )
    .await?;
    client
        .service_liveness(
            (admitted + WINDOW).saturating_duration_since(tokio::time::Instant::now()),
        )
        .await?;
    require_damage(
        &mut client,
        monster,
        Duration::from_secs(20),
        "step=after-window",
    )
    .await?;
    let shelter = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| tile.protection_zone() && !tile.no_logout())
    })
    .ok_or("step=logout;needed=walkable-protection-zone-outside-rat-aggro")?;
    walk(&mut client, world, &shelter).await?;
    logout_after_combat(&mut client).await?;
    Ok((
        "PASS",
        format!(
            "quiet-until-{:.1}s-after-admission-then-monster-damage",
            WINDOW.as_secs_f64()
        ),
    ))
}

async fn stow_equipment(client: &mut DevClientSession) -> TestResult {
    use oteryn_dev_client::{ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome};

    // The Rat's authored physical attack caps at 8 and is mitigated by armour and shields.
    let equipped = client
        .inventory()
        .ok_or("step=setup;needed=equipment-view")?
        .equipment
        .clone();
    for entry in equipped {
        let moved = client
            .move_item(&ItemMoveIntent {
                source: entry.item.handle,
                destination: ItemMoveDestination::MainBackpack,
            })
            .await?;
        if moved.outcome != ItemMoveOutcome::Moved {
            return Err(format!(
                "step=setup;observed=could-not-stow-{:?}-{moved:?}",
                entry.slot
            )
            .into());
        }
    }
    Ok(())
}

fn require_attribution(client: &DevClientSession) -> TestResult {
    use oteryn_protocol_oteryn::analyser::CAPABILITY_ANALYSER_HIT_ATTRIBUTION_V1;
    if !client
        .selected_capabilities()
        .contains(&CAPABILITY_ANALYSER_HIT_ATTRIBUTION_V1)
    {
        return Err("step=setup;needed=negotiated-damage-attribution".into());
    }
    Ok(())
}

fn monster_damage(event: &SessionEvent, monster: EntityRef) -> bool {
    use oteryn_protocol_oteryn::analyser::{AnalyserDamageSource, AnalyserFact};
    matches!(event, SessionEvent::AnalyserAttributedFacts(delta) if delta.value.iter().any(|hit|
        hit.attacker == Some(monster)
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
    before: u32,
    deadline: tokio::time::Instant,
    step: &str,
) -> TestResult {
    loop {
        if client
            .take_events()
            .iter()
            .any(|event| monster_damage(event, monster))
            || health(client)? < before
        {
            return Err(format!(
                "{step};observed=monster-damage-or-hp-loss-inside-login-protection"
            )
            .into());
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
    client.take_events();
    let deadline = tokio::time::Instant::now() + duration;
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    let mut frame = false;
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(25)).await?;
        frame |= client
            .take_events()
            .iter()
            .any(|event| monster_damage(event, monster));
        if frame && health(client)? < before {
            return Ok(());
        }
    }
    Err(format!(
        "{step};observed=damage-frame:{frame},hp-before:{before},hp-after:{};needed=monster-damage-after-window", health(client)?
    )
    .into())
}

/// A logout refused for combat is retried until the combat lock expires.
async fn logout_after_combat(client: &mut DevClientSession) -> TestResult {
    use oteryn_protocol_oteryn::logout::LogoutResult;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    loop {
        let reply = client.session_mut().logout().await?;
        match reply.result {
            Some(LogoutResult::Accepted) => return Ok(()),
            Some(LogoutResult::InFight { .. } | LogoutResult::Busy)
                if tokio::time::Instant::now() < deadline =>
            {
                client.service_liveness(Duration::from_millis(250)).await?;
            }
            other => return Err(format!("step=logout;observed={other:?}").into()),
        }
    }
}

type Verdict = (&'static str, String);

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok((status, detail))) => (status, token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=S15a.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

/// One evidence token: no whitespace, bounded length.
fn token(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "-_.:/=<>,;".contains(character) {
                character
            } else {
                '-'
            }
        })
        .take(240)
        .collect()
}

fn expect(ok: bool, step: &str, observed: impl std::fmt::Debug) -> TestResult {
    if ok {
        Ok(())
    } else {
        Err(format!("step={step};observed={observed:?}").into())
    }
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    expect(
        reply.result == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted),
        "logout",
        reply.result,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_has_no_whitespace_and_is_bounded() {
        let value = token(&format!("step=a b\tc\n{}", "x".repeat(400)));
        assert!(!value.contains(char::is_whitespace));
        assert!(value.len() <= 240);
        assert!(value.starts_with("step=a-b-c-"));
    }
}
