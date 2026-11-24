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

pub(super) const LIGHT_CHARACTER: usize = 2;
const WORDS: &str = "utevo lux";

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("W20B_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

fn hex(bytes: &[u8; 16]) -> String {
    hex_bytes(bytes)
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(super) async fn connect_player_caster(
    clients: &SeamClients<'_>,
    required_level: u32,
    premium_required: bool,
    hunt_budget: Duration,
) -> TestResult<oteryn_dev_client::DevClientSession> {
    let mut client = connect_character(clients, 0x97, LIGHT_CHARACTER).await?;
    if premium_required
        && let Err(error) = full_e2ef7::require_current_light_premium(&mut client).await
    {
        // The packet capture reuses this Character after the full journey.
        let cleanup = logout_retrying_busy(&mut client).await;
        if !matches!(cleanup, Ok((Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted), ..))) {
            return Err(format!("{error}; personal-light preflight logout: {cleanup:?}").into());
        }
        return Err(error);
    }
    if full_e2ef7::player_level(&mut client).await? < required_level {
        let world = full_e2ef7::qualification_world(clients)?;
        full_e2ef7::earn_player_level(&mut client, world, required_level, hunt_budget).await?;
    }
    Ok(client)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let spell = native_spell_book()?
        .spoken(WORDS)
        .ok_or("missing authored Light spell")?
        .spell;
    let mut client = connect_player_caster(
        clients, spell.level, spell.premium, Duration::from_secs(60),
    ).await?;
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
            return Err("step=light the Character's own light did not appear after the cast".into());
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
    capture("w20b-entities.bin", &payload);
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
    );
    let _ = client.session_mut().logout().await;
    Ok(format!("light={}:{}", light.0, light.1))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await {
        Ok(Ok(observed)) => evidence(&format!("e2e_all scenario=W20b.client status=CAPTURED evidence={observed}")),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W20b.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=W20b.client status=FAIL evidence=timeout"),
    }
}

#[test]
fn qualification_review_w20b_never_uses_privileged_profile() -> TestResult {
    let capture = include_str!("client_row_w20b.rs");
    let (_, journey) = include_str!("full_e2ef7.rs")
        .split_once("async fn light_journey(")
        .ok_or("missing W20b full journey")?;
    let (journey, _) = journey.split_once("\nasync fn ").ok_or("missing W20b journey boundary")?;
    assert!(!capture.contains(concat!("provisioning::", "provision_profile")));
    assert!(!journey.contains(concat!("client_row_w20b::", "connect_caster")));
    assert!(journey.contains(concat!("client_row_w20b::", "connect_player_caster")));
    let (setup, packet) = capture.split_once("async fn journey(").ok_or("missing W20b packet journey")?;
    assert!(setup.contains(concat!("full_e2ef7::", "earn_player_level")));
    let (packet, _) = packet.split_once("pub(super) async fn run_packet(").ok_or("missing W20b runner")?;
    assert!(packet.contains(concat!("connect_player_", "caster(")));
    Ok(())
}
