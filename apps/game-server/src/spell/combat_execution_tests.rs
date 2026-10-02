use super::*;
use crate::ability::{AbilityIntent, AbilityOccurrence, CommitGroup, RevisionSet};
use crate::foundation::{
    ChannelContentPin, ChannelId, CommandId, GameSessionId, MovementLocalPosition, NodeId, WorldId,
};
use crate::spell::ResolvedEffect;

fn uuid(tag: u8) -> [u8; 16] {
    [1, 0x90, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag]
}

fn position(x: i32) -> MovementLocalPosition {
    MovementLocalPosition { x, y: 10, floor: 7 }
}

struct Owner {
    runtime: ChannelRuntimeV1,
    caster: ExactActorRef,
    target: ExactActorRef,
    session: GameSessionId,
    character: CharacterId,
}

fn make_owner(tag: u8, generation: u64) -> Owner {
    let world = WorldId::decode(&uuid(tag)).expect("world");
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&uuid(tag + 1)).expect("channel"),
        NodeId::decode(&uuid(tag + 2)).expect("node"),
        1,
        generation,
        1,
        "runtime-scope-assignment:1",
        3,
        ChannelContentPin::test(world),
    )
    .expect("committed runtime");
    let session = GameSessionId::decode(&uuid(0x21)).expect("session");
    let reservation = runtime.reserve_fresh_session(session).expect("reserve");
    let caster = runtime
        .commit_fresh_session(reservation)
        .expect("commit actor");
    runtime
        .initialize_movement_test_position(caster, position(10))
        .expect("position");
    let target = runtime.admit_test_creature(position(11)).expect("creature");
    Owner {
        runtime,
        caster,
        target,
        session,
        character: CharacterId::decode(&uuid(0x22)).expect("character"),
    }
}

fn command(owner: &Owner, sequence: u64) -> CommandRef {
    CommandRef::new(owner.session, CommandId::new(sequence).expect("command"))
}

fn cast_plan(owner: &Owner, effects: Vec<Effect>) -> CastPlan {
    let occurrence = AbilityOccurrence::new(
        "spell-cast:physical-owner-test",
        RevisionSet::new("rules:1", "content:1", "world:1", "formula:1", "sim:1")
            .expect("revisions"),
    )
    .expect("occurrence");
    let targets: Vec<&str> = effects
        .iter()
        .map(|effect| effect.target().as_str())
        .collect();
    let intent = AbilityIntent::resolve(
        ProposalSource::Client,
        &actor_atom(owner.caster),
        &targets,
        &targets,
    )
    .expect("intent");
    CastPlan {
        effects: Some(
            EffectPlan::immediate(
                occurrence,
                intent,
                effects,
                vec![],
                CommitGroup::atomic("channel-owner", "spell:primary").expect("group"),
            )
            .expect("typed plan"),
        ),
        side_effects: vec![],
    }
}

fn damage(owner: &Owner, magnitude: i64) -> CastPlan {
    cast_plan(
        owner,
        vec![Effect::damage("test:creature", magnitude).expect("damage")],
    )
}

fn prepared(owner: &Owner, magnitude: i64) -> PreparedOwnerDamage {
    prepare(
        &owner.runtime,
        owner.caster,
        command(owner, 1),
        owner.target,
        &damage(owner, magnitude),
    )
    .expect("current owner preparation")
}

#[test]
fn actual_creature_health_changes_once_and_same_command_replays_original_receipt() {
    let mut owner = make_owner(0x40, 1);
    let prepared = prepared(&owner, 7);
    assert_eq!(prepared.target(), owner.target);
    assert_eq!(prepared.plan().effects()[0].magnitude(), 7);
    let first = commit(&mut owner.runtime, &prepared, owner.character, 1).expect("damage");
    assert_eq!(
        (first.applied, first.health_before, first.health_after),
        (true, 20, 13)
    );
    let replay = commit(&mut owner.runtime, &prepared, owner.character, 1).expect("replay");
    assert_eq!(
        (replay.applied, replay.health_before, replay.health_after),
        (false, 20, 13)
    );
    assert!(owner.runtime.contains_live_creature(owner.target));
}

#[test]
fn lethal_replay_uses_retained_resolution_and_does_not_require_live_target_again() {
    let mut owner = make_owner(0x40, 1);
    let prepared = prepared(&owner, 25);
    let first = commit(&mut owner.runtime, &prepared, owner.character, 1).expect("lethal");
    assert_eq!((first.health_before, first.health_after), (20, 0));
    assert!(!owner.runtime.contains_live_creature(owner.target));
    let replay = commit(&mut owner.runtime, &prepared, owner.character, 1).expect("lethal replay");
    assert_eq!(
        (replay.applied, replay.health_before, replay.health_after),
        (false, 20, 0)
    );
    assert!(matches!(
        prepare(
            &owner.runtime,
            owner.caster,
            command(&owner, 2),
            owner.target,
            &damage(&owner, 1)
        ),
        Err(Error::Resolution(
            ExactActorResolutionError::NotCurrentActor
        ))
    ));
}

