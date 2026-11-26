//! G23 checks Wheel and Gem presentations on a freshly promoted Premium Knight.
//! Admission initializes the starter gems. Compare their committed identities and
//! catalogue after logout and fresh admission.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::{
    atelier, gem_presentation as gem, wheel as allocation, wheel_presentation as wheel,
};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    character_prep::declare_seeded(
        "G23.full",
        "knight-level-51,premium,promotion-fee-gold",
        "qualify for Wheel and Atelier without leveling within the row budget",
    );
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(180), verdict::observe(journey(clients)))
            .await;
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

async fn presentations(client: &mut DevClientSession) -> TestResult<(wheel::Reply, gem::Reply)> {
    let query = allocation::WheelAllocationRequest {
        action: 0,
        occurrence: [0; 16],
        expected_revision: 0,
        points: [0; allocation::WHEEL_SLOTS],
        vessels: None,
    };
    let (status, bytes) = send(
        client,
        CAPABILITY,
        allocation::COMMAND_TYPE_WHEEL_ALLOCATION,
        &allocation::encode_wheel_request(&query)
            .map_err(|e| format!("wheel query encode: {e:?}"))?,
    )
    .await?;
    let alloc = allocation::decode_wheel_reply(&bytes)
        .map_err(|e| format!("wheel query status={status:?} decode={e:?}"))?;
    if status != CommandStatus::Accepted
        || alloc.refused
        || alloc.points != query.points
        || alloc.vessels.is_some_and(|vessels| vessels != [None; 4])
    {
        return Err(format!("step=wheel-query observed={status:?}/{alloc:?}").into());
    }

    let (status, bytes) = send(
        client,
        CAPABILITY,
        atelier::COMMAND_TYPE_ATELIER_QUERY,
        &atelier::encode_query(),
    )
    .await?;
    let state = atelier::decode_reply(&bytes)
        .map_err(|e| format!("atelier query status={status:?} decode={e:?}"))?;
    if status != CommandStatus::Accepted || state.refused || state.gems.len() != 8 {
        return Err(format!("step=atelier-query observed={status:?}/{state:?}").into());
    }

    let request = wheel::Request {
        nonce: 1,
        wheel_revision: alloc.revision,
        atelier_revision: state.revision,
        points: alloc.points,
        vessels: alloc.vessels.unwrap_or([None; 4]),
    };
    let (status, bytes) = send(
        client,
        CAPABILITY,
        wheel::COMMAND_TYPE_WHEEL_PRESENTATION,
        &request.encode().ok_or("wheel presentation encode")?,
    )
    .await?;
    if status != CommandStatus::Accepted {
        return Err(format!("step=wheel observed={status:?}").into());
    }
    let wheel = wheel::Reply::decode(&bytes).ok_or("wheel presentation decode")?;
    if wheel.request != request
        || !consistent(
            wheel.eligible,
            &wheel.reason,
            wheel.nodes.len(),
            &wheel.catalogue,
            &wheel.vocation,
        )
        || !wheel.eligible
        || wheel.available != 1
        || wheel.vocation != "knight"
    {
        return Err(format!("step=wheel observed={wheel:?}").into());
    }

    let request = gem::Request {
        nonce: 2,
        revision: state.revision,
    };
    let (status, bytes) = send(
        client,
        gem::CAPABILITY_GEM_PRESENTATION_V1,
        gem::COMMAND_TYPE_GEM_PRESENTATION,
        &request.encode().ok_or("gem presentation encode")?,
    )
    .await?;
    if status != CommandStatus::Accepted {
        return Err(format!("step=gem observed={status:?} revision={}", state.revision).into());
    }
    let gem = gem::Reply::decode(&bytes).ok_or("gem presentation decode")?;
    let mut ids: Vec<_> = state.gems.iter().map(|gem| gem.id).collect();
    ids.sort_unstable();
    if gem.request != request
        || gem.modifiers.len() != 69
        || gem.catalogue != wheel.catalogue
        || gem.vocation != wheel.vocation
        || gem.qualified_gems != ids
    {
        return Err(format!(
            "step=gem observed=modifiers:{}-catalogue:{}-wheel-catalogue:{}-qualified:{}",
            gem.modifiers.len(),
            gem.catalogue,
            wheel.catalogue,
            gem.qualified_gems.len()
        )
        .into());
    }
    Ok((wheel, gem))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_Q_G23_ACCOUNT").await?;
    let clients = &dedicated.clients(clients);
    let mut client = connect_character(clients, 0x88, 0).await?;
    let original = client.join_snapshot().game_session_id;
    request_fixture_control(
        uuid_text(original.as_bytes()),
        "--profile",
        "g23-knight".into(),
    )
    .await?;
    character_prep::grant_item(&client, 3043, 2).await?;
    character_prep::grant_premium(&client).await?;
    finish(&mut client).await?;
    drop(client);

    let mut client = connect_character(clients, 0x88, 0).await?;
    let promoted_session = client.join_snapshot().game_session_id;
    if original == promoted_session {
        return Err("fixture relog reused session".into());
    }
    let world = full_e2ef7::qualification_world(clients)?;
    let bundle = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bundle, |_, _| Ok(()))?;
    let king = full_row_p20::approach_npc(
        &mut client,
        world,
        &metadata.npcs,
        "oteryn:npc.king_tibianus",
    )
    .await?;
    if full_row_p20::talk(&mut client, king, "hi")
        .await?
        .is_empty()
        || full_row_p20::talk(&mut client, king, "promotion").await?
            != ["Do you want to be promoted for 20000 gold coins?"]
        || full_row_p20::talk(&mut client, king, "yes").await? != ["You are now promoted."]
    {
        return Err("step=promotion observed=dialogue-refused".into());
    }
    finish(&mut client).await?;
    drop(client);

    let mut client = connect_character(clients, 0x88, 0).await?;
    let initial_session = client.join_snapshot().game_session_id;
    if promoted_session == initial_session {
        return Err("promotion relog reused session".into());
    }
    let before = verdict::observe(presentations(&mut client)).await;
    let close = verdict::observe(finish(&mut client)).await;
    let before = before?;
    close?;
    drop(client);

    let mut client = connect_character(clients, 0x88, 0).await?;
    let fresh_session = client.join_snapshot().game_session_id;
    let after = verdict::observe(presentations(&mut client)).await;
    let close = verdict::observe(finish(&mut client)).await;
    if after? != before || fresh_session == initial_session {
        return Err("step=relog observed=presentation-or-session-mismatch".into());
    }
    close?;
    evidence("G23_CHECK status=PASS points=1 starter_gems=8 modifiers=69 durable_after_relog=1");
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

    #[test]
    fn ineligible_wheel_must_explain_itself() {
        assert!(consistent(true, "", 3, "c@1", "knight"));
        assert!(consistent(false, "level", 3, "c@1", "knight"));
        assert!(!consistent(false, "", 3, "c@1", "knight"));
        assert!(!consistent(true, "", 0, "c@1", "knight"));
    }
}
