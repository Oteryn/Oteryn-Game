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
fn periodic(dead_guard: bool, completed_guard: bool) -> TestResult {
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(),String> {
        let key="oteryn:creature.lady_tenebris";
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
        runtime.initialize_pinned_test_position(player,MovementLocalPosition{x:80,y:100,floor:7}).map_err(debug)?;
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

        use oteryn_gameplay_foundation::foundation::condition::{ApplicationFacts, ConditionDefinition, ConditionSourceKind, TickFacts};
        use oteryn_gameplay_domain::domain::condition::{ConditionValues, DotElement};
        use oteryn_simulation_determinism::{GameplayDecisionRoot, DecisionOccurrenceId};
        let before=runtime.companion_snapshot(actor).map_err(debug)?;
        // Existing origin/main canonical profile, including the producer's adopt_wiki overlay
        // (canary_batch.py:1296), is unchanged by this callback test. The pinned Canary
        // base Lua has physical0 / unspecified mitigation: existing profile provenance is
        // wiki-pending; this test does not claim those base Lua values are 2.32 / 20.
        let mitigation=before.state.policy.mitigation.as_ref().ok_or("canonical mitigation")?;
        assert_eq!((mitigation.numerator,mitigation.denominator),(58,25));
        let physical=before.state.policy.resistances.iter().find(|r|r.damage_type=="physical").ok_or("canonical physical resistance")?;
        assert_eq!((physical.percent.numerator,physical.percent.denominator),(20,1));
        // Independent canonical rounding: trunc(7 - float32(7)*2.32/100)=6;
        // round(6*(100-20)/100)=5. HealthForgotten's signed-primary x2 is applied
        // after these steps, once, before HP: Canary47df data-otservbr-global/scripts/
        // quests/forgotten_knowledge/creaturescripts_healthchange_forgotten.lua:21.
        let mut after=before.state.clone();
        let definition=ConditionDefinition::new("test.source.health-forgotten.periodic",1,
            ConditionValues::DamageOverTime {element:DotElement::Bleeding,total_min:21,total_max:21,per_tick:7,interval_ms:1000,delayed:true}).ok_or("valid literal periodic definition")?;
        let root=GameplayDecisionRoot::from_bytes([7;32]);
        after.conditions.apply(&definition,Some(player),ConditionSourceKind::Player,&[],&ApplicationFacts{
            now:0,base_speed:220,mana_shield_capacity:0,target_reentry_protected:false,source_reentry_protected:false,
            target_is_player:false,decision_root:&root,occurrence:DecisionOccurrenceId::from_bytes([5;16])
        }).map_err(debug)?;
        runtime.compare_companion_state(&before,after).map_err(debug)?;
        let expected=if dead_guard&&!completed_guard {5}else{10};
        let early=runtime.stage_creature_periodic_turn(actor,999_999,TickFacts::default()).map_err(debug)?;
        let early=runtime.commit_creature_periodic_turn(early).map_err(debug)?;
        assert!(early.ticks.is_empty());
        assert_eq!(early.health_before,early.health_after);
        for second in 1..=3 {
            let mut source=|_:&ChannelRuntimeV1, actual:ExactActorRef,_:u64| {
                assert_eq!(actual,player);
                Ok(Some(100))
            };
            let receipt=runtime.apply_creature_periodic_turn_with_sources(actor,second*1_000_000,TickFacts::default(),&mut source).map_err(debug)?;
            assert_eq!(receipt.ticks.len(),1,"one actual due tick");
            assert_eq!(receipt.health_before-receipt.health_after,expected,"ORIGIN_CONDITION HEALTHCHANGE primary transforms once before canonical HP");
            assert_eq!(receipt.impacts.len(),1);
            assert_eq!(receipt.impacts[0].damage,u64::try_from(expected).map_err(debug)?);
            let replay=runtime.stage_creature_periodic_turn(actor,second*1_000_000,TickFacts::default()).map_err(debug)?;
            let replay=runtime.commit_creature_periodic_turn(replay).map_err(debug)?;
            assert!(replay.ticks.is_empty(),"same due boundary cannot replay damage");
            assert_eq!(replay.health_before,replay.health_after);
        }
        assert_eq!(runtime.live_creature_health(actor),Some(u64::try_from(1000-3*expected).map_err(debug)?));
        assert!(runtime.companion_snapshot(actor).map_err(debug)?.state.conditions.instances().is_empty());
        Ok(())
    })?.join().map_err(|_|"periodic HealthForgotten worker panicked")?.map_err(Into::into)
}
#[test]
fn srx3_health_forgotten_native_condition_tick_doubles_primary_once_without_guard() -> TestResult {
    periodic(false, false)
}
#[test]
fn srx3_health_forgotten_native_condition_tick_registered_hp0_guard_preserves_primary() -> TestResult
{
    periodic(true, false)
}
#[test]
fn srx3_health_forgotten_native_condition_tick_completed_guard_restores_double() -> TestResult {
    periodic(true, true)
}
