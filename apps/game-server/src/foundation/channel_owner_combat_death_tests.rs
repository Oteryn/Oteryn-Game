//! Nonshipping fixed-one-creature lethal -> death -> corpse proof.

use super::super::exact_actor_test_ability::commit::commit_exact_owner_damage;
use super::super::exact_actor_test_ability::exact_actor_resolution::{
    ExactActorProposal, ResolvedExactActor, resolve_exact_actor,
};
use super::super::exact_actor_test_ability::{
    AbilityIntent, AbilityOccurrence, CommitGroup, Effect, EffectPlan, ProposalSource, RevisionSet,
};
use super::super::exact_actor_test_combat::project_fixed_one_creature_death;
use super::*;

const FIXTURE_POSITION: LocalPosition = LocalPosition {
    x: 120,
    y: 84,
    floor: 7,
};

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

fn position_context(owner: &NamespaceContinuityGuard) -> PreProductionPositionContext {
    PreProductionPositionContext {
        world_id: owner.world_id,
        channel_id: owner.channel_id,
        scope_generation: owner.current_generation,
        coordinate_frame_marker: 31,
        map_revision_marker: 32,
        content_generation_marker: 33,
    }
}

fn fixture(
    seed: u64,
    positioned: bool,
) -> (NamespaceContinuityGuard, ChannelActorCarrier, ExactActorRef) {
    let mut owner = NamespaceContinuityGuard::from_pre_production_grant(grant(seed, 1));
    let mut carrier = ChannelActorCarrier::bootstrap_pre_production(&mut owner, 2)
        .expect("two finite carrier slots");
    let actor = ExactActorRef(
        carrier
            .admit_creature(&owner, ActorState(1), "target:one", 20)
            .expect("fixed creature"),
    );
    if positioned {
        carrier
            .initialize_position(&owner, actor.0, position_context(&owner), FIXTURE_POSITION)
            .expect("owner position");
    }
    (owner, carrier, actor)
}

fn occurrence(id: &str, revision: &str) -> AbilityOccurrence {
    AbilityOccurrence::new(
        id,
        RevisionSet::new(
            revision,
            "content:combat:1",
            "world:combat:1",
            "formula:combat:1",
            "simulation:combat:1",
        )
        .expect("revisions"),
    )
    .expect("occurrence")
}

fn plan(occurrence: AbilityOccurrence, source: ProposalSource, damage: i64) -> EffectPlan {
    EffectPlan::immediate(
        occurrence,
        AbilityIntent::normalize(source, "actor:combat-fixture", &["target:one"]).expect("intent"),
        vec![Effect::damage("target:one", damage).expect("damage")],
        vec![],
        CommitGroup::atomic("scope:combat-fixture", "group:one").expect("group"),
    )
    .expect("typed plan")
}

fn resolve(
    carrier: &ChannelActorCarrier,
    owner: &NamespaceContinuityGuard,
    actor: ExactActorRef,
    occurrence: &AbilityOccurrence,
    source: ProposalSource,
) -> ResolvedExactActor {
    let proposal = match source {
        ProposalSource::Client => ExactActorProposal::client(actor),
        ProposalSource::Ai => ExactActorProposal::ai(actor),
        // Script is deliberately outside this profile; choosing the client
        // proposal makes any accidental use fail the later source comparison.
        ProposalSource::Script => ExactActorProposal::client(actor),
    };
    resolve_exact_actor(
        &carrier.current_owner_exact_lookup(owner),
        occurrence,
        proposal,
    )
    .expect("current actor")
}

fn commit_ability(
    carrier: &mut ChannelActorCarrier,
    owner: &NamespaceContinuityGuard,
    actor: ExactActorRef,
    occurrence: &AbilityOccurrence,
    source: ProposalSource,
    damage: i64,
) -> OwnerDamageResult {
    let resolved = resolve(carrier, owner, actor, occurrence, source);
    let plan = plan(occurrence.clone(), source, damage);
    commit_exact_owner_damage(
        &mut carrier.current_owner_exact_commit(owner),
        &resolved,
        &plan,
    )
    .expect("owner commit")
}

