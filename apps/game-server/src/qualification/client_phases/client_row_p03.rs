//! P03 client replay capture: real PREY replies for Query, a free list reroll with its current
//! state, and a refused SelectAny of a creature that is not offered. The consumer is
//! `apply_result` in `crates/client-ui/src/features/0140-prey/protocol.rs`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::prey::{self, CommandStatus};
use prey::prey as wire;

const NOT_A_CREATURE: &str = "oteryn:creature.not_a_creature";

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!("P03_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
    }
}

async fn send(
    client: &mut DevClientSession,
    request: wire::Request,
    name: &str,
) -> TestResult<(prey::PreyResponse, wire::Reply, Option<wire::Reply>)> {
    let response = prey::send(client.session_mut(), &request).await?;
    if !matches!(
        response.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        return Err(format!("{request:?} status={:?}", response.status).into());
    }
    capture(&format!("p03-{name}.bin"), &response.payload);
    let receipt = wire::decode_reply(&response.payload)
        .map_err(|error| format!("{request:?} receipt: {error:?}"))?;
    let current = match &response.current {
        Some(bytes) => {
            capture(&format!("p03-{name}-current.bin"), bytes);
            Some(
                wire::decode_reply(bytes)
                    .map_err(|error| format!("{request:?} current: {error:?}"))?,
            )
        }
        None => None,
    };
    Ok((response, receipt, current))
}

fn slot(reply: &wire::Reply, number: u8) -> TestResult<&wire::Slot> {
    reply
        .slots
        .iter()
        .find(|slot| slot.slot == number)
        .ok_or_else(|| format!("slot {number} absent").into())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0x23).await?;
    if !client.selected_capabilities().contains(&wire::CAPABILITY_PREY_V1) {
        return Err("admission omitted Prey capability 140".into());
    }
    let (_, initial, _) = send(&mut client, wire::Request::Query, "query").await?;
    let before = initial
        .slots
        .iter()
        .find(|slot| slot.state == 1)
        .or_else(|| initial.slots.iter().find(|slot| slot.state != 0))
        .ok_or("no unlocked Prey slot")?
        .clone();
    let (_, reroll, current) = send(
        &mut client,
        wire::Request::ListReroll { slot: before.slot },
        "reroll",
    )
    .await?;
    let current = current.ok_or("reroll returned no current state")?;
    let after = slot(&current, before.slot)?.clone();
    let (_, refusal, refused_current) = send(
        &mut client,
        wire::Request::SelectAny {
            slot: before.slot,
            creature: NOT_A_CREATURE.into(),
        },
        "refused",
    )
    .await?;
    let refused_current = refused_current.ok_or("refusal returned no current state")?;
    if refusal.status == 0 || slot(&refused_current, before.slot)? != &after {
        return Err(format!("non-offered SelectAny was not refused: {refusal:?}").into());
    }
    let expected = format!(
        "slot={}\nstate_before={}\ncreatures_before={}\nfree_reroll_before={}\nreroll_status={}\nstate_after={}\ncreatures_after={}\nfree_reroll_after={}\nwildcards={}\nrefusal_status={}\n",
        before.slot,
        before.state,
        before.creatures.join("|"),
        before.free_reroll_at,
        reroll.status,
        after.state,
        after.creatures.join("|"),
        after.free_reroll_at,
        current.wildcard_balance,
        refusal.status
    );
    capture("p03-expected.txt", expected.as_bytes());
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_all scenario=P03.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P03.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P03.client status=FAIL evidence=timeout"),
    }
}
