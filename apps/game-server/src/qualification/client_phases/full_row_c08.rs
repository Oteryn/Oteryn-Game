//! C08 full-row player scenario, derived from the matrix/audit and the native condition owners:
//! 1. Walk from the admitted town to a compiled poison-spider spawn and allow its ordinary combat
//!    AI to apply poison, then require a creature-sourced periodic Earth damage fact.
//! 2. Send the served, available `exana pox` spell command through the native spell protocol;
//!    expect `Cast`, a mana delta, removal of the poison icon and immediate tick cessation.
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
    character_prep::declare_seeded("C08.full", "level-10-sorcerer", "Cure Poison eligibility");
    let mut row = Row {
        started: std::time::Instant::now(),
        step: 1,
    };
    let result = verdict::indexed_timeout(Duration::from_secs(720), verdict::observe(journey(&mut row, clients))).await;
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

pub(super) async fn poison_status(client: &mut DevClientSession) -> TestResult<bool> {
    Ok(poison_remaining(client).await?.is_some())
}

async fn poison_remaining(client: &mut DevClientSession) -> TestResult<Option<i64>> {
    use oteryn_protocol_oteryn::condition_status as wire;
    let reply = client.session_mut().gated_command(
        wire::CAPABILITY_CONDITION_STATUS_V1, wire::COMMAND_TYPE_CONDITION_STATUS_QUERY, &[],
    ).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step=poison-status observed={reply:?} need=ConditionStatus").into());
    }
    let status = wire::ConditionStatus::decode(&reply.payload)
        .ok_or("step=poison-status observed=invalid-condition-frame")?;
    Ok(poison_duration(&status))
}

fn poison_duration(status: &oteryn_protocol_oteryn::condition_status::ConditionStatus) -> Option<i64> {
    status.conditions.iter().find(|condition| condition.icon == 1).map(|condition| condition.remaining_ms)
}

