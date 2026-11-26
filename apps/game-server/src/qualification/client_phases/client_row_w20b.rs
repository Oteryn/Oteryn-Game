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
use std::io::Write as _;

pub(super) const LIGHT_CHARACTER: usize = 2;
const WORDS: &str = "utevo lux";
// Crystal 201189f8, data/scripts/spells/support/light.lua:5-8.
const EXPECTED_LIGHT: (u8, u8) = (6, 215);

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Err("W20b requires E2E_ALL_CAPTURE_DIR".into());
    };
    std::fs::File::create_new(Path::new(&root).join(name))
        .and_then(|mut file| file.write_all(bytes))
        .map_err(|error| format!("step=capture file={name} error={error}"))?;
    Ok(())
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
    preparation_budget: Duration,
) -> TestResult<oteryn_dev_client::DevClientSession> {
    let deadline = tokio::time::Instant::now() + preparation_budget;
    verdict::Harness::global().bounded_wait(deadline, "personal-light-preparation");
    tokio::time::timeout_at(deadline, prepare_player_caster(clients, required_level, premium_required))
        .await
        .map_err(|_| format!("personal-light preparation exceeded {preparation_budget:?}"))?
}

async fn prepare_player_caster(
    clients: &SeamClients<'_>,
    required_level: u32,
    premium_required: bool,
) -> TestResult<oteryn_dev_client::DevClientSession> {
    const PREPARATION: character_prep::Seeded = character_prep::Seeded {
        profile: "w20b-light",
        reason: "Light tests cast, expiry and relog with an eligible caster",
    };
    let mut client = PREPARATION.connect(clients, 0x97, LIGHT_CHARACTER).await?;
    if premium_required {
        character_prep::grant_premium(&client).await?;
    }
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
        return Err(format!("W20b fixture did not supply level {required_level}").into());
    }
    Ok(client)
}

async fn logout_once(client: &mut oteryn_dev_client::DevClientSession) -> TestResult {
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("W20b first logout refused: {logout:?}").into());
    }
    Ok(())
}