fn direct_command<'a>(
    occurrence: &'a [u8],
    binding: &'a [u8],
    damage: i64,
) -> OwnerDamageCommand<'a> {
    OwnerDamageCommand {
        target: b"target:one",
        occurrence,
        binding,
        damage,
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ProjectionSignature {
    actor: ExactActorRef,
    commit_binding: Vec<u8>,
    damage: i64,
    health_before: i64,
    position: MovementLocalPosition,
    position_revision: u64,
    context_markers: (u64, u64, u64),
}

fn projection_signature(projection: &RuntimeCorpseProjection) -> ProjectionSignature {
    ProjectionSignature {
        actor: projection.occurrence().actor(),
        commit_binding: projection.occurrence().commit_binding().to_vec(),
        damage: projection.occurrence().damage(),
        health_before: projection.occurrence().health_before(),
        position: projection.position(),
        position_revision: projection.position_revision(),
        context_markers: projection.context_markers(),
    }
}

#[test]
fn client_and_ai_lethal_commits_project_their_exact_owner_lineage() {
    let mut bindings = Vec::new();
    for (seed, source) in [(10, ProposalSource::Client), (20, ProposalSource::Ai)] {
        let (owner, mut carrier, actor) = fixture(seed, true);
        let cast = occurrence("cast:lethal", "rules:1");
        let result = commit_ability(&mut carrier, &owner, actor, &cast, source, 25);
        assert_eq!((result.health_before, result.health_after), (20, 0));
        let owner_binding = carrier
            .slots
            .iter()
            .find_map(|slot| match slot {
                Slot::CreatureOccupied {
                    committed: Some(committed),
                    ..
                } => Some(committed.binding.to_vec()),
                _ => None,
            })
            .expect("lethal owner commit must be retained");

        let projection = {
            let mut combat = carrier.current_owner_combat_death(&owner);
            projection_signature(
                project_fixed_one_creature_death(&mut combat, actor)
                    .expect("one owner corpse projection"),
            )
        };
        assert_eq!(projection.actor, actor);
        assert_eq!(projection.damage, 25);
        assert_eq!(projection.health_before, 20);
        assert_eq!(
            projection.position,
            MovementLocalPosition {
                x: 120,
                y: 84,
                floor: 7,
            }
        );
        assert_eq!(projection.position_revision, 1);
        assert_eq!(projection.context_markers, (31, 32, 33));
        assert_eq!(projection.commit_binding, owner_binding);
        bindings.push(projection.commit_binding);
    }
    assert_ne!(bindings[0], bindings[1]);
}

#[test]
fn lethal_replay_returns_the_same_projection_without_a_second_write() {
    let (owner, mut carrier, actor) = fixture(30, true);
    let cast = occurrence("cast:replay", "rules:1");
    let resolved = resolve(&carrier, &owner, actor, &cast, ProposalSource::Client);
    let lethal_plan = plan(cast, ProposalSource::Client, 20);
    let first_commit = commit_exact_owner_damage(
        &mut carrier.current_owner_exact_commit(&owner),
        &resolved,
        &lethal_plan,
    )
    .expect("first owner commit");
    assert!(first_commit.applied);
    let first = projection_signature(
        project_fixed_one_creature_death(&mut carrier.current_owner_combat_death(&owner), actor)
            .expect("first projection"),
    );

    let replay = commit_exact_owner_damage(
        &mut carrier.current_owner_exact_commit(&owner),
        &resolved,
        &lethal_plan,
    )
    .expect("exact Ability replay");
    assert!(!replay.applied);
    let second = projection_signature(
        project_fixed_one_creature_death(&mut carrier.current_owner_combat_death(&owner), actor)
            .expect("idempotent projection replay"),
    );
    assert_eq!(second, first);
    assert_eq!(
        projection_signature(
            carrier
                .corpse_projection
                .as_ref()
                .expect("one retained projection")
        ),
        first
    );
}

#[test]
fn different_lethal_occurrence_cannot_replace_the_retained_death() {
    let (owner, mut carrier, actor) = fixture(35, true);
    let cast = occurrence("cast:original", "rules:1");
    commit_ability(
        &mut carrier,
        &owner,
        actor,
        &cast,
        ProposalSource::Client,
        20,
    );
    let original = projection_signature(
        project_fixed_one_creature_death(&mut carrier.current_owner_combat_death(&owner), actor)
            .expect("original projection"),
    );
    let slots = carrier.slots.clone();
    assert_eq!(
        carrier.commit_creature_damage_inner(
            &owner,
            actor.0,
            direct_command(b"cast:different", b"cast:different\0different-binding", 20,),
            false,
        ),
        Err(CarrierError::OccurrenceConflict)
    );
    assert_eq!(carrier.slots, slots);
    assert_eq!(
        projection_signature(
            carrier
                .corpse_projection
                .as_ref()
                .expect("original projection retained")
        ),
        original
    );
}

#[test]
fn nonlethal_and_precommit_failure_leave_no_death_projection() {
    let (owner, mut carrier, actor) = fixture(40, true);
    let cast = occurrence("cast:nonlethal", "rules:1");
    let result = commit_ability(
        &mut carrier,
        &owner,
        actor,
        &cast,
        ProposalSource::Client,
        5,
    );
    assert_eq!(result.health_after, 15);
    assert_eq!(
        project_fixed_one_creature_death(&mut carrier.current_owner_combat_death(&owner), actor,),
        Err(CarrierError::CommittedLethalUnavailable)
    );
    assert!(carrier.corpse_projection.is_none());

    let (owner, mut carrier, actor) = fixture(41, true);
    let before = carrier.slots.clone();
    assert_eq!(
        carrier.commit_creature_damage_inner(
            &owner,
            actor.0,
            direct_command(b"cast:failed", b"cast:failed\0binding", 20),
            true,
        ),
        Err(CarrierError::InjectedCommitFailure)
    );
    assert_eq!(carrier.slots, before);
    assert!(carrier.corpse_projection.is_none());
}

#[test]
fn projection_failures_preserve_retry_and_lost_response_idempotency() {
    let (owner, mut carrier, actor) = fixture(50, true);
    let cast = occurrence("cast:fault", "rules:1");
    commit_ability(
        &mut carrier,
        &owner,
        actor,
        &cast,
        ProposalSource::Client,
        20,
    );

    let receipt = carrier
        .current_owner_combat_death(&owner)
        .committed_lethal_receipt(actor)
        .expect("lethal receipt");
    assert_eq!(
        carrier
            .current_owner_combat_death(&owner)
            .project_committed_lethal_with_failures(receipt, true, false),
        Err(CarrierError::InjectedCorpseProjectionFailure)
    );
    assert!(carrier.corpse_projection.is_none());

    let receipt = carrier
        .current_owner_combat_death(&owner)
        .committed_lethal_receipt(actor)
        .expect("retry receipt");
    assert_eq!(
        carrier
            .current_owner_combat_death(&owner)
            .project_committed_lethal_with_failures(receipt, false, true),
        Err(CarrierError::InjectedCorpseResponseFailure)
    );
    let retained = projection_signature(
        carrier
            .corpse_projection
            .as_ref()
            .expect("write survived lost response"),
    );
    let replay = {
        let mut combat = carrier.current_owner_combat_death(&owner);
        projection_signature(
            project_fixed_one_creature_death(&mut combat, actor).expect("lost response retry"),
        )
    };
    assert_eq!(replay, retained);
}

#[test]
fn missing_death_position_cannot_be_invented_after_lethal_commit() {
    let (owner, mut carrier, actor) = fixture(60, false);
    let cast = occurrence("cast:no-position", "rules:1");
    commit_ability(
        &mut carrier,
        &owner,
        actor,
        &cast,
        ProposalSource::Client,
        20,
    );
    assert_eq!(
        project_fixed_one_creature_death(&mut carrier.current_owner_combat_death(&owner), actor,),
        Err(CarrierError::PositionUnavailable)
    );
    assert_eq!(
        carrier.initialize_position(&owner, actor.0, position_context(&owner), FIXTURE_POSITION),
        Err(CarrierError::CreatureNotActionable)
    );
    assert_eq!(
        carrier.lookup(&owner, actor.0),
        Err(CarrierError::CreatureNotActionable)
    );
    assert!(carrier.corpse_projection.is_none());
}

#[test]
fn dead_position_mutation_and_changed_receipt_fail_without_projection() {
    let (owner, mut carrier, actor) = fixture(70, true);
    let original_position = carrier.read_position(&owner, actor.0).expect("position");
    let cast = occurrence("cast:immutable-position", "rules:1");
    commit_ability(
        &mut carrier,
        &owner,
        actor,
        &cast,
        ProposalSource::Client,
        20,
    );
    assert_eq!(
        carrier.compare_commit_position(
            &owner,
            original_position,
            original_position.version.context,
            LocalPosition {
                x: 121,
                ..FIXTURE_POSITION
            },
        ),
        Err(CarrierError::CreatureNotActionable)
    );

    let mut changed = carrier
        .current_owner_combat_death(&owner)
        .committed_lethal_receipt(actor)
        .expect("receipt");
    changed.projection.position.revision += 1;
    assert_eq!(
        carrier
            .current_owner_combat_death(&owner)
            .project_committed_lethal(changed),
        Err(CarrierError::CorpseReceiptMismatch)
    );
    assert!(carrier.corpse_projection.is_none());

    let mut changed = carrier
        .current_owner_combat_death(&owner)
        .committed_lethal_receipt(actor)
        .expect("receipt");
    changed.projection.occurrence.commit_binding = b"changed".to_vec().into_boxed_slice();
    assert_eq!(
        carrier
            .current_owner_combat_death(&owner)
            .project_committed_lethal(changed),
        Err(CarrierError::CorpseReceiptMismatch)
    );
    assert!(carrier.corpse_projection.is_none());
}

#[test]
fn administrative_removal_emits_no_death_and_stales_an_issued_receipt() {
    let (owner, mut carrier, actor) = fixture(80, true);
    let cast = occurrence("cast:removed", "rules:1");
    commit_ability(
        &mut carrier,
        &owner,
        actor,
        &cast,
        ProposalSource::Client,
        20,
    );
    let receipt = carrier
        .current_owner_combat_death(&owner)
        .committed_lethal_receipt(actor)
        .expect("receipt before administration");
    assert_eq!(carrier.remove(&owner, actor.0), Ok(ActorState(1)));
    assert!(carrier.corpse_projection.is_none());
    assert_eq!(
        carrier
            .current_owner_combat_death(&owner)
            .project_committed_lethal(receipt),
        Err(CarrierError::StaleActorGeneration)
    );
    let replacement = carrier
        .admit_creature(&owner, ActorState(2), "target:one", 20)
        .expect("new generation before any corpse exists");
    assert_ne!(
        replacement.actor_local_generation,
        actor.0.actor_local_generation
    );
}

#[test]
fn retained_corpse_blocks_replacement_and_stale_owner_cannot_replay() {
    let (mut owner, mut carrier, actor) = fixture(90, true);
    let cast = occurrence("cast:retained", "rules:1");
    commit_ability(
        &mut carrier,
        &owner,
        actor,
        &cast,
        ProposalSource::Client,
        20,
    );
    let projected_actor =
        project_fixed_one_creature_death(&mut carrier.current_owner_combat_death(&owner), actor)
            .expect("projection")
            .occurrence()
            .actor();
    assert_eq!(carrier.remove(&owner, actor.0), Ok(ActorState(1)));
    let retained =
        projection_signature(carrier.corpse_projection.as_ref().expect("retained corpse"));
    assert_eq!(
        carrier.admit_creature(&owner, ActorState(2), "target:two", 20),
        Err(CarrierError::CapacityExceeded)
    );
    assert_eq!(
        projection_signature(carrier.corpse_projection.as_ref().expect("retained corpse")),
        retained
    );

    owner.advance(grant(90, 2)).expect("new owner generation");
    assert_eq!(
        project_fixed_one_creature_death(
            &mut carrier.current_owner_combat_death(&owner),
            projected_actor,
        ),
        Err(CarrierError::WrongScope)
    );
    assert_eq!(
        projection_signature(carrier.corpse_projection.as_ref().expect("retained corpse")),
        retained
    );
}

#[test]
fn maximum_commit_binding_projects_and_max_plus_one_rejects_before_write() {
    let (owner, mut carrier, actor) = fixture(100, true);
    let mut maximum = b"cast:max\0".to_vec();
    maximum.resize(MAX_OWNER_COMMIT_BINDING_BYTES, b'x');
    assert!(
        carrier
            .commit_creature_damage_inner(
                &owner,
                actor.0,
                direct_command(b"cast:max", &maximum, 20),
                false,
            )
            .is_ok()
    );
    let projected_binding_len = {
        let mut combat = carrier.current_owner_combat_death(&owner);
        project_fixed_one_creature_death(&mut combat, actor)
            .expect("bounded maximum projects")
            .occurrence()
            .commit_binding()
            .len()
    };
    assert_eq!(projected_binding_len, MAX_OWNER_COMMIT_BINDING_BYTES);

    let (owner, mut carrier, actor) = fixture(101, true);
    maximum.push(b'x');
    let before = carrier.slots.clone();
    assert_eq!(
        carrier.commit_creature_damage_inner(
            &owner,
            actor.0,
            direct_command(b"cast:max", &maximum, 20),
            false,
        ),
        Err(CarrierError::CommitBindingTooLarge)
    );
    assert_eq!(carrier.slots, before);
    assert!(carrier.corpse_projection.is_none());
}
