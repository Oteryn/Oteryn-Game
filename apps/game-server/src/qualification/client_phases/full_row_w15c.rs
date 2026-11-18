//! W15c Killing in the Name Of (Grizzly Adams), derived from the NPC conversation owner
//! (`gameplay_transport/npc_command/grizzly.rs`) and the quest rules (`quest/grizzly_adams/rules.rs`).
//!
//! 1. Take Captain Bluebear’s boat to Port Hope, then walk to Grizzly Adams. Say `hi`,
//!    `join`, and `points` and assert the server replies.
//! 2. Ask for the lowest-level task by name; assert the exact reply: a start offer (confirmed with
//!    `yes`, then `status` must list it), or the exact level/premium refusal.
//! 3. Log out, obtain a fresh admission, walk back and say `hi`; the durable join must answer
//!    `Welcome back`, and an accepted task must still be listed by `status`.
//! 4. PASS only when a task was started and survived the fresh admission; a refusal reports FAIL
//!    naming the precondition. Never writes state outside the real conversation.
use super::*;
use crate::map::view::MapFacts;
use oteryn_dev_client::client_runtime::npc_shop as runtime;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::npc_service as wire;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

const CAPTAIN: &str = "oteryn:npc.captain_bluebear";
const FARE: u64 = 160;
const BOARDING_LADDER: TilePos = TilePos {
    x: 32321,
    y: 32211,
    floor: -7,
};
const LADDER_APPEARANCE: u16 = 1948;
const PORT_HOPE: TilePos = TilePos {
    x: 32527,
    y: 32784,
    floor: -6,
};
// An enterable tile on the raised Port Hope walkway, east of the harbor.
const WALKWAY: TilePos = TilePos {
    x: 32682,
    y: 32770,
    floor: -6,
};
// Crystal 201189f8 data-global/world/world.otbm: the north-facing ramp at (32694, 32772, 7).
const RAMP_FOOT: TilePos = TilePos {
    x: 32694,
    y: 32773,
    floor: -7,
};

const NPC: &str = "oteryn:npc.grizzly_adams";
const REFUSALS: [&str; 2] = [
    "This task requires Premium.",
    "That task is outside your level range.",
];

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(720), verdict::observe(journey(clients)))
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
fn first_task() -> TestResult<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalogue: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/quests/killing-in-the-name-of.json"),
    )?)?;
    catalogue["tasks"]
        .as_array()
        .ok_or("step=catalogue observed=no-tasks")?
        .iter()
        .filter_map(|task| Some((task["minimum_level"].as_u64()?, task["name"].as_str()?)))
        .min()
        .map(|(_, name)| name.to_owned())
        .ok_or_else(|| "step=catalogue observed=empty".into())
}

fn placement<'a>(
    table: &'a oteryn_world_bundle::npc::Table,
    key: &str,
) -> TestResult<&'a oteryn_world_bundle::npc::Npc> {
    table
        .npcs
        .iter()
        .find(|npc| npc.key == key)
        .ok_or_else(|| format!("step=reach observed=missing-bundle-npc:{key}").into())
}

fn to_walkway(world: &BundleWorld, from: TilePos) -> Option<Vec<StepDirection>> {
    route(world, from, |at| at == WALKWAY)
}

fn from_ramp(
    world: &BundleWorld,
    npc: &oteryn_world_bundle::npc::Npc,
) -> TestResult<Vec<StepDirection>> {
    route(world, RAMP_FOOT, |at| near(at, npc))
        .ok_or_else(|| "step=reach observed=no-walk-route-from-ramp-to-grizzly-adams".into())
}

fn near(at: TilePos, npc: &oteryn_world_bundle::npc::Npc) -> bool {
    npc.placements
        .iter()
        .any(|p| p.floor == at.floor && at.x.abs_diff(p.x).max(at.y.abs_diff(p.y)) <= 3)
}

