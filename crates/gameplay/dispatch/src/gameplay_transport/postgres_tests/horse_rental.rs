//! Actual native rental conversation must yield a temporary personal mount.
use super::fixture::{Harness, debug, fence, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::domain::appearance::AppearanceContent;
use crate::foundation::{
    CharacterLease, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use std::sync::Arc;
#[path = "horse_rental/appearance_revisions.rs"]
mod appearance_revisions;
#[path = "horse_rental/expiry.rs"]
mod expiry;
#[path = "horse_rental/source_room.rs"]
mod source_room;
#[path = "horse_rental/world.rs"]
mod world;
#[test]
fn srv7_horse_rental_real_npc_acquires_personal_twenty_four_hour_mount() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "horse rental thread")??;
    Ok(())
}
fn scenario() -> TestResult {
    scenario_mode(false)
}
fn scenario_mode(large_cash: bool) -> TestResult {
    run(async |admin| {
        reset();
        let started = std::time::Instant::now();
        let h = Harness::create_appearance_with_admission_seed(
            admin,
            "srv7_horse_rental",
            false,
            false,
        )
        .await?;
        prepare_current_session(&h).await?;
        super::item_look::seed_main_backpack(&h, &[("oteryn:item.tibia.i3035", 15)]).await?;
        if large_cash {
            cash_layout::seed(&h).await?;
        }
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT DO NOTHING").bind(id(41).as_slice()).execute(&h.pool).await?;
        let npc_key = "oteryn:npc.appaloosa";
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world_id = fence(1)?.runtime_scope.world_id();
        let (controller, pin) = super::raid_live::hare_production_activation::activate(
            world_id,
            "OTERYN_SRV7_HORSE_CLIENT_ARTIFACT",
        )?;
        let active = controller.active().ok_or("active generation")?;
        let bundle = source_room::world(
            world_id,
            crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
            active.native_gameplay().ok_or("native")?,
        )?;
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
                    x: 32851,
                    y: 32124,
                    floor: 7,
                },
            )
            .ok_or("bundle room")?;
        let channel_pin = crate::foundation::ChannelContentPin::from_activation(
            world_id,
            1,
            pin.identity().server_artifact_digest(),
            room.compiled().client_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (32851, 32124, 7),
        );
        // The in-transaction custody owner independently reads this real activation.
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4) ON CONFLICT DO NOTHING")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(channel_pin.server_artifact_digest().as_slice())
            .bind(channel_pin.client_artifact_digest().as_slice())
            .bind(channel_pin.frame_binding_digest().as_slice()).execute(&h.pool).await?;
        let deps = Dependencies::with_content_and_capacity(
            channel_pin,
            room,
            active
                .native_gameplay()
                .ok_or("native")?
                .spell_book()
                .clone(),
            64,
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
                .install_companion_policies(
                    active
                        .native_gameplay()
                        .ok_or("actual horse native generation")?
                        .policies()
                        .clone(),
                )
                .map_err(debug)?;
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
                            key: npc_key.into(),
                            placements: vec![NpcPlacement {
                                x: 32852,
                                y: 32124,
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
                .ok_or("actual source horse-station NPC actor")?;
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
                &[3, 4, 6, 11, 14, 15, 18, 25, 86, 92, 385],
            )
            .ok_or("NPC capability")?;
        assert!(admitted.continuity.selected_capabilities.contains(25));
        super::grizzly_npc::wire::talk(
            &mut owner,
            &mut admitted,
            npc,
            &[(1, "hi"), (2, "rent"), (3, "yes")],
        )
        .await?;
        let current = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|_| "current rental fence")?
            .ok_or("rental fence")?;
        let content = active
            .native_gameplay()
            .and_then(|n| n.cosmetics())
            .ok_or("active cosmetics")?;
        let state = h
            .root
            .read_current_character_appearance_state(
                &authority,
                &h.node,
                current,
                content.earned_grant_keys(),
            )
            .await?;
        assert!(
            state.earned.iter().any(|(key, _)| matches!(
                key.as_str(),
                "oteryn:rental.horse.22" | "oteryn:rental.horse.25" | "oteryn:rental.horse.26"
            )),
            "actual hi/rent/yes for500gold must expose a source horse to its current Character"
        );
        let permanent:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlocks WHERE unlock_key LIKE 'oteryn:rental.horse.%'").fetch_one(&h.pool).await?;
        assert_eq!(
            permanent, 0,
            "rentals must never become permanent account entitlements"
        );
        if large_cash {
            cash_layout::verify(&h).await?;
            return Ok(());
        }
        expiry::exercise(&h, &mut owner, &mut admitted, npc).await?;
        world::exercise(&h, &mut owner, &mut admitted, npc).await?;
        eprintln!(
            "SRV7_HORSE_RENTAL_NATIVE_EXECUTED elapsed={:.3}s",
            started.elapsed().as_secs_f64()
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

#[path = "horse_rental/cash_layout.rs"]
mod cash_layout;
#[test]
fn srv7_horse_rental_native_418_recursive_coin_leaves_conserve_and_decode_audit() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_mode(true).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "horsecashsource thread")??;
    Ok(())
}
