//! Q02: The Order of the Stag, played from the first boat to the reward (QSTAG-E2E).
//!
//! The row simulates a player and records one `Q02_CHECK <step> PASS|FAIL <evidence>` line per
//! step. A step blocked by missing content or by an earlier step is a FAIL with the exact
//! refusal; no step is skipped, and every step still runs after a FAIL. The row fails when any
//! step failed.
//!
//! Source: `content/quests/world-callbacks.json` (boat to Ada, boat return, mark wall, crypt
//! exit), `content/dialogues/quest-rules/quest-batch3-c-the_order_of_the_stag.json` (Kesar) and
//! `content/quests/native-death-bindings/quest-batch3-c-the_order_of_the_stag.json` (Court Warlock).
//! The quest needs level 250 and a premium account; setup raises the character with the node-boot
//! fixture's `sweep` build (knight, level 1000), the way Q01 uses the knight build.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, SessionEvent};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_protocol_oteryn::zones_nologout::ServerMessage;
use std::collections::HashMap;

const QUEST_LEVEL: u32 = 250;
const KESAR: &str = "oteryn:npc.kesar_the_younger_day";
const WARLOCK: &str = "oteryn:creature.court_warlock";
const REWARD_ITEM: u32 = 52824;
const BOAT_REFUSAL: &str = "This small fishing boat is tied up firmly.";

const fn at(x: u16, y: u16, z: i8) -> TilePos {
    TilePos { x, y, floor: -z }
}

const BOAT_TO_ADA: TilePos = at(32393, 32513, 7);
const ADA_LANDING: TilePos = at(33875, 32249, 7);
const BOAT_BACK: TilePos = at(33874, 32250, 7);
const BOUNAC_LANDING: TilePos = at(32395, 32511, 7);
const MARK_WALL: TilePos = at(33834, 32139, 4);
const WALL_LANDING: TilePos = at(33833, 32138, 4);
const CRYPT_EXIT: TilePos = at(32402, 32472, 5);
const CRYPT_LANDING: TilePos = at(32402, 32473, 4);
/// Day and night tiles of Kesar's clock spawn (`apps/game-server/src/node/serve/bundle_npcs.rs`).
const KESAR_DAY: TilePos = at(32365, 32478, 2);
const KESAR_NIGHT: TilePos = at(32368, 32473, 3);

fn show(pos: TilePos) -> String {
    format!("{},{},{}", pos.x, pos.y, -i16::from(pos.floor))
}

struct Row {
    failed: usize,
}

impl Row {
    fn check(&mut self, step: &str, result: TestResult<String>) {
        match result {
            Ok(evidence_text) => evidence(&format!("Q02_CHECK {step} PASS {evidence_text}")),
            Err(error) => {
                self.failed += 1;
                evidence(&format!("Q02_CHECK {step} FAIL {error}"));
            }
        }
    }
}

/// The depth-0 item index of the last item on each wanted tile (the boat, the wall, the hatch).
fn ordinals(bytes: &[u8], wanted: &[TilePos]) -> TestResult<HashMap<TilePos, u8>> {
    let mut found = HashMap::new();
    oteryn_world_bundle::bundle::visit(bytes, |_, sector| {
        for tile in sector.tiles {
            let pos = TilePos {
                x: tile.x,
                y: tile.y,
                floor: sector.floor,
            };
            if !wanted.contains(&pos) {
                continue;
            }
            let count = tile.items.iter().filter(|item| item.depth == 0).count();
            if let Some(last) = count.checked_sub(1).and_then(|n| u8::try_from(n).ok()) {
                found.insert(pos, last);
            }
        }
        Ok(())
    })?;
    Ok(found)
}

/// Every server message the USE produced, for the exact refusal text.
async fn messages(client: &mut DevClientSession) -> TestResult<Vec<String>> {
    let mut texts = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_millis(1500);
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(50)).await?;
        for event in client.take_events() {
            if let SessionEvent::ServerMessage(delta) = event {
                texts.push(match delta.value {
                    ServerMessage::StatusText(text)
                    | ServerMessage::Warning(text)
                    | ServerMessage::Advance(text) => text,
                });
            }
        }
    }
    Ok(texts)
}