async fn walk_leg(
    client: &mut DevClientSession,
    world: &BundleWorld,
    leg: &str,
    path: &[StepDirection],
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    walk(client, world, path).await?;
    let to = tile(client.world_spatial().actor_position)?;
    evidence(&format!(
        "W15c_walk leg={leg} from={from:?} to={to:?} steps={} span_x={} span_y={}",
        path.len(),
        from.x.abs_diff(to.x),
        from.y.abs_diff(to.y)
    ));
    Ok(())
}

fn actor_at(
    client: &DevClientSession,
    npc: &oteryn_world_bundle::npc::Npc,
) -> TestResult<EntityRef> {
    client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entity| {
                entity.kind == oteryn_dev_client::EntityKind::Npc
                    && npc.placements.iter().any(|p| {
                        entity.position.x == i32::from(p.x)
                            && entity.position.y == i32::from(p.y)
                            && entity.position.floor == -i16::from(p.floor)
                    })
            })
        })
        .map(|entry| entry.entity)
        .ok_or_else(|| format!("step=reach observed=npc-not-visible-after-walk:{}", npc.key).into())
}

fn carried_gold(
    client: &DevClientSession,
    coins: &[(std::num::NonZeroU32, u64)],
) -> TestResult<u64> {
    Ok(client
        .inventory()
        .ok_or("step=boat observed=no-inventory")?
        .entries
        .iter()
        .filter_map(|entry| {
            coins
                .iter()
                .find(|(key, _)| key == &entry.item_definition_ref)
                .map(|(_, value)| u64::from(entry.count.get()) * value)
        })
        .sum())
}

fn near_boarding_ladder(at: TilePos) -> bool {
    at.floor == BOARDING_LADDER.floor
        && at
            .x
            .abs_diff(BOARDING_LADDER.x)
            .max(at.y.abs_diff(BOARDING_LADDER.y))
            <= 1
}

fn boarding_ladder_target(world: &BundleWorld) -> TestResult<WorldObjectTarget> {
    let at = BOARDING_LADDER;
    let base = world
        .base()
        .tile(at.x, at.y, at.floor)
        .ok_or("step=board observed=no-ladder-tile")?;
    let ordinal = base
        .ids()
        .iter()
        .zip(base.depths())
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
        .find_map(|(ordinal, (id, _))| {
            let ordinal = u8::try_from(ordinal).ok()?;
            world
                .facts()
                .base_entry(at, ordinal, *id)
                .filter(|entry| entry.appearance_id == LADDER_APPEARANCE && !entry.pickupable)
                .map(|_| ordinal)
        })
        .ok_or("step=board observed=no-authored-ladder")?;
    let key = crate::map::view::placement_key(at, ordinal)
        .ok_or("step=board observed=no-ladder-placement-key")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: world.facts().object_revision(key),
    })
}

