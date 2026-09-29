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
                .corpse_projections
                .first()
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
                .corpse_projections
                .first()
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
    assert!(carrier.corpse_projections.is_empty());

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
    assert!(carrier.corpse_projections.is_empty());
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
    assert!(carrier.corpse_projections.is_empty());

    let receipt = carrier
        .current_owner_combat_death(&owner)
        .committed_lethal_receipt(actor)
        .expect("retry receipt");
    let direct_retry = carrier
        .current_owner_combat_death(&owner)
        .committed_lethal_receipt(actor)
        .expect("direct retry receipt");
    let mut conflicting_retry = carrier
        .current_owner_combat_death(&owner)
        .committed_lethal_receipt(actor)
        .expect("conflicting retry receipt");
    conflicting_retry.projection.occurrence.commit_binding =
        b"cast:conflicting\0binding".to_vec().into_boxed_slice();
    assert_eq!(
        carrier
            .current_owner_combat_death(&owner)
            .project_committed_lethal_with_failures(receipt, false, true),
        Err(CarrierError::InjectedCorpseResponseFailure)
    );
    let retained = projection_signature(
        carrier
            .corpse_projections
            .first()
            .expect("write survived lost response"),
    );
    assert_eq!(carrier.remove(&owner, actor.0), Ok(ActorState(1)));
    let direct_replay = {
        let mut combat = carrier.current_owner_combat_death(&owner);
        projection_signature(
            combat
                .project_committed_lethal(direct_retry)
                .expect("issued receipt reconciles after source removal"),
        )
    };
    assert_eq!(direct_replay, retained);
    assert_eq!(
        carrier
            .current_owner_combat_death(&owner)
            .project_committed_lethal(conflicting_retry),
        Err(CarrierError::CorpseProjectionConflict)
    );
    let replay = {
        let mut combat = carrier.current_owner_combat_death(&owner);
        projection_signature(
            project_fixed_one_creature_death(&mut combat, actor)
                .expect("lost response retry reconciles after source removal"),
        )
    };
    assert_eq!(replay, retained);
    let wrong_generation = ExactActorRef(ActorRef {
        actor_local_generation: ActorLocalGeneration(actor.0.actor_local_generation.0 + 1),
        ..actor.0
    });
    assert!(
        project_fixed_one_creature_death(
            &mut carrier.current_owner_combat_death(&owner),
            wrong_generation,
        )
        .is_err()
    );
    assert_eq!(
        projection_signature(carrier.corpse_projections.first().expect("retained corpse")),
        retained
    );
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
    assert!(carrier.corpse_projections.is_empty());
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
    assert!(carrier.corpse_projections.is_empty());

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
    assert!(carrier.corpse_projections.is_empty());
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
    assert!(carrier.corpse_projections.is_empty());
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
fn retained_corpse_survives_a_new_admission_and_stale_owner_cannot_replay() {
    // AI-2 (GAME-AI-01 §4.1 envelope): a new, unrelated creature admission is no longer
    // blocked by an earlier actor's retained corpse projection (that per-actor record is
    // independent of carrier-wide admission capacity now); the retained record itself is
    // untouched by the new admission.
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
        projection_signature(carrier.corpse_projections.first().expect("retained corpse"));
    assert!(
        carrier
            .admit_creature(&owner, ActorState(2), "target:two", 20)
            .is_ok(),
        "an unrelated new creature is no longer blocked by another actor's retained corpse"
    );
    assert_eq!(
        projection_signature(carrier.corpse_projections.first().expect("retained corpse")),
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
        projection_signature(carrier.corpse_projections.first().expect("retained corpse")),
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
    assert!(carrier.corpse_projections.is_empty());
}

fn key_fields(
    key: CreatureDeathOccurrenceKey,
) -> (WorldId, ChannelId, ScopeOwnershipGeneration, u32, u64) {
    (
        key.world_id(),
        key.channel_id(),
        key.scope_ownership_generation(),
        key.actor_local_id(),
        key.actor_local_generation(),
    )
}

fn d1_fixture(seed: u64) -> CombatDeathFixture {
    let scope = grant(seed, 1);
    CombatDeathFixture::new(scope.world_id, scope.channel_id, scope.scope_generation)
        .expect("fixture creature")
}

#[test]
fn committed_lethal_death_key_is_the_exact_owner_actor_ref_and_replay_stable() {
    let (owner, mut carrier, actor) = fixture(110, true);
    let cast = occurrence("cast:death-key", "rules:1");
    let resolved = resolve(&carrier, &owner, actor, &cast, ProposalSource::Client);
    let lethal_plan = plan(cast, ProposalSource::Client, 20);
    let lethal = commit_exact_owner_damage(
        &mut carrier.current_owner_exact_commit(&owner),
        &resolved,
        &lethal_plan,
    )
    .expect("lethal owner commit");
    assert!(lethal.applied);
    let first =
        project_fixed_one_creature_death(&mut carrier.current_owner_combat_death(&owner), actor)
            .expect("projection")
            .occurrence()
            .death_key();
    assert_eq!(
        key_fields(first),
        (
            actor.0.world_id,
            actor.0.channel_id,
            actor.0.scope_generation,
            actor.0.actor_local_id.0,
            actor.0.actor_local_generation.0,
        )
    );

    let replay = commit_exact_owner_damage(
        &mut carrier.current_owner_exact_commit(&owner),
        &resolved,
        &lethal_plan,
    )
    .expect("exact Ability replay");
    assert!(!replay.applied);
    let replayed =
        project_fixed_one_creature_death(&mut carrier.current_owner_combat_death(&owner), actor)
            .expect("idempotent projection")
            .occurrence()
            .death_key();
    assert_eq!(replayed, first);

    // The runtime MINT cause is the full typed tuple of that key (§4.2).
    use crate::durability::item_mint::{ItemMintCause, TypedDefinitionRef};
    let loot = TypedDefinitionRef {
        family: "LootTable".into(),
        production_key: "fixture:vsl-combat.loot.single".into(),
        revision_ref: "VSL_COMBAT_FIXTURE_PROFILE/v1".into(),
    };
    let (world_id, channel_id, generation, local_id, local_generation) = key_fields(first);
    assert_eq!(
        ItemMintCause::from_creature_death(first, loot.clone(), "fixture:drop".into(), 0),
        ItemMintCause::for_test(
            world_id,
            channel_id,
            generation,
            local_id,
            local_generation,
            loot,
            "fixture:drop".into(),
            0,
        )
    );
}

#[test]
fn only_a_committed_projected_lethal_occurrence_yields_a_death_key() {
    // Non-lethal damage: no death.
    let mut grazed = d1_fixture(120);
    let graze = grazed.strike("strike:graze", 5).expect("nonlethal commit");
    assert_eq!((graze.health_before, graze.health_after), (20, 15));
    assert_eq!(
        grazed.project_death(),
        Err(CarrierError::CommittedLethalUnavailable)
    );

    // Administrative despawn, alive or after the lethal commit: no death.
    let mut living = d1_fixture(122);
    living.despawn().expect("despawn");
    assert!(living.project_death().is_err());
    let mut struck = d1_fixture(124);
    struck
        .strike("strike:lethal", CombatDeathFixture::HEALTH)
        .expect("lethal commit");
    struck.despawn().expect("despawn");
    assert!(struck.project_death().is_err());

    // A stale owner after the scope moved cannot derive the death.
    let mut moved = d1_fixture(126);
    moved
        .strike("strike:lethal", CombatDeathFixture::HEALTH)
        .expect("lethal commit");
    moved
        .advance_owner(ScopeOwnershipGeneration::new(2).expect("generation"))
        .expect("scope moved");
    assert_eq!(moved.project_death(), Err(CarrierError::WrongScope));
}

#[test]
fn two_fixture_creature_deaths_have_distinct_death_keys() {
    let mut first = d1_fixture(130);
    let mut second = d1_fixture(132);
    let mut keys = Vec::new();
    for creature in [&mut first, &mut second] {
        let lethal = creature
            .strike("strike:lethal", CombatDeathFixture::HEALTH)
            .expect("lethal commit");
        assert!(lethal.applied);
        let (key, _) = creature.project_death().expect("death");
        assert_eq!(
            key_fields(key).4,
            creature.actor().0.actor_local_generation.0
        );
        keys.push(key);
    }
    assert_ne!(keys[0], keys[1]);
    assert_ne!(keys[0].channel_id(), keys[1].channel_id());
}

// ---------------------------------------------------------------------------
// D2b: `reward_occurrence`, the memoized per-(death, character) XP
// occurrence the physical Channel owner keeps in its death record
// (DUR-03 decision §4.2).
// ---------------------------------------------------------------------------

fn is_valid_reward_occurrence(bytes: [u8; 16]) -> bool {
    bytes[6] >> 4 == 7 && bytes[8] & 0xc0 == 0x80
}

#[test]
fn reward_occurrence_requires_a_committed_death_first() {
    let (owner, mut carrier, actor) = fixture(140, true);
    assert_eq!(
        carrier
            .current_owner_combat_death(&owner)
            .reward_occurrence(actor, [9; 16]),
        Err(CarrierError::CommittedLethalUnavailable)
    );
}

#[test]
fn reward_occurrence_is_minted_once_and_replay_reuses_the_same_value() {
    let mut fixture = d1_fixture(142);
    fixture
        .strike("strike:reward-once", CombatDeathFixture::HEALTH)
        .expect("lethal commit");
    let (_, _) = fixture.project_death().expect("death");
    let actor = fixture.actor();
    let character = [11_u8; 16];

    let (first, first_is_fresh) = fixture
        .borrow_combat_death()
        .reward_occurrence(actor, character)
        .expect("first mint");
    assert!(is_valid_reward_occurrence(first));
    assert!(first_is_fresh);
    let (replay, replay_is_fresh) = fixture
        .borrow_combat_death()
        .reward_occurrence(actor, character)
        .expect("idempotent replay");
    assert_eq!(replay, first);
    assert!(!replay_is_fresh);
}

#[test]
fn reward_occurrence_conflicts_on_a_different_reward_principal() {
    let mut fixture = d1_fixture(144);
    fixture
        .strike("strike:reward-conflict", CombatDeathFixture::HEALTH)
        .expect("lethal commit");
    let (_, _) = fixture.project_death().expect("death");
    let actor = fixture.actor();

    let (first, _) = fixture
        .borrow_combat_death()
        .reward_occurrence(actor, [21_u8; 16])
        .expect("first mint");
    assert!(is_valid_reward_occurrence(first));
    assert_eq!(
        fixture
            .borrow_combat_death()
            .reward_occurrence(actor, [22_u8; 16]),
        Err(CarrierError::RewardPrincipalConflict)
    );
}

#[test]
fn two_fixture_creature_deaths_mint_distinct_reward_occurrences() {
    let mut first = d1_fixture(146);
    let mut second = d1_fixture(148);
    let mut minted = Vec::new();
    for creature in [&mut first, &mut second] {
        creature
            .strike("strike:reward-distinct", CombatDeathFixture::HEALTH)
            .expect("lethal commit");
        creature.project_death().expect("death");
        let actor = creature.actor();
        minted.push(
            creature
                .borrow_combat_death()
                .reward_occurrence(actor, [33_u8; 16])
                .expect("mint")
                .0,
        );
    }
    assert_ne!(minted[0], minted[1]);
}

#[test]
fn stale_generation_death_cannot_mint_a_reward_occurrence() {
    let mut fixture = d1_fixture(150);
    fixture
        .strike("strike:reward-stale", CombatDeathFixture::HEALTH)
        .expect("lethal commit");
    fixture.project_death().expect("death");
    let actor = fixture.actor();
    fixture
        .advance_owner(ScopeOwnershipGeneration::new(2).expect("generation"))
        .expect("scope moved");
    assert_eq!(
        fixture
            .borrow_combat_death()
            .reward_occurrence(actor, [44_u8; 16]),
        Err(CarrierError::WrongScope)
    );
}

#[test]
fn projected_death_is_read_only_from_the_owner_projection() {
    let (owner, mut carrier, actor) = fixture(146, true);
    assert_eq!(
        carrier
            .current_owner_combat_death(&owner)
            .projected_death(actor)
            .map(|_| ()),
        Err(CarrierError::CommittedLethalUnavailable)
    );

    let mut fixture = d1_fixture(148);
    fixture
        .strike("strike:projected-death", CombatDeathFixture::HEALTH)
        .expect("lethal commit");
    let projected = fixture.project_death().expect("death");
    let actor = fixture.actor();
    assert_eq!(
        fixture.borrow_combat_death().projected_death(actor),
        Ok(projected)
    );
}
