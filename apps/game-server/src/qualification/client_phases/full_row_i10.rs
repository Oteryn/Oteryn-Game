//! I10 concrete player scenario, derived from the matrix/audit and the item-action owners:
//! 1. Admit the node-boot character and retain its complete native inventory view.
//! 2. Send HOUSE_FURNITURE_WRAP `Wrap` for carried/house furniture; expect an accepted reply
//!    with a committed revision and an inventory/house delta replacing it with its kit.
//! 3. Send `Unwrap` for that construction kit at a free house cell; expect an accepted reply,
//!    a later revision, and a delta moving the constructed furniture into house custody.
//! 4. Use a sword on the exact authored outdoor bars and assert the resulting map revision.
//! 5. Use an ordinarily held fluid container on a fluid source and then on the constructed
//!    furniture/map target; expect committed USE replies and subtype/field deltas for fill/pour.
//! 6. Repeat wrap with a stale revision; expect a `Stale` refusal and no inventory delta.
//! 7. Log out, freshly admit the character, and re-check the complete durable inventory view.
//!
//! The fixture isolates acquisition with a dedicated character and an ended authored auction.
//! The live session drives all furniture and fluid actions and reads durable state after relog.
use super::*;
use oteryn_dev_client::client_runtime::{item_use, world_interaction::Target};
use oteryn_dev_client::{DevClientSession, ItemHandle};
use oteryn_protocol_oteryn::{CommandStatus, house_wrap as wrap, world_object};

/// The refusal is the server's correct answer outside a house, so the row fails on the missing
/// precondition: House ownership is only settled by the Control-side auction settlement, which
/// no player command reaches during the run.
const GAP: &str = "step=wrap-observed=NotInHouse;needed=owned-house;settlement-only:crates/gameplay/durability/src/durability/house_ownership/settlement.rs:32";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(180), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "lifecycle-and-fresh-admission-validated".to_owned()),
        Ok(Err(error)) => classify(&error.to_string()),
        Err(_) => ("FAIL", "step=journey observed=timeout".to_owned()),
    };
    if status == "FAIL" {
        evidence(&format!("I10_GAP {detail}"));
    }
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=I10.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn classify(error: &str) -> (&'static str, String) {
    if error.contains("NotInHouse") {
        ("FAIL", GAP.to_owned())
    } else {
        ("FAIL", compact(error))
    }
}

