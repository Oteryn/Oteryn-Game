//! I12 client replay capture: use the real pick on a legitimately carried shiny stone and retain
//! the command result plus the inventory snapshot/delta bytes consumed by `oteryn-session`.
use super::*;
use oteryn_dev_client::native_session::{Admission, Session};
use oteryn_dev_client::{SessionError, SessionEvent};
use oteryn_protocol_oteryn::capability_tables::CAPABILITY_WORLD_SPATIAL_FIELDS;
use oteryn_protocol_oteryn::item_view;
use oteryn_protocol_oteryn::logout::CAPABILITY_LOGOUT_V1;
use oteryn_protocol_oteryn::world_interaction::CAPABILITY_WORLD_INTERACTION_V1;
use oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1;
use oteryn_protocol_oteryn::world_object::{
    CAPABILITY_ITEM_USE_V1, MapUseTarget, UseDisposition, UseIntent, UseTarget,
    encode_item_use_intent,
};
use oteryn_protocol_oteryn::world_spatial_entities::CAPABILITY_WORLD_SPATIAL_ENTITIES;

const PICK: u32 = 3456;
const SHINY_STONE: u32 = 10310;

// The channel requires map interaction, map view, logout and spatial fields. Item use
// is required to refine the stone. None of these extend the cap-4 inventory payload.
const CAPTURE_CAPABILITIES: &[u32] = &[
    item_view::CAPABILITY_ITEM_VIEW_MOVE_V1,
    CAPABILITY_WORLD_SPATIAL_ENTITIES,
    CAPABILITY_WORLD_INTERACTION_V1,
    CAPABILITY_ITEM_USE_V1,
    CAPABILITY_WORLD_MAP_VIEW_V1,
    CAPABILITY_LOGOUT_V1,
    CAPABILITY_WORLD_SPATIAL_FIELDS,
];

pub(super) async fn connect_for_capture<S: oteryn_dev_client::native_session::SessionStream>(
    clients: &SeamClients<'_>,
    tag: u8,
    index: usize,
    supported_capabilities: &[u32],
    wrap: impl FnOnce(tokio_rustls::client::TlsStream<TcpStream>) -> S,
) -> TestResult<Session<S>> {
    wait_character_released(clients, index).await?;
    let generation = platform_generation(clients.descriptor, &clients.accounts[index]).await?;
    let (nonce, attempt) = fresh_grant_material(tag)?;
    let grant = Grant {
        signing: clients.signing,
        key_id: clients.key_id,
        account_id: &clients.accounts[index],
        character_id: clients.characters[index],
        world_id: clients.world,
        channel_id: clients.channel,
        security_generation: generation,
        scope_generation: clients.scope_generation,
        nonce,
        attempt,
    };
    let token = sign_grant(&grant, now_seconds()?);
    let deadline = Duration::from_secs(20);
    let connector = connector(clients.certificate, &EXACT)?;
    let server_name = ServerName::try_from("localhost")?;
    let stream = tokio::time::timeout(deadline, async {
        let tcp = TcpStream::connect(clients.address).await?;
        connector.connect(server_name, tcp).await
    })
    .await??;
    if stream.get_ref().1.alpn_protocol() != EXACT.alpn {
        return Err("capture admission ALPN mismatch".into());
    }
    let session = Session::admit(
        wrap(stream),
        Admission {
            appearance_items: None,
            schema_revision: 1,
            character_id: CharacterId::decode(&clients.characters[index])?,
            admission_material: token.as_bytes(),
            client_build_id: "oteryn-dev-client/replay-qualification",
            supported_capabilities,
            deadline,
        },
    )
    .await?;
    verdict::Harness::global().progress("admission");
    Ok(session)
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(300), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", detail),
        Ok(Err(error)) => ("FAIL", error.to_string().replace([' ', '\n', '\r'], "_")),
        Err(_) => ("FAIL", "timeout".to_owned()),
    };
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=I12.client status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let index = provisioning::item_index()?;
    let pick_ref = provisioning::item_reference(&index, PICK)?;
    let stone_ref = provisioning::item_reference(&index, SHINY_STONE)?;
    prepare_inputs(clients).await?;
    match refine(clients, pick_ref, stone_ref).await {
        Err(error) if error.downcast_ref::<SessionError>().is_some_and(|error| {
            matches!(error, SessionError::Io(error) if error.kind() == io::ErrorKind::UnexpectedEof)
        }) => {
            // A lost result can leave a committed refinement. The new admission
            // must still contain the single original stone before retrying.
            evidence("I12 retrying after lost result; checking fresh inventory");
            refine(clients, pick_ref, stone_ref).await
        }
        result => result,
    }
}

