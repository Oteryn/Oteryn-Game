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

fn source_teleport_case(maxxen: bool) -> TestResult {
    let (source_kind, slug, ability, origin, caster_at) = if maxxen {
        (crate::content::bosses::activity_template::PinnedActivitySourceRoom::CallbackProfessorMaxxen,
         "professor_maxxen","oteryn:ability.spell.maxxenteleport",
         MovementLocalPosition{x:33711,y:32052,floor:15}, MovementLocalPosition{x:33711,y:32050,floor:15})
    } else {
        (
            crate::content::bosses::activity_template::PinnedActivitySourceRoom::CallbackZamulosh,
            "zamulosh",
            "oteryn:ability.spell.zamulosh_tp",
            MovementLocalPosition {
                x: 33644,
                y: 32760,
                floor: 11,
            },
            MovementLocalPosition {
                x: 33644,
                y: 32759,
                floor: 11,
            },
        )
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
            let h = Harness::create_with_admission_seed(admin, &format!("srx3_native_teleport_{slug}"), false, false).await?;
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
            let mut relocated=false;
            let map=crate::gameplay_transport::soul_war_scene::source_map(&room).ok_or("actualsourceMap")?;
            for _ in 0..128 {
                let cursor=map.facts.movement_effects_since(0).last().map_or(0,|entry|entry.0);
                owner.drain_monster_melee().await;
                let runtime=owner.runtime.lock().await;
                let states=owner.states.spell_states.lock().await;
                if let Some(report)=states.monster_melee.creature_ai.last_report_for_test(caster){
                    selected|=report.proposals.iter().any(|proposal|proposal.ability.key==ability);
                }
                let at=runtime.read_actor_position(caster).map_err(debug)?.position();
                relocated=if maxxen {
                    at.floor==15&&(33704..=33718).contains(&at.x)&&(32040..=32053).contains(&at.y)
                }else{at==MovementLocalPosition{x:33644,y:32757,floor:11}};
                let cues=map.facts.movement_effects_since(cursor);
                let wire=crate::gameplay_transport::soul_war_scene::wire_position(at).ok_or("actual source floor")?;
                relocated &= cues.iter().any(|(_,_,effect)|*effect==3)
                    && cues.iter().any(|(_,position,effect)|*effect==11&&*position==wire);
                drop(states);drop(runtime);
                if selected{break;}
                owner.clock_origin-=std::time::Duration::from_secs(2);
            }
            assert!(selected,"actual native schedule must select {ability}");
            assert!(relocated,"selected {ability} must use source exact teleport owner and fixed qualified source frame");
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        }).map_err(|error|error.to_string())
    })?.join().map_err(|_|"native source spawn thread panicked")??;
    Ok(())
}

#[test]
fn srx3_maxxen_actual_native_callback_teleports_in_source_rectangle_postgres() -> TestResult {
    source_teleport_case(true)
}
#[test]
fn srx3_zamulosh_actual_native_callback_teleports_to_source_anchor_postgres() -> TestResult {
    source_teleport_case(false)
}
