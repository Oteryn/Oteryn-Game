//! C08 full-row player scenario, derived from the matrix/audit and the native condition owners:
//! 1. Walk from the admitted town to a compiled poison-spider spawn and allow its ordinary combat
//!    AI to hit the character; require its poison condition from CONDITION_STATUS, then walk
//!    into a protection zone and observe a delayed health loss with melee excluded.
//! 2. Send the served, available `exana pox` spell command through the native spell protocol;
//!    expect `Cast`, a mana delta, and no further poison health delta during the next tick window.
//! 3. Request logout while the combat condition is live; expect the explicit `InFight` refusal,
//!    then continue on the same admitted session and prove that the refusal changed no vitals.
//! 4. With health below maximum, observe a later positive `ActorVitals` health delta from the
//!    server's ordinary regeneration owner, without changing state through a fixture or database.
//! 5. Return to the poison spider, acquire another delayed poison delta, request logout (first
//!    refusal may be `InFight`), retry until accepted, then make a fresh admission; expect a new
//!    game-session id and another delayed health delta, proving the durable condition survived.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
use std::num::NonZeroU32;

struct Row {
    started: std::time::Instant,
    step: usize,
}

impl Row {
    fn pass(&mut self, detail: impl std::fmt::Debug) {
        evidence(&format!(
            "C08_CHECK step={} PASS observed={detail:?}",
            self.step
        ));
        self.step += 1;
    }
    fn finish(self, result: &TestResult) {
        if let Err(error) = result {
            evidence(&format!("C08_GAP step={} observed={error}", self.step));
        }
        evidence(&format!(
            "e2e_all scenario=C08.full status={} seconds={:.3}",
            if result.is_ok() && self.step == 6 {
                "PASS"
            } else {
                "FAIL"
            },
            self.started.elapsed().as_secs_f64()
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row {
        started: std::time::Instant::now(),
        step: 1,
    };
    let result = row_complete_with_timeout(Duration::from_secs(240), journey(&mut row, clients)).await;
    let result = match result {
        Ok(result) => result,
        Err(_) => Err(format!(
            "timeout waiting for server reply/delta at step {}",
            row.step
        )
        .into()),
    };
    row.finish(&result);
}

async fn next_health_change(
    client: &mut DevClientSession,
    before: u32,
    seconds: u64,
) -> TestResult<u32> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(seconds);
    loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        for event in client.take_events() {
            if let SessionEvent::ActorVitals(delta) = event
                && delta.value.health != before
            {
                return Ok(delta.value.health);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("timeout: no ActorVitals health delta from {before}").into());
        }
    }
}

async fn assert_health_unchanged(
    client: &mut DevClientSession,
    expected: u32,
    seconds: u64,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(seconds);
    loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        for event in client.take_events() {
            if let SessionEvent::ActorVitals(delta) = event
                && delta.value.health < expected
            {
                return Err(format!(
                    "step dispel: poison health changed during post-cast window: {expected}->{}",
                    delta.value.health
                )
                .into());
            }
        }
        if tokio::time::Instant::now() >= deadline {
            let observed = client
                .actor_vitals()
                .ok_or("step dispel: vitals absent after post-cast window")?
                .health;
            if observed < expected || row_poison_active(client).await? {
                return Err(format!(
                    "step dispel: post-cast health changed without an observed delta: {expected}->{observed}"
                )
                .into());
            }
            return Ok(());
        }
    }
}

async fn poison(client: &mut DevClientSession, world: &BundleWorld) -> TestResult<(u32, u32)> {
    client.session_mut().attack_target(None).await?;
    client.take_events();
    let before = client
        .actor_vitals()
        .ok_or("step poison: vitals absent")?
        .health;
    let direct = next_health_change(client, before, 30).await?;
    if direct >= before {
        return Err(
            format!("step poison: first delta was not direct damage: {before}->{direct}").into(),
        );
    }
    row_isolate_poison(client, world).await?;
    let isolated = client.actor_vitals().ok_or("isolated vitals absent")?.health;
    let delayed = next_health_change(client, isolated, 15).await?;
    if delayed >= isolated {
        return Err(
            format!("step poison: delayed delta was not damage: {direct}->{delayed}").into(),
        );
    }
    Ok((direct, delayed))
}

fn antidote(client: &mut DevClientSession) -> TestResult<(NonZeroU32, u32)> {
    use oteryn_protocol_oteryn::actor_spell_catalog as wire;
    let snapshot = client
        .session_mut()
        .gated_snapshot(wire::STATE_DOMAIN_ACTOR_SPELL_CATALOG)
        .ok_or("step dispel: served spell catalogue snapshot absent")?;
    antidote_from(&snapshot.payload)
}

