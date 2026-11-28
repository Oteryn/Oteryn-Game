//! Capture the creature enter delta produced by using Animate Dead on a hunted Rat.
use super::*;
use oteryn_dev_client::{EntityKind, SessionEvent};
use oteryn_protocol_oteryn::attack::AttackIntentDisposition;

const SELLER: &str = "oteryn:npc.alexander";

async fn place_buyer(clients: &SeamClients<'_>, at: TilePos, purpose: &str) -> TestResult {
    wait_character_released(clients, 1).await?;
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let updated = sqlx::query(
        "UPDATE game_character_last_positions
         SET tile_x=$1, tile_y=$2, tile_floor=$3, write_sequence=write_sequence+1
         WHERE character_id=encode($4,'hex')::uuid AND world_id=encode($5,'hex')::uuid",
    )
    .bind(i32::from(at.x))
    .bind(i32::from(at.y))
    .bind(-i16::from(at.floor))
    .bind(clients.characters[1].as_slice())
    .bind(clients.world.as_slice())
    .execute(&mut db)
    .await?;
    if updated.rows_affected() != 1 {
        return Err("C16c fixture needs a saved position for the released buyer".into());
    }
    evidence(&format!(
        "FIXTURE_PLACEMENT row=C16c purpose={purpose} tile={at:?}"
    ));
    Ok(())
}

async fn buy_rune(
    client: &mut oteryn_dev_client::DevClientSession,
    seller: TilePos,
) -> TestResult<std::num::NonZeroU32> {
    use oteryn_dev_client::client_runtime::npc_shop as runtime;
    use oteryn_protocol_oteryn::npc_service as wire;
    let index = provisioning::item_index()?;
    let rune = provisioning::item_reference(&index, 3203)?;
    let actor = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entity| {
                entity.kind == EntityKind::Npc && tile(entity.position).ok() == Some(seller)
            })
        })
        .map(|entity| entity.entity)
        .ok_or("C16c Alexander absent from the admitted buyer's view")?;
    for text in ["hi", "trade"] {
        let reply = runtime::talk(
            client.session_mut(),
            &wire::NpcTalkIntent {
                npc_actor: actor,
                text: text.into(),
            },
        )
        .await?;
        if reply != wire::NpcIntentDisposition::Ok {
            return Err(format!("C16c Alexander {text} refused: {reply:?}").into());
        }
    }
    let delta = client
        .session_mut()
        .read_gated_delta(wire::STATE_DOMAIN_NPC_TRADE_WINDOW, 1)
        .await?;
    let window = wire::decode_npc_trade_window(&delta.payload)
        .map_err(|error| format!("C16c trade window decode: {error:?}"))?;
    let (offer_index, offer) = window
        .offers
        .iter()
        .enumerate()
        .find(|(_, offer)| {
            offer.side == wire::NpcTradeSide::Buy && offer.item_definition_ref == rune.get()
        })
        .ok_or("C16c Alexander did not offer rune 3203")?;
    if offer.unit_price != 375 {
        return Err(format!(
            "C16c rune 3203 expected price 375, got {}",
            offer.unit_price
        )
        .into());
    }
    let balances = runtime::balances(client.session_mut()).await?;
    let reply = runtime::buy(
        client.session_mut(),
        &wire::NpcTradeIntent {
            npc_actor: actor,
            catalogue_revision: window.open.ok_or("C16c trade window closed")?.1,
            offer_index: u32::try_from(offer_index)?,
            side: offer.side,
            quantity: 1,
            expected_unit_price: offer.unit_price,
        },
    )
    .await?;
    if reply != wire::NpcIntentDisposition::Ok {
        return Err(format!(
            "C16c first purchase refused: {reply:?}; price={} balances={balances:?}",
            offer.unit_price
        )
        .into());
    }
    client.service_liveness(Duration::from_millis(500)).await?;
    if !client.inventory().is_some_and(|inventory| {
        inventory
            .entries
            .iter()
            .any(|item| item.item_definition_ref == rune && item.count.get() > 0)
    }) {
        return Err("C16c accepted purchase did not deliver rune 3203".into());
    }
    Ok(rune)
}

