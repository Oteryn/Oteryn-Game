//! G12 weapon proficiency state and perks (capabilities 448 and 195, commands 732 and 378).
//! 1. Admit the node-boot character; read catalogue page 0 (command 660) for the weapon
//!    definitions the server knows.
//! 2. Query the state projection (command 732) for each definition and require the echoed
//!    definition; this proves the read route.
//! 3. A perk can only be chosen on a slot whose proficiency level the server granted from weapon
//!    experience. For an unlocked, unselected, unmodified catalogue level, select the first perk
//!    (command 378) and require
//!    a newer state revision; otherwise fail naming the missing experience.
//!
//! The row never grants experience.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::{weapon_proficiency as catalogue, weapon_proficiency_state as wire};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete_with_timeout(Duration::from_secs(90), journey(clients)).await;
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

async fn finish(client: &mut RowClient) -> TestResult {
    if client.row_logout().await?.result
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
) -> TestResult<catalogue::ProficiencyCataloguePage> {
    let payload = accepted(
        client,
        catalogue::CAPABILITY_WEAPON_PROFICIENCY_V1,
        catalogue::COMMAND_TYPE_PROFICIENCY_CATALOGUE,
        &0u32.to_be_bytes(),
    )
    .await?;
    catalogue::ProficiencyCataloguePage::decode(&payload)
        .ok_or_else(|| "step=catalogue observed=decode-failed".into())
}

/// Find an unlocked, unselected catalogue level outside the shaping slots.
fn selectable_level(
    entry: &catalogue::ProficiencyCatalogueEntry,
    state: &wire::State,
) -> Option<u8> {
    entry
        .levels
        .iter()
        .zip(entry.thresholds)
        .enumerate()
        .find_map(|(index, (level, threshold))| {
            let index = u8::try_from(index).ok()?;
            (entry.experience >= threshold
                && level.selected.is_none()
                && !level.perks.is_empty()
                && !state.slots.iter().flatten().any(|slot| slot.level == index))
            .then_some(index)
        })
}

/// Select the first ordinary unlocked perk; returns `(definition, level, revision)`.
async fn body(client: &mut DevClientSession) -> TestResult<(u32, u8, u64)> {
    let page = catalogue_page(client).await?;
    let definitions: Vec<_> = page
        .entries
        .iter()
        .filter(|entry| entry.definition != 0)
        .collect();
    if definitions.is_empty() {
        return Err("step=catalogue observed=no-weapon-definitions needed=weapon-catalogue".into());
    }
    let mut slotless = Vec::new();
    for entry in definitions {
        let definition = entry.definition;
        let before = state(client, definition).await?;
        let Some(level) = selectable_level(entry, &before) else {
            slotless.push(format!("{definition}:xp{}", entry.experience));
            continue;
        };
        let request = catalogue::SelectPerkRequest {
            definition: std::num::NonZeroU32::new(definition).ok_or("step=perk observed=zero")?,
            level,
            perk: Some(0),
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
        if reply.result != catalogue::SelectPerkResult::Accepted || chosen != Some(Some(0)) {
            return Err(format!(
                "step=perk observed=reply:{:?}-selection:{chosen:?} needed=accepted-selection-of-perk-0",
                reply.result
            )
            .into());
        }
        if after.revision <= before.revision {
            return Err(format!(
                "step=perk observed=level{level}-reply:{:?}-revision:{}->{} needed=state-revision-advance",
                reply.result, before.revision, after.revision
            )
            .into());
        }
        return Ok((definition, level, after.revision));
    }
    Err(format!(
        "step=state observed=no-selectable-level:{} needed=unlocked-unselected-unmodified-proficiency-level",
        slotless.join(",")
    )
    .into())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = row_connect_world(clients, 0x72).await?;
    let outcome = body(&mut client).await;
    let closed = finish(&mut client).await;
    let (definition, level, revision) = outcome?;
    closed?;
    drop(client);

    let mut fresh = row_connect_world(clients, 0x73).await?;
    let durable = durable_selection(&mut fresh, definition, level, revision).await;
    let closed = finish(&mut fresh).await;
    durable?;
    closed
}

/// After relog the committed selection and the state revision must still be there.
async fn durable_selection(
    client: &mut DevClientSession,
    definition: u32,
    level: u8,
    revision: u64,
) -> TestResult {
    let page = catalogue_page(client).await?;
    let selected = page
        .entries
        .iter()
        .find(|entry| entry.definition == definition)
        .and_then(|entry| entry.levels.get(usize::from(level)))
        .map(|slot| slot.selected);
    if selected != Some(Some(0)) {
        return Err(format!(
            "step=relog observed=selection:{selected:?} needed=durable-perk-0-at-level-{level}"
        )
        .into());
    }
    let after = state(client, definition).await?;
    if after.revision < revision {
        return Err(format!(
            "step=relog observed=revision:{} needed=at-least:{revision}",
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
    fn ordinary_selection_uses_catalogue_unlocks_and_excludes_modified_levels() {
        let mut entry = catalogue::ProficiencyCatalogueEntry {
            definition: 1,
            weapon: "weapon".into(),
            name: "Weapon".into(),
            experience: 10,
            thresholds: [10, 20, 30, 40, 50, 60, 70, 80, 90],
            levels: vec![
                catalogue::ProficiencyCatalogueLevel {
                    selected: None,
                    perks: vec!["perk".into()]
                };
                2
            ],
        };
        let mut state = wire::State {
            definition: 1,
            revision: 1,
            slots: [None, None],
            modify_dust: [0; 2],
            rank_dust: [0; 10],
            reshape_dust: 0,
            clear_dust: 0,
            orbs: 0,
        };
        assert_eq!(selectable_level(&entry, &state), Some(0));
        entry.experience = 9;
        assert_eq!(selectable_level(&entry, &state), None);
        entry.experience = 10;
        entry.levels[0].selected = Some(0);
        assert_eq!(selectable_level(&entry, &state), None);
        entry.levels[0].selected = None;
        state.slots[0] = Some(wire::Slot {
            level: 0,
            rank: 0,
            current: "current".into(),
            next: "next".into(),
            maximum: "maximum".into(),
            offers: vec!["offer".into(); 3],
        });
        assert_eq!(selectable_level(&entry, &state), None);
        entry.experience = 20;
        assert_eq!(selectable_level(&entry, &state), Some(1));
        entry.levels[1].perks.clear();
        assert_eq!(selectable_level(&entry, &state), None);
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