fn compact(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

fn validate_use_reply(
    step: &str,
    status: CommandStatus,
    disposition: world_object::UseDisposition,
) -> TestResult {
    if status != CommandStatus::Accepted || disposition != world_object::UseDisposition::Committed {
        return Err(format!(
            "step={step} observed=status:{status:?},disposition:{disposition:?} needed=accepted-committed-with-state-delta"
        )
        .into());
    }
    Ok(())
}

const HOUSE: &str = "oteryn:content.house.upper_swamp_lane_2";
const AUCTION: &str = "01934f10-7c04-7024-805b-3b1122334491";

async fn seed_auction(clients: &SeamClients<'_>) -> TestResult {
    let catalogue: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../content/houses/houses-00500-00994.json"
    ))?;
    let house = catalogue["houses"]
        .as_array()
        .ok_or("I10 house fixture catalogue")?
        .iter()
        .find(|house| house["identity"]["key"] == HOUSE)
        .ok_or("I10 house fixture authored property missing")?;
    let rent = house["rent_gold"]
        .as_i64()
        .ok_or("I10 house fixture authored rent missing")?;
    if house["kind"] != "private_house" || rent <= 0 {
        return Err("I10 house fixture requires an authored private property with rent".into());
    }
    let world = uuid_text(&clients.world);
    let channel = uuid_text(&clients.channel);
    let character = uuid_text(&clients.characters[0]);
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let mut tx = sqlx::Connection::begin(&mut database).await?;
    let property = sqlx::query(
        "INSERT INTO game_house_properties(world_id,house_key,kind,state,rent_gold,content_digest)
         SELECT world_id,$3,'private_house',2,$4,server_artifact_digest
         FROM game_content_activations WHERE world_id=$1::text::uuid AND channel_id=$2::text::uuid
         ORDER BY activation_sequence DESC LIMIT 1",
    )
    .bind(&world)
    .bind(&channel)
    .bind(HOUSE)
    .bind(rent)
    .execute(&mut *tx)
    .await?;
    if property.rows_affected() != 1 {
        return Err("I10 house fixture active content missing".into());
    }
    sqlx::query("INSERT INTO game_house_ownership VALUES($1::text::uuid,$2,NULL,1,game_character_uuid_v7())")
        .bind(&world).bind(HOUSE).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_house_auctions(auction_id,world_id,house_key,first_rent,ends_at,next_order)
                 VALUES($1::text::uuid,$2::text::uuid,$3,$4,clock_timestamp()-interval '1 second',2)")
        .bind(AUCTION).bind(&world).bind(HOUSE).bind(rent).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_house_bids VALUES($1::text::uuid,$2::text::uuid,$3::text::uuid,$4::text::uuid,1,$5,1,1,$5+1)")
        .bind(AUCTION).bind(&clients.accounts[0]).bind(&world).bind(&character).bind(rent).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn prepare(clients: &SeamClients<'_>) -> TestResult<DevClientSession> {
    character_prep::declare_seeded(
        "I10.full",
        "level 20, settled Upper Swamp Lane 2 auction, construction kit and tools",
        "isolate the furniture and fluid actions from house auction and item acquisition",
    );
    let client = character_prep::Seeded {
        profile: "house-i10-build",
        reason: "authored house requires level 20",
    }
    .connect(clients, 0xb0, 0)
    .await?;
    let vitals = client.actor_vitals().ok_or("I10 fixture vitals missing")?;
    let (health, mana, _) = oteryn_gameplay_characters::vitals::maxima(
        "sorcerer",
        20,
        oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
        oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
    )
    .ok_or("I10 fixture has no source vitals")?;
    if (vitals.max_health, vitals.max_mana) != (health, mana) {
        return Err("step=fixture-admission observed=incorrect-level-20-vitals".into());
    }
    seed_auction(clients).await?;
    request_fixture_control(
        uuid_text(client.join_snapshot().game_session_id.as_bytes()),
        "--profile",
        "house-i10".into(),
    )
    .await?;
    character_prep::grant_item(&client, 2775, 1).await?;
    evidence("I10_FIXTURE request=item-3264-1");
    character_prep::grant_item(&client, 3264, 1).await?;
    character_prep::grant_item(&client, 2874, 1).await?;
    Ok(client)
}

async fn catalogue(client: &mut DevClientSession) -> TestResult<wrap::Catalogue> {
    let payload = wrap::encode_catalogue_query(0).ok_or("I10 catalogue encoding")?;
    let reply = client
        .session_mut()
        .gated_command(
            wrap::CAPABILITY_HOUSE_FURNITURE_WRAP_V1,
            wrap::COMMAND_WRAP,
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!(
            "step=furniture-catalogue status={:?} reply={:?}",
            reply.status,
            wrap::Reply::decode(&reply.payload)
        )
        .into());
    }
    let page =
        wrap::Catalogue::decode(&reply.payload).ok_or("step=furniture-catalogue invalid-body")?;
    if page.page != 0 || page.more {
        return Err("step=furniture-catalogue expected complete page zero".into());
    }
    Ok(page)
}

fn furniture(
    catalogue: &wrap::Catalogue,
    name: &str,
    action: wrap::Action,
) -> TestResult<wrap::Request> {
    let mut matches = catalogue
        .entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.name == name && entry.action == action);
    let (index, _) = matches.next().ok_or_else(|| {
        format!(
            "step=furniture-catalogue missing={name:?} action={action:?} entries={:?}",
            catalogue.entries
        )
    })?;
    if matches.next().is_some() {
        return Err(format!("step=furniture-catalogue ambiguous={name:?}").into());
    }
    let mut request = catalogue
        .request(index)
        .ok_or("I10 furniture catalogue entry")?;
    request.position = [32348, 32254, 7];
    Ok(request)
}

