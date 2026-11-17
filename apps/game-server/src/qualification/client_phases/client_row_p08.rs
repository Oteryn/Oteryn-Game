//! P08 client replay capture: real weapon-proficiency catalogue (660), browse (641), perk
//! selection (378) and a stale-revision RankUp refusal. The consumer is `apply_result` in
//! `crates/client-ui/src/features/0195-weapon-proficiency/protocol.rs`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::weapon_proficiency as wire;
use std::num::NonZeroU32;

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("P08_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

async fn command(
    client: &mut DevClientSession,
    kind: u32,
    payload: &[u8],
    name: &str,
) -> TestResult<Vec<u8>> {
    let reply = client
        .session_mut()
        .gated_command(wire::CAPABILITY_WEAPON_PROFICIENCY_V1, kind, payload)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("{name} status={:?}", reply.status).into());
    }
    capture(&format!("p08-{name}.bin"), &reply.payload);
    Ok(reply.payload)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0x2a).await?;
    let catalogue = command(
        &mut client,
        wire::COMMAND_TYPE_PROFICIENCY_CATALOGUE,
        &0_u32.to_be_bytes(),
        "catalogue",
    )
    .await?;
    let catalogue = wire::ProficiencyCataloguePage::decode(&catalogue).ok_or("catalogue page")?;
    let browse = command(
        &mut client,
        wire::COMMAND_TYPE_PROFICIENCY_BROWSE,
        &0_u32.to_be_bytes(),
        "browse",
    )
    .await?;
    let browse = wire::ProficiencyPage::decode(&browse).ok_or("browse page")?;
    let (entry, definition) = catalogue
        .entries
        .iter()
        .find_map(|entry| Some((entry, NonZeroU32::new(entry.definition)?)))
        .ok_or("no admitted weapon definition in the catalogue")?;
    let select = command(
        &mut client,
        wire::COMMAND_TYPE_PROFICIENCY_SELECT_PERK,
        &wire::SelectPerkRequest {
            definition,
            level: 0,
            perk: Some(0),
        }
        .encode(),
        "select",
    )
    .await?;
    let select = wire::SelectPerkReply::decode(&select).ok_or("select reply")?;
    let operation = wire::ModificationOperation::RankUp;
    let stale = command(
        &mut client,
        operation.command_type(),
        &wire::ModificationRequest {
            definition,
            slot: 1,
            operation,
            expected_revision: u64::MAX,
        }
        .encode(),
        "stale",
    )
    .await?;
    let stale = wire::ModificationReply::decode(&stale).ok_or("stale reply")?;
    if stale.result == wire::ModificationResult::Accepted || stale.slot.is_some() {
        return Err(format!("stale RankUp was not refused: {stale:?}").into());
    }
    let expected = format!(
        "catalogue_total={}\ncatalogue_first={}|{}|{}\nbrowse_total={}\nbrowse_rows={}\nselect_result={:?}\nselect_levels={:?}\nstale_result={:?}\n",
        catalogue.total,
        entry.definition,
        entry.name,
        entry.experience,
        browse.total,
        browse.rows.len(),
        select.result,
        select.selections,
        stale.result
    );
    capture("p08-expected.txt", expected.as_bytes());
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P08.client status=CAPTURED evidence={}",
            expected.replace(' ', "_")
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P08.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P08.client status=FAIL evidence=timeout"),
    }
}
