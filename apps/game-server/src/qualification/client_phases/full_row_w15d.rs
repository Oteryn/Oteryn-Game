//! W15d — per-quest executable population, exercised through the production callback owner.
//!
//! Concrete player scenario (The Way of the Monk, Shrine of Guidance):
//! 1. Log in with the node-boot character, walk from its saved position to an adjacent reachable
//!    tile at (32454, 32292, 7), and identify the authored shrine from the served bundle.
//! 2. Send native USE for that shrine. Expect an accepted/committed reply plus quest-log state
//!    advancement; the callback requires level 6, questline 3, and guidance 0 and sets guidance 1.
//! 3. Send USE again with the server's current object revision. Expect a rejected reply and no
//!    quest-log advancement because this one-way per-quest callback is no longer executable.
//! 4. Log out, obtain a fresh Platform admission, query the quest log, and require the committed
//!    quest entry and its completion flag to equal the post-USE durable state.
use super::*;
use crate::map::view::MapFacts;
use oteryn_dev_client::client_runtime::quest_log as journal;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

pub(super) const SHRINE: TilePos = TilePos {
    x: 32454,
    y: 32292,
    floor: -7,
};
pub(super) const APPEARANCE: u16 = 50242;

struct Row {
    started: std::time::Instant,
    step: &'static str,
}

impl Row {
    fn fail(self, observed: impl std::fmt::Display) {
        let observed = observed.to_string();
        evidence(&format!("W15D_GAP step={} observed={observed}", self.step));
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=W15d.full status=FAIL seconds={:.3} evidence=step-{}:{}",
            self.started.elapsed().as_secs_f64(),
            self.step,
            observed.replace(char::is_whitespace, "_")
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row {
        started: std::time::Instant::now(),
        step: "admission",
    };
    let result = verdict::indexed_timeout(
        Duration::from_secs(900),
        verdict::observe(journey(&mut row, clients)),
    )
    .await;
    match result {
        Ok(Ok(())) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=W15d.full status=PASS seconds={:.3}",
            row.started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => row.fail(error),
        Err(_) => row.fail("timeout waiting for server reply/delta"),
    }
}

async fn quest_list(
    client: &mut oteryn_dev_client::DevClientSession,
) -> TestResult<Vec<journal::QuestListEntry>> {
    client.service_liveness(Duration::from_millis(1050)).await?;
    let snapshot = client
        .session_mut()
        .gated_snapshot(journal::STATE_DOMAIN_QUEST_LOG)
        .ok_or("missing quest-log snapshot")?
        .clone();
    let state = journal::QuestState::from_snapshot(client.selected_capabilities(), Some(&snapshot))
        .map_err(|error| format!("quest snapshot: {error:?}"))?;
    let command = state
        .command(&journal::QuestLogQuery::List)
        .map_err(|error| format!("quest list command: {error:?}"))?;
    let result = journal::send(client.session_mut(), &command).await?;
    if result.status != CommandStatus::Accepted {
        return Err(format!("quest list reply status={:?}", result.status).into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(journal::STATE_DOMAIN_QUEST_LOG, 1)
        .await?;
    let log = journal::decode_quest_log(&delta.payload)
        .map_err(|error| format!("quest list delta: {error:?}"))?;
    let journal::QuestLogView::List(entries) = log.view.ok_or("quest list delta has no view")?
    else {
        return Err("quest list delta is not a list".into());
    };
    Ok(entries)
}

pub(super) fn object_target(
    world: &BundleWorld,
    at: TilePos,
    appearance: u16,
) -> TestResult<WorldObjectTarget> {
    let tile = world
        .base()
        .tile(at.x, at.y, at.floor)
        .ok_or("authored object tile absent")?;
    let ordinal = tile
        .ids()
        .iter()
        .zip(tile.depths())
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
        .find_map(|(ordinal, (id, _))| {
            let ordinal = u8::try_from(ordinal).ok()?;
            world
                .facts()
                .base_entry(at, ordinal, *id)
                .filter(|facts| facts.appearance_id == appearance)
                .map(|_| ordinal)
        })
        .ok_or("authored object appearance absent")?;
    let key = crate::map::view::placement_key(at, ordinal).ok_or("object placement key")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: world.facts().object_revision(key),
    })
}

pub(super) async fn prepare(
    row: &str,
    scope: &SeamClients<'_>,
    tag: u8,
) -> TestResult<oteryn_dev_client::DevClientSession> {
    use oteryn_protocol_oteryn::cyclopedia_character as stats;
    character_prep::declare_seeded(row, "monk-level-6", "Guidance Shrine level gate");
    let mut client = connect_character(scope, tag, 0).await?;
    let session = client.join_snapshot().game_session_id;
    request_fixture_control(uuid_text(session.as_bytes()), "--profile", "q_w15d".into()).await?;
    logout_once(&mut client).await?;
    drop(client);
    let mut client = connect_character(scope, tag, 0).await?;
    if client.join_snapshot().game_session_id == session {
        return Err("seeded stats require a fresh admission".into());
    }
    let reply = client
        .session_mut()
        .gated_command(
            stats::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            stats::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &[stats::CharacterTab::GeneralStats as u8, 0, 0, 1],
        )
        .await?;
    let vocation = general_stats::validate(&reply.payload, general_stats::selected(&client))?;
    let experience = u64::from_le_bytes(reply.payload[1..9].try_into()?);
    let level = u32::from_le_bytes(reply.payload[9..13].try_into()?);
    if reply.status != CommandStatus::Accepted
        || reply.payload[0] != stats::CharacterTab::GeneralStats as u8
        || vocation != stats::vocation_id("monk")
        || (experience, level) != (1500, 6)
    {
        return Err(format!(
            "seeded stats status={:?} vocation={vocation:?} xp={experience} level={level}",
            reply.status
        )
        .into());
    }
    let vitals = client.actor_vitals().ok_or("seeded actor vitals absent")?;
    if (vitals.max_health, vitals.max_mana) != (175, 80) {
        return Err(format!("seeded level-6 Monk vitals observed={vitals:?}").into());
    }
    evidence(&format!(
        "W15D_SEEDED_STATS row={row} vocation=monk xp={experience} level={level} max_health={} max_mana={}",
        vitals.max_health, vitals.max_mana
    ));
    Ok(client)
}

pub(super) async fn logout_once(client: &mut oteryn_dev_client::DevClientSession) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    loop {
        let state = client.session_mut().combat_state().ok_or("combat state absent before logout")?;
        if !state.in_fight {
            break;
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(format!("combat lock still active before first logout: {state:?}").into());
        }
        client.service_liveness(remaining.min(Duration::from_millis(250))).await?;
        client.take_events();
    }
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("first logout refused: {reply:?}").into());
    }
    Ok(())
}

