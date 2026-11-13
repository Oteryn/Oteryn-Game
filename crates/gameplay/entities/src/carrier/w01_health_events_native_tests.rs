//! Literal attached HealthForgotten event through canonical native HP owners.
use super::*;
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn debug(value: impl std::fmt::Debug) -> String {
    format!("{value:?}")
}
fn id(tag: u8) -> [u8; 16] {
    let mut bytes = [tag; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes
}
fn run(heal: bool, dead_guard: bool, completed_guard: bool) -> TestResult {
    run_source(
        heal,
        dead_guard,
        completed_guard,
        "oteryn:creature.lady_tenebris",
        true,
    )
}
fn run_source(
    heal: bool,
    dead_guard: bool,
    completed_guard: bool,
    key: &'static str,
    doubles: bool,
) -> TestResult {
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(),String> {
        let world=oteryn_gameplay_content::content::native_gameplay_test_world_id().map_err(debug)?;
        let manifest=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../content/spells.manifest.json");
        let native=oteryn_gameplay_content::content::native_gameplay::callback_native_policy_fixture_at(&manifest).map_err(debug)?;
        let mut runtime=ChannelRuntimeV1::from_committed_assignment(world,ChannelId::decode(&id(71)).map_err(debug)?,NodeId::decode(&id(72)).map_err(debug)?,1,1,1,"runtime-scope-assignment:1",8,
            ChannelContentPin::from_activation(world,1,native.source_digest(),[2;32],[3;32],[4;32],(100,100,7))).map_err(debug)?;
        runtime.install_companion_policies(native.policies().clone()).map_err(debug)?;
        let session=GameSessionId::decode(&id(73)).map_err(debug)?;
        let reserved=runtime.reserve_fresh_session(session).map_err(debug)?;
        let player=runtime.commit_fresh_session(reserved).map_err(debug)?;
        let character=CharacterId::decode(&id(74)).map_err(debug)?;
        runtime.bind_attacker_lease(player,session,CharacterLease::new(character,1).map_err(debug)?).map_err(debug)?;
        runtime.initialize_pinned_test_position(player,MovementLocalPosition{x:90,y:100,floor:7}).map_err(debug)?;
        let maximum=runtime.companion_policy(key).map_err(debug)?.maximum_health;
        let actor=runtime.admit_source_pinned_lab_creature(MovementLocalPosition{x:100,y:100,floor:7},key,maximum).map_err(debug)?;
        runtime.install_creature_policy(actor,key).map_err(debug)?;
        runtime.set_test_creature_health(actor,1000).map_err(debug)?;
        let guard=if dead_guard {
            let guard_key="oteryn:creature.shadow_tentacle";
            let max=runtime.companion_policy(guard_key).map_err(debug)?.maximum_health;
            let guard=runtime.admit_source_pinned_lab_creature(MovementLocalPosition{x:107,y:107,floor:7},guard_key,max).map_err(debug)?;
            runtime.install_creature_policy(guard,guard_key).map_err(debug)?;
            let guard_batch=runtime_actor_spell_types::OwnerCombatBatch {
                caster:player,attacker:character,current_lease_generation:1,
                command:CommandRef::new(session,oteryn_gameplay_foundation::foundation::CommandId::new(1).map_err(debug)?),
                occurrence:runtime_actor_spell_types::SpellOccurrenceBinding{id:"source.health-forgotten.guard-death".into(),
                    revisions:["rules:1","content:1","world:1","formula:1","sim:1"].map(str::to_owned)},
                binding:b"source.health-forgotten.guard-death".to_vec(),anchor:None,now_us:Some(oteryn_simulation_determinism::SemanticTimeMicros::from_micros(10)),now_ms:0,
                effects:vec![runtime_actor_spell_types::OwnerCombatEffect{target:guard,sub_ordinal:0,
                    change:runtime_actor_spell_types::OwnerCombatChange::Damage{target_atom:guard_key.into(),magnitude:max}}],deferred:None,
            };
            let guard_prepared=runtime.stage_spell_batch(&guard_batch).map_err(debug)?;
            runtime.commit_spell_batch(guard_prepared).map_err(debug)?;
            assert!(runtime.source_creature_is_registered(guard).map_err(debug)?,"queued onDeath leaves HP0 spectator registered");
            assert!(runtime.source_registered_creature_census().map_err(debug)?.iter().any(|(candidate,_)|*candidate==guard));
            Some(guard)
        } else {None};
        if completed_guard {
            let guard=guard.ok_or("real registered guard")?;
            let mut receipt=runtime.borrow_combat_death().committed_lethal_receipt(guard).map_err(debug)?;
            let death=receipt.projection_mut().occurrence.death_key().map_err(debug)?;
            runtime.complete_source_creature_death(guard,death).map_err(debug)?;
            assert!(!runtime.source_creature_is_registered(guard).map_err(debug)?,"actual queued source death completion removes spectator registration");
        }
        assert!(runtime.observe_native_health_change(player,&|_: ExactActorRef, _: MovementPositionSnapshot| None).map_err(debug)?.is_none(),"unrelated real Player has no native Creature event");
        let raw=runtime.admit_pinned_test_creature(MovementLocalPosition{x:150,y:150,floor:7}).map_err(debug)?;
        assert!(runtime.observe_native_health_change(raw,&|_: ExactActorRef, _: MovementPositionSnapshot| None).map_err(debug)?.is_none(),"unrelated raw Creature does not acquire a Companion prerequisite");
        runtime.remove_test_actor(raw).map_err(debug)?;
        assert!(runtime.observe_native_health_change(raw,&|_: ExactActorRef, _: MovementPositionSnapshot| None).is_err(),"retired exact incarnation cannot issue an observation");
        let unbound=runtime.admit_source_pinned_lab_creature(MovementLocalPosition{x:151,y:150,floor:7},key,maximum).map_err(debug)?;
        assert!(runtime.observe_native_health_change(unbound,&|_: ExactActorRef, _: MovementPositionSnapshot| None).is_err(),"a reserved source identity still requires actual policy metadata");
        // The actual bound Player is outside +/-7, so no invented Player name
        // is needed to qualify this current source spectator observation.
        let unknown_names = |_: ExactActorRef, _: MovementPositionSnapshot| None;
        let mut observation=runtime.observe_native_health_change(actor,&unknown_names).map_err(debug)?;
        if let Some(source)=observation.as_ref() {
            assert_eq!(source.primary(-100).map_err(debug)?,if doubles&&(guard.is_none()||completed_guard){-200}else{-100});
            assert_eq!(source.primary(0).map_err(debug)?,0);
            runtime.validate_native_health_change_observation(source).map_err(debug)?;
            let before=runtime.read_actor_position(actor).map_err(debug)?;
            runtime.borrow_movement_position().commit_creature_relocation(actor,before,MovementLocalPosition{x:96,y:100,floor:7}).map_err(debug)?;
            assert!(runtime.observe_native_health_change(actor,&unknown_names).is_err(),"unknown Player name INSIDE the spectator bounds cannot become no guard");
            assert!(runtime.validate_native_health_change_observation(source).is_err(),"a changed target/source cohort cannot reuse an old preprocessing observation");
            let moved=runtime.read_actor_position(actor).map_err(debug)?;
            runtime.borrow_movement_position().commit_creature_relocation(actor,moved,before.position()).map_err(debug)?;
            observation=runtime.observe_native_health_change(actor,&unknown_names).map_err(debug)?;
        }
        let primary=observation.as_ref().map_or(Ok(100),|source|source.primary(100)).map_err(debug)?;
        let change=if heal {
            runtime_actor_spell_types::OwnerCombatChange::Heal{target_atom:key.into(),magnitude:primary}
        } else {
            runtime_actor_spell_types::OwnerCombatChange::Damage{target_atom:key.into(),magnitude:primary}
        };
        let batch=runtime_actor_spell_types::OwnerCombatBatch {
            caster:player,attacker:character,current_lease_generation:1,
            command:CommandRef::new(session,oteryn_gameplay_foundation::foundation::CommandId::new(2).map_err(debug)?),
            occurrence:runtime_actor_spell_types::SpellOccurrenceBinding{id:"source.health-forgotten.combat".into(),
                revisions:["rules:1","content:1","world:1","formula:1","sim:1"].map(str::to_owned)},
            binding:b"source.health-forgotten.combat".to_vec(),anchor:None,now_us:None,now_ms:20,
            effects:vec![runtime_actor_spell_types::OwnerCombatEffect{target:actor,sub_ordinal:0,change}],deferred:None,
        };
        let prepared=runtime.stage_spell_batch(&batch).map_err(debug)?;
        runtime.commit_spell_batch(prepared).map_err(debug)?;
        let magnitude=if doubles&&(guard.is_none()||completed_guard){200}else{100};
        assert_eq!(runtime.live_creature_health(actor),Some(if heal{1000+magnitude}else{1000-magnitude}),
            "actual Combat HEALTHCHANGE doubles signed primary only when no registered guard exists");

        Ok(())
    })?.join().map_err(|_|"native HealthForgotten worker panicked")?.map_err(Into::into)
}
#[test]
fn srx3_health_forgotten_native_negative_damage_doubles_without_guard() -> TestResult {
    run(false, false, false)
}
#[test]
fn srx3_health_forgotten_native_positive_heal_doubles_without_guard() -> TestResult {
    run(true, false, false)
}
#[test]
fn srx3_health_forgotten_native_registered_hp0_guard_preserves_damage() -> TestResult {
    run(false, true, false)
}
#[test]
fn srx3_health_forgotten_native_registered_hp0_guard_preserves_heal() -> TestResult {
    run(true, true, false)
}

#[test]
fn srx3_health_forgotten_native_completed_guard_death_restores_damage_multiplier() -> TestResult {
    run(false, true, true)
}
#[test]
fn srx3_health_forgotten_native_completed_guard_death_restores_heal_multiplier() -> TestResult {
    run(true, true, true)
}

#[test]
fn srx3_rupture_native_default_resonance_negative_combat_has_no_preheal() -> TestResult {
    run_source(false, false, false, "oteryn:creature.rupture", false)
}
#[test]
fn srx3_rupture_native_default_resonance_positive_combat_has_no_preheal() -> TestResult {
    run_source(true, false, false, "oteryn:creature.rupture", false)
}
