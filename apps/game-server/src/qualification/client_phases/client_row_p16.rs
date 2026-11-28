//! P16 client replay capture: the real Eremo blessing dialogue (greeting, the quote for The
//! Wisdom of Solitude and the answer to the confirmation). The consumer is
//! `DialoguePanel::request`/`apply` in `crates/client-ui/src/features/0003-npc-service`.
use super::client_row_p20::{capture, hex, talk};
use super::*;

use crate::map::view::{MapFacts, placement_key};
use oteryn_dev_client::{DevClientSession, EntityKind, EntityRef};
use oteryn_protocol_oteryn::npc_service as wire;

const FERRIES: [(&str, &str, TilePos, u64); 3] = [
    (
        "oteryn:npc.captain_bluebear",
        "edron",
        TilePos {
            x: 33175,
            y: 31764,
            floor: -6,
        },
        160,
    ),
    (
        "oteryn:npc.captain_seahorse",
        "cormaya",
        TilePos {
            x: 33288,
            y: 31956,
            floor: -6,
        },
        20,
    ),
    (
        "oteryn:npc.pemaret",
        "eremo",
        TilePos {
            x: 33314,
            y: 31883,
            floor: -7,
        },
        0,
    ),
];

async fn approach(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    key: &str,
) -> TestResult<EntityRef> {
    let npc = npcs
        .npcs
        .iter()
        .find(|npc| npc.key == key)
        .ok_or_else(|| format!("P16 {key} absent from compiled world"))?;
    let near = |at: TilePos| {
        npc.placements.iter().any(|placement| {
            at.floor == placement.floor
                && at.x.abs_diff(placement.x).max(at.y.abs_diff(placement.y)) <= 3
        })
    };
    let path = route(world, tile(client.world_spatial().actor_position)?, near)
        .ok_or_else(|| format!("P16 {key} unreachable"))?;
    walk_p16(client, world, &path).await?;
    client
        .world_entities()
        .into_iter()
        .flat_map(|entities| entities.iter())
        .find(|actor| {
            actor.kind == EntityKind::Npc
                && npc.placements.iter().any(|p| {
                    tile(actor.position).ok()
                        == Some(TilePos {
                            x: p.x,
                            y: p.y,
                            floor: p.floor,
                        })
                })
        })
        .map(|actor| actor.entity)
        .ok_or_else(|| format!("P16 {key} not visible").into())
}

async fn walk_p16(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    for direction in path {
        let before = client.world_spatial().actor_position;
        let expected =
            destination(world, tile(before)?, *direction).ok_or("planned step has no landing")?;
        let mut result = client.step(*direction).await?;
        if result.disposition == StepDisposition::Blocked
            && client.world_spatial().actor_position == before
        {
            let blocker = client.world_entities().into_iter().flat_map(|entities| entities.iter())
                .find(|actor| actor.kind == EntityKind::Creature && tile(actor.position).ok() == Some(expected))
                .map(|actor| actor.entity)
                .ok_or_else(|| format!("P16 blocked step without visible creature from={before:?} landing={expected:?} result={result:?}"))?;
            evidence(&format!(
                "P16 travel_blocked creature={blocker:?} landing={expected:?} result={result:?}"
            ));
            tokio::time::timeout(Duration::from_secs(3), async {
                loop {
                    client.service_liveness(Duration::from_millis(50)).await?;
                    let entities = client
                        .world_entities()
                        .ok_or("P16 creature census absent")?;
                    if !entities.iter().any(|actor| {
                        actor.entity == blocker && tile(actor.position).ok() == Some(expected)
                    }) {
                        if entities.iter().any(|actor| {
                            actor.kind == EntityKind::Creature
                                && tile(actor.position).ok() == Some(expected)
                        }) {
                            return Err("P16 landing occupied by a different creature".into());
                        }
                        return TestResult::Ok(());
                    }
                }
            })
            .await
            .map_err(|_| format!("P16 creature {blocker:?} remained on {expected:?}"))??;
            evidence(&format!(
                "P16 travel_blocker_cleared creature={blocker:?} landing={expected:?}"
            ));
            result = client.step(*direction).await?;
        }
        if result.disposition != StepDisposition::Moved
            || result.world_spatial_delta.is_none()
            || tile(client.world_spatial().actor_position)? != expected
        {
            return Err(
                format!("P16 step from={before:?} landing={expected:?} result={result:?}").into(),
            );
        }
        verdict::Harness::global().progress("walk");
    }
    Ok(())
}

