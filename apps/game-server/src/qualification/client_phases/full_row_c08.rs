//! C08 full-row player scenario, derived from the matrix/audit and the native condition owners:
//! 1. Walk from the admitted town to a compiled poison-spider spawn and allow its ordinary combat
//!    AI to apply poison, then require a creature-sourced periodic Earth damage fact.
//! 2. Send the served, available `exana pox` spell command through the native spell protocol;
//!    expect `Cast`, a mana delta, removal of the poison icon and immediate tick cessation.
//! 3. Before dispelling, require the explicit `InFight` logout refusal during active combat.
//! 4. With health below maximum, observe a later positive `ActorVitals` health delta from the
//!    server's ordinary regeneration owner, without changing state through a fixture or database.
//! 5. Return to the poison spider, acquire another delayed poison delta, retreat to
//!    protection, then make one logout request and a fresh admission; expect a new
//!    game-session id, the retained poison schedule, and resumed HP loss without an attacker.
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
    let result = verdict::indexed_timeout(
        Duration::from_secs(720),
        verdict::observe(journey(&mut row, clients)),
    )
    .await;
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
    Ok(poison_observation(client).await?.1)
}

async fn poison_observation(client: &mut DevClientSession) -> TestResult<(u64, Option<i64>)> {
    use oteryn_protocol_oteryn::condition_status as wire;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_CONDITION_STATUS_V1,
            wire::COMMAND_TYPE_CONDITION_STATUS_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step=poison-status observed={reply:?} need=ConditionStatus").into());
    }
    let status = wire::ConditionStatus::decode(&reply.payload)
        .ok_or("step=poison-status observed=invalid-condition-frame")?;
    Ok((reply.server_sequence, poison_duration(&status)))
}

fn report_condition_receipt(client: &DevClientSession, sequence: u64) -> TestResult {
    use std::io::BufRead;

    let path = required("NODE_BOOT_CAST_EVIDENCE_LOG")?;
    let session = format!("session={}", uuid_text(client.join_snapshot().game_session_id.as_bytes()));
    let sequence = format!("server_sequence={sequence}");
    for line in std::io::BufReader::new(std::fs::File::open(path)?).lines() {
        let line = line?;
        let fields = line.split_whitespace();
        if line.starts_with("oteryn-game-server ")
            && fields.clone().any(|field| field == "event=qualification_poison_condition")
            && fields.clone().any(|field| field == session)
            && fields.clone().any(|field| field == sequence)
            && fields.clone().any(|field| field.starts_with("ts="))
            && fields.clone().any(|field| field.starts_with("trace="))
        {
            println!("{line}");
            return Ok(());
        }
    }
    Err(format!("C08 Server condition trace absent for {session} {sequence}").into())
}

fn poison_duration(
    status: &oteryn_protocol_oteryn::condition_status::ConditionStatus,
) -> Option<i64> {
    status
        .conditions
        .iter()
        .find(|condition| condition.icon == 1 && condition.sub_id == 0)
        .map(|condition| condition.remaining_ms)
}

