//! Actual bounded native callback witness and physical HP owner. Map coordinates
//! are irrelevant DATA here: these tests grant no room or placement authority.
use super::*;
use crate::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_content::content::native_monster_callbacks::{
    NativeCallbackList, qualify_native_callback,
};
use oteryn_gameplay_content::content::{ProjectV2DefinitionRef, ProjectV2Family};
fn id(tag: u8) -> [u8; 16] {
    let mut out = [tag; 16];
    out[6] = 0x70;
    out[8] = 0x80;
    out
}
fn debug(value: impl std::fmt::Debug) -> String {
    format!("{value:?}")
}
fn run_case(zero: bool) -> Result<(), Box<dyn std::error::Error>> {
    std::thread::Builder::new().stack_size(64<<20).spawn(move||->Result<(),String>{
        let world=oteryn_gameplay_content::content::native_gameplay_test_world_id().map_err(debug)?;
        let manifest=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../content/spells.manifest.json");
        let native=oteryn_gameplay_content::content::native_gameplay::retained_callback_native_fixture_at(world,&manifest);
        let mut runtime=ChannelRuntimeV1::from_committed_assignment(world,ChannelId::decode(&id(60)).map_err(debug)?,NodeId::decode(&id(61)).map_err(debug)?,1,1,1,"runtime-scope-assignment:1",8,
            ChannelContentPin::from_activation(world,1,native.source_digest(),[2;32],[3;32],[4;32],(100,100,7))).map_err(debug)?;
        native.install_companion_policies(&mut runtime).map_err(debug)?;
        let parent=ProjectV2DefinitionRef{family:ProjectV2Family::Creature,key:if zero{"oteryn:creature.charging_outburst"}else{"oteryn:creature.the_time_guardian"}.into(),revision:"definition-r1".into()};
        let source=qualify_native_callback(&native,&parent,if zero{5}else{0},if zero{NativeCallbackList::Attack}else{NativeCallbackList::Defence}).map_err(debug)?.ok_or("actual bounded source callback")?;
        let target_key=if zero{"oteryn:creature.outburst"}else{"oteryn:creature.the_blazing_time_guardian"};
        let maximum=runtime.companion_policy(target_key).map_err(debug)?.maximum_health;
        let target=runtime.admit_source_pinned_lab_creature(MovementLocalPosition{x:101,y:100,floor:7},target_key,maximum).map_err(debug)?;
        runtime.install_creature_policy(target,target_key).map_err(debug)?;
        if zero {
            assert_eq!(runtime.native_null_health_entry_sample(&native,&source).map_err(debug)?.health(),290_000);
            let sample=runtime.native_null_health_default_sample(&native,&source).map_err(debug)?;
            let snapshot=runtime.companion_snapshot(target).map_err(debug)?;
            let result=runtime.commit_native_null_health_sync(&native,&source,&sample,&snapshot,b"source-outburst-default-zero",b"source-outburst-default-zero\0native-source-default").map_err(debug)?;
            assert_eq!(result.health_after,0);
            assert!(runtime.borrow_combat_death().committed_lethal_receipt(target).is_ok(),"source zero must pass the real canonical native lethal owner");
            assert!(runtime.borrow_combat_death().current_session_damage_shares(target).map_err(debug)?.is_empty(),"null-source synchronization invents no player contribution");
        }else {
            let key="oteryn:creature.the_time_guardian";
            let max=runtime.companion_policy(key).map_err(debug)?.maximum_health;
            let base=runtime.admit_source_pinned_lab_creature(MovementLocalPosition{x:100,y:100,floor:7},key,max).map_err(debug)?;
            runtime.install_creature_policy(base,key).map_err(debug)?;
            runtime.set_test_creature_health(base,10_000).map_err(debug)?;
            runtime.set_test_creature_health(target,50_000).map_err(debug)?;
            let sample=runtime.capture_native_null_health_sample(&native,&source,base).map_err(debug)?;
            let target_before=runtime.companion_snapshot(target).map_err(debug)?;
            let first=runtime.commit_native_null_health_sync(&native,&source,&sample,&target_before,b"source-time-drain",b"source-time-drain\0native").map_err(debug)?;
            assert_eq!((first.health_before,first.health_after),(50_000,10_000));
            assert!(runtime.borrow_combat_death().current_session_damage_shares(target).map_err(debug)?.is_empty());
            assert!(runtime.commit_native_null_health_sync(&native,&source,&sample,&target_before,b"source-time-stale",b"source-time-stale\0native").is_err(),"expected snapshot cannot be replayed after HP changed");
            runtime.set_test_creature_health(target,2_000).map_err(debug)?;
            let before=runtime.companion_snapshot(target).map_err(debug)?;
            let healed=runtime.commit_native_null_health_sync(&native,&source,&sample,&before,b"source-time-heal",b"source-time-heal\0native").map_err(debug)?;
            assert_eq!(healed.health_after,10_000);
            let decrease=qualify_native_callback(&native,&parent,1,NativeCallbackList::Defence).map_err(debug)?.ok_or("actual time guardiann")?;
            let sample=runtime.capture_native_null_health_sample(&native,&decrease,base).map_err(debug)?;
            runtime.set_test_creature_health(target,2_000).map_err(debug)?;
            let before=runtime.companion_snapshot(target).map_err(debug)?;
            let unchanged=runtime.commit_native_null_health_sync(&native,&decrease,&sample,&before,b"source-time-noheal",b"source-time-noheal\0native").map_err(debug)?;
            assert_eq!(unchanged.health_after,2_000,"alternate source must never heal upwards");
            assert!(runtime.native_null_health_default_sample(&native,&decrease).is_err(),"literal zero belongs only to the Outburst global fallback");
        }
        Ok(())
    })?.join().map_err(|_|"native null-health test thread panicked")??;
    Ok(())
}
#[test]
fn srx3_native_null_health_time_pair_signed_delta_and_stale_snapshot()
-> Result<(), Box<dyn std::error::Error>> {
    run_case(false)
}
#[test]
fn srx3_native_null_health_outburst_source_zero_is_real_uncredited_lethal()
-> Result<(), Box<dyn std::error::Error>> {
    run_case(true)
}
