//! W23 — Town citizenship / home temple.
//!
//! Player scenario derived from the W23 matrix/audit row and the movement owner:
//! 1. Log in an ordinarily provisioned node-boot character, walk to a reachable non-Svargrond
//!    citizen tile, and step onto it; expect a moved STEP reply,
//!    spatial delta, temple teleport, and the town-specific citizenship confirmation from Game.
//! 2. Log out with the native logout command, obtain a fresh admission, and verify that its
//!    authoritative initial position is the selected town's temple (durable home-town state).
//! 3. Pay Captain Bluebear and walk to the shipped Svargrond citizen tile (unique 9066).
//! 4. Step onto that tile without Barbarian Test progress; expect a moved STEP reply and spatial
//!    delta followed by the exact Game refusal and a teleport to the Svargrond temple
//!    (Crystal 201189f8 citizen.lua:39-43). The saved home town remains the prior town.
//! 5. Seed the completed Barbarian Test as a prerequisite, then repeat the same native STEP
//!    and verify the successful Svargrond citizenship and durable home town.

use super::*;
use oteryn_dev_client::{ChatLine, DevClientSession, EntityKind, EntityRef, SessionEvent};
use oteryn_protocol_oteryn::npc_service as wire;

const SVARGROND_UNIQUE: u16 = 9066;
const ACCOUNT: &str = "01934f10-7c00-7000-8000-000000000022";
const CAPTAIN: &str = "oteryn:npc.captain_bluebear";
const REFUSAL: &str = "You first need to absolve the Barbarian Test Quest to become citizen!";

#[derive(Clone, Copy, Debug)]
struct CitizenTile {
    at: TilePos,
    unique: u16,
    town: u32,
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients))).await;
    let result = match result {
        Ok(result) => result,
        Err(_) => Err("step=journey observed=timeout-after-240s need=reachable-citizen-route".into()),
    };
    if let Err(error) = &result {
        evidence(&format!("W23_GAP {error}"));
    }
    evidence(&format!(
        "e2e_all scenario=W23.full status={} seconds={:.3}{}",
        if result.is_ok() { "PASS" } else { "FAIL" },
        started.elapsed().as_secs_f64(),
        result
            .err()
            .map(|error| format!(" evidence={}", evidence_token(&error.to_string())))
            .unwrap_or_default(),
    ));
}

fn evidence_token(value: &str) -> String {
    value
        .chars()
        .map(|character| if character.is_ascii_alphanumeric() || "-_:./".contains(character) { character } else { '-' })
        .collect()
}

fn town_for(unique: u16) -> Option<u32> {
    Some(match unique {
        9056 => 6, 9057 => 8, 9058 => 9, 9059 => 5, 9060 => 7, 9061 => 13,
        9062 => 10, 9063 => 11, 9064 => 14, 9065 => 15, 9066 => 16, 9067 => 17,
        9068 => 12, 9240 => 18, 9500 => 21, 9510 => 20, 9515 => 22,
        _ => return None,
    })
}

fn citizen_tiles(bytes: &[u8]) -> TestResult<Vec<CitizenTile>> {
    let mut found = Vec::new();
    oteryn_world_bundle::bundle::visit(bytes, |_, sector| {
        for tile in sector.tiles {
            for item in tile.items.iter().filter(|item| item.depth == 0) {
                if let Some(unique) = item.attrs.unique
                    && let Some(town) = town_for(unique)
                {
                    found.push(CitizenTile {
                        at: TilePos { x: tile.x, y: tile.y, floor: sector.floor },
                        unique,
                        town,
                    });
                }
            }
        }
        Ok(())
    })?;
    found.sort_by_key(|entry| (entry.unique, entry.at));
    Ok(found)
}

fn enters_citizen_tile(
    from: TilePos,
    direction: StepDirection,
    trigger: TilePos,
    landing: Option<TilePos>,
) -> bool {
    let (dx, dy) = direction.delta();
    landing.is_some()
        && from.floor == trigger.floor
        && i32::from(from.x) + dx == i32::from(trigger.x)
        && i32::from(from.y) + dy == i32::from(trigger.y)
}

