//! Qualify poison and its cure using a live journey and the original inbound Server frames.
use super::full_row_c08::{
    poison_observation, poison_status, poison_tick_fact, poison_tick_sequence, report_condition_receipt, require_isolated_poison_site, travel_to_poison_spider, wait_for_c08_logout, walk_c08, walk_to_dispel_site,
};
use super::*;
use oteryn_dev_client::{CLIENT_SUPPORTED_CAPABILITIES, DevClientSession, SessionEvent};
use oteryn_dev_client::native_session::{Admission, test_support::{capture_inbound, replay_vitals}};
use oteryn_protocol_oteryn::{CharacterId, GameSessionId};
use oteryn_protocol_oteryn::actor_spell::{
    ActorVitals, SpellCastDisposition, SpellTarget,
};
use oteryn_protocol_oteryn::actor_spell_catalog as catalogue;
use oteryn_protocol_oteryn::{analyser::{AnalyserDamageSource, AnalyserFact, AttributedFact}, damage_element::DamageElement};

const CURE_MANA: u32 = 30;
const POISON_INTERVAL_US: u64 = 4_000_000;

struct Replay {
    account: character_prep::DedicatedCharacter,
    saved_position: oteryn_protocol_oteryn::world_spatial::ActorPosition,
    session: GameSessionId,
    checkpoints: [(u64, ActorVitals); 5],
    observed: String,
}

/// The next health decrease must match the attributed hit or condition tick.
async fn next_damage(
    client: &mut DevClientSession,
    above: u32,
    seconds: u64,
    spider: oteryn_dev_client::EntityRef,
    periodic: bool,
) -> TestResult<(u64, ActorVitals, Vec<u32>)> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(seconds);
    let mut observed: Option<(u64, ActorVitals)> = None;
    let mut attributed_damage = 0u32;
    let mut poison_amounts = Vec::new();
    loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        if client.world_entities().and_then(|entities| entities.get(&spider)).is_none() {
            return Err("C08 authored spider generation left the damage observation".into());
        }
        for event in client.take_events() {
            match event {
                SessionEvent::ActorVitals(delta) if observed.is_some() || delta.value.health < above => {
                    if observed.is_some_and(|(_, vitals)| vitals.health < delta.value.health) {
                        return Err("C08 health recovered during the damage observation".into());
                    }
                    observed = Some((delta.server_sequence, delta.value));
                }
                SessionEvent::AnalyserAttributedFacts(delta) => {
                    for fact in &delta.value {
                        if let AnalyserFact::DamageInput { value, .. } = fact.fact {
                            if damage_kind(fact, spider, periodic)? {
                                poison_amounts.push(value.get());
                            }
                            attributed_damage = attributed_damage
                                .checked_add(value.get())
                                .ok_or("C08 damage attribution overflow")?;
                        }
                    }
                }
                _ => {}
            }
        }
        if let Some((sequence, vitals)) = observed {
            let health_loss = above - vitals.health;
            if attributed_damage > health_loss {
                return Err(format!(
                    "C08 attributed damage {attributed_damage} exceeded health loss {above}->{}",
                    vitals.health
                )
                .into());
            }
            if (if periodic { poison_amounts.len() >= 2 } else { attributed_damage > 0 }) && attributed_damage == health_loss {
                return Ok((sequence, vitals, poison_amounts));
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("no attributed C08 damage below health {above}").into());
        }
    }
}

fn damage_kind(fact: &AttributedFact, spider: oteryn_dev_client::EntityRef, periodic: bool) -> TestResult<bool> {
    let AnalyserFact::DamageInput { value, element, source } = fact.fact else {
        return Err("C08 expected a damage fact".into());
    };
    if fact.attacker == Some(spider) && element == DamageElement::Physical
        && matches!(source, AnalyserDamageSource::Creature(_)) && value.get() <= 20 {
        return Ok(false);
    }
    if periodic && poison_tick_fact(fact) && (1..=2).contains(&value.get()) {
        return Ok(true);
    }
    Err(format!("C08 unexpected damage attribution or magnitude: {fact:?}").into())
}

