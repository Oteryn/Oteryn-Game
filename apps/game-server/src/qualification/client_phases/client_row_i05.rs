//! I05 native-client replay capture.
//!
//! The real consumer is `crates/session/src/domain_dispatch.rs`,
//! `Session::apply_delta`'s `STATE_DOMAIN_CHARACTER_INVENTORY` arm. It decodes
//! the domain-9 payload, replaces `Session::inventory`, and exposes that state
//! through `crates/session/src/session_access.rs::Session::inventory` (and thus
//! `tools/dev-client/src/lib.rs::DevClientSession::inventory`).
use super::*;
use oteryn_dev_client::client_runtime::inventory::MoveRequest;
use oteryn_dev_client::native_session as native;
use oteryn_protocol_oteryn::item_view as items;

const ENERGY_RING: u32 = 3051;
const ACTIVE_ENERGY_RING: u32 = 3088;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result = verdict::observe(capture(clients)).await;
    evidence(&format!(
        "e2e_all scenario=I05.client status={}{}",
        if result.is_ok() { "PASS" } else { "FAIL" },
        result
            .as_ref()
            .err()
            .map_or(String::new(), |error| format!(" evidence={error}"))
    ));
}

async fn capture(clients: &SeamClients<'_>) -> TestResult {
    let identity = character_prep::dedicated_character(clients, "NODE_BOOT_Q_I05_ACCOUNT").await?;
    let clients = &identity.clients(clients);
    let index = provisioning::item_index()?;
    let base = provisioning::item_reference(&index, ENERGY_RING)?;
    let active = provisioning::item_reference(&index, ACTIVE_ENERGY_RING)?;

    character_prep::declare_seeded("I05.client", "one base energy ring", "equipment transform");
    let client = connect_character(clients, 0xb2, 0).await?;
    character_prep::grant_item(&client, ENERGY_RING, 1).await?;
    logout(client).await?;

    let mut client = connect_character(clients, 0xb3, 0).await?;
    let inventory = client.inventory().ok_or("i05-grant-inventory-missing")?;
    let mut rings = inventory
        .entries
        .iter()
        .filter(|item| item.item_definition_ref == base);
    let ring = rings.next().ok_or("i05-base-ring-not-durable")?;
    if ring.count.get() != 1 || rings.next().is_some() || equipped_ring(inventory).is_some() {
        return Err("i05-unexpected-starting-ring-custody".into());
    }
    let handle = ring.handle;
    let slot = items::ItemMoveDestination::Equipment(items::EquipmentSlot::Ring);
    let delta = move_ring(client.session_mut(), handle, slot).await?;
    verify_active_ring(&delta.value, active)?;
    logout(client).await?;

    let (live, frames) = native::test_support::capture_inbound(async {
        let mut client = connect_character(clients, 0xb4, 0).await?;
        let session = client.session_mut();
        let id = session.join_snapshot().game_session_id;
        let handle = unequip(session, base, active).await?;
        let split = native::test_support::captured_frame_count();
        let delta = move_ring(session, handle, slot).await?;
        verify_active_ring(&delta.value, active)?;
        Ok::<_, Box<dyn std::error::Error>>((client, id, delta.server_sequence, split))
    })
    .await;
    let (client, id, sequence, split) = live?;
    logout(client).await?;

    let client = connect_character(clients, 0xb5, 0).await?;
    verify_active_ring(
        client
            .inventory()
            .ok_or("i05-final-equipped-inventory-missing")?,
        active,
    )?;
    logout(client).await?;
    replay(
        clients.characters[0],
        id,
        sequence,
        (frames, split),
        base,
        active,
    )
    .await?;
    evidence("I05_CLIENT_CHECK PASS observed=base-to-active+active-to-base+active-after-relog");
    Ok(())
}

fn equipped_ring(inventory: &items::CharacterInventory) -> Option<&items::EquippedItem> {
    inventory
        .equipment
        .iter()
        .find(|entry| entry.slot == items::EquipmentSlot::Ring)
}

fn verify_active_ring(
    inventory: &items::CharacterInventory,
    active: std::num::NonZeroU32,
) -> TestResult<items::ItemHandle> {
    let ring = equipped_ring(inventory).ok_or("i05-active-ring-not-durable")?;
    if ring.item.item_definition_ref != active || ring.item.count.get() != 1 {
        return Err("i05-ring-slot-does-not-contain-one-active-ring".into());
    }
    Ok(ring.item.handle)
}

