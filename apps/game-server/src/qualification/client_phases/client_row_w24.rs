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

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

fn batches(client: &mut DevClientSession) -> Vec<Vec<PresentationEvent>> {
    client
        .take_events()
        .into_iter()
        .filter_map(|event| match event {
            SessionEvent::Presentation(delta) => Some(delta.value),
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
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(240);
    let mut client = connect_character(clients, 0x9d, 1).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        if !client
            .selected_capabilities()
            .contains(&oteryn_protocol_oteryn::presentation::CAPABILITY_AMBIENT_SOUND_V1)
        {
            return Err("step=admission ambient capability 288 not selected".into());
        }
        let initial = batches(&mut client);
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
            let reply = client.step_retrying(direction).await?;
            if reply.disposition != StepDisposition::Moved {
                return Err(format!("step=walk disposition={:?}", reply.disposition).into());
            }
            for events in batches(&mut client) {
                if cue(&events).is_some_and(|cue| cue != 0) && entry.is_none() {
                    entry = Some(events);
                }
            }
        }
        let entry = entry
            .ok_or("step=city-entry no non-zero ambient replacement after walking into the city")?;
        let city_cue = cue(&entry).ok_or("step=city-entry cue absent")?;
        let outside_payload =
            encode_presentation_batch(&outside).map_err(|error| format!("encode {error:?}"))?;
        let entry_payload =
            encode_presentation_batch(&entry).map_err(|error| format!("encode {error:?}"))?;
        capture("w24-outside.bin", &outside_payload)?;
        capture("w24-city.bin", &entry_payload)?;
        capture(
            "w24-expected.txt",
            format!("outside=0\ncity={city_cue}\n").as_bytes(),
        )?;

        Ok(format!("outside=0 city={city_cue}"))
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>, world: &BundleWorld) {
    match row_complete(journey(clients, world)).await {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ambient_cue_preserves_zero_and_high_ids() {
        assert_eq!(cue(&[]), None);
        for cue_id in [0, u32::MAX] {
            let events = [PresentationEvent {
                decision_ordinal: 1,
                emission_ordinal: 1,
                kind: PresentationEventKind::AmbientSound { cue_id },
            }];
            assert_eq!(cue(&events), Some(cue_id));
        }
    }
}