async fn journey(clients: &SeamClients<'_>, world: &BundleWorld, directory: &Path) -> TestResult {
    let spell = native_spell_book()?
        .rune(3203)
        .ok_or("Animate Dead rune absent")?;
    let client = connect_world(clients, 0xd5).await?;
    let mut client =
        provisioning::provision_profile(client, clients, 1, 100, "c16c-sorcerer").await?;
    evidence("FIXTURE_INPUT row=C16c magic_level=4 platinum_coins=4");
    let bundle = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bundle, |_, _| Ok(()))?;
    let seller = metadata
        .npcs
        .npcs
        .iter()
        .find(|npc| npc.key == SELLER)
        .and_then(|npc| npc.placements.first())
        .ok_or("C16c Alexander absent from the live world bundle")?;
    let seller = TilePos {
        x: seller.x,
        y: seller.y,
        floor: seller.floor,
    };
    let at = [(0, -1), (1, 0), (0, 1), (-1, 0)]
        .into_iter()
        .filter_map(|(dx, dy)| {
            Some(TilePos {
                x: seller.x.checked_add_signed(dx)?,
                y: seller.y.checked_add_signed(dy)?,
                floor: seller.floor,
            })
        })
        .find(|at| world.enterable(*at))
        .ok_or("C16c Alexander has no walkable adjacent tile")?;
    let origin = tile(client.world_spatial().actor_position)?;
    classa3_observation::logout(&mut client).await?;
    drop(client);
    place_buyer(clients, at, SELLER).await?;
    let mut client = connect_world(clients, 0xd5).await?;
    if tile(client.world_spatial().actor_position)? != at {
        return Err("C16c admission did not restore the buyer beside Alexander".into());
    }
    let rune = buy_rune(&mut client, seller).await?;
    classa3_observation::logout(&mut client).await?;
    drop(client);
    place_buyer(clients, origin, "hunt-start").await?;
    let mut client = connect_world(clients, 0xd5).await?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone())
    })
    .ok_or("C16c has no route to a hunt tile outside protection")?;
    classa3_observation::walk(&mut client, world, &path).await?;

    // Save a nearby observation position before introducing the exact fixture Rat.
    let mut observer = connect_character(clients, 0xd6, 0).await?;
    let encounter = client.world_spatial().actor_position;
    full_e2ef5::with_observer_liveness(
        &mut client,
        classa3_observation::approach(&mut observer, world, encounter),
    )
    .await?;
    full_e2ef5::with_observer_liveness(&mut client, classa3_observation::logout(&mut observer))
        .await?;
    drop(observer);

    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let answer = request_spawn(session, "oteryn:creature.rat".into())
        .await
        .map_err(|reason| format!("C16c qualification Rat spawn refused: {reason}"))?;
    let expected = parse_spawned_entity(&answer)?;
    let rat = verdict::indexed_timeout(Duration::from_secs(10), async {
        loop {
            if let Some(rat) = client.world_entities().and_then(|view| view.get(&expected))
                && rat.kind == EntityKind::Creature
            {
                return Ok::<_, Box<dyn std::error::Error>>(*rat);
            }
            client.service_liveness(Duration::from_millis(50)).await?;
        }
    })
    .await
    .map_err(|_| "C16c fixture Rat was absent from the live spatial stream")??;
    evidence(&format!(
        "FIXTURE_PLACEMENT row=C16c rat={expected:?} position={:?}",
        rat.position
    ));
    classa3_observation::approach(&mut client, world, rat.position).await?;
    client.take_events();
    let attack = client
        .session_mut()
        .attack_target(Some(&rat.entity))
        .await?;
    if attack.disposition != AttackIntentDisposition::Ok {
        return Err(format!("C16c attack refused: {attack:?}").into());
    }
    let mut position = rat.position;

    let corpse = verdict::indexed_timeout(Duration::from_secs(75), async {
        loop {
            for event in client.take_events() {
                if let SessionEvent::WorldSpatialEntities(delta) = event {
                    if let Some(update) = delta
                        .value
                        .update
                        .iter()
                        .find(|entry| entry.entity == rat.entity)
                    {
                        position = update.position;
                    }
                    if let Some(corpse) = delta.value.enter.iter().find(|entry| {
                        entry.kind == EntityKind::Corpse
                            && entry.position == position
                            && client
                                .world_entities()
                                .is_some_and(|world| world.get(&rat.entity).is_none())
                    }) {
                        return Ok::<_, Box<dyn std::error::Error>>(*corpse);
                    }
                }
            }
            client.service_liveness(Duration::from_millis(100)).await?;
        }
    })
    .await
    .map_err(|_| "C16c Rat produced no visible corpse")??;
    classa3_observation::approach(&mut client, world, corpse.position).await?;
    let index = provisioning::item_index()?;
    let movable = provisioning::item_reference(&index, 3994)?;
    let corpse = verdict::indexed_timeout(Duration::from_secs(25), async {
        loop {
            if let Some(current) = client.world_entities().and_then(|world| world.get(&corpse.entity))
                && matches!(
                    current.detail,
                    oteryn_dev_client::EntityDetail::Object { item_definition_ref, .. }
                        | oteryn_dev_client::EntityDetail::ObjectWithAppearance { item_definition_ref, .. }
                        if item_definition_ref == movable.get()
                )
            {
                return Ok::<_, Box<dyn std::error::Error>>(*current);
            }
            client.service_liveness(Duration::from_millis(100)).await?;
        }
    }).await.map_err(|_| "C16c Rat corpse did not reach movable item 3994")??;
    evidence(&format!(
        "C16C_CORPSE_STAGE entity={:?} item=3994",
        corpse.entity
    ));
    // A fresh observer starts its retained replay window at the cast site.
    let mut observer =
        full_e2ef5::with_observer_liveness(&mut client, connect_character(clients, 0xd7, 0))
            .await?;
    if observer
        .world_entities()
        .is_none_or(|view| view.get(&corpse.entity).is_none())
    {
        return Err("C16c fresh replay observer cannot see the exact Rat corpse".into());
    }
    full_e2ef5::cast_rune(&mut client, spell, rune, None, Some(corpse.position)).await?;
    let files = verdict::indexed_timeout(Duration::from_secs(5), async {
        loop {
            for event in observer.take_events() {
                if let SessionEvent::WorldSpatialEntities(delta) = event
                    && let Some(summon) = delta.value.enter.iter().find(|entry| {
                        entry.kind == EntityKind::Creature
                            && entry.position.floor == corpse.position.floor
                            && entry
                                .position
                                .x
                                .abs_diff(corpse.position.x)
                                .max(entry.position.y.abs_diff(corpse.position.y))
                                <= 1
                            && entry.display_name.as_ref().is_some_and(|name| {
                                name.as_str()
                                    .is_ok_and(|name| name.eq_ignore_ascii_case("Skeleton"))
                            })
                    })
                {
                    return observer
                        .session_mut()
                        .c16c_capture_files(summon.entity)
                        .ok_or_else(|| "C16c summon replay history incomplete".into());
                }
            }
            observer
                .service_liveness(Duration::from_millis(100))
                .await?;
        }
    })
    .await
    .map_err(|_| "C16c Animate Dead produced no Skeleton enter delta")?;
    let files: TestResult<_> = files;
    for (name, bytes) in files? {
        std::fs::write(directory.join(name), bytes)?;
    }
    let retreat = |position| -> TestResult<Vec<StepDirection>> {
        route(world, tile(position)?, |at| is_logout_rest_tile(world, at))
            .ok_or_else(|| "C16c no route away from the hunt to a logout rest tile".into())
    };
    let path = retreat(observer.world_spatial().actor_position)?;
    full_e2ef5::with_observer_liveness(
        &mut client,
        classa3_observation::walk(&mut observer, world, &path),
    )
    .await?;
    full_e2ef5::with_observer_liveness(
        &mut client,
        classa3_observation::logout(&mut observer),
    )
    .await?;
    let path = retreat(client.world_spatial().actor_position)?;
    classa3_observation::walk(&mut client, world, &path).await?;
    classa3_observation::logout(&mut client).await
}

pub(super) async fn run_packet(clients: &SeamClients<'_>, world: &BundleWorld) {
    let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    match verdict::indexed_timeout(
        Duration::from_secs(330),
        verdict::observe(journey(clients, world, Path::new(&directory))),
    )
    .await
    {
        Ok(Ok(())) => evidence(
            "SEAM_EVIDENCE e2e_all scenario=C16c.client status=PASS evidence=animate-dead-replay-captured",
        ),
        Ok(Err(error)) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C16c.client status=FAIL evidence={error}"
        )),
        Err(_) => {
            evidence("SEAM_EVIDENCE e2e_all scenario=C16c.client status=FAIL evidence=timeout")
        }
    }
}
