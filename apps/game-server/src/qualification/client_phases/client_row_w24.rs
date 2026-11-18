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

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "W24_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
}

fn batches(client: &mut DevClientSession, after: u64) -> Vec<Vec<PresentationEvent>> {
    client
        .take_events()
        .into_iter()
        .filter_map(|event| match event {
            SessionEvent::Presentation(delta) if delta.server_sequence > after => Some(delta.value),
            _ => None,
        })
        .map(|events| {
            events
                .into_iter()
                .filter(|event| matches!(event.kind, PresentationEventKind::AmbientSound { .. }))
                .collect::<Vec<_>>()
        })
        .filter(|events| !events.is_empty())
        .collect()
}

fn cue(events: &[PresentationEvent]) -> Option<u32> {
    events.iter().find_map(|event| match event.kind {
        PresentationEventKind::AmbientSound { cue_id } => Some(cue_id),
        _ => None,
    })
}

fn city_entry_cue(was_city: bool, is_city: bool, cue: Option<u32>) -> bool {
    !was_city && is_city && cue.is_some_and(|cue| cue != 0)
}

#[test]
fn ambient_cue_requires_the_city_crossing() {
    assert!(!city_entry_cue(false, false, Some(7)));
    assert!(!city_entry_cue(true, true, Some(7)));
    assert!(!city_entry_cue(false, true, Some(0)));
    assert!(city_entry_cue(false, true, Some(7)));
}

fn city(world: &BundleWorld, at: TilePos) -> bool {
    let table = &world.base().subareas;
    let Some((_, id)) = table.resolve(at.x, at.y, at.floor) else {
        return false;
    };
    let Some(area) = table.areas.iter().find(|area| area.id == id) else {
        return false;
    };
    let Some(key) = area.key.strip_prefix("oteryn:content.area.") else {
        return false;
    };
    let name = key.strip_prefix("city.").or_else(|| {
        key.strip_prefix("subregion.")
            .and_then(|rest| rest.strip_suffix("_city"))
    });
    matches!(
        name,
        Some("venore" | "thais" | "carlin" | "edron" | "kazordoon" | "port_hope")
    )
}

async fn journey(clients: &SeamClients<'_>, world: &BundleWorld) -> TestResult<String> {
    let mut client = connect_character(clients, 0x9d, 1).await?;
    if !client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::presentation::CAPABILITY_AMBIENT_SOUND_V1)
    {
        return Err("step=admission ambient capability 288 not selected".into());
    }
    let initial = batches(&mut client, 0);
    let outside = initial
        .iter()
        .find(|events| cue(events) == Some(0))
        .ok_or_else(|| {
            format!(
                "step=outside observed={:?}",
                initial.iter().map(|e| cue(e)).collect::<Vec<_>>()
            )
        })?
        .clone();
    let from = tile(client.world_spatial().actor_position)?;
    let path = route(world, from, |at| city(world, at))
        .ok_or("step=city-entry no compiled player route into a supported city")?;
    let mut entry = None;
    for direction in path {
        let was_city = city(world, tile(client.world_spatial().actor_position)?);
        let reply = client.step_retrying(direction).await?;
        if reply.disposition != StepDisposition::Moved {
            return Err(format!("step=walk disposition={:?}", reply.disposition).into());
        }
        let is_city = city(world, tile(client.world_spatial().actor_position)?);
        if was_city || !is_city {
            batches(&mut client, 0);
            continue;
        }
        let after = reply
            .world_spatial_delta
            .as_ref()
            .ok_or("step=city-entry spatial delta absent")?
            .server_sequence;
        entry = Some(
            tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    if let Some(events) = batches(&mut client, after)
                        .into_iter()
                        .find(|events| city_entry_cue(was_city, is_city, cue(events)))
                    {
                        return Ok::<_, oteryn_dev_client::DevClientError>(events);
                    }
                    client.service_liveness(Duration::from_millis(50)).await?;
                }
            })
            .await
            .map_err(|_| "step=city-entry no ambient replacement after crossing")??,
        );
        break;
    }
    let entry = entry
        .ok_or("step=city-entry no non-zero ambient replacement after walking into the city")?;
    let city_cue = cue(&entry).ok_or("step=city-entry cue absent")?;
    let outside_payload =
        encode_presentation_batch(&outside).map_err(|error| format!("encode {error:?}"))?;
    let entry_payload =
        encode_presentation_batch(&entry).map_err(|error| format!("encode {error:?}"))?;
    capture("w24-outside.bin", &outside_payload);
    capture("w24-city.bin", &entry_payload);
    capture(
        "w24-expected.txt",
        format!("outside=0\ncity={city_cue}\n").as_bytes(),
    );
    let _ = client.session_mut().logout().await;
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
