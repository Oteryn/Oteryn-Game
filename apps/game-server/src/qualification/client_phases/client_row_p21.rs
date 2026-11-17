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
use std::num::NonZeroU64;

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "P21_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
}

async fn query(
    client: &mut DevClientSession,
    item: NonZeroU64,
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let payload = encode_imbuement_request(&ImbuementRequest {
        item,
        action: ImbuementAction::Query,
    })
    .map_err(|error| format!("request encoder: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(CAPABILITY_IMBUE_V1, COMMAND_TYPE_IMBUEMENT, &payload)
        .await?;
    Ok((reply.status, reply.payload))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_character(clients, 0x46, 1).await?;
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
    let handles: Vec<NonZeroU64> = {
        let inventory = client.inventory().ok_or("inventory frame missing")?;
        inventory
            .entries
            .iter()
            .chain(inventory.main_backpack.iter())
            .chain(inventory.equipment.iter().map(|entry| &entry.item))
            .map(|entry| entry.handle)
            .collect()
    };
    let mut window = None;
    for handle in handles {
        let (status, payload) = query(&mut client, handle).await?;
        if status != CommandStatus::Accepted {
            continue;
        }
        let (reply, success) =
            decode(&payload).map_err(|error| format!("reply decode: {error:?}"))?;
        if let Some(found) = reply.window
            && found.slot_count > 0
            && !found.offers.is_empty()
        {
            window = Some((payload, found, success));
            break;
        }
    }
    let (payload, found, success) =
        window.ok_or("no carried item has imbuement slots and offers")?;
    capture("p21-window.bin", &payload);
    let missing = NonZeroU64::new(u64::MAX).ok_or("nonzero handle")?;
    let (status, refused) = query(&mut client, missing).await?;
    let refused_result = if status == CommandStatus::Accepted {
        let (reply, _) = decode(&refused).map_err(|error| format!("refusal decode: {error:?}"))?;
        if reply.result == ImbuementResult::Ok || reply.window.is_some() {
            return Err(format!("unknown item query was not refused: {reply:?}").into());
        }
        capture("p21-refused.bin", &refused);
        format!("{:?}", reply.result)
    } else {
        "None".to_owned()
    };
    let mut expected = format!(
        "success_selected={success_selected}\nslot_count={}\nclear_gold={}\napplied={}\noffers={}\n",
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
        "refused_status={status:?}\nrefused_result={refused_result}\n"
    ));
    capture("p21-expected.txt", expected.as_bytes());
    let _ = client.session_mut().logout().await?;
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
