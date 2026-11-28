//! C10 native-client capture: exercise a real soul-costing cast, its immediate
//! cooldown refusal, and kill-funded soul regeneration. The actor-vitals bytes
//! are replayed by `oteryn-session`'s C10 client test.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::actor_spell::{self, SpellCastDisposition, SpellTarget};
use super::full_row_c10::{prepare_caster, progress, record_cast_evidence, soul_spell};
use std::num::NonZeroU32;
use std::path::Path;
use std::time::Duration;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    // The authored approach takes 107 paced steps before the combat and soul clocks.
    let result = verdict::indexed_timeout(Duration::from_secs(450), verdict::observe(journey(clients))).await;
    let status = match result {
        Ok(Ok(())) => "PASS",
        Ok(Err(error)) => {
            evidence(&format!("C10_CLIENT_GAP {error}"));
            "FAIL"
        }
        Err(_) => {
            evidence("C10_CLIENT_GAP capture journey timed out");
            "FAIL"
        }
    };
    evidence(&format!(
        "e2e_all scenario=C10.client status={status} seconds={:.3}",
        started.elapsed().as_secs_f64()
    ));
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let generation = load_world_generation(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let world = &generation.bundle;
    let identity = character_prep::dedicated_character(clients, "NODE_BOOT_C10_CLIENT_ACCOUNT").await?;
    let caster_clients = identity.clients(clients);
    let mut client = prepare_caster(&caster_clients, "C10.client", 0xc1).await?;
    let before = client.actor_vitals().copied().ok_or("C10 client vitals absent")?;
    let spell = soul_spell(&mut client).await?;
    let index = NonZeroU32::new(spell.cast_index).ok_or("C10 client zero cast index")?;
    let cast = client.cast_spell(index, SpellTarget::None, false).await?;
    record_cast_evidence("C10.client", &spell, &cast);
    if cast.disposition != SpellCastDisposition::Cast {
        return Err(format!("C10 client cast reply: {cast:?}").into());
    }
    let expected = before.soul.checked_sub(spell.soul).ok_or("C10 client insufficient soul")?;
    let debit = cast.actor_vitals_delta
        .filter(|delta| delta.value.soul == expected)
        .ok_or("C10 client cast did not carry the exact soul debit")?;
    let refused = client.cast_spell(index, SpellTarget::None, false).await?;
    record_cast_evidence("C10.client", &spell, &refused);
    if refused.disposition != SpellCastDisposition::CoolingDown
        || client.actor_vitals() != Some(&debit.value)
    {
        return Err(format!("C10 client cooldown changed state: {refused:?}").into());
    }
    let (xp_before, level) = progress(&mut client).await?;
    let creature = hunt_for_soul(&mut client, world, &metadata.spawns, &generation.rewards, level).await?;
    let (xp_after, _) = progress(&mut client).await?;
    if xp_after.checked_sub(xp_before).is_none_or(|earned| earned < u64::from(level)) {
        return Err(format!("C10 client kill {creature} observed xp={xp_before}->{xp_after}, need at least {level}").into());
    }
    let regenerated = wait_soul(&mut client, |soul| soul == debit.value.soul + 1, Duration::from_secs(130)).await?;
    capture(&before, &debit, &regenerated)
}

