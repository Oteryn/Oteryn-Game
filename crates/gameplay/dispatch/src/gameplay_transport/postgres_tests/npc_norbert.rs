//! W18: actual Norbert NPC BUY, exact23398 origin, receipt replay and immutable metadata.
use super::fixture::{Harness, debug, fence, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    CharacterLease, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use std::sync::Arc;

#[test]
fn srv7_npc_norbert_actual_world_sells_wedding_box_and_replays() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(true).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "NPC cosmetic acquisition thread panicked")??;
    Ok(())
}

fn scenario(actual_world: bool) -> TestResult {
    run(async |admin| {
        reset();
        let started = std::time::Instant::now();
        let h =
            Harness::create_with_admission_seed(admin, "srv7_npc_norbert", false, false).await?;
        prepare_current_session(&h).await?;
        super::item_look::seed_main_backpack(&h, &[("oteryn:item.tibia.i3035", 100)]).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world_id = fence(1)?.runtime_scope.world_id();
        let (controller, pin) = super::death_ml_loss_composed::production_generation_with_source(
            world_id,
            "content-1",
            None,
        )?;
        let active = controller.active().ok_or("active generation")?;
        let entry = if actual_world {
            actual_source::ENTRY
        } else {
            (2, 0, 7)
        };
        let (bundle, actual_npcs) = if actual_world {
            let (world, npcs) = actual_source::world(
                world_id,
                active
                    .native_gameplay()
                    .ok_or("actual retailer native content")?,
            )?;
            (world, Some(npcs))
        } else {
            (super::grizzly_npc::wire::world(world_id)?, None)
        };
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
                    x: entry.0,
                    y: entry.1,
                    floor: entry.2,
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
            entry,
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
        let book = Arc::new(NpcTalkBook::from_model(&model));
        let index = active
            .native_gameplay()
            .and_then(|native| native.item_index())
            .ok_or("native Item index")?;
        let offers = oteryn_gameplay_npc::npc_command::projected(
            book.get("oteryn:npc.norbert")
                .ok_or("Norbert source")?
                .offers(),
            index,
        )
        .collect::<Vec<_>>();
        let offer_index = offers
            .iter()
            .position(|(shown, source)| {
                source.item.key == "oteryn:item.tibia.i9586"
                    && shown.side == oteryn_protocol_oteryn::npc_service::NpcTradeSide::Buy
            })
            .ok_or("wedding box source offer")?;
        let catalogue_revision = book.catalogue_revision();
        assert!(book.get("oteryn:npc.norbert").is_some());
        owner.states.chat.install_npc_talk(Arc::clone(&book));
        owner
            .states
            .chat
            .admit(admitted.game_session_id, "Citizen", owner.owner_now());
        owner.states.chat.set_npc_service(admitted.game_session_id);
        let npc = {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .place_npcs(
                    actual_npcs.as_ref().unwrap_or(&NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: "oteryn:npc.norbert".into(),
                            placements: vec![NpcPlacement {
                                x: 3,
                                y: 0,
                                floor: -7,
                                direction: MovementFacing::West,
                            }],
                        }],
                    }),
                    |key| book.get(key).is_some(),
                    true,
                )
                .map_err(debug)?;
            let placed = runtime
                .visible_entities()
                .npcs
                .into_iter()
                .find(|entry| runtime.npc_key(entry.actor) == Some("oteryn:npc.norbert"))
                .ok_or("Norbert actor")?;
            if actual_world {
                assert_eq!(
                    (placed.position.x, placed.position.y, placed.position.floor),
                    (32952, 32103, 6)
                );
            }
            oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                identity: placed.actor.placement_identity(),
                generation: placed.generation,
            }
        };
        admitted.continuity.selected_capabilities =
            crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                owner.offered_capabilities(),
                if actual_world {
                    &[3, 4, 6, 11, 15]
                } else {
                    &[3, 4, 6, 15]
                },
            )
            .ok_or("NPC trade capabilities")?;
        let intent = oteryn_protocol_oteryn::npc_service::NpcTradeIntent {
            npc_actor: npc,
            catalogue_revision,
            offer_index: u32::try_from(offer_index)?,
            side: oteryn_protocol_oteryn::npc_service::NpcTradeSide::Buy,
            quantity: 1,
            expected_unit_price: 5000,
        };
        assert_eq!(
            super::grizzly_npc::wire::trade(
                &mut owner,
                &mut admitted,
                &h,
                1,
                &intent,
                None,
                true,
            )
            .await?,
            vec![oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Ok; 2]
        );
        let minted:(String,i64)=sqlx::query_as("SELECT i.definition_production_key,i.quantity FROM game_item_instances i WHERE i.minted_transaction_id=(SELECT transaction_id FROM game_npc_trades)").fetch_one(&h.pool).await?;
        assert_eq!(
            minted,
            ("oteryn:item.tibia.i9586".into(), 1),
            "actual native Norbert sells the raw source wedding box"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*)FROM game_npc_trades")
                .fetch_one(&h.pool)
                .await?,
            1,
            "wire replay cannot charge or mint twice"
        );
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT quantity FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3035' AND lifecycle=1").fetch_one(&h.pool).await?,50);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*)FROM game_house_furniture_kit_origins")
                .fetch_one(&h.pool)
                .await?,
            0,
            "wedding box is never substituted with a furniture kit"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        println!(
            "PG_MARKER srv7-norbert-source-wedding-box elapsed={:?}",
            started.elapsed()
        );
        Ok(())
    })
}

#[path = "npc_norbert/actual_source.rs"]
mod actual_source;
