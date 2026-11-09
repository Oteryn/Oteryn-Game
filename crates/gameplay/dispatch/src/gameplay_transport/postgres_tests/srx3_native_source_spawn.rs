//! Actual native proposals must reach the source spawn owner and physical carrier.
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
            "SRX3_NATIVE_SOURCE_SPAWN POSTGRES_CASE_EXECUTED slug={} duration={:?}",
            self.slug,
            self.started.elapsed()
        );
    }
}

fn source_spawn_case(
    source_kind: crate::content::bosses::activity_template::PinnedActivitySourceRoom,
    slug: &'static str,
    ability: &'static str,
    child: &'static str,
    origin: MovementLocalPosition,
    caster_at: MovementLocalPosition,
) -> TestResult {
    source_spawn_case_with_gaz_heal(source_kind, slug, ability, child, origin, caster_at, false)
}

#[allow(
    clippy::too_many_arguments,
    reason = "test-only native scenario preserves existing source spawn callers"
)]
fn source_spawn_case_with_gaz_heal(
    source_kind: crate::content::bosses::activity_template::PinnedActivitySourceRoom,
    slug: &'static str,
    ability: &'static str,
    child: &'static str,
    origin: MovementLocalPosition,
    caster_at: MovementLocalPosition,
    gaz_heal: bool,
) -> TestResult {
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
            let h = Harness::create_with_admission_seed(admin, &format!("srx3_native_spawn_{slug}"), false, false).await?;
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
            let original_pose = runtime.read_actor_position(caster).map_err(debug)?;
            if gaz_heal {
                // A bounded injury is only the source HEAL-test precondition;
                // no source stats, marker, clock or callback result is injected.
                let maximum=runtime.companion_snapshot(caster).map_err(debug)?.maximum_health;
                runtime.set_test_creature_health(caster,maximum/10).map_err(debug)?;
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
            owner.clock_origin -= std::time::Duration::from_secs(10);
            if gaz_heal {
                owner.drain_monster_melee().await;
                {
                    let states=owner.states.spell_states.lock().await;
                    assert!(states.monster_melee.creature_ai.last_report_for_test(caster)
                        .is_some_and(|report|report.sequence>0),"real native reserved think must precede source-heal assertion");
                }
                let before={
                    let runtime=owner.runtime.lock().await;
                    let marker=runtime.actor_conditions(caster,None).map_err(debug)?;
                    assert!(marker.instances().iter().any(|condition|
                        serde_json::to_value(condition.definition().condition_type()).ok()
                            ==Some(serde_json::json!({"SpellRegeneration":88888}))),
                        "attached GazHaragothHeal must install SAME canonical seven-second marker during actual onThink");
                    runtime.companion_snapshot(caster).map_err(debug)?.health
                };
                use crate::map::boot::BundleCollisionIndexExt;
                let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index)=room.movement_cells().index()
                    else{return Err("actual source map cue owner".into())};
                let facts=&index.map().facts;
                assert!(facts.movement_speech_since(0).iter().any(|(_,line)|
                    line.text=="Gaz'haragoth begins to draw on the nightmares to HEAL himself!"),"literal source start yell");
                let cursor=facts.movement_speech_since(0).last().map_or(0,|entry|entry.0);
                owner.clock_origin-=std::time::Duration::from_secs(7);
                owner.drain_monster_melee().await;
                let runtime=owner.runtime.lock().await;
                let after=runtime.companion_snapshot(caster).map_err(debug)?;
                assert!(after.health>=(before+300000).min(after.maximum_health),"real retained seven-second source heal300000");
                assert!(facts.movement_speech_since(cursor).iter().any(|(_,line)|line.text=="Gaz'haragoth HEALS himself!"),"literal source completion yell");
                drop(runtime);
            } else {
            let mut selected = false;
            let mut created = false;
            for _ in 0..128 {
                owner.drain_monster_melee().await;
                let runtime = owner.runtime.lock().await;
                let states = owner.states.spell_states.lock().await;
                if let Some(report) = states.monster_melee.creature_ai.last_report_for_test(caster) {
                    if slug == "rat" && report.target == Some(deps.actor) {
                        assert_eq!(states.monster_melee.creature_ai.state(caster).ok_or("actual native AI state")?.attack_facing(),
                            crate::movement::CardinalStep::North, "actual stationary native target selection must look north");
                        let pose = runtime.read_actor_position(caster).map_err(debug)?;
                        assert_eq!(pose.position(), original_pose.position(), "a turn never moves the creature");
                        assert_eq!(pose.last_translation_revision(), original_pose.last_translation_revision(),
                            "publishing a native turn cannot fabricate a translation");
                        assert_eq!(pose.facing(), Some(crate::foundation::MovementFacing::North),
                            "actual native attack facing must reach the canonical pose read by delayed Lua callbacks");
                        selected = true;
                        created = true;
                        break;
                    }
                    selected |= report.proposals.iter().any(|proposal| proposal.ability.key == ability);
                }
                let child_key = format!("oteryn:creature.{child}");
                created = runtime.visible_entities().creatures.iter().any(|entity|
                    runtime.current_live_creature_identity(entity.actor).ok()
                        == Some(child_key.as_bytes()));
                drop(states);
                drop(runtime);
                if selected && created {break;}
                owner.clock_origin -= std::time::Duration::from_secs(2);
            }
            assert!(selected,"actual native scheduler must select {ability}");
            assert!(created,"selected {ability} must admit actual source child {child} through the production callback owner");
            }
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        }).map_err(|error|error.to_string())
    })?.join().map_err(|_|"native source spawn thread panicked")??;
    Ok(())
}

