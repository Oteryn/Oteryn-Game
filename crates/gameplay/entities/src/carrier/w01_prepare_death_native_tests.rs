//! Actual native bodies + canonical HP owner. Unattached declarations are
//! installed explicitly only in review-engine cases; production cases stay inactive.
use super::runtime_actor_periodic::CreatureFieldEntry;
use super::*;
use crate::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_domain::domain::condition::{ConditionType, ConditionValues};
use oteryn_gameplay_foundation::foundation::CommandId;
use oteryn_gameplay_foundation::foundation::condition::{
    ApplicationFacts, ConditionDefinition, ConditionSourceKind,
};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, SemanticTimeMicros,
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn id(tag: u8) -> [u8; 16] {
    let mut bytes = [tag; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes
}
fn debug(value: impl std::fmt::Debug) -> String {
    format!("{value:?}")
}
// Share only immutable official content. Every case still creates a fresh
// carrier, session, position, policy installation, conditions and review tag.
fn retained_native(
    world: WorldId,
) -> &'static oteryn_gameplay_content::content::native_gameplay::NativeGameplayState {
    static SOURCE: std::sync::OnceLock<(
        WorldId,
        oteryn_gameplay_content::content::native_gameplay::NativeGameplayState,
    )> = std::sync::OnceLock::new();
    let source = SOURCE.get_or_init(|| {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../content/spells.manifest.json");
        (
            world,
            oteryn_gameplay_content::content::native_gameplay::retained_callback_native_fixture_at(
                world, &manifest,
            ),
        )
    });
    assert_eq!(
        source.0, world,
        "immutable fixture must stay bound to the same test World"
    );
    &source.1
}

