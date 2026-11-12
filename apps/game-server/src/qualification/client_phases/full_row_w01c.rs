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
//! 4. Escape by ordinary STEP commands, then retry LOGOUT until its reply is `Accepted`; freshly
//!    admit the same character and require a new game session plus the exact server-saved position,
//!    current/max health, mana and soul observed immediately before logout.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::logout::LogoutResult;
use std::collections::BTreeSet;

const ACTIONS: &[&str] = &[
    "admitted-and-walked-to-spawn",
    "attack-command-and-vitals-delta",
    "in-fight-logout-refused",
    "escaped-and-logout-accepted",
    "fresh-admission-retains-state",
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
        evidence(&format!(
            "W01C_CHECK {action} PASS observed={observed:?}"
        ));
    }

    fn finish(self, result: TestResult) {
        for action in ACTIONS {
            if !self.passed.contains(action) {
                evidence(&format!("W01C_CHECK {action} FAIL assertion-not-completed"));
            }
        }
        if let Err(error) = &result {
            evidence(&format!("W01C_GAP step={} observed={error}", self.passed.len() + 1));
        }
        let pass = result.is_ok()
            && !self.invalid
            && ACTIONS.iter().all(|action| self.passed.contains(action));
        evidence(&format!(
            "e2e_all scenario=W01c.full status={} seconds={:.3}{}",
            if pass { "PASS" } else { "FAIL" },
            self.started.elapsed().as_secs_f64(),
            if pass { "" } else { " evidence=runtime-observation" },
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(&mut row, clients))).await;
    row.finish(match result {
        Ok(result) => result,
        Err(_) => Err("journey timeout after real player commands; server supplied no required next frame".into()),
    });
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
    let mut client = connect_world(clients, 0xc1).await?;
    let first_session = client.join_snapshot().game_session_id;
    travel_to_spawn(
        &mut client,
        world,
        &metadata.spawns,
        "oteryn:creature.poison_spider",
    )
    .await?;
    row.pass(
        "admitted-and-walked-to-spawn",
        client.world_spatial().actor_position,
    );

    let before = *client.actor_vitals().ok_or("step 2: admission supplied no ActorVitals snapshot")?;
    let cancel = client.session_mut().attack_target(None).await?;
    if cancel.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(format!("step 2: attack-cancel command refused: {cancel:?}").into());
    }
    client.take_events();
    let damaged = wait_for_damage(&mut client, before.health).await?;
    row.pass("attack-command-and-vitals-delta", (cancel, damaged));

    let refusal = client.session_mut().logout().await?;
    if !matches!(refusal.result, Some(LogoutResult::InFight { .. })) {
        return Err(format!("step 3: expected InFight logout refusal, observed {refusal:?}").into());
    }
    if client.actor_vitals().is_none_or(|vitals| vitals.health == 0) {
        return Err("step 3: logout refusal did not retain a living damaged actor".into());
    }
    row.pass("in-fight-logout-refused", refusal);

    let home = route(world, tile(client.world_spatial().actor_position)?, |at| at == TOWN)
        .ok_or("step 4: no legitimate walking route away from the encounter")?;
    walk(&mut client, world, &home).await?;
    let saved = wait_for_logout(&mut client).await?;
    let saved_position = client.world_spatial().actor_position;
    row.pass("escaped-and-logout-accepted", (saved_position, saved));
    drop(client);

    let mut client = connect_world(clients, 0xc2).await?;
    let observed = *client.actor_vitals().ok_or("step 5: fresh admission supplied no ActorVitals snapshot")?;
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
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(LogoutResult::Accepted) {
        return Err(format!("step 5: final logout refused: {logout:?}").into());
    }
    Ok(())
}

async fn wait_for_damage(
    client: &mut DevClientSession,
    before: u32,
) -> TestResult<oteryn_protocol_oteryn::actor_spell::ActorVitals> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(45);
    loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        for event in client.take_events() {
            if let SessionEvent::ActorVitals(delta) = event
                && delta.value.health > 0
                && delta.value.health < before
            {
                return Ok(delta.value);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "step 2: timeout waiting for poison-spider ActorVitals damage delta; initial_health={before} current={:?}; server needs an admitted poison-spider attack source wired through the monster melee owner",
                client.actor_vitals()
            )
            .into());
        }
    }
}

async fn wait_for_logout(
    client: &mut DevClientSession,
) -> TestResult<oteryn_protocol_oteryn::actor_spell::ActorVitals> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    loop {
        let saved = *client.actor_vitals().ok_or("step 4: ActorVitals disappeared before logout")?;
        let reply = client.session_mut().logout().await?;
        match reply.result {
            Some(LogoutResult::Accepted) => return Ok(saved),
            Some(LogoutResult::InFight { .. }) if tokio::time::Instant::now() < deadline => {
                client.service_liveness(Duration::from_millis(100)).await?;
                client.take_events();
            }
            _ => return Err(format!("step 4: logout did not become accepted: {reply:?}").into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
