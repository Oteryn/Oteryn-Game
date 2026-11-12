//! N07c client capture: a real rat kill whose ANALYSER facts (state domain 15) the live session
//! applies through `analyser_transport::apply` and delivers as `SessionEvent::AnalyserFacts`.
//! The dev client decodes those deltas and retains no raw payload, so this row records the
//! server-observed facts (one line per fact) and totals, not `.bin` bytes. The consumer is the
//! analyser reducer in `apps/client/src/features/0010-analyser/protocol.rs`.
use super::*;
use oteryn_dev_client::client_runtime::analyser::{AnalyserFact, AnalyserImpact};
use oteryn_dev_client::{DevClientSession, EntityKind, SessionEvent};

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("N07C_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

fn collect(client: &mut DevClientSession, facts: &mut Vec<AnalyserFact>) {
    for event in client.take_events() {
        if let SessionEvent::AnalyserFacts(delta) = event {
            facts.extend(delta.value);
        }
    }
}

/// Fight the adjacent creature and collect every analyser fact the server pushes for the kill.
async fn fight(client: &mut DevClientSession) -> TestResult<Vec<AnalyserFact>> {
    let here = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == EntityKind::Creature
                    && entry.position.floor == here.floor
                    && entry.position.x.abs_diff(here.x).max(entry.position.y.abs_diff(here.y)) <= 1
            })
        })
        .copied()
        .ok_or("no adjacent creature after travel")?;
    let attack = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    if attack.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(format!("attack refused: {attack:?}").into());
    }
    let mut facts = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    while !facts.iter().any(|fact| matches!(fact, AnalyserFact::Kill { .. })) {
        client.service_liveness(Duration::from_millis(100)).await?;
        collect(client, &mut facts);
        if tokio::time::Instant::now() >= deadline {
            return Err("no analyser Kill fact before the deadline".into());
        }
    }
    // The loot and experience facts of the same settlement may follow in another batch.
    for _ in 0..8 {
        client.service_liveness(Duration::from_millis(100)).await?;
        collect(client, &mut facts);
    }
    Ok(facts)
}

fn line(fact: &AnalyserFact) -> String {
    match fact {
        AnalyserFact::Kill { .. } => "kill".into(),
        AnalyserFact::Experience { raw, gained } => format!("xp {raw} {gained}"),
        AnalyserFact::KillLoot {
            corpse_items, gold, ..
        } => format!(
            "loot {gold} {}",
            corpse_items.iter().map(|item| u64::from(item.count.get())).sum::<u64>()
        ),
        AnalyserFact::SupplyUsed(item) => format!("supply {}", item.count.get()),
        AnalyserFact::Impact(AnalyserImpact::Damage { value, .. }) => format!("damage {value}"),
        AnalyserFact::Impact(AnalyserImpact::Healing(value)) => format!("healing {value}"),
        AnalyserFact::DamageInput { value, .. } => format!("received {value}"),
        AnalyserFact::Dropped(count) => format!("dropped {count}"),
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
    let mut client = connect_world(clients, 0x35).await?;
    if !client.selected_capabilities().contains(&10) {
        return Err("analyser capability 10 not selected".into());
    }
    travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
    let facts = fight(&mut client).await?;
    let kills = facts
        .iter()
        .filter(|fact| matches!(fact, AnalyserFact::Kill { .. }))
        .count();
    let (mut raw, mut gained, mut gold, mut items) = (0u64, 0u64, 0u64, 0u64);
    for fact in &facts {
        match fact {
            AnalyserFact::Experience { raw: r, gained: g } => {
                raw += r;
                gained += g;
            }
            AnalyserFact::KillLoot {
                corpse_items,
                gold: g,
                ..
            } => {
                gold += g;
                items += corpse_items.iter().map(|item| u64::from(item.count.get())).sum::<u64>();
            }
            _ => {}
        }
    }
    let lines: Vec<String> = facts.iter().map(line).collect();
    let expected = format!(
        "kills={kills}\nraw_xp={raw}\ngained_xp={gained}\ngold={gold}\nloot_items={items}\nfacts={}\n",
        lines.join("|")
    );
    capture("n07c-expected.txt", expected.as_bytes());
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(300), verdict::observe(journey(clients))).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=N07c.client status=CAPTURED evidence=raw-delta-not-retained-by-dev-client;{observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=N07c.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=N07c.client status=FAIL evidence=timeout"),
    }
}