async fn poison(
    client: &mut DevClientSession,
    spider: oteryn_dev_client::EntityRef,
    stage: &str,
) -> TestResult<(u64, u64)> {
    require_poison_attribution(client)?;
    let session = client.join_snapshot().game_session_id;
    let hp = client.actor_vitals().map(|vitals| vitals.health);
    let position = client.world_spatial().actor_position;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let mut first = true;
    let mut direct_damage_sequence = None;
    loop {
        let (sequence, remaining) = poison_observation(client).await?;
        for event in client.take_events() {
            if let SessionEvent::AnalyserAttributedFacts(delta) = event
                && delta.value.iter().any(|fact| {
                    fact.attacker == Some(spider)
                        && matches!(
                            fact.fact,
                            oteryn_protocol_oteryn::analyser::AnalyserFact::DamageInput { .. }
                        )
                })
            {
                direct_damage_sequence.get_or_insert(delta.server_sequence);
            }
        }
        let expired = tokio::time::Instant::now() >= deadline;
        if first || remaining.is_some() || expired {
            report_condition_receipt(client, sequence)?;
            let current_hp = client.actor_vitals().map(|vitals| vitals.health);
            evidence(&format!(
                "C08_POISON_EXPOSURE stage={stage} session={session:?} spider={spider:?} position={position:?} server_sequence={sequence} remaining_ms={remaining:?} direct_damage_sequence={direct_damage_sequence:?} hp={hp:?}->{current_hp:?}"
            ));
            first = false;
        }
        if remaining.is_some() {
            return wait_poison_tick(client, stage).await;
        }
        if expired {
            return Err(format!("step={stage} observed=no-creature-poison-condition server_sequence={sequence} direct_damage_sequence={direct_damage_sequence:?}").into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

fn require_poison_attribution(client: &DevClientSession) -> TestResult {
    use oteryn_protocol_oteryn::analyser::{
        CAPABILITY_ANALYSER_HIT_ATTRIBUTION_V1, CAPABILITY_ANALYSER_V1,
    };
    let caps = client.selected_capabilities();
    if !caps.contains(&CAPABILITY_ANALYSER_V1)
        || !caps.contains(&CAPABILITY_ANALYSER_HIT_ATTRIBUTION_V1)
    {
        return Err(
            "step=poison observed=no-attributed-analyser need=creature-condition-damage-evidence"
                .into(),
        );
    }
    Ok(())
}

fn poison_tick_sequence(events: Vec<SessionEvent>, after: u64) -> Option<u64> {
    events.into_iter().find_map(|event| match event {
        SessionEvent::AnalyserAttributedFacts(delta)
            if delta.server_sequence > after && delta.value.iter().any(poison_tick_fact) =>
        {
            Some(delta.server_sequence)
        }
        _ => None,
    })
}

async fn wait_poison_tick(client: &mut DevClientSession, stage: &str) -> TestResult<(u64, u64)> {
    use oteryn_protocol_oteryn::analyser::AnalyserFact;

    let after = client.last_server_sequence();
    let before_hp = client.actor_vitals().map(|vitals| vitals.health);
    let started = tokio::time::Instant::now();
    let until = started + Duration::from_secs(15);
    let mut sampled_expected_tick = false;
    let mut observed = Vec::new();
    loop {
        let events = client.take_events();
        for event in &events {
            if let SessionEvent::AnalyserAttributedFacts(delta) = event
                && delta.server_sequence > after
            {
                observed.extend(
                    delta
                        .value
                        .iter()
                        .filter(|fact| matches!(fact.fact, AnalyserFact::DamageInput { .. }))
                        .take(8usize.saturating_sub(observed.len()))
                        .cloned(),
                );
            }
        }
        if let Some(sequence) = poison_tick_sequence(events, after) {
            evidence(&format!("C08_POISON_TICK stage={stage} session={:?} condition_server_sequence={after} tick_server_sequence={sequence}", client.join_snapshot().game_session_id));
            return Ok((after, sequence));
        }
        if tokio::time::Instant::now() >= until {
            let after_hp = client.actor_vitals().map(|vitals| vitals.health);
            let position = client.world_spatial().actor_position;
            let (sequence, remaining) = poison_observation(client).await?;
            report_condition_receipt(client, sequence)?;
            return Err(format!(
                "step={stage} observed=active-icon-without-attributed-tick position={position:?} condition_server_sequence={after} final_condition_server_sequence={sequence} hp={before_hp:?}->{after_hp:?} remaining_ms={remaining:?} damage_facts={observed:?} need=creature-sourced-periodic-earth-damage"
            ).into());
        }
        if !sampled_expected_tick && started.elapsed() >= Duration::from_secs(4) {
            let (sequence, remaining) = poison_observation(client).await?;
            report_condition_receipt(client, sequence)?;
            evidence(&format!(
                "C08_EXPECTED_TICK_SAMPLE stage={stage} session={:?} server_sequence={sequence} poison_remaining_ms={remaining:?}",
                client.join_snapshot().game_session_id
            ));
            sampled_expected_tick = true;
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
    if poison_status(&mut client).await? {
        return Err("C08 first exposure started with pre-existing poison".into());
    }
    let spider = travel_to_poison_spider(&mut client, world, &metadata.spawns).await?;
    let damage = poison(&mut client, spider, "first-exposure").await?;
    row.pass(damage);

    let before_refusal = client
        .actor_vitals()
        .copied()
        .ok_or("step=logout-refusal observed=no-vitals")?;
    let position_before_refusal = client.world_spatial().actor_position;
    let session_before_refusal = client.join_snapshot().game_session_id;
    let initial_refusal = client.session_mut().logout().await?;
    if !matches!(initial_refusal.result, Some(oteryn_protocol_oteryn::logout::LogoutResult::InFight { retry_after_ms }) if retry_after_ms > 0)
        || client.actor_vitals() != Some(&before_refusal)
        || client.world_spatial().actor_position != position_before_refusal
        || client.join_snapshot().game_session_id != session_before_refusal
        || !poison_status(&mut client).await?
    {
        return Err(format!(
            "step=logout-refusal observed={initial_refusal:?} need=InFight-during-poison-combat"
        )
        .into());
    }

    let (spell, mana_cost) = antidote(&mut client)?;
    let dispel_site = walk_to_dispel_site(&mut client, world).await?;
    let sampled = tokio::time::Instant::now();
    let remaining = poison_remaining(&mut client).await?.ok_or(
        "step=dispel observed=poison-expired-before-cast need=live-poison-at-isolated-site",
    )?;
    if remaining <= 0 || !poison_survived_window(remaining, sampled.elapsed()) {
        return Err("step=dispel observed=poison-near-natural-expiry-before-cast".into());
    }
    let mana = client
        .actor_vitals()
        .ok_or("step dispel: vitals absent")?
        .mana;
    client.take_events();
    let cast = client.cast_spell(spell, SpellTarget::None, false).await?;
    let clean_until = tokio::time::Instant::now() + Duration::from_secs(15);
    let Some(committed) = cast.actor_vitals_delta.as_ref().filter(|delta| {
        cast.status == CommandStatus::Accepted
            && cast.disposition == SpellCastDisposition::Cast
            && delta.server_sequence > cast.result_server_sequence
            && client.actor_vitals() == Some(&delta.value)
    }) else {
        return Err(format!("step=dispel observed=no-correlated-cast-receipt reply={cast:?}").into());
    };
    let mana_after = committed.value.mana;
    if mana_after.checked_add(mana_cost) != Some(mana) {
        return Err(format!(
            "step dispel: accepted reply missing mana delta {mana}->{mana_after} cost={mana_cost}"
        )
        .into());
    }
    let (cleared_sequence, poison_after_cast) = poison_observation(&mut client).await?;
    if cleared_sequence <= committed.server_sequence || poison_after_cast.is_some() || !poison_survived_window(remaining, sampled.elapsed()) {
        return Err(format!("step=dispel observed=poison-active-or-natural-expiry-possible remaining_ms={remaining} elapsed={:?}", sampled.elapsed()).into());
    }
    loop {
        require_isolated_poison_site(&mut client, dispel_site, "dispel")?;
        if poison_tick_sequence(client.take_events(), cast.result_server_sequence).is_some() {
            return Err(
                "step=dispel observed=post-cast-poison-tick need=immediate-tick-cessation".into(),
            );
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
    evidence(&format!(
        "C08_RETREAT_PRESTEP session={:?} cast_server_sequence={} vitals_server_sequence={} cleared_server_sequence={cleared_sequence} last_server_sequence={}",
        client.join_snapshot().game_session_id,
        cast.result_server_sequence,
        committed.server_sequence,
        client.last_server_sequence()
    ));

    let safe = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| tile.protection_zone())
    })
    .ok_or("step=regeneration observed=no-reachable-protection-zone need=retreat-from-spider")?;
    walk_c08(&mut client, world, &safe).await?;
    let damaged = client
        .actor_vitals()
        .ok_or("step regeneration: vitals absent")?
        .health;
    let regenerated = next_health_change(&mut client, damaged, 45).await?;
    if damaged.checked_add(1) != Some(regenerated) {
        return Err(format!(
            "step regeneration: expected exactly one health point, got {damaged}->{regenerated}"
        )
        .into());
    }
    row.pass((damaged, regenerated));

    if poison_status(&mut client).await? {
        return Err("C08 second exposure started with pre-existing poison".into());
    }
    let aggressor = travel_to_poison_spider(&mut client, world, &metadata.spawns).await?;
    let second = poison(&mut client, aggressor, "second-exposure").await?;
    let safe = route(world, tile(client.world_spatial().actor_position)?, |at| {
        is_logout_rest_tile(world, at)
    })
    .ok_or("step=poison-persistence observed=no-safe-retreat-route")?;
    walk_c08(&mut client, world, &safe).await?;
    let old_session = client.join_snapshot().game_session_id;
    let saved_position = client.world_spatial().actor_position;
    if tile(saved_position)? == TOWN {
        return Err("C08 persistence logout position equals the default town entry".into());
    }
    wait_for_c08_logout(&mut client, Some(aggressor)).await?;
    let in_fight = client
        .session_mut()
        .combat_state()
        .ok_or("C08 combat state absent")?
        .in_fight;
    let sampled = tokio::time::Instant::now();
    let (saved_sequence, saved_remaining) = poison_observation(&mut client).await?;
    report_condition_receipt(&client, saved_sequence)?;
    let saved_remaining = saved_remaining
        .filter(|remaining| *remaining > 0)
        .ok_or("step=poison-persistence observed=no-finite-live-poison-at-save")?;
    evidence(&format!(
        "C08_PERSISTENCE_PRELOGOUT position={saved_position:?} remaining_ms={saved_remaining} in_fight={in_fight}"
    ));
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=poison-persistence first logout refused: {reply:?}").into());
    }
    drop(client);
    let mut client = connect_character(&dedicated, 0xc9, 0)
        .await
        .map_err(|error| admission_error("fresh-admission", error))?;
    if client.join_snapshot().game_session_id == old_session {
        return Err("step persistence: fresh admission reused game session".into());
    }
    if client.world_spatial().actor_position != saved_position {
        return Err("step=poison-persistence observed=fresh-admission-position-mismatch".into());
    }
    let (restored_sequence, restored) = poison_observation(&mut client).await?;
    report_condition_receipt(&client, restored_sequence)?;
    let restored = restored
        .ok_or("step=poison-persistence observed=missing-poison-icon-on-fresh-admission need=saved-live-condition")?;
    if !retained_duration(saved_remaining, restored, sampled.elapsed()) {
        return Err(format!("step=poison-persistence remaining-at-save={saved_remaining} restored={restored} elapsed={:?}", sampled.elapsed()).into());
    }
    let tick = resumed_poison_tick(&mut client, world).await?;
    wait_for_c08_logout(&mut client, None).await?;
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=poison-persistence final logout refused: {reply:?}").into());
    }
    row.pass((second, tick, saved_position, saved_remaining, restored));
    Ok(())
}

fn retained_duration(before: i64, restored: i64, elapsed: Duration) -> bool {
    restored > 0
        && restored <= before
        && (before - restored) as u128 <= elapsed.as_millis().saturating_add(2)
}

pub(super) async fn wait_for_c08_logout(
    client: &mut DevClientSession,
    aggressor: Option<oteryn_dev_client::EntityRef>,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    loop {
        let combat = client
            .session_mut()
            .combat_state()
            .ok_or("C08 combat state absent")?;
        let clear = !combat.in_fight && combat.target.is_none();
        let position = client.world_spatial().actor_position;
        let entities = client
            .world_entities()
            .ok_or("C08 logout visibility absent")?;
        let aggressor_near = aggressor
            .and_then(|id| entities.get(&id))
            .is_some_and(|actor| {
                actor.position.floor == position.floor
                    && actor
                        .position
                        .x
                        .abs_diff(position.x)
                        .max(actor.position.y.abs_diff(position.y))
                        <= 8
            });
        if clear && !aggressor_near && !creature_can_interfere(client, position) {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("C08 combat state did not clear before first logout".into());
        }
        client.service_liveness(Duration::from_millis(250)).await?;
        client.take_events();
    }
}

pub(super) async fn travel_to_poison_spider(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult<oteryn_dev_client::EntityRef> {
    const SPAWN: TilePos = TilePos {
        x: 32463,
        y: 32226,
        floor: -7,
    };
    const APPROACH: TilePos = TilePos {
        x: 32464,
        y: 32225,
        floor: -7,
    };
    let authored = spawns.sources.iter().any(|source| {
        source.floor == SPAWN.floor
            && source.points.iter().any(|point| {
                point.x == SPAWN.x
                    && point.y == SPAWN.y
                    && spawns
                        .creatures
                        .get(point.creature as usize)
                        .is_some_and(|creature| creature.key == "oteryn:creature.poison_spider")
            })
    });
    if !authored {
        return Err("C08 authored Poison Spider spawn missing".into());
    }
    if !world
        .base()
        .tile(APPROACH.x, APPROACH.y, APPROACH.floor)
        .is_some_and(|tile| !tile.protection_zone())
    {
        return Err("C08 authored encounter approach missing or protected".into());
    }
    let from = tile(client.world_spatial().actor_position)?;
    let occupied = c08_visible_occupants(client);
    let path = route_avoiding(
        world,
        from,
        |at| at == APPROACH,
        |at| !occupied.contains(&at),
    )
    .ok_or("C08 authored Poison Spider approach is unreachable within town route budget")?;
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
    // Let this exact aggressive creature close the distance over the wire.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    loop {
        let target = client
            .world_entities()
            .and_then(|entities| entities.get(&spider))
            .ok_or("C08 lost the selected Poison Spider")?
            .position;
        if client.world_spatial().actor_position != here {
            return Err("C08 actor moved while waiting for the Poison Spider".into());
        }
        if target.floor == here.floor
            && target.x.abs_diff(here.x).max(target.y.abs_diff(here.y)) == 1
        {
            return Ok(spider);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "C08 selected Poison Spider did not approach: player={here:?} spider={target:?}"
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
}

fn c08_visible_occupants(client: &DevClientSession) -> std::collections::BTreeSet<TilePos> {
    client
        .world_entities()
        .into_iter()
        .flat_map(|entities| entities.others())
        .filter(|actor| {
            matches!(
                actor.kind,
                oteryn_dev_client::EntityKind::Creature
                    | oteryn_dev_client::EntityKind::Npc
                    | oteryn_dev_client::EntityKind::Player
            )
        })
        .filter_map(|actor| tile(actor.position).ok())
        .collect()
}

pub(super) async fn walk_c08(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    for &direction in path {
        let from = tile(client.world_spatial().actor_position)?;
        let expected = destination(world, from, direction).ok_or("C08 route landing absent")?;
        let mut outcome = client.step(direction).await?;
        if outcome.disposition == StepDisposition::Blocked {
            let occupant = client
                .world_entities()
                .and_then(|entities| {
                    entities
                        .others()
                        .find(|actor| tile(actor.position).ok() == Some(expected))
                })
                .ok_or_else(|| {
                    format!(
                        "BLOCKER-SERVER C08 blocked unoccupied landing {expected:?}: {outcome:?}"
                    )
                })?;
            if occupant.kind != oteryn_dev_client::EntityKind::Creature {
                return Err(
                    format!("C08 blocked by non-creature at {expected:?}: {occupant:?}").into(),
                );
            }
            let id = occupant.entity;
            evidence(&format!(
                "C08_MOVEMENT_BLOCKED occupant={id:?} landing={expected:?} first={outcome:?}"
            ));
            let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
            while client
                .world_entities()
                .ok_or("C08 movement visibility absent")?
                .get(&id)
                .is_some_and(|actor| tile(actor.position).ok() == Some(expected))
            {
                if tokio::time::Instant::now() >= deadline {
                    return Err(format!("C08 creature {id:?} did not leave {expected:?}").into());
                }
                client.service_liveness(Duration::from_millis(50)).await?;
                client.take_events();
            }
            if tile(client.world_spatial().actor_position)? != from
                || c08_visible_occupants(client).contains(&expected)
            {
                return Err(format!(
                    "C08 blocked movement changed before retry: {from:?} -> {expected:?}"
                )
                .into());
            }
            evidence(&format!(
                "C08_MOVEMENT_CLEARED occupant={id:?} landing={expected:?}"
            ));
            outcome = client.step(direction).await?;
            if outcome.disposition == StepDisposition::Blocked
                && !c08_visible_occupants(client).contains(&expected)
            {
                return Err(format!(
                    "BLOCKER-SERVER C08 blocked unoccupied landing {expected:?}: {outcome:?}"
                )
                .into());
            }
        }
        let actual = tile(client.world_spatial().actor_position)?;
        if outcome.disposition != StepDisposition::Moved
            || outcome.world_spatial_delta.is_none()
            || actual != expected
        {
            return Err(format!("C08 step refused or mismatched: from={from:?} direction={direction:?} expected={expected:?} actual={actual:?} occupied={} outcome={outcome:?}", c08_visible_occupants(client).contains(&expected)).into());
        }
        verdict::Harness::global().progress("walk");
        client.take_events();
    }
    Ok(())
}

fn creature_can_interfere(client: &DevClientSession, at: ActorPosition) -> bool {
    client.world_entities().is_none_or(|entities| {
        entities.iter().any(|entry| {
            entry.entity.identity != *entities.own_identity()
                && matches!(
                    entry.kind,
                    oteryn_dev_client::EntityKind::Creature | oteryn_dev_client::EntityKind::Player
                )
                && entry.position.floor == at.floor
                && entry
                    .position
                    .x
                    .abs_diff(at.x)
                    .max(entry.position.y.abs_diff(at.y))
                    <= 8
        })
    })
}

fn require_isolated_poison_site(
    client: &mut DevClientSession,
    site: ActorPosition,
    stage: &str,
) -> TestResult {
    let unsafe_actors = creature_can_interfere(client, site);
    let actual = client.world_spatial().actor_position;
    let map = client.session_mut().world_map_snapshot();
    let stack = map
        .as_ref()
        .and_then(|map| map.tiles.iter().find(|tile| tile.position == site))
        .map(|tile| (tile.items.len(), tile.more));
    if unsafe_actors || actual != site || stack != Some((1, false)) {
        return Err(format!(
            "step={stage} observed=unisolated-site site={site:?} actual={actual:?} unsafe_actors={unsafe_actors} stack={stack:?} need=stationary-bare-ground-without-nearby-actors"
        ).into());
    }
    Ok(())
}

async fn walk_to_dispel_site(
    client: &mut DevClientSession,
    world: &BundleWorld,
) -> TestResult<ActorPosition> {
    const DISPEL_SITE: TilePos = TilePos {
        x: 32452,
        y: 32225,
        floor: -7,
    };
    let basis = spatial_basis::current(client)?;
    let site = basis.position(DISPEL_SITE);
    if !world
        .base()
        .tile(DISPEL_SITE.x, DISPEL_SITE.y, DISPEL_SITE.floor)
        .is_some_and(|tile| {
            !tile.protection_zone() && tile.ground().is_some() && tile.ids().len() == 1
        })
    {
        return Err("C08 authored dispel site is not bare ground outside protection".into());
    }
    let from = tile(client.world_spatial().actor_position)?;
    let occupied = c08_visible_occupants(client);
    let path = route_avoiding(
        world,
        from,
        |at| at == DISPEL_SITE,
        |at| !occupied.contains(&at),
    )
    .ok_or("C08 authored dispel site is unreachable")?;
    evidence(&format!(
        "C08_SITE stage=dispel site={site:?} steps={}",
        path.len()
    ));
    walk_c08(client, world, &path).await?;
    require_isolated_poison_site(client, site, "dispel")?;
    Ok(site)
}

async fn walk_to_poison_site(
    client: &mut DevClientSession,
    world: &BundleWorld,
    stage: &str,
) -> TestResult<ActorPosition> {
    let basis = spatial_basis::current(client)?;
    let from = tile(client.world_spatial().actor_position)?;
    let path = route(world, from, |at| {
        world.base().tile(at.x, at.y, at.floor).is_some_and(|tile| {
            !tile.protection_zone() && tile.ground().is_some() && tile.ids().len() == 1
        }) && !creature_can_interfere(client, basis.position(at))
    })
    .ok_or_else(|| format!("step={stage} observed=no-safe-tick-site-outside-pz"))?;
    let planned = path
        .iter()
        .try_fold(from, |at, &direction| destination(world, at, direction))
        .ok_or("C08 planned poison site missing")?;
    let site = basis.position(planned);
    evidence(&format!(
        "C08_SITE stage={stage} site={site:?} steps={}",
        path.len()
    ));
    walk_c08(client, world, &path).await?;
    require_isolated_poison_site(client, site, stage)?;
    Ok(site)
}

async fn resumed_poison_tick(
    client: &mut DevClientSession,
    world: &BundleWorld,
) -> TestResult<(u64, u32, u32)> {
    require_poison_attribution(client)?;
    let site = walk_to_poison_site(client, world, "poison-persistence").await?;
    let (sequence, remaining) = poison_observation(client).await?;
    report_condition_receipt(client, sequence)?;
    if remaining.is_none() {
        return Err(
            "step=poison-persistence observed=expired-poison need=active-poison-on-bare-ground"
                .into(),
        );
    }
    let mut health = client
        .actor_vitals()
        .ok_or("C08 restored vitals absent")?
        .health;
    client.take_events();
    let after = client.last_server_sequence();
    let started = tokio::time::Instant::now();
    let deadline = started + Duration::from_secs(15);
    let mut sampled_expected_tick = false;
    let mut damage = None;
    let mut health_loss = None;
    loop {
        require_isolated_poison_site(client, site, "poison-persistence")?;
        for event in client.take_events() {
            match event {
                SessionEvent::ActorVitals(delta) if delta.server_sequence > after => {
                    if delta.value.health < health
                        && health_loss.replace((health, delta.value.health)).is_some()
                    {
                        return Err(
                            "C08 multiple HP decreases before restored poison attribution".into(),
                        );
                    }
                    health = delta.value.health;
                }
                SessionEvent::AnalyserAttributedFacts(delta) if delta.server_sequence > after => {
                    for fact in &delta.value {
                        if let Some(amount) = restored_poison_amount(fact)
                            && damage.replace((delta.server_sequence, amount)).is_some()
                        {
                            return Err(
                                "C08 multiple restored poison facts before HP verification".into(),
                            );
                        }
                    }
                }
                _ => {}
            }
        }
        if let (Some((sequence, amount)), Some((before, after))) = (damage, health_loss) {
            if before.checked_sub(amount) != Some(after) {
                return Err(format!(
                    "C08 restored poison amount {amount} did not match HP {before}->{after}"
                )
                .into());
            }
            return Ok((sequence, before, after));
        }
        if tokio::time::Instant::now() >= deadline {
            let (sequence, _) = poison_observation(client).await?;
            report_condition_receipt(client, sequence)?;
            return Err(format!("C08 restored poison did not yield matching unowned Earth damage and HP change: damage={damage:?} health_loss={health_loss:?}").into());
        }
        if !sampled_expected_tick && started.elapsed() >= Duration::from_secs(4) {
            let (sequence, _) = poison_observation(client).await?;
            report_condition_receipt(client, sequence)?;
            sampled_expected_tick = true;
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

fn restored_poison_amount(fact: &oteryn_protocol_oteryn::analyser::AttributedFact) -> Option<u32> {
    use oteryn_protocol_oteryn::{
        analyser::{AnalyserDamageSource, AnalyserFact},
        damage_element::DamageElement,
    };
    // Crystal serializes the damage list and leaves out ConditionDamage::owner.
    let AnalyserFact::DamageInput {
        value,
        element: DamageElement::Earth,
        source: AnalyserDamageSource::None,
    } = fact.fact
    else {
        return None;
    };
    fact.attacker.is_none().then_some(value.get())
}

pub(super) fn poison_tick_fact(fact: &oteryn_protocol_oteryn::analyser::AttributedFact) -> bool {
    use oteryn_protocol_oteryn::{
        analyser::{AnalyserDamageSource, AnalyserFact},
        damage_element::DamageElement,
    };
    fact.attacker.is_none()
        && matches!(
            fact.fact,
            AnalyserFact::DamageInput {
                element: DamageElement::Earth,
                source: AnalyserDamageSource::Creature(None),
                ..
            }
        )
}

fn poison_survived_window(remaining: i64, elapsed: Duration) -> bool {
    remaining == -1
        || u64::try_from(remaining)
            .is_ok_and(|ms| elapsed + Duration::from_millis(1) < Duration::from_millis(ms))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restored_poison_requires_earth_and_no_live_source() {
        use oteryn_protocol_oteryn::{
            analyser::{AnalyserDamageSource, AnalyserFact, AttributedFact},
            damage_element::DamageElement,
        };
        let fact = |element, source, attacker| AttributedFact {
            fact: AnalyserFact::DamageInput {
                value: NonZeroU32::MIN,
                element,
                source,
            },
            attacker,
        };
        assert_eq!(
            restored_poison_amount(&fact(
                DamageElement::Earth,
                AnalyserDamageSource::None,
                None
            )),
            Some(1)
        );
        assert_eq!(
            restored_poison_amount(&fact(
                DamageElement::Earth,
                AnalyserDamageSource::Creature(None),
                None
            )),
            None
        );
        assert_eq!(
            restored_poison_amount(&fact(
                DamageElement::Physical,
                AnalyserDamageSource::None,
                None
            )),
            None
        );
        let source = oteryn_dev_client::EntityRef {
            identity: [1; 16],
            generation: 1,
        };
        assert_eq!(
            restored_poison_amount(&fact(
                DamageElement::Earth,
                AnalyserDamageSource::None,
                Some(source)
            )),
            None
        );
    }

    #[test]
    fn retained_poison_duration_rejects_reset_expiry_and_excess_time_loss() {
        let elapsed = Duration::from_millis(100);
        assert!(retained_duration(1_000, 1_000, elapsed));
        assert!(retained_duration(1_000, 900, elapsed));
        assert!(!retained_duration(1_000, 1_001, elapsed));
        assert!(!retained_duration(1_000, 800, elapsed));
        assert!(!retained_duration(1_000, 0, elapsed));
        assert!(!retained_duration(1_000, -1, elapsed));
        assert!(!retained_duration(-1, -1, elapsed));
        assert!(!retained_duration(-1, 1_000, elapsed));
    }

    #[test]
    fn second_review_c08_rejects_false_evidence() {
        assert!(poison_survived_window(300, Duration::from_millis(100)));
        assert!(!poison_survived_window(100, Duration::from_millis(100)));
        assert!(!poison_survived_window(1, Duration::from_millis(20)));
    }

    #[test]
    fn dispel_window_detects_residual_tick_in_first_post_cast_batch() {
        use oteryn_dev_client::AppliedDelta;
        use oteryn_protocol_oteryn::{
            analyser::{AnalyserDamageSource, AnalyserFact, AttributedFact},
            damage_element::DamageElement,
        };
        let tick = |server_sequence| {
            SessionEvent::AnalyserAttributedFacts(AppliedDelta {
                server_sequence,
                base_revision: 0,
                new_revision: 1,
                value: vec![AttributedFact {
                    fact: AnalyserFact::DamageInput {
                        value: NonZeroU32::MIN,
                        element: DamageElement::Earth,
                        source: AnalyserDamageSource::Creature(None),
                    },
                    attacker: None,
                }],
            })
        };
        assert_eq!(poison_tick_sequence(vec![tick(9), tick(11)], 10), Some(11));
        assert_eq!(poison_tick_sequence(vec![tick(9), tick(10)], 10), None);
    }

    #[test]
    fn independent_review_c08_rejects_unproven_evidence() {
        use oteryn_protocol_oteryn::{
            analyser::{AnalyserDamageSource, AnalyserFact, AttributedFact},
            damage_element::DamageElement,
        };
        let value = std::num::NonZeroU32::MIN;
        let hit = |element, source| AttributedFact {
            fact: AnalyserFact::DamageInput {
                value,
                element,
                source,
            },
            attacker: None,
        };
        assert!(!poison_tick_fact(&hit(
            DamageElement::Physical,
            AnalyserDamageSource::Creature(None)
        )));
        assert!(!poison_tick_fact(&hit(
            DamageElement::Earth,
            AnalyserDamageSource::None
        )));
        assert!(poison_tick_fact(&hit(
            DamageElement::Earth,
            AnalyserDamageSource::Creature(None)
        )));
    }

    use oteryn_protocol_oteryn::condition_status::{ActiveCondition, ConditionStatus};

    #[test]
    fn burning_does_not_prove_poison() {
        let mut status = ConditionStatus {
            conditions: vec![ActiveCondition {
                icon: 2,
                sub_id: 0,
                remaining_ms: 1000,
            }],
        };
        assert!(poison_duration(&status).is_none());
        status.conditions[0].icon = 1;
        assert!(poison_duration(&status).is_some());
    }
}
