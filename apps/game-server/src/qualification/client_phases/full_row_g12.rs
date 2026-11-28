//! Use one authored proficiency catalyst on the equipped weapon, then apply a catalogue perk.
//! Crystal separates ordinary apply-perks from reshaping and picking a reshape offer.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::world_object::{
    self, MapUseTarget, UseDisposition, UseIntent, UseTarget,
};
use std::num::NonZeroU32;

const CATALYST: u32 = 54266;
// Global: https://tibia.fandom.com/wiki/Lesser_Proficiency_Catalyst
const CATALYST_EXPERIENCE: u32 = 5_000;
use oteryn_protocol_oteryn::{weapon_proficiency as catalogue, weapon_proficiency_state as wire};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "proficiency-state-and-perk-selected".to_owned()),
        Ok(Err(error)) => ("FAIL", compact(&error.to_string())),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G12.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn compact(value: &str) -> String {
    let joined = value.split_whitespace().collect::<Vec<_>>().join("-");
    joined.chars().take(400).collect()
}

fn gate(client: &DevClientSession, capability: u32) -> TestResult {
    if client.selected_capabilities().contains(&capability) {
        return Ok(());
    }
    Err(format!(
        "step=admit observed=capability-{capability}-not-selected needed=capability-{capability}-selected"
    )
    .into())
}

async fn send(
    client: &mut DevClientSession,
    capability: u32,
    command: u32,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    gate(client, capability)?;
    let reply = client
        .session_mut()
        .gated_command(capability, command, payload)
        .await?;
    client.service_liveness(Duration::from_millis(50)).await?;
    Ok((reply.status, reply.payload))
}

async fn accepted(
    client: &mut DevClientSession,
    capability: u32,
    command: u32,
    payload: &[u8],
) -> TestResult<Vec<u8>> {
    let (status, payload) = send(client, capability, command, payload).await?;
    if status != CommandStatus::Accepted {
        return Err(format!(
            "step=command-{command} observed=status:{status:?} needed=accepted-reply"
        )
        .into());
    }
    Ok(payload)
}

async fn finish(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

async fn state(client: &mut DevClientSession, definition: u32) -> TestResult<wire::State> {
    let request = wire::encode_request(definition).ok_or("step=state observed=request-encode")?;
    let payload = accepted(
        client,
        wire::CAPABILITY_WEAPON_PROFICIENCY_STATE_V1,
        wire::COMMAND_TYPE_WEAPON_PROFICIENCY_STATE,
        &request,
    )
    .await?;
    let state = wire::State::decode(&payload).ok_or("step=state observed=decode-failed")?;
    if state.definition != definition {
        return Err(format!("step=state observed=echo-{}", state.definition).into());
    }
    // Global unlocks ordinary perks with XP. Replacing a perk is a separate
    // operation, available from proficiency level 3.
    if state.slots.iter().any(Option::is_some) {
        return Err("step=state observed=unexpected-shaped-slot".into());
    }
    Ok(state)
}

async fn catalogue_page(
    client: &mut DevClientSession,
    offset: u32,
) -> TestResult<catalogue::ProficiencyCataloguePage> {
    let payload = accepted(
        client,
        catalogue::CAPABILITY_WEAPON_PROFICIENCY_V1,
        catalogue::COMMAND_TYPE_PROFICIENCY_CATALOGUE,
        &offset.to_be_bytes(),
    )
    .await?;
    let page = catalogue::ProficiencyCataloguePage::decode(&payload)
        .ok_or("step=catalogue observed=decode-failed")?;
    if page.offset != offset || page.entries.is_empty() {
        return Err(format!(
            "step=catalogue observed=page-offset:{};expected:{offset};entries:{}",
            page.offset,
            page.entries.len()
        )
        .into());
    }
    Ok(page)
}

async fn weapon_entry(
    client: &mut DevClientSession,
    definitions: &[u32],
) -> TestResult<catalogue::ProficiencyCatalogueEntry> {
    let index = provisioning::item_index()?;
    let keys: Vec<_> = definitions
        .iter()
        .filter_map(|&reference| index.definition_at(NonZeroU32::new(reference)?))
        .map(|(key, _)| key)
        .collect();
    let data = oteryn_gameplay_content::content::import_committed_weapon_proficiency_data()?;
    let position = data
        .bindings()
        .keys()
        .position(|key| keys.contains(&key.as_str()))
        .ok_or_else(|| format!("step=catalogue observed=no-authored-track-for:{definitions:?}"))?;
    let page_size = catalogue::PROFICIENCY_CATALOGUE_PAGE_SIZE;
    let offset = u32::try_from(position / page_size * page_size)?;
    let first = catalogue_page(client, 0).await?;
    if first.entries.iter().all(|entry| entry.definition == 0) {
        return Err("step=catalogue observed=no-weapon-definitions-on-page-zero".into());
    }
    let page = if offset == 0 {
        first
    } else {
        let page = catalogue_page(client, offset).await?;
        if page.total != first.total {
            return Err("step=catalogue observed=page-total-changed".into());
        }
        page
    };
    page.entries
        .into_iter()
        .find(|entry| definitions.contains(&entry.definition))
        .ok_or_else(|| {
            format!("step=catalogue observed=no-served-track-for:{definitions:?}").into()
        })
}

fn catalyst_quantity(client: &DevClientSession, catalyst: NonZeroU32) -> TestResult<u64> {
    Ok(client
        .inventory()
        .ok_or("step=inventory observed=absent")?
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref == catalyst)
        .map(|entry| u64::from(entry.count.get()))
        .sum())
}