fn antidote_from(payload: &[u8]) -> TestResult<(NonZeroU32, u32)> {
    use oteryn_protocol_oteryn::actor_spell_catalog as wire;
    let entries = wire::decode_catalog(payload)
        .map_err(|error| format!("step dispel: catalogue decode: {error:?}"))?;
    let entry = entries
        .iter()
        .find(|entry| entry.words == "exana pox")
        .ok_or("step dispel: exana pox absent from served catalogue")?;
    Ok((
        NonZeroU32::new(entry.cast_index).ok_or("step dispel: zero cast index")?,
        entry.mana,
    ))
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(240);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = row_connect_world(clients, 0xc8).await?;
    let mut row_needs_logout_0 = true;
    let row_result: TestResult<_> = async {
        travel_to_spawn(
            &mut client,
            world,
            &metadata.spawns,
            "oteryn:creature.poison_spider",
        )
        .await?;
        let damage = poison(&mut client, world).await?;
        row.pass(damage);

        let (spell, mana_cost) = antidote(&mut client)?;
        let mana = client
            .actor_vitals()
            .ok_or("step dispel: vitals absent")?
            .mana;
        let cast = client.cast_spell(spell, SpellTarget::None, false).await?;
        if cast.disposition != SpellCastDisposition::Cast {
            return Err(format!("step dispel: exana pox reply={cast:?}").into());
        }
        let mana_after = client
            .actor_vitals()
            .ok_or("step dispel: post-cast vitals absent")?
            .mana;
        if mana_after.checked_add(mana_cost) != Some(mana) {
            return Err(format!(
            "step dispel: accepted reply missing mana delta {mana}->{mana_after} cost={mana_cost}"
        )
        .into());
        }
        let health_after = client
            .actor_vitals()
            .ok_or("step dispel: post-cast vitals absent")?
            .health;
        assert_health_unchanged(&mut client, health_after, 15).await?;
        row.pass((cast.disposition, mana, mana_after, health_after));

        let before_refusal = *client.actor_vitals().ok_or("step refusal: vitals absent")?;
        let refusal = client.row_logout().await?;
        if !matches!(
            refusal.result,
            Some(oteryn_protocol_oteryn::logout::LogoutResult::InFight { .. })
        ) || client.actor_vitals() != Some(&before_refusal)
        {
            return Err(format!(
                "step refusal: combat logout was not an unchanged InFight refusal: {refusal:?}"
            )
            .into());
        }
        row.pass(refusal.result);

        let damaged = client
            .actor_vitals()
            .ok_or("step regeneration: vitals absent")?
            .health;
        let regenerated = next_health_change(&mut client, damaged, 45).await?;
        if regenerated <= damaged {
            return Err(format!(
                "step regeneration: expected positive health delta, got {damaged}->{regenerated}"
            )
            .into());
        }
        row.pass((damaged, regenerated));

        travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.poison_spider").await?;
        let second = poison(&mut client, world).await?;
        let old_session = client.join_snapshot().game_session_id;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
        loop {
            let reply = client.row_logout().await?;
            match reply.result {
                Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) => {
                    row_needs_logout_0 = false;
                    break;
                }
                Some(oteryn_protocol_oteryn::logout::LogoutResult::InFight { .. }) => {}
                _ => {
                    return Err(
                        format!("step persistence: unexpected logout reply={reply:?}").into(),
                    );
                }
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!(
                    "step persistence: logout remained refused after poison, last={reply:?}"
                )
                .into());
            }
            client.service_liveness(Duration::from_millis(250)).await?;
            client.take_events();
        }
        if row_needs_logout_0 {
            row_finish(&mut client).await?;
            row_needs_logout_0 = false;
        }
        let mut client = row_connect_world(clients, 0xc9).await?;
        let mut row_needs_logout_1 = true;
        let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
            if client.join_snapshot().game_session_id == old_session {
                return Err("step persistence: fresh admission reused game session".into());
            }
            let admitted = client
                .actor_vitals()
                .ok_or("step persistence: admitted vitals absent")?
                .health;
            if !row_poison_active(&mut client).await? { return Err("fresh admission lost poison condition".into()); }
            let tick = next_health_change(&mut client, admitted, 15).await?;
            if tick >= admitted {
                return Err(format!(
            "step persistence: first fresh-admission delta was not durable DoT: {admitted}->{tick}"
        )
        .into());
            }
            row.pass((second, admitted, tick));
            row_finish(&mut client).await?;
            row_needs_logout_1 = false;
            Ok(())
        })
        .await
        .unwrap_or_else(|error| Err(error.into()));
        if row_needs_logout_1 {
            row_finish(&mut client).await?;
        }
        row_result
    }
    .await;
    if row_needs_logout_0 {
        row_finish(&mut client).await?;
    }
    row_result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_or_malformed_spell_catalogue_cannot_supply_an_antidote() {
        assert!(antidote_from(&[]).is_err());
        assert!(antidote_from(&[255]).is_err());
    }
}