async fn poison(client: &mut DevClientSession) -> TestResult<(u64, u64)> {
    require_poison_attribution(client)?;
    client.session_mut().attack_target(None).await?;
    client.take_events();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    while !poison_status(client).await? {
        if tokio::time::Instant::now() >= deadline {
            return Err("step=poison observed=no-creature-poison-condition".into());
        }
        client.take_events();
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    wait_poison_tick(client, "poison", None).await
}

fn require_poison_attribution(client: &DevClientSession) -> TestResult {
    use oteryn_protocol_oteryn::analyser::{CAPABILITY_ANALYSER_HIT_ATTRIBUTION_V1, CAPABILITY_ANALYSER_V1};
    let caps = client.selected_capabilities();
    if !caps.contains(&CAPABILITY_ANALYSER_V1) || !caps.contains(&CAPABILITY_ANALYSER_HIT_ATTRIBUTION_V1) {
        return Err("step=poison observed=no-attributed-analyser need=creature-condition-damage-evidence".into());
    }
    Ok(())
}

fn poison_tick_sequence(events: Vec<SessionEvent>, after: u64) -> Option<u64> {
    events.into_iter().find_map(|event| match event {
        SessionEvent::AnalyserAttributedFacts(delta)
            if delta.server_sequence > after && delta.value.iter().any(poison_tick_fact) => Some(delta.server_sequence),
        _ => None,
    })
}

async fn wait_poison_tick(client: &mut DevClientSession, stage: &str, isolated_site: Option<ActorPosition>) -> TestResult<(u64, u64)> {
    let after = client.last_server_sequence();
    let until = tokio::time::Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(site) = isolated_site {
            require_isolated_poison_site(client, site, stage)?;
        }
        if let Some(sequence) = poison_tick_sequence(client.take_events(), after) {
            return Ok((after, sequence));
        }
        if tokio::time::Instant::now() >= until {
            return Err(format!("step={stage} observed=active-icon-without-attributed-tick need=creature-sourced-periodic-earth-damage").into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
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
    let (account, mut client) = character_prep::connect_c08(clients, 0xc8, "Poison Cure Caster")
        .await
        .map_err(|error| admission_error("initial-admission", error))?;
    let dedicated = account.clients(clients);
    travel_to_poison_spider(&mut client, world, &metadata.spawns).await?;
    let damage = poison(&mut client).await?;
    row.pass(damage);

    let before_refusal = client.actor_vitals().copied().ok_or("step=logout-refusal observed=no-vitals")?;
    let initial_refusal = client.session_mut().logout().await?;
    if !matches!(initial_refusal.result, Some(oteryn_protocol_oteryn::logout::LogoutResult::InFight { .. })) || client.actor_vitals() != Some(&before_refusal) {
        return Err(format!("step=logout-refusal observed={initial_refusal:?} need=InFight-during-poison-combat").into());
    }

    let (spell, mana_cost) = antidote(&mut client)?;
    let dispel_site = walk_to_poison_site(&mut client, world, "dispel").await?;
    let sampled = tokio::time::Instant::now();
    let remaining = poison_remaining(&mut client).await?
        .ok_or("step=dispel observed=poison-expired-before-cast need=live-poison-at-isolated-site")?;
    if !poison_survived_window(remaining, sampled.elapsed()) {
        return Err("step=dispel observed=poison-near-natural-expiry-before-cast".into());
    }
    let mana = client
        .actor_vitals()
        .ok_or("step dispel: vitals absent")?
        .mana;
    client.take_events();
    let cast = client.cast_spell(spell, SpellTarget::None, false).await?;
    let clean_until = tokio::time::Instant::now() + Duration::from_secs(15);
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
    if poison_status(&mut client).await? || !poison_survived_window(remaining, sampled.elapsed()) {
        return Err(format!("step=dispel observed=poison-active-or-natural-expiry-possible remaining_ms={remaining} elapsed={:?}", sampled.elapsed()).into());
    }
    loop {
        require_isolated_poison_site(&mut client, dispel_site, "dispel")?;
        if poison_tick_sequence(client.take_events(), cast.result_server_sequence).is_some() {
            return Err("step=dispel observed=post-cast-poison-tick need=immediate-tick-cessation".into());
        }
        if tokio::time::Instant::now() >= clean_until {
            break;
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    if poison_status(&mut client).await?
        || poison_tick_sequence(client.take_events(), cast.result_server_sequence).is_some()
    {
        return Err("step=dispel observed=poison-or-tick-returned-during-observation".into());
    }
    row.pass((cast.disposition, mana, mana_after));
    row.pass(initial_refusal.result);

    let safe = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world.base().tile(at.x, at.y, at.floor).is_some_and(|tile| tile.protection_zone())
    }).ok_or("step=regeneration observed=no-reachable-protection-zone need=retreat-from-spider")?;
    walk_c08(&mut client, world, &safe).await?;
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

    travel_to_poison_spider(&mut client, world, &metadata.spawns).await?;
    let second = poison(&mut client).await?;
    let safe = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world.base().tile(at.x, at.y, at.floor).is_some_and(|tile| tile.protection_zone())
    }).ok_or("step=poison-persistence observed=no-safe-retreat-route")?;
    walk_c08(&mut client, world, &safe).await?;
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
    let mut client = connect_character(&dedicated, 0xc9, 0)
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

pub(super) async fn travel_to_poison_spider(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult<oteryn_dev_client::EntityRef> {
    let from = tile(client.world_spatial().actor_position)?;
    let points: Vec<_> = spawns
        .sources
        .iter()
        .flat_map(|source| {
            source.points.iter().filter_map(|point| {
                let creature = spawns.creatures.get(point.creature as usize)?;
                (creature.key == "oteryn:creature.poison_spider").then_some((
                    source.floor,
                    point.x,
                    point.y,
                ))
            })
        })
        .collect();
    let path = route(world, from, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone())
            && points.iter().any(|&(floor, x, y)| {
                at.floor == floor && at.x.abs_diff(x) <= 1 && at.y.abs_diff(y) <= 1
            })
    })
    .ok_or("no reachable oteryn:creature.poison_spider spawn within town route budget")?;
    walk_c08(client, world, &path).await?;

    let here = client.world_spatial().actor_position;
    let spider = client
        .world_entities()
        .ok_or("C08 visible entities absent")?
        .iter()
        .filter(|actor| {
            actor.kind == oteryn_dev_client::EntityKind::Creature
                && actor.position.floor == here.floor
                && actor
                    .display_name
                    .as_ref()
                    .and_then(|name| name.as_str().ok())
                    .is_some_and(|name| name.eq_ignore_ascii_case("Poison Spider"))
        })
        .min_by_key(|actor| {
            actor
                .position
                .x
                .abs_diff(here.x)
                .max(actor.position.y.abs_diff(here.y))
        })
        .ok_or("C08 authored spawn has no visible Poison Spider")?
        .entity;
    for _ in 0..32 {
        let target = client
            .world_entities()
            .and_then(|entities| entities.get(&spider))
            .ok_or("C08 lost the selected Poison Spider")?
            .position;
        let target = tile(target)?;
        let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
            at.floor == target.floor && at.x.abs_diff(target.x).max(at.y.abs_diff(target.y)) <= 1
        })
        .ok_or("C08 selected Poison Spider unreachable")?;
        let Some(direction) = path.first() else {
            return Ok(spider);
        };
        walk_c08(client, world, &[*direction]).await?;
    }
    Err("C08 could not approach the selected Poison Spider within the route budget".into())
}

