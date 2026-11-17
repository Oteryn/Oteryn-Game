//! G12 weapon proficiency state and perks (capabilities 448 and 195, commands 732 and 378).
//! 1. Admit the node-boot character; read catalogue pages (command 660) for the weapon
//!    definitions the server knows.
//! 2. Query the state projection (command 732) for each definition and require the echoed
//!    definition; this proves the read route.
//! 3. A perk can only be chosen on a slot whose proficiency level the server granted from weapon
//!    experience. When a slot with offers exists, select a different perk (command 378) and require
//!    a newer state revision; otherwise fail naming the missing experience.
//!
//! The row never grants experience.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::{weapon_proficiency as catalogue, weapon_proficiency_state as wire};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
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
    if page.offset != offset {
        return Err(format!("step=catalogue observed=offset:{};needed={offset}", page.offset).into());
    }
    Ok(page)
}

fn alternate_perk(selected: Option<u8>, offer_count: usize) -> Option<u8> {
    let perk = u8::from(selected == Some(0));
    (usize::from(perk) < offer_count).then_some(perk)
}

#[derive(Debug)]
struct Selection {
    offset: u32,
    definition: u32,
    level: u8,
    perk: u8,
    revision: u64,
}

/// Read the catalogue view of the exact level, using the owner's zero-based level ordinal.
fn selected_perk(page: &catalogue::ProficiencyCataloguePage, definition: u32, level: u8) -> Option<Option<u8>> {
    page.entries.iter().find(|entry| entry.definition == definition)
        .and_then(|entry| entry.levels.get(usize::from(level)))
        .map(|level| level.selected)
}

async fn body(client: &mut DevClientSession) -> TestResult<Selection> {
    let mut offset = 0u32;
    let mut definitions = Vec::new();
    loop {
        let page = catalogue_page(client, offset).await?;
        let next = offset.checked_add(u32::try_from(page.entries.len())?).ok_or("step=catalogue observed=offset-overflow")?;
        definitions.extend(page.entries.into_iter().filter(|entry| entry.definition != 0).map(|entry| (offset, entry)));
        if next == page.total {
            break;
        }
        if next <= offset {
            return Err("step=catalogue observed=non-advancing-offset".into());
        }
        offset = next;
    }
    if definitions.is_empty() {
        return Err("step=catalogue observed=no-weapon-definitions needed=weapon-catalogue".into());
    }
    let mut slotless = Vec::new();
    for (offset, entry) in definitions {
        let definition = entry.definition;
        let before = state(client, definition).await?;
        let Some((level, perk)) = before.slots.iter().flatten().find_map(|slot| {
            let level = entry.levels.get(usize::from(slot.level))?;
            alternate_perk(level.selected, level.perks.len())
                .filter(|_| !slot.offers.is_empty())
                .map(|perk| (slot.level, perk))
        }) else {
            slotless.push(format!("{definition}:xp{}", entry.experience));
            continue;
        };
        let request = catalogue::SelectPerkRequest {
            definition: std::num::NonZeroU32::new(definition).ok_or("step=perk observed=zero")?,
            level,
            perk: Some(perk),
        };
        let payload = accepted(
            client,
            catalogue::CAPABILITY_WEAPON_PROFICIENCY_V1,
            catalogue::COMMAND_TYPE_PROFICIENCY_SELECT_PERK,
            &request.encode(),
        )
        .await?;
        let reply = catalogue::SelectPerkReply::decode(&payload)
            .ok_or("step=perk observed=reply-decode-failed")?;
        let after = state(client, definition).await?;
        let chosen = reply.selections.get(usize::from(level)).copied();
        if reply.result != catalogue::SelectPerkResult::Accepted || chosen != Some(Some(perk)) {
            return Err(format!("step=perk observed=reply:{:?}-selection:{chosen:?};needed=accepted-perk-{perk}", reply.result).into());
        }
        if after.revision <= before.revision {
            return Err(format!("step=perk observed=revision:{}->{};needed=advance", before.revision, after.revision).into());
        }
        let selection = Selection { offset, definition, level, perk, revision: after.revision };
        verify_selection(client, &selection).await?;
        return Ok(selection);
    }
    Err(format!(
        "step=state observed=no-slot-with-offers:{} needed=weapon-proficiency-experience",
        slotless.join(",")
    )
    .into())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0x72).await?;
    let outcome = verdict::observe(body(&mut client)).await;
    let closed = verdict::observe(finish(&mut client)).await;
    let selection = outcome?;
    closed?;
    drop(client);

    let mut fresh = connect_world(clients, 0x73).await?;
    let durable = verdict::observe(verify_selection(&mut fresh, &selection)).await;
    let closed = verdict::observe(finish(&mut fresh)).await;
    durable?;
    closed
}

/// Read the selection independently of the mutation reply, both live and after admission.
async fn verify_selection(client: &mut DevClientSession, selection: &Selection) -> TestResult {
    let page = catalogue_page(client, selection.offset).await?;
    let selected = selected_perk(&page, selection.definition, selection.level);
    let after = state(client, selection.definition).await?;
    if selected != Some(Some(selection.perk)) || after.revision < selection.revision {
        return Err(format!("step=selection-query observed=perk:{selected:?},revision:{};needed={selection:?}", after.revision).into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perk_selection_requires_a_different_available_perk() {
        assert_eq!(alternate_perk(Some(0), 2), Some(1));
        assert_eq!(alternate_perk(Some(0), 1), None);
        assert_eq!(alternate_perk(Some(1), 2), Some(0));
    }

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