async fn hunt_for_soul<'a>(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &'a oteryn_world_bundle::spawn::Table,
    rewards: &crate::content::creature_reward::CreatureRewardTable,
    level: u32,
) -> TestResult<&'a str> {
    use oteryn_dev_client::EntityKind;
    use std::collections::BTreeMap;

    let from = tile(client.world_spatial().actor_position)?;
    let eligible: Vec<_> = spawns.creatures.iter().map(|creature| {
        rewards.row(&creature.key).is_ok_and(|reward| reward.xp_amount.get() >= i64::from(level))
    }).collect();
    let mut approaches = BTreeMap::new();
    for source in &spawns.sources {
        for point in &source.points {
            if !eligible.get(point.creature as usize).copied().unwrap_or(false)
                || source.floor.abs_diff(from.floor) > 2
                || point.x.abs_diff(from.x) > 161 || point.y.abs_diff(from.y) > 161
            {
                continue;
            }
            let creature = spawns.creatures[point.creature as usize].key.as_str();
            for dx in -1..=1 {
                for dy in -1..=1 {
                    if (dx, dy) == (0, 0) {
                        continue;
                    }
                    let Some((x, y)) = point.x.checked_add_signed(dx).zip(point.y.checked_add_signed(dy)) else { continue };
                    let at = TilePos { x, y, floor: source.floor };
                    if world.base().tile(x, y, source.floor).is_some_and(|tile| !tile.protection_zone()) {
                        approaches.entry(at).or_insert(creature);
                    }
                }
            }
        }
    }
    let path = route(world, from, |at| approaches.contains_key(&at))
        .ok_or_else(|| format!("C10 no reachable authored kill worth at least {level} XP"))?;
    let approach = path.iter().try_fold(from, |at, &dir| destination(world, at, dir))
        .ok_or("C10 planned hunt route has no landing")?;
    let creature = *approaches.get(&approach).ok_or("C10 hunt route has no authored target")?;
    let expected_name = &rewards.encounter_facts(creature)
        .ok_or("C10 authored kill target has no native encounter profile")?.display_name;
    evidence(&format!("C10_HUNT_PLAN creature={creature} approach={approach:?} steps={}", path.len()));
    walk_strict(client, world, &path).await?;

    let position = client.world_spatial().actor_position;
    let adjacent = client.world_entities().and_then(|entities| {
        entities.iter().find(|entity| {
            entity.kind == EntityKind::Creature && entity.position.floor == position.floor
                && entity.position.x.abs_diff(position.x).max(entity.position.y.abs_diff(position.y)) <= 1
        })
    }).ok_or("C10 authored hunt approach has no adjacent creature")?;
    let observed_name = adjacent.display_name.and_then(|name| name.as_str().ok().map(str::to_owned));
    if observed_name.as_deref() != Some(expected_name.as_str()) {
        return Err(format!("C10 hunt first adjacent creature={observed_name:?} expected={expected_name:?}").into());
    }
    evidence(&format!("C10_HUNT_TARGET creature={creature} entity={:?}", adjacent.entity));
    verdict::Harness::global().bounded_wait(tokio::time::Instant::now() + Duration::from_secs(130), "C10-combat");
    client_hunt::hunt(client).await.map_err(|error| {
        verdict::record_error(verdict::IndexedError::as_error(&error));
        verdict::context(format!("C10 hunt creature={creature}: {error}"), error)
    })?;
    Ok(creature)
}

async fn walk_strict(client: &mut DevClientSession, world: &BundleWorld, path: &[StepDirection]) -> TestResult {
    for &direction in path {
        let before = tile(client.world_spatial().actor_position)?;
        let expected = destination(world, before, direction).ok_or("C10 route has no landing")?;
        let step = client.step(direction).await?;
        let after = tile(client.world_spatial().actor_position)?;
        if step.disposition != StepDisposition::Moved || step.world_spatial_delta.is_none() || after != expected {
            return Err(format!("C10 first step {direction:?} from={before:?} expected={expected:?} observed={after:?} reply={step:?}").into());
        }
        verdict::Harness::global().progress("C10-walk");
        client.take_events();
    }
    Ok(())
}

async fn wait_soul(
    client: &mut DevClientSession,
    wanted: impl Fn(u32) -> bool,
    duration: Duration,
) -> TestResult<oteryn_dev_client::AppliedDelta<actor_spell::ActorVitals>> {
    let deadline = tokio::time::Instant::now() + duration;
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        for event in client.take_events() {
            if let SessionEvent::ActorVitals(delta) = event
                && wanted(delta.value.soul)
            {
                return Ok(delta);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("C10 client soul delta deadline".into());
        }
    }
}

fn capture(
    before: &actor_spell::ActorVitals,
    debit: &oteryn_dev_client::AppliedDelta<actor_spell::ActorVitals>,
    regenerated: &oteryn_dev_client::AppliedDelta<actor_spell::ActorVitals>,
) -> TestResult {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    let write = || -> TestResult {
        let path = Path::new(&path);
        let encode = |v| {
            actor_spell::encode_actor_vitals(v)
                .map_err(|error| format!("C10 actor-vitals encode: {error:?}"))
        };
        std::fs::write(path.join("c10-saved-rest.bin"), encode(before)?)?;
        std::fs::write(path.join("c10-debit.bin"), encode(&debit.value)?)?;
        std::fs::write(path.join("c10-regenerated.bin"), encode(&regenerated.value)?)?;
        std::fs::write(
            path.join("c10-expected.txt"),
            format!(
                "{} {} {}\n",
                before.soul,
                debit.value.soul,
                regenerated.value.soul
            ),
        )?;
        Ok(())
    };
    write()
}
