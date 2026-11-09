//! W01c: the real native scheduler consumes Cerebellum's source callback.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

#[test]
fn srx3_cerebellum_native_callback_heals_brain_head_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
        run(async |admin| {
            let mut deps = Dependencies::new()?;
            let (controller, pin) = super::death_ml_loss_composed::production_generation_with_pin(deps.admitted.world_id, "content-1")?;
            let active = controller.active().ok_or("native activation")?;
            let native = active.native_gameplay().ok_or("native gameplay")?;
            let kind = crate::content::bosses::activity_template::PinnedActivitySourceRoom::BrainHead;
            let source = active.qualify_activity_source_room(pin.qualified_room(), kind).map_err(debug)?;
            let room = crate::gameplay_transport::activity_runtime::source_bundle::qualify(
                &source, kind, active, deps.admitted.world_id, deps.admitted.channel_id,
            ).map_err(|_|"Brain Head exact source Bundle")?;
            let cells = room.movement_cells().clone();
            deps.room = pin.qualified_room().clone();
            deps.combat_room = Some(room.clone());
            let content = ChannelContentPin::from_activation(deps.admitted.world_id, 1,
                active.identity().server_artifact_digest(), active.identity().client_artifact_digest(),
                room.frame_binding().digest(), room.map_revision_digest(), (31963,32324,10));
            // Current SQL/session/lease proofs follow the expensive native catalogue load.
            let h = Harness::create_with_admission_seed(admin, "srx3_brain_heal", false, false).await?;
            super::reset();
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let mut runtime = ChannelRuntimeV1::from_committed_assignment(deps.admitted.world_id, deps.admitted.channel_id,
                crate::foundation::NodeId::decode(&id(1)).map_err(debug)?, 1,1,1,"runtime-scope-assignment:1",32,content).map_err(debug)?;
            native.install_companion_policies(&mut runtime).map_err(debug)?;
            let reservation = runtime.reserve_fresh_session(deps.admitted.game_session_id).map_err(debug)?;
            deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime.bind_attacker_lease(deps.actor,deps.admitted.game_session_id,
                crate::foundation::CharacterLease::new(crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,2).map_err(debug)?).map_err(debug)?;
            runtime.initialize_pinned_test_position(deps.actor,
                MovementLocalPosition{x:31953,y:32325,floor:10}).map_err(debug)?;
            let cerebellum = runtime.spawn_arena_opponent("oteryn:creature.cerebellum",
                MovementLocalPosition{x:31953,y:32324,floor:10}, |_,_|Some(true)).map_err(debug)?;
            let brain = runtime.spawn_arena_opponent("oteryn:creature.brain_head",
                MovementLocalPosition{x:31954,y:32325,floor:10}, |_,_|Some(true)).map_err(debug)?;
            runtime.set_test_creature_health(brain, 1_000).map_err(debug)?;
            deps.admitted.runtime_actor = Some(deps.actor);
            deps.runtime = tokio::sync::Mutex::new(runtime);
            let mut owner = deps.owner(&h,&authority);
            owner.active_generation = Some(active);
            owner.movement_cells = (&cells).into();
            {
                let runtime = owner.runtime.lock().await;
                owner.states.spell_states.lock().await.initialize(&runtime,deps.actor,deps.admitted.game_session_id,
                    crate::spell::cast::CharacterCastFacts {max_health:oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
                        ..crate::gameplay_transport::actor_spell::tests::FACTS},(0,0),owner.owner_now()).ok_or("player spell state")?;
            }
            owner.clock_origin -= std::time::Duration::from_secs(10);
            let mut health = 1_000;
            let mut selected_callback = false;
            for _ in 0..128 {
                owner.drain_monster_melee().await;
                {
                    let runtime = owner.runtime.lock().await;
                    runtime.companion_snapshot(cerebellum).map_err(debug)?;
                    let states = owner.states.spell_states.lock().await;
                    let report = states.monster_melee.creature_ai.last_report_for_test(cerebellum)
                        .ok_or("actual native Cerebellum think report")?;
                    selected_callback |= report.proposals.iter().any(|proposal|
                        proposal.ability.key == "oteryn:ability.creature.cerebellum.field-fill-heal_brain_head");
                }
                let runtime = owner.runtime.lock().await;
                health = runtime.live_creature_health(brain).ok_or("live Brain Head")?;
                if health > 1_000 { break; }
                assert!(runtime.contains_live_creature(cerebellum));
                drop(runtime);
                owner.clock_origin -= std::time::Duration::from_secs(2);
            }
            assert!(selected_callback,"the real native scheduler must select the source callback before its consumer is tested");
            assert!((1_300..=1_500).contains(&health), "real source callback must heal 300..500: {health}");
            let before = health;
            owner.drain_monster_melee().await;
            assert_eq!(owner.runtime.lock().await.live_creature_health(brain),Some(before),"same native think cannot heal twice");
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        }).map_err(|error|error.to_string())
    })?.join().map_err(|_|"Cerebellum callback thread panicked")??;
    eprintln!(
        "SRX3_BRAIN_HEAL POSTGRES_CASE_COMPLETED duration={:?}",
        started.elapsed()
    );
    Ok(())
}
