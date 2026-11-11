//! Actual native source proposal must transform and restore independently owned ground.
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
            "SRX3_NATIVE_SOURCE_MAP POSTGRES_CASE_EXECUTED slug={} duration={:?}",
            self.slug,
            self.started.elapsed()
        );
    }
}

fn source_map_case() -> TestResult {
    let source_kind =
        crate::content::bosses::activity_template::PinnedActivitySourceRoom::CallbackAnomaly;
    let slug = "charged_anomaly";
    let ability = "oteryn:ability.spell.charge_vortex";
    let origin = MovementLocalPosition {
        x: 32271,
        y: 31257,
        floor: 14,
    };
    let caster_at = MovementLocalPosition {
        x: 32271,
        y: 31256,
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
            let h = Harness::create_with_admission_seed(admin, &format!("srx3_native_vortex_{slug}"), false, false).await?;
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
            let mut selected = false;
            let anchors = [(32264,31253),(32269,31258),(32275,31255),(32280,31253),
                (32271,31248),(32264,31245),(32270,31240),(32269,31253),
                (32275,31245),(32276,31250),(32266,31249)];
            let read = || -> Result<Vec<_>,String> {
                anchors.iter().map(|(x,y)| crate::gameplay_transport::soul_war_scene::current_ground(
                    &room,MovementLocalPosition{x:*x,y:*y,floor:14}).ok_or_else(||format!("source ground {x},{y}"))).collect()
            };
            assert!(read()?.iter().all(|ground|ground.appearance_id == 22893),
                "all eleven actual captured source vortex grounds retain donor map22893");
            let mut changed = Vec::new();
            for _ in 0..128 {
                owner.drain_monster_melee().await;
                let runtime = owner.runtime.lock().await;
                let states = owner.states.spell_states.lock().await;
                if let Some(report) = states.monster_melee.creature_ai.last_report_for_test(caster) {
                    selected |= report.proposals.iter().any(|proposal| proposal.ability.key == ability);
                }
                changed = read()?.into_iter().filter(|ground|ground.appearance_id == 22894).collect();
                drop(states);
                drop(runtime);
                if selected && !changed.is_empty() {break;}
                owner.clock_origin -= std::time::Duration::from_secs(2);
            }
            assert!(selected,"actual native scheduler must select {ability}");
            assert!(!changed.is_empty(),"selected native charge vortex must transform actual captured current ground to22894");
            // Lua's addEvent closure retains the position, not the caster. Its real
            // canonical administrative removal cannot cancel the ten-second restoration.
            owner.runtime.lock().await.remove_encounter_actor(caster).map_err(debug)?;
            owner.clock_origin -= std::time::Duration::from_secs(11);
            owner.drain_monster_melee().await;
            assert!(read()?.iter().all(|ground|ground.appearance_id ==
                if changed.iter().any(|old|old.key == ground.key) {23049} else {22893}),
                "Lua restores touched current ground to23049 after10s, leaving untouched donor map22893 unchanged");
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        }).map_err(|error|error.to_string())
    })?.join().map_err(|_|"native source spawn thread panicked")??;
    Ok(())
}

#[test]
fn srx3_vortex_actual_native_callback_transforms_and_restores_current_source_ground_postgres()
-> TestResult {
    source_map_case()
}
