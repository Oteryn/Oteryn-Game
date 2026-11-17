//! NPC-TRADE-3 drives the production conversation, trade owner and runtime-role PG.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::{
    NpcDataCatalogue, ProjectV2ServiceOfferDirection, npc_catalogue_preproduction_limits,
};
use crate::foundation::{
    ChannelContentPin, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use crate::gameplay_transport::npc_trade_command::NpcTradeStep;
use oteryn_protocol_oteryn::npc_service::{
    NpcIntentDisposition, NpcTalkIntent, NpcTradeIntent, NpcTradeSide,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

#[test]
fn srv9b_fluids_postgres_buy_sell_subtype_fence_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "SRV9B fluid PG thread panicked")??;
    assert!(started.elapsed() > std::time::Duration::from_secs(1));
    eprintln!(
        "POSTGRES_CASE_COMPLETED SRV9B fluids duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn srv9b_charges_postgres_source_purchase_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(true).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "SRV9B charge PG thread panicked")??;
    assert!(started.elapsed() > std::time::Duration::from_secs(1));
    eprintln!(
        "POSTGRES_CASE_COMPLETED SRV9B charges duration={:?}",
        started.elapsed()
    );
    Ok(())
}

fn scenario(charged: bool) -> TestResult {
    run(async move |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "srv9b_fluids", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("active content")?;
        let entry = (2, 0, 7);
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            entry,
        );
        // One player plus Alice and Rashid each occupy a carrier slot.
        let deps = Dependencies::with_content_and_capacity(
            channel_pin,
            crate::content::qualify_native_entry_room(world).map_err(debug)?,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
            3,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        deps.initialize(&owner).await;
        let catalogue = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?;
        let model = catalogue.service_model_for_project().map_err(debug)?;
        let mut book = NpcTalkBook::from_model(&model);
        let npc_key = if charged {
            "oteryn:npc.baltim"
        } else {
            "oteryn:npc.maria"
        };
        if charged {
            let baltim = model
                .npcs()
                .iter()
                .find(|(key, _)| key.key == "oteryn:npc.baltim")
                .ok_or("Baltim")?
                .1;
            let offers: Vec<_> = baltim
                .services
                .iter()
                .filter_map(|key| model.services().get(key))
                .flat_map(|service| &service.offers)
                .filter(|offer| offer.item.key == "oteryn:item.tibia.i50293")
                .cloned()
                .collect();
            assert_eq!(offers.len(), 1, "full charge source wrap admitted once");
            assert_eq!(
                (offers[0].count, offers[0].sub_type, offers[0].unit_price),
                (Some(1), Some(500), 347222)
            );
            book = book.with(
                npc_key,
                crate::gameplay_transport::npc_talk::NpcTalkEntry::new(
                    "Maria",
                    Some("Charge fixture"),
                )
                .with_offers(offers),
            );
        }

        let maria = model
            .npcs()
            .iter()
            .find(|(key, _)| key.key == "oteryn:npc.maria")
            .ok_or("Maria")?
            .1;
        let beer = maria
            .services
            .iter()
            .filter_map(|key| model.services().get(key))
            .flat_map(|service| &service.offers)
            .find(|offer| {
                offer.item.key == "oteryn:item.tibia.i2880"
                    && offer.sub_type == Some(3)
                    && offer.direction == ProjectV2ServiceOfferDirection::SellToPlayer
            })
            .ok_or("production Maria beer offer")?
            .clone();
        assert_eq!((beer.count, beer.unit_price), (Some(1), 2));
        let mut lemonade = beer.clone();
        lemonade.sub_type = Some(12);
        let mut sell = beer.clone();
        sell.direction = ProjectV2ServiceOfferDirection::BuyFromPlayer;
        sell.unit_price = 1;
        let mut water = sell.clone();
        water.sub_type = Some(1);
        let mut empty = sell.clone();
        empty.sub_type = Some(0);
        let mut buy_empty = empty.clone();
        buy_empty.direction = ProjectV2ServiceOfferDirection::SellToPlayer;
        buy_empty.unit_price = 2;
        if !charged {
            book = book.with(
                "oteryn:npc.maria",
                crate::gameplay_transport::npc_talk::NpcTalkEntry::new("Maria", Some("Drinks"))
                    .with_offers(vec![beer, sell, water, empty, lemonade, buy_empty]),
            );
        }
        let revision = book.catalogue_revision();
        owner.states.chat.install_npc_talk(Arc::new(book));
        let session = deps.admitted.game_session_id;
        owner
            .states
            .chat
            .admit(session, "Fixture Hero", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        let npc = {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            let at = runtime
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position();
            let actors = runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: npc_key.into(),
                            placements: vec![NpcPlacement {
                                x: (at.x + 1) as u16,
                                y: at.y as u16,
                                floor: -7,
                                direction: MovementFacing::South,
                            }],
                        }],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?
                .actors;
            let actor = actors[0];
            EntityRef {
                identity: actor.placement_identity(),
                generation: actor.actor_local_generation(),
            }
        };
        super::item_look::seed_main_backpack(
            &h,
            &[(
                if charged {
                    "oteryn:item.tibia.i3043"
                } else {
                    "oteryn:item.tibia.i3031"
                },
                100,
            )],
        )
        .await?;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(session)
                .await?
                .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        let command = |command_id| UseCommand {
            game_session_id: session,
            command_id,
            item_fence: Some(fence),
        };
        for (id, text) in [(1, "hi"), (2, "trade")] {
            let outcome = owner
                .npc_talk(
                    deps.actor,
                    command(id),
                    NpcTalkIntent {
                        npc_actor: npc,
                        text: text.into(),
                    },
                )
                .await;
            assert!(!outcome.unknown);
            assert_eq!(outcome.disposition, NpcIntentDisposition::Ok);
        }
        let intent = |index, side, price| NpcTradeIntent {
            npc_actor: npc,
            catalogue_revision: revision,
            offer_index: index,
            side,
            quantity: 1,
            expected_unit_price: price,
        };
        if charged {
            let buy = intent(0, NpcTradeSide::Buy, 347222);
            assert!(matches!(
                owner.npc_trade(deps.actor, command(3), buy).await,
                NpcTradeStep::Traded
            ));
            let stored: (Option<i32>, i64) =
                sqlx::query_as("SELECT full_charges,item_units::bigint FROM game_npc_trades")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(stored, (Some(500), 1));
            let quantity:i64=sqlx::query_scalar("SELECT quantity::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i50293'").fetch_one(&h.pool).await?;
            assert_eq!(quantity, 1);
            assert!(matches!(
                owner.npc_trade(deps.actor, command(3), buy).await,
                NpcTradeStep::Traded
            ));
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_npc_trades")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(count, 1);
            return Ok(());
        }
        let buy = intent(0, NpcTradeSide::Buy, 2);
        let looked = owner
            .npc_shop(
                deps.actor,
                command(12),
                oteryn_protocol_oteryn::npc_shop::NpcShopIntent {
                    npc_actor: npc,
                    catalogue_revision: revision,
                    offer_index: 0,
                    side: NpcTradeSide::Buy,
                    action: oteryn_protocol_oteryn::npc_shop::NpcShopAction::Look,
                    expected_unit_price: 2,
                },
            )
            .await;
        assert!(
            matches!(looked,crate::gameplay_transport::npc_shop::NpcShopStep::Looked(text) if text.contains("It contains beer."))
        );
        assert!(matches!(
            owner.npc_trade(deps.actor, command(3), buy).await,
            NpcTradeStep::Traded
        ));
        let fluid:(i16,i64)=sqlx::query_as("SELECT f.fluid_subtype,i.quantity::bigint FROM game_item_fluid_states f JOIN game_item_instances i USING(item_instance_id) WHERE i.lifecycle=1").fetch_one(&h.pool).await?;
        assert_eq!(fluid, (3, 1));
        // Exercise the actual composed observer used by join/resume and later
        // inventory refresh. Durable NPC fluid facts must reach the client view.
        for _ in 0..2 {
            let observed = owner.observe_character_inventory(deps.actor, deps.admitted.game_session_id)
                .await.ok_or("filled-fluid native inventory observation")?;
            let item: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(i.item_instance_id) FROM game_item_instances i JOIN game_item_fluid_states f USING(item_instance_id) WHERE i.lifecycle=1")
                .fetch_one(&h.pool).await?;
            let view = observed.entries.iter().find(|entry| matches!(entry.key,
                crate::gameplay_transport::item_view::ItemKey::Instance(id) if id.as_slice()==item.as_slice()))
                .ok_or("filled-fluid direct inventory entry")?;
            assert_eq!(view.count.get(),1);
            assert_eq!(view.sub_type,3,"NPC beer must retain its native subtype in inventory snapshots");
        }
        let gold:i64=sqlx::query_scalar("SELECT quantity::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i3031'").fetch_one(&h.pool).await?;
        assert_eq!(gold, 98);
        let audit:Vec<u8>=sqlx::query_scalar("SELECT envelope FROM game_item_audit_outbox a JOIN game_npc_trades t USING(transaction_id)").fetch_one(&h.pool).await?;
        let (_, payload) =
            crate::durability::npc_trade_audit::decode_npc_trade_envelope(&audit).map_err(debug)?;
        assert_eq!(payload.fluid_subtype, Some(3));
        assert!(
            sqlx::query("UPDATE game_item_fluid_states SET fluid_subtype=1")
                .execute(&h.pool)
                .await
                .is_err(),
            "unreceipted subtype mutation refused"
        );
        assert!(
            matches!(
                owner
                    .npc_trade(deps.actor, command(4), intent(2, NpcTradeSide::Sell, 1))
                    .await,
                NpcTradeStep::Refused(_)
            ),
            "water cannot sell beer"
        );
        assert!(
            matches!(
                owner
                    .npc_trade(deps.actor, command(5), intent(3, NpcTradeSide::Sell, 1))
                    .await,
                NpcTradeStep::Refused(_)
            ),
            "empty cannot sell beer"
        );
        let mut stale = fence;
        stale.connection_generation =
            crate::foundation::ConnectionGeneration::new(fence.connection_generation.get() + 1)
                .map_err(debug)?;
        assert!(matches!(
            owner
                .npc_trade(
                    deps.actor,
                    UseCommand {
                        item_fence: Some(stale),
                        ..command(6)
                    },
                    buy
                )
                .await,
            NpcTradeStep::Refused(_)
        ));
        assert!(matches!(
            owner
                .npc_trade(deps.actor, command(7), intent(1, NpcTradeSide::Sell, 1))
                .await,
            NpcTradeStep::Traded
        ));
        for next in [8, 9] {
            assert!(matches!(
                owner.npc_trade(deps.actor, command(next), buy).await,
                NpcTradeStep::Traded
            ));
        }
        let sell_all = oteryn_protocol_oteryn::npc_shop::NpcShopIntent {
            npc_actor: npc,
            catalogue_revision: revision,
            offer_index: 1,
            side: NpcTradeSide::Sell,
            action: oteryn_protocol_oteryn::npc_shop::NpcShopAction::SellAll,
            expected_unit_price: 1,
        };
        assert!(matches!(
            owner.npc_shop(deps.actor, command(10), sell_all).await,
            crate::gameplay_transport::npc_shop::NpcShopStep::Sold { units: 2, gold: 2 }
        ));
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_npc_trades")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(count, 5);
        // SELL_ALL is complete within the selected subtype, even when another subtype remains.
        for (id, offer) in [(14, intent(5, NpcTradeSide::Buy, 2)), (15, buy)] {
            assert!(matches!(
                owner.npc_trade(deps.actor, command(id), offer).await,
                NpcTradeStep::Traded
            ));
        }
        let empty_all = oteryn_protocol_oteryn::npc_shop::NpcShopIntent {
            offer_index: 3,
            ..sell_all
        };
        assert!(matches!(
            owner.npc_shop(deps.actor, command(16), empty_all).await,
            crate::gameplay_transport::npc_shop::NpcShopStep::Sold { units: 1, gold: 1 }
        ));
        let remaining: (i64, i16) = sqlx::query_as("SELECT count(*)::bigint, min(f.fluid_subtype) FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) JOIN game_item_fluid_states f USING(item_instance_id) WHERE i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i2880'").fetch_one(&h.pool).await?;
        assert_eq!(
            remaining,
            (1, 3),
            "filled vial survives empty-vial SELL_ALL"
        );
        assert!(matches!(
            owner.npc_shop(deps.actor, command(17), sell_all).await,
            crate::gameplay_transport::npc_shop::NpcShopStep::Sold { units: 1, gold: 1 }
        ));
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_npc_trades")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(count, 9);
        // Replay precedes window and fence liveness, preserves both outcomes after sale.
        let _ = owner
            .npc_talk(
                deps.actor,
                command(11),
                NpcTalkIntent {
                    npc_actor: npc,
                    text: "bye".into(),
                },
            )
            .await;
        assert!(matches!(
            owner
                .npc_trade(
                    deps.actor,
                    UseCommand {
                        item_fence: Some(stale),
                        ..command(3)
                    },
                    buy
                )
                .await,
            NpcTradeStep::Traded
        ));
        assert!(matches!(
            owner
                .npc_trade(deps.actor, command(7), intent(1, NpcTradeSide::Sell, 1))
                .await,
            NpcTradeStep::Traded
        ));
        assert!(matches!(
            owner.npc_shop(deps.actor, command(10), sell_all).await,
            crate::gameplay_transport::npc_shop::NpcShopStep::Sold { units: 2, gold: 2 }
        ));
        assert!(matches!(
            owner
                .npc_trade(deps.actor, command(3), intent(4, NpcTradeSide::Buy, 2))
                .await,
            NpcTradeStep::Refused(_)
        ));
        let again: i64 = sqlx::query_scalar("SELECT count(*) FROM game_npc_trades")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(again, count);
        Ok(())
    })
}
