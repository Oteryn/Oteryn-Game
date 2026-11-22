//! C08 full-row player scenario, derived from the matrix/audit and the native condition owners:
//! 1. Walk from the admitted town to a compiled poison-spider spawn and allow its ordinary combat
//!    AI to hit the character; expect an `ActorVitals` health delta followed by a second, delayed
//!    health delta, proving that a live condition/DoT turn (rather than one direct hit) ran.
//! 2. Send the served, available `exana pox` spell command through the native spell protocol;
//!    expect `Cast`, a mana delta, and removal of the authoritative poison icon.
//! 3. Before dispelling, require the explicit `InFight` logout refusal during active combat.
//! 4. With health below maximum, observe a later positive `ActorVitals` health delta from the
//!    server's ordinary regeneration owner, without changing state through a fixture or database.
//! 5. Return to the poison spider, acquire another delayed poison delta, request logout (first
//!    refusal may be `InFight`), retry until accepted, then make a fresh admission; expect a new
//!    game-session id, an authoritative active poison icon, and resumed damage out of combat.
use super::full_row_c01::admission_error;
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
            let detail = error.to_string();
            if detail.starts_with("step=") {
                evidence(&format!("C08_GAP {detail}"));
            } else {
                evidence(&format!("C08_GAP step={} observed={detail}", self.step));
            }
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
    let result = verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(&mut row, clients))).await;
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

async fn poison_status(client: &mut DevClientSession) -> TestResult<bool> {
    use oteryn_protocol_oteryn::condition_status as wire;
    let reply = client.session_mut().gated_command(
        wire::CAPABILITY_CONDITION_STATUS_V1, wire::COMMAND_TYPE_CONDITION_STATUS_QUERY, &[],
    ).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step=poison-status observed={reply:?} need=ConditionStatus").into());
    }
    let status = wire::ConditionStatus::decode(&reply.payload)
        .ok_or("step=poison-status observed=invalid-condition-frame")?;
    Ok(has_poison(&status))
}

fn has_poison(status: &oteryn_protocol_oteryn::condition_status::ConditionStatus) -> bool {
    status.conditions.iter().any(|condition| condition.icon == 1)
}

async fn poison(client: &mut DevClientSession) -> TestResult<(u32, u32)> {
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
    let delayed = next_health_change(client, direct, 15).await?;
    if delayed >= direct {
        return Err(
            format!("step poison: delayed delta was not damage: {direct}->{delayed}").into(),
        );
    }
    if !poison_status(client).await? {
        return Err("step=poison observed=damage-without-poison-icon need=live-poison-condition".into());
    }
    Ok((direct, delayed))
}