fn approach(
    world: &BundleWorld,
    from: TilePos,
    target: TilePos,
) -> Option<(Vec<StepDirection>, StepDirection)> {
    shortest_approach(|direction| {
        route(world, from, |at| {
            enters_citizen_tile(at, direction, target, destination(world, at, direction))
        })
    })
}

fn shortest_approach(
    mut route_for: impl FnMut(StepDirection) -> Option<Vec<StepDirection>>,
) -> Option<(Vec<StepDirection>, StepDirection)> {
    [StepDirection::North, StepDirection::East, StepDirection::South, StepDirection::West]
        .into_iter()
        .filter_map(|direction| route_for(direction).map(|path| (path, direction)))
        .min_by_key(|(path, _)| path.len())
}

async fn messages_until(
    client: &mut DevClientSession,
    expected: &str,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    loop {
        for event in client.take_events() {
            if let SessionEvent::ChatLine(delta) = event
                && let ChatLine::Private { speaker_name, text } = delta.value
                && speaker_name == "Game"
                && text == expected
            {
                return Ok(());
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step=server-message observed=timeout expected={expected:?} need=Game-chat-delta").into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

async fn step_onto(
    client: &mut DevClientSession,
    direction: StepDirection,
) -> TestResult<oteryn_dev_client::StepOutcome> {
    let outcome = client.step_retrying(direction).await?;
    if outcome.disposition != StepDisposition::Moved || outcome.world_spatial_delta.is_none() {
        return Err(format!("step=citizen-step observed={outcome:?} need=moved-reply-and-spatial-delta").into());
    }
    Ok(outcome)
}

async fn home_town(database: &mut sqlx::PgConnection, character: &[u8; 16]) -> TestResult<Option<i32>> {
    Ok(sqlx::query_scalar(
        "SELECT town_id FROM game_character_home_towns WHERE character_id=encode($1,'hex')::uuid",
    ).bind(character.as_slice()).fetch_optional(database).await?)
}

async fn accepted_logout(client: &mut DevClientSession) -> TestResult {
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("W23 logout reply={logout:?}").into());
    }
    Ok(())
}

async fn seed_barbarian_test(
    clients: &SeamClients<'_>,
    database: &mut sqlx::PgConnection,
) -> TestResult {
    use sqlx::Connection as _;

    wait_character_released(clients, 0).await?;
    let mut transaction = database.begin().await?;
    // Apply the same missing predecessor fact as the citizenship PostgreSQL
    // fixture. Native movement and citizenship run after this transaction.
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *transaction)
        .await?;
    let written = sqlx::query(
        "INSERT INTO game_character_quest_tracks
           (character_id, track_key, quest_key, value, committed_character_revision)
         SELECT character_id, 'oteryn:quest-progress/storage/41174',
                'oteryn:quest.barbarian_test', 8, character_revision
         FROM game_character_roots
         WHERE character_id=encode($1,'hex')::uuid AND account_id=$2::uuid
           AND world_id=encode($3,'hex')::uuid AND lifecycle=1",
    )
    .bind(clients.characters[0].as_slice())
    .bind(ACCOUNT)
    .bind(clients.world.as_slice())
    .execute(&mut *transaction)
    .await?;
    if written.rows_affected() != 1 {
        return Err("W23 Barbarian Test seed did not match the dedicated character".into());
    }
    transaction.commit().await?;
    Ok(())
}

#[derive(serde::Deserialize)]
struct BoatRoute {
    key: String,
    price: u64,
    destination: BoatDestination,
}

#[derive(serde::Deserialize)]
struct BoatDestination {
    x: i32,
    y: i32,
    floor: i16,
}

fn svargrond_boat() -> TestResult<BoatRoute> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../content/services/travel/travel-00000-00056.json");
    let content: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
    let record = content["records"]
        .as_array()
        .and_then(|records| {
            records
                .iter()
                .map(|record| &record["declaration"])
                .find(|record| {
                    record["identity"]["key"] == "oteryn:service.travel.captain_bluebear"
                })
        })
        .ok_or("step=svargrond-route Captain Bluebear travel service absent")?;
    let routes: Vec<BoatRoute> = serde_json::from_value(record["routes"].clone())?;
    routes
        .into_iter()
        .find(|entry| entry.key == "svargrond")
        .ok_or_else(|| {
            "step=svargrond-route Captain Bluebear has no authored Svargrond route".into()
        })
}