/// Walk next to `object`, USE it as a player does and return the answer and the messages.
async fn use_tile(
    client: &mut DevClientSession,
    planner: &travel::Planner<'_>,
    ordinals: &HashMap<TilePos, u8>,
    label: &str,
    object: TilePos,
) -> TestResult<(UseDisposition, Vec<String>)> {
    let ordinal = *ordinals
        .get(&object)
        .ok_or_else(|| format!("{label} no object on {} in the world bundle", show(object)))?;
    let mut passages = travel::Passages::default();
    travel::travel_to(
        client,
        planner,
        label,
        object,
        &travel::beside(object),
        &mut passages,
    )
    .await?;
    client.take_events();
    let reply = travel::use_map_object(client, &planner.placement(object, ordinal)?).await?;
    Ok((reply, messages(client).await?))
}

/// USE must be refused with `text`; anything else is a FAIL naming what the server did.
async fn refused(
    client: &mut DevClientSession,
    planner: &travel::Planner<'_>,
    ordinals: &HashMap<TilePos, u8>,
    label: &str,
    object: TilePos,
    text: &str,
) -> TestResult<String> {
    let (reply, said) = use_tile(client, planner, ordinals, label, object).await?;
    if reply != UseDisposition::Committed && said.iter().any(|line| line.contains(text)) {
        return Ok(format!("use={reply:?} message={said:?}"));
    }
    Err(
        format!("expected a refusal with {text:?}; observed use={reply:?} messages={said:?}")
            .into(),
    )
}