async fn board(client: &mut DevClientSession, world: &BundleWorld, from: TilePos) -> TestResult {
    let target = boarding_ladder_target(world)?;
    let path = route(world, from, near_boarding_ladder)
        .ok_or("step=board observed=no-walk-route-to-boarding-ladder")?;
    walk_leg(client, world, "boarding-ladder", &path).await?;
    let used = client
        .use_object(&target.placement, target.expected_revision)
        .await?;
    if used.status != CommandStatus::Accepted || used.disposition != UseDisposition::Committed {
        return Err(format!("step=board observed=ladder-use-refused:{used:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        let at = tile(client.world_spatial().actor_position)?;
        if at.floor == BOARDING_LADDER.floor + 1
            && near_boarding_ladder(TilePos {
                floor: BOARDING_LADDER.floor,
                ..at
            })
        {
            evidence(&format!(
                "W15c_board ladder={BOARDING_LADDER:?} landed={at:?}"
            ));
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("step=board observed=no-upper-deck-position-delta".into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
}

async fn sail(
    client: &mut DevClientSession,
    world: &BundleWorld,
    captain: &oteryn_world_bundle::npc::Npc,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult {
    let index = provisioning::item_index()?;
    let coins = [
        (provisioning::item_reference(&index, 3031)?, 1),
        (provisioning::item_reference(&index, 3035)?, 100),
        (provisioning::item_reference(&index, 3043)?, 10_000),
    ];
    let deadline = tokio::time::Instant::now() + Duration::from_secs(240);
    verdict::Harness::global().bounded_wait(deadline, "boat-fare");
    while carried_gold(client, &coins)? < FARE {
        if tokio::time::Instant::now() >= deadline {
            return Err("step=boat observed=fare-loot-budget-expired".into());
        }
        travel_to_spawn(client, world, spawns, "oteryn:creature.rat").await?;
        client_hunt::hunt(client).await?;
    }

    let from = tile(client.world_spatial().actor_position)?;
    let path = if let Some(path) = route(world, from, |at| near(at, captain)) {
        path
    } else {
        board(client, world, from).await?;
        let deck = tile(client.world_spatial().actor_position)?;
        route(world, deck, |at| near(at, captain))
            .ok_or("step=boat observed=no-walk-route-to-captain-bluebear")?
    };
    walk_leg(client, world, "captain", &path).await?;
    let actor = actor_at(client, captain)?;
    say(client, actor, "hi").await?;
    let quote = say(client, actor, "port hope").await?;
    expect(
        &quote
            .iter()
            .map(|line| line.to_ascii_lowercase())
            .collect::<Vec<_>>(),
        &format!("port hope for {FARE} gold"),
        "boat-quote",
    )?;
    let before = carried_gold(client, &coins)?;
    expect(
        &say(client, actor, "yes").await?,
        "Set the sails!",
        "boat-confirm",
    )?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    while tile(client.world_spatial().actor_position)? != PORT_HOPE
        || carried_gold(client, &coins)?.checked_add(FARE) != Some(before)
    {
        if tokio::time::Instant::now() >= deadline {
            return Err("step=boat observed=missing-port-hope-arrival-or-fare-debit".into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
    evidence(&format!("W15c_boat destination={PORT_HOPE:?} fare={FARE}"));
    Ok(())
}

async fn reach(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    spawns: &oteryn_world_bundle::spawn::Table,
    npc: &oteryn_world_bundle::npc::Npc,
) -> TestResult<EntityRef> {
    let from = tile(client.world_spatial().actor_position)?;
    let ramp = route(world, WALKWAY, |at| at == RAMP_FOOT)
        .ok_or("step=reach observed=no-walk-route-from-walkway-to-ramp")?;
    let approach = from_ramp(world, npc)?;
    let path = if let Some(path) = to_walkway(world, from) {
        path
    } else {
        let harbor = to_walkway(world, PORT_HOPE)
            .ok_or("step=reach observed=no-walk-route-from-port-hope-to-walkway")?;
        sail(client, world, placement(npcs, CAPTAIN)?, spawns).await?;
        harbor
    };
    walk_leg(client, world, "port-hope-walkway", &path).await?;
    walk_leg(client, world, "grizzly-ramp-foot", &ramp).await?;
    approach_grizzly(client, world, npc, "grizzly-adams", &approach).await
}

async fn approach_grizzly(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npc: &oteryn_world_bundle::npc::Npc,
    leg: &str,
    path: &[StepDirection],
) -> TestResult<EntityRef> {
    let from = tile(client.world_spatial().actor_position)?;
    if from != RAMP_FOOT || path.is_empty() {
        return Err(format!("step={leg} observed=invalid-approach-start:{from:?}").into());
    }
    walk_leg(client, world, leg, path).await?;
    actor_at(client, npc)
}

/// One real talk intent; returns the reply lines the server addressed to this character.
async fn say(
    client: &mut DevClientSession,
    actor: EntityRef,
    text: &str,
) -> TestResult<Vec<String>> {
    client.take_events();
    let reply = runtime::talk(
        client.session_mut(),
        &wire::NpcTalkIntent {
            npc_actor: actor,
            text: text.into(),
        },
    )
    .await?;
    if reply != wire::NpcIntentDisposition::Ok {
        return Err(format!("step=say-{text} observed=disposition-{reply:?}").into());
    }
    let mut lines = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        for event in client.take_events() {
            if let SessionEvent::NpcConversation(delta) = event {
                lines.extend(delta.value.lines);
            }
        }
        if !lines.is_empty() || tokio::time::Instant::now() >= deadline {
            break;
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    if lines.is_empty() {
        return Err(format!("step=say-{text} observed=no-reply-line").into());
    }
    Ok(lines)
}

fn expect(lines: &[String], needle: &str, step: &str) -> TestResult {
    if lines.iter().any(|line| line.contains(needle)) {
        Ok(())
    } else {
        Err(format!(
            "step={step} observed={} expected={needle}",
            compact(&lines.join("|"))
        )
        .into())
    }
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<Outcome> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let npc = placement(&metadata.npcs, NPC)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let task = first_task()?;
    let mut client = connect_world(clients, 0xe5).await?;
    let actor = reach(&mut client, world, &metadata.npcs, &metadata.spawns, npc).await?;
    expect(&say(&mut client, actor, "hi").await?, "join", "hi")?;
    expect(&say(&mut client, actor, "join").await?, "Welcome", "join")?;
    expect(
        &say(&mut client, actor, "points").await?,
        "Paw and Fur points",
        "points",
    )?;
    let reply = say(&mut client, actor, &task).await?;
    let line = reply.first().cloned().unwrap_or_default();
    let started = match classify(&line) {
        TaskReply::Offer => {
            expect(
                &say(&mut client, actor, "yes").await?,
                "Happy hunting",
                "confirm",
            )?;
            expect(&say(&mut client, actor, "status").await?, &task, "status")?;
            true
        }
        TaskReply::Refusal => false,
        TaskReply::Other => {
            return Err(format!("step=task observed=unexpected-reply:{}", compact(&line)).into());
        }
    };
    let from = tile(client.world_spatial().actor_position)?;
    let path = route(world, from, |at| at == RAMP_FOOT)
        .ok_or("step=replay observed=no-walk-route-back-to-ramp-foot")?;
    walk_leg(&mut client, world, "return-to-ramp-foot", &path).await?;
    logout(&mut client).await?;
    let mut client = connect_world(clients, 0xe5).await?;
    let actor = approach_grizzly(
        &mut client,
        world,
        npc,
        "fresh-admission-grizzly-adams",
        &from_ramp(world, npc)?,
    )
    .await?;
    expect(
        &say(&mut client, actor, "hi").await?,
        "Welcome back",
        "durable-join",
    )?;
    if started {
        expect(
            &say(&mut client, actor, "status").await?,
            &task,
            "durable-task",
        )?;
    }
    logout(&mut client).await?;
    Ok(if started {
        Outcome::Started(task)
    } else {
        Outcome::Refused(line)
    })
}

pub(crate) fn port_hope_route_smoke() -> TestResult {
    use oteryn_world_bundle_compiler::{bundle::BuildClass, compile, source};

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // Match the node boot runner's production region compilation.
    let project =
        source::project_regions(&root, &[(126, 125), (126, 126), (127, 127), (127, 128)])?;
    let (registry, palette) = source::registry(&root, &project)?;
    let identity = serde_json::from_slice(&std::fs::read(
        root.join("content/world/pins/oteryn.identity.json"),
    )?)?;
    let compiled = compile::compile(
        &compile::Input {
            regions: &project.regions,
            palette: &palette,
            identity,
            world: source::world_extent(&root)?,
            build_class: BuildClass::NonProduction,
            draft_areas: oteryn_world_bundle_compiler::project::draft_areas(&project.index)?,
            families: &project.families,
        },
        &registry,
    )?;
    assert!(
        project.families.routes.iter().any(|route| {
            route.service == "oteryn:service.travel.captain_bluebear"
                && route.route == "port_hope"
                && route.destination == (PORT_HOPE.x, PORT_HOPE.y, PORT_HOPE.floor.unsigned_abs())
        }),
        "the compiled regions must include the authored Port Hope passage"
    );
    assert!(
        !compiled.npcs.routes_held.iter().any(|route| {
            route.service == "oteryn:service.travel.captain_bluebear" && route.route == "port_hope"
        }),
        "the compiler must admit the Port Hope arrival"
    );
    let metadata = oteryn_world_bundle::bundle::visit(&compiled.bytes, |_, _| Ok(()))?;
    let mut pin: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("content/world/pins/oteryn.json"))?)?;
    pin["digest"] = compiled
        .digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
        .into();
    let world = boot_world(
        compiled.bytes,
        &pin,
        WorldId::decode(&v7(1, 2))?,
        ChannelId::decode(&v7(1, 3))?,
    )?;
    let grizzly = placement(&metadata.npcs, NPC)?;
    let check_leg = |name, from: TilePos, path: Vec<StepDirection>| -> TestResult<TilePos> {
        assert!(!path.is_empty(), "{name} must walk");
        let mut at = from;
        let mut max_offset = (0, 0);
        for &direction in &path {
            at = destination(&world, at, direction).ok_or("leg has an invalid landing")?;
            max_offset.0 = max_offset.0.max(at.x.abs_diff(from.x));
            max_offset.1 = max_offset.1.max(at.y.abs_diff(from.y));
            assert!(
                !world.facts().doors().blocks(at),
                "{name} crossed a closed door at {at:?}"
            );
        }
        assert!(max_offset.0 <= 160 && max_offset.1 <= 160);
        eprintln!(
            "{name}: {from:?} -> {at:?}, {} steps, span {}x{}, max offset {}x{}",
            path.len(),
            at.x.abs_diff(from.x),
            at.y.abs_diff(from.y),
            max_offset.0,
            max_offset.1
        );
        Ok(at)
    };
    assert!(world.enterable(PORT_HOPE));
    let walkway = check_leg(
        "Port Hope to the raised walkway",
        PORT_HOPE,
        to_walkway(&world, PORT_HOPE).ok_or("walkway unreachable")?,
    )?;
    assert_eq!(walkway, WALKWAY);
    assert!(route(&world, PORT_HOPE, |at| near(at, grizzly)).is_none());
    let ramp = check_leg(
        "Raised walkway to Grizzly's ramp",
        walkway,
        route(&world, walkway, |at| at == RAMP_FOOT).ok_or("ramp foot unreachable")?,
    )?;
    assert_eq!(ramp, RAMP_FOOT);
    let approach = from_ramp(&world, grizzly)?;
    let reached = check_leg("Ramp foot to Grizzly Adams", ramp, approach)?;
    assert!(near(reached, grizzly));
    let replay_start = check_leg(
        "Grizzly Adams to replay logout tile",
        reached,
        route(&world, reached, |at| at == RAMP_FOOT).ok_or("logout ramp foot unreachable")?,
    )?;
    assert_eq!(replay_start, RAMP_FOOT);
    assert_eq!(
        check_leg(
            "Fresh-admission approach to Grizzly Adams",
            replay_start,
            from_ramp(&world, grizzly)?
        )?,
        reached
    );
    boarding_ladder_target(&world)?;
    check_leg(
        "Thais to boarding ladder",
        TOWN,
        route(&world, TOWN, near_boarding_ladder).ok_or("boarding ladder unreachable")?,
    )?;
    let captain = placement(&metadata.npcs, CAPTAIN)?;
    let deck = TilePos {
        y: BOARDING_LADDER.y + 1,
        floor: BOARDING_LADDER.floor + 1,
        ..BOARDING_LADDER
    };
    assert!(
        world
            .base()
            .tile(deck.x, deck.y, deck.floor)
            .is_some_and(|tile| tile.walkable())
    );
    assert!(world.enterable(deck));
    check_leg(
        "Upper deck to Captain Bluebear",
        deck,
        route(&world, deck, |at| near(at, captain)).ok_or("captain unreachable")?,
    )?;
    Ok(())
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