fn run_case(slug: &'static str, heal: i64, periodic: bool) -> TestResult {
    run_attachment_case(slug, heal, periodic, true)
}
fn run_attachment_case(
    slug: &'static str,
    heal: i64,
    periodic: bool,
    review_declared_event: bool,
) -> TestResult {
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(),String> {
        let world=oteryn_gameplay_content::content::native_gameplay_test_world_id().map_err(debug)?;
        let native = retained_native(world);
        let mut runtime=ChannelRuntimeV1::from_committed_assignment(world,ChannelId::decode(&id(50)).map_err(debug)?,NodeId::decode(&id(51)).map_err(debug)?,1,1,1,"runtime-scope-assignment:1",8,
            ChannelContentPin::from_activation(world,1,native.source_digest(),[2;32],[3;32],[4;32],(100,100,7))).map_err(debug)?;
        native.install_companion_policies(&mut runtime).map_err(debug)?;
        let session=GameSessionId::decode(&id(52)).map_err(debug)?;
        let reservation=runtime.reserve_fresh_session(session).map_err(debug)?;
        let player=runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime.initialize_pinned_test_position(player,MovementLocalPosition{x:100,y:100,floor:7}).map_err(debug)?;
        runtime.bind_attacker_lease(player,session,CharacterLease::new(CharacterId::decode(&id(53)).map_err(debug)?,1).map_err(debug)?).map_err(debug)?;
        let key=format!("oteryn:creature.{slug}");
        // Source max and policy remain native; only initial nonlethal injury is
        // a bounded prepare-death test precondition, with no damage attribution.
        let maximum=runtime.companion_policy(&key).map_err(debug)?.maximum_health;
        let actor=runtime.admit_source_pinned_lab_creature(MovementLocalPosition{x:101,y:100,floor:7},&key,maximum).map_err(debug)?;
        runtime.install_creature_policy(actor,&key).map_err(debug)?;
        if !review_declared_event {
            let index = runtime.carrier.validate_ref(&runtime.continuity, actor.0).map_err(debug)?;
            let Slot::CreatureOccupied { spell_combat, .. } = &runtime.carrier.slots[index] else {
                return Err("actual native creature slot".into());
            };
            assert!(spell_combat.soul_war_health.is_none(),
                "declared prepare-death event absent from pinned monster.events must remain inactive: {slug}");
            return Ok(());
        }
        // Explicit review engine installation; production does not attach the
        // unregistered declaration merely by matching the native creature name.
        let snapshot = runtime.companion_snapshot(actor).map_err(debug)?;
        let callback = native.declared_w01_prepare_death_for_review(
            &snapshot.state.policy.definition_key, &snapshot.state.policy.definition_revision)
            .ok_or("exact declared review event body")?;
        let index = runtime.carrier.validate_ref(&runtime.continuity, actor.0).map_err(debug)?;
        let Slot::CreatureOccupied { spell_combat, .. } = &mut runtime.carrier.slots[index] else {
            return Err("actual review creature slot".into());
        };
        spell_combat.soul_war_health = Some(std::sync::Arc::new(callback));
        runtime.set_test_creature_health(actor,1).map_err(debug)?;
        assert!(runtime.borrow_combat_death().current_session_damage_shares(actor).map_err(debug)?.is_empty());
        let policy=runtime.companion_snapshot(actor).map_err(debug)?.state.policy;
        let root=GameplayDecisionRoot::from_bytes(native.source_digest());
        let invisible=ConditionDefinition::new("w01.prepare-death.existing-invisible",1,ConditionValues::Invisible{duration_ms:1000}).ok_or("invisible fixture")?;
        let facts=ApplicationFacts {now:10_000,base_speed:u16::try_from(policy.base_speed).map_err(debug)?,mana_shield_capacity:0,target_reentry_protected:false,source_reentry_protected:false,target_is_player:false,decision_root:&root,occurrence:DecisionOccurrenceId::from_bytes([0x91;16])};
        let now=SemanticTimeMicros::from_micros(10_000);
        let plan=runtime.prepare_actor_condition(actor,None,ActorConditionTransition::Apply {definition:&invisible,source:ConditionSource{actor,session:None,kind:ConditionSourceKind::SelfUse},immunities:&[],facts},now).map_err(debug)?;
        runtime.commit_actor_condition(&plan,now).map_err(debug)?;
        if periodic {
            // Real physical condition entry/tick owner; no caller-supplied HP
            // delta or manually projected death substitutes for the DoT path.
            let definition=ConditionDefinition::new("w01.prepare-death.native-periodic",1,
                ConditionValues::Agony {total_min:1000,total_max:1000,per_tick:1000,interval_ms:1000,delayed:false})
                .ok_or("native periodic condition")?;
            let field=CreatureFieldEntry {definition,source:None,lineage:None,
                occurrence:DecisionOccurrenceId::from_bytes([0x95;16])};
            runtime.commit_creature_field_entry(actor,10_000,Some(&field)).map_err(debug)?;
            let prepared=runtime.stage_creature_periodic_turn(actor,10_000,
                oteryn_gameplay_foundation::foundation::condition::TickFacts::default()).map_err(debug)?;
            let receipt=runtime.commit_creature_periodic_turn(prepared).map_err(debug)?;
            assert!(!receipt.ticks.is_empty(),"actual periodic owner must execute the lethal source tick");
            assert_eq!(runtime.live_creature_health(actor),Some((1+heal) as u64),
                "native prepare-death must cancel periodic lethal damage and heal originalHP");
            let store=runtime.actor_conditions(actor,None).map_err(debug)?;
            assert!(store.invisible_at(10_000),"canceled periodic damage preserves canonical invisibility");
            let marker=store.instances().iter().find(|condition|condition.definition().condition_type()==ConditionType::SpellRegeneration(88888))
                .ok_or("periodic hit must install SAME canonical88888 marker")?;
            assert_eq!(marker.remaining_duration_ms(10_000),Some(900_000));
            assert!(runtime.borrow_combat_death().current_session_damage_shares(actor).map_err(debug)?.is_empty());
            let replay=runtime.stage_creature_periodic_turn(actor,10_000,
                oteryn_gameplay_foundation::foundation::condition::TickFacts::default()).map_err(debug)?;
            runtime.commit_creature_periodic_turn(replay).map_err(debug)?;
            assert_eq!(runtime.live_creature_health(actor),Some((1+heal) as u64),"consumed periodic tick cannot replayhealing");
            return Ok(());
        }
        let command=CommandRef::new(session,CommandId::new(1).map_err(debug)?);
        let hit=OwnerDamageCommand{target:key.as_bytes(),occurrence:b"w01:first-fatal",binding:b"w01:first-fatal-native",damage:1000};
        let first=runtime.borrow_exact_actor_commit_at(10).commit_damage_for_bound_attacker(actor,player,command,0,hit).map_err(debug)?;
        assert!(first.applied);
        assert_eq!((first.health_before,first.health_after),(1+heal,1+heal),"source bare return cancels FULL hit and heals ORIGINAL pre-hit HP");
        assert_eq!(runtime.live_creature_health(actor),Some((1+heal) as u64));
        let store=runtime.actor_conditions(actor,None).map_err(debug)?;
        assert!(store.invisible_at(10_000),"canceled damage cannot drain canonical invisibility");
        let marker=store.instances().iter().find(|condition|condition.definition().condition_type()==ConditionType::SpellRegeneration(88888)).ok_or("actual canonical shared marker")?;
        assert_eq!(marker.remaining_duration_ms(10_000),Some(900_000));
        assert!(matches!(marker.definition().values(), ConditionValues::SpellRegeneration {
            sub_id:88888,health_gain:0,health_interval_ms:900_000,mana_gain:0,..
        }),"canonical marker retains explicit source dormant health interval");
        assert!(runtime.borrow_combat_death().current_session_damage_shares(actor).map_err(debug)?.is_empty(),"callback healing cannot fabricate attacker damage credit");
        let replay=runtime.borrow_exact_actor_commit_at(20).commit_damage_for_bound_attacker(actor,player,command,0,OwnerDamageCommand{target:key.as_bytes(),occurrence:b"w01:first-fatal",binding:b"w01:first-fatal-native",damage:1000}).map_err(debug)?;
        assert!(!replay.applied);
        assert_eq!(runtime.live_creature_health(actor),Some((1+heal) as u64));
        let marker=runtime.actor_conditions(actor,None).map_err(debug)?.instances().iter().find(|condition|condition.definition().condition_type()==ConditionType::SpellRegeneration(88888)).ok_or("retained marker")?;
        assert_eq!(marker.remaining_duration_ms(20_000),Some(899_990),"retry cannot refresh canonical marker lifetime");
        let command=CommandRef::new(session,CommandId::new(2).map_err(debug)?);
        let killed=runtime.borrow_exact_actor_commit_at(20).commit_damage_for_bound_attacker(actor,player,command,0,OwnerDamageCommand{target:key.as_bytes(),occurrence:b"w01:second-fatal",binding:b"w01:second-fatal-native",damage:1+heal}).map_err(debug)?;
        assert_eq!(killed.health_after,0,"source marker-present branch returns true and allows ordinary lethal damage");
        Ok(())
    })?.join().map_err(|_|"prepare-death owner test thread panicked")??;
    Ok(())
}
#[test]
fn srx3_hirintror_review_declared_prepare_death_uses_original_hp_shared_marker_and_full_cancel()
-> TestResult {
    run_case("hirintror", 1000, false)
}
#[test]
fn srx3_white_pale_review_declared_prepare_death_uses_original_hp_shared_marker_and_full_cancel()
-> TestResult {
    run_case("white_pale", 400, false)
}

#[test]
fn srx3_hirintror_review_declared_periodic_lethal_uses_same_marker_and_full_cancel() -> TestResult {
    run_case("hirintror", 1000, true)
}
#[test]
fn srx3_white_pale_review_declared_periodic_lethal_uses_same_marker_and_full_cancel() -> TestResult
{
    run_case("white_pale", 400, true)
}

#[test]
fn srx3_unattached_hirintror_prepare_death_event_is_inactive_in_production() -> TestResult {
    run_attachment_case("hirintror", 1000, false, false)
}
#[test]
fn srx3_unattached_white_pale_prepare_death_event_is_inactive_in_production() -> TestResult {
    run_attachment_case("white_pale", 400, false, false)
}
