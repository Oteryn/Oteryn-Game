//! E02 client replay capture: the NPC gold-to-platinum exchange changes the backpack coins.
//! Client owner: the CharacterInventory domain in `crates/session` (`Session::inventory`). The
//! replay consumes the incoming admission and inventory frames from this live session.
use super::*;
use oteryn_dev_client::client_runtime::npc_shop as runtime;
use oteryn_dev_client::{DevClientSession, EntityKind, EntityRef};
use oteryn_protocol_oteryn::item_view;
use oteryn_protocol_oteryn::npc_service as wire;

// Global TibiaWiki (Kepar) documents his coin exchange at the Adventurers' Guild.
const BANKER: &str = "oteryn:npc.kepar";
const GOLD_ID: u32 = 3031;
const PLATINUM_ID: u32 = 3035;
const CRYSTAL_ID: u32 = 3043;
const PROMPT: &str = "How many platinum coins would you like to get?";

fn pairs(client: &DevClientSession) -> String {
    client.inventory().map_or_else(String::new, |inventory| {
        inventory
            .entries
            .iter()
            .map(|item| format!("{}:{};", item.item_definition_ref, item.count))
            .collect()
    })
}

pub(super) fn encoded_backpack(client: &DevClientSession) -> TestResult<Vec<u8>> {
    // The coin replays negotiate the backpack-only inventory capability.
    let mut inventory = client.inventory().cloned().ok_or("inventory absent")?;
    inventory.equipment.clear();
    item_view::encode_character_inventory(&inventory)
        .map_err(|error| format!("inventory encode: {error:?}").into())
}

pub(super) fn coin_refs() -> TestResult<[std::num::NonZeroU32; 3]> {
    let index = provisioning::item_index()?;
    Ok([
        provisioning::item_reference(&index, GOLD_ID)?,
        provisioning::item_reference(&index, PLATINUM_ID)?,
        provisioning::item_reference(&index, CRYSTAL_ID)?,
    ])
}

pub(super) fn backpack_coins(client: &DevClientSession, refs: [std::num::NonZeroU32; 3]) -> TestResult<[u64; 3]> {
    let inventory = client.inventory().ok_or("step=inventory observed=snapshot-absent")?;
    let mut counts = [0; 3];
    for entry in &inventory.entries {
        if let Some(index) = refs.iter().position(|reference| *reference == entry.item_definition_ref) {
            counts[index] += u64::from(entry.count.get());
        }
    }
    Ok(counts)
}

pub(super) async fn wait_coins(
    client: &mut DevClientSession,
    refs: [std::num::NonZeroU32; 3],
    expected: [u64; 3],
    step: &'static str,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    loop {
        let observed = backpack_coins(client, refs)?;
        if observed == expected {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step={step} observed=coin-delta-timeout:{observed:?} expected={expected:?}").into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
}

async fn talk(client: &mut DevClientSession, actor: EntityRef, text: &str) -> TestResult<wire::NpcConversation> {
    tokio::time::sleep(Duration::from_millis(260)).await;
    let disposition = runtime::talk(
        client.session_mut(),
        &wire::NpcTalkIntent { npc_actor: actor, text: text.into() },
    )
    .await?;
    if disposition != wire::NpcIntentDisposition::Ok {
        return Err(format!("step=talk-{text} observed=disposition:{disposition:?}").into());
    }
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_CONVERSATION, 1)
        .await?;
    wire::decode_npc_conversation(&delta.payload)
        .map_err(|error| format!("step=talk-{text} observed=malformed-conversation:{error:?}").into())
}

fn visible_npc(client: &DevClientSession, npc: &oteryn_world_bundle::npc::Npc) -> Option<EntityRef> {
    client.world_entities()?.iter().find(|entity| {
        entity.kind == EntityKind::Npc
            && npc.placements.iter().any(|placement| {
                entity.position.x == i32::from(placement.x)
                    && entity.position.y == i32::from(placement.y)
                    && entity.position.floor == -i16::from(placement.floor)
            })
    }).map(|entity| entity.entity)
}

