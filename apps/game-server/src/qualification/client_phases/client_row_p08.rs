//! P08 client replay capture: real weapon-proficiency catalogue (660), browse (641), perk
//! selection (378) and a stale-revision RankUp refusal. The consumer is `apply_result` in
//! `crates/client-ui/src/features/0195-weapon-proficiency/protocol.rs`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::weapon_proficiency as wire;
use std::num::NonZeroU32;

const WAND: u32 = 3074;

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("P08 logout refused: {reply:?}").into());
    }
    Ok(())
}

async fn track(
    client: &mut DevClientSession,
    definition: NonZeroU32,
) -> TestResult<wire::ProficiencyRow> {
    let mut offset = 0_u32;
    loop {
        let reply = client
            .session_mut()
            .gated_command(
                wire::CAPABILITY_WEAPON_PROFICIENCY_V1,
                wire::COMMAND_TYPE_PROFICIENCY_BROWSE,
                &offset.to_be_bytes(),
            )
            .await?;
        let page = wire::ProficiencyPage::decode(&reply.payload).ok_or("P08 track browse page")?;
        if reply.status != CommandStatus::Accepted || page.offset != offset || page.rows.is_empty()
        {
            return Err("P08 track browse refused or did not advance".into());
        }
        let count = u32::try_from(page.rows.len())?;
        if let Some(row) = page
            .rows
            .into_iter()
            .find(|row| row.definition == definition)
        {
            return Ok(row);
        }
        offset = offset
            .checked_add(count)
            .ok_or("P08 browse offset overflow")?;
        if offset >= page.total {
            return Err("P08 wand omitted from browse".into());
        }
    }
}

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    use std::io::Write;
    std::fs::File::create_new(Path::new(&root).join(name))?.write_all(bytes)?;
    Ok(())
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
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let metadata = format!(
        "{session} {} {} {}",
        reply.command_id, reply.server_sequence, reply.status as u32
    );
    capture(&format!("p08-{name}.meta"), metadata.as_bytes())?;
    capture(&format!("p08-{name}.bin"), &reply.payload)?;
    Ok(reply.payload)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0x2a).await?;
    let first_session = client.join_snapshot().game_session_id;
    character_prep::declare_seeded(
        "P08.client",
        "Wand of Vortex track at first threshold, no perks selected",
        "select and RankUp replay prerequisites",
    );
    request_fixture_control(
        uuid_text(first_session.as_bytes()),
        "--profile",
        "p08-track".into(),
    )
    .await?;
    logout(&mut client).await?;
    drop(client);
    let mut client = connect_world(clients, 0x2b).await?;
    if client.join_snapshot().game_session_id == first_session {
        return Err("P08 fixture relog reused its admission".into());
    }
    let definition = provisioning::item_reference(&provisioning::item_index()?, WAND)?;
    let before = track(&mut client, definition).await?;
    if before.level != 1
        || before.experience
            != crate::domain::weapon_proficiency::ProficiencyThresholdClass::Standard.thresholds()
                [0]
        || before.selections.is_empty()
        || before.selections.iter().any(Option::is_some)
    {
        return Err(format!("P08 unlocked unselected pre-state missing: {before:?}").into());
    }
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
    let entry = catalogue.entries.first().ok_or("empty catalogue page")?;
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
    let mut selected = before;
    selected.selections[0] = Some(0);
    if select.result != wire::SelectPerkResult::Accepted
        || select.selections != selected.selections
        || track(&mut client, definition).await? != selected
    {
        return Err(
            format!("P08 first selection did not change the exact track: {select:?}").into(),
        );
    }
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
    if stale.result != wire::ModificationResult::StaleRevision || stale.slot.is_some() {
        return Err(
            format!("RankUp did not return StaleRevision without a slot: {stale:?}").into(),
        );
    }
    if track(&mut client, definition).await? != selected {
        return Err("P08 stale RankUp changed the track".into());
    }
    let session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let mut client = connect_world(clients, 0x2c).await?;
    if client.join_snapshot().game_session_id == session
        || track(&mut client, definition).await? != selected
    {
        return Err("P08 fresh admission did not restore the exact selected track".into());
    }
    logout(&mut client).await?;
    let session = uuid_text(session.as_bytes());
    let expected = format!(
        "session={session}\ncatalogue_total={}\ncatalogue_first={}|{}|{}\nmutation_definition={definition}\nbrowse_total={}\nbrowse_rows={}\nselect_result={:?}\nselect_levels={:?}\nstale_result={:?}\n",
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
    capture("p08-expected.txt", expected.as_bytes())?;
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
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
