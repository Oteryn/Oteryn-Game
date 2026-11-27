//! W15c Killing in the Name Of (Grizzly Adams), derived from the NPC conversation owner
//! (`gameplay_transport/npc_command/grizzly.rs`) and the quest rules (`quest/grizzly_adams/rules.rs`).
//!
//! 1. Use the Thais dock ladder, take Captain Bluebear to Port Hope and walk to the authored Grizzly Adams placement.
//!    Say `hi`, then `join`; assert the server's
//!    reply lines (the durable join) and `points`.
//! 2. Ask for the lowest-level task by name; assert the exact reply: a start offer (confirmed with
//!    `yes`, then `status` must list it), or the exact level/premium refusal.
//! 3. Log out, obtain a fresh admission, walk back and say `hi`; the durable join must answer
//!    `Welcome back`, and an accepted task must still be listed by `status`.
//! 4. PASS only when a task was started and survived the fresh admission; a refusal reports FAIL
//!    naming the precondition. The fixture supplies only the eligible build, Premium and fare.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::npc_service as wire;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

const DOCK_LADDER: TilePos = TilePos {
    x: 32321,
    y: 32220,
    floor: -7,
};
const DOCK_LANDING: TilePos = TilePos {
    x: 32321,
    y: 32221,
    floor: -6,
};
const CAPTAIN: &str = "oteryn:npc.captain_bluebear";
const PORT_HOPE: TilePos = TilePos {
    x: 32527,
    y: 32784,
    floor: -6,
};
const FARE: u64 = 160;
const PROFILE: &str = "w15c-sorcerer";
const LEVEL: u32 = 8;
const NPC: &str = "oteryn:npc.grizzly_adams";
const REFUSALS: [&str; 2] = [
    "This task requires Premium.",
    "That task is outside your level range.",
];

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    character_prep::declare_seeded(
        "W15c.full",
        "level-8-sorcerer,premium,port-hope-fare",
        "Drive Grizzly's eligible task conversation within the row budget",
    );
    let started = std::time::Instant::now();
    // The first 89 paced steps took about 280 seconds; the authored route adds 208.
    let result = verdict::indexed_timeout(
        Duration::from_secs(1080),
        verdict::observe(journey(clients)),
    )
    .await;
    let (status, detail) = match result {
        Ok(Ok(Outcome::Started(task))) => (
            "PASS",
            format!(
                "task-started-and-durable-after-fresh-admission:{}",
                compact(&task)
            ),
        ),
        Ok(Ok(Outcome::Refused(line))) => (
            "FAIL",
            format!(
                "needed:character-eligible-for-a-task(level-and-premium);join-durable;refusal={}",
                compact(&line)
            ),
        ),
        Ok(Err(error)) => ("FAIL", compact(&error.to_string())),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=W15c.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

enum Outcome {
    Started(String),
    Refused(String),
}

fn compact(value: &str) -> String {
    let token: String = value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(300)
        .collect();
    if token.is_empty() {
        "unknown".to_owned()
    } else {
        token
    }
}

/// How the server answered a task request.
#[derive(Debug, PartialEq, Eq)]
enum TaskReply {
    Offer,
    Refusal,
    Other,
}

fn classify(line: &str) -> TaskReply {
    if line.starts_with("Hunt ") {
        TaskReply::Offer
    } else if REFUSALS.contains(&line) {
        TaskReply::Refusal
    } else {
        TaskReply::Other
    }
}

/// The catalogue's lowest-level task name; the server matches the typed text case-insensitively.
fn first_task() -> TestResult<(String, u64)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalogue: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/quests/killing-in-the-name-of.json"),
    )?)?;
    let (_, name, required) = catalogue["tasks"]
        .as_array()
        .ok_or("step=catalogue observed=no-tasks")?
        .iter()
        .filter_map(|task| {
            Some((
                task["minimum_level"].as_u64()?,
                task["name"].as_str()?,
                task["required"].as_u64(),
            ))
        })
        .min_by_key(|&(level, name, _)| (level, name))
        .ok_or("step=catalogue observed=empty")?;
    Ok((
        name.to_owned(),
        required.ok_or("step=catalogue observed=missing-task-target")?,
    ))
}