async fn exchange(
    client: &mut DevClientSession,
    request: wrap::Request,
) -> TestResult<(CommandStatus, wrap::Reply)> {
    let reply = client
        .session_mut()
        .gated_command(
            wrap::CAPABILITY_HOUSE_FURNITURE_WRAP_V1,
            wrap::COMMAND_WRAP,
            &request.encode(),
        )
        .await?;
    let decoded = wrap::Reply::decode(&reply.payload).ok_or_else(|| {
        format!(
            "step=furniture-exchange action={:?} status={:?} payload={:?}",
            request.action, reply.status, reply.payload
        )
    })?;
    Ok((reply.status, decoded))
}

async fn committed(client: &mut DevClientSession, request: wrap::Request) -> TestResult<u64> {
    let observed = exchange(client, request).await?;
    match observed {
        (CommandStatus::Accepted, wrap::Reply::Committed(revision))
            if request.expected_revision.checked_add(1) == Some(revision) =>
        {
            Ok(revision)
        }
        _ => Err(format!(
            "step=furniture-commit action={:?} observed={observed:?}",
            request.action
        )
        .into()),
    }
}

fn reference(id: u32) -> TestResult<std::num::NonZeroU32> {
    provisioning::item_reference(&provisioning::item_index()?, id)
}

fn inventory_item(
    client: &DevClientSession,
    id: u32,
) -> TestResult<Option<oteryn_protocol_oteryn::item_view::ItemEntry>> {
    let reference = reference(id)?;
    let inventory = client
        .inventory()
        .ok_or("step=inventory observed=no-view")?;
    let mut entries = inventory
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref == reference);
    let entry = entries.next().copied();
    if entries.next().is_some() || entry.is_some_and(|entry| entry.count.get() != 1) {
        return Err(format!("step=inventory expected at most one item id={id}").into());
    }
    Ok(entry)
}

const BARS_START: TilePos = TilePos {
    x: 33124,
    y: 32151,
    floor: -8,
};
const BARS_TARGET: TilePos = TilePos {
    x: 33123,
    y: 32151,
    floor: -8,
};
const BARS_POSITION: oteryn_protocol_oteryn::world_spatial::ActorPosition =
    oteryn_protocol_oteryn::world_spatial::ActorPosition {
        x: 33123,
        y: 32151,
        floor: 8,
    };

