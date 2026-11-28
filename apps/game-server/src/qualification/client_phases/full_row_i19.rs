//! Global 11.40 removed shrine gem enchanting and legacy weapon enchanting.
use super::*;
use crate::map::view::MapFacts;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::actor_spell::ActorVitals;
use oteryn_protocol_oteryn::item_view::ItemHandle;
use oteryn_protocol_oteryn::world_map::MapDefinition;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, decode_use_result,
    encode_item_use_intent,
};
use sqlx::Connection;
use std::num::NonZeroU32;

const RUBY: u32 = 3030;
const WEAPON: u32 = 3271;
const ALTAR: u32 = 146;
const ENCHANTED_RUBY: u32 = 676;
const ENCHANTED_WEAPON: u32 = 660;
const SHRINE: TilePos = TilePos {
    x: 32342,
    y: 32269,
    floor: -11,
};
const APPROACH: TilePos = TilePos {
    x: 32343,
    y: 32268,
    floor: -11,
};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(420), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", detail),
        Ok(Err(error)) => ("FAIL", error.to_string()),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    let detail = detail.split_whitespace().collect::<Vec<_>>().join("-");
    evidence(&format!(
        "e2e_all scenario=I19.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn inventory(client: &DevClientSession, refs: [NonZeroU32; 4]) -> TestResult<[u64; 4]> {
    let inventory = client
        .inventory()
        .ok_or("enchanting inventory snapshot absent")?;
    Ok(refs.map(|reference| {
        inventory
            .entries
            .iter()
            .filter(|item| item.item_definition_ref == reference)
            .map(|item| u64::from(item.count.get()))
            .sum()
    }))
}

fn handle(client: &DevClientSession, reference: NonZeroU32) -> TestResult<ItemHandle> {
    client
        .inventory()
        .and_then(|inventory| {
            inventory
                .entries
                .iter()
                .find(|item| item.item_definition_ref == reference)
        })
        .map(|item| item.handle)
        .ok_or_else(|| format!("missing item handle for {reference}").into())
}

fn vitals(client: &DevClientSession) -> TestResult<ActorVitals> {
    client
        .actor_vitals()
        .copied()
        .ok_or_else(|| "enchanting vitals snapshot absent".into())
}

async fn relog(
    mut client: DevClientSession,
    clients: &SeamClients<'_>,
    tag: u8,
) -> TestResult<DevClientSession> {
    let session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    let fresh = connect_character(clients, tag, 0).await?;
    if fresh.join_snapshot().game_session_id == session {
        return Err("enchanting relog reused the previous session".into());
    }
    Ok(fresh)
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.status != CommandStatus::Accepted
        || reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err(format!("enchanting logout refused: {reply:?}").into());
    }
    Ok(())
}

fn shrine_target(world: &BundleWorld, reference: NonZeroU32) -> TestResult<MapUseTarget> {
    let tile = world
        .base()
        .tile(SHRINE.x, SHRINE.y, SHRINE.floor)
        .ok_or("authored fire shrine tile absent")?;
    let ordinal = 1;
    let id = tile
        .ids()
        .iter()
        .zip(tile.depths())
        .filter(|(_, depth)| **depth == 0)
        .nth(usize::from(ordinal))
        .map(|(id, _)| *id)
        .ok_or("authored fire altar layer absent")?;
    if world
        .facts()
        .base_entry(SHRINE, ordinal, id)
        .is_none_or(|entry| entry.definition != MapDefinition::Item(reference))
    {
        return Err("authored fire altar 146 absent from its shrine layer".into());
    }
    let key = crate::map::view::placement_key(SHRINE, ordinal)
        .ok_or("fire altar placement key absent")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(MapUseTarget::Object(WorldObjectTarget {
        placement,
        expected_revision: 0,
    }))
}

// Placement is a logged fixture input. The test still sends the real use-with
// command from the exact adjacent tile and observes the server disposition.
async fn place_at_shrine(clients: &SeamClients<'_>) -> TestResult {
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let updated = sqlx::query(
        "UPDATE game_character_last_positions
         SET tile_x=$1, tile_y=$2, tile_floor=$3, write_sequence=write_sequence+1
         WHERE character_id=$4::uuid AND world_id=$5::uuid",
    )
    .bind(i32::from(APPROACH.x))
    .bind(i32::from(APPROACH.y))
    .bind(-i16::from(APPROACH.floor))
    .bind(uuid_text(&clients.characters[0]))
    .bind(uuid_text(&clients.world))
    .execute(&mut db)
    .await?;
    if updated.rows_affected() != 1 {
        return Err("offline shrine placement did not update one character".into());
    }
    Ok(())
}

