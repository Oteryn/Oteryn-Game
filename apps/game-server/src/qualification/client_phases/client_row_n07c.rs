//! N07c client capture: a real rat kill whose ANALYSER facts (state domain 15) the live session
//! applies through `analyser_transport::apply` as legacy or attributed typed events.
//! The dev client decodes those deltas and retains no raw payload, so this row records the
//! complete canonical typed events (including attacker identities), selection and totals,
//! not original `.bin` bytes. Flash behavior is outside this bounded replay. The consumer is the
//! analyser reducer in `apps/client/src/features/0010-analyser/protocol.rs`.
use super::*;
use oteryn_dev_client::client_runtime::analyser::{AnalyserFact, AnalyserImpact};
use oteryn_dev_client::{DevClientSession, EntityKind, SessionEvent};

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "N07C_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
}

/// Canonical re-encoding of complete typed events, not the discarded raw state-delta bytes.
fn collect(
    client: &mut DevClientSession,
    selected: bool,
    facts: &mut Vec<AnalyserFact>,
    events: &mut Vec<String>,
) -> TestResult {
    use oteryn_protocol_oteryn::analyser::{encode_analyser_fact, encode_attributed_analyser_fact};
    let hex = |bytes: Vec<u8>| {
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    };
    for event in client.take_events() {
        let (kind, sequence, base, revision, encoded) = match event {
            SessionEvent::AnalyserFacts(delta) => {
                if selected {
                    return Err("selected attribution session emitted legacy analyser event".into());
                }
                let encoded = delta
                    .value
                    .iter()
                    .map(|fact| encode_analyser_fact(fact).map(&hex))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|error| format!("analyser capture codec: {error:?}"))?;
                facts.extend(delta.value);
                (
                    "legacy",
                    delta.server_sequence,
                    delta.base_revision,
                    delta.new_revision,
                    encoded,
                )
            }
            SessionEvent::AnalyserAttributedFacts(delta) => {
                if !selected {
                    return Err(
                        "unselected attribution session emitted attributed analyser event".into(),
                    );
                }
                let encoded = delta
                    .value
                    .iter()
                    .map(|fact| {
                        encode_attributed_analyser_fact(&fact.fact, fact.attacker.as_ref())
                            .map(&hex)
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|error| format!("analyser capture codec: {error:?}"))?;
                facts.extend(delta.value.into_iter().map(|fact| fact.fact));
                (
                    "attributed",
                    delta.server_sequence,
                    delta.base_revision,
                    delta.new_revision,
                    encoded,
                )
            }
            _ => continue,
        };
        events.push(format!(
            "{kind} {sequence} {base} {revision} {}",
            encoded.join(",")
        ));
    }
    Ok(())
}

/// Fight the adjacent creature and collect every analyser fact the server pushes for the kill.
async fn fight(
    client: &mut DevClientSession,
    selected: bool,
) -> TestResult<(Vec<AnalyserFact>, Vec<String>)> {
    let here = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == EntityKind::Creature
                    && entry.position.floor == here.floor
                    && entry
                        .position
                        .x
                        .abs_diff(here.x)
                        .max(entry.position.y.abs_diff(here.y))
                        <= 1
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
    let mut events = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    while !facts
        .iter()
        .any(|fact| matches!(fact, AnalyserFact::Kill { .. }))
    {
        client.service_liveness(Duration::from_millis(100)).await?;
        collect(client, selected, &mut facts, &mut events)?;
        if tokio::time::Instant::now() >= deadline {
            return Err("no analyser Kill fact before the deadline".into());
        }
    }
    // The loot and experience facts of the same settlement may follow in another batch.
    for _ in 0..8 {
        client.service_liveness(Duration::from_millis(100)).await?;
        collect(client, selected, &mut facts, &mut events)?;
    }
    Ok((facts, events))
}

fn line(fact: &AnalyserFact) -> String {
    match fact {
        AnalyserFact::Kill { .. } => "kill".into(),
        AnalyserFact::Experience { raw, gained } => format!("xp {raw} {gained}"),
        AnalyserFact::KillLoot {
            corpse_items, gold, ..
        } => format!(
            "loot {gold} {}",
            corpse_items
                .iter()
                .map(|item| u64::from(item.count.get()))
                .sum::<u64>()
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
    let attribution_selected = client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::analyser::CAPABILITY_ANALYSER_HIT_ATTRIBUTION_V1);
    let (facts, events) = fight(&mut client, attribution_selected).await?;
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
                items += corpse_items
                    .iter()
                    .map(|item| u64::from(item.count.get()))
                    .sum::<u64>();
            }
            _ => {}
        }
    }
    let lines: Vec<String> = facts.iter().map(line).collect();
    let mut expected = format!(
        "capture_kind=typed-events-v1\nattribution_selected={attribution_selected}\ncoverage=totals-and-typed-events;flash-not-exercised\nevent_count={}\nkills={kills}\nraw_xp={raw}\ngained_xp={gained}\ngold={gold}\nloot_items={items}\nfacts={}\n",
        events.len(),
        lines.join("|")
    );
    for (index, event) in events.iter().enumerate() {
        expected.push_str(&format!("event_{index}={event}\n"));
    }
    capture("n07c-expected.txt", expected.as_bytes());
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(300), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=N07c.client status=CAPTURED evidence=raw-delta-not-retained-by-dev-client;{observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=N07c.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=N07c.client status=FAIL evidence=timeout"),
    }
}
