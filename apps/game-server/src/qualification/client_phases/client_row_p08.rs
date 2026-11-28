//! P08 client replay capture: real weapon-proficiency catalogue (660), browse (641), perk
//! selection (378) and a stale-revision RankUp refusal. The consumer is `apply_result` in
//! `crates/client-ui/src/features/0195-weapon-proficiency/protocol.rs`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::weapon_proficiency as wire;
use oteryn_protocol_oteryn::weapon_proficiency_state as shaping;
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
    let root =
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("P08 live capture directory missing")?;
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
        .gated_command(
            if kind == shaping::COMMAND_TYPE_WEAPON_PROFICIENCY_STATE {
                shaping::CAPABILITY_WEAPON_PROFICIENCY_STATE_V1
            } else {
                wire::CAPABILITY_WEAPON_PROFICIENCY_V1
            },
            kind,
            payload,
        )
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

async fn shaping_state(
    client: &mut DevClientSession,
    definition: NonZeroU32,
    capture_name: Option<&str>,
) -> TestResult<shaping::State> {
    let kind = shaping::COMMAND_TYPE_WEAPON_PROFICIENCY_STATE;
    let request = definition.get().to_be_bytes();
    let payload = if let Some(name) = capture_name {
        command(client, kind, &request, name).await?
    } else {
        let reply = client
            .session_mut()
            .gated_command(
                shaping::CAPABILITY_WEAPON_PROFICIENCY_STATE_V1,
                kind,
                &request,
            )
            .await?;
        if reply.status != CommandStatus::Accepted {
            return Err(format!("P08 shaping read refused: {:?}", reply.status).into());
        }
        reply.payload
    };
    let state = shaping::State::decode(&payload).ok_or("P08 invalid shaping state")?;
    if state.definition != definition.get() || state.revision == 0 {
        return Err(format!("P08 wrong committed shaping state: {state:?}").into());
    }
    Ok(state)
}