pub(super) async fn walk_c08(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    for &direction in path {
        let from = tile(client.world_spatial().actor_position)?;
        let expected = destination(world, from, direction).ok_or("C08 route landing absent")?;
        let outcome = client.step(direction).await?;
        let actual = tile(client.world_spatial().actor_position)?;
        if outcome.disposition != StepDisposition::Moved
            || outcome.world_spatial_delta.is_none()
            || actual != expected
        {
            return Err(format!("C08 step refused or mismatched: from={from:?} direction={direction:?} expected={expected:?} actual={actual:?} outcome={outcome:?}").into());
        }
        verdict::Harness::global().progress("walk");
        client.take_events();
    }
    Ok(())
}

fn creature_can_interfere(client: &DevClientSession, at: ActorPosition) -> bool {
    client.world_entities().is_none_or(|entities| entities.iter().any(|entry| {
        entry.entity.identity != *entities.own_identity()
            && matches!(entry.kind, oteryn_dev_client::EntityKind::Creature | oteryn_dev_client::EntityKind::Player)
            && entry.position.floor == at.floor
            && entry.position.x.abs_diff(at.x).max(entry.position.y.abs_diff(at.y)) <= 8
    }))
}

fn require_isolated_poison_site(client: &mut DevClientSession, site: ActorPosition, stage: &str) -> TestResult {
    let unsafe_actors = creature_can_interfere(client, site);
    let has_ground_only = client.session_mut().world_map_snapshot().is_some_and(|map| {
        map.tiles.iter().any(|tile| tile.position == site && !tile.more && tile.items.len() == 1)
    });
    if unsafe_actors || client.world_spatial().actor_position != site || !has_ground_only {
        return Err(format!("step={stage} observed=unisolated-site need=stationary-bare-ground-without-nearby-actors").into());
    }
    Ok(())
}

async fn walk_to_poison_site(client: &mut DevClientSession, world: &BundleWorld, stage: &str) -> TestResult<ActorPosition> {
    let basis = spatial_basis::current(client)?;
    let safe_site = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world.base().tile(at.x, at.y, at.floor).is_some_and(|tile| !tile.protection_zone())
            && !creature_can_interfere(client, basis.position(at))
    }).ok_or_else(|| format!("step={stage} observed=no-safe-tick-site-outside-pz"))?;
    walk_c08(client, world, &safe_site).await?;
    let site = client.world_spatial().actor_position;
    require_isolated_poison_site(client, site, stage)?;
    Ok(site)
}

