//! I07 client replay capture: real quick-loot filter, Gold Pouch destination and loot-all replies.
use super::*;
use oteryn_protocol_oteryn::quick_loot::{
    QuickLootAction, QuickLootCategory, QuickLootFilterMode, QuickLootRecipient, QuickLootRequest,
    decode_quick_loot_result, encode_quick_loot_request,
};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let result =
        verdict::indexed_timeout(Duration::from_secs(30), verdict::observe(journey(clients))).await;
    let result = result.unwrap_or_else(|_| Err("I07 client capture timed out".into()));
    evidence(&format!(
        "e2e_all scenario=I07.client status={}",
        if result.is_ok() { "PASS" } else { "FAIL" }
    ));
    if let Err(error) = result {
        evidence(&format!("I07_CLIENT_GAP {error}"));
    }
}

fn request(action: QuickLootAction) -> QuickLootRequest {
    QuickLootRequest {
        action,
        corpse: None,
        category: None,
        recipient: None,
        recipient_id: None,
        fallback_to_main: false,
        filter_mode: None,
        filter_definition_ref: None,
    }
}

fn write_capture(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write as _;
    std::fs::File::create_new(path)?.write_all(bytes)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let capture_dir = std::env::var_os("E2E_ALL_CAPTURE_DIR")
        .ok_or("I07 requires a fresh E2E_ALL_CAPTURE_DIR")?;
    let capture_dir = Path::new(&capture_dir);
    let index = provisioning::item_index()?;
    let skip_reference = provisioning::item_reference(&index, 3031)?.get();
    let accept_reference = provisioning::item_reference(&index, 3350)?.get();
    let mut client = connect_world(clients, 0x07).await?;
    let mut skip = request(QuickLootAction::FilterAdd);
    skip.filter_mode = Some(QuickLootFilterMode::Skip);
    skip.filter_definition_ref = Some(skip_reference);
    let mut accept = request(QuickLootAction::FilterAdd);
    accept.filter_mode = Some(QuickLootFilterMode::Accept);
    accept.filter_definition_ref = Some(accept_reference);
    let mut pouch = request(QuickLootAction::Assign);
    pouch.category = Some(QuickLootCategory::Gold);
    pouch.recipient = Some(QuickLootRecipient::GoldPouch);
    pouch.recipient_id = Some([7; 16]);
    pouch.fallback_to_main = true;
    let requests = [skip, accept, pouch, request(QuickLootAction::LootAll)];
    let mut item_count = 0;
    for (index, request) in requests.into_iter().enumerate() {
        let payload = encode_quick_loot_request(&request).map_err(|error| format!("{error:?}"))?;
        let reply = client
            .session_mut()
            .gated_command(98, 114, &payload)
            .await?;
        let expected_status = if index == 2 {
            CommandStatus::Rejected
        } else {
            CommandStatus::Accepted
        };
        if reply.status != expected_status {
            return Err(format!(
                "quick-loot {:?} refused: {:?}",
                request.action, reply.status
            )
            .into());
        }
        if reply.status == CommandStatus::Accepted {
            item_count = decode_quick_loot_result(&reply.payload)
                .map_err(|error| format!("quick-loot reply: {error:?}"))?
                .items
                .len();
        }
        let key = index + 1;
        write_capture(
            &capture_dir.join(format!("i07-{key}-request.bin")),
            &payload,
        )?;
        write_capture(
            &capture_dir.join(format!("i07-{key}-status.bin")),
            &(reply.status as u32).to_be_bytes(),
        )?;
        write_capture(&capture_dir.join(format!("i07-{key}.bin")), &reply.payload)?;
    }
    write_capture(
        &capture_dir.join("i07-expected.txt"),
        format!(
            "skip={skip_reference} accept={accept_reference} confirmed=2 results={item_count}\n"
        )
        .as_bytes(),
    )?;
    Ok(())
}
