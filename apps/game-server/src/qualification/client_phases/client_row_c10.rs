//! C10 native-client capture: exercise a real soul-costing cast, its immediate
//! cooldown refusal, and kill-funded soul regeneration. The actor-vitals bytes
//! are replayed by `oteryn-session`'s C10 client test.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::actor_spell::{self, SpellCastDisposition, SpellTarget};
use super::full_row_c10::{prepare_caster, progress, soul_spell, soul_window_target};
use std::num::NonZeroU32;
use std::path::Path;
use std::time::Duration;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(180), verdict::observe(journey(clients))).await;
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
    if cast.disposition != SpellCastDisposition::Cast {
        return Err(format!("C10 client cast reply: {cast:?}").into());
    }
    let expected = before.soul.checked_sub(spell.soul).ok_or("C10 client insufficient soul")?;
    let debit = cast.actor_vitals_delta
        .filter(|delta| delta.value.soul == expected)
        .ok_or("C10 client cast did not carry the exact soul debit")?;
    let refused = client.cast_spell(index, SpellTarget::None, false).await?;
    if refused.disposition != SpellCastDisposition::CoolingDown
        || client.actor_vitals() != Some(&debit.value)
    {
        return Err(format!("C10 client cooldown changed state: {refused:?}").into());
    }
    let (xp_before, level) = progress(&mut client).await?;
    let creature = soul_window_target(world, &metadata.spawns, &generation.rewards,
        tile(client.world_spatial().actor_position)?, level)?;
    travel_to_spawn(&mut client, world, &metadata.spawns, creature).await?;
    client_hunt::hunt(&mut client)
        .await
        .map_err(|error| { verdict::record_error(verdict::IndexedError::as_error(&error)); verdict::context(format!("C10 client hunt: {error}"), error) })?;
    let (xp_after, _) = progress(&mut client).await?;
    if xp_after.checked_sub(xp_before).is_none_or(|earned| earned < u64::from(level)) {
        return Err(format!("C10 client kill {creature} observed xp={xp_before}->{xp_after}, need at least {level}").into());
    }
    let regenerated = wait_soul(&mut client, |soul| soul == debit.value.soul + 1, Duration::from_secs(130)).await?;
    capture(&before, &debit, &regenerated)
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