async fn board_thais_ship(client: &mut DevClientSession, world: &BundleWorld) -> TestResult {
    const LADDER: TilePos = TilePos {
        x: 32321,
        y: 32220,
        floor: -7,
    };
    const DECK: TilePos = TilePos {
        x: 32321,
        y: 32221,
        floor: -6,
    };
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at.floor == LADDER.floor && at.x.abs_diff(LADDER.x).max(at.y.abs_diff(LADDER.y)) <= 1
    })
    .ok_or("P16 authored Thais boarding ladder unreachable")?;
    walk_p16(client, world, &path).await?;
    let ladder = world
        .base()
        .tile(LADDER.x, LADDER.y, LADDER.floor)
        .ok_or("P16 Thais boarding ladder tile absent")?;
    let (ordinal, _) = ladder
        .ids()
        .iter()
        .zip(ladder.depths())
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
        .find(|(ordinal, (id, _))| {
            u8::try_from(*ordinal)
                .ok()
                .and_then(|ordinal| world.facts().base_entry(LADDER, ordinal, **id))
                .is_some_and(|facts| facts.appearance_id == 1948)
        })
        .ok_or("P16 authored Thais boarding ladder missing from tile")?;
    let key =
        placement_key(LADDER, u8::try_from(ordinal)?).ok_or("P16 ladder placement key absent")?;
    let mut target = world.base().digest().to_vec();
    target.extend_from_slice(&key.to_be_bytes());
    let before = tile(client.world_spatial().actor_position)?;
    let result = client
        .use_object(&target, world.facts().object_revision(key))
        .await?;
    if result.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed {
        return Err(
            format!("P16 Thais boarding ladder refused from={before:?} result={result:?}").into(),
        );
    }
    tokio::time::timeout(Duration::from_secs(3), async {
        while tile(client.world_spatial().actor_position)? == before {
            client.service_liveness(Duration::from_millis(25)).await?;
        }
        TestResult::Ok(())
    })
    .await
    .map_err(|_| "P16 boarding ladder did not move the actor")??;
    let after = tile(client.world_spatial().actor_position)?;
    if after != DECK {
        return Err(format!(
            "P16 Thais boarding ladder from={before:?} observed={after:?} expected={DECK:?}"
        )
        .into());
    }
    evidence(&format!(
        "P16 boarded ladder={LADDER:?} from={before:?} deck={after:?}"
    ));
    Ok(())
}

fn gold(client: &DevClientSession, coins: &[(std::num::NonZeroU32, u64)]) -> TestResult<u64> {
    let inventory = client.inventory().ok_or("P16 inventory unavailable")?;
    Ok(inventory
        .entries
        .iter()
        .map(|entry| {
            coins
                .iter()
                .find(|(key, _)| key == &entry.item_definition_ref)
                .map_or(0, |(_, value)| u64::from(entry.count.get()) * value)
        })
        .sum())
}