#[test]
fn srx3_gaz_actual_native_proposal_creates_source_minions_postgres() -> TestResult {
    source_spawn_case(
        crate::content::bosses::activity_template::PinnedActivitySourceRoom::BrainHead,
        "gaz_haragoth",
        "oteryn:ability.spell.gaz_haragoth_summon",
        "minion_of_gaz_haragoth",
        MovementLocalPosition {
            x: 31953,
            y: 32325,
            floor: 10,
        },
        MovementLocalPosition {
            x: 31953,
            y: 32324,
            floor: 10,
        },
    )
}
#[test]
fn srx3_generator_actual_native_proposal_creates_energy_pulse_postgres() -> TestResult {
    source_spawn_case(
        crate::content::bosses::activity_template::PinnedActivitySourceRoom::BrainHead,
        "glooth_generator",
        "oteryn:ability.spell.glooth_generator_summon",
        "energy_pulse",
        MovementLocalPosition {
            x: 31953,
            y: 32325,
            floor: 10,
        },
        MovementLocalPosition {
            x: 31953,
            y: 32324,
            floor: 10,
        },
    )
}
#[test]
fn srx3_maxxen_actual_native_proposal_creates_source_generator_postgres() -> TestResult {
    source_spawn_case(crate::content::bosses::activity_template::PinnedActivitySourceRoom::CallbackProfessorMaxxen,
        "professor_maxxen","oteryn:ability.spell.generator","glooth_generator",
        MovementLocalPosition{x:33711,y:32052,floor:15},MovementLocalPosition{x:33711,y:32051,floor:15})
}

#[test]
fn srx3_stationary_native_target_selection_publishes_source_facing_without_translation_postgres()
-> TestResult {
    // The same real source map/current session/native owner as the spawn cases;
    // Rat's unchanged pinned melee policy supplies a stationary adjacent target.
    source_spawn_case(
        crate::content::bosses::activity_template::PinnedActivitySourceRoom::BrainHead,
        "rat",
        "",
        "",
        MovementLocalPosition {
            x: 31953,
            y: 32324,
            floor: 10,
        },
        MovementLocalPosition {
            x: 31953,
            y: 32325,
            floor: 10,
        },
    )
}

#[test]
fn srx3_gaz_actual_reserved_think_installs_shared_marker_and_heals_after_seven_seconds_postgres()
-> TestResult {
    source_spawn_case_with_gaz_heal(
        crate::content::bosses::activity_template::PinnedActivitySourceRoom::BrainHead,
        "gaz_haragoth",
        "",
        "",
        MovementLocalPosition {
            x: 31953,
            y: 32325,
            floor: 10,
        },
        MovementLocalPosition {
            x: 31953,
            y: 32324,
            floor: 10,
        },
        true,
    )
}
