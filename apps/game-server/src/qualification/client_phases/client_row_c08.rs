//! Qualify poison and its cure using a live journey and the original inbound Server frames.
use super::full_row_c08::{
    poison_status, poison_tick_fact, travel_to_poison_spider, wait_for_c08_logout, walk_c08,
};
use super::*;
use oteryn_dev_client::{CLIENT_SUPPORTED_CAPABILITIES, DevClientSession, SessionEvent};
use oteryn_dev_client::native_session::{Admission, test_support::{capture_inbound, replay_vitals}};
use oteryn_protocol_oteryn::{CharacterId, GameSessionId};
use oteryn_protocol_oteryn::actor_spell::{
    ActorVitals, SpellCastDisposition, SpellTarget,
};
use oteryn_protocol_oteryn::actor_spell_catalog as catalogue;
use oteryn_protocol_oteryn::analyser::AnalyserFact;

struct Replay {
    account: character_prep::DedicatedCharacter,
    saved_position: oteryn_protocol_oteryn::world_spatial::ActorPosition,
    session: GameSessionId,
    checkpoints: [(u64, ActorVitals); 4],
    observed: String,
}

/// The next health decrease must match the attributed hit or condition tick.
async fn next_damage(
    client: &mut DevClientSession,
    above: u32,
    seconds: u64,
    spider: oteryn_dev_client::EntityRef,
    periodic: bool,
) -> TestResult<(u64, ActorVitals)> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(seconds);
    let mut observed: Option<(u64, ActorVitals)> = None;
    let mut attributed_damage = 0u32;
    let mut expected_hit = false;
    loop {
        client.service_liveness(Duration::from_millis(50)).await?;
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
                            let direct = fact.attacker == Some(spider);
                            let tick = poison_tick_fact(fact);
                            if !(direct || periodic && tick) {
                                return Err(format!("C08 unexpected damage attribution: {fact:?}").into());
                            }
                            expected_hit |= if periodic { tick } else { direct };
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
            if expected_hit && attributed_damage == health_loss {
                return Ok((sequence, vitals));
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("no attributed C08 damage below health {above}").into());
        }
    }
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
    let spider = travel_to_poison_spider(&mut client, world, &metadata.spawns).await?;
    let before = *client.actor_vitals().ok_or("C08 initial vitals absent")?;
    let before_sequence = client.last_server_sequence();
    let (direct_sequence, direct) =
        next_damage(&mut client, before.health, 30, spider, false).await?;
    let (delayed_sequence, delayed) =
        next_damage(&mut client, direct.health, 15, spider, true).await?;
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
    let mana_before = client
        .actor_vitals()
        .ok_or("C08 pre-cast vitals absent")?
        .mana;
    let cast = client
        .cast_spell(
            std::num::NonZeroU32::new(entry.cast_index).ok_or("C08 zero cast index")?,
            SpellTarget::None,
            false,
        )
        .await?;
    if cast.disposition != SpellCastDisposition::Cast {
        return Err(format!("C08 exana pox refused: {cast:?}").into());
    }
    let dispelled = *client.actor_vitals().ok_or("C08 post-cast vitals absent")?;
    if dispelled.mana.checked_add(entry.mana) != Some(mana_before) {
        return Err(format!(
            "C08 exana pox mana {mana_before}->{} does not match cost {}",
            dispelled.mana, entry.mana
        )
        .into());
    }

    if poison_status(&mut client).await? {
        return Err("C08 poison remained active after Cure Poison".into());
    }

    let dispel_sequence = cast.actor_vitals_delta.ok_or("C08 cure vitals delta absent")?.server_sequence;
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
            (dispel_sequence, dispelled),
        ],
        observed: format!(
            "health={}>{}>{} mana={}>{}",
            before.health, direct.health, delayed.health, mana_before, dispelled.mana
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