async fn walk_to(
    client: &mut DevClientSession,
    world: &BundleWorld,
    key: &str,
    goal: impl Fn(TilePos) -> bool,
) -> TestResult {
    let here = tile(client.world_spatial().actor_position)?;
    let path = find_route(here, goal, |at, direction| {
        destination(world, at, direction)
    })
    .map_err(|visited| {
        format!("step=reach target={key} from={here:?} observed=no-walk-route visited={visited}")
    })?;
    evidence(&format!(
        "row=W15c.full step=planned-walk target={key} steps={}",
        path.len()
    ));
    for direction in path {
        let before = tile(client.world_spatial().actor_position)?;
        let landing = destination(world, before, direction)
            .ok_or("step=walk observed=missing-planned-landing")?;
        let outcome = client.step(direction).await?;
        if outcome.status != CommandStatus::Accepted
            || outcome.disposition != StepDisposition::Moved
            || outcome.world_spatial_delta.is_none()
            || tile(client.world_spatial().actor_position)? != landing
        {
            return Err(format!("step=walk npc={key} from={before:?} direction={direction:?} expected={landing:?} observed={outcome:?}").into());
        }
        // The session already applied these deltas to its view during the exchange.
        client.take_events();
        verdict::Harness::global().progress("walk");
        tokio::time::sleep(Duration::from_millis(u64::from(outcome.step_wait_ms))).await;
    }
    Ok(())
}

async fn reach(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    key: &str,
) -> TestResult<EntityRef> {
    let npc = npcs
        .npcs
        .iter()
        .find(|npc| npc.key == key)
        .ok_or_else(|| format!("step=reach npc={key} observed=no-authored-placement"))?;
    let [placement] = npc.placements.as_slice() else {
        return Err(format!("step=reach npc={key} expected=unique-authored-placement").into());
    };
    walk_to(client, world, key, |at| {
        at.floor == placement.floor
            && at.x.abs_diff(placement.x).max(at.y.abs_diff(placement.y)) <= 3
    })
    .await?;
    let display = if key == CAPTAIN {
        "Captain Bluebear"
    } else {
        "Grizzly Adams"
    };
    client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entity| {
                entity.kind == oteryn_dev_client::EntityKind::Npc
                    && entity
                        .display_name
                        .as_ref()
                        .and_then(|name| name.as_str().ok())
                        == Some(display)
                    && npc.placements.iter().any(|p| {
                        entity.position.x == i32::from(p.x)
                            && entity.position.y == i32::from(p.y)
                            && entity.position.floor == -i16::from(p.floor)
                    })
            })
        })
        .map(|entry| entry.entity)
        .ok_or_else(|| format!("step=reach npc={key} observed=not-visible-after-walk").into())
}

/// One real talk intent; returns the reply lines the server addressed to this character.
async fn say(
    client: &mut DevClientSession,
    actor: EntityRef,
    text: &str,
) -> TestResult<Vec<String>> {
    say_with_speaker(client, actor, text, None).await
}

