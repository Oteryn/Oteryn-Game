//! W20b client replay capture: casting Light (`utevo lux`) as a player gives the caster's own
//! entity a light the client reads from `Session::visible_appearances` (`light`). The spatial
//! entity delta that carries it is session-owned, so (the C17c practice) it is re-encoded with the
//! production encoder from the session's own visible entity and labelled as such in
//! `crates/session/src/tests/e2e_w20b.rs`. A cast the starter cannot afford is printed as FAIL.
use super::*;
use oteryn_dev_client::{ChatDisposition, ChatIntent, ChatSpeechMode};
use oteryn_protocol_oteryn::world_spatial_entities::{
    EntityFields, EntityLight, EntityLights, WorldSpatialEntitiesDelta,
    encode_world_spatial_entities_delta_with_fields,
};

const WORDS: &str = "utevo lux";

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

fn hex(bytes: &[u8; 16]) -> String {
    hex_bytes(bytes)
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    let mut client = connect_world(clients, 0x97).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    let own = *client.world_entities().ok_or("step=entities absent")?.own_identity();
    let before = client
        .session_mut()
        .visible_appearances()
        .into_iter()
        .find(|entry| entry.entity.entity.identity == own)
        .ok_or("step=own entity absent")?;
    if before.light.is_some() {
        return Err("step=precondition the Character already carries a light".into());
    }
    client.take_events();
    let cast = client
        .chat(&ChatIntent::Say { mode: ChatSpeechMode::Say, text: WORDS.into() })
        .await?;
    if cast.status != CommandStatus::Accepted || cast.disposition != ChatDisposition::Ok {
        return Err(format!("step=cast status={:?} disposition={:?}", cast.status, cast.disposition).into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    let (entity, light) = loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        let seen = client
            .session_mut()
            .visible_appearances()
            .into_iter()
            .find(|entry| entry.entity.entity.identity == own);
        if let Some(entry) = seen
            && let Some(light) = entry.light
        {
            break (entry.entity, light);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step=light the Character's own light did not appear after the cast; the ordinary starter lacks the level (full_e2ef7 W20b)".into());
        }
    };
    let position = client.world_spatial().actor_position;
    let delta = WorldSpatialEntitiesDelta {
        content_generation: client.world_spatial().content_generation,
        actor_position: position,
        enter: vec![],
        update: vec![entity],
        leave: vec![],
    };
    let payload = encode_world_spatial_entities_delta_with_fields(
        &delta,
        EntityFields::selected(&[6, 19]),
        &EntityLights::from([(own, EntityLight { level: light.0, color: light.1 })]),
    )
    .map_err(|error| format!("step=encode {error:?}"))?;
    capture("w20b-entities.bin", &payload)?;
    capture(
        "w20b-expected.txt",
        format!(
            "words={WORDS}\nidentity={}\ngeneration={}\nlight={}:{}\ncontent={}\n",
            hex(&own),
            entity.entity.generation,
            light.0,
            light.1,
            hex_bytes(&client.world_spatial().content_generation)
        )
        .as_bytes(),
    )?;

    Ok(format!("light={}:{}", light.0, light.1))

    }).await.unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=W20b.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W20b.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=W20b.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_preserves_full_identity_width() {
        assert_eq!(hex(&[0; 16]), "00".repeat(16));
        assert_eq!(hex(&[255; 16]), "ff".repeat(16));
    }

    #[test]
    fn hex_bytes_preserves_zero_padding_and_high_bytes() {
        assert_eq!(hex_bytes(&[]), "");
        assert_eq!(hex_bytes(&[0, 15, 128, 255]), "000f80ff");
    }
}
