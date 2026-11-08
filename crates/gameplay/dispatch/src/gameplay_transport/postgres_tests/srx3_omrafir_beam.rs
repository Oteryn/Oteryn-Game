//! Omrafir's real restored native attack must install the canonical marker and retain its four-second beam.
use super::fixture::{Harness, debug, id, run};
use super::{TestResult, prepare_current_session};
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

struct CaseDuration {
    slug: &'static str,
    started: std::time::Instant,
}
impl Drop for CaseDuration {
    fn drop(&mut self) {
        eprintln!(
            "SRX3_OMRAFIR_POSTGRES_CASE_EXECUTED slug={} duration={:?}",
            self.slug,
            self.started.elapsed()
        );
    }
}
fn omrafir_case(explode: bool, exact_wall: Option<bool>, terminal: bool) -> TestResult {
    omrafir_attachment_case(explode, exact_wall, terminal, true)
}
fn omrafir_attachment_case(
    explode: bool,
    exact_wall: Option<bool>,
    terminal: bool,
    review_declared_event: bool,
) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let _duration = CaseDuration {
        started,
        slug: match exact_wall {
            Some(true) => "exact-wall",
            Some(false) => "no-exact-wall",
            None if terminal => "terminal-known-source-error",
            None if explode => "think-boom-reform",
            None => "beam",
        },
    };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || {
        run(async |admin| {
            // Positive exact-ID case uses a genuine published stone wall stack
            // and its adjacent actual dirt-floor tile, never a fabricated fire.
            let origin=if exact_wall==Some(true) {MovementLocalPosition{x:32501,y:32002,floor:15}}
                else {MovementLocalPosition{x:33587,y:32379,floor:12}};
            let scene=super::srx3_soul_war_world_fixture::source_world(origin)?;
            let mut deps=scene.dependencies()?;
            let active=scene.controller.active().ok_or("actual World activation")?;
            let native=active.native_gameplay().ok_or("actual Omrafir native")?;
            let room=scene.room.clone();
            let cells=room.movement_cells().clone();
            deps.combat_room=Some(room.clone());
            let content=scene.pin.clone();
            let context=active.qualify_published_world_context(&room,&content).ok_or("actual published World frame")?;
            let frame=context.source_frame();
            let source_origin=[origin.x,origin.y,i32::from(origin.floor)];
            let (x,y,floor)=frame.physical_position(source_origin).ok_or("source player projection")?;
            let origin=MovementLocalPosition{x,y,floor};
            assert_eq!(frame.source_position((x,y,floor)),Some(source_origin));
            let source_caster=if exact_wall==Some(true){[32500,32002,15]}else{[33587,32378,12]};
            let (x,y,floor)=frame.physical_position(source_caster).ok_or("source raid actor projection")?;
            let caster_at=MovementLocalPosition{x,y,floor};
            // Current SQL/session/lease proofs follow the expensive native catalogue load.
            let h = Harness::create_with_admission_seed(admin, "srx3_omrafir_native_beam", false, false).await?;
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
            let caster = runtime.spawn_arena_opponent("oteryn:creature.omrafir",
                caster_at, |_,_|Some(true)).map_err(debug)?;
            if explode {
                let maximum=runtime.companion_snapshot(caster).map_err(debug)?.maximum_health;
                runtime.set_test_creature_health(caster,if terminal {maximum*3/100} else {maximum/2}).map_err(debug)?;
            }
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
            if review_declared_event && explode {
                let runtime = owner.runtime.lock().await;
                let callbacks = owner.states.spell_states.lock().await.native_callbacks.clone();
                callbacks.enable_omrafir_declared_think_review_for_test(&runtime, &room, native)
                    .await.map_err(|()| "explicit review-only Omrafir declaration")?;
            }
            owner.clock_origin -= std::time::Duration::from_secs(10);
            let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index)=room.movement_cells().index() else {return Err("actual World Bundle".into())};
            use crate::map::boot::BundleCollisionIndexExt;
            let facts=&index.map().facts;
            if let Some(expected)=exact_wall {
                let mut selected=false;
                let mut literal_say=false;
                for _ in 0..128 {
                    {
                        let mut runtime=owner.runtime.lock().await;
                        let maximum=runtime.companion_snapshot(caster).map_err(debug)?.maximum_health;
                        runtime.set_test_creature_health(caster,maximum-20_000).map_err(debug)?;
                    }
                    owner.drain_monster_melee().await;
                    let states=owner.states.spell_states.lock().await;
                    selected|=states.monster_melee.creature_ai.last_report_for_test(caster)
                        .is_some_and(|report|report.proposals.iter().any(|p|p.ability.key=="oteryn:ability.spell.omrafir_healing_2"));
                    literal_say=facts.movement_speech_since(0).iter().any(|(_,line)|line.text=="Omrafir gains new strength from the fire");
                    drop(states);
                    if selected && (!expected || literal_say){break;}
                    owner.clock_origin-=std::time::Duration::from_secs(2);
                }
                assert!(selected,"actual restored native defense must select its exact-item healing callback");
                assert_eq!(literal_say,expected,"only actual1487/1492/1500 stack qualifies; Rosha ordinary ground never substitutes fire");
            } else if !review_declared_event {
                {
                    let mut runtime = owner.runtime.lock().await;
                    let maximum = runtime.companion_snapshot(caster).map_err(debug)?.maximum_health;
                    runtime.set_test_creature_health(caster, maximum*3/100).map_err(debug)?;
                }
                let speech_cursor = facts.movement_speech_since(0).last().map_or(0,|entry|entry.0);
                owner.drain_monster_melee().await;
                assert!(owner.states.spell_states.lock().await.monster_melee.creature_ai
                    .last_report_for_test(caster).is_some(), "actual reserved native think must reach the source-event boundary");
                owner.clock_origin -= std::time::Duration::from_millis(11);
                owner.drain_monster_melee().await;
                assert!(owner.runtime.lock().await.contains_live_creature(caster),
                    "OmrafirExplode declaration is not attached by pinned monster.events; ordinary source actor must remain live");
                assert!(!facts.movement_speech_since(speech_cursor).iter().any(|(_,line)|line.text=="OMRAFIR EXPLODES INTO FLAMES!"),
                    "unattached event must not publish the review engine's explosion");
            } else if explode {
                let mut current=caster;
                for cycle in 0..if terminal {4} else {1} {
                    if cycle>0 {
                        let mut runtime=owner.runtime.lock().await;
                        let maximum=runtime.companion_snapshot(current).map_err(debug)?.maximum_health;
                        runtime.set_test_creature_health(current,maximum*3/100).map_err(debug)?;
                    }
                    let speech_cursor=facts.movement_speech_since(0).last().map_or(0,|entry|entry.0);
                    owner.drain_monster_melee().await;
                    owner.clock_origin-=std::time::Duration::from_millis(11);
                    owner.drain_monster_melee().await;
                    assert!(facts.movement_speech_since(speech_cursor).iter().any(|(_,line)|line.text=="OMRAFIR EXPLODES INTO FLAMES!"),
                        "actual reserved source onThink must retain its10ms explosion");
                    {
                        let runtime=owner.runtime.lock().await;
                        assert!(!runtime.contains_live_creature(current),"source boom administratively removes original actor");
                        if cycle==0 {
                            let fighters=runtime.positioned_actor_census().map_err(debug)?.iter().filter(|(actor,_,_)|
                                runtime.matches_live_creature_identity(*actor,b"oteryn:creature.hellfire_fighter")).count();
                            assert_eq!(fighters,9,"all nine actual extended+forced source births survive occupied centers");
                        }
                    }
                    let callbacks=owner.states.spell_states.lock().await.native_callbacks.clone();
                    assert_eq!(callbacks.omrafir_terminal_failure_for_test(current).await,None,
                        "terminal failure is not emitted before the original26764ms deadline");
                    owner.clock_origin-=std::time::Duration::from_millis(26764);
                    owner.drain_monster_melee().await;
                    let next={
                        let runtime=owner.runtime.lock().await;
                        assert!(!runtime.positioned_actor_census().map_err(debug)?.iter().any(|(actor,_,_)|
                            runtime.current_live_creature_identity(*actor).is_ok_and(|key|key==b"oteryn:creature.omrafir2")),
                            "undefined donor Omrafir2 must never gain fabricated statistics");
                        runtime.positioned_actor_census().map_err(debug)?.into_iter().find_map(|(actor,_,_)|
                            (actor!=current && runtime.matches_live_creature_identity(actor,b"oteryn:creature.omrafir")).then_some(actor))
                    };
                    if terminal && cycle==3 {
                        assert_eq!(callbacks.omrafir_terminal_failure_for_test(current).await,
                            Some("unknown-monster-nil-then-lua-error"),
                            "fourth branch follows real shared storage-1→0→1→2 and records the pinned nil/say failure");
                        assert_eq!(next,None,"source known failure creates no normal reform either");
                    } else {
                        assert_eq!(callbacks.omrafir_terminal_failure_for_test(current).await,None);
                        current=next.ok_or("source reform is parent-independent at26764ms")?;
                    }
                }
            } else {
            let mut selected=false;
            for _ in 0..128 {
                owner.drain_monster_melee().await;
                let runtime=owner.runtime.lock().await;
                let states=owner.states.spell_states.lock().await;
                if let Some(report)=states.monster_melee.creature_ai.last_report_for_test(caster){
                    selected=report.proposals.iter().any(|p|p.ability.key=="oteryn:ability.spell.omrafir_beam");
                }
                if selected {
                    assert!(runtime.actor_conditions(caster,None).map_err(debug)?.instances().iter().any(|condition|
                        serde_json::to_value(condition.definition().condition_type()).ok()
                            ==Some(serde_json::json!({"SpellRegeneration":88888}))),
                        "actual Omrafir beam must install SAME canonical marker88888 before its retained callback");
                    assert!(facts.movement_speech_since(0).iter().any(|(_,line)|line.text=="OMRAFIR INHALES DEEPLY!"),
                        "actual source starts with its literal monster yell");
                    break;
                }
                drop(states);drop(runtime);
                owner.clock_origin-=std::time::Duration::from_secs(2);
            }
            assert!(selected,"actual restored Omrafir native schedule must select beam");
            let speech_cursor=facts.movement_speech_since(0).last().map_or(0,|entry|entry.0);
            owner.clock_origin-=std::time::Duration::from_secs(4);
            owner.drain_monster_melee().await;
            assert!(facts.movement_speech_since(speech_cursor).iter().any(|(_,line)|line.text=="OMRAFIR BREATHES INFERNAL FIRE"),"actual retained source beam must fire at4seconds");
            }
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        }).map_err(|error|error.to_string())
    })?.join().map_err(|_|"Omrafir callback thread panicked")??;
    eprintln!(
        "SRX3_OMRAFIR_NATIVE_BEAM POSTGRES_CASE_COMPLETED duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn srx3_omrafir_actual_native_beam_installs_shared_marker_and_fires_after_four_seconds_postgres()
-> TestResult {
    omrafir_case(false, None, false)
}
#[test]
fn srx3_omrafir_review_declared_think_explodes_and_reforms_after_original_retirement_postgres()
-> TestResult {
    omrafir_case(true, None, false)
}

#[test]
fn srx3_omrafir_actual_native_exact_stone_wall_heal_postgres() -> TestResult {
    omrafir_case(false, Some(true), false)
}
#[test]
fn srx3_omrafir_actual_native_rosha_no_exact_wall_does_not_heal_callback_postgres() -> TestResult {
    omrafir_case(false, Some(false), false)
}

#[test]
fn srx3_omrafir_review_declared_terminal_undefined_type_records_source_failure_postgres()
-> TestResult {
    omrafir_case(true, None, true)
}

#[test]
fn srx3_omrafir_unattached_think_event_is_inactive_in_production_postgres() -> TestResult {
    omrafir_attachment_case(true, None, false, false)
}
