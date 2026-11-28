//! C17c replays the original admission and cast frames through the client Session.
use super::*;
use oteryn_dev_client::native_session::{self, Session, SessionStream};
use oteryn_dev_client::{ChatDisposition, ChatIntent, ChatSpeechMode, DevClientSession};
use oteryn_protocol_oteryn::appearance::{self, Look};
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

const WORDS: &str = "utevo res ina \"rat\"";

const RAT: Look = Look::Outfit {
    look_type: 21,
    colours: [0; 4],
    addons: 0,
};

#[derive(Debug, PartialEq, Eq)]
struct CastEvidence {
    session: [u8; 16],
    result_sequence: u64,
    mapping_sequence: u64,
    last_sequence: u64,
    mana: u32,
    soul: u32,
}

async fn replay_capture(
    frames: &[u8],
    character: crate::foundation::CharacterId,
    live: &CastEvidence,
) -> TestResult {
    let root =
        std::env::var_os("E2E_ALL_CAPTURE_DIR").map_or_else(std::env::temp_dir, PathBuf::from);
    let path = root.join(format!(
        "c17c-{}-{}.wire",
        uuid_text(&live.session),
        live.last_sequence
    ));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)?;
    std::io::Write::write_all(&mut file, frames)?;
    drop(file);
    let recorded = std::fs::read(&path)?;
    if recorded != frames {
        return Err("C17c capture file does not match the live transcript".into());
    }

    let (stream, mut peer) = tokio::io::duplex(recorded.len().max(4096));
    peer.write_all(&recorded).await?;
    let mut client = Session::admit(
        stream,
        native_session::Admission {
            appearance_items: None,
            schema_revision: 1,
            character_id: character,
            // The offline peer consumes no client input. Never retain a real grant.
            admission_material: b"offline-replay",
            client_build_id: "oteryn-dev-client/world-qualification",
            supported_capabilities: oteryn_dev_client::CLIENT_SUPPORTED_CAPABILITIES,
            deadline: Duration::from_secs(5),
        },
    )
    .await?;
    let replayed = cast_rat(&mut client).await?;
    if &replayed != live {
        return Err(
            format!("C17c replay evidence mismatch: live={live:?} replay={replayed:?}").into(),
        );
    }
    drop(peer);
    evidence(&format!(
        "C17C_REPLAY status=PASS session={} result_sequence={} mapping_sequence={} final_sequence={}",
        uuid_text(&live.session),
        live.result_sequence,
        live.mapping_sequence,
        live.last_sequence
    ));
    Ok(())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("C17c logout refused: {reply:?}").into());
    }
    Ok(())
}

async fn prepared_stats<S: SessionStream>(client: &mut Session<S>) -> TestResult {
    use oteryn_protocol_oteryn::cyclopedia_character as wire;
    let request = [wire::CharacterTab::GeneralStats as u8, 0, 0, 1];
    let result = client
        .gated_command(wire::CAPABILITY_CYCLOPEDIA_CHARACTER_V1, 388, &request)
        .await?;
    if result.status != CommandStatus::Accepted {
        return Err("C17c GeneralStats refused".into());
    }
    let vocation = general_stats::validate(
        &result.payload,
        client
            .selected_capabilities()
            .contains(&wire::CAPABILITY_CYCLOPEDIA_CHARACTER_VOCATION_V1),
    )?;
    let level = u32::from_le_bytes(result.payload[9..13].try_into()?);
    if level != 23 || vocation != wire::vocation_id("sorcerer") {
        return Err(
            format!("C17c seeded stats differ: level={level} vocation={vocation:?}").into(),
        );
    }
    evidence(&format!("C17C_PREP level={level} vocation={vocation:?}"));
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    character_prep::declare_seeded(
        "C17c.client",
        "sorcerer-level-23",
        "ordinary leveling exceeds the client capture budget",
    );
    let dedicated =
        character_prep::dedicated_character(clients, "NODE_BOOT_C17C_CLIENT_ACCOUNT").await?;
    let scope = dedicated.clients(clients);
    let mut client = connect_character(&scope, 0x31, 0).await?;
    let seed_session = client.join_snapshot().game_session_id;
    request_fixture_control(
        uuid_text(seed_session.as_bytes()),
        "--profile",
        "f5-c17c".into(),
    )
    .await?;
    logout(&mut client).await?;
    drop(client);

    let (cast, frames) = native_session::capture_server_frames(async {
        let mut client = connect_character(&scope, 0x31, 0).await?;
        if client.join_snapshot().game_session_id == seed_session {
            return Err("C17c seeded admission reused its original session".into());
        }
        let cast = cast_rat(client.session_mut()).await?;
        Ok::<_, Box<dyn std::error::Error>>((client, cast))
    })
    .await;
    let (mut client, cast) = cast?;
    let frames = frames.ok_or("C17c transcript exceeded the capture bound")?;
    let cast_session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(&scope, 0x31, 0).await?;
    if fresh.join_snapshot().game_session_id == cast_session {
        return Err("C17c cast admission reused its original session".into());
    }
    let fresh_own = *fresh
        .world_entities()
        .ok_or("C17c fresh entities absent")?
        .own_identity();
    let fresh_look = fresh
        .session_mut()
        .visible_appearances()
        .into_iter()
        .find(|entry| entry.entity.entity.identity == fresh_own)
        .and_then(|entry| entry.look);
    if fresh_look.as_ref() != Some(&RAT) {
        return Err(
            format!("C17c fresh admission outfit after committed cast: {fresh_look:?}").into(),
        );
    }
    if !fresh
        .actor_vitals()
        .is_some_and(|after| after.mana == cast.mana && after.soul == cast.soul)
    {
        return Err(format!(
            "C17c fresh admission did not retain mana/soul: {:?}",
            fresh.actor_vitals()
        )
        .into());
    }
    logout(&mut fresh).await?;
    drop(fresh);
    wait_character_released(&scope, 0).await?;
    replay_capture(
        &frames,
        crate::foundation::CharacterId::decode(&scope.characters[0])?,
        &cast,
    )
    .await?;
    Ok(format!(
        "look=21_0_0_0_0_0 sequence={} replay=pass",
        cast.result_sequence
    ))
}

