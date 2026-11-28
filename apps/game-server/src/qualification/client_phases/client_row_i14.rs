//! I14 client replay capture. The native session consumer is
//! `crates/session/src/domain_dispatch.rs::Session::apply_state_delta`; it decodes a Character
//! Inventory payload with `decode_character_inventory_with_equip_drop`, then replaces the store
//! returned by `crates/session/src/session_access.rs::Session::inventory`.
use super::*;
use oteryn_dev_client::SessionEvent;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if let Err(error) = verdict::observe(capture(clients)).await {
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=I14.client status=FAIL evidence={error}"
        ));
    }
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
    let mut client = connect_character(&scope, 0xb4, 0).await?;
    let use_session = client.join_snapshot().game_session_id;
    if use_session == seed_session {
        return Err("I14 seed relog reused the session".into());
    }
    let index = provisioning::item_index()?;
    let meat = provisioning::item_reference(&index, 3577)?;
    let before = client
        .inventory()
        .ok_or("I14 client inventory absent")?
        .clone();
    let handle = before
        .entries
        .iter()
        .find(|entry| entry.item_definition_ref == meat)
        .map(|entry| entry.handle)
        .ok_or("I14 full journey left no server-issued meat")?;
    client.take_events();
    let reply = client.use_item(handle).await?;
    if reply.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
        return Err(format!("I14 replay action reply={:?}", reply.disposition).into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let delta = loop {
        let remaining = deadline
            .checked_duration_since(tokio::time::Instant::now())
            .ok_or("I14 committed food use sent no inventory delta")?;
        if let Some(delta) = client
            .wait_for_events(remaining)
            .await?
            .into_iter()
            .find_map(|event| match event {
                SessionEvent::Inventory(delta) => Some(delta),
                _ => None,
            })
        {
            break delta;
        }
    };
    let after = client.inventory().ok_or("I14 inventory absent after use")?;
    let visible = |inventory: &oteryn_protocol_oteryn::item_view::CharacterInventory| {
        inventory
            .entries
            .iter()
            .filter(|entry| entry.item_definition_ref == meat)
            .map(|entry| u64::from(entry.count.get()))
            .sum::<u64>()
    };
    if visible(&before) != 2 || visible(after) != 1 || after != &delta.value {
        return Err("I14 food use must remove exactly one meat in the captured inventory".into());
    }
    let expected = format!(
        "{} {} {} {} {}\n",
        delta.server_sequence,
        delta.base_revision,
        delta.new_revision,
        visible(after),
        meat.get()
    );
    {
        let path = std::env::var_os("E2E_ALL_CAPTURE_DIR")
            .ok_or("I14 requires a fresh capture directory")?;
        let path = Path::new(&path);
        let writes = [
            (
                "i14-inventory-before.bin",
                oteryn_protocol_oteryn::item_view::encode_character_inventory_with_equip_drop(
                    &before, true,
                )
                .map_err(|error| format!("I14 before codec: {error:?}"))?,
            ),
            (
                "i14-inventory-after.bin",
                oteryn_protocol_oteryn::item_view::encode_character_inventory_with_equip_drop(
                    &delta.value,
                    true,
                )
                .map_err(|error| format!("I14 after codec: {error:?}"))?,
            ),
            ("i14-expected.txt", expected.into_bytes()),
        ];
        for (name, bytes) in writes {
            use std::io::Write;
            std::fs::File::create_new(path.join(name))?.write_all(&bytes)?;
        }
    }
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(&scope, 0xb4, 0).await?;
    if fresh.join_snapshot().game_session_id == use_session
        || fresh.inventory().map(visible) != Some(1)
    {
        return Err("I14 meat debit did not survive fresh admission".into());
    }
    logout(&mut fresh).await?;
    evidence("e2e_all scenario=I14.client status=CAPTURED evidence=inventory-2-to-1-and-relog");
    Ok(())
}

async fn logout(client: &mut oteryn_dev_client::DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("I14 logout refused: {reply:?}").into());
    }
    Ok(())
}