async fn sail(
    client: &mut DevClientSession,
    npc: EntityRef,
    keyword: &str,
    arrival: TilePos,
    fare: u64,
    coins: &[(std::num::NonZeroU32, u64)],
) -> TestResult {
    let expected_gold = gold(client, coins)?
        .checked_sub(fare)
        .ok_or("P16 ferry fare not funded")?;
    let before = tile(client.world_spatial().actor_position)?;
    if before == arrival {
        return Err(format!("P16 {keyword} departure already at arrival {arrival:?}").into());
    }
    let _ = talk(client, npc, "hi").await?;
    let _ = talk(client, npc, keyword).await?;
    tokio::time::sleep(Duration::from_millis(260)).await;
    let payload = wire::encode_npc_talk_intent(&wire::NpcTalkIntent {
        npc_actor: npc,
        text: "yes".into(),
    })
    .map_err(|error| format!("P16 {keyword} confirmation encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_NPC_SERVICE_V1,
            wire::COMMAND_TYPE_NPC_TALK_INTENT,
            &payload,
        )
        .await?;
    let disposition = wire::decode_npc_intent_result(&reply.payload)
        .map_err(|error| format!("P16 {keyword} confirmation decode: {error:?}"))?;
    if reply.status != CommandStatus::Accepted || disposition != wire::NpcIntentDisposition::Ok {
        return Err(
            format!("P16 {keyword} confirmation refused: {reply:?} {disposition:?}").into(),
        );
    }
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let delta = client
                .session_mut()
                .read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1)
                .await?;
            if delta.server_sequence <= reply.server_sequence {
                continue;
            }
            let response = wire::decode_npc_conversation(&delta.payload)
                .map_err(|error| format!("P16 {keyword} departure decode: {error:?}"))?;
            if response.open.is_some() || response.lines != ["Set the sails!"] {
                return Err(
                    format!("P16 {keyword} departure response: {:?}", response.lines).into(),
                );
            }
            break;
        }
        while tile(client.world_spatial().actor_position)? != arrival
            || gold(client, coins)? != expected_gold
        {
            client.service_liveness(Duration::from_millis(25)).await?;
        }
        TestResult::Ok(())
    })
    .await
    .map_err(|_| {
        format!(
            "P16 {keyword} expected arrival={arrival:?} observed={:?}",
            client.world_spatial().actor_position
        )
    })??;
    evidence(&format!(
        "P16 ferry_arrival route={keyword} from={before:?} arrival={arrival:?}"
    ));
    verdict::Harness::global().progress("ferry-arrival");
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_P16_ACCOUNT").await?;
    let scope = dedicated.clients(clients);
    let mut client = connect_character(&scope, 0x51, 0).await?;
    let index = provisioning::item_index()?;
    let coins = [
        (provisioning::item_reference(&index, 3031)?, 1),
        (provisioning::item_reference(&index, 3035)?, 100),
        (provisioning::item_reference(&index, 3043)?, 10_000),
    ];
    let initial = gold(&client, &coins)?;
    character_prep::grant_item(&client, 3035, 2).await?;
    let preparation_session = client.join_snapshot().game_session_id;
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("P16 fare preparation logout refused: {logout:?}").into());
    }
    drop(client);
    let mut client = connect_character(&scope, 0x52, 0).await?;
    if client.join_snapshot().game_session_id == preparation_session
        || gold(&client, &coins)? != initial + 200
    {
        return Err("P16 ferry funding missing after fresh admission".into());
    }
    board_thais_ship(&mut client, world).await?;
    for (key, keyword, arrival, fare) in FERRIES {
        let npc = approach(&mut client, world, &metadata.npcs, key).await?;
        sail(&mut client, npc, keyword, arrival, fare, &coins).await?;
    }
    let npc = approach(&mut client, world, &metadata.npcs, "oteryn:npc.eremo").await?;
    let (greeting_bytes, greeting) = talk(&mut client, npc, "hi").await?;
    if greeting.is_empty() {
        return Err("hi produced an empty conversation".into());
    }
    let greeting_sequence = client.last_server_sequence();
    capture("P16", "p16-greeting.bin", &greeting_bytes);
    let (quote_bytes, quote) = talk(&mut client, npc, "solitude").await?;
    if quote != ["Would you like to receive that protection for a sacrifice of 2000 gold, child?"] {
        return Err(format!("blessing quote observed={quote:?}").into());
    }
    let quote_sequence = client.last_server_sequence();
    capture("P16", "p16-quote.bin", &quote_bytes);
    let (answer_bytes, answer) = talk(&mut client, npc, "no").await?;
    if answer != ["Fine. You are free to decline my offer."] {
        return Err(format!("blessing decline observed={answer:?}").into());
    }
    let answer_sequence = client.last_server_sequence();
    capture("P16", "p16-answer.bin", &answer_bytes);
    if !(greeting_sequence < quote_sequence && quote_sequence < answer_sequence) {
        return Err("P16 capture sequences did not advance".into());
    }
    let capture_session = hex(client.join_snapshot().game_session_id.as_bytes());
    let binding =
        format!("{capture_session}:{greeting_sequence}:{quote_sequence}:{answer_sequence}");
    let expected = format!(
        "binding={binding}\nnpc_identity={}\nnpc_generation={}\ngreeting={}\nquote={}\nanswer={}\n",
        hex(&npc.identity),
        npc.generation,
        greeting.join("|"),
        quote.join("|"),
        answer.join("|")
    );
    capture("P16", "p16-expected.txt", expected.as_bytes());
    let _ = client.session_mut().logout().await?;
    Ok(binding)
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_capture scenario=P16.client status=CAPTURED binding={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P16.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P16.client status=FAIL evidence=timeout"),
    }
}