async fn observe_tile(
    client: &mut DevClientSession,
    position: oteryn_protocol_oteryn::world_spatial::ActorPosition,
    after: u64,
) -> TestResult<oteryn_protocol_oteryn::world_map::MapTile> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        for event in client.take_events() {
            use oteryn_dev_client::SessionEvent;
            let (sequence, tiles) = match event {
                SessionEvent::WorldMap(delta) => (delta.server_sequence, delta.value.tiles),
                SessionEvent::WorldMapSnapshot(snapshot) => {
                    (snapshot.server_sequence, snapshot.value.tiles)
                }
                _ => continue,
            };
            if sequence > after
                && let Some(tile) = tiles.into_iter().find(|tile| tile.position == position)
            {
                return Ok(tile);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step=observe-map missing tile={position:?} after={after}").into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
}

async fn use_on(
    client: &mut DevClientSession,
    step: &str,
    source: ItemHandle,
    target: Target,
) -> TestResult<u64> {
    let payload = item_use::use_with_payload(source, &target)?;
    let reply = client
        .session_mut()
        .world_context_use(&payload)
        .await
        .map_err(|error| format!("step={step} exchange={error}"))?;
    let disposition = world_object::decode_use_result(&reply.payload)
        .map_err(|error| format!("step={step} observed=reply-decode-{error:?}"))?;
    validate_use_reply(step, reply.status, disposition)?;
    Ok(reply.command_id)
}

fn expected_destroy(
    root: [u8; 32],
    session: &[u8],
    target: &[u8],
    command: u64,
) -> TestResult<bool> {
    use oteryn_simulation_determinism::{
        DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
    };
    use sha2::{Digest, Sha256};
    let digest = Sha256::new()
        .chain_update(b"oteryn.channel.wooden_bars.v1")
        .chain_update(session)
        .chain_update(command.to_be_bytes())
        .chain_update(target)
        .finalize();
    let occurrence = DecisionOccurrenceId::from_bytes(digest[..16].try_into()?);
    let root = GameplayDecisionRoot::from_bytes(root);
    for index in 0..8 {
        let draw = deterministic_decision_u64(
            &root,
            occurrence,
            "channel.wooden_bars.destroy.roll",
            index,
        )
        .map_err(|error| format!("step=destroy oracle={error:?}"))?;
        if draw < u64::MAX - u64::MAX % 80 {
            // Crystal donor sword 3264 has attack 14.
            return Ok(draw % 80 < 14);
        }
    }
    Err("step=destroy exhausted deterministic roll oracle".into())
}

async fn stage_position(clients: &SeamClients<'_>, from: TilePos, to: TilePos) -> TestResult {
    let character = clients
        .characters
        .first()
        .ok_or("I10 dedicated character absent")?;
    evidence(&format!(
        "I10_FIXTURE request=offline-position from={from:?} to={to:?}"
    ));
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let result = sqlx::query(
        "UPDATE game_character_last_positions
         SET tile_x=$2, tile_y=$3, tile_floor=$4, write_sequence=write_sequence+1
         WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($5,'hex')::uuid
           AND tile_x=$6 AND tile_y=$7 AND tile_floor=$8 AND house_key IS NULL
           AND NOT EXISTS (
               SELECT 1 FROM game_durability_reconnect_sessions
               WHERE character_id=encode($1,'hex')::uuid AND session_state IN (1,2)
           )",
    )
    .bind(character.as_slice())
    .bind(i32::from(to.x))
    .bind(i32::from(to.y))
    .bind(-i16::from(to.floor))
    .bind(clients.world.as_slice())
    .bind(i32::from(from.x))
    .bind(i32::from(from.y))
    .bind(-i16::from(from.floor))
    .execute(&mut db)
    .await?;
    db.close().await?;
    if result.rows_affected() != 1 {
        return Err("step=offline-position observed=no-matching-saved-position".into());
    }
    Ok(())
}

fn bar_state(client: &mut DevClientSession) -> TestResult<(u16, u64)> {
    use oteryn_protocol_oteryn::world_map::MapOrigin;
    let snapshot = client
        .session_mut()
        .world_map_snapshot()
        .ok_or("I10 map view absent")?;
    let tile = snapshot
        .tiles
        .iter()
        .find(|tile| tile.position == BARS_POSITION)
        .ok_or("I10 authored bar tile absent from client map view")?;
    let bar = tile
        .items
        .iter()
        .find_map(|item| match item.origin {
            MapOrigin::BaseOrdinal {
                ordinal: 1,
                object_revision,
            } => Some((item.appearance_id, object_revision)),
            _ => None,
        })
        .ok_or("I10 exact bar ordinal absent")?;
    Ok(bar)
}

async fn destroy(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
) -> TestResult<(u16, u64)> {
    let at = tile(client.world_spatial().actor_position)?;
    if at != BARS_START || bar_state(client)? != (2295, 0) {
        return Err(format!("step=destroy observed=wrong-initial-map-state at={at:?}").into());
    }
    let sword = inventory_item(client, 3264)?.ok_or("step=destroy setup missing sword")?;
    let key = crate::map::view::placement_key(BARS_TARGET, 1).ok_or("I10 bar placement")?;
    let payload = item_use::use_with_payload(sword.handle, &object_at(world, BARS_TARGET, 1)?)?;
    let capabilities = client.selected_capabilities();
    let decoded = world_object::decode_use_intent_target(capabilities, &payload)
        .map_err(|error| format!("step=destroy decoded-request={error:?}"))?;
    evidence(&format!(
        "I10_DESTROY request={decoded:?} capabilities={capabilities:?} target={BARS_TARGET:?} ordinal=1 expected_revision=0 map_pin={:02x?}",
        world.base().digest()
    ));
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let root: Vec<u8> = sqlx::query_scalar(
        "SELECT server_artifact_digest FROM game_content_activations
         WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid
         ORDER BY activation_sequence DESC LIMIT 1",
    )
    .bind(clients.world.as_slice())
    .bind(clients.channel.as_slice())
    .fetch_one(&mut db)
    .await?;
    db.close().await?;
    let after = client.last_server_sequence();
    let session = client.join_snapshot().game_session_id;
    let reply = client.session_mut().world_context_use(&payload).await?;
    let disposition = world_object::decode_use_result(&reply.payload);
    evidence(&format!(
        "I10_DESTROY reply={reply:?} disposition={disposition:?}"
    ));
    let disposition =
        disposition.map_err(|error| format!("step=destroy reply-decode={error:?}"))?;
    validate_use_reply("destroy", reply.status, disposition)?;
    let succeeds = expected_destroy(
        root.as_slice().try_into()?,
        session.as_bytes(),
        &key.to_be_bytes(),
        reply.command_id,
    )?;
    if succeeds {
        observe_tile(client, BARS_POSITION, after).await?;
    }
    let observed = bar_state(client)?;
    let expected = if succeeds { (3145, 1) } else { (2295, 0) };
    if observed != expected {
        return Err(format!("step=destroy observed={observed:?} expected={expected:?}").into());
    }
    evidence(&format!(
        "I10_CHECK step=wooden-bars expected={expected:?} observed={observed:?}"
    ));
    Ok(expected)
}

fn has_map_item(
    tile: &oteryn_protocol_oteryn::world_map::MapTile,
    id: u32,
    sub_type: u32,
) -> TestResult<bool> {
    use oteryn_protocol_oteryn::world_map::MapDefinition;
    let definition = MapDefinition::Item(reference(id)?);
    Ok(tile
        .items
        .iter()
        .any(|item| item.definition == definition && item.sub_type == sub_type))
}

fn object_at(world: &BundleWorld, tile: TilePos, ordinal: u8) -> TestResult<Target> {
    let key = crate::map::view::placement_key(tile, ordinal).ok_or("I10 authored placement")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(Target::Object(world_object::WorldObjectTarget {
        placement,
        expected_revision: 0,
    }))
}

async fn fluid(client: &mut DevClientSession, clients: &SeamClients<'_>) -> TestResult {
    use crate::map::view::MapFacts;
    use oteryn_protocol_oteryn::world_map::MapDefinition;
    let world = full_e2ef7::qualification_world(clients)?;
    let fountain = MapDefinition::Item(reference(1922)?);
    let pos = tile(client.world_spatial().actor_position)?;
    let facts = world.facts().pinned();
    let (source, source_ordinal) = world
        .base()
        .tiles()
        .filter(|(floor, x, y, _)| {
            *floor == pos.floor && x.abs_diff(pos.x).max(y.abs_diff(pos.y)) <= 64
        })
        .filter_map(|(floor, x, y, view)| {
            let tile = TilePos { x, y, floor };
            view.ids()
                .iter()
                .zip(view.depths())
                .filter(|(_, depth)| **depth == 0)
                .enumerate()
                .find_map(|(ordinal, (id, _))| {
                    let ordinal = u8::try_from(ordinal).ok()?;
                    (facts.base_entry(tile, ordinal, *id)?.definition == fountain)
                        .then_some((tile, ordinal))
                })
        })
        .min_by_key(|(tile, ordinal)| {
            (
                tile.x.abs_diff(pos.x).max(tile.y.abs_diff(pos.y)),
                tile.y,
                tile.x,
                *ordinal,
            )
        })
        .ok_or("step=fluid source fountain 1922 not authored near the house")?;
    let there = [(-1_i16, 0_i16), (0, -1), (0, 1), (1, 0)]
        .into_iter()
        .map(|(dx, dy)| TilePos {
            x: source.x.saturating_add_signed(dx),
            y: source.y.saturating_add_signed(dy),
            floor: source.floor,
        })
        .filter(|tile| world.enterable(*tile))
        .min_by_key(|tile| {
            (
                tile.x.abs_diff(pos.x) + tile.y.abs_diff(pos.y),
                tile.y,
                tile.x,
            )
        })
        .ok_or("step=fluid no authored staging tile at fountain 1922")?;
    client_row_i10::walk_to(client, world, there).await?;
    let (pour_ordinal, _) = world
        .base()
        .tile(there.x, there.y, there.floor)
        .and_then(|tile| tile.ground())
        .ok_or("step=fluid no ground at the staging tile")?;
    let vial = inventory_item(client, 2874)?.ok_or("step=fluid no vial")?;
    if vial.sub_type != 0 {
        return Err("step=fluid vial started filled".into());
    }
    use_on(
        client,
        "fluid-fill",
        vial.handle,
        object_at(world, source, source_ordinal)?,
    )
    .await?;
    client.service_liveness(Duration::from_millis(100)).await?;
    let full = inventory_item(client, 2874)?.ok_or("step=fluid fill lost vial")?;
    if full.sub_type != 1 {
        return Err(format!("step=fluid-fill observed-subtype={}", full.sub_type).into());
    }
    client.take_events();
    let sequence = client.last_server_sequence();
    use_on(
        client,
        "fluid-pour",
        full.handle,
        object_at(world, there, pour_ordinal)?,
    )
    .await?;
    let pour_position = client.world_spatial().actor_position;
    let spilled = observe_tile(client, pour_position, sequence).await?;
    if inventory_item(client, 2874)?.is_none_or(|vial| vial.sub_type != 0)
        || !has_map_item(&spilled, 2886, 1)?
    {
        return Err("step=fluid-pour expected empty vial and water splash".into());
    }
    evidence("I10_CHECK step=fluid-fill-and-pour status=PASS");
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let account = character_prep::dedicated_character(clients, "NODE_BOOT_I10_ACCOUNT").await?;
    let clients = &account.clients(clients);
    let mut prepared = prepare(clients).await?;
    let town = tile(prepared.world_spatial().actor_position)?;
    if prepared.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=fixture-logout observed=refused".into());
    }
    drop(prepared);
    stage_position(clients, town, BARS_START).await?;
    let mut bars = connect_character(clients, 0xb0, 0).await?;
    let expected = destroy(&mut bars, clients, world).await?;
    let prior_session = bars.join_snapshot().game_session_id;
    client_row_i10::logout(&mut bars).await?;
    drop(bars);
    let mut bars = connect_character(clients, 0xb4, 0).await?;
    if bars.join_snapshot().game_session_id == prior_session || bar_state(&mut bars)? != expected {
        return Err("step=destroy-relog observed=incorrect-bar-state".into());
    }
    client_row_i10::logout(&mut bars).await?;
    drop(bars);
    stage_position(clients, BARS_START, town).await?;
    let mut client = connect_character(clients, 0xb5, 0).await?;
    let initial = client
        .inventory()
        .cloned()
        .ok_or("step=admit observed=no-inventory")?;
    if inventory_item(&client, 2775)?.is_none() {
        return Err("step=admit observed=no-construction-kit".into());
    }
    client_row_i10::enter_house(&mut client, world).await?;
    let install = furniture(
        &catalogue(&mut client).await?,
        "red cushioned chair kit",
        wrap::Action::Unwrap,
    )?;
    let install_revision = committed(&mut client, install).await?;
    let to_wrap = furniture(
        &catalogue(&mut client).await?,
        "red cushioned chair",
        wrap::Action::Wrap,
    )?;
    if to_wrap.item != install.item
        || to_wrap.expected_revision != install_revision
        || inventory_item(&client, 2775)?.is_some()
        || client.inventory() == Some(&initial)
    {
        return Err("step=construction observed=incorrect-custody-or-revision".into());
    }
    let wrap_revision = committed(&mut client, to_wrap).await?;
    let to_unwrap = furniture(
        &catalogue(&mut client).await?,
        "red cushioned chair kit",
        wrap::Action::Unwrap,
    )?;
    if to_unwrap.item != install.item
        || to_unwrap.expected_revision != wrap_revision
        || inventory_item(&client, 23398)?.is_none()
    {
        return Err("step=wrap observed=incorrect-kit-or-revision".into());
    }
    let final_revision = committed(&mut client, to_unwrap).await?;
    let installed = catalogue(&mut client).await?;
    let restored = furniture(&installed, "red cushioned chair", wrap::Action::Wrap)?;
    if restored.item != install.item
        || restored.expected_revision != final_revision
        || inventory_item(&client, 23398)?.is_some()
    {
        return Err("step=unwrap observed=incorrect-custody-or-revision".into());
    }
    let before_refusal = client
        .inventory()
        .cloned()
        .ok_or("step=stale observed=no-inventory")?;
    let refused = exchange(&mut client, to_wrap).await?;
    if refused
        != (
            CommandStatus::Rejected,
            wrap::Reply::Refused(wrap::Refusal::Stale),
        )
        || catalogue(&mut client).await? != installed
        || client.inventory() != Some(&before_refusal)
    {
        return Err(format!("step=stale observed={refused:?} or state changed").into());
    }
    evidence("I10_CHECK step=furniture-cycle status=PASS");
    client_row_i10::exit_house(&mut client).await?;
    fluid(&mut client, clients).await?;
    let before_logout = client
        .inventory()
        .cloned()
        .ok_or("step=logout observed=no-inventory")?;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    let mut fresh = connect_character(clients, 0xb1, 0).await?;
    if fresh.inventory() != Some(&before_logout) {
        return Err("step=fresh-admission observed=inventory-state-changed".into());
    }
    client_row_i10::enter_house(&mut fresh, world).await?;
    if furniture(
        &catalogue(&mut fresh).await?,
        "red cushioned chair",
        wrap::Action::Wrap,
    )? != restored
    {
        return Err("step=fresh-admission observed=placed-furniture-changed".into());
    }
    if inventory_item(&fresh, 2874)?.is_none_or(|vial| vial.sub_type != 0) {
        return Err("step=fresh-admission observed=vial-not-empty".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction_gap_preserves_the_specific_missing_player_action() {
        let gap =
            "step=construction-kit missing=ordinary-kit-acquisition-install-and-relog-readback";
        assert_eq!(classify(gap), ("FAIL", compact(gap)));
    }

    #[test]
    fn verdict_depends_on_observed_reply() {
        assert_eq!(
            classify("step=wrap observed=NotInHouse"),
            ("FAIL", GAP.to_owned())
        );
        assert_eq!(classify("step=wrap observed=Blocked").0, "FAIL");
    }

    #[test]
    fn use_steps_require_committed_reply_and_observable_delta() {
        let rejected = validate_use_reply(
            "destroy",
            CommandStatus::Rejected,
            world_object::UseDisposition::Rejected,
        );
        let rejected = rejected.err().map(|error| error.to_string());
        assert!(
            rejected.is_some_and(|error| error.contains("step=destroy observed=status:Rejected"))
        );

        let committed = validate_use_reply(
            "fluid-fill",
            CommandStatus::Accepted,
            world_object::UseDisposition::Committed,
        );
        assert!(committed.is_ok());
    }
}
