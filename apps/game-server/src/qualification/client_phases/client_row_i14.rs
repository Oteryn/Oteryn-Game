//! I14 client replay capture. The native session consumer is
//! `crates/session/src/domain_dispatch.rs::Session::apply_state_delta`; it decodes a Character
//! Inventory payload with `decode_character_inventory_with_equip_drop`, then replaces the store
//! returned by `crates/session/src/session_access.rs::Session::inventory`.
use super::*;
use oteryn_dev_client::SessionEvent;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let (status, detail) = match verdict::observe(capture(clients)).await {
        Ok(()) => (
            "PASS",
            "fresh-raw-wire-replay-inventory-2-to-1-and-relog".to_owned(),
        ),
        Err(error) => ("FAIL", error.to_string()),
    };
    evidence(&format!(
        "e2e_all scenario=I14.client status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

async fn capture(clients: &SeamClients<'_>) -> TestResult {
    let fixture = character_prep::dedicated_character(clients, "NODE_BOOT_I14_ACCOUNT").await?;
    let scope = fixture.clients(clients);
    let mut client = connect_character(&scope, 0xb4, 0).await?;
    let seed_session = client.join_snapshot().game_session_id;
    character_prep::declare_seeded("I14.client", "meat (3577) x2", "food-use input");
    character_prep::grant_item(&client, 3577, 2).await?;
    logout(&mut client).await?;
    drop(client);
    let index = provisioning::item_index()?;
    let meat = provisioning::item_reference(&index, 3577)?;
    let (live, wire) = oteryn_dev_client::native_session::capture_server_frames(async {
        let mut client = connect_character(&scope, 0xb4, 0).await?;
        if client.join_snapshot().game_session_id == seed_session {
            return Err("I14 seed relog reused the session".into());
        }
        let proof = consume(client.session_mut(), meat).await?;
        Ok::<_, Box<dyn std::error::Error>>((client, proof))
    })
    .await;
    let (mut client, live) = live?;
    let use_session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(&scope, 0xb4, 0).await?;
    if fresh.join_snapshot().game_session_id == use_session
        || fresh.inventory().map(|inventory| visible(inventory, meat)) != Some(1)
    {
        return Err("I14 meat debit did not survive fresh admission".into());
    }
    logout(&mut fresh).await?;
    let capture_dir = std::env::var_os("E2E_ALL_CAPTURE_DIR")
        .map_or_else(std::env::temp_dir, std::path::PathBuf::from);
    let path = capture_dir.join(format!(
        "i14-food-server-{}.wire",
        uuid_text(use_session.as_bytes())
    ));
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let wire = wire?;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)?
        .write_all(&wire)?;
    let captured = std::fs::read(path)?;
    if captured != wire {
        return Err("I14 capture differs from this run's inbound wire bytes".into());
    }
    replay(
        captured,
        crate::foundation::CharacterId::decode(&scope.characters[0])?,
        meat,
        use_session.as_bytes(),
        live,
    )
    .await?;
    Ok(())
}

type InventoryProof = (u64, u64, u64);

fn visible(
    inventory: &oteryn_protocol_oteryn::item_view::CharacterInventory,
    meat: std::num::NonZeroU32,
) -> u64 {
    inventory
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref == meat)
        .map(|entry| u64::from(entry.count.get()))
        .sum()
}

async fn consume<S: oteryn_dev_client::native_session::SessionStream>(
    session: &mut oteryn_dev_client::native_session::Session<S>,
    meat: std::num::NonZeroU32,
) -> TestResult<InventoryProof> {
    let before = session.inventory().ok_or("I14 client inventory absent")?;
    if visible(before, meat) != 2 {
        return Err("I14 expected exactly two meat before use".into());
    }
    let handle = before
        .entries
        .iter()
        .find(|entry| entry.item_definition_ref == meat)
        .map(|entry| entry.handle)
        .ok_or("I14 server-issued meat handle absent")?;
    session.take_events();
    let reply = session.use_item(handle).await?;
    if reply.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
        return Err(format!("I14 food use reply={:?}", reply.disposition).into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let remaining = deadline
            .checked_duration_since(tokio::time::Instant::now())
            .ok_or("I14 committed food use sent no inventory delta")?;
        for event in session.wait_for_events(remaining).await? {
            if let SessionEvent::Inventory(delta) = event {
                let after = session
                    .inventory()
                    .ok_or("I14 inventory absent after use")?;
                if visible(after, meat) != 1 || after != &delta.value {
                    return Err("I14 food use must remove exactly one meat".into());
                }
                return Ok((
                    delta.server_sequence,
                    delta.base_revision,
                    delta.new_revision,
                ));
            }
        }
    }
}

async fn replay(
    wire: Vec<u8>,
    character_id: crate::foundation::CharacterId,
    meat: std::num::NonZeroU32,
    session_id: &[u8],
    expected: InventoryProof,
) -> TestResult {
    use oteryn_dev_client::native_session::{Admission, Session};
    use tokio::io::AsyncWriteExt;
    let (stream, peer) = tokio::io::duplex(64 * 1024);
    let (mut outbound, mut inbound) = tokio::io::split(peer);
    let writer = tokio::spawn(async move {
        let mut sink = tokio::io::sink();
        tokio::try_join!(
            inbound.write_all(&wire),
            tokio::io::copy(&mut outbound, &mut sink)
        )
    });
    let result = async {
        let mut client = Session::admit(
            stream,
            Admission {
                appearance_items: None,
                schema_revision: 1,
                character_id,
                admission_material: b"offline-replay",
                client_build_id: "oteryn-dev-client/world-qualification",
                supported_capabilities: oteryn_dev_client::CLIENT_SUPPORTED_CAPABILITIES,
                deadline: Duration::from_secs(5),
            },
        )
        .await?;
        if client.join_snapshot().game_session_id.as_bytes() != session_id {
            return Err("I14 replay did not admit the captured live session".into());
        }
        let actual = consume(&mut client, meat).await?;
        if actual != expected {
            return Err(format!(
                "I14 replay sequence/revision mismatch: live={expected:?} replay={actual:?}"
            )
            .into());
        }
        Ok(())
    }
    .await;
    if result.is_err() {
        writer.abort();
        return result;
    }
    writer.await??;
    result
}

async fn logout(client: &mut oteryn_dev_client::DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("I14 logout refused: {reply:?}").into());
    }
    Ok(())
}
