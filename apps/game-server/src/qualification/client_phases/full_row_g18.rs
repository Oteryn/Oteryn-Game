//! G18 house podium preview (capability 150, command 739).
//! 1. Admit the node-boot character (no house is needed for a preview).
//! 2. Request a preview of a creature key authored in the content: the server must answer with
//!    non-zero look data and echo the requested direction.
//! 3. Request an unknown key: the server must answer with the explicit "unknown content" reply.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::house_podium::preview as wire;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => (
            "PASS",
            "podium-preview-artwork-and-unknown-key-validated".to_owned(),
        ),
        Ok(Err(error)) => ("FAIL", compact(&error.to_string())),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G18.full status={status} seconds={:.3} evidence={detail}",
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

const CAPABILITY: u32 = 150;
const CREATURE: &str = "oteryn:creature.rat";

async fn artwork(
    client: &mut DevClientSession,
    kind: u8,
    direction: u8,
    key: &str,
) -> TestResult<Option<wire::Artwork>> {
    let request = wire::Request {
        display_kind: kind,
        direction,
        key: key.to_owned(),
    };
    let payload = request
        .encode()
        .ok_or("step=preview observed=request-encode")?;
    let reply = accepted(client, CAPABILITY, wire::COMMAND_PREVIEW, &payload).await?;
    wire::Artwork::decode(&reply).ok_or_else(|| "step=preview observed=decode-failed".into())
}

async fn body(client: &mut DevClientSession, _clients: &SeamClients<'_>) -> TestResult {
    let Some(known) = artwork(client, 3, 2, CREATURE).await? else {
        return Err(format!(
            "step=preview observed=unknown-content-for-{CREATURE} needed=authored-creature-key"
        )
        .into());
    };
    if known.direction != 2 || known.looks == [0, 0] {
        return Err(format!("step=preview observed={known:?}").into());
    }
    if artwork(client, 3, 2, "oteryn:creature.no-such-creature")
        .await?
        .is_some()
    {
        return Err("step=unknown observed=artwork-for-unknown-key".into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0x7e).await?;
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

    #[test]
    fn out_of_range_direction_does_not_encode() {
        let request = |direction| wire::Request {
            display_kind: 3,
            direction,
            key: CREATURE.to_owned(),
        };
        assert!(request(3).encode().is_some());
        assert!(request(4).encode().is_none());
    }
}
