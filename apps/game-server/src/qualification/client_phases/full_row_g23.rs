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
        verdict::indexed_timeout(Duration::from_secs(420), verdict::observe(journey(clients)))
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

async fn strict_step(
    client: &mut DevClientSession,
    direction: StepDirection,
    expected: TilePos,
) -> TestResult {
    // Pace the first input at walking speed, with no resend on refusal.
    client.service_liveness(Duration::from_millis(800)).await?;
    let reply = client.step(direction).await?;
    let actual = tile(client.world_spatial().actor_position)?;
    if reply.disposition != StepDisposition::Moved
        || reply.world_spatial_delta.is_none()
        || actual != expected
    {
        return Err(format!(
            "step=castle-walk direction={direction:?} expected={expected:?} actual={actual:?} reply={reply:?}"
        ).into());
    }
    client.take_events();
    verdict::Harness::global().progress("g23 castle walk");
    Ok(())
}

async fn strict_walk(
    client: &mut DevClientSession,
    world: &BundleWorld,
    goal: TilePos,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let path = route(world, from, |at| at == goal)
        .ok_or_else(|| format!("step=castle-walk no route from {from:?} to {goal:?}"))?;
    evidence(&format!(
        "G23_WALK from={from:?} to={goal:?} steps={}",
        path.len()
    ));
    for direction in path {
        let from = tile(client.world_spatial().actor_position)?;
        let expected =
            destination(world, from, direction).ok_or("castle route lost its landing")?;
        strict_step(client, direction, expected).await?;
    }
    Ok(())
}

async fn open_castle_door(
    client: &mut DevClientSession,
    world: &BundleWorld,
    door: TilePos,
    approach: TilePos,
    direction: StepDirection,
) -> TestResult {
    strict_walk(client, world, approach).await?;
    if !world.facts().doors().at(door) {
        return Err(format!("step=castle-door missing door at {door:?}").into());
    }
    let key = crate::map::view::placement_key(door, 1).ok_or("castle door placement key")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let reply = client.use_object(&placement, 0).await?;
    if reply.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
        return Err(format!(
            "step=castle-door at={door:?} disposition={:?}",
            reply.disposition
        )
        .into());
    }
    strict_step(client, direction, door).await
}

async fn promotion_line(
    client: &mut DevClientSession,
    npc: oteryn_dev_client::EntityRef,
    command: &str,
) -> TestResult<Vec<String>> {
    use oteryn_protocol_oteryn::npc_service as wire;
    let payload = wire::encode_npc_talk_intent(&wire::NpcTalkIntent {
        npc_actor: npc,
        text: command.into(),
    })
    .map_err(|e| format!("promotion {command} encode: {e:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_NPC_SERVICE_V1,
            wire::COMMAND_TYPE_NPC_TALK_INTENT,
            &payload,
        )
        .await?;
    let disposition = wire::decode_npc_intent_result(&reply.payload)
        .map_err(|e| format!("promotion {command} status={:?} decode={e:?}", reply.status))?;
    if reply.status != CommandStatus::Accepted || disposition != wire::NpcIntentDisposition::Ok {
        return Err(format!(
            "step=promotion command={command} status={:?} disposition={disposition:?}",
            reply.status
        )
        .into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1)
        .await?;
    let conversation = wire::decode_npc_conversation(&delta.payload)
        .map_err(|e| format!("promotion {command} conversation: {e:?}"))?;
    if delta.server_sequence <= reply.server_sequence
        || conversation.open.map(|(actor, _)| actor) != Some(npc)
    {
        return Err(format!("step=promotion command={command} unbound conversation").into());
    }
    Ok(conversation.lines)
}

async fn promote(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    // From the street, pass the gate into the west passage and climb to the King.
    let ground = |x, y| TilePos { x, y, floor: -7 };
    open_castle_door(
        client,
        world,
        ground(32310, 32188),
        ground(32311, 32188),
        StepDirection::West,
    )
    .await?;
    open_castle_door(
        client,
        world,
        ground(32308, 32183),
        ground(32308, 32184),
        StepDirection::North,
    )
    .await?;
    open_castle_door(
        client,
        world,
        ground(32308, 32180),
        ground(32308, 32181),
        StepDirection::North,
    )
    .await?;
    strict_walk(
        client,
        world,
        TilePos {
            x: 32311,
            y: 32175,
            floor: -6,
        },
    )
    .await?;
    let king = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|actor| {
                actor.kind == oteryn_dev_client::EntityKind::Npc
                    && actor.position
                        == (ActorPosition {
                            x: 32311,
                            y: 32172,
                            floor: 6,
                        })
            })
        })
        .map(|actor| actor.entity)
        .ok_or("step=promotion King Tibianus absent")?;
    let greeting = promotion_line(client, king, "hi").await?;
    if greeting.is_empty() {
        return Err("step=promotion-greeting observed=empty-conversation".into());
    }
    let quote = promotion_line(client, king, "promotion").await?;
    if quote != ["Do you want to be promoted for 20000 gold coins?"] {
        return Err(format!("step=promotion-quote observed={quote:?}").into());
    }
    let confirmation = promotion_line(client, king, "yes").await?;
    evidence(&format!("G23_PROMOTION confirmation={confirmation:?}"));
    if confirmation != ["You are now promoted."] {
        return Err(format!("step=promotion-confirmation observed={confirmation:?}").into());
    }
    Ok(())
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
    promote(&mut client, world).await?;
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
