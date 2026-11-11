//! W22b client replay capture, Prey loot-bonus slice: the Character holds an active Prey slot on
//! `oteryn:creature.rat` (selected by the full row, or selected here with ListReroll and Select
//! when no slot is active), and the raw PREY replies are written byte for byte. A SelectAny of a
//! non-offered creature is the refusal. The consumer is `apply_result` in
//! `crates/client-ui/src/features/0140-prey/protocol.rs`. Charm, daily boost, Wealth Duplex and
//! stamina modifiers have no capture here (reported by the packet).
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::prey::{self, CommandStatus};
use prey::prey as wire;

const RAT: &str = "oteryn:creature.rat";
const NOT_A_CREATURE: &str = "oteryn:creature.not_a_creature";

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

async fn send(
    client: &mut DevClientSession,
    request: wire::Request,
) -> TestResult<(prey::PreyResponse, wire::Reply, Option<wire::Reply>)> {
    let response = prey::send(client.session_mut(), &request).await?;
    if !matches!(
        response.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        return Err(format!("{request:?} status={:?}", response.status).into());
    }
    let receipt = wire::decode_reply(&response.payload)
        .map_err(|error| format!("{request:?} receipt {error:?}"))?;
    let current = match &response.current {
        Some(bytes) => Some(
            wire::decode_reply(bytes).map_err(|error| format!("{request:?} current {error:?}"))?,
        ),
        None => None,
    };
    Ok((response, receipt, current))
}

fn active(reply: &wire::Reply) -> Option<&wire::Slot> {
    reply
        .slots
        .iter()
        .find(|slot| slot.selected == RAT && slot.bonus != 0)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let mut client = connect_world(clients, 0x99).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    if !client.selected_capabilities().contains(&wire::CAPABILITY_PREY_V1) {
        return Err("step=admission Prey capability 140 not selected".into());
    }
    let (mut response, mut receipt, _) = send(&mut client, wire::Request::Query).await?;
    if active(&receipt).is_none() {
        let slot = receipt
            .slots
            .iter()
            .find(|slot| slot.state != 0)
            .map(|slot| slot.slot)
            .ok_or("step=query no unlocked Prey slot")?;
        let (_, _, current) = send(&mut client, wire::Request::ListReroll { slot }).await?;
        let current = current.ok_or("step=reroll no current state")?;
        let index = current
            .slots
            .iter()
            .find(|row| row.slot == slot)
            .and_then(|row| row.creatures.iter().position(|key| key == RAT))
            .ok_or("step=reroll the rat was not offered")?;
        send(&mut client, wire::Request::Select { slot, index: u8::try_from(index + 1)? }).await?;
        (response, receipt, _) = send(&mut client, wire::Request::Query).await?;
    }
    let slot = active(&receipt)
        .ok_or("step=select no active rat loot bonus slot")?
        .clone();
    capture("w22b-active.bin", &response.payload)?;
    let (refused, refusal, refused_current) = send(
        &mut client,
        wire::Request::SelectAny { slot: slot.slot, creature: NOT_A_CREATURE.into() },
    )
    .await?;
    let refused_current = refused_current.ok_or("step=refusal no current state")?;
    let after = refused_current.slots.iter().find(|row| row.slot == slot.slot).ok_or("step=refusal slot absent")?;
    if refusal.status == 0 || after != &slot {
        return Err(format!("step=refusal non-offered SelectAny not refused or slot changed: {refusal:?}").into());
    }
    capture("w22b-refused.bin", &refused.payload)?;
    capture(
        "w22b-refused-current.bin",
        refused.current.as_deref().ok_or("step=refusal current payload absent")?,
    )?;
    let expected = format!(
        "slot={}\nstate={}\nselected={}\nbonus={}\ngrade={}\npercent={}\nselection_epoch={}\nrefusal_status={}\n",
        slot.slot, slot.state, slot.selected, slot.bonus, slot.grade, slot.percent, slot.selection_epoch, refusal.status
    );
    capture("w22b-expected.txt", expected.as_bytes())?;
    Ok(expected.replace('\n', ";"))

    }).await.unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match row_complete(journey(clients)).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=W22b.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=W22b.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=W22b.client status=FAIL evidence=timeout"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_prey_state_is_not_an_active_or_selected_slot() {
        let reply = wire::Reply {
            status: 0,
            wildcard_balance: 0,
            slots: vec![],
            list_size: 0,
            expansion_owned: false,
            offered_today: vec![],
            offered_next: String::new(),
        };
        assert!(active(&reply).is_none());
    }
}
