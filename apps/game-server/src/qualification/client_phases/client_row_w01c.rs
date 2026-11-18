//! W01c client replay capture: an admitted poison spider's melee attack reaches the player as an
//! `ACTOR_VITALS` health decrease. The session retains typed vitals only, so the captured payload
//! is the server-delivered `ActorVitals` encoded with the production encoder (the same convention
//! as M11a). The consumer is `Session::apply_state_delta` and `Session::actor_vitals` in
//! `crates/session`.
use super::*;
use oteryn_dev_client::SessionEvent;
use oteryn_protocol_oteryn::actor_spell;
use oteryn_protocol_oteryn::attack::AttackIntentDisposition;

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "W01C_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
}

fn spider_damage(
    before: u32,
    after: u32,
    matching_hit: Option<u32>,
    competing_damage: bool,
) -> bool {
    !competing_damage && after > 0 && after < before && before.checked_sub(after) == matching_hit
}

#[test]
fn spider_vitals_require_the_attributed_health_debit() {
    assert!(!spider_damage(30, 25, None, false));
    assert!(!spider_damage(30, 25, Some(3), false));
    assert!(!spider_damage(30, 0, Some(30), false));
    assert!(spider_damage(30, 25, Some(5), false));
    assert!(!spider_damage(30, 25, Some(5), true));
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
    let mut client = connect_world(clients, 0x0d).await?;
    travel_to_spawn(
        &mut client,
        world,
        &metadata.spawns,
        "oteryn:creature.poison_spider",
    )
    .await?;
    use oteryn_protocol_oteryn::analyser::{AnalyserFact, CAPABILITY_ANALYSER_HIT_ATTRIBUTION_V1};
    use oteryn_protocol_oteryn::damage_element::DamageElement;
    if !client
        .selected_capabilities()
        .contains(&CAPABILITY_ANALYSER_HIT_ATTRIBUTION_V1)
    {
        return Err("step=admission analyser hit attribution not selected".into());
    }
    let spider = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == oteryn_dev_client::EntityKind::Creature
                    && entry.display_name.and_then(|name| {
                        name.as_str()
                            .ok()
                            .map(|name| name.eq_ignore_ascii_case("Poison Spider"))
                    }) == Some(true)
            })
        })
        .map(|entry| entry.entity)
        .ok_or("step=spider no visible poison spider")?;
    let before = *client
        .actor_vitals()
        .ok_or("W01c client capture: admission supplied no ActorVitals")?;
    let cancel = client.session_mut().attack_target(None).await?;
    if cancel.disposition != AttackIntentDisposition::Ok {
        return Err(format!("W01c client capture: attack-cancel refused: {cancel:?}").into());
    }
    client.take_events();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(45);
    let mut previous = *client
        .actor_vitals()
        .ok_or("vitals after attack cancel absent")?;
    let mut debit = None;
    let mut hit = None;
    let mut competing_damage = false;
    let damaged = 'wait: loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        for event in client.take_events() {
            match event {
                SessionEvent::ActorVitals(delta) => {
                    if delta.value.health < previous.health {
                        competing_damage |= debit.is_some();
                        debit = Some((previous.health, delta.value));
                    }
                    previous = delta.value;
                }
                SessionEvent::AnalyserAttributedFacts(delta) => {
                    for attributed in delta.value {
                        if let AnalyserFact::DamageInput { value, element, .. } = attributed.fact {
                            if attributed.attacker == Some(spider)
                                && element == DamageElement::Physical
                            {
                                competing_damage |= hit.replace(value.get()).is_some();
                            } else {
                                competing_damage = true;
                            }
                        } else if matches!(attributed.fact, AnalyserFact::Dropped(_)) {
                            competing_damage = true;
                        }
                    }
                }
                _ => {}
            }
        }
        if competing_damage {
            return Err("step=spider competing damage or ambiguous vitals in observation".into());
        }
        // DamageInput is emitted from the committed hit. Compare the entire
        // observation drain and reject an ambiguous second hit or debit.
        if let (Some((health, vitals)), Some(amount)) = (debit, hit)
            && spider_damage(health, vitals.health, Some(amount), competing_damage)
            && vitals.health < before.health
        {
            break 'wait vitals;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("W01c client capture: no attributed spider damage delta in 45s".into());
        }
    };
    capture(
        "w01c-vitals.bin",
        &actor_spell::encode_actor_vitals(&damaged)
            .map_err(|error| format!("encode vitals: {error:?}"))?,
    );
    let expected = format!(
        "health_before={}\nhealth={}\nmax_health={}\nmana={}\nmax_mana={}\nsoul={}\n",
        before.health,
        damaged.health,
        damaged.max_health,
        damaged.mana,
        damaged.max_mana,
        damaged.soul
    );
    capture("w01c-expected.txt", expected.as_bytes());
    Ok(expected)
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    match verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=W01c.client status=CAPTURED evidence={:?}",
            expected.replace('\n', ";")
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W01c.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=W01c.client status=FAIL evidence=timeout"),
    }
}