async fn dust(clients: &SeamClients<'_>) -> TestResult<(i64, Option<String>)> {
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let result = sqlx::query_as(
        "SELECT COALESCE(d.balance, 0), d.last_entry_id::text
         FROM game_character_roots r
         LEFT JOIN game_character_forge_dust d USING (character_id)
         WHERE r.character_id = $1::uuid",
    )
    .bind(uuid_text(&clients.characters[1]))
    .fetch_one(&mut db)
    .await?;
    Ok(result)
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
    use crate::content::{
        ProjectV2ProficiencyThresholdClass as Class, import_committed_weapon_proficiency_data,
    };
    use crate::domain::weapon_proficiency::ProficiencyThresholdClass as Threshold;
    let data = import_committed_weapon_proficiency_data()?;
    let index = provisioning::item_index()?;
    let catalogue_total = u32::try_from(data.bindings().len())?;
    let ordinal = data
        .bindings()
        .keys()
        .position(|key| key == "oteryn:item.tibia.i3074")
        .ok_or("P08 pinned catalogue lacks Wand")?;
    let page_size = wire::PROFICIENCY_CATALOGUE_PAGE_SIZE;
    let wand_offset = ordinal / page_size * page_size;
    let mut first = None;
    for offset in (0..=wand_offset).step_by(page_size) {
        let expected_count = (data.bindings().len() - offset).min(page_size);
        let payload = command(
            &mut client,
            wire::COMMAND_TYPE_PROFICIENCY_CATALOGUE,
            &u32::try_from(offset)?.to_be_bytes(),
            &format!("catalogue-{offset}"),
        )
        .await?;
        let page = wire::ProficiencyCataloguePage::decode(&payload).ok_or("P08 catalogue page")?;
        if page.offset != u32::try_from(offset)?
            || page.total != catalogue_total
            || page.entries.len() != expected_count
        {
            return Err(format!("P08 wrong catalogue bounds at {offset}").into());
        }
        for (entry, (key, binding)) in page.entries.iter().zip(data.bindings().iter().skip(offset))
        {
            let profile = &data.definitions()[&binding.profile_binding.key];
            let reference = index
                .definition_ref("Item", key, &binding.item.revision)
                .map_or(0, NonZeroU32::get);
            let thresholds = match binding.threshold_class {
                Class::Standard => Threshold::Standard,
                Class::Knight => Threshold::Knight,
                Class::Crossbow => Threshold::Crossbow,
            };
            let xp = if reference == definition.get() {
                before.experience
            } else {
                0
            };
            if entry.weapon != *key
                || entry.definition != reference
                || entry.name != profile.name
                || entry.experience != xp
                || entry.thresholds != *thresholds.thresholds()
                || entry.levels.len() != profile.levels.len()
                || entry
                    .levels
                    .iter()
                    .zip(&profile.levels)
                    .any(|(actual, expected)| {
                        actual.selected.is_some() || actual.perks.len() != expected.perks.len()
                    })
            {
                return Err(
                    format!("P08 catalogue differs from pinned binding {key}: {entry:?}").into(),
                );
            }
        }
        if offset == 0 {
            if !page.entries.iter().any(|entry| entry.definition != 0) {
                return Err("P08 first catalogue page contains no admitted weapon".into());
            }
            first = page.entries.into_iter().next();
        }
    }
    let entry = first.ok_or("P08 empty initial catalogue")?;
    let browse = command(
        &mut client,
        wire::COMMAND_TYPE_PROFICIENCY_BROWSE,
        &0_u32.to_be_bytes(),
        "browse",
    )
    .await?;
    let browse = wire::ProficiencyPage::decode(&browse).ok_or("browse page")?;
    const FIRST_BROWSE_NAMES: [&str; 20] = [
        "drakinata",
        "traditional sai",
        "Zaoan sword",
        "drachaku",
        "twin hooks",
        "Zaoan halberd",
        "twiceslicer",
        "snake god's sceptre",
        "blade of corruption",
        "wand of dimensions",
        "wooden sword",
        "heavy trident",
        "shimmer sword",
        "shimmer rod",
        "shimmer bow",
        "shimmer wand",
        "deepling staff",
        "deepling axe",
        "ornate mace",
        "warrior's axe",
    ];
    let admitted = data
        .bindings()
        .iter()
        .filter_map(|(key, binding)| {
            index
                .definition_ref("Item", key, &binding.item.revision)
                .map(|reference| (reference, &data.definitions()[&binding.profile_binding.key]))
        })
        .collect::<Vec<_>>();
    if browse.offset != 0
        || usize::try_from(browse.total)? != admitted.len()
        || browse.rows.len() != FIRST_BROWSE_NAMES.len()
        || browse
            .rows
            .iter()
            .zip(&admitted)
            .zip(FIRST_BROWSE_NAMES)
            .any(|((row, (reference, profile)), name)| {
                row.definition != *reference
                    || row.name != name
                    || row.experience != 0
                    || row.level != 0
                    || row.selections != vec![None; profile.levels.len()]
            })
    {
        return Err(format!("P08 Browse differs from pinned first page: {browse:?}").into());
    }
    let prior_shaping = shaping_state(&mut client, definition, Some("state")).await?;
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
    let committed = shaping_state(&mut client, definition, None).await?;
    if committed.revision <= prior_shaping.revision || committed.slots != prior_shaping.slots {
        return Err(
            "P08 selection did not advance the revision with shaping slots unchanged".into(),
        );
    }
    let dust_before = dust(clients).await?;
    let operation = wire::ModificationOperation::RankUp;
    let stale = command(
        &mut client,
        operation.command_type(),
        &wire::ModificationRequest {
            definition,
            slot: 1,
            operation,
            expected_revision: prior_shaping.revision,
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
    if track(&mut client, definition).await? != selected
        || shaping_state(&mut client, definition, None).await? != committed
        || dust(clients).await? != dust_before
    {
        return Err("P08 stale RankUp changed the track".into());
    }
    let session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let mut client = connect_world(clients, 0x2c).await?;
    if client.join_snapshot().game_session_id == session
        || track(&mut client, definition).await? != selected
        || shaping_state(&mut client, definition, None).await? != committed
        || dust(clients).await? != dust_before
    {
        return Err("P08 fresh admission did not restore the exact selected track".into());
    }
    logout(&mut client).await?;
    let session = uuid_text(session.as_bytes());
    let expected = format!(
        "session={session}\ncatalogue_total={catalogue_total}\nwand_offset={wand_offset}\ncatalogue_first={}|{}|{}\nmutation_definition={definition}\nbrowse_total={}\nbrowse_rows={}\nselect_result={:?}\nselect_levels={:?}\nstale_result={:?}\n",
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