fn verify_tick_receipts(client: &DevClientSession, spider: oteryn_dev_client::EntityRef, amounts: &[u32]) -> TestResult {
    use std::io::BufRead;
    let session = format!("session={}", uuid_text(client.join_snapshot().game_session_id.as_bytes()));
    let source = format!("source=creature:{}", spider.identity.iter().map(|byte| format!("{byte:02x}")).collect::<String>());
    let log = std::fs::File::open(required("NODE_BOOT_CAST_EVIDENCE_LOG")?)?;
    let mut ticks = Vec::new();
    for line in std::io::BufReader::new(log).lines() {
        let line = line?;
        if let Some(tick) = read_tick_receipt(&line, &session, &source)? {
            evidence(&format!("C08_CLIENT_TICK_RECEIPT {line}"));
            ticks.push(tick);
        }
    }
    verify_tick_series(&ticks, amounts)
}

fn read_tick_receipt(line: &str, session: &str, source: &str) -> TestResult<Option<(u64, u32)>> {
    let mut tokens = line.split_ascii_whitespace();
    if !tokens.clone().any(|token| token == "event=qualification_poison_tick")
        || !tokens.clone().any(|token| token == session)
    {
        return Ok(None);
    }
    if !tokens.clone().any(|token| token == source) {
        return Err(format!("C08 poison came from a different source: {line}").into());
    }
    let due = tokens.clone().find_map(|token| token.strip_prefix("due_us="))
        .ok_or("C08 tick deadline absent")?.parse()?;
    let amount = tokens.find_map(|token| token.strip_prefix("amount="))
        .ok_or("C08 tick amount absent")?.parse()?;
    Ok(Some((due, amount)))
}

fn verify_tick_series(ticks: &[(u64, u32)], amounts: &[u32]) -> TestResult {
    if ticks.len() < 2 || !ticks.iter().map(|&(_, amount)| amount).eq(amounts.iter().copied())
        || !ticks.windows(2).all(|pair| pair[1].0.checked_sub(pair[0].0) == Some(POISON_INTERVAL_US)
            && pair[1].1 <= pair[0].1)
    {
        return Err(format!("C08 poison tick timing or amounts disagreed: owner={ticks:?} wire={amounts:?}").into());
    }
    Ok(())
}

fn verify_cure_vitals(before: ActorVitals, after: ActorVitals, advertised_mana: u32) -> TestResult {
    if advertised_mana != CURE_MANA || after.mana.checked_add(CURE_MANA) != Some(before.mana)
        || before.health != after.health {
        return Err(format!("C08 cure vitals or Global cost disagreed: before={before:?} after={after:?} advertised={advertised_mana}").into());
    }
    Ok(())
}

