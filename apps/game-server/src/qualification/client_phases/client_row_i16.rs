//! I16 client replay capture: inspect a real, carried instance with imbuement
//! slots and physical weight, then preserve the exact command-result payload.
use super::*;
use oteryn_protocol_oteryn::inspection as inspect;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(30), verdict::observe(capture(clients))).await;
    let (status, evidence_detail) = match result {
        Ok(Ok(item)) => ("PASS", format!("definition={};name={}", item.definition_ref, item.name)),
        Ok(Err(error)) => ("FAIL", error.to_string().replace([' ', '\n', '\r'], "_")),
        Err(_) => ("FAIL", "capture-timeout".into()),
    };
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=I16.client status={status} seconds={:.3} evidence={evidence_detail}",
        started.elapsed().as_secs_f64()
    ));
}

async fn capture(clients: &SeamClients<'_>) -> TestResult<inspect::InspectedItem> {
    let mut client = connect_character(clients, 0xd6, 1).await?;
    let inventory = client.inventory().ok_or("I16 client inventory absent")?;
    let handles = inventory
        .entries
        .iter()
        .chain(inventory.main_backpack.iter())
        .chain(inventory.equipment.iter().map(|entry| &entry.item))
        .map(|entry| entry.handle)
        .collect::<Vec<_>>();
    for handle in handles {
        let request = inspect::encode_request(inspect::Request::Item(handle))
            .map_err(|error| format!("I16 inspection request encode: {error:?}"))?;
        let response = client
            .session_mut()
            .gated_command(
                inspect::CAPABILITY_INSPECTION_V1,
                inspect::COMMAND_TYPE_INSPECTION,
                &request,
            )
            .await?;
        if response.status != CommandStatus::Accepted {
            continue;
        }
        let reply = inspect::decode_reply(&response.payload)
            .map_err(|error| format!("I16 inspection reply decode: {error:?}"))?;
        let Some(item) = reply.items.first() else {
            continue;
        };
        let has_imbuement = item
            .attributes
            .iter()
            .any(|(label, _)| label.starts_with("Imbuement Slot "));
        let has_weight = item
            .attributes
            .iter()
            .any(|(label, value)| (label == "Weight" || label == "Total Weight") && value != "0.00 oz");
        if has_imbuement && has_weight {
            capture_files(&response.payload, item);
            return Ok(item.clone());
        }
    }
    Err("I16 no carried instance exposed both imbuement slots and nonzero weight".into())
}

fn capture_files(payload: &[u8], item: &inspect::InspectedItem) {
    let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    let directory = Path::new(&directory);
    let expected = visible_item(item);
    for (name, result) in [
        ("i16-inspection.bin", std::fs::write(directory.join("i16-inspection.bin"), payload)),
        ("i16-expected.txt", std::fs::write(directory.join("i16-expected.txt"), expected)),
    ] {
        if let Err(error) = result {
            evidence(&format!("I16_CAPTURE_WRITE_ERROR file={name} error={error}"));
        }
    }
}

fn visible_item(item: &inspect::InspectedItem) -> String {
    let mut text = format!(
        "Inspection\r\nInspection received\r\nSlot {}: {} ×{} (tier {})",
        item.slot, item.name, item.count, item.tier
    );
    for (key, value) in &item.attributes {
        text.push_str(&format!("\r\n  {key}: {value}"));
    }
    text.push_str("\r\nPermission requests: 0");
    text
}