/// USE must commit and put the character on `landing`; a refusal is the FAIL with its text.
async fn crossed(
    client: &mut DevClientSession,
    planner: &travel::Planner<'_>,
    ordinals: &HashMap<TilePos, u8>,
    label: &str,
    object: TilePos,
    landing: TilePos,
) -> TestResult<String> {
    let (reply, said) = use_tile(client, planner, ordinals, label, object).await?;
    if reply != UseDisposition::Committed {
        return Err(format!(
            "USE at {} refused: use={reply:?} messages={said:?}",
            show(object)
        )
        .into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        let here = tile(client.world_spatial().actor_position)?;
        if here == landing {
            return Ok(format!("from={} landed={}", show(object), show(here)));
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "USE at {} committed but the character is at {}, not {}; messages={said:?}",
                show(object),
                show(here),
                show(landing)
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
}

/// Setup: the character must be level 250. A lower character gets the fixture `sweep` build
/// (knight, level 1000) through the fenced fixture, then a fresh admission.
async fn setup(clients: &SeamClients<'_>, client: &mut DevClientSession) -> TestResult<String> {
    let before = full_row_q01::level(client).await?;
    if before >= QUEST_LEVEL {
        return Ok(format!("level={before}"));
    }
    let after = full_row_q01::provision(clients, client, 0xe6, 1, "sweep", "Q02").await?;
    if after < QUEST_LEVEL {
        return Err(format!(
            "setup: needs level {QUEST_LEVEL}; the sweep fixture build left the character at level {after} (before {before})"
        )
        .into());
    }
    Ok(format!(
        "level_before={before} level={after} profile=sweep reload=fresh_admission"
    ))
}

/// Talk to Kesar as a player: walk to him and say hi, problem, help, help, help.
/// Kesar has no fixed placement: the day/night clock spawns him at his day or night tile
/// (`bundle_npcs.rs`), so the row walks to the day tile, then the night tile, until it sees him.
async fn kesar(
    client: &mut DevClientSession,
    planner: &travel::Planner<'_>,
    npcs: &oteryn_world_bundle::npc::Table,
) -> TestResult<String> {
    let mut spots: Vec<TilePos> = npcs
        .npcs
        .iter()
        .filter(|npc| npc.key == KESAR)
        .flat_map(|npc| npc.placements.iter())
        .map(|place| TilePos {
            x: place.x,
            y: place.y,
            floor: place.floor,
        })
        .collect();
    spots.extend([KESAR_DAY, KESAR_NIGHT]);
    let mut misses = Vec::new();
    let mut actor = None;
    for spot in spots {
        let near = move |here: TilePos| {
            here.floor == spot.floor && here.x.abs_diff(spot.x).max(here.y.abs_diff(spot.y)) <= 2
        };
        let mut passages = travel::Passages::default();
        if let Err(error) =
            travel::travel_to(client, planner, "Q02:kesar", spot, &near, &mut passages).await
        {
            misses.push(format!("{}: {error}", show(spot)));
            continue;
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            let found = client.world_entities().and_then(|entities| {
                entities.iter().find(|entity| {
                    entity.kind == EntityKind::Npc
                        && entity.position.floor == -i16::from(spot.floor)
                        && entity.position.x.abs_diff(i32::from(spot.x)) <= 3
                        && entity.position.y.abs_diff(i32::from(spot.y)) <= 3
                })
            });
            if let Some(entity) = found {
                actor = Some(entity.entity);
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                misses.push(format!("{}: no NPC visible within 3 tiles", show(spot)));
                break;
            }
            client.service_liveness(Duration::from_millis(50)).await?;
            client.take_events();
        }
        if actor.is_some() {
            break;
        }
    }
    let actor = actor.ok_or_else(|| format!("kesar not found: {misses:?}"))?;
    let mut replies = Vec::new();
    for text in ["hi", "problem", "help", "help", "help"] {
        let lines = travel::converse(client, actor, text).await?;
        evidence(&format!("Q02_NPC say={text:?} reply={lines:?}"));
        if lines.is_empty() {
            return Err(format!("kesar gave no reply to {text:?}").into());
        }
        replies.push(lines);
    }
    Ok(format!("replies={replies:?}"))
}

/// The Court Warlock must be fought: it needs a spawn, a route to it and a won hunt.
async fn warlock(
    client: &mut DevClientSession,
    planner: &travel::Planner<'_>,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult<String> {
    let point = spawns
        .sources
        .iter()
        .find_map(|source| {
            source.points.iter().find_map(|point| {
                (spawns.creatures[point.creature as usize].key == WARLOCK)
                    .then_some(TilePos {
                        x: point.x,
                        y: point.y,
                        floor: source.floor,
                    })
            })
        })
        .ok_or_else(|| format!("{WARLOCK} has no spawn in the world bundle; the death binding needs it in room 33830,32133..33847,32150 z2"))?;
    let mut passages = travel::Passages::default();
    travel::travel_to(
        client,
        planner,
        "Q02:warlock",
        point,
        &travel::beside(point),
        &mut passages,
    )
    .await?;
    client_hunt::hunt(client).await?;
    Ok(format!("killed at {}", show(point)))
}

/// The final reward must be carried.
fn reward(client: &DevClientSession) -> TestResult<String> {
    let reference = provisioning::item_reference(&provisioning::item_index()?, REWARD_ITEM)?;
    let held = full_row_q01::carried(client, reference);
    if held == 0 {
        return Err(format!(
            "reward item {REWARD_ITEM} (The Broken Sword of Eldoran Etzel) is not in the backpack"
        )
        .into());
    }
    Ok(format!("item={REWARD_ITEM} carried={held}"))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let planner = full_row_q01::planner(world, &bytes)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let objects = ordinals(&bytes, &[BOAT_TO_ADA, BOAT_BACK, MARK_WALL, CRYPT_EXIT])?;
    let mut client = connect_character(clients, 0xe6, 1).await?;
    let mut row = Row { failed: 0 };
    let c = &mut client;
    row.check("setup", setup(clients, c).await);
    row.check(
        "boat_refusal",
        refused(
            c,
            planner,
            &objects,
            "Q02:boat-refusal",
            BOAT_TO_ADA,
            BOAT_REFUSAL,
        )
        .await,
    );
    row.check("kesar", kesar(c, planner, &metadata.npcs).await);
    row.check(
        "boat_to_ada",
        crossed(
            c,
            planner,
            &objects,
            "Q02:boat-to-ada",
            BOAT_TO_ADA,
            ADA_LANDING,
        )
        .await,
    );
    row.check(
        "mark_wall",
        crossed(
            c,
            planner,
            &objects,
            "Q02:mark-wall",
            MARK_WALL,
            WALL_LANDING,
        )
        .await,
    );
    row.check("court_warlock", warlock(c, planner, &metadata.spawns).await);
    row.check(
        "boat_return",
        crossed(
            c,
            planner,
            &objects,
            "Q02:boat-return",
            BOAT_BACK,
            BOUNAC_LANDING,
        )
        .await,
    );
    row.check(
        "crypt_exit",
        crossed(
            c,
            planner,
            &objects,
            "Q02:crypt-exit",
            CRYPT_EXIT,
            CRYPT_LANDING,
        )
        .await,
    );
    row.check("reward", reward(c));
    let _ = client.session_mut().logout().await;
    if row.failed > 0 {
        return Err(format!("{} Q02 step(s) failed", row.failed).into());
    }
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result: TestResult = verdict::observe(journey(clients)).await;
    if let Err(error) = &result {
        evidence(&format!("Q02_GAP {error}"));
    }
    evidence(&format!(
        "e2e_all scenario=Q02.full status={} seconds={:.3}",
        if result.is_ok() { "PASS" } else { "FAIL" },
        started.elapsed().as_secs_f64()
    ));
}
