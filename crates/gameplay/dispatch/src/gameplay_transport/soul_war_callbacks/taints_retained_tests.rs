use super::*;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
fn id(tag: u8) -> [u8; 16] {
    let mut value = [tag; 16];
    value[6] = 0x70;
    value[8] = 0x80;
    value
}
fn setup_error(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

#[test]
fn srx3_taint_timer_read_distinguishes_unavailable_stale_and_known_empty() -> Result<(), String> {
    let world = crate::foundation::WorldId::decode(&id(81)).map_err(setup_error)?;
    let runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        crate::foundation::ChannelId::decode(&id(82)).map_err(setup_error)?,
        crate::foundation::NodeId::decode(&id(83)).map_err(setup_error)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        1,
        crate::foundation::ChannelContentPin::test(world),
    )
    .map_err(setup_error)?;
    let mut facts = CurrentTaints {
        context: Some((binding(&runtime), Geometry::Boss(Room::GoshnarsMalice))),
        ..Default::default()
    };
    let now = 2_000_000;
    assert!(facts.geometry(&runtime).is_some());
    assert!(facts.current_player_read(&runtime, now).is_none());
    facts.observed_at = Some(now);
    assert!(
        facts
            .current_player_read(&runtime, now)
            .is_some_and(<[_]>::is_empty),
        "a successful empty census can complete the real periodic check"
    );
    assert!(
        facts.current_player_read(&runtime, now + 1).is_none(),
        "an older empty read cannot consume the next source occurrence"
    );
    facts.observed_at = None;
    assert!(
        facts.current_player_read(&runtime, now).is_none(),
        "a failed refresh must leave the timer pending even in a valid geometry"
    );
    Ok(())
}

#[test]
fn srx3_actual_native_retained_foam_component_keeps_current_policy_after_lethal()
-> Result<(), String> {
    std::thread::Builder::new().stack_size(64<<20).spawn(|| -> Result<(), String> {
        let world=crate::content::native_gameplay_test_world_id().map_err(setup_error)?;
        let manifest=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../content/spells.manifest.json");
        let native=crate::content::native_gameplay::retained_callback_native_fixture_at(world,&manifest);
        let key="oteryn:creature.foam_stalker";
        let record=native.creature_profiles().records.iter().find(|record|record.profile.target.key==key).ok_or("actual native Foam Stalker profile")?;
        let source=crate::content::native_monster_callbacks::qualify_native_callback(&native,&record.profile.target,4,crate::content::native_monster_callbacks::NativeCallbackList::Attack).map_err(setup_error)?.ok_or("actual native callback source")?;
        let stage=source.stage(0).map_err(setup_error)?.ok_or("actual first callback stage")?;
        let pin=crate::foundation::ChannelContentPin::from_activation(world,1,native.source_digest(),[2;32],[3;32],[4;32],(100,100,7));
        let mut runtime=ChannelRuntimeV1::from_committed_assignment(world,crate::foundation::ChannelId::decode(&id(91)).map_err(setup_error)?,crate::foundation::NodeId::decode(&id(92)).map_err(setup_error)?,1,1,1,"runtime-scope-assignment:1",8,pin).map_err(setup_error)?;
        native.install_companion_policies(&mut runtime).map_err(setup_error)?;
        let maximum=runtime.companion_policy(key).map_err(setup_error)?.maximum_health;
        let actor=runtime.admit_source_pinned_lab_creature(MovementLocalPosition{x:100,y:100,floor:7},key,maximum).map_err(setup_error)?;
        runtime.install_creature_policy(actor,key).map_err(setup_error)?;
        let session=GameSessionId::decode(&id(93)).map_err(setup_error)?;
        let reservation=runtime.reserve_fresh_session(session).map_err(setup_error)?;
        let player=runtime.commit_fresh_session(reservation).map_err(setup_error)?;
        runtime.initialize_pinned_test_position(player,MovementLocalPosition{x:101,y:100,floor:7}).map_err(setup_error)?;
        let stamp=runtime.issue_owner_work().map_err(setup_error)?;
        let retained=runtime.capture_native_retained_callback_caster(actor,&source,stamp).map_err(setup_error)?;
        let now=2_000_000;
        let policy=CurrentTaints::default().third_policy(&runtime,actor,player,now).ok_or("current third-taint component policy")?;
        assert_eq!(policy.apply(&runtime,actor,player,now,100),Some(100));
        runtime.borrow_exact_actor_commit_at_us(1_500_000).commit_damage(actor,oteryn_gameplay_entities::OwnerDamageCommand{target:key.as_bytes(),occurrence:b"retained-foam-component:lethal",binding:b"retained-foam-component:lethal\0source",damage:maximum}).map_err(setup_error)?;
        let mut receipt=runtime.borrow_combat_death().committed_lethal_receipt(actor).map_err(setup_error)?;
        let death=receipt.projection_mut().occurrence.death_key().map_err(setup_error)?;
        runtime.borrow_combat_death().project_committed_lethal(receipt).map_err(setup_error)?;
        assert!(runtime.source_creature_is_registered(actor).map_err(setup_error)?);
        assert!(retained.matches_stage(&runtime,actor,&stage));
        assert_eq!(policy.apply(&runtime,actor,player,now,100),None,"ordinary component authority stays live-only");
        assert_eq!(policy.apply_retained(&runtime,actor,player,now,100,&retained,&stage),Some(100),"actual retained native stage must preserve current outside-SoulWar component magnitude");
        assert_eq!(policy.apply_retained(&runtime,actor,player,now+1,100,&retained,&stage),None,"retained actor does not relax current-turn data");
        runtime.complete_source_creature_death(actor,death).map_err(setup_error)?;
        assert_eq!(policy.apply_retained(&runtime,actor,player,now,100,&retained,&stage),None,"completed actor is not retained source authority");
        Ok(())
    }).map_err(setup_error)?.join().map_err(|_| "retained native Foam test thread panicked".to_owned())?
}