fn antidote(client: &mut DevClientSession) -> TestResult<(NonZeroU32, u32)> {
    use oteryn_protocol_oteryn::actor_spell_catalog as wire;
    let snapshot = client
        .session_mut()
        .gated_snapshot(wire::STATE_DOMAIN_ACTOR_SPELL_CATALOG)
        .ok_or("step dispel: served spell catalogue snapshot absent")?;
    let entries = wire::decode_catalog(&snapshot.payload)
        .map_err(|error| format!("step dispel: catalogue decode: {error:?}"))?;
    let entry = entries
        .iter()
        .find(|entry| entry.words == "exana pox")
        .ok_or("step dispel: exana pox absent from served catalogue")?;
    if !entry.available {
        return Err(format!("step=dispel-eligibility observed=exana-pox-unavailable required-level={} learned={} need=player-earned-antidote-eligibility", entry.level, entry.learned).into());
    }
    Ok((
        NonZeroU32::new(entry.cast_index).ok_or("step dispel: zero cast index")?,
        entry.mana,
    ))
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
    let mut client = connect_world(clients, 0xc8)
        .await
        .map_err(|error| admission_error("initial-admission", error))?;
    travel_to_spawn(
        &mut client,
        world,
        &metadata.spawns,
        "oteryn:creature.poison_spider",
    )
    .await?;
    let damage = poison(&mut client).await?;
    row.pass(damage);

    let before_refusal = client.actor_vitals().copied().ok_or("step=logout-refusal observed=no-vitals")?;
    let initial_refusal = client.session_mut().logout().await?;
    if !matches!(initial_refusal.result, Some(oteryn_protocol_oteryn::logout::LogoutResult::InFight { .. })) || client.actor_vitals() != Some(&before_refusal) {
        return Err(format!("step=logout-refusal observed={initial_refusal:?} need=InFight-during-poison-combat").into());
    }

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
    if poison_status(&mut client).await? {
        return Err("step=dispel observed=poison-still-active need=poison-removed".into());
    }
    row.pass((cast.disposition, mana, mana_after));
    row.pass(initial_refusal.result);

    let safe = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world.base().tile(at.x, at.y, at.floor).is_some_and(|tile| tile.protection_zone())
    }).ok_or("step=regeneration observed=no-reachable-protection-zone need=retreat-from-spider")?;
    walk(&mut client, world, &safe).await?;
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
    client.take_events();
    let clean_until = tokio::time::Instant::now() + Duration::from_secs(15);
    let mut health = regenerated;
    while tokio::time::Instant::now() < clean_until {
        client.service_liveness(Duration::from_millis(50)).await?;
        for event in client.take_events() {
            if let SessionEvent::ActorVitals(delta) = event {
                if delta.value.health < health {
                    return Err(format!("step=dispel observed=health-loss-after-retreat:{health}->{} need=no-further-poison-damage", delta.value.health).into());
                }
                health = delta.value.health;
            }
        }
    }

    travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.poison_spider").await?;
    let second = poison(&mut client).await?;
    let safe = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world.base().tile(at.x, at.y, at.floor).is_some_and(|tile| tile.protection_zone())
    }).ok_or("step=poison-persistence observed=no-safe-retreat-route")?;
    walk(&mut client, world, &safe).await?;
    let old_session = client.join_snapshot().game_session_id;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    loop {
        if !poison_status(&mut client).await? {
            return Err("step=poison-persistence observed=poison-expired-before-accepted-logout need=live-condition-at-save".into());
        }
        let reply = client.session_mut().logout().await?;
        match reply.result {
            Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) => break,
            Some(oteryn_protocol_oteryn::logout::LogoutResult::InFight { .. }) => {}
            _ => return Err(format!("step persistence: unexpected logout reply={reply:?}").into()),
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
    drop(client);
    let mut client = connect_world(clients, 0xc9)
        .await
        .map_err(|error| admission_error("fresh-admission", error))?;
    if client.join_snapshot().game_session_id == old_session {
        return Err("step persistence: fresh admission reused game session".into());
    }
    if !poison_status(&mut client).await? {
        return Err("step=poison-persistence observed=missing-poison-icon-on-fresh-admission need=saved-live-condition".into());
    }
    let tick = resumed_poison_tick(&mut client, world).await?;
    row.pass((second, tick));
    let _ = client.session_mut().logout().await?;
    Ok(())
}

fn creature_can_interfere(client: &DevClientSession, at: ActorPosition) -> bool {
    client.world_entities().is_none_or(|entities| entities.iter().any(|entry| {
        entry.kind == oteryn_dev_client::EntityKind::Creature
            && entry.position.floor == at.floor
            && entry.position.x.abs_diff(at.x).max(entry.position.y.abs_diff(at.y)) <= 8
    }))
}

async fn resumed_poison_tick(client: &mut DevClientSession, world: &BundleWorld) -> TestResult<(u32, u32)> {
    let basis = spatial_basis::current(client)?;
    let safe_site = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world.base().tile(at.x, at.y, at.floor).is_some_and(|tile| !tile.protection_zone())
            && !creature_can_interfere(client, basis.position(at))
    }).ok_or("step=poison-persistence observed=no-safe-tick-site-outside-pz need=uninterrupted-durable-poison-tick")?;
    walk(client, world, &safe_site).await?;
    let site = client.world_spatial().actor_position;
    let mut health = client.actor_vitals().ok_or("step=poison-persistence observed=no-post-admission-vitals")?.health;
    client.take_events();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    loop {
        if creature_can_interfere(client, site) || client.world_spatial().actor_position != site {
            return Err("step=poison-persistence observed=unsafe-tick-site need=damage-without-creature-contact".into());
        }
        for event in client.take_events() {
            if let SessionEvent::ActorVitals(delta) = event {
                if delta.value.health < health {
                    return Ok((health, delta.value.health));
                }
                health = delta.value.health;
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step=poison-persistence observed=active-icon-without-damage need=resumed-saved-poison-tick".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_protocol_oteryn::condition_status::{ActiveCondition, ConditionStatus};

    #[test]
    fn burning_does_not_prove_poison() {
        let mut status = ConditionStatus { conditions: vec![ActiveCondition { icon: 2, sub_id: 0, remaining_ms: 1000 }] };
        assert!(!has_poison(&status));
        status.conditions[0].icon = 1;
        assert!(has_poison(&status));
    }
}
