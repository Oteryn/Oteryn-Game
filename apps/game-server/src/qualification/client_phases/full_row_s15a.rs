//! S15a (login protection: monsters ignore a fresh login for 10 s):
//! 1. Admit the node-boot character and start the clock, walk to a compiled rat spawn with
//!    player STEP commands and keep the nearest visible creature as the monster under test.
//! 2. For the rest of the 10 s window after admission, require no monster-attributed damage
//!    frame and no HP loss. If the walk itself used up the window, the row reports the
//!    precondition instead of passing.
//! 3. After the window, require a monster-attributed damage frame and a lower HP, proving the
//!    monster was only held back by the protection, then log out once the combat lock expires.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityDetail, EntityKind, EntityRef, SessionEvent};

const WINDOW: Duration = Duration::from_secs(10);
const MARGIN: Duration = Duration::from_millis(750);

async fn journey(clients: &SeamClients<'_>) -> TestResult<Verdict> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0x8d).await?;
    let admitted = tokio::time::Instant::now();
    travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
    let (monster, own) = actors(&client)?;
    let used = admitted.elapsed();
    if used + MARGIN + MARGIN >= WINDOW {
        logout(&mut client).await?;
        return Ok((
            "FAIL",
            format!(
                "step=window;observed=monster-reached-after-{:.1}s;needed=monster-within-10s-of-login",
                used.as_secs_f64()
            ),
        ));
    }
    client.take_events();
    require_quiet(
        &mut client,
        monster,
        own,
        WINDOW.saturating_sub(used + MARGIN),
        "step=protected-window",
    )
    .await?;
    require_damage(
        &mut client,
        monster,
        own,
        Duration::from_secs(20),
        "step=after-window",
    )
    .await?;
    logout_after_combat(&mut client).await?;
    Ok((
        "PASS",
        format!(
            "quiet-for-{:.1}s-then-monster-damage",
            WINDOW.saturating_sub(used + MARGIN).as_secs_f64()
        ),
    ))
}

fn actors(client: &DevClientSession) -> TestResult<(EntityRef, EntityRef)> {
    let entities = client
        .world_entities()
        .ok_or("step=setup;observed=missing-entity-snapshot")?;
    let own = entities
        .iter()
        .find(|entry| entry.entity.identity == *entities.own_identity())
        .map(|entry| entry.entity)
        .ok_or("step=setup;observed=own-actor-absent")?;
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
        .ok_or("step=setup;observed=no-visible-monster")?;
    Ok((monster, own))
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
        .ok_or("observed=own-actor-left-entity-snapshot")?;
    match entry.detail {
        EntityDetail::Actor { health_percent, .. } => Ok(health_percent),
        _ => Err("observed=own-actor-has-no-health".into()),
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
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(25)).await?;
        if client
            .take_events()
            .iter()
            .any(|event| monster_damage(event, monster, own))
            || health(client, own)? < before
        {
            return Err(format!(
                "{step};observed=monster-damage-or-hp-loss-inside-login-protection"
            )
            .into());
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
    Err(format!(
        "{step};observed=damage-frame:{frame},hp-before:{before};needed=monster-damage-after-window"
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
    let result = tokio::time::timeout(Duration::from_secs(240), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok((status, detail))) => (status, token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=S15A.full status={status} seconds={:.3} evidence={detail}",
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
