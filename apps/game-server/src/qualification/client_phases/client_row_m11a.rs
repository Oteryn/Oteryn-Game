//! Captures the native-client projection inputs for SCENARIO_MATRIX M11a.
use super::*;
use oteryn_dev_client::{ChatIntent, ChatSpeechMode, SessionEvent};
use oteryn_protocol_oteryn::{actor_spell, world_spatial};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = verdict::indexed_timeout(Duration::from_secs(45), verdict::observe(capture(clients)))
        .await.unwrap_or_else(|_| Err("M11a levitate journey deadline".into()));
    match result {
        Ok(()) => evidence("e2e_all scenario=M11a.client status=PASS evidence=up-and-down-with-vitals-debits"),
        Err(error) => evidence(&format!("e2e_all scenario=M11a.client status=FAIL evidence={error}")),
    }
}

fn horizontal_landing_matches(from: ActorPosition, observed: ActorPosition, offset: (i32, i32)) -> bool {
    from.x.checked_add(offset.0) == Some(observed.x) && from.y.checked_add(offset.1) == Some(observed.y)
}

fn levitate_matches(
    accepted: bool, before_floor: i16, after_floor: Option<i16>,
    up: bool, mana_before: u32, mana_after: Option<u32>,
) -> bool {
    let next_floor = before_floor.checked_add(if up { -1 } else { 1 });
    accepted && after_floor.is_some() && next_floor == after_floor
        && mana_after.is_some() && mana_before.checked_sub(50) == mana_after
}

async fn capture(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0xb3).await?;
    let directory = std::env::var_os("E2E_ALL_CAPTURE_DIR");
    let mut expected = String::new();
    for (name, words) in [("up", "exani hur up"), ("down", "exani hur down")] {
        let from = client.world_spatial().actor_position;
        let offset = match client_row_m01::own_direction(&client) {
            Some(StepDirection::North) => (0, -1),
            Some(StepDirection::South) => (0, 1),
            Some(StepDirection::East) => (1, 0),
            Some(StepDirection::West) => (-1, 0),
            _ => return Err("M11a no cardinal facing before Levitate".into()),
        };
        let mana = client.actor_vitals().ok_or("M11a pre-cast vitals missing")?.mana;
        client.take_events();
        let reply = client.chat(&ChatIntent::Say {
            mode: ChatSpeechMode::Say,
            text: words.into(),
        }).await?;
        if reply.status != CommandStatus::Accepted || reply.disposition != oteryn_dev_client::ChatDisposition::Ok {
            return Err(format!("M11a {name} levitate refused: {reply:?}").into());
        }
        let mut spatial = None;
        let mut vitals = None;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        let (spatial, vitals) = 'observation: loop {
            for event in client.take_events() {
                match event {
                    SessionEvent::WorldSpatial(delta) if delta.server_sequence > reply.result_server_sequence => {
                        spatial = Some(delta.value);
                    }
                    SessionEvent::WorldSpatialEntities(delta) if delta.server_sequence > reply.result_server_sequence => {
                        spatial = Some(world_spatial::WorldSpatialObservation {
                            content_generation: delta.value.content_generation,
                            actor_position: delta.value.actor_position,
                        });
                    }
                    SessionEvent::ActorVitals(delta) if delta.server_sequence > reply.result_server_sequence => {
                        vitals = Some(delta.value);
                    }
                    _ => {}
                }
                if let (Some(spatial), Some(vitals)) = (spatial, vitals)
                    && levitate_matches(true, from.floor, Some(spatial.actor_position.floor), name == "up", mana, Some(vitals.mana))
                    && horizontal_landing_matches(from, spatial.actor_position, offset)
                {
                    break 'observation (spatial, vitals);
                }
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("M11a {name} missing successful step and mana debit: from={from:?} spatial={spatial:?} pre_mana={mana} vitals={vitals:?}").into());
            }
            client.service_liveness(Duration::from_millis(25)).await?;
        };
        if let Some(directory) = directory.as_deref() {
            let directory = Path::new(directory);
            std::fs::write(directory.join(format!("m11a-{name}-spatial.bin")), world_spatial::encode_world_spatial(&spatial))?;
            std::fs::write(directory.join(format!("m11a-{name}-vitals.bin")),
                actor_spell::encode_actor_vitals(&vitals).map_err(|error| format!("M11a {name} vitals encoding: {error:?}"))?)?;
            expected.push_str(&format!(
                "{name} Accepted {} {} {} {} {}\n",
                spatial.actor_position.x, spatial.actor_position.y, spatial.actor_position.floor, vitals.mana, vitals.soul
            ));
        }
        if name == "up" {
            let until = tokio::time::Instant::now() + Duration::from_millis(2_100);
            while tokio::time::Instant::now() < until {
                client.service_liveness(Duration::from_millis(100)).await?;
                client.take_events();
            }
        }
    }
    if let Some(directory) = directory {
        std::fs::write(Path::new(&directory).join("m11a-expected.txt"), expected)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levitate_requires_accepted_cast_floor_transition_and_cost() {
        let from = ActorPosition {x: 20, y: 30, floor: 7};
        assert!(!horizontal_landing_matches(from, ActorPosition {y: 31, floor: 6, ..from}, (1, 0)));
        assert!(horizontal_landing_matches(from, ActorPosition {x: 21, floor: 6, ..from}, (1, 0)));
        assert!(!levitate_matches(false, 7, Some(6), true, 70, Some(20)));
        assert!(!levitate_matches(true, 7, None, true, 70, Some(20)));
        assert!(!levitate_matches(true, 7, Some(8), true, 70, Some(20)));
        assert!(!levitate_matches(true, 7, Some(6), true, 70, None));
        assert!(levitate_matches(true, 7, Some(6), true, 70, Some(20)));
        assert!(levitate_matches(true, 6, Some(7), false, 70, Some(20)));
    }
}