pub(super) async fn approach(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npc: &oteryn_world_bundle::npc::Npc,
) -> TestResult<EntityRef> {
    let path = npc_approach_path(client, world, npc)
        .await?
        .ok_or_else(|| format!("step=walk-npc observed=no-route:{}", npc.key))?;
    approach_along(client, world, npc, &path).await
}

async fn approach_along(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npc: &oteryn_world_bundle::npc::Npc,
    path: &[StepDirection],
) -> TestResult<EntityRef> {
    let from = client.world_spatial().actor_position;
    evidence(&format!(
        "E02_ROUTE npc={} from={from:?} steps={}",
        npc.key,
        path.len()
    ));
    walk(client, world, path).await?;
    visible_npc(client, npc)
        .ok_or_else(|| format!("step=walk-npc observed=actor-not-visible:{}", npc.key).into())
}

pub(super) async fn find_exchange_npc(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    book: &crate::gameplay_transport::NpcTalkBook,
) -> TestResult<(EntityRef, String)> {
    let banker = npcs
        .npcs
        .iter()
        .find(|npc| npc.key == BANKER)
        .filter(|npc| book.get(&npc.key).is_some_and(|entry| entry.bank()))
        .ok_or_else(|| format!("step=find-exchange-npc observed=missing-authored-banker:{BANKER}"))?;
    let path = npc_approach_path(client, world, banker)
        .await?
        .filter(|path| !path.is_empty())
        .ok_or("step=walk-npc observed=no-discriminating-route")?;
    for direction in path {
        let before = tile(client.world_spatial().actor_position)?;
        let expected = destination(world, before, direction)
            .filter(|landing| *landing != before)
            .ok_or("step=walk-npc observed=no-discriminating-landing")?;
        let outcome = client.step(direction).await?;
        let after = tile(client.world_spatial().actor_position)?;
        if outcome.disposition != StepDisposition::Moved
            || outcome.world_spatial_delta.is_none()
            || after != expected
        {
            return Err(format!(
                "step=walk-npc observed={after:?} expected={expected:?} outcome={outcome:?}"
            ).into());
        }
        verdict::Harness::global().progress("walk");
        client.take_events();
    }
    let actor = visible_npc(client, banker).ok_or("step=walk-npc observed=kepar-not-visible")?;
    let greeting = talk(client, actor, "hi").await?;
    let reply = talk(client, actor, "change gold").await?;
    if !reply.lines.iter().any(|line| line == PROMPT) {
        return Err(
            format!(
                "step=find-exchange-npc observed=wrong-live-prompt:{BANKER} hi={:?} change-gold={:?}",
                greeting.lines, reply.lines
            )
            .into(),
        );
    }
    Ok((actor, banker.key.clone()))
}

async fn exchange(
    client: &mut DevClientSession,
    actor: EntityRef,
    words: &[&str],
) -> TestResult<wire::NpcConversation> {
    let mut last = None;
    for word in words {
        last = Some(talk(client, actor, word).await?);
    }
    last.ok_or_else(|| "step=exchange observed=empty-command-sequence".into())
}

pub(super) async fn coin_profile<'a>(
    clients: &SeamClients<'a>,
    row: &str,
    name: &str,
    tag: u8,
    refs: [std::num::NonZeroU32; 3],
) -> TestResult<(Vec<[u8; 16]>, DevClientSession)> {
    let characters = seed_coin_profile(clients, row, name, tag, refs).await?;
    let dedicated = SeamClients {
        characters: &characters,
        ..*clients
    };
    let client = connect_world(&dedicated, tag).await?;
    require_seeded_coins(&client, refs)?;
    Ok((characters, client))
}

async fn seed_coin_profile(
    clients: &SeamClients<'_>,
    row: &str,
    name: &str,
    tag: u8,
    refs: [std::num::NonZeroU32; 3],
) -> TestResult<Vec<[u8; 16]>> {
    mod hunt_route {
        use super::*;
        include!("qualification_hunt_route.rs");
    }

    let mut characters = clients.characters.to_vec();
    characters[1] = hunt_route::qualification_character(clients, 1, tag, name, "Knight").await?;
    let dedicated = SeamClients {
        characters: &characters,
        ..*clients
    };
    let mut client = connect_world(&dedicated, tag).await?;
    let before = backpack_coins(&client, refs)?;
    if before != [50, 0, 0] {
        return Err(format!("E02 starter gold expected [50, 0, 0], observed {before:?}").into());
    }
    character_prep::declare_seeded(
        row,
        "100 carried gold coins",
        "complete the 50-coin starter balance on the dedicated character",
    );
    // The fixture accepts at most 20 per ordinary named profile and replays
    // identical claims in one session, so use three distinct grants totaling 50.
    for count in [11, 19, 20] {
        character_prep::grant_item(&client, GOLD_ID, count).await?;
    }
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("E02 setup logout refused: {:?}", logout.result).into());
    }
    Ok(characters)
}

