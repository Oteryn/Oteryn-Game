use super::*;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
fn id(tag: u8) -> [u8; 16] {
    let mut value = [tag; 16];
    value[6] = 0x70;
    value[8] = 0x80;
    value
}
#[test]
fn srx3_actual_native_retained_foam_component_keeps_current_policy_after_lethal() {
    std::thread::Builder::new().stack_size(64<<20).spawn(|| {
        let world=crate::content::native_gameplay_test_world_id().unwrap();
        let manifest=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../content/spells.manifest.json");
        let native=crate::content::native_gameplay::retained_callback_native_fixture_at(world,&manifest);
        let key="oteryn:creature.foam_stalker";
        let record=native.creature_profiles().records.iter().find(|record|record.profile.target.key==key).unwrap();
        let source=crate::content::native_monster_callbacks::qualify_native_callback(&native,&record.profile.target,4,crate::content::native_monster_callbacks::NativeCallbackList::Attack).unwrap().unwrap();
        let stage=source.stage(0).unwrap().unwrap();
        let pin=crate::foundation::ChannelContentPin::from_activation(world,1,native.source_digest(),[2;32],[3;32],[4;32],(100,100,7));
        let mut runtime=ChannelRuntimeV1::from_committed_assignment(world,crate::foundation::ChannelId::decode(&id(91)).unwrap(),crate::foundation::NodeId::decode(&id(92)).unwrap(),1,1,1,"runtime-scope-assignment:1",8,pin).unwrap();
        native.install_companion_policies(&mut runtime).unwrap();
        let maximum=runtime.companion_policy(key).unwrap().maximum_health;
        let actor=runtime.admit_source_pinned_lab_creature(MovementLocalPosition{x:100,y:100,floor:7},key,maximum).unwrap();
        runtime.install_creature_policy(actor,key).unwrap();
        let session=GameSessionId::decode(&id(93)).unwrap();
        let reservation=runtime.reserve_fresh_session(session).unwrap();
        let player=runtime.commit_fresh_session(reservation).unwrap();
        runtime.initialize_pinned_test_position(player,MovementLocalPosition{x:101,y:100,floor:7}).unwrap();
        let stamp=runtime.issue_owner_work().unwrap();
        let retained=runtime.capture_native_retained_callback_caster(actor,&source,stamp).unwrap();
        let now=2_000_000;
        let policy=CurrentTaints::default().third_policy(&runtime,actor,player,now).unwrap();
        assert_eq!(policy.apply(&runtime,actor,player,now,100),Some(100));
        runtime.borrow_exact_actor_commit_at_us(1_500_000).commit_damage(actor,oteryn_gameplay_entities::OwnerDamageCommand{target:key.as_bytes(),occurrence:b"retained-foam-component:lethal",binding:b"retained-foam-component:lethal\0source",damage:maximum}).unwrap();
        let mut receipt=runtime.borrow_combat_death().committed_lethal_receipt(actor).unwrap();
        let death=receipt.projection_mut().occurrence.death_key().unwrap();
        runtime.borrow_combat_death().project_committed_lethal(receipt).unwrap();
        assert!(runtime.source_creature_is_registered(actor).unwrap());
        assert!(retained.matches_stage(&runtime,actor,&stage));
        assert_eq!(policy.apply(&runtime,actor,player,now,100),None,"ordinary component authority stays live-only");
        assert_eq!(policy.apply_retained(&runtime,actor,player,now,100,&retained,&stage),Some(100),"actual retained native stage must preserve current outside-SoulWar component magnitude");
        assert_eq!(policy.apply_retained(&runtime,actor,player,now+1,100,&retained,&stage),None,"retained actor does not relax current-turn data");
        runtime.complete_source_creature_death(actor,death).unwrap();
        assert_eq!(policy.apply_retained(&runtime,actor,player,now,100,&retained,&stage),None,"completed actor is not retained source authority");
    }).unwrap().join().unwrap();
}
