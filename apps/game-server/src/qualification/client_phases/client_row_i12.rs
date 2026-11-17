//! I12 client replay capture: use the real pick on a legitimately carried shiny stone and retain
//! the command result plus the inventory snapshot/delta bytes consumed by `oteryn-session`.
use super::*;
use oteryn_dev_client::SessionEvent;
use oteryn_protocol_oteryn::item_view;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};

const PICK: u32 = 3456;
const SHINY_STONE: u32 = 10310;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(30), verdict::observe(journey(clients))).await;
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

fn refinement_matches(before: [u64; 6], after: [u64; 6], command: u64, delta: u64) -> bool {
    if delta <= command || before[0] != after[0] || before[1].checked_sub(1) != Some(after[1]) {
        return false;
    }
    let mut rewards = 0;
    for (&old, &new) in before[2..].iter().zip(&after[2..]) {
        if old != new {
            if old.checked_add(1) != Some(new) {
                return false;
            }
            rewards += 1;
        }
    }
    rewards == 1
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_character(clients, 0xc2, 0).await?;
    let index = provisioning::item_index()?;
    let pick_ref = provisioning::item_reference(&index, PICK)?;
    let stone_ref = provisioning::item_reference(&index, SHINY_STONE)?;
    let references = [
        pick_ref, stone_ref,
        provisioning::item_reference(&index, 3043)?,
        provisioning::item_reference(&index, 3031)?,
        provisioning::item_reference(&index, 3035)?,
        provisioning::item_reference(&index, 3028)?,
    ];
    let counts = |inventory: &item_view::CharacterInventory| references.map(|reference| {
        inventory.entries.iter().filter(|entry| entry.item_definition_ref == reference)
            .map(|entry| u64::from(entry.count.get())).sum::<u64>()
    });
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
    let before = counts(inventory);
    let snapshot = client
        .session_mut()
        .gated_snapshot(item_view::STATE_DOMAIN_CHARACTER_INVENTORY)
        .ok_or("no inventory gated snapshot")?
        .clone();
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(pick),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Item(stone)),
        use_with_position: None,
    })
    .map_err(|error| format!("pick intent: {error:?}"))?;
    client.take_events();
    let reply = client
        .session_mut()
        .world_context_use(&payload)
        .await?;
    let disposition = oteryn_protocol_oteryn::world_object::decode_use_result(&reply.payload)
        .map_err(|error| format!("pick reply: {error:?}"))?;
    if reply.status != CommandStatus::Accepted || disposition != UseDisposition::Committed {
        return Err(format!("pick refine returned {:?}/{disposition:?}", reply.status).into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let delta = loop {
        if let Some(delta) = client.take_events().into_iter().find_map(|event| match event {
            SessionEvent::Inventory(delta) if refinement_matches(
                before, counts(&delta.value), reply.server_sequence, delta.server_sequence,
            ) => Some(delta),
            _ => None,
        }) {
            break delta;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("I12 no inventory delta with one shiny stone debit and one refinement reward".into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    };
    let delta_payload = item_view::encode_character_inventory(&delta.value)
        .map_err(|error| format!("inventory capture: {error:?}"))?;
    capture(&snapshot.payload, &delta_payload, &reply.payload, snapshot.revision, &delta);
    Ok("committed-inventory-delta-captured".into())
}

fn capture(
    snapshot: &[u8],
    delta: &[u8],
    reply: &[u8],
    snapshot_revision: u64,
    applied: &oteryn_dev_client::AppliedDelta<oteryn_dev_client::CharacterInventory>,
) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
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
        if let Err(error) = std::fs::write(root.join(name), bytes) {
            evidence(&format!("I12_CAPTURE_WRITE_ERROR file={name} error={error}"));
        }
    }
    for (name, value) in [
        ("i12-expected.txt", expected),
        (
            "i12-revisions.txt",
            format!(
                "{} {} {} {}",
                snapshot_revision, applied.base_revision, applied.new_revision, applied.server_sequence
            ),
        ),
    ] {
        if let Err(error) = std::fs::write(root.join(name), value) {
            evidence(&format!("I12_CAPTURE_WRITE_ERROR file={name} error={error}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shiny_stone_capture_requires_fresh_exact_refinement() {
        let before = [1, 4, 0, 3, 0, 0];
        assert!(!refinement_matches(before, before, 2, 3));
        assert!(!refinement_matches(before, [1, 3, 1, 4, 0, 0], 2, 3));
        assert!(!refinement_matches(before, [1, 3, 1, 3, 0, 0], 3, 2));
        assert!(refinement_matches(before, [1, 3, 1, 3, 0, 0], 2, 3));
    }
}
