//! Isolated preproduction composition of typed Ability and the physical owner slot.
use super::*;
use crate::ability::exact_actor_resolution::{
    ExactActorProposal, ExactActorResolutionError, resolve_exact_actor,
};
use crate::ability::{
    AbilityIntent, AbilityOccurrence, CommitGroup, Effect, EffectPlan, ProposalSource,
    RevisionSet,
};
use crate::ability::commit::{OwnerCommitError, commit_exact_owner_damage};

fn uuid_v7(seed: u64) -> [u8; 16] {
    let mut value = [0; 16];
    value[8..].copy_from_slice(&seed.to_be_bytes());
    value[6] = 0x70;
    value[8] = (value[8] & 0x3f) | 0x80;
    value
}

fn grant(seed: u64, generation: u64) -> PreProductionContinuityGrant {
    PreProductionContinuityGrant {
        world_id: WorldId::decode(&uuid_v7(seed)).expect("world"),
        channel_id: ChannelId::decode(&uuid_v7(seed + 1)).expect("channel"),
        scope_generation: ScopeOwnershipGeneration::new(generation).expect("scope"),
    }
}

fn fixture(seed: u64) -> (NamespaceContinuityGuard, ChannelActorCarrier, ExactActorRef) {
    let mut continuity = NamespaceContinuityGuard::from_pre_production_grant(grant(seed, 1));
    let mut carrier = ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 1)
        .expect("one explicit carrier slot");
    let actor = ExactActorRef(carrier.admit_creature(&continuity, ActorState(1), 20)
        .expect("fixture HP"));
    (continuity, carrier, actor)
}

fn occurrence(id: &str, revision: &str) -> AbilityOccurrence {
    AbilityOccurrence::new(
        id,
        RevisionSet::new(revision, "content:1", "world:1", "formula:1", "simulation:1")
            .expect("revisions"),
    ).expect("occurrence")
}

fn plan(occurrence: AbilityOccurrence, target: &str, damage: i64) -> EffectPlan {
    EffectPlan::immediate(
        occurrence,
        AbilityIntent::normalize(ProposalSource::Client, "actor:fixture", &[target]).expect("intent"),
        vec![Effect::damage(target, damage).expect("damage")],
        vec![],
        CommitGroup::atomic("scope:fixture", "group:one").expect("group"),
    ).expect("typed plan")
}

fn resolve(
    carrier: &ChannelActorCarrier,
    continuity: &NamespaceContinuityGuard,
    actor: ExactActorRef,
    occurrence: &AbilityOccurrence,
) -> crate::ability::exact_actor_resolution::ResolvedExactActor {
    resolve_exact_actor(
        &carrier.current_owner_exact_lookup(continuity), occurrence,
        ExactActorProposal::client(actor),
    ).expect("current actor")
}

fn health(carrier: &ChannelActorCarrier) -> i64 {
    match &carrier.slots[0] {
        Slot::CreatureOccupied { health, .. } => *health,
        _ => panic!("creature slot"),
    }
}

#[test]
fn typed_plan_commits_once_and_identical_replay_returns_original_transition() {
    let (continuity, mut carrier, actor) = fixture(10);
    let cast = occurrence("cast:1", "rules:1");
    let resolved = resolve(&carrier, &continuity, actor, &cast);
    let plan = plan(cast, "target:one", 7);
    let first = commit_exact_owner_damage(&mut carrier.current_owner_exact_commit(&continuity), &resolved, &plan)
        .expect("owner commit");
    assert_eq!((first.applied, first.health_before, first.health_after), (true, 20, 13));
    let after = carrier.slots.clone();
    let replay = commit_exact_owner_damage(&mut carrier.current_owner_exact_commit(&continuity), &resolved, &plan)
        .expect("same exact plan replay");
    assert_eq!((replay.applied, replay.health_before, replay.health_after), (false, 20, 13));
    assert_eq!(carrier.slots, after);
    assert_eq!(health(&carrier), 13);
}

