//! One isolated DB, actual source material production, ordinary logout/assignment/admission,
//! then actual Chondur source-frame dialogue. No fixture cost items or quest flags.
use super::*;
use crate::foundation::{ChannelId, GameSessionId, WorldId};
use crate::gameplay_transport::connection::{
    ConnectionIdentifiers, FreshAdmissionAuthority, LogoutReply,
};
use oteryn_protocol_oteryn::logout::LogoutResult;
pub(in super::super) fn scenario() -> TestResult {
    run_scenario(false)
}
pub(in super::super) fn historical_scenario() -> TestResult {
    run_scenario(true)
}
fn run_scenario(historical: bool) -> TestResult {
    run(async move |admin| {
        reset();
        let began = std::time::Instant::now();
        let h = Harness::create_appearance_with_admission_seed(
            admin,
            "srv7_chondur_natural_source_trade",
            false,
            false,
        )
        .await?;
        prepare_current_session(&h).await?;
        super::super::super::item_look::seed_main_backpack(&h, &[]).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let channel = ChannelId::decode(&id(43)).map_err(debug)?;
        let account: sqlx::types::Uuid = sqlx::query_scalar(
            "SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(41).as_slice())
        .fetch_one(&h.pool)
        .await?;
        let (controller, pin) =
            super::super::super::death_ml_loss_composed::production_generation_with_source(
                world,
                "content-1",
                None,
            )?;
        let active = controller
            .active()
            .ok_or("actual active source generation")?;
        let native = active.native_gameplay().ok_or("actual native source")?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4) ON CONFLICT DO NOTHING")
   .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&h.pool).await?;
        let mut generation = 1_u64;
        let mut session = GameSessionId::decode(&id(50)).map_err(debug)?;
        let mut needs_fresh = false;
        let mut old_lease = 2_u64;
        let mut final_trade = historical;
        let mut total_deaths = 0_usize;
        for batch in 0..2_u64 {
            let frame = source_frame::load(world, native, final_trade)?;
            let cells = frame
                .world
                .movement_cells(pin.qualified_room().movement_cells())
                .map_err(debug)?;
            let room = pin
                .qualified_room()
                .bundle_view(
                    cells.clone(),
                    frame.world.frame_binding_digest(world),
                    frame.world.map_revision_digest(),
                    crate::content::NativeEntryStart {
                        x: frame.start.x,
                        y: frame.start.y,
                        floor: frame.start.floor,
                    },
                )
                .ok_or("actual source bundle view")?;
            if batch == 0 || final_trade {
                let previous = h
                    .root
                    .read_current_content_activation(world, channel)
                    .await?;
                let sequence = previous.as_ref().map_or(1, |p| p.activation_sequence + 1);
                assert!(
                    h.root
                        .record_content_activation(
                            &crate::durability::content_activation::ContentActivationRequest {
                                world_id: world,
                                channel_id: channel,
                                activation_sequence: sequence,
                                previous_sequence: previous.map(|p| p.activation_sequence),
                                server_artifact_digest: room.compiled().server_digest(),
                                client_artifact_digest: room.compiled().client_digest(),
                                frame_binding_digest: room.frame_binding().digest()
                            }
                        )
                        .await?
                );
            }
            // The existing test scaffold's unserved initial runtime is immediately
            // replaced from the actual current assignment before actor admission.
            let scaffold = crate::foundation::ChannelContentPin::from_activation(
                world,
                1,
                room.compiled().server_digest(),
                room.compiled().client_digest(),
                room.frame_binding().digest(),
                room.map_revision_digest(),
                (frame.start.x, frame.start.y, frame.start.floor),
            );
            let npc_count = if final_trade {
                frame
                    .npcs
                    .npcs
                    .iter()
                    .filter(|n| n.key == "oteryn:npc.chondur")
                    .map(|n| n.placements.len())
                    .sum::<usize>()
            } else {
                0
            };
            let capacity = (1 + npc_count + usize::from(!final_trade)) * 2;
            let mut deps = Dependencies::with_content_and_capacity(
                scaffold,
                room,
                native.spell_book().clone(),
                capacity,
            )?;
            let runtime_room = deps.room.clone();
            restore_owner::replace_runtime(
                &h,
                &mut deps,
                &runtime_room,
                world,
                channel,
                generation,
                capacity,
            )
            .await?;
            if needs_fresh {
                let successor = *crate::gameplay_transport::SecureIdentifiers
                    .game_session_id()
                    .ok_or("fresh source session")?
                    .as_bytes();
                session = fresh_after_reset::fresh_after_reset(
                    &h,
                    id(41),
                    *account.as_bytes(),
                    successor,
                    generation,
                    old_lease,
                )
                .await?;
            }
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(active);
            owner.movement_cells = &cells;
            owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&deps.room));
            owner.reset_epoch = h.root.active_world_reset_epoch(world).await?.unwrap_or(0);
            let mut admitted =
                restore_owner::attach(&h, &owner, deps.admitted, session, *account.as_bytes())
                    .await?;
            let offered = crate::gameplay_transport::appearance_change::offer::offered(
                owner.offered_capabilities(),
                owner.appearance_available(),
            );
            admitted.continuity.selected_capabilities =
                crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                    &offered,
                    &[3, 6, 25, 350],
                )
                .ok_or("actual source offered capabilities")?;
            owner.admit_quest_session(&admitted).await;
            if final_trade {
                let model = NpcDataCatalogue::load(
                    std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
                    std::ffi::OsStr::new("world"),
                    npc_catalogue_preproduction_limits(),
                )
                .map_err(debug)?
                .service_model_for_project()
                .map_err(debug)?;
                let book = Arc::new(NpcTalkBook::from_model(&model));
                assert!(book.get("oteryn:npc.chondur").is_some());
                owner.states.chat.install_npc_talk(Arc::clone(&book));
                owner
                    .states
                    .chat
                    .admit(session, "Citizen", owner.owner_now());
                owner.states.chat.set_npc_service(session);
                let table = NpcPlacementTable {
                    npcs: frame
                        .npcs
                        .npcs
                        .into_iter()
                        .filter(|n| n.key == "oteryn:npc.chondur")
                        .collect(),
                };
                let npc = {
                    let mut runtime = owner.runtime.lock().await;
                    runtime
                        .place_npcs(&table, |key| book.get(key).is_some(), true)
                        .map_err(debug)?;
                    let n = runtime
                        .visible_entities()
                        .npcs
                        .into_iter()
                        .find(|n| runtime.npc_key(n.actor) == Some("oteryn:npc.chondur"))
                        .ok_or("actual source Chondur")?;
                    oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                        identity: n.actor.placement_identity(),
                        generation: n.generation,
                    }
                };
                let at = owner
                    .runtime
                    .lock()
                    .await
                    .read_actor_position(admitted.runtime_actor.ok_or("actual fresh actor")?)
                    .map_err(debug)?
                    .position();
                assert_eq!(
                    (at.x, at.y, at.floor),
                    (32361, 32551, 7),
                    "production different-frame admission fallback uses verified source start; no position write"
                );
                if historical {
                    historical::mint_materials(&h, &mut owner, &mut admitted).await?;
                    trade::trade_with_provenance(&h, &mut owner, &mut admitted, npc, false).await?;
                } else {
                    trade::trade(&h, &mut owner, &mut admitted, npc).await?;
                }
                eprintln!(
                    "SRV7_CHONDUR_SOURCE_NATIVE_TRADE_EXECUTED historical_owned_operands={historical} actual_stampor_deaths={total_deaths} source_frame_phase={batch} source_frame_fixture_composition=true elapsed={:.3}s",
                    began.elapsed().as_secs_f64()
                );
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }
            let (home, respawn) = frame.stampor.ok_or("original Stampor source")?;
            combat::initialize(&owner, frame.spawns, home).await?;
            loop {
                let result =
                    collect_batch::collect_batch(&h, &mut owner, &mut admitted, home, respawn)
                        .await?;
                match result {
                    collect_batch::Batch::Complete { actual_deaths } => {
                        total_deaths = total_deaths
                            .checked_add(actual_deaths)
                            .ok_or("source kill bound")?;
                        decay::reclaim(&h, &owner, true).await?;
                        final_trade = true;
                        break;
                    }
                    collect_batch::Batch::NeedsCorpseRetirement { actual_deaths } => {
                        total_deaths = total_deaths
                            .checked_add(actual_deaths)
                            .ok_or("source kill bound")?;
                        assert!(total_deaths <= 10000, "source draw bound; no rate changes");
                        decay::reclaim(&h, &owner, false).await?;
                    }
                }
            }
            old_lease = admitted
                .item_fence
                .ok_or("last actual source fence")?
                .character_lease_generation;
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(120))
                .ok_or("logout combat clock")?;
            assert_eq!(
                owner.logout_session(admitted).await,
                LogoutReply::Result(LogoutResult::Accepted)
            );
            generation = replace_assignment::replace(&h, world, channel).await?;
            needs_fresh = true;
            drop(owner);
        }
        Err("bounded natural source coordinator exhausted without trade".into())
    })
}