async fn captain_talk(
    client: &mut DevClientSession,
    npc_actor: EntityRef,
    text: &str,
    deadline: tokio::time::Instant,
) -> TestResult<(Vec<String>, bool)> {
    client.take_events();
    let payload = wire::encode_npc_talk_intent(&wire::NpcTalkIntent {
        npc_actor,
        text: text.into(),
    })
    .map_err(|error| format!("Captain Bluebear {text} encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_NPC_SERVICE_V1,
            wire::COMMAND_TYPE_NPC_TALK_INTENT,
            &payload,
        )
        .await?;
    let disposition = wire::decode_npc_intent_result(&reply.payload)
        .map_err(|error| format!("Captain Bluebear {text} reply decode: {error:?}"))?;
    if reply.status != CommandStatus::Accepted || disposition != wire::NpcIntentDisposition::Ok {
        return Err(format!(
            "Captain Bluebear {text} refused: {:?} {disposition:?}",
            reply.status
        )
        .into());
    }
    let mut inventory_changed = false;
    loop {
        let mut lines = None;
        for event in client.take_events() {
            match event {
                SessionEvent::NpcConversation(delta)
                    if delta.server_sequence > reply.server_sequence =>
                {
                    lines = Some(delta.value.lines);
                }
                SessionEvent::Inventory(delta) if delta.server_sequence > reply.server_sequence => {
                    inventory_changed = true;
                }
                _ => {}
            }
        }
        if let Some(lines) = lines {
            return Ok((lines, inventory_changed));
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("Captain Bluebear {text} conversation delta timed out").into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
}

fn carried_gold(client: &DevClientSession) -> TestResult<u64> {
    let index = provisioning::item_index()?;
    let coins = [
        (provisioning::item_reference(&index, 3031)?, 1),
        (provisioning::item_reference(&index, 3035)?, 100),
        (provisioning::item_reference(&index, 3043)?, 10_000),
    ];
    Ok(client
        .inventory()
        .ok_or("fare inventory absent")?
        .entries
        .iter()
        .map(|entry| {
            coins
                .iter()
                .find(|(definition, _)| *definition == entry.item_definition_ref)
                .map_or(0, |(_, value)| u64::from(entry.count.get()) * value)
        })
        .sum())
}

async fn travel_to_svargrond(
    client: &mut DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    boat: BoatRoute,
) -> TestResult {
    let captain = npcs
        .npcs
        .iter()
        .find(|entry| entry.key == CAPTAIN)
        .ok_or("step=svargrond-travel Captain Bluebear absent from the compiled bundle")?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        captain.placements.iter().any(|p| {
            p.floor == at.floor
                && u32::from(at.x.abs_diff(p.x).max(at.y.abs_diff(p.y))) <= wire::TALK_RANGE_TILES
        })
    })
    .ok_or("step=svargrond-travel no STEP route to Captain Bluebear")?;
    walk(client, world, &path).await?;
    let actor = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entity| {
                entity.kind == EntityKind::Npc
                    && captain.placements.iter().any(|p| {
                        tile(entity.position)
                            .is_ok_and(|at| at.x == p.x && at.y == p.y && at.floor == p.floor)
                    })
            })
        })
        .ok_or("step=svargrond-travel Captain Bluebear not visible at his authored position")?
        .entity;
    captain_talk(
        client,
        actor,
        "hi",
        tokio::time::Instant::now() + Duration::from_secs(8),
    )
    .await?;
    let (quote, _) = captain_talk(
        client,
        actor,
        &boat.key,
        tokio::time::Instant::now() + Duration::from_secs(8),
    )
    .await?;
    if !quote
        .iter()
        .any(|line| line.contains("Svargrond") && line.contains(&boat.price.to_string()))
    {
        return Err(
            format!("step=svargrond-travel missing destination and fare quote: {quote:?}").into(),
        );
    }
    let before = carried_gold(client)?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    let (answer, mut inventory_changed) = captain_talk(client, actor, "yes", deadline).await?;
    let arrival = ActorPosition {
        x: boat.destination.x,
        y: boat.destination.y,
        floor: boat.destination.floor,
    };
    while tokio::time::Instant::now() < deadline
        && (!inventory_changed || client.world_spatial().actor_position != arrival)
    {
        client.service_liveness(Duration::from_millis(25)).await?;
        inventory_changed |= client
            .take_events()
            .iter()
            .any(|event| matches!(event, SessionEvent::Inventory(_)));
    }
    if !answer.iter().any(|line| line.contains("Set the sails"))
        || !inventory_changed
        || client.world_spatial().actor_position != arrival
        || carried_gold(client)?.checked_add(boat.price) != Some(before)
    {
        return Err(format!(
            "step=svargrond-travel answer={answer:?} position={:?} expected={arrival:?} fare={}",
            client.world_spatial().actor_position,
            boat.price
        )
        .into());
    }
    evidence(&format!(
        "W23_CHECK paid-svargrond-travel PASS arrival={arrival:?} fare={}",
        boat.price
    ));
    Ok(())
}

