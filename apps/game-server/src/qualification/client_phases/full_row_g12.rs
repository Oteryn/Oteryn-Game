//! G12 weapon proficiency state and perks (capabilities 448 and 195, commands 732 and 378).
//! 1. Admit the node-boot character; read catalogue page 0 (command 660) for the weapon
//!    definitions the server knows.
//! 2. Query the state projection (command 732) for each definition and require the echoed
//!    definition; this proves the read route.
//! 3. A perk can only be chosen on a slot whose proficiency level the server granted from weapon
//!    experience. When a slot with offers exists, select the first perk (command 378) and require
//!    a newer state revision; otherwise fail naming the missing experience.
//!
//! The row never grants experience.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::{weapon_proficiency as catalogue, weapon_proficiency_state as wire};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(90), journey(clients)).await;
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

async fn body(client: &mut DevClientSession, _clients: &SeamClients<'_>) -> TestResult {
    let payload = accepted(
        client,
        catalogue::CAPABILITY_WEAPON_PROFICIENCY_V1,
        catalogue::COMMAND_TYPE_PROFICIENCY_CATALOGUE,
        &0u32.to_be_bytes(),
    )
    .await?;
    let page = catalogue::ProficiencyCataloguePage::decode(&payload)
        .ok_or("step=catalogue observed=decode-failed")?;
    let definitions: Vec<(u32, u32)> = page
        .entries
        .iter()
        .filter(|entry| entry.definition != 0)
        .map(|entry| (entry.definition, entry.experience))
        .collect();
    if definitions.is_empty() {
        return Err("step=catalogue observed=no-weapon-definitions needed=weapon-catalogue".into());
    }
    let mut slotless = Vec::new();
    for (definition, experience) in definitions {
        let before = state(client, definition).await?;
        let Some((index, slot)) = before.slots.iter().enumerate().find_map(|(index, slot)| {
            slot.as_ref()
                .filter(|s| !s.offers.is_empty())
                .map(|s| (index, s))
        }) else {
            slotless.push(format!("{definition}:xp{experience}"));
            continue;
        };
        let request = catalogue::SelectPerkRequest {
            definition: std::num::NonZeroU32::new(definition).ok_or("step=perk observed=zero")?,
            level: slot.level,
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
        if after.revision <= before.revision {
            return Err(format!(
                "step=perk observed=slot{index}-reply:{:?}-revision:{}->{} needed=state-revision-advance",
                reply.result, before.revision, after.revision
            )
            .into());
        }
        return Ok(());
    }
    Err(format!(
        "step=state observed=no-slot-with-offers:{} needed=weapon-proficiency-experience",
        slotless.join(",")
    )
    .into())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0x72).await?;
    let outcome = body(&mut client, clients).await;
    let closed = finish(&mut client).await;
    outcome?;
    closed
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
