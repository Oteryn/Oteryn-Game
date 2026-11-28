//! C09 client replay capture: create real HP/mana deficits, purchase and consume real food, then
//! capture the ACTOR_VITALS payload applied by `crates/session/src/domain_dispatch.rs` through
//! `Session::apply_domain_delta` and exposed by `Session::actor_vitals` in `session_access.rs`.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::actor_spell::ActorVitals;
use oteryn_protocol_oteryn::world_object::UseDisposition;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result =
        verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients)))
            .await;
    match result {
        Ok(Ok(())) => evidence(&format!(
            "e2e_all scenario=C09.client status=FAIL evidence={}",
            super::full_row_c02::PLATFORM_ADMISSION_GAP
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=C09.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=C09.client status=FAIL evidence=journey-timeout"),
    }
}

fn quantity(client: &DevClientSession, reference: std::num::NonZeroU32) -> u64 {
    client.inventory().map_or(0, |inventory| {
        inventory
            .entries
            .iter()
            .filter(|item| item.item_definition_ref == reference)
            .map(|item| u64::from(item.count.get()))
            .sum()
    })
}

async fn acquire_meat(
    client: &mut DevClientSession,
    world: &BundleWorld,
) -> TestResult<std::num::NonZeroU32> {
    let index = provisioning::item_index()?;
    let reference = provisioning::item_reference(&index, 3577)?;
    if quantity(client, reference) == 0 {
        super::full_row_c09::buy_meat(client, world, reference, 1).await?;
    }
    Ok(reference)
}

async fn replay_capture(
    clients: &SeamClients<'_>,
    capture: &oteryn_dev_client::InboundCapture,
    session_id: &oteryn_protocol_oteryn::GameSessionId,
    sequences: (u64, u64, u64),
    expected: (ActorVitals, ActorVitals),
) -> TestResult {
    use oteryn_dev_client::native_session::{Admission, Session};
    use std::io::Write as _;
    let root = if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        std::path::PathBuf::from(root)
    } else {
        let log = std::path::PathBuf::from(required("NODE_BOOT_CAST_EVIDENCE_LOG")?);
        log.parent()
            .ok_or("node log directory missing")?
            .to_path_buf()
    };
    let path = root.join("c09-inbound.frames");
    let bytes = capture.bytes().ok_or("C09 inbound transcript overflow")?;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?
        .write_all(&bytes)?;
    let transport = tokio::io::join(
        std::io::Cursor::new(std::fs::read(path)?),
        tokio::io::sink(),
    );
    let mut replay = Session::admit(
        transport,
        Admission {
            appearance_items: None,
            schema_revision: 1,
            character_id: oteryn_protocol_oteryn::CharacterId::decode(&clients.characters[1])?,
            admission_material: b"offline-c09-replay",
            client_build_id: "oteryn-dev-client/c09-replay",
            supported_capabilities: oteryn_dev_client::CLIENT_SUPPORTED_CAPABILITIES,
            deadline: Duration::from_secs(2),
        },
    )
    .await?;
    if &replay.join_snapshot().game_session_id != session_id
        || replay
            .replay_food_vitals(sequences.0, sequences.1, sequences.2)
            .await?
            != expected
    {
        return Err("C09 raw replay disagrees with the live session or vitals".into());
    }
    evidence(&format!(
        "C09_REPLAY verified before={} use={} regenerated={}",
        sequences.0, sequences.1, sequences.2
    ));
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let limit = std::num::NonZeroUsize::MIN.saturating_add(32 * 1024 * 1024 - 1);
    let (mut client, capture) =
        connect_character_with_capture(clients, 0xc9, 1, Some(limit)).await?;
    let capture = capture.ok_or("C09 capture disabled")?;
    let meat = acquire_meat(&mut client, world).await?;
    travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
    let full = *client.actor_vitals().ok_or("C09 initial vitals absent")?;
    super::full_row_c09::spend_mana(&mut client).await?;
    for _ in 0..3 {
        client_hunt::hunt(&mut client).await.map_err(|error| {
            verdict::record_error(verdict::IndexedError::as_error(&error));
            verdict::context(format!("C09 combat: {error}"), error)
        })?;
        if client
            .actor_vitals()
            .is_some_and(|v| v.health < full.health)
        {
            break;
        }
    }
    let before = *client.actor_vitals().ok_or("deficit vitals absent")?;
    if before.health >= full.health || before.mana >= full.mana {
        return Err(format!("HP/mana deficits missing full={full:?} before={before:?}").into());
    }
    let handle = client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|item| item.item_definition_ref == meat)
        })
        .map(|item| item.handle)
        .ok_or("meat handle absent")?;
    let baseline = client.last_server_sequence();
    let use_reply = client.use_item(handle).await?;
    if use_reply.status != CommandStatus::Accepted
        || use_reply.disposition != UseDisposition::Committed
    {
        return Err("food use was not committed".into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        for event in client.take_events() {
            if let SessionEvent::ActorVitals(delta) = event
                && delta.server_sequence > use_reply.result_server_sequence
                && delta.value.health > before.health
                && delta.value.mana > before.mana
            {
                replay_capture(
                    clients,
                    &capture,
                    &client.join_snapshot().game_session_id,
                    (
                        baseline,
                        use_reply.result_server_sequence,
                        delta.server_sequence,
                    ),
                    (before, delta.value),
                )
                .await?;
                return Ok(());
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "food regeneration timeout before={before:?} current={:?}",
                client.actor_vitals()
            )
            .into());
        }
    }
}

#[test]
fn c09_missing_capture_directory_is_an_error() {
    let directory = Path::new(file!()).join("cannot-create-child-under-source");
    assert!(write_packet(&directory, "capture.bin", b"frame").is_err());
}
fn write_packet(root: &Path, name: &str, bytes: &[u8]) -> TestResult {
    std::fs::write(root.join(name), bytes)
        .map_err(|error| format!("capture-write-error {name}: {error}").into())
}
