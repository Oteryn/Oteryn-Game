//! I14 client replay capture. The native session consumer is
//! `crates/session/src/domain_dispatch.rs::Session::apply_state_delta`; it decodes a Character
//! Inventory payload with `decode_character_inventory_with_equip_drop`, then replaces the store
//! returned by `crates/session/src/session_access.rs::Session::inventory`.
use super::*;
use oteryn_dev_client::SessionEvent;

fn consumption_matches(before: u64, after: u64, command: u64, delta: u64) -> bool {
    delta > command && before.checked_sub(1) == Some(after)
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if let Err(error) = verdict::observe(capture(clients)).await {
        evidence(&format!("SEAM_EVIDENCE e2e_all scenario=I14.client status=FAIL evidence={error}"));
    }
}

async fn capture(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0xb4).await?;
    let index = provisioning::item_index()?;
    let meat = provisioning::item_reference(&index, 3577)?;
    let before = client.inventory().ok_or("I14 client inventory absent")?.clone();
    let handle = before
        .entries
        .iter()
        .find(|entry| entry.item_definition_ref == meat)
        .map(|entry| entry.handle)
        .ok_or("I14 full journey left no server-issued meat")?;
    client.take_events();
    let reply = client.use_item(handle).await?;
    if reply.status != CommandStatus::Accepted || reply.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
        return Err(format!("I14 replay action reply={:?}", reply.disposition).into());
    }
    let visible = |inventory: &oteryn_protocol_oteryn::item_view::CharacterInventory| {
        inventory
            .entries
            .iter()
            .filter(|entry| entry.item_definition_ref == meat)
            .map(|entry| u64::from(entry.count.get()))
            .sum::<u64>()
    };
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let count_before = visible(&before);
    let delta = loop {
        if let Some(delta) = client.take_events().into_iter().find_map(|event| match event {
            SessionEvent::Inventory(delta) if consumption_matches(
                count_before, visible(&delta.value), reply.result_server_sequence, delta.server_sequence,
            ) => Some(delta),
            _ => None,
        }) {
            break delta;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("I14 committed food use sent no matching one-meat inventory debit".into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    };
    let after = &delta.value;
    let expected = format!(
        "{} {} {} {}\n",
        delta.server_sequence,
        delta.base_revision,
        delta.new_revision,
        visible(after)
    );
    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
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
            if let Err(error) = std::fs::write(path.join(name), bytes) {
                evidence(&format!("I14_CAPTURE_WRITE_ERROR file={name} error={error}"));
            }
        }
    }
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=I14.client status=PASS evidence=inventory-before-{}-after-{}",
        visible(&before),
        visible(after)
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn food_consumption_requires_fresh_one_unit_debit() {
        assert!(!consumption_matches(2, 2, 5, 6));
        assert!(!consumption_matches(2, 1, 5, 4));
        assert!(!consumption_matches(2, 0, 5, 6));
        assert!(consumption_matches(2, 1, 5, 6));
    }
}