async fn earn_experience(
    client: &mut DevClientSession,
    catalyst: NonZeroU32,
    definition: u32,
) -> TestResult {
    let inventory = client.inventory().ok_or("step=inventory observed=absent")?;
    let source = inventory
        .entries
        .iter()
        .find(|entry| entry.item_definition_ref == catalyst)
        .ok_or("step=fixture observed=catalyst-missing")?;
    let weapon = inventory
        .equipment
        .iter()
        .find(|entry| entry.item.item_definition_ref.get() == definition)
        .ok_or("step=fixture observed=weapon-not-equipped")?;
    let payload = world_object::encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(source.handle),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Item(weapon.item.handle)),
        use_with_position: None,
    })
    .map_err(|error| format!("step=catalyst observed=request-encode:{error:?}"))?;
    let reply = client.session_mut().world_context_use(&payload).await?;
    let result = world_object::decode_use_result(&reply.payload)
        .map_err(|error| format!("step=catalyst observed=reply-decode:{error:?}"))?;
    if reply.status != CommandStatus::Accepted || result != UseDisposition::Committed {
        return Err(format!("step=catalyst observed={:?}/{result:?}", reply.status).into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while catalyst_quantity(client, catalyst)? != 0 {
        if tokio::time::Instant::now() >= deadline {
            return Err("step=catalyst observed=unit-not-consumed".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    Ok(())
}

async fn body(
    client: &mut DevClientSession,
    catalyst: NonZeroU32,
) -> TestResult<(u32, u8, u64, u32)> {
    let definitions: Vec<_> = client
        .inventory()
        .ok_or("step=inventory observed=absent")?
        .equipment
        .iter()
        .filter(|entry| {
            use oteryn_protocol_oteryn::item_view::EquipmentSlot;
            matches!(
                entry.slot,
                EquipmentSlot::LeftHand | EquipmentSlot::RightHand
            )
        })
        .map(|entry| entry.item.item_definition_ref.get())
        .collect();
    let initial = weapon_entry(client, &definitions).await?;
    state(client, initial.definition).await?;
    evidence(&format!(
        "G12_CHECK step=initial-track definition={} experience={}",
        initial.definition, initial.experience
    ));
    if initial.experience != 0 || initial.levels.iter().any(|slot| slot.selected.is_some()) {
        return Err("step=fixture observed=existing-proficiency".into());
    }
    earn_experience(client, catalyst, initial.definition).await?;
    evidence("G12_CHECK step=catalyst-committed-and-debited");
    let entry = weapon_entry(client, &[initial.definition]).await?;
    if entry.experience != CATALYST_EXPERIENCE {
        return Err(format!(
            "step=catalyst observed=xp:{};expected:{CATALYST_EXPERIENCE}",
            entry.experience
        )
        .into());
    }
    let level = 0;
    let first_perk = entry.levels.first().ok_or("step=perk observed=no-first-level")?;
    if initial.thresholds[0] <= initial.experience
        || entry.thresholds[0] != initial.thresholds[0]
        || entry.thresholds[0] > entry.experience
        || first_perk.selected.is_some()
        || first_perk.perks.first().is_none_or(|label| label.is_empty())
    {
        return Err("step=perk observed=first-ordinary-offer-not-unlocked".into());
    }
    let before = state(client, entry.definition).await?;
    let request = catalogue::SelectPerkRequest {
        definition: std::num::NonZeroU32::new(entry.definition).ok_or("step=perk observed=zero")?,
        level,
        perk: Some(0),
    };
    evidence(&format!(
        "G12_CHECK step=select level={level} experience={}",
        entry.experience
    ));
    let payload = accepted(
        client,
        catalogue::CAPABILITY_WEAPON_PROFICIENCY_V1,
        catalogue::COMMAND_TYPE_PROFICIENCY_SELECT_PERK,
        &request.encode(),
    )
    .await?;
    let reply = catalogue::SelectPerkReply::decode(&payload)
        .ok_or("step=perk observed=reply-decode-failed")?;
    let chosen = reply.selections.get(usize::from(level)).copied();
    if reply.result != catalogue::SelectPerkResult::Accepted || chosen != Some(Some(0)) {
        return Err(format!(
            "step=perk observed=reply:{:?}-selection:{chosen:?}",
            reply.result
        )
        .into());
    }
    let after = state(client, entry.definition).await?;
    if after.revision <= before.revision {
        return Err(format!(
            "step=perk observed=revision:{}->{} needed=state-revision-advance",
            before.revision, after.revision
        )
        .into());
    }
    let selected = weapon_entry(client, &[entry.definition]).await?;
    if selected.experience != entry.experience
        || selected
            .levels
            .get(usize::from(level))
            .map(|slot| slot.selected)
            != Some(Some(0))
    {
        return Err("step=perk observed=live-catalogue-selection-or-experience-mismatch".into());
    }
    Ok((entry.definition, level, after.revision, entry.experience))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    character_prep::declare_seeded(
        "G12",
        "one Lesser Proficiency Catalyst 54266",
        "earn the first ordinary proficiency perk through item use",
    );
    let identity = character_prep::dedicated_character(clients, "NODE_BOOT_G12_ACCOUNT").await?;
    let clients = &identity.clients(clients);
    let catalyst = provisioning::item_reference(&provisioning::item_index()?, CATALYST)?;
    let mut preparation = connect_character(clients, 0x71, 0).await?;
    if catalyst_quantity(&preparation, catalyst)? != 0 {
        return Err("step=fixture observed=existing-catalyst".into());
    }
    character_prep::grant_item(&preparation, CATALYST, 1).await?;
    finish(&mut preparation).await?;
    drop(preparation);

    let mut client = connect_character(clients, 0x72, 0).await?;
    if catalyst_quantity(&client, catalyst)? != 1 {
        return Err("step=fixture observed=expected-exactly-one-served-catalyst".into());
    }
    let outcome = verdict::observe(body(&mut client, catalyst)).await;
    let closed = verdict::observe(finish(&mut client)).await;
    let (definition, level, revision, experience) = outcome?;
    closed?;
    drop(client);

    let mut fresh = connect_character(clients, 0x73, 0).await?;
    let durable = verdict::observe(durable_selection(
        &mut fresh, definition, level, revision, experience, catalyst,
    ))
    .await;
    let closed = verdict::observe(finish(&mut fresh)).await;
    durable?;
    closed
}

/// After relog the committed selection and the state revision must still be there.
async fn durable_selection(
    client: &mut DevClientSession,
    definition: u32,
    level: u8,
    revision: u64,
    experience: u32,
    catalyst: NonZeroU32,
) -> TestResult {
    let entry = weapon_entry(client, &[definition]).await?;
    if entry.experience != experience || catalyst_quantity(client, catalyst)? != 0 {
        return Err("step=relog observed=catalyst-debit-or-proficiency-lost".into());
    }
    let selected = entry
        .levels
        .get(usize::from(level))
        .map(|slot| slot.selected);
    if selected != Some(Some(0)) {
        return Err(format!(
            "step=relog observed=selection:{selected:?} needed=durable-perk-0-at-level-{level}"
        )
        .into());
    }
    let after = state(client, definition).await?;
    if after.revision != revision {
        return Err(format!(
            "step=relog observed=revision:{} needed=exactly:{revision}",
            after.revision
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_is_one_bounded_token() {
        assert_eq!(
            compact("step=a  observed=b\nneeded=c"),
            "step=a-observed=b-needed=c"
        );
        assert!(!compact(&"x ".repeat(500)).contains(' '));
        assert!(compact(&"y".repeat(900)).len() <= 400);
    }
}