async fn walk_to(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    goal: TilePos,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let path = find_route(
        from,
        |at| at.floor == goal.floor && at.x.abs_diff(goal.x).max(at.y.abs_diff(goal.y)) == 1,
        |at, direction| {
            let next = destination(world, at, direction)?;
            (next.x.abs_diff(from.x) <= 180
                && next.y.abs_diff(from.y) <= 180
                && next.floor.abs_diff(from.floor) <= 2)
                .then_some(next)
        },
    )
    .map_err(|visited| format!("no playable route from {from:?} to {goal:?}, visited={visited}"))?;
    evidence(&format!(
        "W15D_ROUTE from={from:?} goal={goal:?} steps={}",
        path.len()
    ));
    for direction in path {
        let from = tile(client.world_spatial().actor_position)?;
        let expected = destination(world, from, direction).ok_or("planned step landing absent")?;
        let mut moved = client.step(direction).await?;
        if moved.disposition == StepDisposition::Blocked {
            if tile(client.world_spatial().actor_position)? != from {
                return Err(format!("blocked STEP changed position: {moved:?}").into());
            }
            let occupant = client.world_entities().and_then(|entities| {
                entities.iter().find(|entry| {
                    entry.kind == oteryn_dev_client::EntityKind::Creature
                        && tile(entry.position).ok() == Some(expected)
                })
            }).ok_or_else(|| format!("BLOCKER-SERVER unoccupied blocked STEP from={from:?} to={expected:?} result={moved:?}"))?.entity;
            evidence(&format!("W15D_CREATURE_BLOCK id={occupant:?} tile={expected:?} first={moved:?}"));
            let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
            loop {
                let observed = client.world_entities().and_then(|entities| {
                    entities.iter().find(|entry| entry.entity == occupant)
                }).and_then(|entry| tile(entry.position).ok());
                if let Some(observed) = observed
                    && observed != expected
                {
                    evidence(&format!("W15D_CREATURE_MOVED id={occupant:?} tile={observed:?}"));
                    break;
                }
                let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                if remaining.is_zero() {
                    return Err(format!("creature {occupant:?} did not visibly leave {expected:?}").into());
                }
                client.service_liveness(remaining.min(Duration::from_millis(250))).await?;
            }
            moved = client.step(direction).await?;
        }
        let observed = tile(client.world_spatial().actor_position)?;
        if moved.disposition != StepDisposition::Moved
            || moved.world_spatial_delta.is_none()
            || observed != expected
        {
            return Err(format!("first STEP from={from:?} direction={direction:?} expected={expected:?} observed={observed:?} result={moved:?}").into());
        }
        verdict::Harness::global().progress("walk");
        let duration = client
            .world_entities()
            .and_then(|entities| {
                entities
                    .iter()
                    .find(|entry| &entry.entity.identity == entities.own_identity())
            })
            .and_then(|entry| entry.motion)
            .ok_or("moved actor motion duration missing")?
            .step_duration_ms;
        client
            .service_liveness(Duration::from_millis(u64::from(duration)))
            .await?;
        client.take_events();
    }
    Ok(())
}

