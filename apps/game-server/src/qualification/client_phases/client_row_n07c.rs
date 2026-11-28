//! N07c client capture: a real rat kill whose ANALYSER facts (state domain 15) the live session
//! applies through `analyser_transport::apply` as legacy or attributed typed events.
//! The dev client decodes those deltas and retains no raw payload, so this row records the
//! complete canonical typed events (including attacker identities), selection and totals,
//! not original `.bin` bytes. Flash behavior is outside this bounded replay. The consumer is the
//! analyser reducer in `apps/client/src/features/0010-analyser/protocol.rs`.
use super::*;
use oteryn_dev_client::client_runtime::analyser::{AnalyserFact, AnalyserImpact};
use oteryn_dev_client::{DevClientSession, EntityKind, EntityRef, SessionEvent};

fn capture(bytes: &[u8]) -> TestResult {
    use std::io::Write;
    let root = required("NODE_BOOT_N07C_CAPTURE_DIR")?;
    let path = Path::new(&root).join("n07c-expected.txt");
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?
        .write_all(bytes)?;
    Ok(())
}

fn track_aggressors(event: &SessionEvent, aggressors: &mut Vec<EntityRef>) -> TestResult {
    match event {
        SessionEvent::AnalyserAttributedFacts(delta) => {
            for attributed in &delta.value {
                if matches!(
                    attributed.fact,
                    AnalyserFact::DamageInput {
                        source: oteryn_protocol_oteryn::analyser::AnalyserDamageSource::Creature(_),
                        ..
                    }
                ) {
                    let attacker = attributed
                        .attacker
                        .ok_or("incoming creature damage has no exact attacker")?;
                    if !aggressors.contains(&attacker) {
                        aggressors.push(attacker);
                    }
                }
            }
        }
        SessionEvent::AnalyserFacts(delta)
            if delta.value.iter().any(|fact| {
                matches!(
                    fact,
                    AnalyserFact::DamageInput {
                        source: oteryn_protocol_oteryn::analyser::AnalyserDamageSource::Creature(_),
                        ..
                    }
                )
            }) =>
        {
            return Err("legacy creature damage cannot prove exact aggressor departure".into());
        }
        _ => {}
    }
    Ok(())
}

fn drain_aggressors(client: &mut DevClientSession, aggressors: &mut Vec<EntityRef>) -> TestResult {
    for event in client.take_events() {
        track_aggressors(&event, aggressors)?;
    }
    Ok(())
}

fn verify_no_aggressor(client: &DevClientSession, aggressors: &[EntityRef]) -> TestResult {
    let view = client
        .world_entities()
        .ok_or("aggressor check has no entity view")?;
    if let Some(attacker) = aggressors.iter().find(|id| {
        view.get(id)
            .is_some_and(|entry| entry.kind == EntityKind::Creature)
    }) {
        return Err(format!("Rat logout still has a live aggressor in view: {attacker:?}").into());
    }
    Ok(())
}

struct RatObservation {
    target_selected: bool,
    last_position: ActorPosition,
}