async fn cast_rat<S: SessionStream>(client: &mut Session<S>) -> TestResult<CastEvidence> {
    prepared_stats(&mut client).await?;
    let own = *client
        .world_entities()
        .ok_or("C17c entities absent")?
        .own_identity();
    let snapshot = client
        .gated_snapshot(appearance::STATE_DOMAIN_APPEARANCE)
        .ok_or("C17c appearance snapshot absent")?
        .clone();
    let decoded = appearance::Snapshot::decode(&snapshot.payload)
        .map_err(|error| format!("appearance snapshot: {error:?}"))?;
    let baseline = decoded
        .bindings
        .iter()
        .find(|binding| binding.entity.identity == own)
        .ok_or("C17c own appearance binding absent")?
        .clone();
    if baseline.look == RAT {
        return Err("C17c baseline already has the rat outfit".into());
    }
    let before = *client.actor_vitals().ok_or("C17c pre-cast vitals absent")?;
    let expected_mana = before
        .mana
        .checked_sub(100)
        .ok_or("C17c seeded mana below cost")?;
    client.take_events();
    let cast = client
        .chat(&ChatIntent::Say {
            mode: ChatSpeechMode::Say,
            text: WORDS.into(),
        })
        .await?;
    if cast.status != CommandStatus::Accepted || cast.disposition != ChatDisposition::Ok {
        return Err(format!(
            "illusion cast refused status={:?} disposition={:?}",
            cast.status, cast.disposition
        )
        .into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mapping_sequence = loop {
        let seen = client
            .visible_appearances()
            .into_iter()
            .find(|entry| entry.entity.entity.identity == own)
            .and_then(|entry| entry.look);
        if let Some(look) = seen
            && look != baseline.look
        {
            if look != RAT {
                return Err(format!("C17c cast applied unexpected outfit: {look:?}").into());
            }
            if client
                .actor_vitals()
                .is_some_and(|after| after.mana == expected_mana && after.soul == before.soul)
            {
                let delta = client
                    .raw_appearance_deltas()
                    .iter()
                    .find(|delta| {
                        delta.server_sequence > cast.result_server_sequence
                            && delta.base_revision == snapshot.revision
                            && delta.delta_type == appearance::DELTA_TYPE_APPEARANCE_MAPPINGS
                            && appearance::Mappings::decode(&delta.payload).is_ok_and(|mapping| {
                                mapping.upserts.iter().any(|binding| {
                                    binding.entity == baseline.entity
                                        && binding.appearance_ref == baseline.appearance_ref
                                        && binding.look == RAT
                                })
                            })
                    })
                    .ok_or("C17c exact rat binding absent from the first live Mappings delta")?;
                break delta.server_sequence;
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(
                "C17c own visible look or exact mana debit did not arrive after the cast".into(),
            );
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    };
    Ok(CastEvidence {
        session: *client.join_snapshot().game_session_id.as_bytes(),
        result_sequence: cast.result_server_sequence,
        mapping_sequence,
        last_sequence: client.last_server_sequence(),
        mana: expected_mana,
        soul: before.soul,
    })
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    match verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=C17c.client status=PASS seconds={:.3} evidence={}",
            started.elapsed().as_secs_f64(),
            observed.replace(' ', "_")
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=C17c.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=C17c.client status=FAIL evidence=timeout"),
    }
}