async fn refine(
    clients: &SeamClients<'_>,
    pick_ref: std::num::NonZeroU32,
    stone_ref: std::num::NonZeroU32,
) -> TestResult<String> {
    let mut client = connect_for_capture(
        clients,
        0xc2,
        0,
        CAPTURE_CAPABILITIES,
        std::convert::identity,
    )
    .await?;
    let inventory = client.inventory().ok_or("no inventory snapshot")?;
    let pick = inventory
        .entries
        .iter()
        .find(|entry| entry.item_definition_ref == pick_ref)
        .ok_or("no legitimately carried pick")?
        .handle;
    let stone = inventory
        .entries
        .iter()
        .find(|entry| entry.item_definition_ref == stone_ref)
        .ok_or("no legitimately carried shiny stone")?
        .handle;
    let snapshot = client
        .admission_snapshot(item_view::STATE_DOMAIN_CHARACTER_INVENTORY)
        .ok_or("no inventory admission snapshot")?
        .clone();
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(pick),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Item(stone)),
        use_with_position: None,
    })
    .map_err(|error| format!("pick intent: {error:?}"))?;
    client
        .service_liveness_discarding_events(Duration::from_secs(1))
        .await?;
    let reply = client.world_context_use(&payload).await?;
    let disposition = oteryn_protocol_oteryn::world_object::decode_use_result(&reply.payload)
        .map_err(|error| format!("pick reply: {error:?}"))?;
    if reply.status != CommandStatus::Accepted || disposition != UseDisposition::Committed {
        return Err(format!("pick refine returned {:?}/{disposition:?}", reply.status).into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let delta = loop {
        if let Some(delta) = client
            .take_events()
            .into_iter()
            .find_map(|event| match event {
                SessionEvent::Inventory(delta) if delta.base_revision == snapshot.revision => {
                    Some(delta)
                }
                _ => None,
            })
        {
            break delta;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("committed pick refine sent no inventory delta from admission".into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    };
    let delta_payload = item_view::encode_character_inventory_with_appearances(
        &delta.value,
        client.is_selected(item_view::CAPABILITY_ITEM_EQUIP_DROP_V1),
        client.is_selected(item_view::CAPABILITY_ITEM_APPEARANCE_V1),
    )
    .map_err(|error| format!("inventory capture: {error:?}"))?;
    capture(
        &snapshot.payload,
        &delta_payload,
        &reply.payload,
        snapshot.revision,
        &delta,
    )?;
    Ok("committed-inventory-delta-captured".into())
}

async fn prepare_inputs(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_character(clients, 0xc1, 0).await?;
    let socket = required("NODE_BOOT_CONTROL_SOCKET")?;
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/qualification/node_boot/provision.py");
    let status = tokio::task::spawn_blocking(move || {
        provision_python()
            .arg(script)
            .args([
                "--socket",
                &socket,
                "--session",
                &session,
                "--profile",
                "i12",
            ])
            .status()
    })
    .await??;
    if !status.success() {
        return Err("I12 audited fixture grant failed".into());
    }
    full_row_p05::logout_after_combat(&mut client).await
}

fn capture(
    snapshot: &[u8],
    delta: &[u8],
    reply: &[u8],
    snapshot_revision: u64,
    applied: &oteryn_dev_client::AppliedDelta<oteryn_dev_client::CharacterInventory>,
) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    let root = Path::new(&root);
    let expected = applied
        .value
        .entries
        .iter()
        .map(|entry| format!("{} {}\n", entry.item_definition_ref, entry.count))
        .collect::<String>();
    for (name, bytes) in [
        ("i12-snapshot.bin", snapshot),
        ("i12-inventory.bin", delta),
        ("i12-result.bin", reply),
    ] {
        std::fs::write(root.join(name), bytes)?;
    }
    for (name, value) in [
        ("i12-expected.txt", expected),
        (
            "i12-revisions.txt",
            format!(
                "{} {} {} {}",
                snapshot_revision,
                applied.base_revision,
                applied.new_revision,
                applied.server_sequence
            ),
        ),
    ] {
        std::fs::write(root.join(name), value)?;
    }
    Ok(())
}