async fn approach_svargrond_portal(
    client: &mut DevClientSession,
    world: &BundleWorld,
    portal: TilePos,
) -> TestResult<StepDirection> {
    use oteryn_dev_client::{ChatIntent, ChatSpeechMode};

    let origin = tile(client.world_spatial().actor_position)?;
    if let Some((path, step)) = approach(world, origin, portal) {
        walk(client, world, &path).await?;
        return Ok(step);
    }
    // The citizenship room is above the temple. Step south to face the
    // authored ledge and use normal Levitate from the uncovered tile.
    let facing_step = TilePos { x: 32212, y: 31128, floor: -6 };
    let cast_from = TilePos { y: 31129, ..facing_step };
    let ledge = TilePos { y: 31130, floor: -5, ..facing_step };
    if destination(world, facing_step, StepDirection::South) != Some(cast_from)
        || !world.enterable(ledge)
        || world.base().tile(cast_from.x, cast_from.y, ledge.floor).is_some()
    {
        return Err("step=svargrond-route authored temple ledge geometry was not served".into());
    }
    let (path, step) = approach(world, ledge, portal)
        .ok_or("step=svargrond-route no native approach from temple ledge to portal")?;
    let mut to_ledge = route(world, origin, |at| at == facing_step)
        .ok_or("step=svargrond-route no native STEP route to Levitate approach")?;
    to_ledge.push(StepDirection::South);
    walk(client, world, &to_ledge).await?;
    let mana_before = client.actor_vitals().ok_or("Levitate pre-cast vitals absent")?.mana;
    client.take_events();
    let cast = client.chat(&ChatIntent::Say {
        mode: ChatSpeechMode::Say,
        text: "exani hur up".into(),
    }).await?;
    if cast.status != CommandStatus::Accepted {
        return Err(format!("step=svargrond-route Levitate speech reply={cast:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    loop {
        if tile(client.world_spatial().actor_position)? == ledge
            && client.actor_vitals().is_some_and(|vitals| vitals.mana < mana_before)
        {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step=svargrond-route Levitate did not move to {ledge:?} and debit mana").into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
        client.take_events();
    }
    evidence(&format!("W23_CHECK temple-ledge PASS from={cast_from:?} to={ledge:?}"));
    walk(client, world, &path).await?;
    Ok(step)
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
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let tiles = citizen_tiles(&bytes)?;
    let mut portals = tiles
        .iter()
        .filter(|entry| entry.unique == SVARGROND_UNIQUE);
    let svargrond = portals
        .next()
        .filter(|entry| {
            entry.at
                == (TilePos {
                    x: 32214,
                    y: 31133,
                    floor: -5,
                })
        })
        .ok_or("step=svargrond-content need=UID-9066-at-32214,31133,5-in-W23-profile")?;
    if portals.next().is_some() {
        return Err("step=svargrond-content observed=unique-9066-ambiguous".into());
    }
    let boat = svargrond_boat()?;
    character_prep::declare_seeded(
        "W23",
        "character,boat-fare,levitate-eligibility",
        "dedicated character, travel funds and spell eligibility",
    );
    let identity = character_prep::bootstrap_row_character(
        clients,
        ACCOUNT,
        "01934f10-7c04-7022-805b-3b1122334422",
        "Coastal Citizen",
    )
    .await?;
    let clients = &identity.clients(clients);
    let mut provisioned = character_prep::connect_row_character(clients).await?;
    request_fixture_control(
        uuid_text(provisioned.join_snapshot().game_session_id.as_bytes()),
        "--profile",
        "sorcerer".into(),
    ).await?;
    let coins = u32::try_from(boat.price.div_ceil(100))?;
    character_prep::grant_item(&provisioned, 3035, coins).await?;
    accepted_logout(&mut provisioned).await?;
    drop(provisioned);
    let mut client = character_prep::connect_row_character(clients).await?;

    use sqlx::Connection as _;
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let previous_home = home_town(&mut database, &clients.characters[0]).await?;
    let from = tile(client.world_spatial().actor_position)?;
    let (selected, path, direction) = tiles.iter()
        .filter(|entry| entry.unique != SVARGROND_UNIQUE && previous_home.and_then(|town| u32::try_from(town).ok()) != Some(entry.town))
        .filter_map(|entry| approach(world, from, entry.at).map(|(path, direction)| (*entry, path, direction)))
        .min_by_key(|(_, path, _)| path.len())
        .ok_or("step=citizenship-route observed=no-walking-route need=reachable-shipped-citizen-tile-for-different-home-town")?;
    walk(&mut client, world, &path).await?;
    client.take_events();
    let outcome = step_onto(&mut client, direction).await?;
    let area = world
        .base()
        .subareas
        .town(selected.town)
        .ok_or("step=town-catalogue observed=selected-town-absent need=town-area")?;
    let temple = area
        .temple
        .ok_or("step=town-catalogue observed=temple-absent need=home-temple")?;
    let expected_position = ActorPosition {
        x: i32::from(temple.x),
        y: i32::from(temple.y),
        floor: -i16::from(temple.floor),
    };
    let expected_message = format!("You are now a citizen of {}.", area.name);
    messages_until(&mut client, &expected_message).await?;
    if client.world_spatial().actor_position != expected_position {
        return Err(format!(
            "step=temple-teleport observed={:?} need={expected_position:?}",
            client.world_spatial().actor_position
        )
        .into());
    }
    evidence(&format!(
        "W23_CHECK citizenship PASS town={} reply={outcome:?} temple={expected_position:?}",
        selected.town
    ));

    let logout = client.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=logout observed={logout:?} need=accepted").into());
    }
    drop(client);
    let mut fresh = character_prep::connect_row_character(clients).await?;
    if fresh.world_spatial().actor_position != expected_position {
        return Err(format!(
            "step=fresh-admission observed={:?} need=durable-home-temple-{expected_position:?}",
            fresh.world_spatial().actor_position
        )
        .into());
    }
    let home = home_town(&mut database, &clients.characters[0]).await?;
    if home != Some(i32::try_from(selected.town)?) {
        return Err(format!(
            "step=fresh-admission-home-town observed={home:?} need={}",
            selected.town
        )
        .into());
    }
    evidence(&format!(
        "W23_CHECK persistence PASS town={} fresh={expected_position:?}",
        selected.town
    ));

    travel_to_svargrond(&mut fresh, world, &metadata.npcs, boat).await?;
    let direction = approach_svargrond_portal(&mut fresh, world, svargrond.at).await?;
    let temple = world
        .base()
        .subareas
        .town(svargrond.town)
        .and_then(|area| area.temple)
        .ok_or("step=svargrond-refusal observed=temple-absent need=Svargrond-temple")?;
    let temple = ActorPosition {
        x: i32::from(temple.x),
        y: i32::from(temple.y),
        floor: -i16::from(temple.floor),
    };
    fresh.take_events();
    let outcome = step_onto(&mut fresh, direction).await?;
    messages_until(&mut fresh, REFUSAL).await?;
    if fresh.world_spatial().actor_position != temple {
        return Err(format!(
            "step=svargrond-refusal observed=position-{:?} need=temple-{temple:?}",
            fresh.world_spatial().actor_position
        )
        .into());
    }
    if home_town(&mut database, &clients.characters[0]).await? != home {
        return Err("step=svargrond-refusal changed the durable home town".into());
    }
    evidence(&format!(
        "W23_CHECK refusal PASS reply={outcome:?} position={temple:?}"
    ));
    let logout = fresh.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=final-logout observed={logout:?} need=accepted").into());
    }
    drop(fresh);
    character_prep::declare_seeded(
        "W23",
        "barbarian-test-completed",
        "positive Svargrond citizenship after the live refusal",
    );
    seed_barbarian_test(clients, &mut database).await?;
    let mut qualified = character_prep::connect_row_character(clients).await?;
    let direction = approach_svargrond_portal(&mut qualified, world, svargrond.at).await?;
    qualified.take_events();
    let outcome = step_onto(&mut qualified, direction).await?;
    let area = world
        .base()
        .subareas
        .town(svargrond.town)
        .ok_or("Svargrond town absent")?;
    messages_until(
        &mut qualified,
        &format!("You are now a citizen of {}.", area.name),
    )
    .await?;
    if qualified.world_spatial().actor_position != temple
        || home_town(&mut database, &clients.characters[0]).await?
            != Some(i32::try_from(svargrond.town)?)
    {
        return Err(
            "step=svargrond-qualified missing temple relocation or durable Svargrond home town"
                .into(),
        );
    }
    accepted_logout(&mut qualified).await?;
    evidence(&format!(
        "W23_CHECK svargrond-citizenship PASS reply={outcome:?} town={} position={temple:?}",
        svargrond.town
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn journey_regression_uses_citizen_trigger_even_when_step_lands_at_temple() {
        let from = super::TilePos { x: 10, y: 10, floor: -7 };
        let trigger = super::TilePos { x: 11, ..from };
        let temple = super::TilePos { x: 20, ..from };
        assert!(super::enters_citizen_tile(from, super::StepDirection::East, trigger, Some(temple)));
        assert!(!super::enters_citizen_tile(from, super::StepDirection::East, trigger, None));
        assert!(!super::enters_citizen_tile(from, super::StepDirection::North, trigger, Some(temple)));
    }

    #[test]
    fn journey_regression_uses_the_shortest_citizen_tile_approach() {
        use super::StepDirection::{East, North};
        let picked = super::shortest_approach(|direction| match direction {
            North => Some(vec![North, North]),
            East => Some(vec![East]),
            _ => None,
        });
        assert_eq!(picked, Some((vec![East], East)));
    }

    #[test]
    fn evidence_is_one_report_token() {
        assert_eq!(super::evidence_token("step=x observed=a b"), "step-x-observed-a-b");
    }
}