#[test]
fn wrong_carrier_target_atom_and_changed_plan_cannot_mutate_health() {
    let mut owner = make_owner(0x40, 1);
    let wrong = cast_plan(
        &owner,
        vec![Effect::damage("foreign:creature", 7).expect("damage")],
    );
    let wrong = prepare(
        &owner.runtime,
        owner.caster,
        command(&owner, 1),
        owner.target,
        &wrong,
    )
    .expect("lookup does not manufacture target atom");
    assert_eq!(
        commit(&mut owner.runtime, &wrong, owner.character, 1),
        Err(Error::Commit(OwnerCommitError::Owner(
            CarrierError::CreatureTargetMismatch
        )))
    );
    let good = prepared(&owner, 7);
    let first = commit(&mut owner.runtime, &good, owner.character, 1).expect("good damage");
    assert_eq!((first.health_before, first.health_after), (20, 13));
    let changed = prepared(&owner, 8);
    assert_eq!(
        commit(&mut owner.runtime, &changed, owner.character, 1),
        Err(Error::Commit(OwnerCommitError::Owner(
            CarrierError::PlanConflict
        )))
    );
    let replay = commit(&mut owner.runtime, &good, owner.character, 1).expect("unchanged receipt");
    assert_eq!(replay.health_after, 13);
}

#[test]
fn heal_multitarget_and_side_effects_fail_before_actual_damage_mutation() {
    let mut owner = make_owner(0x40, 1);
    let mut plans = vec![
        cast_plan(
            &owner,
            vec![Effect::heal("test:creature", 7).expect("heal")],
        ),
        cast_plan(
            &owner,
            vec![
                Effect::damage("test:creature", 7).expect("damage"),
                Effect::damage("target:second", 8).expect("damage"),
            ],
        ),
        CastPlan {
            effects: None,
            side_effects: vec![],
        },
    ];
    let mut with_side_effect = damage(&owner, 7);
    with_side_effect
        .side_effects
        .push(ResolvedEffect::RemoveCondition {
            condition: "paralysis".into(),
        });
    plans.push(with_side_effect);
    for plan in plans {
        assert!(matches!(
            prepare(
                &owner.runtime,
                owner.caster,
                command(&owner, 1),
                owner.target,
                &plan
            ),
            Err(Error::UnsupportedEffects)
        ));
    }
    assert!(Effect::damage("test:creature", 0).is_err());
    assert!(Effect::damage("test:creature", -1).is_err());
    let good = prepared(&owner, 7);
    let first = commit(&mut owner.runtime, &good, owner.character, 1).expect("first mutation");
    assert_eq!(first.health_before, 20);
}

#[test]
fn recycled_actor_and_foreign_owner_refs_cannot_use_prepared_authority() {
    let mut owner = make_owner(0x40, 1);
    let prepared = prepared(&owner, 7);
    owner
        .runtime
        .remove_test_actor(owner.target)
        .expect("remove");
    let replacement = owner
        .runtime
        .admit_test_creature(position(11))
        .expect("reuse slot");
    assert_ne!(replacement, owner.target);
    assert_eq!(
        commit(&mut owner.runtime, &prepared, owner.character, 1),
        Err(Error::Commit(OwnerCommitError::Owner(
            CarrierError::StaleActorGeneration
        )))
    );
    let valid = prepare(
        &owner.runtime,
        owner.caster,
        command(&owner, 1),
        replacement,
        &damage(&owner, 7),
    )
    .expect("current replacement");
    assert_eq!(
        commit(&mut owner.runtime, &valid, owner.character, 1)
            .expect("current write")
            .health_before,
        20
    );
    let foreign = make_owner(0x50, 1);
    assert!(matches!(
        prepare(
            &owner.runtime,
            owner.caster,
            command(&owner, 2),
            foreign.target,
            &damage(&owner, 7)
        ),
        Err(Error::Owner(CarrierError::WrongScope))
    ));
}

#[test]
fn current_session_and_nonzero_lease_are_independent_of_prepared_plan() {
    let mut owner = make_owner(0x40, 1);
    let prepared = prepared(&owner, 7);
    assert_eq!(
        commit(&mut owner.runtime, &prepared, owner.character, 0),
        Err(Error::InvalidLeaseGeneration)
    );
    let wrong_session = GameSessionId::decode(&uuid(0x24)).expect("other session");
    assert!(matches!(
        prepare(
            &owner.runtime,
            owner.caster,
            CommandRef::new(wrong_session, CommandId::new(1).expect("id")),
            owner.target,
            &damage(&owner, 7)
        ),
        Err(Error::Owner(_))
    ));
    let first = commit(&mut owner.runtime, &prepared, owner.character, 1).expect("valid lease");
    assert_eq!((first.health_before, first.health_after), (20, 13));
    let mut newer = make_owner(0x40, 2);
    assert!(matches!(
        commit(&mut newer.runtime, &prepared, newer.character, 1),
        Err(Error::Owner(CarrierError::WrongScope))
    ));
}

#[test]
fn a_prepared_cast_cannot_mutate_after_current_control_is_lost() {
    let mut owner = make_owner(0x40, 1);
    let prepared = prepared(&owner, 7);
    owner
        .runtime
        .record_control_loss(
            owner.caster,
            owner.session,
            crate::foundation::ControlLossMark {
                epoch: 1,
                grace_deadline: 500,
            },
        )
        .expect("owner mirrors independently committed loss");
    assert_eq!(
        commit(&mut owner.runtime, &prepared, owner.character, 1),
        Err(Error::CasterUncontrolled)
    );
    assert!(matches!(
        prepare(
            &owner.runtime,
            owner.caster,
            command(&owner, 1),
            owner.target,
            &damage(&owner, 7)
        ),
        Err(Error::CasterUncontrolled)
    ));
    owner
        .runtime
        .restore_control(owner.caster, owner.session, 1)
        .expect("exact recovery");
    let first = commit(&mut owner.runtime, &prepared, owner.character, 1)
        .expect("first mutation after restoration");
    assert_eq!((first.health_before, first.health_after), (20, 13));
}
