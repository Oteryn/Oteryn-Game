//! Literal attached HealthForgotten event through canonical native HP owners.
use super::*;
use crate::content_adapters::NativeGameplayRuntimeExt;
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
        let native=oteryn_gameplay_content::content::native_gameplay::retained_callback_native_fixture_at(world,&manifest);
        let mut runtime=ChannelRuntimeV1::from_committed_assignment(world,ChannelId::decode(&id(71)).map_err(debug)?,NodeId::decode(&id(72)).map_err(debug)?,1,1,1,"runtime-scope-assignment:1",8,
            ChannelContentPin::from_activation(world,1,native.source_digest(),[2;32],[3;32],[4;32],(100,100,7))).map_err(debug)?;
        native.install_companion_policies(&mut runtime).map_err(debug)?;
        let session=GameSessionId::decode(&id(73)).map_err(debug)?;
        let reserved=runtime.reserve_fresh_session(session).map_err(debug)?;
        let player=runtime.commit_fresh_session(reserved).map_err(debug)?;
        let character=CharacterId::decode(&id(74)).map_err(debug)?;
        runtime.bind_attacker_lease(player,session,CharacterLease::new(character,1).map_err(debug)?).map_err(debug)?;
        runtime.initialize_pinned_test_position(player,MovementLocalPosition{x:99,y:100,floor:7}).map_err(debug)?;
        let maximum=runtime.companion_policy(key).map_err(debug)?.maximum_health;
        let actor=runtime.admit_source_pinned_lab_creature(MovementLocalPosition{x:100,y:100,floor:7},key,maximum).map_err(debug)?;
        runtime.install_creature_policy(actor,key).map_err(debug)?;
        runtime.set_test_creature_health(actor,1000).map_err(debug)?;
        let guard=if dead_guard {
            let guard_key="oteryn:creature.shadow_tentacle";
            let max=runtime.companion_policy(guard_key).map_err(debug)?.maximum_health;
            let guard=runtime.admit_source_pinned_lab_creature(MovementLocalPosition{x:107,y:107,floor:7},guard_key,max).map_err(debug)?;
            runtime.install_creature_policy(guard,guard_key).map_err(debug)?;
            runtime.borrow_exact_actor_commit_at(10).commit_damage(guard,OwnerDamageCommand{target:guard_key.as_bytes(),occurrence:b"health-forgotten:guard-death",binding:b"health-forgotten:guard-death",damage:max}).map_err(debug)?;
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
        let change=if heal {
            runtime_actor_spell_types::OwnerCombatChange::Heal{target_atom:key.into(),magnitude:100}
        } else {
            runtime_actor_spell_types::OwnerCombatChange::Damage{target_atom:key.into(),magnitude:100}
        };
        let batch=runtime_actor_spell_types::OwnerCombatBatch {
            caster:player,attacker:character,current_lease_generation:1,
            command:CommandRef::new(session,oteryn_gameplay_foundation::foundation::CommandId::new(1).map_err(debug)?),
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
