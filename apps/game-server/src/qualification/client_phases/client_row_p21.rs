//! P21 client replay capture: the real IMBUEMENT query reply for a carried item that has imbuement
//! slots and offers, and the server's answer to a query for an item that is not carried. The
//! consumer is `apply_result` and `panel_text` in `crates/client-ui/src/features/0068-imbue`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::imbuement::{
    CAPABILITY_IMBUE_V1, CAPABILITY_IMBUEMENT_SUCCESS_CHANCE_V1, COMMAND_TYPE_IMBUEMENT,
    ImbuementAction, ImbuementRequest, ImbuementResult, decode_imbuement_reply,
    decode_imbuement_reply_with_success, encode_imbuement_request,
};
use sha2::{Digest as _, Sha256};
use std::num::NonZeroU64;

const SWORD: u32 = 3264;

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let root = required("E2E_ALL_CAPTURE_DIR")?;
    std::fs::write(Path::new(&root).join(name), bytes)?;
    Ok(())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let outcome = client.session_mut().logout().await?;
    if outcome.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("P21 logout refused: {:?}", outcome.result).into());
    }
    Ok(())
}

async fn query(
    client: &mut DevClientSession,
    item: NonZeroU64,
) -> TestResult<(CommandStatus, u64, Vec<u8>)> {
    let payload = encode_imbuement_request(&ImbuementRequest {
        item,
        action: ImbuementAction::Query,
    })
    .map_err(|error| format!("request encoder: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(CAPABILITY_IMBUE_V1, COMMAND_TYPE_IMBUEMENT, &payload)
        .await?;
    Ok((reply.status, reply.server_sequence, reply.payload))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    character_prep::declare_seeded(
        "P21.client",
        "sword-3264",
        "the row exercises imbuement queries and client rendering",
    );
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_P21_ACCOUNT").await?;
    let scope = dedicated.clients(clients);
    let mut client = connect_character(&scope, 0x46, 0).await?;
    let definitions = provisioning::item_index()?;
    let definition = provisioning::item_reference(&definitions, SWORD)?;
    let before = client
        .inventory()
        .ok_or("fixture inventory frame missing")?;
    if before
        .entries
        .iter()
        .chain(before.main_backpack.iter())
        .chain(before.equipment.iter().map(|entry| &entry.item))
        .any(|entry| entry.item_definition_ref == definition)
    {
        return Err("P21 sword already present before fixture grant".into());
    }
    let setup_session = client.join_snapshot().game_session_id;
    character_prep::grant_item(&client, SWORD, 1).await?;
    logout(&mut client).await?;
    drop(client);
    let mut client = connect_character(&scope, 0x46, 0).await?;
    if client.join_snapshot().game_session_id == setup_session {
        return Err("P21 reused the fixture grant session".into());
    }
    if !client
        .selected_capabilities()
        .contains(&CAPABILITY_IMBUE_V1)
    {
        return Err("admission omitted the imbue capability".into());
    }
    let success_selected = client
        .selected_capabilities()
        .contains(&CAPABILITY_IMBUEMENT_SUCCESS_CHANCE_V1);
    let decode = |payload: &[u8]| {
        if success_selected {
            decode_imbuement_reply_with_success(payload)
        } else {
            decode_imbuement_reply(payload).map(|reply| (reply, Vec::new()))
        }
    };
    let inventory = client.inventory().ok_or("inventory frame missing")?;
    let mut swords = inventory
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref == definition);
    let sword = swords.next().ok_or("fixture sword missing after relog")?;
    if sword.count.get() != 1 || swords.next().is_some() {
        return Err("P21 expected exactly one fixture sword".into());
    }
    let handle = sword.handle;
    let (status, window_sequence, payload) = query(&mut client, handle).await?;
    if status != CommandStatus::Accepted {
        return Err(format!("fixture sword query refused: {status:?}").into());
    }
    let (reply, success) = decode(&payload).map_err(|error| format!("reply decode: {error:?}"))?;
    let found = reply.window.ok_or("fixture sword query has no window")?;
    if reply.result != ImbuementResult::Ok
        || found.slot_count != 2
        || found.offers.is_empty()
        || !found.applied.is_empty()
    {
        return Err(format!("unexpected fixture sword window: {found:?}").into());
    }
    capture("p21-window.bin", &payload)?;
    let before_refusal = client
        .inventory()
        .ok_or("inventory before refusal missing")?
        .clone();
    let missing = NonZeroU64::new(u64::MAX).ok_or("nonzero handle")?;
    let (status, refused_sequence, refused) = query(&mut client, missing).await?;
    if status != CommandStatus::Accepted {
        return Err(format!("unknown item query failed at command boundary: {status:?}").into());
    }
    let (refusal, _) = decode(&refused).map_err(|error| format!("refusal decode: {error:?}"))?;
    if refusal.result != ImbuementResult::NotAllowed || refusal.window.is_some() {
        return Err(format!("unexpected unknown item query result: {refusal:?}").into());
    }
    if client.inventory() != Some(&before_refusal) {
        return Err("unknown item query changed the carried inventory".into());
    }
    capture("p21-refused.bin", &refused)?;
    if window_sequence == 0 || refused_sequence <= window_sequence {
        return Err("P21 result sequences are not increasing".into());
    }
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let window_sha256 = format!("{:x}", Sha256::digest(&payload));
    let refused_sha256 = format!("{:x}", Sha256::digest(&refused));
    let refused_result = refusal.result;
    let mut expected = format!(
        "session={session}\nwindow_sequence={window_sequence}\nrefused_sequence={refused_sequence}\nwindow_sha256={window_sha256}\nrefused_sha256={refused_sha256}\nsuccess_selected={success_selected}\nslot_count={}\nclear_gold={}\napplied={}\noffers={}\n",
        found.slot_count,
        found.clear_gold,
        found.applied.len(),
        found.offers.len()
    );
    for (index, offer) in found.offers.iter().enumerate() {
        expected.push_str(&format!(
            "offer_{index}={},{},{},{}\n",
            offer.family, offer.tier, offer.gold, offer.requires_premium
        ));
    }
    for (index, percent) in success.iter().enumerate() {
        expected.push_str(&format!("offer_{index}_success={percent}\n"));
    }
    expected.push_str(&format!(
        "refused_status={status:?}\nrefused_result={refused_result:?}\n"
    ));
    capture("p21-expected.txt", expected.as_bytes())?;
    logout(&mut client).await?;
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P21.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P21.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P21.client status=FAIL evidence=timeout"),
    }
}
