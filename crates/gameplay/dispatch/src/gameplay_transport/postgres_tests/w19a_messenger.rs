//! Messenger's Christmas offers through the native dialogue, quote and durable trade owners.
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{ChannelContentPin, MovementFacing, NpcPlacement, WorldId};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::item_fence_of;
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use crate::gameplay_transport::npc_trade_command::NpcTradeStep;
use oteryn_protocol_oteryn::npc_service::{
    NpcIntentDisposition, NpcTalkIntent, NpcTradeIntent, NpcTradeSide,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::num::NonZeroU32;
use std::sync::Arc;

const MESSENGER: &str = "oteryn:npc.messenger_of_santa";
const GOLD: &str = "oteryn:item.tibia.i3031";
const STARTING_GOLD: i64 = 200;

#[test]
fn w19a_messenger_christmas_shop_quotes_and_delivers_four_purchases_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(scenario)
}

async fn scenario(admin: String) -> TestResult {
    reset();
    let h = Harness::create_with_admission_seed(admin, "w19a_messenger_shop", true, false).await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) = activated_with_loot_tables(world, None);
    let active = controller.active().ok_or("native generation")?;
    let index = active
        .native_gameplay()
        .and_then(|native| native.item_index())
        .ok_or("native Item index")?;
    let room = pin.qualified_room();
    let channel_pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        room.map_revision_digest(),
        (2, 0, 7),
    );
    let deps = Dependencies::with_content(
        channel_pin,
        room.clone(),
        crate::spell::cast::v1_spell_book().map_err(debug)?,
    )?;
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(active);
    deps.initialize(&owner).await;

    // Seed the Christmas reset decision consumed by the node's seasonal placement.
    let digest = pin.identity().server_artifact_digest();
    sqlx::query("INSERT INTO game_world_reset_epochs
        (world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month)
        VALUES(encode($1,'hex')::uuid,1,$2,'messenger-fixture-map',$2,$2,$2,
               'TIBIA_SERVER_SAVE_BERLIN_06',1,TIMESTAMPTZ '2025-12-12 06:00 Europe/Berlin','2025-12')")
        .bind(world.as_bytes().as_slice())
        .bind(digest.as_slice())
        .execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    sqlx::query(
        "INSERT INTO game_world_event_epochs
        (world_id,reset_epoch,active,announcement,seasonal_calendar_revision,seasonal_event_keys)
        VALUES(encode($1,'hex')::uuid,1,true,'Christmas is active.',1,ARRAY['christmas'])",
    )
    .bind(world.as_bytes().as_slice())
    .execute(&h.pool)
    .await?;
    assert_eq!(
        h.root.active_world_seasonal_events(world, 1).await?,
        ["christmas"]
    );

    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let book = NpcTalkBook::from_model(&catalogue.service_model_for_project().map_err(debug)?);
    assert!(book.get(MESSENGER).is_some(), "native Messenger dialogue");
    owner.states.chat.install_npc_talk(Arc::new(book));
    let session = deps.admitted.game_session_id;
    owner
        .states
        .chat
        .admit(session, "Fixture Hero", owner.owner_now());
    owner.states.chat.set_npc_service(session);
    let npc_actor = {
        let mut runtime = deps.runtime.lock().await;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let spot = runtime
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position();
        let placed = runtime
            .place_dynamic_npc(
                MESSENGER,
                NpcPlacement {
                    x: u16::try_from(spot.x + 1)?,
                    y: u16::try_from(spot.y)?,
                    floor: -i8::try_from(spot.floor)?,
                    direction: MovementFacing::South,
                },
            )
            .map_err(debug)?;
        let npc = runtime
            .visible_entities()
            .npcs
            .into_iter()
            .find(|npc| npc.actor == placed)
            .ok_or("visible Messenger")?;
        EntityRef {
            identity: npc.actor.placement_identity(),
            generation: npc.generation,
        }
    };

    super::item_look_fixtures::seed_main_backpack(&h, &[(GOLD, 100), (GOLD, 100)]).await?;
    let current = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(session)
        .await?
        .0;
    let fence = item_fence_of(current).ok_or("current item fence")?;
    let command = |command_id| UseCommand {
        game_session_id: session,
        command_id,
        item_fence: Some(fence),
    };
    let talk = |text: &str| NpcTalkIntent {
        npc_actor,
        text: text.into(),
    };
    assert_eq!(
        owner
            .npc_talk(deps.actor, command(1), talk("hi"))
            .await
            .disposition,
        NpcIntentDisposition::Ok
    );
    let trade = owner.npc_talk(deps.actor, command(2), talk("trade")).await;
    assert_eq!(trade.disposition, NpcIntentDisposition::Ok);
    let window = owner
        .npc_quote_window(session, &trade.window.ok_or("trade window")?)
        .ok_or("quoted window")?;
    let (merchant, catalogue_revision) = window.open.ok_or("closed Messenger shop")?;
    assert_eq!(merchant, npc_actor);
    assert_eq!(window.offers.len(), 12, "Crystal Messenger catalogue");

    // Crystal messenger_of_santa.lua:53–59 (the 50-gold row is the tree package).
    let expected = [
        ("oteryn:item.tibia.i6504", 25),
        ("oteryn:item.tibia.i6488", 40),
        ("oteryn:item.tibia.i6509", 20),
        ("oteryn:item.tibia.i2812", 50),
    ];
    let mut observed = Vec::new();
    let mut remaining_gold = STARTING_GOLD;
    for (offset, (item, price)) in expected.into_iter().enumerate() {
        let (offer_index, offer) = window
            .offers
            .iter()
            .enumerate()
            .find(|(_, offer)| {
                let key = NonZeroU32::new(offer.item_definition_ref)
                    .and_then(|reference| index.definition_at(reference))
                    .map(|(key, _)| key);
                offer.side == NpcTradeSide::Buy
                    && (key == Some(item) || (price == 50 && offer.unit_price == 50))
            })
            .ok_or_else(|| format!("missing Messenger offer for {item}"))?;
        let result = owner
            .npc_trade(
                deps.actor,
                command(3 + u64::try_from(offset)?),
                NpcTradeIntent {
                    npc_actor,
                    catalogue_revision,
                    offer_index: u32::try_from(offer_index)?,
                    side: offer.side,
                    quantity: 1,
                    expected_unit_price: offer.unit_price,
                },
            )
            .await;
        let inventory: Vec<(String, i64)> = sqlx::query_as(
            "SELECT i.definition_production_key,i.quantity FROM game_item_container_entries e
             JOIN game_item_instances i USING(item_instance_id)
             WHERE e.parent_item_instance_id=encode($1,'hex')::uuid AND i.lifecycle=1",
        )
        .bind(id(70).as_slice())
        .fetch_all(&h.pool)
        .await?;
        let gold: i64 = inventory
            .iter()
            .map(|(key, quantity)| match key.as_str() {
                GOLD => *quantity,
                "oteryn:item.tibia.i3035" => 100 * quantity,
                _ => 0,
            })
            .sum();
        let delivered: i64 = inventory
            .iter()
            .filter(|(key, _)| key == item)
            .map(|(_, quantity)| quantity)
            .sum();
        observed.push((
            item,
            matches!(result, NpcTradeStep::Traded),
            remaining_gold - gold,
            delivered,
        ));
        remaining_gold = gold;
    }
    let quoted = window
        .details
        .iter()
        .filter(|detail| !detail.name.is_empty() && detail.maximum_quantity > 0)
        .count();
    assert_eq!(
        (quoted, observed),
        (
            window.offers.len(),
            expected
                .map(|(item, price)| (item, true, i64::from(price), 1))
                .to_vec()
        ),
        "all offers must be quoted and each purchase must debit its price and deliver its exact item",
    );
    eprintln!("POSTGRES_CASE_COMPLETED W19a Messenger quotes and four purchases");
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}