/// Canonical re-encoding of complete typed events, not the discarded raw state-delta bytes.
fn collect(
    client: &mut DevClientSession,
    selected: bool,
    facts: &mut Vec<AnalyserFact>,
    events: &mut Vec<String>,
    target: oteryn_dev_client::EntityRef,
    target_state: &mut RatObservation,
    aggressors: &mut Vec<EntityRef>,
) -> TestResult {
    use oteryn_protocol_oteryn::analyser::{encode_analyser_fact, encode_attributed_analyser_fact};
    let hex = |bytes: Vec<u8>| {
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    };
    for event in client.take_events() {
        track_aggressors(&event, aggressors)?;
        let (kind, sequence, base, revision, encoded) = match event {
            SessionEvent::AnalyserFacts(delta) => {
                if selected {
                    return Err("selected attribution session emitted legacy analyser event".into());
                }
                let encoded = delta
                    .value
                    .iter()
                    .map(|fact| encode_analyser_fact(fact).map(hex))
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
                        encode_attributed_analyser_fact(&fact.fact, fact.attacker.as_ref()).map(hex)
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
            SessionEvent::ActorCombatState(delta) => {
                target_state.target_selected |= delta.value.target == Some(target);
                continue;
            }
            SessionEvent::WorldSpatialEntities(delta) => {
                if let Some(entry) = delta
                    .value
                    .update
                    .iter()
                    .find(|entry| entry.entity == target)
                {
                    target_state.last_position = entry.position;
                }
                continue;
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

// Honor step pacing and detour only around a confirmed creature.
async fn walk_rat_route(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
    aggressors: &mut Vec<EntityRef>,
) -> TestResult {
    let center = tile(client.world_spatial().actor_position)?;
    let mut path = VecDeque::from(path.to_vec());
    let mut detours = 0;
    while let Some(direction) = path.pop_front() {
        let from = tile(client.world_spatial().actor_position)?;
        let expected = destination(world, from, direction).ok_or("Rat route landing missing")?;
        let step = client.step(direction).await?;
        let actual = tile(client.world_spatial().actor_position)?;
        if step.disposition == StepDisposition::Blocked && actual == from {
            client.service_liveness(Duration::from_millis(100)).await?;
            let occupant = client
                .world_entities()
                .and_then(|view| {
                    view.iter().find(|entry| {
                        matches!(
                            entry.kind,
                            EntityKind::Creature | EntityKind::Npc | EntityKind::Player
                        ) && tile(entry.position).ok() == Some(expected)
                    })
                })
                .map(|entry| (entry.entity, entry.kind));
            let Some((occupant, kind)) = occupant else {
                return Err(format!("BLOCKER-SERVER Rat route blocked without an occupant from={from:?} expected={expected:?} step={step:?}").into());
            };
            if kind != EntityKind::Creature {
                return Err(
                    format!("Rat route blocked by {kind:?} {occupant:?} at {expected:?}").into(),
                );
            }
            if detours == 8 {
                return Err("Rat route exhausted its creature detour budget".into());
            }
            detours += 1;
            client.service_liveness(Duration::from_millis(100)).await?;
            if tile(client.world_spatial().actor_position)? != from {
                return Err("Rat route actor moved while observing a blocked step".into());
            }
            let view = client
                .world_entities()
                .ok_or("Rat route entity view missing")?;
            let position = view.get(&occupant).map(|entry| entry.position);
            evidence(&format!(
                "N07C_CREATURE_BLOCK attempt={detours} entity={occupant:?} blocked={expected:?} current={position:?} step={step:?}"
            ));
            let blocked: std::collections::BTreeSet<_> = view
                .iter()
                .filter(|entry| &entry.entity.identity != view.own_identity())
                .filter_map(|entry| tile(entry.position).ok())
                .collect();
            path = route_around_occupant(
                from,
                center,
                expected,
                &path,
                |at| !blocked.contains(&at),
                |at, direction| destination(world, at, direction),
            )
            .ok_or("Rat route has no detour to the original goal")?
            .into();
            continue;
        }
        if step.disposition != StepDisposition::Moved
            || step.world_spatial_delta.is_none()
            || actual != expected
        {
            return Err(format!(
                "Rat route step refused or diverged direction={direction:?} from={from:?} expected={expected:?} actual={actual:?} disposition={:?}",
                step.disposition
            ).into());
        }
        verdict::Harness::global().progress("walk");
        if step.step_wait_ms != 0 {
            client
                .service_liveness(Duration::from_millis(u64::from(step.step_wait_ms)))
                .await?;
        }
        drain_aggressors(client, aggressors)?;
    }
    Ok(())
}

async fn travel_to_rat(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    aggressors: &mut Vec<EntityRef>,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let mut rats = Vec::new();
    let mut occupied = std::collections::BTreeSet::new();
    for source in &spawns.sources {
        for point in &source.points {
            let position = TilePos {
                x: point.x,
                y: point.y,
                floor: source.floor,
            };
            let creature = spawns
                .creatures
                .get(point.creature as usize)
                .ok_or("invalid Rat route spawn creature index")?;
            if creature.key == "oteryn:creature.rat" {
                rats.push(position);
            } else if from.floor.abs_diff(position.floor) <= 2
                && from.x.abs_diff(position.x).max(from.y.abs_diff(position.y)) <= 162
            {
                // Keep the approach outside neighboring spawn tiles before sending any step.
                // The shortest static path crossed an active Wolf spawn.
                for dx in -2..=2 {
                    for dy in -2..=2 {
                        if let (Some(x), Some(y)) = (
                            position.x.checked_add_signed(dx),
                            position.y.checked_add_signed(dy),
                        ) {
                            occupied.insert(TilePos {
                                x,
                                y,
                                floor: position.floor,
                            });
                        }
                    }
                }
            }
        }
    }
    let path = route_avoiding(
        world,
        from,
        |at| {
            world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|tile| !tile.protection_zone())
                && rats.iter().any(|rat| {
                    at.floor == rat.floor && at.x.abs_diff(rat.x).max(at.y.abs_diff(rat.y)) <= 1
                })
        },
        |at| !occupied.contains(&at),
    )
    .ok_or("no reachable Rat spawn outside neighboring spawn tiles")?;
    evidence(&format!("N07C_RAT_ROUTE steps={}", path.len()));
    walk_rat_route(client, world, &path, aggressors).await
}

/// Fight the adjacent creature and collect every analyser fact the server pushes for the kill.
async fn fight(
    client: &mut DevClientSession,
    selected: bool,
    aggressors: &mut Vec<EntityRef>,
) -> TestResult<(Vec<AnalyserFact>, Vec<String>)> {
    let here = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == EntityKind::Creature
                    && entry
                        .display_name
                        .is_some_and(|name| name.as_str() == Ok("Rat"))
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
        .ok_or("no adjacent Rat after travel")?;
    if client
        .session_mut()
        .combat_state()
        .is_none_or(|state| state.target.is_some())
    {
        return Err("Rat attack requires an observed clear combat target".into());
    }
    let existing_corpses: Vec<_> = client
        .world_entities()
        .ok_or("Rat fight has no entity view")?
        .iter()
        .filter(|entry| entry.kind == EntityKind::Corpse)
        .map(|entry| entry.entity)
        .collect();
    let attack = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    if attack.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(format!("Rat attack refused: {attack:?}").into());
    }
    let mut target_state = RatObservation {
        target_selected: false,
        last_position: target.position,
    };
    let mut facts = Vec::new();
    let mut events = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    loop {
        let kill = facts
            .iter()
            .any(|fact| matches!(fact, AnalyserFact::Kill { .. }));
        let xp = facts.iter().any(|fact| matches!(fact, AnalyserFact::Experience { raw, gained } if *raw > 0 && *gained > 0));
        if target_state.target_selected && kill && xp {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "Rat analyser settlement deadline target={} kill={kill} xp={xp}",
                target_state.target_selected
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        collect(
            client,
            selected,
            &mut facts,
            &mut events,
            target.entity,
            &mut target_state,
            aggressors,
        )?;
    }
    // Keep the trailing observation to catch duplicate settlement facts.
    for _ in 0..8 {
        client.service_liveness(Duration::from_millis(100)).await?;
        collect(
            client,
            selected,
            &mut facts,
            &mut events,
            target.entity,
            &mut target_state,
            aggressors,
        )?;
    }
    let entities = client
        .world_entities()
        .ok_or("Rat settlement has no entity observation")?;
    if entities
        .get(&target.entity)
        .is_some_and(|entry| entry.kind == EntityKind::Creature)
    {
        return Err("Rat kill fact left the attacked Rat alive".into());
    }
    let corpse = entities
        .iter()
        .find(|entry| {
            entry.kind == EntityKind::Corpse
                && entry.position == target_state.last_position
                && !existing_corpses.contains(&entry.entity)
        })
        .ok_or("Rat settlement has no new corpse at the target's last position")?;
    let oteryn_dev_client::EntityDetail::Object {
        item_handle: Some(handle),
        ..
    } = corpse.detail
    else {
        return Err("Rat corpse has no server item handle".into());
    };
    let used = client.use_item(handle).await?;
    if used.status != CommandStatus::Accepted
        || used.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    {
        return Err(format!("Rat corpse USE refused: {used:?}").into());
    }
    let until = tokio::time::Instant::now() + Duration::from_secs(5);
    let entries = loop {
        if let Some(view) = client
            .open_container()
            .filter(|view| view.container_handle == Some(handle))
        {
            break view.entries.clone();
        }
        if tokio::time::Instant::now() >= until {
            return Err("Rat corpse container delta missing".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    };
    let counts = |items: Vec<(u32, u32)>| {
        let mut counts = BTreeMap::<u32, u64>::new();
        for (id, count) in items {
            *counts.entry(id).or_default() += u64::from(count);
        }
        counts
    };
    let actual = counts(
        entries
            .iter()
            .map(|item| (item.item_definition_ref.get(), item.count.get()))
            .collect(),
    );
    let reported = counts(
        facts
            .iter()
            .filter_map(|fact| match fact {
                AnalyserFact::KillLoot { corpse_items, .. } => Some(corpse_items),
                _ => None,
            })
            .flatten()
            .map(|item| (item.item_type.get(), u32::from(item.count.get())))
            .collect(),
    );
    let mut actual_gold = 0;
    let mut actual_cheese = 0;
    for item in &entries {
        match item.appearance_id {
            3031 => actual_gold += u64::from(item.count.get()),
            3607 => actual_cheese += u64::from(item.count.get()),
            appearance => {
                return Err(format!("unexpected Rat corpse item appearance={appearance}").into());
            }
        }
    }
    let reported_gold: u64 = facts
        .iter()
        .filter_map(|fact| match fact {
            AnalyserFact::KillLoot { gold, .. } => Some(*gold),
            _ => None,
        })
        .sum();
    if actual_gold != reported_gold || actual_gold > 4 || actual_cheese > 1 {
        return Err(format!("Rat corpse totals gold={actual_gold} cheese={actual_cheese} analyser_gold={reported_gold}").into());
    }
    if actual != reported {
        return Err(format!(
            "Rat corpse loot differs from analyser facts actual={actual:?} reported={reported:?}"
        )
        .into());
    }
    evidence(&format!(
        "N07C_RAT_CORPSE gold={actual_gold} cheese={actual_cheese}"
    ));
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

async fn experience(client: &mut DevClientSession) -> TestResult<u64> {
    use oteryn_protocol_oteryn::cyclopedia_character as wire;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            wire::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &[1, 0, 0, 1],
        )
        .await?;
    general_stats::validate(&reply.payload, general_stats::selected(client))?;
    if reply.status != CommandStatus::Accepted || reply.payload.first() != Some(&1) {
        return Err("Rat experience read did not return GeneralStats".into());
    }
    Ok(u64::from_le_bytes(
        reply
            .payload
            .get(1..9)
            .ok_or("Rat experience absent")?
            .try_into()?,
    ))
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("Rat capture logout refused: {reply:?}").into());
    }
    Ok(())
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
    let mut aggressors = Vec::new();
    travel_to_rat(&mut client, world, &metadata.spawns, &mut aggressors).await?;
    verdict::Harness::global().progress("rat-attack");
    let attribution_selected = client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::analyser::CAPABILITY_ANALYSER_HIT_ATTRIBUTION_V1);
    let before_xp = experience(&mut client).await?;
    drain_aggressors(&mut client, &mut aggressors)?;
    let combat_session = client.join_snapshot().game_session_id;
    let sequence_before = client.last_server_sequence();
    let (facts, events) = fight(&mut client, attribution_selected, &mut aggressors).await?;
    let sequence_after = client.last_server_sequence();
    if sequence_after <= sequence_before {
        return Err("Rat combat did not advance the admitting session's sequence".into());
    }
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
    if kills != 1
        || raw != 5
        || gained == 0
        || facts
            .iter()
            .any(|fact| matches!(fact, AnalyserFact::Dropped(_)))
    {
        return Err(format!(
            "incomplete Rat analyser settlement kills={kills} raw_xp={raw} gained_xp={gained} gold={gold}"
        ).into());
    }
    let expected_xp = before_xp
        .checked_add(gained)
        .ok_or("Rat experience overflow")?;
    let after_xp = experience(&mut client).await?;
    if after_xp != expected_xp {
        return Err(format!(
            "Rat experience before={before_xp} expected={expected_xp} actual={after_xp}"
        )
        .into());
    }
    let saved_position = client.world_spatial().actor_position;
    drain_aggressors(&mut client, &mut aggressors)?;
    verify_no_aggressor(&client, &aggressors)?;
    let combat_expiry = tokio::time::Instant::now() + Duration::from_secs(60);
    verdict::Harness::global().bounded_wait(combat_expiry, "pve-logout-lock");
    while client
        .session_mut()
        .combat_state()
        .is_none_or(|state| state.in_fight)
    {
        if tokio::time::Instant::now() >= combat_expiry {
            return Err("Rat combat did not release the observed PvE logout lock".into());
        }
        client.service_liveness(Duration::from_millis(200)).await?;
        drain_aggressors(&mut client, &mut aggressors)?;
        verify_no_aggressor(&client, &aggressors)?;
    }
    logout(&mut client).await?;
    drop(client);
    wait_character_released(clients, 1).await?;

    let mut fresh = connect_world(clients, 0x36).await?;
    let durable_xp = experience(&mut fresh).await?;
    if fresh.join_snapshot().game_session_id == combat_session
        || fresh.world_spatial().actor_position != saved_position
        || durable_xp != expected_xp
    {
        return Err(format!(
            "Rat relog state mismatch expected_xp={expected_xp} actual_xp={durable_xp}"
        )
        .into());
    }
    logout(&mut fresh).await?;
    drop(fresh);
    wait_character_released(clients, 1).await?;

    let session_hex = combat_session
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    evidence(&format!(
        "N07C_DURABLE_XP before={before_xp} after={after_xp} relog={durable_xp}"
    ));
    let lines: Vec<String> = facts.iter().map(line).collect();
    let mut expected = format!(
        "capture_kind=typed-events-v1\nsession={session_hex}\nsequence_before={sequence_before}\nsequence_after={sequence_after}\nattribution_selected={attribution_selected}\ncoverage=totals-and-typed-events;flash-not-exercised\nevent_count={}\nkills={kills}\nraw_xp={raw}\ngained_xp={gained}\ngold={gold}\nloot_items={items}\nfacts={}\n",
        events.len(),
        lines.join("|")
    );
    for (index, event) in events.iter().enumerate() {
        expected.push_str(&format!("event_{index}={event}\n"));
    }
    capture(expected.as_bytes())?;
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(900), verdict::observe(journey(clients)))
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