async fn resumed_poison_tick(client: &mut DevClientSession, world: &BundleWorld) -> TestResult<(u64, u64)> {
    require_poison_attribution(client)?;
    let site = walk_to_poison_site(client, world, "poison-persistence").await?;
    if !poison_status(client).await? {
        return Err("step=poison-persistence observed=expired-poison need=active-poison-on-bare-ground".into());
    }
    wait_poison_tick(client, "poison-persistence", Some(site)).await
}

pub(super) fn poison_tick_fact(fact: &oteryn_protocol_oteryn::analyser::AttributedFact) -> bool {
    use oteryn_protocol_oteryn::{analyser::{AnalyserFact, AnalyserDamageSource}, damage_element::DamageElement};
    fact.attacker.is_none() && matches!(fact.fact, AnalyserFact::DamageInput {
        element: DamageElement::Earth,
        source: AnalyserDamageSource::Creature(None),
        ..
    })
}

fn poison_survived_window(remaining: i64, elapsed: Duration) -> bool {
    remaining == -1 || u64::try_from(remaining).is_ok_and(|ms| elapsed + Duration::from_millis(1) < Duration::from_millis(ms))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_review_c08_rejects_false_evidence() {
        assert!(poison_survived_window(300, Duration::from_millis(100)));
        assert!(!poison_survived_window(100, Duration::from_millis(100)));
        assert!(!poison_survived_window(1, Duration::from_millis(20)));
    }

    #[test]
    fn dispel_window_detects_residual_tick_in_first_post_cast_batch() {
        use oteryn_dev_client::AppliedDelta;
        use oteryn_protocol_oteryn::{analyser::{AnalyserFact, AnalyserDamageSource, AttributedFact}, damage_element::DamageElement};
        let tick = |server_sequence| SessionEvent::AnalyserAttributedFacts(AppliedDelta {
            server_sequence, base_revision: 0, new_revision: 1,
            value: vec![AttributedFact {
                fact: AnalyserFact::DamageInput {
                    value: NonZeroU32::MIN, element: DamageElement::Earth,
                    source: AnalyserDamageSource::Creature(None),
                },
                attacker: None,
            }],
        });
        assert_eq!(poison_tick_sequence(vec![tick(9), tick(11)], 10), Some(11));
        assert_eq!(poison_tick_sequence(vec![tick(9), tick(10)], 10), None);
    }

    #[test]
    fn independent_review_c08_rejects_unproven_evidence() {
        use oteryn_protocol_oteryn::{analyser::{AnalyserFact, AnalyserDamageSource, AttributedFact}, damage_element::DamageElement};
        let value = std::num::NonZeroU32::MIN;
        let hit = |element, source| AttributedFact { fact: AnalyserFact::DamageInput { value, element, source }, attacker: None };
        assert!(!poison_tick_fact(&hit(DamageElement::Physical, AnalyserDamageSource::Creature(None))));
        assert!(!poison_tick_fact(&hit(DamageElement::Earth, AnalyserDamageSource::None)));
        assert!(poison_tick_fact(&hit(DamageElement::Earth, AnalyserDamageSource::Creature(None))));
    }

    use oteryn_protocol_oteryn::condition_status::{ActiveCondition, ConditionStatus};

    #[test]
    fn burning_does_not_prove_poison() {
        let mut status = ConditionStatus { conditions: vec![ActiveCondition { icon: 2, sub_id: 0, remaining_ms: 1000 }] };
        assert!(poison_duration(&status).is_none());
        status.conditions[0].icon = 1;
        assert!(poison_duration(&status).is_some());
    }
}