fn require_seeded_coins(client: &DevClientSession, refs: [std::num::NonZeroU32; 3]) -> TestResult {
    let after = backpack_coins(client, refs)?;
    if after != [100, 0, 0] {
        return Err(format!("E02 seeded gold expected [100, 0, 0], observed {after:?}").into());
    }
    Ok(())
}

fn exact_delta(before: [u64; 3], after: [u64; 3], delta: [i64; 3]) -> bool {
    before
        .into_iter()
        .zip(after)
        .zip(delta)
        .all(|((before, after), delta)| i128::from(after) - i128::from(before) == i128::from(delta))
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
    let refs = coin_refs()?;
    use oteryn_dev_client::native_session::test_support::capture_server_frames;

    let characters =
        seed_coin_profile(clients, "E02.client", "Coin Exchange Capture", 0x33, refs).await?;
    let dedicated = SeamClients {
        characters: &characters,
        ..*clients
    };
    let (exchange_result, frames) = capture_server_frames(async {
        let mut client = connect_world(&dedicated, 0x33).await?;
        require_seeded_coins(&client, refs)?;
        let talk_book = full_e2ef20::qualification_npc_talk_book(&mut client).await?;
        let initial = backpack_coins(&client, refs)?;
        let (actor, npc_key) = find_exchange_npc(&mut client, world, &metadata.npcs, &talk_book).await?;
        let before = pairs(&client);
        let before_sequence = client.last_server_sequence();
        let reply = exchange(&mut client, actor, &["1", "yes"]).await?;
        let platinum = [initial[0] - 100, initial[1] + 1, initial[2]];
        wait_coins(&mut client, refs, platinum, "gold-to-platinum").await?;
        if !exact_delta(initial, platinum, [-100, 1, 0]) || !reply.lines.iter().any(|line| line == "Here you are.") {
            return Err(format!("step=gold-to-platinum observed=reply:{:?},coins:{platinum:?}", reply.lines).into());
        }
        let after = pairs(&client);
        let after_sequence = client.last_server_sequence();
        let proof = format!("before={before}\nafter={after}\nbefore_sequence={before_sequence}\nafter_sequence={after_sequence}\n");
        TestResult::Ok((client, npc_key, initial, platinum, proof))
    }).await;
    let (mut client, npc_key, initial, platinum, proof) = exchange_result?;
    let old_session = client.join_snapshot().game_session_id;
    let session_id = uuid_text(old_session.as_bytes());
    let sequence = client.last_server_sequence();
    let frames = frames.ok_or("E02 capture overflow")?;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=client-capture-logout observed=refused".into());
    }
    drop(client);

    let mut fresh = connect_world(&dedicated, 0x33).await?;
    let persisted = backpack_coins(&fresh, refs)?;
    let reused = fresh.join_snapshot().game_session_id == old_session;
    if reused || persisted != platinum {
        return Err(format!(
            "step=client-capture-relogin observed=session-reused:{reused},coins:{persisted:?}"
        )
        .into());
    }
    if fresh.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=client-capture-final-logout observed=refused".into());
    }
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = Path::new(&root);
        std::fs::write(root.join("e02-server.frames"), frames)?;
        std::fs::write(
            root.join("e02-expected.txt"),
            format!("session={session_id}\n{proof}"),
        )?;
    }
    Ok(format!(
        "npc={npc_key} before={initial:?} after={platinum:?} relog={persisted:?} session={session_id} sequence={sequence}"
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(150), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=E02.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=E02.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=E02.client status=FAIL evidence=timeout"),
    }
}
