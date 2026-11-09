//! Full personal Cake journey: bridge, all five real extraction steps and annual NPC rewards.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use oteryn_protocol_oteryn::world_spatial::StepDirection;

#[test]
fn m57_a_piece_of_cake_full_journey_completes_with_both_annual_rewards() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256 << 20).spawn(|| {
        run(async |admin| {
            let started = std::time::Instant::now();
            let h = Harness::create_with_admission_seed(admin, "m57_full_journey", true, false).await?;
            {
                prepare_current_session(&h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
                let mut deps = Dependencies::new()?;
                let (controller, pin) = crate::content::native_gameplay::test_support::activated_with_loot_tables(deps.admitted.world_id, None);
                let active = controller.active().ok_or("active content")?;
                let native = active.native_gameplay().ok_or("native content")?;
                let start = MovementLocalPosition{x:33303,y:32051,floor:7};
                let mark = MovementLocalPosition{x:33303,y:32065,floor:7};
                let machines = [(32346,31822),(32350,32275),(33251,31802),(33191,32397),(32377,32801)];
                let mut tops = Vec::new();
                for x in 33300..=33306 { for y in 32052..=32059 { tops.push((MovementLocalPosition{x,y,floor:7},4597)); }}
                for y in 32060..=32064 { tops.push((MovementLocalPosition{x:33303,y,floor:7},4519)); }
                tops.push((mark,12090));
                for (x,y) in machines { tops.push((MovementLocalPosition{x,y,floor:7},8886)); }
                let world = super::cake_phases::cake_bundle(native,deps.admitted.world_id,deps.admitted.channel_id,start,&tops)?;
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
                deps.admitted.runtime_actor = Some(deps.actor);
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
                sqlx::query("INSERT INTO game_cake_phases(world_id,event_year,stage,layer,brutus_present) VALUES(encode($1,'hex')::uuid,2026,3,4,true)")
                    .bind(id(42).as_slice()).execute(&mut *tx).await?;
                tx.commit().await?;
                // Shared community completion is historical input. No personal
                // mark, visited city, reward flag, receipt or Item is pre-completed.
                super::item_look::seed_main_backpack(&h,&[]).await?;
                let catalogue = crate::content::NpcDataCatalogue::load(
                    std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../content")),
                    std::ffi::OsStr::new("world"),crate::content::npc_catalogue_preproduction_limits()).map_err(debug)?;
                let model = catalogue.service_model_for_project().map_err(debug)?;
                owner.states.chat.install_npc_talk(std::sync::Arc::new(crate::gameplay_transport::npc_talk::NpcTalkBook::from_model(&model)));
                owner.states.chat.admit(deps.admitted.game_session_id,"Cake witness",owner.owner_now());
                owner.states.chat.set_npc_service(deps.admitted.game_session_id);
                assert!(!world.enterable(crate::map::overlay::TilePos{x:33303,y:32053,floor:-7}));
                for y in 32052..=32063 {
                    step(&mut owner,deps.actor,deps.admitted.game_session_id,StepDirection::South).await?;
                    assert_eq!(deps.runtime.lock().await.read_actor_position(deps.actor).map_err(debug)?.position().y,y);
                }
                let npc = {
                    let runtime=deps.runtime.lock().await;
                    let npc=runtime.visible_entities().npcs.into_iter().find(|npc|runtime.npc_key(npc.actor)==Some("oteryn:npc.brutus_the_baker")).ok_or("projected Brutus")?;
                    oteryn_protocol_oteryn::world_spatial_entities::EntityRef{identity:npc.actor.placement_identity(),generation:npc.generation}
                };
                super::classic_quests::talk(&mut owner,&deps.admitted,npc,9000,"hi").await?;
                super::classic_quests::talk(&mut owner,&deps.admitted,npc,9001,"help").await?;
                let denied=super::classic_quests::talk(&mut owner,&deps.admitted,npc,9002,"backpack").await?;
                assert!(denied.iter().any(|line|line.contains("did not earn")),"a visible Brutus must refuse an unfinished personal errand");
                let mut expected=0_i16;
                for (city,(x,y)) in machines.into_iter().enumerate() {
                    // Travel is arranged through the real movement owner. Every
                    // quest-changing mark and extraction is an ordinary STEP.
                    travel(&owner,deps.actor,deps.admitted.game_session_id,MovementLocalPosition{y:mark.y+1,..mark}).await?;
                    step(&mut owner,deps.actor,deps.admitted.game_session_id,StepDirection::North).await?;
                    travel(&owner,deps.actor,deps.admitted.game_session_id,MovementLocalPosition{x,y:y-1,floor:7}).await?;
                    step(&mut owner,deps.actor,deps.admitted.game_session_id,StepDirection::South).await?;
                    expected |= 1_i16 << city;
                    let state:(bool,i16)=sqlx::query_as("SELECT armed,cities FROM game_cake_errands WHERE world_id=encode($1,'hex')::uuid AND event_year=2026 AND character_id=encode($2,'hex')::uuid")
                        .bind(id(42).as_slice()).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
                    assert_eq!(state,(false,expected));
                }
                travel(&owner,deps.actor,deps.admitted.game_session_id,MovementLocalPosition{x:33303,y:32063,floor:7}).await?;
                super::classic_quests::talk(&mut owner,&deps.admitted,npc,9010,"hi").await?;
                super::classic_quests::talk(&mut owner,&deps.admitted,npc,9011,"reward").await?;
                super::classic_quests::talk(&mut owner,&deps.admitted,npc,9012,"one").await?;
                super::classic_quests::talk(&mut owner,&deps.admitted,npc,9013,"backpack").await?;
                super::classic_quests::talk(&mut owner,&deps.admitted,npc,9013,"backpack").await?;
                super::classic_quests::talk(&mut owner,&deps.admitted,npc,9014,"cupcake").await?;
                let menu=super::classic_quests::talk(&mut owner,&deps.admitted,npc,9018,"blueberry cupcake").await?;
                assert!(menu.iter().any(|line|line.contains("which one do you want")),"donor cupcake branch precedes flavour matching");
                let before_cupcake:i64=sqlx::query_scalar("SELECT count(*) FROM game_cake_reward_claims WHERE character_id=encode($1,'hex')::uuid AND category=2")
                    .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(before_cupcake,0,"opening the source menu never awards an item");
                super::classic_quests::talk(&mut owner,&deps.admitted,npc,9015,"blueberry").await?;
                let item_again=super::classic_quests::talk(&mut owner,&deps.admitted,npc,9016,"tapestry").await?;
                assert!(item_again.iter().any(|line|line.contains("already got a reward this year")));
                let cupcake_again=super::classic_quests::talk(&mut owner,&deps.admitted,npc,9017,"lemon").await?;
                assert!(cupcake_again.iter().any(|line|line.contains("already got a cupcake this year")));
                let complete:(i16,bool,bool,bool)=sqlx::query_as("SELECT cities,started,item_claimed,cupcake_claimed FROM game_cake_errands WHERE world_id=encode($1,'hex')::uuid AND event_year=2026 AND character_id=encode($2,'hex')::uuid")
                    .bind(id(42).as_slice()).bind(id(41).as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(complete,(31,true,true,true),"the full personal journey must finish both independent annual rewards");
                let rewards:Vec<String>=sqlx::query_scalar("SELECT reward_key FROM game_cake_reward_claims WHERE world_id=encode($1,'hex')::uuid AND event_year=2026 AND character_id=encode($2,'hex')::uuid ORDER BY category")
                    .bind(id(42).as_slice()).bind(id(41).as_slice()).fetch_all(&h.pool).await?;
                assert_eq!(rewards,["oteryn:item.tibia.i20347","oteryn:item.tibia.i28484"]);
                let audited:i64=sqlx::query_scalar("SELECT count(*) FROM game_cake_reward_claims c JOIN game_reward_claim_mint_receipts r USING(transaction_id) JOIN game_item_audit_outbox a USING(transaction_id) WHERE c.character_id=encode($1,'hex')::uuid")
                    .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(audited,2,"replay and alternate choices never duplicate a normal audited MINT");
                let achievements:Vec<String>=sqlx::query_scalar("SELECT a.achievement_key FROM game_account_achievements a JOIN game_character_roots c USING(account_id) WHERE c.character_id=encode($1,'hex')::uuid ORDER BY a.achievement_key")
                    .bind(id(41).as_slice()).fetch_all(&h.pool).await?;
                assert_eq!(achievements,["oteryn:achievement/cake_conqueror","oteryn:achievement/i_did_my_part"]);
                let grants:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_achievement_grant_requests WHERE source_kind='oteryn:cake-isle-step' AND character_id=encode($1,'hex')::uuid")
                    .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(grants,2,"five island entries grant each account achievement only once");
                // The next community occurrence is historical input too. Personal
                // progress and annual MINT admission are exercised afresh below.
                let mut next = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *next).await?;
                sqlx::query("INSERT INTO game_cake_world_progress VALUES(encode($1,'hex')::uuid,2027,200,200)")
                    .bind(id(42).as_slice()).execute(&mut *next).await?;
                sqlx::query("INSERT INTO game_cake_phases(world_id,event_year,stage,layer,brutus_present) VALUES(encode($1,'hex')::uuid,2027,3,4,true)")
                    .bind(id(42).as_slice()).execute(&mut *next).await?;
                next.commit().await?;
                super::classic_quests::talk(&mut owner,&deps.admitted,npc,9030,"help").await?;
                for (x,y) in machines {
                    travel(&owner,deps.actor,deps.admitted.game_session_id,MovementLocalPosition{y:mark.y+1,..mark}).await?;
                    step(&mut owner,deps.actor,deps.admitted.game_session_id,StepDirection::North).await?;
                    travel(&owner,deps.actor,deps.admitted.game_session_id,MovementLocalPosition{x,y:y-1,floor:7}).await?;
                    step(&mut owner,deps.actor,deps.admitted.game_session_id,StepDirection::South).await?;
                }
                travel(&owner,deps.actor,deps.admitted.game_session_id,MovementLocalPosition{x:33303,y:32063,floor:7}).await?;
                super::classic_quests::talk(&mut owner,&deps.admitted,npc,9031,"hi").await?;
                owner.states.npc_conversations.fixture_interrupt_cake_brutus_commit(deps.admitted.game_session_id,9032)?;
                owner.clock_origin=owner.clock_origin.checked_sub(std::time::Duration::from_secs(2)).ok_or("claim clock")?;
                let fence=owner.current_quest_fence(deps.admitted.game_session_id).await.map_err(|_|"reward fence")?.ok_or("reward fence")?;
                let command=UseCommand{game_session_id:deps.admitted.game_session_id,command_id:9032,item_fence:Some(fence.item_fence())};
                let lost=owner.npc_talk(deps.actor,command,oteryn_protocol_oteryn::npc_service::NpcTalkIntent{npc_actor:npc,text:"tapestry".into()}).await;
                assert!(lost.unknown,"lose only the reply after the real annual MINT committed");
                let next_claim:Vec<String>=sqlx::query_scalar("SELECT reward_key FROM game_cake_reward_claims WHERE character_id=encode($1,'hex')::uuid AND event_year=2027")
                    .bind(id(41).as_slice()).fetch_all(&h.pool).await?;
                assert_eq!(next_claim,["oteryn:item.tibia.i20350"],"last year's claim must not consume the new annual entitlement");
                sqlx::query("UPDATE game_cake_phases SET event_end_cleaned_up=true,brutus_present=false WHERE world_id=encode($1,'hex')::uuid AND event_year=2027")
                    .bind(id(42).as_slice()).execute(&h.pool).await?;
                assert!(owner.refresh_cake_bridge().await,"cleanup reaches the real fenced map owner");
                assert!(deps.runtime.lock().await.visible_entities().npcs.iter().all(|entry|entry.actor.placement_identity()!=npc.identity));
                let recovered=owner.resume_npc_bank(deps.actor,command).await.ok_or("retained Cake reward")?;
                assert!(!recovered.unknown,"a committed claim must reconcile after Brutus has disappeared");
                assert_eq!(recovered.disposition,oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Ok);
                assert!(recovered.inventory_changed);
                assert!(recovered.conversation.as_ref().is_some_and(|conversation|conversation.open.is_none()),"reconciliation never reopens the departed NPC");
                assert!(owner.resume_npc_bank(deps.actor,command).await.is_none(),"a known reply clears retained work");
                let total:i64=sqlx::query_scalar("SELECT count(*) FROM game_cake_reward_claims c JOIN game_reward_claim_mint_receipts r USING(transaction_id) JOIN game_item_audit_outbox a USING(transaction_id) WHERE c.character_id=encode($1,'hex')::uuid")
                    .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(total,3,"recovery never repeats an Item MINT or audit event");
            }
            h.cleanup().await?;
            assert!(started.elapsed()>std::time::Duration::from_millis(10));
            eprintln!("M57_CAKE_FULL_JOURNEY_POSTGRES_COMPLETED elapsed={:?}",started.elapsed());
            Ok(())
        }).map_err(|error|error.to_string())
    })?.join().map_err(|_|"M57 journey thread panicked")??;
    Ok(())
}

async fn step(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    direction: StepDirection,
) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("movement clock")?;
    let (outcome, _) = owner
        .ordinary_paced_step(actor, session, direction, None, None)
        .await;
    outcome.moved_to.ok_or("accepted journey STEP")?;
    Ok(())
}
async fn travel(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    destination: MovementLocalPosition,
) -> TestResult {
    let _permit = owner.spell_lane_permit().await.ok_or("travel owner lane")?;
    let map = owner.bundle_map().ok_or("journey map")?;
    let closed = crate::map::floor::BundleDoors(map.facts.doors());
    let mut runtime = owner.runtime.lock().await;
    let before = runtime.read_actor_position(actor).map_err(debug)?;
    let proof = crate::map::floor::prepare_quest_callback_teleport_facing(
        &runtime,
        owner.movement_cells(),
        actor,
        session,
        before,
        destination,
        false,
        &closed,
        None,
    )
    .map_err(debug)?;
    runtime.commit_source_step(proof).map_err(debug)?;
    Ok(())
}
