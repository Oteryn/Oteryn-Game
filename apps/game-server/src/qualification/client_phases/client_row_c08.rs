//! C08 client replay capture: let a real poison spider poison the character, dispel it with the
//! served `exana pox` spell, and capture the ACTOR_VITALS values applied by
//! `crates/session/src/domain_dispatch.rs` through `Session::apply_state_delta` and exposed by
//! `Session::actor_vitals` in `session_access.rs`. The session owns domain 3 and does not retain
//! raw delta bytes, so each capture is the applied vitals value encoded by the production codec.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::actor_spell::{
    ActorVitals, SpellCastDisposition, SpellTarget, encode_actor_vitals,
};
use oteryn_protocol_oteryn::actor_spell_catalog as catalogue;

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("C08_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
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

fn poison_was_dispelled(poisoned: bool, after: bool, paid: bool) -> bool {
    poisoned && !after && paid
}

fn logout_complete(outcome: Option<oteryn_protocol_oteryn::logout::LogoutResult>) -> bool {
    outcome == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
}

pub(super) async fn require_logout(client: &mut DevClientSession, budget: Duration) -> TestResult {
    let deadline = tokio::time::Instant::now() + budget;
    verdict::Harness::global().bounded_wait(deadline, "logout");
    loop {
        let reply = client.session_mut().logout().await?;
        if logout_complete(reply.result) {
            return Ok(());
        }
        if reply.result.is_none() || tokio::time::Instant::now() >= deadline {
            return Err(format!("cleanup logout did not commit: {reply:?}").into());
        }
        client.service_liveness(Duration::from_millis(250)).await?;
        client.take_events();
    }
}

async fn poison_damage(client: &mut DevClientSession, start: ActorVitals) -> TestResult<(ActorVitals, ActorVitals)> {
    use oteryn_protocol_oteryn::analyser::{AnalyserDamageSource, AnalyserFact};
    use oteryn_protocol_oteryn::damage_element::DamageElement;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(45);
    let mut previous = start;
    let mut vitals = Vec::new();
    let mut poison = Vec::new();
    loop {
        for event in client.take_events() {
            match event {
                SessionEvent::ActorVitals(delta) => {
                    if let Some(damage) = previous.health.checked_sub(delta.value.health).filter(|damage| *damage > 0) {
                        vitals.push((damage, delta.value));
                    }
                    previous = delta.value;
                }
                SessionEvent::AnalyserAttributedFacts(delta) => {
                    poison.extend(delta.value.into_iter().filter_map(|entry| match entry.fact {
                        AnalyserFact::DamageInput { element: DamageElement::Earth, source: AnalyserDamageSource::Creature(None), value }
                            if entry.attacker.is_none() => Some(value.get()),
                        _ => None,
                    }));
                }
                _ => {}
            }
        }
        if let Some((_, first)) = vitals.first()
            && let Some((_, tick)) = vitals.iter().skip(1).find(|(damage, _)| poison.contains(damage))
        {
            return Ok((*first, *tick));
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("C08 missing Earth damage from the creature condition owner".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

async fn poisoned(client: &mut DevClientSession) -> TestResult<bool> {
    let (status, _) = client_row_c17a::query(client).await?;
    Ok(status.conditions.iter().any(|condition| condition.icon == 1))
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
    let mut client = connect_world(clients, 0x30).await?;
    travel_to_spawn(
        &mut client,
        world,
        &metadata.spawns,
        "oteryn:creature.poison_spider",
    )
    .await?;
    client.session_mut().attack_target(None).await?;
    client.take_events();
    let before = *client.actor_vitals().ok_or("C08 initial vitals absent")?;
    let (direct, delayed) = poison_damage(&mut client, before).await?;

    let snapshot = client
        .session_mut()
        .gated_snapshot(catalogue::STATE_DOMAIN_ACTOR_SPELL_CATALOG)
        .ok_or("C08 spell catalogue absent")?;
    let entry = catalogue::decode_catalog(&snapshot.payload)
        .map_err(|error| format!("C08 catalogue decode: {error:?}"))?
        .into_iter()
        .find(|entry| entry.words == "exana pox" && entry.available)
        .ok_or("C08 exana pox not served as available")?;
    let poison_before = poisoned(&mut client).await?;
    if !poison_before {
        return Err("C08 poison was absent before the dispel cast".into());
    }
    let mana_before = client.actor_vitals().ok_or("C08 pre-cast vitals absent")?.mana;
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
    let dispelled = cast.actor_vitals_delta.ok_or("C08 cast omitted its vitals delta")?.value;
    if !poison_was_dispelled(poison_before, poisoned(&mut client).await?, dispelled.mana.checked_add(entry.mana) == Some(mana_before)) {
        return Err(format!(
            "C08 exana pox did not clear poison and debit exact mana: {mana_before}->{} cost {}",
            dispelled.mana, entry.mana
        )
        .into());
    }

    let mut expected = String::new();
    for (name, vitals) in [
        ("before", &before),
        ("direct", &direct),
        ("delayed", &delayed),
        ("dispelled", &dispelled),
    ] {
        match encode_actor_vitals(vitals) {
            Ok(payload) => capture(&format!("c08-{name}.bin"), &payload),
            Err(error) => return Err(format!("C08 {name} encode: {error:?}").into()),
        }
        expected.push_str(&line(vitals));
    }
    capture("c08-expected.txt", expected.as_bytes());
    // Leave the saved character out of combat for the phases that follow.
    require_logout(&mut client, Duration::from_secs(70)).await?;
    Ok(format!(
        "health={}>{}>{} mana={}>{}",
        before.health, direct.health, delayed.health, mana_before, dispelled.mana
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients))).await {
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

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_protocol_oteryn::logout::LogoutResult;

    #[test]
    fn paid_cast_is_insufficient_without_poison_removal() {
        assert!(!poison_was_dispelled(false, false, true));
        assert!(!poison_was_dispelled(true, true, true));
        assert!(poison_was_dispelled(true, false, true));
    }

    #[test]
    fn refused_logout_cannot_finish_the_journey() {
        assert!(!logout_complete(None));
        assert!(!logout_complete(Some(LogoutResult::InFight { retry_after_ms: 1 })));
        assert!(!logout_complete(Some(LogoutResult::Busy)));
        assert!(logout_complete(Some(LogoutResult::Accepted)));
    }
}