async fn require_sorcerer(client: &mut oteryn_dev_client::DevClientSession) -> TestResult {
    use oteryn_protocol_oteryn::cyclopedia_character as wire;
    if !client.selected_capabilities().contains(&wire::CAPABILITY_CYCLOPEDIA_CHARACTER_VOCATION_V1) {
        return Err("W20b requires the wire vocation extension".into());
    }
    let tab = wire::CharacterTab::GeneralStats as u8;
    let reply = client.session_mut().gated_command(
        wire::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
        wire::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
        &[tab, 0, 0, 1],
    ).await?;
    if reply.status != CommandStatus::Accepted || reply.payload.first() != Some(&tab)
        || reply.payload.last().copied() != wire::vocation_id("sorcerer")
    {
        return Err("W20b bootstrap must already be a Sorcerer".into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let spell = native_spell_book()?
        .spoken(WORDS)
        .ok_or("missing authored Light spell")?
        .spell;
    if spell.level != 8 || spell.premium {
        return Err("Light must require level 8 and no Premium".into());
    }
    let run_id = std::env::var("OTERYN_W20B_CAPTURE_ID")?;
    if run_id.len() != 32 || !run_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("W20b requires a fresh 128-bit capture ID".into());
    }
    let mut starter = connect_character(clients, 0x97, LIGHT_CHARACTER).await?;
    require_sorcerer(&mut starter).await?;
    request_fixture_control(
        uuid_text(starter.join_snapshot().game_session_id.as_bytes()), "--profile", "w20b-light-strict".into(),
    ).await?;
    logout_once(&mut starter).await?;
    let original = starter.join_snapshot().game_session_id;
    drop(starter);
    let mut client = connect_character(clients, 0x98, LIGHT_CHARACTER).await?;
    if client.join_snapshot().game_session_id == original
        || full_e2ef7::player_level(&mut client).await? != 26
    {
        return Err("W20b preparation did not restore a fresh level-26 admission".into());
    }
    require_sorcerer(&mut client).await?;
    let mana = client.actor_vitals().ok_or("W20b initial vitals absent")?.mana;
    let paid = mana.checked_sub(20).ok_or("W20b caster cannot pay 20 mana")?;
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
    if light != EXPECTED_LIGHT {
        return Err(format!("step=light expected={EXPECTED_LIGHT:?} observed={light:?}").into());
    }
    if client.actor_vitals().map(|vitals| vitals.mana) != Some(paid) {
        return Err(format!("W20b cast must debit 20 mana: before={mana} after={:?}", client.actor_vitals()).into());
    }
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
    let expected = format!(
        "run_id={run_id}\nwords={WORDS}\nidentity={}\ngeneration={}\nlight=6:215\ncontent={}\n",
        hex(&own),
        entity.entity.generation,
        hex_bytes(&client.world_spatial().content_generation)
    );
    let session = client.join_snapshot().game_session_id;
    logout_once(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(clients, 0x99, LIGHT_CHARACTER).await?;
    if fresh.join_snapshot().game_session_id == session
        || fresh.actor_vitals().map(|vitals| vitals.mana) != Some(paid)
    {
        return Err("W20b fresh admission did not retain the paid mana".into());
    }
    let clock = std::path::PathBuf::from(std::env::var_os("OTERYN_QUALIFICATION_CLOCK_FILE")
        .ok_or("W20b requires the disposable qualification clock")?);
    let initial: u64 = std::fs::read_to_string(&clock)?.trim().parse()?;
    let temporary = clock.with_extension("w20b-next");
    for (seconds, expected) in [(0, Some((6, 215))), (65, Some((5, 215))),
        (125, Some((4, 215))), (188, Some((3, 215))), (250, Some((2, 215))),
        (311, Some((1, 215))), (357, Some((1, 215))), (375, None)]
    {
        if seconds > 0 {
            std::fs::write(&temporary, (initial + seconds * 1000).to_string())?;
            std::fs::rename(&temporary, &clock)?;
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        loop {
            let own = *fresh.world_entities().ok_or("W20b restored entities absent")?.own_identity();
            let seen = fresh.session_mut().visible_appearances().into_iter()
                .find(|entry| entry.entity.entity.identity == own)
                .ok_or("W20b restored caster absent")?.light;
            if seen == expected {
                break;
            }
            if seconds == 0 || tokio::time::Instant::now() >= deadline {
                return Err(format!("W20b light at offset {seconds}s expected={expected:?} observed={seen:?}").into());
            }
            fresh.service_liveness(Duration::from_millis(50)).await?;
        }
    }
    logout_once(&mut fresh).await?;
    capture("w20b-entities.bin", &payload)?;
    capture("w20b-expected.txt", expected.as_bytes())?;
    Ok("light=6:215,mana=20,relog=restored,decay=verified,expiry=verified".into())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    character_prep::declare_seeded("W20b.client", "sorcerer-level-26", "native Light packet capture");
    match verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await {
        Ok(Ok(observed)) => evidence(&format!("e2e_all scenario=W20b.client status=PASS evidence={observed}")),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W20b.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=W20b.client status=FAIL evidence=timeout"),
    }
}

#[test]
fn qualification_review_w20b_uses_fenced_seeded_preparation() -> TestResult {
    let capture = include_str!("client_row_w20b.rs");
    let (_, journey) = include_str!("full_e2ef7.rs")
        .split_once("async fn light_journey(")
        .ok_or("missing W20b full journey")?;
    let (journey, _) = journey.split_once("\nasync fn ").ok_or("missing W20b journey boundary")?;
    assert!(!capture.contains(concat!("provisioning::", "provision_profile")));
    assert!(!journey.contains(concat!("client_row_w20b::", "connect_caster")));
    assert!(journey.contains(concat!("client_row_w20b::", "connect_player_caster")));
    let (setup, packet) = capture.split_once("async fn journey(").ok_or("missing W20b packet journey")?;
    assert!(setup.contains(concat!("character_prep::", "Seeded")));
    assert!(setup.contains(concat!("character_prep::", "grant_premium")));
    let (packet, _) = packet.split_once("pub(super) async fn run_packet(").ok_or("missing W20b runner")?;
    assert!(packet.contains(concat!("request_fixture_", "control(")));
    Ok(())
}