async fn use_portal(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    at: TilePos,
    landing: TilePos,
    appearance: u16,
) -> TestResult {
    walk_to(client, world, at).await?;
    let source = tile(client.world_spatial().actor_position)?;
    let target = object_target(world, at, appearance)?;
    let result = client
        .session_mut()
        .use_object(&target.placement, target.expected_revision)
        .await?;
    if result.status != CommandStatus::Accepted || result.disposition != UseDisposition::Committed {
        return Err(format!("Blue Valley portal USE at={at:?} result={result:?}").into());
    }
    // USE returns at its command result. The normal visibility stream carries
    // the committed relocation separately on the same connection.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        let observed = tile(client.world_spatial().actor_position)?;
        if observed == landing {
            evidence(&format!("W15D_PORTAL at={at:?} landing={landing:?}"));
            return Ok(());
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if observed != source || remaining.is_zero() {
            return Err(format!(
                "Blue Valley portal at={at:?} expected={landing:?} observed={observed:?}"
            )
            .into());
        }
        client
            .service_liveness(remaining.min(Duration::from_millis(250)))
            .await?;
        client.take_events();
    }
}

async fn start_pilgrimage(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
) -> TestResult {
    use oteryn_protocol_oteryn::npc_service as npc;
    let pema = TilePos {
        x: 33603,
        y: 31468,
        floor: -7,
    };
    walk_to(client, world, pema).await?;
    let actor = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == oteryn_dev_client::EntityKind::Npc
                    && tile(entry.position).ok() == Some(pema)
            })
        })
        .ok_or("authored Enpa-Deia Pema NPC unavailable")?
        .entity;
    for text in ["hi", "pilgrimage"] {
        let payload = npc::encode_npc_talk_intent(&npc::NpcTalkIntent {
            npc_actor: actor,
            text: text.into(),
        })
        .map_err(|error| format!("Pema intent: {error:?}"))?;
        let reply = client
            .session_mut()
            .gated_command(
                npc::CAPABILITY_NPC_SERVICE_V1,
                npc::COMMAND_TYPE_NPC_TALK_INTENT,
                &payload,
            )
            .await?;
        let disposition = npc::decode_npc_intent_result(&reply.payload)
            .map_err(|error| format!("Pema result: {error:?}"))?;
        if reply.status != CommandStatus::Accepted || disposition != npc::NpcIntentDisposition::Ok {
            return Err(format!(
                "Pema text={text} status={:?} disposition={disposition:?}",
                reply.status
            )
            .into());
        }
        let delta = client
            .session_mut()
            .read_gated_delta(npc::STATE_DOMAIN_NPC_CONVERSATION, 1)
            .await?;
        let conversation = npc::decode_npc_conversation(&delta.payload)
            .map_err(|error| format!("Pema conversation: {error:?}"))?;
        if conversation.open.map(|(npc, _)| npc) != Some(actor)
            || (text == "pilgrimage"
                && !conversation
                    .lines
                    .iter()
                    .any(|line| line.contains("You may now visit the shrines")))
        {
            return Err(format!("Pema text={text} observed={conversation:?}").into());
        }
        client.service_liveness(Duration::from_millis(1050)).await?;
    }
    Ok(())
}

