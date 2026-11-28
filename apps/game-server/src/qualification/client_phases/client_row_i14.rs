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
    let mut client = connect_world(clients, 0xb4).await?;
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
    let delta = client
        .take_events()
        .into_iter()
        .find_map(|event| match event {
            SessionEvent::Inventory(delta) => Some(delta),
            _ => None,
        })
        .ok_or("I14 committed food use sent no inventory delta")?;
    let after = client.inventory().ok_or("I14 inventory absent after use")?;
    let visible = |inventory: &oteryn_protocol_oteryn::item_view::CharacterInventory| {
        inventory
            .entries
            .iter()
            .filter(|entry| entry.item_definition_ref == meat)
            .map(|entry| u64::from(entry.count.get()))
            .sum::<u64>()
    };
    if visible(&before).checked_sub(1) != Some(visible(after)) || after != &delta.value {
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
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=I14.client status=CAPTURED evidence=inventory-before-{}-after-{}",
        visible(&before),
        visible(after)
    ));
    Ok(())
}
