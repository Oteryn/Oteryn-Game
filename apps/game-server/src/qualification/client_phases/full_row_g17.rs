//! G17 returner reward query and claim (capability 182, commands 6124 and 6125).
//! 1. Admit the node-boot character; query the returner state (6124).
//! 2. With no entitlement (state 0) a claim for an unknown identity must be refused and leave
//!    the inventory and the state unchanged; the row then fails naming the precondition,
//!    because an entitlement needs twelve months of inactivity evidence no player route makes.
//! 3. With an entitlement (state 1) claim its identity, require state 2 and a changed inventory,
//!    log out, freshly admit and require the claimed state to persist.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::daily_reward::{self as daily, returner};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(90), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "returner-reward-claimed-and-durable".to_owned()),
        Ok(Err(error)) => ("FAIL", compact(&error.to_string())),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G17.full status={status} seconds={:.3} evidence={detail}",
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

const CAPABILITY: u32 = daily::CAPABILITY_DAILY_REWARD_V2;
const BOGUS: [u8; 16] = [
    0x01, 0x93, 0x5f, 0x3b, 0x2c, 0x4d, 0x7a, 0x11, 0x8b, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
];

async fn state(client: &mut DevClientSession) -> TestResult<returner::Reply> {
    let payload = accepted(client, CAPABILITY, returner::QUERY, &[]).await?;
    Ok(returner::Reply::decode(&payload)?)
}

async fn body(client: &mut DevClientSession, _clients: &SeamClients<'_>) -> TestResult<u8> {
    let initial = state(client).await?;
    let inventory = client.inventory().cloned();
    match (initial.state, initial.id) {
        (1, Some(id)) => {
            accepted(client, CAPABILITY, returner::CLAIM, &id).await?;
            let after = state(client).await?;
            if after.state != 2 || client.inventory().cloned() == inventory {
                return Err(format!("step=claim observed=state:{}", after.state).into());
            }
            Ok(after.state)
        }
        (0, _) => {
            let (status, _) = send(client, CAPABILITY, returner::CLAIM, &BOGUS).await?;
            let after = state(client).await?;
            if status == CommandStatus::Accepted
                || after.state != 0
                || client.inventory().cloned() != inventory
            {
                return Err(format!(
                    "step=bogus-claim observed=status:{status:?}-state:{} needed=refusal-without-change",
                    after.state
                )
                .into());
            }
            Err("step=query observed=state-0-no-entitlement needed=returner-entitlement-after-12-month-inactivity".into())
        }
        (other, _) => Err(format!(
            "step=query observed=state-{other} needed=fresh-returner-entitlement"
        )
        .into()),
    }
}

async fn verify(
    client: &mut DevClientSession,
    _clients: &SeamClients<'_>,
    data: &u8,
) -> TestResult {
    let after = state(client).await?;
    if after.state != *data {
        return Err(format!("step=relog observed=state-{}", after.state).into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0x7c).await?;
    let outcome = body(&mut client, clients).await;
    let closed = finish(&mut client).await;
    let data = outcome?;
    closed?;
    let mut fresh = connect_world(clients, 0x7d).await?;
    let checked = verify(&mut fresh, clients, &data).await;
    let closed = finish(&mut fresh).await;
    checked?;
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

    #[test]
    fn bogus_identity_is_a_uuid_v7() {
        assert_eq!(BOGUS[6] >> 4, 7);
        assert_eq!(BOGUS[8] >> 6, 2);
        assert_eq!(returner::decode_claim(&BOGUS), Ok(BOGUS));
    }
}