#[test]
fn lethal_damage_disables_actions_but_administrative_remove_is_not_death() {
    let (continuity, mut carrier, actor) = fixture(20);
    let cast = occurrence("cast:lethal", "rules:1");
    let resolved = resolve(&carrier, &continuity, actor, &cast);
    let p = plan(cast.clone(), "target:one", 25);
    let committed = commit_exact_owner_damage(&mut carrier.current_owner_exact_commit(&continuity), &resolved, &p)
        .expect("lethal HP transition");
    assert_eq!(committed.health_after, 0);
    assert!(!carrier.current_owner_exact_lookup(&continuity).contains(actor));
    assert_eq!(resolve_exact_actor(&carrier.current_owner_exact_lookup(&continuity), &cast,
        ExactActorProposal::client(actor)), Err(ExactActorResolutionError::NotCurrentActor));
    let before = carrier.slots.clone();
    assert_eq!(carrier.commit_creature_damage_inner(&continuity, actor.0,
        b"cast:later", b"cast:later\0binding", 1, false), Err(CarrierError::OccurrenceConflict));
    assert_eq!(carrier.slots, before);
    assert_eq!(carrier.remove(&continuity, actor.0), Ok(ActorState(1)));
    assert!(matches!(carrier.slots[0], Slot::VacantReusable { .. }));
    let recycled = carrier.admit_creature(&continuity, ActorState(2), 30).expect("recycle");
    assert_eq!(health(&carrier), 30);
    assert_ne!(actor.0.actor_local_generation, recycled.actor_local_generation);
}

#[test]
fn revisions_plan_and_target_substitutions_leave_slot_byte_identical() {
    let (continuity, mut carrier, actor) = fixture(30);
    let cast = occurrence("cast:1", "rules:1");
    let resolved = resolve(&carrier, &continuity, actor, &cast);
    let original = plan(cast.clone(), "target:one", 5);
    commit_exact_owner_damage(&mut carrier.current_owner_exact_commit(&continuity), &resolved, &original)
        .expect("first commit");
    let after = carrier.slots.clone();
    for substituted in [
        plan(occurrence("cast:1", "rules:2"), "target:one", 5),
        plan(cast.clone(), "target:one", 6),
        plan(cast.clone(), "target:two", 5),
    ] {
        assert!(commit_exact_owner_damage(&mut carrier.current_owner_exact_commit(&continuity), &resolved, &substituted).is_err());
        assert_eq!(carrier.slots, after);
    }
}

#[test]
fn owner_and_actor_generation_are_revalidated_after_resolution() {
    let (mut continuity, mut carrier, actor) = fixture(40);
    let cast = occurrence("cast:1", "rules:1");
    let resolved = resolve(&carrier, &continuity, actor, &cast);
    let p = plan(cast, "target:one", 5);
    let mut wrong_actor = actor.0;
    wrong_actor.world_id = grant(60, 1).world_id;
    let before = carrier.slots.clone();
    assert_eq!(carrier.commit_creature_damage_inner(&continuity, wrong_actor, b"cast:1", b"cast:1\0binding", 5, false), Err(CarrierError::WrongScope));
    assert_eq!(carrier.slots, before);
    carrier.remove(&continuity, actor.0).expect("admin remove");
    let recycled = carrier.admit_creature(&continuity, ActorState(2), 20).expect("recycle");
    let after = carrier.slots.clone();
    assert_eq!(commit_exact_owner_damage(&mut carrier.current_owner_exact_commit(&continuity), &resolved, &p),
        Err(OwnerCommitError::Owner(CarrierError::StaleActorGeneration)));
    assert_eq!(carrier.slots, after);
    continuity.advance(grant(40, 2)).expect("new owner");
    assert_eq!(carrier.commit_creature_damage_inner(&continuity, recycled, b"cast:1", b"cast:1\0binding", 5, false), Err(CarrierError::WrongScope));
    assert_eq!(carrier.slots, after);
}

#[test]
fn invalid_magnitude_overflow_and_injected_failure_do_not_mutate_slot() {
    let (continuity, mut carrier, actor) = fixture(50);
    let before = carrier.slots.clone();
    for damage in [0, -1] {
        assert_eq!(carrier.commit_creature_damage_inner(&continuity, actor.0, b"cast:1", b"cast:1\0binding", damage, false), Err(CarrierError::InvalidDamage));
        assert_eq!(carrier.slots, before);
    }
    assert_eq!(carrier.commit_creature_damage_inner(&continuity, actor.0, b"cast:1", b"cast:1\0binding", 5, true), Err(CarrierError::InjectedCommitFailure));
    assert_eq!(carrier.slots, before);
    if let Slot::CreatureOccupied { health, .. } = &mut carrier.slots[0] { *health = i64::MIN; }
    let corrupt = carrier.slots.clone();
    assert_eq!(carrier.commit_creature_damage_inner(&continuity, actor.0, b"cast:1", b"cast:1\0binding", 1, false), Err(CarrierError::DamageOverflow));
    assert_eq!(carrier.slots, corrupt);
}