async fn move_ring<S: native::SessionStream>(
    client: &mut native::Session<S>,
    source: items::ItemHandle,
    destination: items::ItemMoveDestination,
) -> TestResult<oteryn_dev_client::AppliedDelta<items::CharacterInventory>> {
    client.take_events();
    let moved = client
        .move_item_counted(MoveRequest {
            source,
            destination,
            count: None,
        })
        .await?;
    if moved.outcome != items::ItemMoveOutcome::Moved {
        return Err(format!("i05-ring-move-refused:{moved:?}").into());
    }
    let result_sequence = client.last_server_sequence();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in client.take_events() {
            if let oteryn_dev_client::SessionEvent::Inventory(delta) = event {
                if delta.server_sequence <= result_sequence {
                    return Err("i05-inventory-delta-precedes-move-result".into());
                }
                return Ok(delta);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("i05-ring-move-inventory-delta-missing".into());
        }
        client.service_liveness(Duration::from_millis(20)).await?;
    }
}

async fn logout(mut client: oteryn_dev_client::DevClientSession) -> TestResult {
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("i05-client-capture-logout-refused:{:?}", logout.result).into());
    }
    Ok(())
}

async fn unequip<S: native::SessionStream>(
    session: &mut native::Session<S>,
    base: std::num::NonZeroU32,
    active: std::num::NonZeroU32,
) -> TestResult<items::ItemHandle> {
    let handle = verify_active_ring(
        session
            .inventory()
            .ok_or("i05-initial-equipped-inventory-missing")?,
        active,
    )?;
    let removed = move_ring(session, handle, items::ItemMoveDestination::MainBackpack).await?;
    if equipped_ring(&removed.value).is_some()
        || !removed.value.entries.iter().any(|item| {
            item.handle == handle && item.item_definition_ref == base && item.count.get() == 1
        })
    {
        return Err("i05-unequip-did-not-restore-base-ring".into());
    }
    Ok(handle)
}

async fn replay(
    character: [u8; 16],
    id: oteryn_protocol_oteryn::GameSessionId,
    sequence: u64,
    capture_frames: (Vec<Vec<u8>>, usize),
    base: std::num::NonZeroU32,
    active: std::num::NonZeroU32,
) -> TestResult {
    use std::io::Write as _;

    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR")
        .map_or_else(std::env::temp_dir, std::path::PathBuf::from);
    let path = root.join(format!("i05-{}.wire", uuid_text(id.as_bytes())));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut capture = options.open(&path)?;
    let (frames, split) = capture_frames;
    let mut boundary = 0;
    for (index, frame) in frames.into_iter().enumerate() {
        if index == split {
            boundary = usize::try_from(std::io::Seek::stream_position(&mut capture)?)?;
        }
        capture.write_all(&(u32::try_from(frame.len())?).to_be_bytes())?;
        capture.write_all(&frame)?;
    }
    if boundary == 0 {
        return Err("i05-capture-missing-move-boundary".into());
    }
    drop(capture);
    let bytes = std::fs::read(&path)?;
    let (stream, mut peer) = tokio::io::duplex(bytes.len().max(65_536));
    use tokio::io::AsyncWriteExt as _;
    peer.write_all(&bytes[..boundary]).await?;
    let mut session = native::Session::admit(
        stream,
        native::Admission {
            appearance_items: None,
            schema_revision: 1,
            character_id: oteryn_protocol_oteryn::CharacterId::decode(&character)?,
            admission_material: b"i05-local-replay",
            client_build_id: "oteryn-dev-client/world-qualification",
            supported_capabilities: native::CLIENT_SUPPORTED_CAPABILITIES,
            deadline: Duration::from_secs(5),
        },
    )
    .await?;
    if session.join_snapshot().game_session_id != id {
        return Err("i05-replay-session-mismatch".into());
    }
    let handle = unequip(&mut session, base, active).await?;
    peer.write_all(&bytes[boundary..]).await?;
    let delta = move_ring(
        &mut session,
        handle,
        items::ItemMoveDestination::Equipment(items::EquipmentSlot::Ring),
    )
    .await?;
    verify_active_ring(&delta.value, active)?;
    if delta.server_sequence != sequence {
        return Err("i05-replay-sequence-mismatch".into());
    }
    evidence(&format!(
        "I05_REPLAY session={} sequence={sequence} status=PASS",
        uuid_text(id.as_bytes())
    ));
    Ok(())
}