async fn say_with_speaker(
    client: &mut DevClientSession,
    actor: EntityRef,
    text: &str,
    speaker: Option<(&str, &str)>,
) -> TestResult<Vec<String>> {
    use oteryn_protocol_oteryn::chat::{ChatLine, ChatSpeechMode};

    client.take_events();
    let payload = wire::encode_npc_talk_intent(&wire::NpcTalkIntent {
        npc_actor: actor,
        text: text.into(),
    })
    .map_err(|error| format!("step=say-{text} encode={error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_NPC_SERVICE_V1,
            wire::COMMAND_TYPE_NPC_TALK_INTENT,
            &payload,
        )
        .await?;
    let disposition = wire::decode_npc_intent_result(&reply.payload)
        .map_err(|error| format!("step=say-{text} decode={error:?}"))?;
    if reply.status != CommandStatus::Accepted || disposition != wire::NpcIntentDisposition::Ok {
        return Err(format!(
            "step=say-{text} status={:?} disposition={disposition:?}",
            reply.status
        )
        .into());
    }
    let mut lines = Vec::new();
    let mut heard_default = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        for event in client.take_events() {
            match event {
                SessionEvent::NpcConversation(delta)
                    if delta.server_sequence > reply.server_sequence =>
                {
                    let named = delta.value.open.map(|(npc, _)| npc);
                    if named.is_some_and(|npc| npc != actor) {
                        return Err(format!(
                            "step=say-{text} observed=conversation-for-another-npc"
                        )
                        .into());
                    }
                    if speaker.is_some() && !delta.value.lines.is_empty() && named != Some(actor) {
                        return Err(
                            format!("step=say-{text} observed=reply-without-npc-speaker").into(),
                        );
                    }
                    lines.extend(delta.value.lines);
                }
                SessionEvent::ChatLine(delta) if delta.server_sequence > reply.server_sequence => {
                    if let Some((name, expected_text)) = speaker
                        && let ChatLine::Local {
                            speaker,
                            speaker_name,
                            mode: ChatSpeechMode::Say,
                            text,
                            ..
                        } = delta.value
                        && speaker_name == name
                        && text == expected_text
                        && speaker.identity == actor.identity
                        && speaker.generation.get() == actor.generation
                    {
                        heard_default = true;
                    }
                }
                _ => {}
            }
        }
        if (!lines.is_empty() && (speaker.is_none() || heard_default))
            || tokio::time::Instant::now() >= deadline
        {
            break;
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    if lines.is_empty() {
        return Err(format!("step=say-{text} observed=no-reply-line").into());
    }
    if speaker.is_some() && !heard_default {
        return Err(format!("step=say-{text} observed=no-attributed-default-reply").into());
    }
    Ok(lines)
}

fn expect(lines: &[String], needle: &str, step: &str) -> TestResult {
    if lines == [needle] {
        Ok(())
    } else {
        Err(format!(
            "step={step} observed={} expected={needle}",
            compact(&lines.join("|"))
        )
        .into())
    }
}

async fn expect_task_status(
    client: &mut DevClientSession,
    actor: EntityRef,
    expected: &str,
    step: &str,
) -> TestResult {
    expect(&say(client, actor, "status").await?, expected, step)
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.status != CommandStatus::Accepted
        || reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err(format!("step=logout observed={reply:?}").into());
    }
    Ok(())
}

async fn assert_build(client: &mut DevClientSession) -> TestResult {
    let level = full_e2ef7::player_level(client).await?;
    let (max_health, max_mana, _) = oteryn_gameplay_characters::vitals::maxima(
        "sorcerer",
        LEVEL,
        oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
        oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
    )
    .ok_or("step=build observed=missing-sorcerer-vitals")?;
    if level != LEVEL
        || !client
            .actor_vitals()
            .is_some_and(|vitals| vitals.max_health == max_health && vitals.max_mana == max_mana)
    {
        return Err(format!("step=build expected=level-{LEVEL}-hp-{max_health}-mp-{max_mana} observed=level-{level}-{:?}", client.actor_vitals()).into());
    }
    evidence(&format!(
        "row=W15c.full step=build level={level} max_health={max_health} max_mana={max_mana}"
    ));
    Ok(())
}

fn carried_gold(
    client: &DevClientSession,
    coins: &[(std::num::NonZeroU32, u64)],
) -> TestResult<u64> {
    let inventory = client
        .inventory()
        .ok_or("step=fare observed=no-inventory")?;
    Ok(inventory
        .entries
        .iter()
        .map(|entry| {
            coins
                .iter()
                .find(|(reference, _)| *reference == entry.item_definition_ref)
                .map_or(0, |(_, value)| u64::from(entry.count.get()) * value)
        })
        .sum())
}

async fn climb_dock_ladder(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    use crate::map::view::MapFacts;
    walk_to(client, world, "thais-dock-ladder", |at| {
        at.floor == DOCK_LADDER.floor
            && at
                .x
                .abs_diff(DOCK_LADDER.x)
                .max(at.y.abs_diff(DOCK_LADDER.y))
                <= 1
    })
    .await?;
    let base = world
        .base()
        .tile(DOCK_LADDER.x, DOCK_LADDER.y, DOCK_LADDER.floor)
        .ok_or("step=dock-ladder observed=no-authored-tile")?;
    let [ordinal] = base
        .ids()
        .iter()
        .zip(base.depths())
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
        .filter_map(|(ordinal, (id, _))| {
            let ordinal = u8::try_from(ordinal).ok()?;
            world
                .facts()
                .base_entry(DOCK_LADDER, ordinal, *id)
                .filter(|entry| entry.appearance_id == 1948)
                .map(|_| ordinal)
        })
        .collect::<Vec<_>>()[..]
    else {
        return Err("step=dock-ladder expected=unique-authored-ladder-1948".into());
    };
    let key = crate::map::view::placement_key(DOCK_LADDER, ordinal)
        .ok_or("step=dock-ladder observed=no-placement-key")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let before = tile(client.world_spatial().actor_position)?;
    let outcome = client.use_object(&placement, 0).await?;
    if outcome.status != CommandStatus::Accepted
        || outcome.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    {
        return Err(
            format!("step=dock-ladder expected={DOCK_LANDING:?} observed={outcome:?}").into(),
        );
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    while tile(client.world_spatial().actor_position)? == before
        && tokio::time::Instant::now() < deadline
    {
        client.service_liveness(Duration::from_millis(25)).await?;
    }
    let landing = tile(client.world_spatial().actor_position)?;
    if landing != DOCK_LANDING {
        return Err(
            format!("step=dock-ladder observed={landing:?} expected={DOCK_LANDING:?}").into(),
        );
    }
    evidence(&format!(
        "row=W15c.full step=dock-ladder landing={DOCK_LANDING:?}"
    ));
    Ok(())
}

async fn sail_to_port_hope(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    coins: &[(std::num::NonZeroU32, u64)],
) -> TestResult<u64> {
    let before = carried_gold(client, coins)?;
    if before < FARE {
        return Err(format!("step=fare observed={before} expected=at-least-{FARE}").into());
    }
    climb_dock_ladder(client, world).await?;
    let captain = reach(client, world, npcs, CAPTAIN).await?;
    say(client, captain, "hi").await?;
    use oteryn_protocol_oteryn::{character_info, premium_overview};
    let reply = client
        .session_mut()
        .gated_command(
            character_info::CAPABILITY_CHARACTER_INFO_V1,
            character_info::COMMAND_TYPE_CHARACTER_INFO,
            &[premium_overview::OPERATION],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step=premium-prestate status={:?}", reply.status).into());
    }
    let overview = premium_overview::Overview::decode(&reply.payload)
        .ok_or("step=premium-prestate malformed overview")?;
    if overview.state != premium_overview::State::Active || overview.valid_for_us == 0 {
        return Err(format!(
            "step=premium-prestate state={:?} valid_for_us={}",
            overview.state, overview.valid_for_us
        )
        .into());
    }
    evidence(&format!(
        "row=W15c.full step=premium-prestate active=true valid_for_us={}",
        overview.valid_for_us
    ));
    let quote = say(client, captain, "port hope").await?;
    let expected = format!("Do you seek a passage to port_hope for {FARE} gold?");
    if quote.as_slice() != [expected] {
        return Err(
            format!("step=travel-quote observed={quote:?} expected=port-hope-{FARE}").into(),
        );
    }
    if say_with_speaker(
        client,
        captain,
        "yes",
        Some(("Captain Bluebear", "Set the sails!")),
    )
    .await?
    .as_slice()
        != ["Captain Bluebear: Set the sails!"]
    {
        return Err("step=travel-confirm expected=Captain-Bluebear-Set-the-sails".into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    loop {
        let position = tile(client.world_spatial().actor_position)?;
        let gold = carried_gold(client, coins)?;
        if position == PORT_HOPE && gold == before - FARE {
            evidence(&format!(
                "row=W15c.full step=travel arrival={position:?} before_gold={before} after_gold={gold}"
            ));
            return Ok(gold);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step=travel-arrival observed={position:?}-gold-{gold} expected={PORT_HOPE:?}-gold-{}", before - FARE).into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<Outcome> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let (task, target) = first_task()?;
    let expected_status = format!("{task}: 0/{target}");
    let identity = character_prep::dedicated_character(clients, "NODE_BOOT_W15C_ACCOUNT").await?;
    let scoped = identity.clients(clients);
    let mut client = connect_character(&scoped, 0xe5, 0).await?;
    let initial_session = client.join_snapshot().game_session_id;
    request_fixture_control(
        uuid_text(initial_session.as_bytes()),
        "--profile",
        PROFILE.into(),
    )
    .await?;
    character_prep::grant_item(&client, 3035, 2).await?;
    logout(&mut client).await?;
    drop(client);

    let mut client = connect_character(&scoped, 0xe5, 0).await?;
    if client.join_snapshot().game_session_id == initial_session {
        return Err("step=build observed=reused-session".into());
    }
    assert_build(&mut client).await?;
    let index = provisioning::item_index()?;
    let coins = [
        (provisioning::item_reference(&index, 3031)?, 1),
        (provisioning::item_reference(&index, 3035)?, 100),
        (provisioning::item_reference(&index, 3043)?, 10_000),
    ];
    let gold_after_fare = sail_to_port_hope(&mut client, world, &metadata.npcs, &coins).await?;
    let actor = reach(&mut client, world, &metadata.npcs, NPC).await?;
    const NONMEMBER: &str = "Would you like to join the Paw and Fur - Hunting Elite?";
    const MEMBER: &str =
        "Welcome back! Ask me about tasks, status, promotion, bosses or special tasks.";
    const NO_TASK: &str = "You have not started any task yet.";
    expect(
        &say(&mut client, actor, "hi").await?,
        NONMEMBER,
        "before-join",
    )?;
    expect(
        &say(&mut client, actor, "join").await?,
        "Welcome to the Paw and Fur - Hunting Elite! Ask me for a task.",
        "join",
    )?;
    expect(&say(&mut client, actor, "hi").await?, MEMBER, "after-join")?;
    expect(
        &say(&mut client, actor, "points").await?,
        "You have 0 Paw and Fur points. Your rank is Member.",
        "points",
    )?;
    expect_task_status(&mut client, actor, NO_TASK, "before-task").await?;
    let reply = say(&mut client, actor, &task).await?;
    let line = reply.first().cloned().unwrap_or_default();
    let started = match classify(&line) {
        TaskReply::Offer => {
            let expected_offer =
                format!("Hunt {target} {task}. Do you want to start this task? Say yes or no.");
            if reply.as_slice() != [expected_offer.as_str()] {
                return Err(
                    format!("step=task observed={line:?} expected={expected_offer:?}").into(),
                );
            }
            expect(
                &say(&mut client, actor, "yes").await?,
                "Happy hunting! Return here when your task is complete.",
                "confirm",
            )?;
            expect_task_status(&mut client, actor, &expected_status, "status").await?;
            true
        }
        TaskReply::Refusal => {
            expect(&reply, &line, "task-refusal")?;
            expect_task_status(&mut client, actor, NO_TASK, "after-refusal").await?;
            false
        }
        TaskReply::Other => {
            return Err(format!("step=task observed=unexpected-reply:{}", compact(&line)).into());
        }
    };
    let played_session = client.join_snapshot().game_session_id;
    let saved_position = client.world_spatial().actor_position;
    logout(&mut client).await?;
    drop(client);
    let mut client = connect_character(&scoped, 0xe5, 0).await?;
    if client.join_snapshot().game_session_id == played_session
        || client.world_spatial().actor_position != saved_position
    {
        return Err("step=fresh-admission observed=reused-session-or-wrong-position".into());
    }
    assert_build(&mut client).await?;
    let gold_after_relog = carried_gold(&client, &coins)?;
    if gold_after_relog != gold_after_fare {
        return Err(format!(
            "step=durable-fare observed={gold_after_relog} expected={gold_after_fare}"
        )
        .into());
    }
    let actor = reach(&mut client, world, &metadata.npcs, NPC).await?;
    expect(
        &say(&mut client, actor, "hi").await?,
        MEMBER,
        "durable-join",
    )?;
    let durable_status = if started { &expected_status } else { NO_TASK };
    expect_task_status(&mut client, actor, durable_status, "durable-task").await?;
    logout(&mut client).await?;
    Ok(if started {
        Outcome::Started(task)
    } else {
        Outcome::Refused(line)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replies_are_classified_by_their_exact_text() {
        assert_eq!(
            classify("Hunt 300 Crocodiles. Do you want to start this task? Say yes or no."),
            TaskReply::Offer
        );
        assert_eq!(classify("This task requires Premium."), TaskReply::Refusal);
        assert_eq!(
            classify("That task is outside your level range."),
            TaskReply::Refusal
        );
        assert_eq!(classify("Welcome"), TaskReply::Other);
    }

    #[test]
    fn tokens_are_stable() {
        assert_eq!(compact("a  b"), "a-b");
    }
}
