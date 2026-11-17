//! Actual reserved native onThink must execute the pinned Outburst phase event.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

struct CaseDuration {
    slug: &'static str,
    started: std::time::Instant,
}
impl Drop for CaseDuration {
    fn drop(&mut self) {
        eprintln!(
            "SRX3_NATIVE_OUTBURST_PHASE POSTGRES_CASE_EXECUTED slug={} duration={:?}",
            self.slug,
            self.started.elapsed()
        );
    }
}

fn outburst_phase_case() -> TestResult {
    let source_kind =
        crate::content::bosses::activity_template::PinnedActivitySourceRoom::CallbackOutburst;
    let slug = "outburst";
    let origin = MovementLocalPosition {
        x: 32234,
        y: 31292,
        floor: 14,
    };
    let caster_at = MovementLocalPosition {
        x: 32234,
        y: 31284,
        floor: 14,
    };
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let _duration = CaseDuration {
        slug,
        started: std::time::Instant::now(),
    };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || {
        run(async |admin| {
            let mut deps = Dependencies::new()?;
            let (controller, pin) = super::death_ml_loss_composed::production_generation_with_pin(deps.admitted.world_id, "content-1")?;
            let active = controller.active().ok_or("native activation")?;
            let native = active.native_gameplay().ok_or("native gameplay")?;
            let kind = source_kind;
            let source = active.qualify_activity_source_room(pin.qualified_room(), kind).map_err(debug)?;
            let room = crate::gameplay_transport::activity_runtime::source_bundle::qualify(
                &source, kind, active, deps.admitted.world_id, deps.admitted.channel_id,
            ).map_err(|_|"Brain Head exact source Bundle")?;
            let cells = room.movement_cells().clone();
            deps.room = pin.qualified_room().clone();
            deps.combat_room = Some(room.clone());
            let content = ChannelContentPin::from_activation(deps.admitted.world_id, 1,
                active.identity().server_artifact_digest(), active.identity().client_artifact_digest(),
                room.frame_binding().digest(), room.map_revision_digest(), (origin.x,origin.y,origin.floor));
            // Current SQL/session/lease proofs follow the expensive native catalogue load.
            let h = Harness::create_with_admission_seed(admin, &format!("srx3_native_outburst_{slug}"), false, false).await?;
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
                origin).map_err(debug)?;
            let key = format!("oteryn:creature.{slug}");
            let caster = runtime.spawn_arena_opponent(&key,caster_at,|_,_|Some(true)).map_err(debug)?;
            assert_eq!(runtime.companion_policy(&key).map_err(debug)?.maximum_health,290000);
            // Bounded injury is a documented phase-trigger precondition; maxHP,
            // source policy and damage attribution remain actual and unchanged.
            runtime.set_test_creature_health(caster,232000).map_err(debug)?;
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
            let mut thought=false;
            let mut phase=false;
            for _ in 0..128 {
                owner.drain_monster_melee().await;
                let runtime=owner.runtime.lock().await;
                let states=owner.states.spell_states.lock().await;
                thought |= states.monster_melee.creature_ai.last_report_for_test(caster).is_some();
                let mut charging=0;
                let mut sparks=0;
                for entity in runtime.visible_entities().creatures {
                    match runtime.current_live_creature_identity(entity.actor).ok() {
                        Some(b"oteryn:creature.charging_outburst") => charging+=1,
                        Some(b"oteryn:creature.spark_of_destruction") => sparks+=1,
                        _=>{},
                    }
                }
                phase=!runtime.contains_live_creature(caster) && charging==1 && sparks==4;
                drop(states);
                drop(runtime);
                if phase || thought {break;}
                owner.clock_origin -= std::time::Duration::from_secs(2);
            }
            assert!(thought || phase,"actual source actor must receive native reserved think work");
            assert!(phase,"OutburstCharge at80percent must retire original actor then force-birth fourSparks and distinctCharging actor");
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        }).map_err(|error|error.to_string())
    })?.join().map_err(|_|"native source spawn thread panicked")??;
    Ok(())
}

#[test]
fn srx3_outburst_actual_reserved_native_think_advances_source_phase_postgres() -> TestResult {
    outburst_phase_case()
}
