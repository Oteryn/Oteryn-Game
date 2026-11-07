//! W05d: actual source Messenger greeting, all13 current offers and framed BUY/replay.
use super::fixture::{Harness, debug, fence, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    CharacterLease, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use std::sync::Arc;
#[path = "seasonal_messenger_shop/body.rs"]
mod body;

#[test]
fn srv7_messenger_of_santa_native_hi_trade_thirteen_current_goods_buy_and_replay() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Messenger shop thread panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let started = std::time::Instant::now();
        let h =
            Harness::create_appearance_with_admission_seed(admin, "srv7_messenger", false, false)
                .await?;
        prepare_current_session(&h).await?;
        super::item_look::seed_main_backpack(&h, &[("oteryn:item.tibia.i3031", 100)]).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world_id = fence(1)?.runtime_scope.world_id();
        let (controller, pin) = super::raid_live::hare_production_activation::activate(
            world_id,
            "OTERYN_SRV7_MESSENGER_CLIENT_ARTIFACT",
        )?;
        let active = controller.active().ok_or("active generation")?;
        let bundle = super::grizzly_npc::wire::world(world_id)?;
        let cells = bundle
            .movement_cells(pin.qualified_room().movement_cells())
            .map_err(debug)?;
        let room = pin
            .qualified_room()
            .bundle_view(
                cells.clone(),
                bundle.frame_binding_digest(world_id),
                bundle.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: 2,
                    y: 0,
                    floor: 7,
                },
            )
            .ok_or("bundle room")?;
        let channel_pin = crate::foundation::ChannelContentPin::from_activation(
            world_id,
            1,
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (2, 0, 7),
        );
        // The in-transaction custody owner independently reads this real activation.
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4) ON CONFLICT DO NOTHING")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(channel_pin.server_artifact_digest().as_slice())
            .bind(channel_pin.client_artifact_digest().as_slice())
            .bind(channel_pin.frame_binding_digest().as_slice()).execute(&h.pool).await?;
        let deps = Dependencies::with_content(
            channel_pin,
            room,
            active
                .native_gameplay()
                .ok_or("native")?
                .spell_book()
                .clone(),
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
        let actor = deps.actor;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        owner.admit_progression_session(&mut admitted).await;
        {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .initialize_first_entry_position(actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    actor,
                    admitted.game_session_id,
                    CharacterLease::new(
                        crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                        2,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
        }
        let current = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(debug)?
            .ok_or("current fence")?;
        admitted.item_fence = Some(current.item_fence());
        let (crate::gameplay_transport::spell_character_facts::CastFactsLoad::Ready {
            facts, ..
        }
        | crate::gameplay_transport::spell_character_facts::CastFactsLoad::NoVocation {
            facts,
            ..
        }) = crate::gameplay_transport::spell_character_facts::load_character_cast_facts(
            &h.root,
            &authority,
            current.character_id,
        )
        .await
        else {
            return Err("durable NPC customer build".into());
        };
        {
            let runtime = owner.runtime.lock().await;
            assert!(
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .initialize(
                        &runtime,
                        actor,
                        admitted.game_session_id,
                        facts,
                        (0, 0),
                        owner.owner_now()
                    )
                    .is_some()
            );
        }
        let model = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?
        .service_model_for_project()
        .map_err(debug)?;
        let book = NpcTalkBook::from_model(&model);
        let index = active
            .native_gameplay()
            .and_then(|native| native.item_index())
            .ok_or("native Item index")?;
        let offers = oteryn_gameplay_npc::npc_command::projected(
            book.get("oteryn:npc.messenger_of_santa")
                .ok_or("Messenger of Santa source")?
                .offers(),
            index,
        )
        .collect::<Vec<_>>();
        assert_eq!(
            offers.len(),
            13,
            "current Wiki adds the blue present to the twelve pinned donor goods"
        );
        let actual: std::collections::BTreeMap<String, u32> = offers
            .iter()
            .map(|(shown, source)| (source.item.key.clone(), shown.unit_price))
            .collect();
        let expected: std::collections::BTreeMap<String, u32> = [
            (10208, 50),
            (6387, 10),
            (6488, 40),
            (6501, 45),
            (6502, 25),
            (6503, 25),
            (6504, 25),
            (6505, 20),
            (6506, 70),
            (6507, 60),
            (6508, 80),
            (6509, 20),
            (6510, 20),
        ]
        .into_iter()
        .map(|(id, price)| (format!("oteryn:item.tibia.i{id}"), price))
        .collect();
        assert_eq!(
            actual, expected,
            "current source thirteen offers include blue present6510 and one real tree package10208; the rendered tree2812 is not a second package offer"
        );
        let offer_index = offers
            .iter()
            .position(|(shown, source)| {
                source.item.key == "oteryn:item.tibia.i6387"
                    && shown.side == oteryn_protocol_oteryn::npc_service::NpcTradeSide::Buy
            })
            .ok_or("Christmas card source offer")?;
        let catalogue_revision = book.catalogue_revision();
        assert!(book.get("oteryn:npc.messenger_of_santa").is_some());
        owner.states.chat.install_npc_talk(Arc::new(book));
        owner
            .states
            .chat
            .admit(admitted.game_session_id, "Citizen", owner.owner_now());
        owner.states.chat.set_npc_service(admitted.game_session_id);
        let npc = {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: "oteryn:npc.messenger_of_santa".into(),
                            placements: vec![NpcPlacement {
                                x: 3,
                                y: 0,
                                floor: -7,
                                direction: MovementFacing::West,
                            }],
                        }],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?;
            let placed = runtime
                .visible_entities()
                .npcs
                .into_iter()
                .next()
                .ok_or("Messenger of Santa actor")?;
            oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                identity: placed.actor.placement_identity(),
                generation: placed.generation,
            }
        };
        let offered = crate::gameplay_transport::appearance_change::offer::offered(
            owner.offered_capabilities(),
            owner.appearance_available(),
        );
        admitted.continuity.selected_capabilities =
            crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                &offered,
                &[3, 4, 6, 15, 18, 25],
            )
            .ok_or("NPC trade and appearance capabilities")?;
        assert!(admitted.continuity.selected_capabilities.contains(25));
        assert!(!admitted.continuity.selected_capabilities.contains(104));
        body::capture(&owner, &deps, &admitted, npc).await?;
        let intent = oteryn_protocol_oteryn::npc_service::NpcTradeIntent {
            npc_actor: npc,
            catalogue_revision,
            offer_index: u32::try_from(offer_index)?,
            side: oteryn_protocol_oteryn::npc_service::NpcTradeSide::Buy,
            quantity: 1,
            expected_unit_price: 10,
        };
        assert_eq!(
            super::grizzly_npc::wire::trade(&mut owner, &mut admitted, &h, 1, &intent, None, true)
                .await?,
            vec![oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Ok; 2]
        );
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i6387' AND lifecycle=1").fetch_one(&h.pool).await?,1);
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT quantity FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3031' AND lifecycle=1").fetch_one(&h.pool).await?,90);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_npc_trades")
                .fetch_one(&h.pool)
                .await?,
            1,
            "framed replay must not charge or mint twice"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        println!(
            "PG_MARKER srv7-messenger-native-hi-trade-buy elapsed={:?}",
            started.elapsed()
        );
        Ok(())
    })
}
