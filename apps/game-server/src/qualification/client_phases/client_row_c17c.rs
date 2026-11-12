//! C17c client replay capture: casting Creature Illusion (`utevo res ina "rat"`) as a player
//! changes the caster's own appearance binding. Client owner: `AppearanceState` in
//! `crates/session` (`Session::visible_appearances`). The raw appearance join snapshot is the
//! server payload; the Mappings delta is session-owned and re-encoded with the production encoder.
use super::*;
use oteryn_dev_client::{ChatDisposition, ChatIntent, ChatSpeechMode, DevClientSession};
use oteryn_protocol_oteryn::appearance::{self, Look};

const WORDS: &str = "utevo res ina \"rat\"";

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("C17C_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

fn outfit(look: &Look) -> TestResult<String> {
    match look {
        Look::Outfit {
            look_type,
            colours,
            addons,
        } => Ok(format!(
            "{look_type} {} {} {} {} {addons}",
            colours[0], colours[1], colours[2], colours[3]
        )),
        _ => Err("C17c look is not an outfit".into()),
    }
}

fn hex(bytes: &[u8; 16]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client: DevClientSession = connect_world(clients, 0x31).await?;
    let own = *client
        .world_entities()
        .ok_or("C17c entities absent")?
        .own_identity();
    let snapshot = client
        .session_mut()
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
    client.take_events();
    let cast = client
        .chat(&ChatIntent::Say {
            mode: ChatSpeechMode::Say,
            text: WORDS.into(),
        })
        .await?;
    if cast.status != CommandStatus::Accepted || cast.disposition != ChatDisposition::Ok {
        return Err(format!(
            "illusion cast refused status={:?} disposition={:?}; the ordinary starter lacks the level and vocation (full_e2ef5 C17c)",
            cast.status, cast.disposition
        )
        .into());
    }
    // The applied look is read from the session's own visible appearance; the Mappings delta that
    // carries it is session-owned, so it is re-encoded with the production encoder.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let look = loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        let seen = client
            .session_mut()
            .visible_appearances()
            .into_iter()
            .find(|entry| entry.entity.entity.identity == own)
            .and_then(|entry| entry.look);
        if let Some(look) = seen
            && look != baseline.look
        {
            break look;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("own visible look did not change after the cast".into());
        }
    };
    let mut binding = baseline.clone();
    binding.root_revision = binding.root_revision.saturating_add(1);
    binding.look = look;
    let mapping_payload = appearance::Mappings {
        context: decoded.catalogue.context,
        upserts: vec![binding.clone()],
        forgets: Vec::new(),
    }
    .encode()
    .map_err(|error| format!("mapping encode: {error:?}"))?;
    let sequence = cast.result_server_sequence;
    let expected = format!(
        "revision={}\nidentity={}\ngeneration={}\nref={}\nbaseline={}\nlook={}\n",
        snapshot.revision,
        hex(&own),
        baseline.entity.generation,
        baseline.appearance_ref,
        outfit(&baseline.look)?,
        outfit(&binding.look)?
    );
    capture("c17c-snapshot.bin", &snapshot.payload);
    capture("c17c-mapping.bin", &mapping_payload);
    capture("c17c-expected.txt", expected.as_bytes());
    let _ = client.session_mut().logout().await;
    Ok(format!("look={} sequence={sequence}", outfit(&binding.look)?))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=C17c.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=C17c.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=C17c.client status=FAIL evidence=timeout"),
    }
}