pub(super) async fn approach(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
) -> TestResult {
    // Global's pilgrimage starts at the Enchanted Guarded Crystal Sphere in the Guild.
    use_portal(
        client,
        world,
        TilePos {
            x: 32203,
            y: 32303,
            floor: -6,
        },
        TilePos {
            x: 33614,
            y: 31494,
            floor: -7,
        },
        50310,
    )
    .await?;
    start_pilgrimage(client, world).await?;
    use_portal(
        client,
        world,
        TilePos {
            x: 33605,
            y: 31468,
            floor: -7,
        },
        TilePos {
            x: 32456,
            y: 32291,
            floor: -7,
        },
        49948,
    )
    .await?;
    walk_to(client, world, SHRINE).await
}

pub(super) fn assert_guidance_reward(
    client: &oteryn_dev_client::DevClientSession,
    expected: u64,
) -> TestResult {
    let items = provisioning::item_index()?;
    let boots = provisioning::item_reference(&items, 50267)?;
    let inventory = client.inventory().ok_or("Guidance inventory view absent")?;
    let quantity = inventory.entries.iter()
        .chain(inventory.equipment.iter().map(|entry| &entry.item))
        .filter(|entry| entry.item_definition_ref == boots)
        .map(|entry| u64::from(entry.count.get()))
        .sum::<u64>();
    if quantity != expected {
        return Err(format!("Boots of Enlightenment expected={expected} observed={quantity}").into());
    }
    Ok(())
}

pub(super) fn shrine_revision(base: u64, published: Option<u64>) -> TestResult<u64> {
    published.filter(|revision| *revision > base).ok_or_else(|| format!("repeat-use needs a newer published shrine revision, base={base} observed={published:?}").into())
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let character =
        character_prep::dedicated_character(clients, "NODE_BOOT_Q_W15D_ACCOUNT").await?;
    let scope = character.clients(clients);
    let mut client = prepare("W15d.full", &scope, 0xd4).await?;
    row.step = "walk-to-guidance-shrine";
    approach(&mut client, world).await?;

    row.step = "first-use-commit";
    let target = object_target(world, SHRINE, APPEARANCE)?;
    let before = quest_list(&mut client).await?;
    assert_guidance_reward(&client, 0)?;
    let first = client
        .session_mut()
        .use_object(&target.placement, target.expected_revision)
        .await?;
    if first.status != CommandStatus::Accepted || first.disposition != UseDisposition::Committed {
        return Err(format!(
            "USE reply status={:?} disposition={:?}; server needs a player-reachable prerequisite path for questline=3,guidance=0,level>=6",
            first.status, first.disposition
        )
        .into());
    }
    let after = quest_list(&mut client).await?;
    assert_guidance_reward(&client, 1)?;
    if after == before {
        return Err("committed USE produced no quest-log delta".into());
    }

    row.step = "repeat-use-refusal";
    let observed_revision = client
        .world_object_overlay()
        .iter()
        .find(|entry| entry.placement == target.placement)
        .map(|entry| entry.revision);
    let revision = shrine_revision(target.expected_revision, observed_revision)?;
    let second = client
        .session_mut()
        .use_object(&target.placement, revision)
        .await?;
    if second.status != CommandStatus::Rejected || second.disposition != UseDisposition::Rejected {
        return Err(format!(
            "repeat USE status={:?} disposition={:?}",
            second.status, second.disposition
        )
        .into());
    }
    if quest_list(&mut client).await? != after {
        return Err("refused repeat USE changed quest-log state".into());
    }

    assert_guidance_reward(&client, 1)?;
    row.step = "logout-fresh-admission";
    let session = client.join_snapshot().game_session_id;
    logout_once(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(&scope, 0xd5, 0).await?;
    if fresh.join_snapshot().game_session_id == session || quest_list(&mut fresh).await? != after {
        return Err("fresh admission lost committed quest-log state".into());
    }
    assert_guidance_reward(&fresh, 1)?;
    logout_once(&mut fresh).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journey_regression_replays_with_the_published_shrine_revision() {
        assert!(shrine_revision(2, None).is_err());
        assert!(shrine_revision(2, Some(2)).is_err());
        assert_eq!(shrine_revision(2, Some(3)).ok(), Some(3));
    }
}
