//! W24 client replay capture: the private ambient-sound domain (101). The outside join cue
//! (`cue_id: 0`) and the city replacement cue a real STEP route produces are the
//! `SessionEvent::Presentation` batches the session applied; the session owns the decoded
//! events, so (the C17c practice) each batch is re-encoded with the production
//! `encode_presentation_batch` and labelled as such in `crates/session/src/tests/e2e_w24.rs`.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::presentation::{
    PresentationEvent, PresentationEventKind, encode_presentation_batch,
};

use super::full_row_w24::{city_replacement, clock_snapshot, zone};

fn ambient_batch(
    client: &mut DevClientSession,
    minute: &mut u16,
) -> TestResult<Option<(PresentationEvent, u16)>> {
    let mut batch = None;
    for event in client.take_events() {
        match event {
            SessionEvent::WorldClock(delta) => *minute = delta.value.tibian_minute,
            SessionEvent::Presentation(delta) => {
                for event in delta.value {
                    if matches!(event.kind, PresentationEventKind::AmbientSound { .. })
                        && batch.replace((event, *minute)).is_some()
                    {
                        return Err("step=ambient unexpected multiple cues".into());
                    }
                }
            }
            _ => {}
        }
    }
    Ok(batch)
}

async fn wait_for_ambient(
    client: &mut DevClientSession,
    minute: &mut u16,
) -> TestResult<Option<(PresentationEvent, u16)>> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(4);
    loop {
        if let Some(batch) = ambient_batch(client, minute)? {
            return Ok(Some(batch));
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Ok(None);
        }
        client
            .service_liveness(remaining.min(Duration::from_millis(25)))
            .await?;
    }
}

async fn step(
    client: &mut DevClientSession,
    world: &BundleWorld,
    direction: StepDirection,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let expected =
        destination(world, from, direction).ok_or("step=walk missing planned destination")?;
    let reply = client.step(direction).await?;
    let actual = tile(client.world_spatial().actor_position)?;
    if reply.disposition != StepDisposition::Moved
        || reply.world_spatial_delta.is_none()
        || actual != expected
    {
        return Err(format!(
            "step=walk disposition={:?} from={from:?} expected={expected:?} actual={actual:?}",
            reply.disposition
        )
        .into());
    }
    Ok(())
}

fn city_cue(city: &str, event: PresentationEvent, minute: u16) -> TestResult<u32> {
    if let PresentationEventKind::AmbientSound { cue_id } = event.kind
        && city_replacement(city, &[(cue_id, minute)])
    {
        return Ok(cue_id);
    }
    Err(format!(
        "step=city-cue city={city} minute={minute} observed={:?}",
        event.kind
    )
    .into())
}

async fn journey(clients: &SeamClients<'_>, world: &BundleWorld) -> TestResult<String> {
    let mut client = connect_character(clients, 0x9d, 1).await?;
    if !client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::presentation::CAPABILITY_AMBIENT_SOUND_V1)
    {
        return Err("step=admission ambient capability 288 not selected".into());
    }
    let mut minute = clock_snapshot(&mut client)?;
    let from = tile(client.world_spatial().actor_position)?;
    if let Some(city) = zone(world, from) {
        let (event, observed_minute) = wait_for_ambient(&mut client, &mut minute)
            .await?
            .ok_or("step=starting-city no active ambient cue at admission")?;
        city_cue(city, event, observed_minute)?;
        let path = route(world, from, |at| zone(world, at).is_none())
            .ok_or("step=outside no compiled player route out of the starting city")?;
        for direction in path {
            step(&mut client, world, direction).await?;
        }
        let outside = client.world_spatial().actor_position;
        if zone(world, tile(outside)?).is_some() {
            return Err("step=outside route ended inside the city".into());
        }
        if client.session_mut().logout().await?.result
            != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
        {
            return Err("step=outside logout refused".into());
        }
        drop(client);
        client = connect_character(clients, 0x9d, 1).await?;
        minute = clock_snapshot(&mut client)?;
        if client.world_spatial().actor_position != outside {
            return Err("step=outside fresh admission changed the saved position".into());
        }
    }
    let (outside, _) = wait_for_ambient(&mut client, &mut minute)
        .await?
        .ok_or("step=outside no zero ambient cue at admission")?;
    if !matches!(
        outside.kind,
        PresentationEventKind::AmbientSound { cue_id: 0 }
    ) {
        return Err(format!("step=outside incorrect admission cue {:?}", outside.kind).into());
    }
    let from = tile(client.world_spatial().actor_position)?;
    let path = route(world, from, |at| zone(world, at).is_some())
        .ok_or("step=city-entry no compiled player route into a supported city")?;
    for direction in path {
        step(&mut client, world, direction).await?;
        if zone(world, tile(client.world_spatial().actor_position)?).is_none()
            && ambient_batch(&mut client, &mut minute)?.is_some()
        {
            return Err("step=city-entry unexpected ambient cue before crossing into city".into());
        }
    }
    let city = zone(world, tile(client.world_spatial().actor_position)?)
        .ok_or("step=city-entry route ended outside the city")?;
    let (entry, observed_minute) = wait_for_ambient(&mut client, &mut minute)
        .await?
        .ok_or("step=city-entry no ambient replacement after walking into the city")?;
    let city_cue = city_cue(city, entry, observed_minute)?;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=city-entry logout refused".into());
    }
    let root = std::env::var_os("E2E_ALL_W24_CAPTURE_DIR")
        .or_else(|| std::env::var_os("E2E_ALL_CAPTURE_DIR"))
        .ok_or("step=replay capture directory missing")?;
    let root = Path::new(&root);
    let outside_payload =
        encode_presentation_batch(&[outside]).map_err(|error| format!("encode {error:?}"))?;
    let entry_payload =
        encode_presentation_batch(&[entry]).map_err(|error| format!("encode {error:?}"))?;
    std::fs::write(root.join("w24-outside.bin"), outside_payload)?;
    std::fs::write(root.join("w24-city.bin"), entry_payload)?;
    std::fs::write(
        root.join("w24-expected.txt"),
        format!("outside=0\ncity={city_cue}\n"),
    )?;
    Ok(format!("outside=0 city={city_cue}"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>, world: &BundleWorld) {
    match verdict::indexed_timeout(
        Duration::from_secs(240),
        verdict::observe(journey(clients, world)),
    )
    .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W24.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W24.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=W24.client status=FAIL evidence=timeout"),
    }
}