async fn require_refusal(
    client: &mut DevClientSession,
    source: ItemHandle,
    target: MapUseTarget,
    label: &str,
) -> TestResult {
    let items = client.inventory().cloned().ok_or("inventory absent")?;
    let before = vitals(client)?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(source),
        use_with: None,
        use_with_map_item: Some(target),
        use_with_position: None,
    })
    .map_err(|error| format!("{label} intent encode: {error:?}"))?;
    let reply = client.session_mut().world_context_use(&payload).await?;
    let disposition = decode_use_result(&reply.payload)
        .map_err(|error| format!("{label} result decode: {error:?}"))?;
    if reply.status != CommandStatus::Accepted
        || disposition != UseDisposition::NothingToUse
        || client.inventory() != Some(&items)
        || vitals(client)? != before
    {
        return Err(format!(
            "{label} status={:?} disposition={disposition:?} inventory_unchanged={} vitals_unchanged={}",
            reply.status, client.inventory() == Some(&items), vitals(client)? == before
        ).into());
    }
    evidence(&format!(
        "I19_CHECK {label} PASS command={} disposition=NothingToUse",
        reply.command_id
    ));
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let world = full_e2ef7::qualification_world(clients)?;
    let index = provisioning::item_index()?;
    let reference = |id| provisioning::item_reference(&index, id);
    let refs = [
        reference(RUBY)?,
        reference(WEAPON)?,
        reference(ENCHANTED_RUBY)?,
        reference(ENCHANTED_WEAPON)?,
    ];
    let shrine = shrine_target(world, reference(ALTAR)?)?;
    let identity = character_prep::dedicated_named_character(
        clients,
        "NODE_BOOT_I19_ACCOUNT",
        Some("Fire Shrine Tester"),
    )
    .await?;
    let scoped = identity.clients(clients);
    let mut client = connect_character(&scoped, 0xdf, 0).await?;
    character_prep::declare_seeded(
        "I19.full",
        "sorcerer-level-32,ruby-3030,enchanted-ruby-676,weapon-3271,offline-shrine-placement",
        "legacy shrine and weapon use-with refusals are under test",
    );
    request_fixture_control(
        uuid_text(client.join_snapshot().game_session_id.as_bytes()),
        "--profile",
        "i19-sorcerer".into(),
    )
    .await?;
    character_prep::grant_item(&client, RUBY, 1).await?;
    character_prep::grant_item(&client, WEAPON, 1).await?;
    character_prep::grant_item(&client, ENCHANTED_RUBY, 1).await?;
    let session = client.join_snapshot().game_session_id;
    logout(&mut client).await?;
    drop(client);
    wait_character_released(&scoped, 0).await?;
    place_at_shrine(&scoped).await?;
    let mut client = connect_character(&scoped, 0xdf, 0).await?;
    if client.join_snapshot().game_session_id == session
        || tile(client.world_spatial().actor_position)? != APPROACH
    {
        return Err("shrine fixture did not admit a new session at the exact approach".into());
    }

    let initial = vitals(&client)?;
    if (initial.max_health, initial.max_mana, initial.soul) != (305, 810, 100)
        || initial.mana < 300
        || inventory(&client, refs)? != [1, 1, 1, 0]
    {
        return Err(format!(
            "enchanting fixture state vitals={initial:?} items={:?}",
            inventory(&client, refs)?
        )
        .into());
    }
    let before_refusal = client.inventory().cloned();
    let refused = client.use_item(handle(&client, refs[0])?).await?;
    if refused.status != CommandStatus::Accepted
        || refused.disposition != UseDisposition::NothingToUse
        || client.inventory() != before_refusal.as_ref()
        || vitals(&client)? != initial
    {
        return Err(format!(
            "ruby without altar changed state or returned the wrong refusal: {refused:?}"
        )
        .into());
    }
    evidence("I19_CHECK no-altar PASS ruby-retained=1");

    let ruby = handle(&client, refs[0])?;
    require_refusal(&mut client, ruby, shrine, "legacy-shrine").await?;
    let ruby = handle(&client, refs[2])?;
    let weapon = handle(&client, refs[1])?;
    require_refusal(
        &mut client,
        ruby,
        MapUseTarget::Item(weapon),
        "legacy-weapon",
    )
    .await?;

    let fresh = relog(client, &scoped, 0xdf).await?;
    if inventory(&fresh, refs)? != [1, 1, 1, 0]
        || vitals(&fresh)? != initial
        || tile(fresh.world_spatial().actor_position)? != APPROACH
    {
        return Err("legacy use changed durable inventory, vitals or position".into());
    }
    Ok("global-11.40 legacy-shrine=NothingToUse legacy-weapon=NothingToUse durable-inputs=unchanged".into())
}
