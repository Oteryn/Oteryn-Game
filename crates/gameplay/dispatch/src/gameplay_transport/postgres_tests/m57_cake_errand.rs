//! Personal Cake mass and city extraction through committed movement.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use oteryn_protocol_oteryn::world_spatial::StepDirection;

#[test]
fn m57_cake_mass_is_extracted_by_actual_city_steps() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256 << 20).spawn(|| {
        run(async |admin| {
            let started = std::time::Instant::now();
            let h = Harness::create_with_admission_seed(admin, "m57_errand", true, false).await?;
            {
                prepare_current_session(&h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                let mut deps = Dependencies::new()?;
                let (controller, pin) = crate::content::native_gameplay::test_support::activated_with_loot_tables(deps.admitted.world_id, None);
                let active = controller.active().ok_or("active content")?;
                let native = active.native_gameplay().ok_or("native content")?;
                let start = MovementLocalPosition { x: 32346, y: 31820, floor: 7 };
                let mark = MovementLocalPosition { y: 31821, ..start };
                let machine = MovementLocalPosition { y: 31822, ..start };
                let world = super::cake_phases::cake_bundle(native, deps.admitted.world_id, deps.admitted.channel_id, start, &[(mark,12090),(machine,8886)])?;
                let cells = world.movement_cells(pin.qualified_room().movement_cells()).map_err(debug)?;
                let room = super::rust_lottery::bundle_action_room(pin.qualified_room(), &world, deps.admitted.world_id, &cells)?;
                let content_pin = ChannelContentPin::from_activation(
                    deps.admitted.world_id, pin.activation_sequence(),
                    pin.identity().server_artifact_digest(), pin.identity().client_artifact_digest(),
                    room.frame_binding().digest(), room.map_revision_digest(),
                    (start.x, start.y, start.floor),
                );
                let mut runtime = ChannelRuntimeV1::from_committed_assignment(
                    deps.admitted.world_id, deps.admitted.channel_id,
                    crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
                    1, 1, 1, "runtime-scope-assignment:1", 2, content_pin,
                ).map_err(debug)?;
                let reserved = runtime.reserve_fresh_session(deps.admitted.game_session_id).map_err(debug)?;
                deps.actor = runtime.commit_fresh_session(reserved).map_err(debug)?;
                runtime.initialize_pinned_test_position(deps.actor, start).map_err(debug)?;
                runtime.bind_attacker_lease(deps.actor, deps.admitted.game_session_id,
                    crate::foundation::CharacterLease::new(crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?, 2).map_err(debug)?).map_err(debug)?;
                deps.runtime = tokio::sync::Mutex::new(runtime);
                let mut owner = deps.owner(&h, &authority);
                owner.active_generation = Some(active);
                owner.movement_cells = crate::gameplay_transport::activity_runtime::SharedOwner::Borrowed(&cells);
                owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
                owner.reset_epoch = 1;
                deps.initialize(&owner).await;
                let bundle = world.base().digest();
                sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
                    .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
                sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
                    .bind(id(42).as_slice()).bind(id(43).as_slice())
                    .bind(pin.identity().server_artifact_digest().as_slice())
                    .bind(pin.identity().client_artifact_digest().as_slice())
                    .bind(room.frame_binding().digest().as_slice()).execute(&h.pool).await?;
                // Historical world contributions are fixture input. The tested
                // personal changes below use the production current movement owner.
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,$3,$4,$5,$6,'TIBIA_SERVER_SAVE_BERLIN_06',1,'2026-02-21 05:00:00+00','2026-02')")
                    .bind(id(42).as_slice()).bind(bundle.as_slice()).bind(world.map_revision())
                    .bind(pin.identity().server_artifact_digest().as_slice())
                    .bind(pin.identity().client_artifact_digest().as_slice())
                    .bind(room.frame_binding().digest().as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
                    .bind(id(42).as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_cake_world_progress VALUES(encode($1,'hex')::uuid,2026,200,200)")
                    .bind(id(42).as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_cake_phases(world_id,event_year,stage,layer) VALUES(encode($1,'hex')::uuid,2026,3,4)")
                    .bind(id(42).as_slice()).execute(&mut *tx).await?;
                tx.commit().await?;
                sqlx::query("UPDATE game_cake_phases SET stage=2,layer=1 WHERE world_id=encode($1,'hex')::uuid AND event_year=2026")
                    .bind(id(42).as_slice()).execute(&h.pool).await?;
                let (early, _) = owner.ordinary_paced_step(deps.actor, deps.admitted.game_session_id, StepDirection::South, None, None).await;
                assert!(early.moved_to.is_some());
                let early_keys: Vec<String> = sqlx::query_scalar("SELECT a.achievement_key FROM game_account_achievements a JOIN game_character_roots c USING(account_id) WHERE c.character_id=encode($1,'hex')::uuid ORDER BY a.achievement_key")
                    .bind(id(41).as_slice()).fetch_all(&h.pool).await?;
                assert_eq!(early_keys,["oteryn:achievement/cake_conqueror"],"the source Cake base grants Conqueror before stage three");
                let early_progress: i64=sqlx::query_scalar("SELECT count(*) FROM game_cake_errands WHERE character_id=encode($1,'hex')::uuid")
                    .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(early_progress,0,"stage two cannot arm the personal errand");
                owner.clock_origin=owner.clock_origin.checked_sub(std::time::Duration::from_secs(2)).ok_or("early movement clock")?;
                let (back, _) = owner.ordinary_paced_step(deps.actor, deps.admitted.game_session_id, StepDirection::North, None, None).await;
                assert!(back.moved_to.is_some());
                sqlx::query("UPDATE game_cake_phases SET stage=3,layer=4 WHERE world_id=encode($1,'hex')::uuid AND event_year=2026")
                    .bind(id(42).as_slice()).execute(&h.pool).await?;
                owner.clock_origin=owner.clock_origin.checked_sub(std::time::Duration::from_secs(2)).ok_or("completed movement clock")?;
                let (outcome, wait) = owner.ordinary_paced_step(deps.actor, deps.admitted.game_session_id, StepDirection::South, None, None).await;
                assert_eq!(outcome.moved_to.ok_or("step onto source cake mass")?.actor_position.y, 31821);
                let armed: Option<bool> = sqlx::query_scalar("SELECT armed FROM game_cake_errands WHERE world_id=encode($1,'hex')::uuid AND event_year=2026 AND character_id=encode($2,'hex')::uuid")
                    .bind(id(42).as_slice()).bind(id(41).as_slice()).fetch_optional(&h.pool).await?;
                assert_eq!(armed, Some(true), "a real committed Cake step must arm the personal errand");
                let completed_keys: Vec<String> = sqlx::query_scalar("SELECT a.achievement_key FROM game_account_achievements a JOIN game_character_roots c USING(account_id) WHERE c.character_id=encode($1,'hex')::uuid ORDER BY a.achievement_key")
                    .bind(id(41).as_slice()).fetch_all(&h.pool).await?;
                assert_eq!(completed_keys,["oteryn:achievement/cake_conqueror","oteryn:achievement/i_did_my_part"],"stage three adds I Did My Part without re-granting Conqueror");
                if let Some(wait) = wait { tokio::time::sleep(wait).await; }
                let (outcome, _) = owner.ordinary_paced_step(deps.actor, deps.admitted.game_session_id, StepDirection::South, None, None).await;
                assert_eq!(outcome.moved_to.ok_or("step into source Carlin extractor")?.actor_position.y, 31822);
                let state: (bool,i16) = sqlx::query_as("SELECT armed,cities FROM game_cake_errands WHERE world_id=encode($1,'hex')::uuid AND event_year=2026 AND character_id=encode($2,'hex')::uuid")
                    .bind(id(42).as_slice()).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(state, (false,1), "extracting Carlin consumes the mark and records exactly Carlin");
                let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_cake_errand_steps WHERE world_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid")
                    .bind(id(42).as_slice()).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(receipts,2);
                let current = owner.current_quest_fence(deps.admitted.game_session_id).await.map_err(debug)?.ok_or("current Character fence")?;
                let fence = current.item_fence();
                let revision = deps.runtime.lock().await.read_actor_position(deps.actor).map_err(debug)?.revision();
                let mut position = [0;12];
                position[..4].copy_from_slice(&machine.x.to_be_bytes());
                position[4..8].copy_from_slice(&machine.y.to_be_bytes());
                position[8..].copy_from_slice(&i32::from(machine.floor).to_be_bytes());
                let request = crate::durability::cake_event::CakeErrandStep {
                    actor: deps.actor.placement_identity(), movement_revision: revision, position,
                    source_item: 8886, bundle,
                    scope: crate::durability::terrain_tool::TerrainScope {
                        map_revision: world.map_revision().to_owned(), content_generation: native.source_digest(), reset_epoch: 1,
                    },
                };
                let permit = owner.spell_lane_permit().await.ok_or("movement lane")?;
                assert_eq!(h.root.record_world_cake_step(&permit, &authority, owner.holder, fence, request.clone()).await.map_err(debug)?, "replay");
                let mut stale = fence;
                assert_eq!(stale.character_lease_generation,2);
                stale.character_lease_generation = 3;
                assert!(h.root.record_world_cake_step(&permit, &authority, owner.holder, stale, request.clone()).await.is_err(),
                    "a historical step receipt never authorizes a stale Character lease");
                let mut mismatched = request.clone();
                mismatched.scope.content_generation[0] ^= 1;
                assert!(h.root.record_world_cake_step(&permit, &authority, owner.holder, fence, mismatched).await.is_err(),
                    "the independent active Content pin is checked even on replay");
                let mut old_epoch = request;
                old_epoch.scope.reset_epoch = 0;
                assert!(h.root.record_world_cake_step(&permit, &authority, owner.holder, fence, old_epoch).await.is_err(),
                    "a step from another reset epoch cannot grant or replay progress");
                let after: (bool,i16) = sqlx::query_as("SELECT armed,cities FROM game_cake_errands WHERE world_id=encode($1,'hex')::uuid AND event_year=2026 AND character_id=encode($2,'hex')::uuid")
                    .bind(id(42).as_slice()).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(after,state,"refused authority variants leave the personal state unchanged");
                let grants:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_achievement_grant_requests WHERE source_kind='oteryn:cake-isle-step' AND character_id=encode($1,'hex')::uuid")
                    .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(grants,2,"held achievements and refused step replays never append extra grants");
            }
            h.cleanup().await?;
            assert!(started.elapsed() > std::time::Duration::from_millis(10));
            eprintln!("M57_CAKE_ERRAND_POSTGRES_COMPLETED elapsed={:?}", started.elapsed());
            Ok(())
        }).map_err(|error| error.to_string())
    })?.join().map_err(|_| "M57 errand thread panicked")??;
    Ok(())
}
