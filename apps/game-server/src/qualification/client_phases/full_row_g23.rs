//! G23 wheel presentation and gem presentation (capability 148, commands 6119 and 6121).
//! 1. Admit the node-boot character; read the current allocation and atelier projections before
//!    requesting wheel presentation for that build. The server must echo the request, name its
//!    catalogue and vocation and list the graph nodes; an ineligible character must say why.
//! 2. Request the gem presentation at the stored atelier revision: the modifier catalogue must be
//!    non-empty and carry the same catalogue identity as the wheel reply.
//!
//! The gem projection needs a stored atelier state, which only the first gem action creates.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::{atelier::v2 as atelier, gem_presentation as gem, wheel as allocation, wheel_presentation as wheel};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "wheel-and-gem-presentation-validated".to_owned()),
        Ok(Err(error)) => ("FAIL", compact(&error.to_string())),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G23.full status={status} seconds={:.3} evidence={detail}",
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


async fn finish(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

const CAPABILITY: u32 = wheel::CAPABILITY_WHEEL_PRESENTATION_V1;

fn consistent(eligible: bool, reason: &str, nodes: usize, catalogue: &str, vocation: &str) -> bool {
    nodes > 0 && !catalogue.is_empty() && !vocation.is_empty() && (eligible || !reason.is_empty())
}

fn presentation_request(
    wheel_revision: u64, atelier_revision: u64, points: [u16; 36], vessels: [Option<[u8; 16]>; 4],
) -> wheel::Request {
    wheel::Request { nonce: 1, wheel_revision, atelier_revision, points, vessels }
}

async fn checked_state(
    client: &mut DevClientSession,
) -> TestResult<(allocation::WheelAllocationReply, atelier::AtelierReply)> {
    let query = allocation::WheelAllocationRequest {
        action: 0, occurrence: [0; 16], expected_revision: 0,
        points: [0; 36], vessels: None,
    };
    let payload = allocation::encode_wheel_request(&query).map_err(|error| format!("step=allocation encode={error:?}"))?;
    let (status, payload) = send(client, allocation::CAPABILITY_WHEEL_ALLOCATION_V1,
        allocation::COMMAND_TYPE_WHEEL_ALLOCATION, &payload).await?;
    if status != CommandStatus::Accepted {
        return Err(format!("step=allocation observed={status:?}").into());
    }
    let allocation = allocation::decode_wheel_reply(&payload).map_err(|error| format!("step=allocation decode={error:?}"))?;
    let (status, payload) = send(client, atelier::CAPABILITY_GEM_ATELIER_V2,
        atelier::COMMAND_TYPE_ATELIER_V2_QUERY, &atelier::encode_query()).await?;
    if status != CommandStatus::Accepted {
        return Err(format!("step=atelier observed={status:?}").into());
    }
    let atelier = atelier::decode_reply(&payload).map_err(|error| format!("step=atelier decode={error:?}"))?;
    if allocation.refused || atelier.refused || allocation.revision != atelier.wheel_revision {
        return Err(format!("step=state observed=wheel:{:?},atelier-wheel-revision:{};needed=current-allocation-and-atelier", allocation, atelier.wheel_revision).into());
    }
    Ok((allocation, atelier))
}

async fn wheel_reply(client: &mut DevClientSession, request: wheel::Request) -> TestResult<wheel::Reply> {
    let payload = request.encode().ok_or("step=wheel observed=request-encode")?;
    let (status, reply) = send(client, CAPABILITY, wheel::COMMAND_TYPE_WHEEL_PRESENTATION, &payload).await?;
    if status != CommandStatus::Accepted {
        return Err(format!("step=wheel observed={status:?};needed=stored-wheel-and-atelier-revisions").into());
    }
    let reply = wheel::Reply::decode(&reply).ok_or("step=wheel observed=decode-failed")?;
    if reply.request != request {
        return Err("step=wheel observed=request-not-echoed".into());
    }
    Ok(reply)
}

async fn gem_reply(client: &mut DevClientSession, revision: u64) -> TestResult<gem::Reply> {
    let request = gem::Request { nonce: 2, revision };
    let payload = request.encode().ok_or("step=gem observed=request-encode")?;
    let (status, reply) = send(client, gem::CAPABILITY_GEM_PRESENTATION_V1, gem::COMMAND_TYPE_GEM_PRESENTATION, &payload).await?;
    if status != CommandStatus::Accepted {
        return Err(format!("step=gem observed={status:?};needed=character-atelier-state").into());
    }
    let reply = gem::Reply::decode(&reply).ok_or("step=gem observed=decode-failed")?;
    if reply.request != request {
        return Err("step=gem observed=request-not-echoed".into());
    }
    Ok(reply)
}

async fn body(client: &mut DevClientSession, _clients: &SeamClients<'_>) -> TestResult {
    let (allocation, atelier) = checked_state(client).await?;
    let mut vessels = [None; 4];
    for (cell, gem) in vessels.iter_mut().zip(atelier.vessels.iter().filter_map(|slot| slot.gem)) {
        *cell = Some(gem);
    }
    let request = presentation_request(allocation.revision, atelier.revision, allocation.points, vessels);
    let wheel = wheel_reply(client, request).await?;
    if !consistent(
        wheel.eligible,
        &wheel.reason,
        wheel.nodes.len(),
        &wheel.catalogue,
        &wheel.vocation,
    ) {
        return Err(format!(
            "step=wheel observed=eligible:{}-reason:{}-nodes:{}",
            wheel.eligible,
            wheel.reason,
            wheel.nodes.len()
        )
        .into());
    }
    let gem = gem_reply(client, atelier.revision).await?;
    if gem.modifiers.is_empty() || gem.catalogue != wheel.catalogue || gem.vocation.is_empty() {
        return Err(format!(
            "step=gem observed=modifiers:{}-catalogue:{}-wheel-catalogue:{}",
            gem.modifiers.len(),
            gem.catalogue,
            wheel.catalogue
        )
        .into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0x88).await?;
    let outcome = verdict::observe(body(&mut client, clients)).await;
    let closed = verdict::observe(finish(&mut client)).await;
    outcome?;
    closed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presentation_uses_allocated_points_and_both_current_revisions() {
        let mut points = [0; 36];
        points[7] = 4;
        let vessels = [Some([8; 16]), None, None, None];
        let request = presentation_request(27, 14, points, vessels);
        assert_eq!((request.wheel_revision, request.atelier_revision), (27, 14));
        assert_eq!(request.points, points);
        assert_eq!(request.vessels, vessels);
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

    #[test]
    fn ineligible_wheel_must_explain_itself() {
        assert!(consistent(true, "", 3, "c@1", "knight"));
        assert!(consistent(false, "level", 3, "c@1", "knight"));
        assert!(!consistent(false, "", 3, "c@1", "knight"));
        assert!(!consistent(true, "", 0, "c@1", "knight"));
    }
}