fn verify_cure_window(remaining: Option<i64>, sampled: u64, cleared: u64, post: Option<i64>) -> TestResult {
    if post.is_some()
        || !remaining.and_then(|remaining| u64::try_from(remaining).ok())
            .zip(cleared.checked_sub(sampled))
            .is_some_and(|(remaining, elapsed)| remaining > elapsed)
    {
        return Err(format!("C08 cure cannot exclude natural expiry: remaining={remaining:?} sampled={sampled} cleared={cleared} post={post:?}").into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<Replay> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let (account, mut client) =
        character_prep::connect_c08(clients, 0x30, "Poison Replay Caster").await?;
    let starting_position = client.world_spatial().actor_position;
    if poison_status(&mut client).await? {
        return Err("C08 initial character already poisoned".into());
    }
    let spider = travel_to_poison_spider(&mut client, world, &metadata.spawns).await?;
    if poison_status(&mut client).await? {
        return Err("C08 exposure started with pre-existing poison".into());
    }
    let before = *client.actor_vitals().ok_or("C08 initial vitals absent")?;
    let before_sequence = client.last_server_sequence();
    client.take_events();
    let (direct_sequence, direct, _) =
        next_damage(&mut client, before.health, 30, spider, false).await?;
    let (delayed_sequence, delayed, poison_amounts) =
        next_damage(&mut client, direct.health, 15, spider, true).await?;
    verify_tick_receipts(&client, spider, &poison_amounts)?;
    if !poison_status(&mut client).await? {
        return Err("C08 Poison Spider damage did not leave an active poison condition".into());
    }

    let snapshot = client
        .session_mut()
        .gated_snapshot(catalogue::STATE_DOMAIN_ACTOR_SPELL_CATALOG)
        .ok_or("C08 spell catalogue absent")?;
    let entry = catalogue::decode_catalog(&snapshot.payload)
        .map_err(|error| format!("C08 catalogue decode: {error:?}"))?
        .into_iter()
        .find(|entry| entry.words == "exana pox" && entry.available)
        .ok_or("C08 exana pox not served as available")?;
    let site = walk_to_dispel_site(&mut client, world).await?;
    let (active_sequence, remaining) = poison_observation(&mut client).await?;
    let active_ts = report_condition_receipt(&client, active_sequence)?;
    if !remaining.is_some_and(|remaining| remaining > 0) || entry.mana != CURE_MANA {
        return Err("C08 cure precondition failed: finite poison and Global 30 mana required".into());
    }
    let pre_cast = *client.actor_vitals().ok_or("C08 pre-cast vitals absent")?;
    let pre_cast_sequence = client.last_server_sequence();
    client.take_events();
    let cast = client.cast_spell(
        std::num::NonZeroU32::new(entry.cast_index).ok_or("C08 zero cast index")?,
        SpellTarget::None,
        false,
    ).await?;
    if cast.disposition != SpellCastDisposition::Cast || cast.status != CommandStatus::Accepted {
        return Err(format!("C08 exana pox refused: {cast:?}").into());
    }
    let dispel = cast.actor_vitals_delta.ok_or("C08 cure vitals delta absent")?;
    let dispel_sequence = dispel.server_sequence;
    let dispelled = dispel.value;
    verify_cure_vitals(pre_cast, dispelled, entry.mana)?;
    let (clear_sequence, clear) = poison_observation(&mut client).await?;
    if clear_sequence <= dispel_sequence || dispel_sequence <= cast.result_server_sequence {
        return Err("C08 cure receipt sequence is out of order".into());
    }
    let clear_ts = report_condition_receipt(&client, clear_sequence)?;
    verify_cure_window(remaining, active_ts, clear_ts, clear)?;
    loop {
        require_isolated_poison_site(&mut client, site, "client-post-cure")?;
        if poison_tick_sequence(client.take_events(), cast.result_server_sequence).is_some() {
            return Err("C08 residual poison tick after cast".into());
        }
        let (sequence, poison) = poison_observation(&mut client).await?;
        if poison.is_some() {
            return Err("C08 poison returned after the cure".into());
        }
        if report_condition_receipt(&client, sequence)?.saturating_sub(clear_ts) >= 8_000 {
            if poison_tick_sequence(client.take_events(), cast.result_server_sequence).is_some() {
                return Err("C08 residual poison tick at cure observation end".into());
            }
            break;
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    let session = client.join_snapshot().game_session_id;
    let safe = route(world, tile(client.world_spatial().actor_position)?, |at| {
        is_logout_rest_tile(world, at)
    })
    .ok_or("C08 replay cleanup has no reachable logout tile")?;
    walk_c08(&mut client, world, &safe).await?;

    wait_for_c08_logout(&mut client, Some(spider)).await?;
    let saved_position = client.world_spatial().actor_position;
    if saved_position == starting_position || poison_status(&mut client).await? {
        return Err("C08 pre-logout position or cured state did not discriminate".into());
    }
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("C08 cleanup logout refused: {reply:?}").into());
    }
    Ok(Replay {
        account,
        saved_position,
        session,
        checkpoints: [
            (before_sequence, before),
            (direct_sequence, direct),
            (delayed_sequence, delayed),
            (pre_cast_sequence, pre_cast),
            (dispel_sequence, dispelled),
        ],
        observed: format!(
            "health={}>{}>{} mana={}>{}",
            before.health, direct.health, delayed.health, pre_cast.mana, dispelled.mana
        ),
    })
}

async fn qualify(clients: &SeamClients<'_>) -> TestResult<String> {
    let directory = std::env::var_os("E2E_C08_CAPTURE_DIR").ok_or("C08 capture directory absent")?;
    let (result, frames) = capture_inbound(journey(clients)).await;
    let replay = result?;
    let path = Path::new(&directory).join("c08-inbound.bin");
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&path)?;
    std::io::Write::write_all(&mut file, &frames)?;
    drop(file);
    let dedicated = replay.account.clients(clients);
    verify_cure_vitals(replay.checkpoints[3].1, replay.checkpoints[4].1, CURE_MANA)?;
    replay_vitals(
        &std::fs::read(path)?,
        Admission {
            appearance_items: None,
            schema_revision: 1,
            character_id: CharacterId::decode(&dedicated.characters[0])?,
            admission_material: &[1],
            client_build_id: "c08-offline-replay",
            supported_capabilities: CLIENT_SUPPORTED_CAPABILITIES,
            deadline: Duration::from_secs(5),
        },
        replay.session,
        &replay.checkpoints,
    )
    .await
    .map_err(|error| format!("C08 original-frame replay: {error}"))?;
    let mut fresh = connect_character(&dedicated, 0x31, 0).await?;
    if fresh.join_snapshot().game_session_id == replay.session
        || fresh.world_spatial().actor_position != replay.saved_position
        || poison_status(&mut fresh).await?
    {
        return Err("C08 relog did not preserve the cured state and exact retreat position".into());
    }
    wait_for_c08_logout(&mut fresh, None).await?;
    let reply = fresh.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("C08 relog cleanup refused: {reply:?}").into());
    }
    Ok(format!("{} relog=verified replay=original-server-frames", replay.observed))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    character_prep::declare_seeded("C08.client", "level-10-sorcerer", "Cure Poison eligibility");
    let started = std::time::Instant::now();
    match verdict::indexed_timeout(Duration::from_secs(420), verdict::observe(qualify(clients))).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=C08.client status=PASS seconds={:.3} evidence={observed}",
            started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=C08.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=C08.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spider(byte: u8) -> oteryn_dev_client::EntityRef {
        oteryn_dev_client::EntityRef {
            identity: [byte; 16],
            generation: 1,
        }
    }

    fn hit(
        element: DamageElement,
        amount: u32,
        attacker: Option<oteryn_dev_client::EntityRef>,
    ) -> TestResult<AttributedFact> {
        Ok(AttributedFact {
            fact: AnalyserFact::DamageInput {
                value: std::num::NonZeroU32::new(amount).ok_or("zero test damage")?,
                element,
                source: AnalyserDamageSource::Creature(None),
            },
            attacker,
        })
    }

    #[test]
    fn c08_rejects_wrong_spider_damage_and_out_of_range_hits() -> TestResult {
        let exact = spider(1);
        assert!(!damage_kind(&hit(DamageElement::Physical, 20, Some(exact))?, exact, false)?);
        assert!(damage_kind(&hit(DamageElement::Physical, 21, Some(exact))?, exact, false).is_err());
        assert!(damage_kind(&hit(DamageElement::Physical, 1, Some(spider(2)))?, exact, true).is_err());
        assert!(damage_kind(&hit(DamageElement::Earth, 2, None)?, exact, true)?);
        assert!(damage_kind(&hit(DamageElement::Earth, 3, None)?, exact, true).is_err());
        Ok(())
    }

    #[test]
    fn c08_rejects_an_attackerless_tick_with_foreign_owner_provenance() -> TestResult {
        let line = "event=qualification_poison_tick session=one source=creature:b due_us=7 amount=2";
        assert_eq!(read_tick_receipt(line, "session=one", "source=creature:b")?, Some((7, 2)));
        assert_eq!(read_tick_receipt(line, "session=two", "source=creature:b")?, None);
        assert!(read_tick_receipt(line, "session=one", "source=creature:a").is_err());
        Ok(())
    }

    #[test]
    fn c08_requires_consecutive_global_ticks_and_matching_wire_amounts() -> TestResult {
        verify_tick_series(&[(1, 2), (4_000_001, 1)], &[2, 1])?;
        assert!(verify_tick_series(&[(1, 2), (8_000_001, 1)], &[2, 1]).is_err());
        assert!(verify_tick_series(&[(1, 1), (4_000_001, 2)], &[1, 2]).is_err());
        assert!(verify_tick_series(&[(1, 2), (4_000_001, 1)], &[1, 2]).is_err());
        Ok(())
    }

    #[test]
    fn c08_rejects_natural_expiry_retained_poison_wrong_cost_and_hp_change() -> TestResult {
        let before = ActorVitals {
            health: 150,
            max_health: 195,
            mana: 150,
            max_mana: 150,
            soul: 100,
            harmony: 0,
            serene: false,
        };
        let correct = ActorVitals { mana: 120, ..before };
        verify_cure_vitals(before, correct, 30)?;
        assert!(verify_cure_vitals(before, ActorVitals { mana: 121, ..before }, 29).is_err());
        assert!(verify_cure_vitals(before, ActorVitals { health: 151, ..correct }, 30).is_err());
        verify_cure_window(Some(5_000), 100, 200, None)?;
        assert!(verify_cure_window(Some(100), 100, 200, None).is_err());
        assert!(verify_cure_window(Some(5_000), 100, 200, Some(1)).is_err());
        Ok(())
    }
}
