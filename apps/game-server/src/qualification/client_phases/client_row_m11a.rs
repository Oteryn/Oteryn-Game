//! Captures the native-client projection inputs for SCENARIO_MATRIX M11a.
use super::*;
use super::{
    row_connect_world as connect_world,
    row_finish as row_logout,
};
use oteryn_dev_client::{ChatIntent, ChatSpeechMode, SessionEvent};
use oteryn_protocol_oteryn::{actor_spell, world_spatial};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if let Err(error) = capture(clients).await {
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=M11a.client status=FAIL evidence={error}"
        ));
    }
}

async fn capture(clients: &SeamClients<'_>) -> TestResult {
    row_complete(async {
    let mut client = connect_world(clients, 0xb3).await?;
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(300);
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        let mut captures = Vec::new();
        for (name, words) in [("up", "exani hur up"), ("down", "exani hur down")] {
            client.take_events();
            let reply = client
                .chat(&ChatIntent::Say {
                    mode: ChatSpeechMode::Say,
                    text: words.into(),
                })
                .await?;
            client.service_liveness(Duration::from_millis(100)).await?;
            let events = client.take_events();
            let spatial = events.iter().find_map(|event| match event {
                SessionEvent::WorldSpatial(delta) => Some(delta.value),
                SessionEvent::WorldSpatialEntities(delta) => {
                    Some(world_spatial::WorldSpatialObservation {
                        content_generation: delta.value.content_generation,
                        actor_position: delta.value.actor_position,
                    })
                }
                _ => None,
            });
            let vitals = events.iter().find_map(|event| match event {
                SessionEvent::ActorVitals(delta) => Some(delta.value),
                _ => None,
            });
            captures.push((name, reply.status, spatial, vitals));
            let deadline = tokio::time::Instant::now() + Duration::from_millis(2_100);
            while tokio::time::Instant::now() < deadline {
                client.service_liveness(Duration::from_millis(100)).await?;
                client.take_events();
            }
        }

        if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            let write_result = write_captures(Path::new(&directory), &captures);
            write_result?;
        }
        Ok(())
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    tokio::time::timeout(Duration::from_secs(10), row_logout(&mut client)).await??;
    row_result

    }).await?
}

fn write_captures(
    directory: &Path,
    captures: &[(
        &str,
        CommandStatus,
        Option<world_spatial::WorldSpatialObservation>,
        Option<actor_spell::ActorVitals>,
    )],
) -> TestResult {
    let mut expected = String::new();
    for (name, status, spatial, vitals) in captures {
        let spatial = spatial.ok_or_else(|| format!("{name} had no spatial delta"))?;
        let vitals = vitals.ok_or_else(|| format!("{name} had no vitals delta"))?;
        std::fs::write(
            directory.join(format!("m11a-{name}-spatial.bin")),
            world_spatial::encode_world_spatial(&spatial),
        )?;
        std::fs::write(
            directory.join(format!("m11a-{name}-vitals.bin")),
            actor_spell::encode_actor_vitals(&vitals)
                .map_err(|error| format!("encode {name} vitals: {error:?}"))?,
        )?;
        expected.push_str(&format!(
            "{name} {status:?} {} {} {} {} {}\n",
            spatial.actor_position.x,
            spatial.actor_position.y,
            spatial.actor_position.floor,
            vitals.mana,
            vitals.soul
        ));
    }
    std::fs::write(directory.join("m11a-expected.txt"), expected)?;
    Ok(())
}
