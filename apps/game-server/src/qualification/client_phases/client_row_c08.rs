//! C08 client replay capture: let a real poison spider poison the character, dispel it with the
//! served `exana pox` spell, and capture the ACTOR_VITALS values applied by
//! `crates/session/src/domain_dispatch.rs` through `Session::apply_state_delta` and exposed by
//! `Session::actor_vitals` in `session_access.rs`. The session owns domain 3 and does not retain
//! raw delta bytes, so each capture is the applied vitals value encoded by the production codec.
use super::full_row_c08::{
    poison_status, poison_tick_fact, travel_to_poison_spider, wait_for_c08_logout, walk_c08,
};
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::actor_spell::{
    ActorVitals, SpellCastDisposition, SpellTarget, encode_actor_vitals,
};
use oteryn_protocol_oteryn::actor_spell_catalog as catalogue;
use oteryn_protocol_oteryn::analyser::{AnalyserFact, AttributedFact};

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let stage_capture = std::env::var_os("E2E_C08_CAPTURE_DIR");
    let suite_capture = std::env::var_os("E2E_ALL_CAPTURE_DIR");
    if stage_capture.is_none() && suite_capture.is_none() {
        return Err("C08 capture directory absent".into());
    }
    for root in [stage_capture, suite_capture].into_iter().flatten() {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(Path::new(&root).join(name))?;
        std::io::Write::write_all(&mut file, bytes)?;
    }
    Ok(())
}

fn line(vitals: &ActorVitals) -> String {
    format!(
        "{} {} {} {} {} {} {}\n",
        vitals.health,
        vitals.max_health,
        vitals.mana,
        vitals.max_mana,
        vitals.soul,
        vitals.harmony,
        u8::from(vitals.serene)
    )
}

/// The next health decrease must match the attributed hit or condition tick.
async fn next_damage(
    client: &mut DevClientSession,
    above: u32,
    seconds: u64,
    expected: impl Fn(&AttributedFact) -> bool,
) -> TestResult<ActorVitals> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(seconds);
    let mut observed: Option<ActorVitals> = None;
    let mut attributed_damage = 0u32;
    loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        for event in client.take_events() {
            match event {
                SessionEvent::ActorVitals(delta) if delta.value.health < above => {
                    if observed.is_some_and(|vitals| vitals.health != delta.value.health) {
                        return Err(
                            "multiple health decreases before C08 damage attribution".into()
                        );
                    }
                    observed = Some(delta.value);
                }
                SessionEvent::AnalyserAttributedFacts(delta) => {
                    for fact in delta.value.iter().filter(|fact| expected(fact)) {
                        if let AnalyserFact::DamageInput { value, .. } = fact.fact {
                            attributed_damage = attributed_damage
                                .checked_add(value.get())
                                .ok_or("C08 damage attribution overflow")?;
                        }
                    }
                }
                _ => {}
            }
        }
        if let Some(vitals) = observed {
            let health_loss = above - vitals.health;
            if attributed_damage > health_loss {
                return Err(format!(
                    "C08 attributed damage {attributed_damage} exceeded health loss {above}->{}",
                    vitals.health
                )
                .into());
            }
            if attributed_damage == health_loss {
                return Ok(vitals);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("no attributed C08 damage below health {above}").into());
        }
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let (_account, mut client) =
        character_prep::connect_c08(clients, 0x30, "Poison Replay Caster").await?;
    let spider = travel_to_poison_spider(&mut client, world, &metadata.spawns).await?;
    let before = *client.actor_vitals().ok_or("C08 initial vitals absent")?;
    client.session_mut().attack_target(None).await?;
    client.take_events();
    let direct = next_damage(&mut client, before.health, 30, |fact| {
        fact.attacker == Some(spider)
    })
    .await?;
    let delayed = next_damage(&mut client, direct.health, 15, poison_tick_fact).await?;
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

    let mut expected = String::new();
    for (name, vitals) in [
        ("before", &before),
        ("direct", &direct),
        ("delayed", &delayed),
        ("dispelled", &dispelled),
    ] {
        match encode_actor_vitals(vitals) {
            Ok(payload) => capture(&format!("c08-{name}.bin"), &payload)?,
            Err(error) => return Err(format!("C08 {name} encode: {error:?}").into()),
        }
        expected.push_str(&line(vitals));
    }
    capture("c08-expected.txt", expected.as_bytes())?;
    let safe = route(world, tile(client.world_spatial().actor_position)?, |at| {
        is_logout_rest_tile(world, at)
    })
    .ok_or("C08 replay cleanup has no reachable logout tile")?;
    walk_c08(&mut client, world, &safe).await?;

    wait_for_c08_logout(&mut client, Some(spider)).await?;
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("C08 cleanup logout refused: {reply:?}").into());
    }
    Ok(format!(
        "health={}>{}>{} mana={}>{}",
        before.health, direct.health, delayed.health, mana_before, dispelled.mana
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    character_prep::declare_seeded("C08.client", "level-10-sorcerer", "Cure Poison eligibility");
    match verdict::indexed_timeout(Duration::from_secs(420), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=C08.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=C08.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=C08.client status=FAIL evidence=timeout"),
    }
}
