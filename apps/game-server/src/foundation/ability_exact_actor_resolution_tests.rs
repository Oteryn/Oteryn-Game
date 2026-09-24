//! Preproduction composition only: the carrier's private admission fixtures
//! supply refs; Ability receives just the read-only current-owner view.
use super::super::exact_actor_test_ability::exact_actor_resolution::{
    ExactActorProposal, ExactActorResolutionError, resolve_exact_actor,
};
use super::super::exact_actor_test_ability::{AbilityOccurrence, RevisionSet};
use super::*;

fn uuid_v7(raw: u64) -> [u8; 16] {
    let mut value = [0; 16];
    value[8..].copy_from_slice(&raw.to_be_bytes());
    value[6] = 0x70;
    value[8] = (value[8] & 0x3f) | 0x80;
    value
}

fn grant(seed: u64, generation: u64) -> PreProductionContinuityGrant {
    PreProductionContinuityGrant {
        world_id: WorldId::decode(&uuid_v7(seed)).expect("world"),
        channel_id: ChannelId::decode(&uuid_v7(seed + 1)).expect("channel"),
        scope_generation: ScopeOwnershipGeneration::new(generation).expect("generation"),
    }
}

fn fixture(seed: u64, capacity: usize) -> (NamespaceContinuityGuard, ChannelActorCarrier) {
    let mut continuity = NamespaceContinuityGuard::from_pre_production_grant(grant(seed, 1));
    let carrier = ChannelActorCarrier::bootstrap_pre_production(&mut continuity, capacity)
        .expect("test-only bootstrap");
    (continuity, carrier)
}

fn occurrence(id: &str, revision: &str) -> AbilityOccurrence {
    AbilityOccurrence::new(
        id,
        RevisionSet::new(revision, "content", "world", "formula", "simulation").expect("revisions"),
    )
    .expect("occurrence")
}

#[test]
fn client_and_ai_take_the_same_one_lookup_path() {
    let (continuity, mut carrier) = fixture(10, 2);
    let actor = ExactActorRef(carrier.admit(&continuity, ActorState(3)).expect("admit"));
    let occurrence = occurrence("cast-1", "rules-v1");
    let view = carrier.current_owner_exact_lookup(&continuity);
    let client = resolve_exact_actor(&view, &occurrence, ExactActorProposal::client(actor))
        .expect("client resolves");
    let ai = resolve_exact_actor(&view, &occurrence, ExactActorProposal::ai(actor))
        .expect("AI resolves");
    assert_eq!(client.target(), ai.target());
    assert_eq!(client.occurrence(), &occurrence);
    assert_eq!(client.candidate_count(), 1);
    assert_eq!(client.resolved_count(), 1);
    assert_ne!(client.source(), ai.source());
}

#[test]
fn missing_vacant_recycled_and_cross_scope_refs_fail_closed() {
    let (continuity, mut carrier) = fixture(20, 1);
    let current = carrier.admit(&continuity, ActorState(3)).expect("admit");
    let mut missing = current;
    missing.actor_local_id = ActorLocalId(2);
    let cast = occurrence("cast-1", "rules-v1");
    assert_eq!(
        resolve_exact_actor(
            &carrier.current_owner_exact_lookup(&continuity),
            &cast,
            ExactActorProposal::client(ExactActorRef(missing))
        ),
        Err(ExactActorResolutionError::NotCurrentActor)
    );
    carrier.remove(&continuity, current).expect("remove");
    assert_eq!(
        resolve_exact_actor(
            &carrier.current_owner_exact_lookup(&continuity),
            &cast,
            ExactActorProposal::client(ExactActorRef(current))
        ),
        Err(ExactActorResolutionError::NotCurrentActor)
    );
    let recycled = carrier.admit(&continuity, ActorState(4)).expect("reuse");
    assert_eq!(current.actor_local_id, recycled.actor_local_id);
    assert_ne!(
        current.actor_local_generation,
        recycled.actor_local_generation
    );
    let view = carrier.current_owner_exact_lookup(&continuity);
    assert_eq!(
        resolve_exact_actor(&view, &cast, ExactActorProposal::ai(ExactActorRef(current))),
        Err(ExactActorResolutionError::NotCurrentActor)
    );
    assert!(
        resolve_exact_actor(
            &view,
            &cast,
            ExactActorProposal::ai(ExactActorRef(recycled))
        )
        .is_ok()
    );

    let (other_continuity, mut other) = fixture(30, 1);
    let other_ref = ExactActorRef(
        other
            .admit(&other_continuity, ActorState(5))
            .expect("other"),
    );
    assert_eq!(
        resolve_exact_actor(&view, &cast, ExactActorProposal::client(other_ref)),
        Err(ExactActorResolutionError::NotCurrentActor)
    );
    for changed in 0..3 {
        let mut wrong = recycled;
        match changed {
            0 => wrong.world_id = grant(40, 1).world_id,
            1 => wrong.channel_id = grant(40, 1).channel_id,
            _ => wrong.scope_generation = ScopeOwnershipGeneration::new(2).expect("generation"),
        }
        assert_eq!(
            resolve_exact_actor(
                &view,
                &cast,
                ExactActorProposal::client(ExactActorRef(wrong))
            ),
            Err(ExactActorResolutionError::NotCurrentActor)
        );
    }
}

#[test]
fn live_owner_transition_fences_resolver_and_retry_rejects_substitution() {
    let (mut continuity, mut carrier) = fixture(50, 2);
    let original = ExactActorRef(carrier.admit(&continuity, ActorState(1)).expect("first"));
    let alternate = ExactActorRef(carrier.admit(&continuity, ActorState(2)).expect("second"));
    let cast = occurrence("cast-1", "rules-v1");
    let resolved = resolve_exact_actor(
        &carrier.current_owner_exact_lookup(&continuity),
        &cast,
        ExactActorProposal::client(original),
    )
    .expect("resolve");
    assert_eq!(
        resolved.reconcile(&cast, ExactActorProposal::client(original)),
        Ok(())
    );
    assert_eq!(
        resolved.reconcile(&cast, ExactActorProposal::client(alternate)),
        Err(ExactActorResolutionError::TargetSubstitution)
    );
    assert_eq!(
        resolved.reconcile(
            &occurrence("cast-1", "rules-v2"),
            ExactActorProposal::client(original)
        ),
        Err(ExactActorResolutionError::RevisionSubstitution)
    );
    assert_eq!(
        resolved.reconcile(
            &occurrence("cast-2", "rules-v1"),
            ExactActorProposal::client(original)
        ),
        Err(ExactActorResolutionError::OccurrenceSubstitution)
    );
    assert_eq!(
        resolved.reconcile(&cast, ExactActorProposal::ai(original)),
        Err(ExactActorResolutionError::SourceSubstitution)
    );
    continuity
        .advance(grant(50, 2))
        .expect("new owner generation");
    assert_eq!(
        resolve_exact_actor(
            &carrier.current_owner_exact_lookup(&continuity),
            &cast,
            ExactActorProposal::client(original)
        ),
        Err(ExactActorResolutionError::NotCurrentActor)
    );
}
