//! Isolated preproduction composition of typed Ability and the physical owner slot.
use super::super::exact_actor_test_ability::commit::{OwnerCommitError, commit_exact_owner_damage};
use super::super::exact_actor_test_ability::exact_actor_resolution::{
    ExactActorProposal, ExactActorResolutionError, ResolvedExactActor, resolve_exact_actor,
};
use super::super::exact_actor_test_ability::{
    AbilityIntent, AbilityOccurrence, CalculationStage, CommitGroup, Effect, EffectPlan,
    ProposalSource, RevisionSet,
};
use super::*;

/// D4: the typed bridge now needs the attacker and its actual FND-02 `CommandRef`; these tests
/// commit every typed plan as attacker 1 of session 1 at command 1, so an identical plan replays
/// as the same command.
fn typed_commit(
    owner: &mut CurrentOwnerExactActorCommit<'_>,
    resolved: &ResolvedExactActor,
    plan: &EffectPlan,
) -> Result<OwnerDamageResult, OwnerCommitError> {
    let mut session = [0_u8; 16];
    session[6] = 0x70;
    session[8] = 0x80;
    session[15] = 1;
    let mut character = session;
    character[15] = 2;
    commit_exact_owner_damage(
        owner,
        resolved,
        plan,
        CharacterId::decode(&character).expect("attacker"),
        1,
        CommandRef::new(
            GameSessionId::decode(&session).expect("session"),
            super::super::CommandId::new(1).expect("command"),
        ),
    )
}

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
    let actor = ExactActorRef(
        carrier
            .admit_creature(&continuity, ActorState(1), "target:one", 20)
            .expect("fixture HP"),
    );
    (continuity, carrier, actor)
}

fn occurrence(id: &str, revision: &str) -> AbilityOccurrence {
    AbilityOccurrence::new(
        id,
        RevisionSet::new(
            revision,
            "content:1",
            "world:1",
            "formula:1",
            "simulation:1",
        )
        .expect("revisions"),
    )
    .expect("occurrence")
}

fn plan(occurrence: AbilityOccurrence, target: &str, damage: i64) -> EffectPlan {
    EffectPlan::immediate(
        occurrence,
        AbilityIntent::normalize(ProposalSource::Client, "actor:fixture", &[target])
            .expect("intent"),
        vec![Effect::damage(target, damage).expect("damage")],
        vec![],
        CommitGroup::atomic("scope:fixture", "group:one").expect("group"),
    )
    .expect("typed plan")
}

fn resolve(
    carrier: &ChannelActorCarrier,
    continuity: &NamespaceContinuityGuard,
    actor: ExactActorRef,
    occurrence: &AbilityOccurrence,
) -> super::super::exact_actor_test_ability::exact_actor_resolution::ResolvedExactActor {
    resolve_exact_actor(
        &carrier.current_owner_exact_lookup(continuity),
        occurrence,
        ExactActorProposal::client(actor),
    )
    .expect("current actor")
}

fn health(carrier: &ChannelActorCarrier) -> Option<i64> {
    match &carrier.slots[0] {
        Slot::CreatureOccupied { health, .. } => Some(*health),
        _ => None,
    }
}

fn command<'a>(occurrence: &'a [u8], binding: &'a [u8], damage: i64) -> OwnerDamageCommand<'a> {
    OwnerDamageCommand {
        target: b"target:one",
        occurrence,
        binding,
        damage,
    }
}

#[test]
fn typed_plan_commits_once_and_identical_replay_returns_original_transition() {
    let (continuity, mut carrier, actor) = fixture(10);
    let cast = occurrence("cast:1", "rules:1");
    let resolved = resolve(&carrier, &continuity, actor, &cast);
    let plan = plan(cast, "target:one", 7);
    let first = typed_commit(
        &mut carrier.current_owner_exact_commit(&continuity),
        &resolved,
        &plan,
    )
    .expect("owner commit");
    assert_eq!(
        (first.applied, first.health_before, first.health_after),
        (true, 20, 13)
    );
    let after = carrier.slots.clone();
    let replay = typed_commit(
        &mut carrier.current_owner_exact_commit(&continuity),
        &resolved,
        &plan,
    )
    .expect("same exact plan replay");
    assert_eq!(
        (replay.applied, replay.health_before, replay.health_after),
        (false, 20, 13)
    );
    assert_eq!(carrier.slots, after);
    assert_eq!(health(&carrier), Some(13));
}

#[test]
fn lethal_damage_disables_actions_but_administrative_remove_is_not_death() {
    let (continuity, mut carrier, actor) = fixture(20);
    let cast = occurrence("cast:lethal", "rules:1");
    let resolved = resolve(&carrier, &continuity, actor, &cast);
    let p = plan(cast.clone(), "target:one", 25);
    let committed = typed_commit(
        &mut carrier.current_owner_exact_commit(&continuity),
        &resolved,
        &p,
    )
    .expect("lethal HP transition");
    assert_eq!(committed.health_after, 0);
    assert!(
        !carrier
            .current_owner_exact_lookup(&continuity)
            .contains(actor)
    );
    assert_eq!(
        resolve_exact_actor(
            &carrier.current_owner_exact_lookup(&continuity),
            &cast,
            ExactActorProposal::client(actor)
        ),
        Err(ExactActorResolutionError::NotCurrentActor)
    );
    let before = carrier.slots.clone();
    assert_eq!(
        carrier.commit_creature_damage_inner(
            &continuity,
            actor.0,
            command(b"cast:later", b"cast:later\0binding", 1),
            None,
            false,
        ),
        Err(CarrierError::CreatureNotActionable)
    );
    assert_eq!(carrier.slots, before);
    assert_eq!(carrier.remove(&continuity, actor.0), Ok(ActorState(1)));
    assert!(matches!(carrier.slots[0], Slot::VacantReusable { .. }));
    let recycled = carrier
        .admit_creature(&continuity, ActorState(2), "target:one", 30)
        .expect("recycle");
    assert_eq!(health(&carrier), Some(30));
    assert_ne!(
        actor.0.actor_local_generation,
        recycled.actor_local_generation
    );
}

#[test]
fn revisions_plan_and_target_substitutions_leave_slot_byte_identical() {
    let (continuity, mut carrier, actor) = fixture(30);
    let cast = occurrence("cast:1", "rules:1");
    let resolved = resolve(&carrier, &continuity, actor, &cast);
    let original = plan(cast.clone(), "target:one", 5);
    typed_commit(
        &mut carrier.current_owner_exact_commit(&continuity),
        &resolved,
        &original,
    )
    .expect("first commit");
    let after = carrier.slots.clone();
    for substituted in [
        plan(occurrence("cast:1", "rules:2"), "target:one", 5),
        plan(cast.clone(), "target:one", 6),
        plan(cast.clone(), "target:two", 5),
        EffectPlan::immediate(
            cast.clone(),
            AbilityIntent::normalize(ProposalSource::Client, "actor:other", &["target:one"])
                .expect("changed actor marker"),
            vec![Effect::damage("target:one", 5).expect("damage")],
            vec![],
            CommitGroup::atomic("scope:fixture", "group:one").expect("group"),
        )
        .expect("changed intent"),
        EffectPlan::immediate(
            cast.clone(),
            AbilityIntent::normalize(ProposalSource::Client, "actor:fixture", &["target:one"])
                .expect("intent"),
            vec![Effect::damage("target:one", 5).expect("damage")],
            vec![CalculationStage::new("stage:other").expect("stage")],
            CommitGroup::atomic("scope:fixture", "group:one").expect("group"),
        )
        .expect("changed stage"),
        EffectPlan::immediate(
            cast.clone(),
            AbilityIntent::normalize(ProposalSource::Client, "actor:fixture", &["target:one"])
                .expect("intent"),
            vec![Effect::damage("target:one", 5).expect("damage")],
            vec![],
            CommitGroup::atomic("scope:fixture", "group:other").expect("group"),
        )
        .expect("changed group"),
    ] {
        assert!(
            typed_commit(
                &mut carrier.current_owner_exact_commit(&continuity),
                &resolved,
                &substituted
            )
            .is_err()
        );
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
    assert_eq!(
        carrier.commit_creature_damage_inner(
            &continuity,
            wrong_actor,
            command(b"cast:1", b"cast:1\0binding", 5),
            None,
            false,
        ),
        Err(CarrierError::WrongScope)
    );
    assert_eq!(carrier.slots, before);
    carrier.remove(&continuity, actor.0).expect("admin remove");
    let recycled = carrier
        .admit_creature(&continuity, ActorState(2), "target:one", 20)
        .expect("recycle");
    let after = carrier.slots.clone();
    assert_eq!(
        typed_commit(
            &mut carrier.current_owner_exact_commit(&continuity),
            &resolved,
            &p
        ),
        Err(OwnerCommitError::Owner(CarrierError::StaleActorGeneration))
    );
    assert_eq!(carrier.slots, after);
    continuity.advance(grant(40, 2)).expect("new owner");
    assert_eq!(
        carrier.commit_creature_damage_inner(
            &continuity,
            recycled,
            command(b"cast:1", b"cast:1\0binding", 5),
            None,
            false,
        ),
        Err(CarrierError::WrongScope)
    );
    assert_eq!(carrier.slots, after);
}

#[test]
fn invalid_magnitude_overflow_and_injected_failure_do_not_mutate_slot() {
    let (continuity, mut carrier, actor) = fixture(50);
    let before = carrier.slots.clone();
    for damage in [0, -1] {
        assert_eq!(
            carrier.commit_creature_damage_inner(
                &continuity,
                actor.0,
                command(b"cast:1", b"cast:1\0binding", damage),
                None,
                false,
            ),
            Err(CarrierError::InvalidDamage)
        );
        assert_eq!(carrier.slots, before);
    }
    assert_eq!(
        carrier.commit_creature_damage_inner(
            &continuity,
            actor.0,
            command(b"cast:1", b"cast:1\0binding", 5),
            None,
            true,
        ),
        Err(CarrierError::InjectedCommitFailure)
    );
    assert_eq!(carrier.slots, before);
    if let Slot::CreatureOccupied { health, .. } = &mut carrier.slots[0] {
        *health = i64::MIN;
    }
    let corrupt = carrier.slots.clone();
    assert_eq!(
        carrier.commit_creature_damage_inner(
            &continuity,
            actor.0,
            command(b"cast:1", b"cast:1\0binding", 1),
            None,
            false,
        ),
        Err(CarrierError::DamageOverflow)
    );
    assert_eq!(carrier.slots, corrupt);
}

#[test]
fn target_is_bound_to_owner_slot_before_first_commit() {
    let (continuity, mut carrier, actor) = fixture(70);
    let cast = occurrence("cast:target", "rules:1");
    let resolved = resolve(&carrier, &continuity, actor, &cast);
    let before = carrier.slots.clone();
    assert_eq!(
        typed_commit(
            &mut carrier.current_owner_exact_commit(&continuity),
            &resolved,
            &plan(cast.clone(), "target:other", 5),
        ),
        Err(OwnerCommitError::Owner(
            CarrierError::CreatureTargetMismatch
        ))
    );
    assert_eq!(carrier.slots, before);
    assert_eq!(health(&carrier), Some(20));
    assert!(
        typed_commit(
            &mut carrier.current_owner_exact_commit(&continuity),
            &resolved,
            &plan(cast, "target:one", 5),
        )
        .is_ok()
    );
}

#[test]
fn general_capacity_and_binding_size_bound_reject_without_mutation() {
    // AI-2 (GAME-AI-01 §4.1 envelope): a one-slot carrier's second admission now fails on
    // general capacity (not a one-creature-only gate); a wider carrier's second creature
    // admission succeeds instead (`ai2_multiple_creatures_coexist_bounded_only_by_general_capacity`,
    // `runtime_actor_carrier`'s own tests).
    let mut continuity = NamespaceContinuityGuard::from_pre_production_grant(grant(80, 1));
    let mut carrier =
        ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 1).expect("one finite slot");
    let actor = ExactActorRef(
        carrier
            .admit_creature(&continuity, ActorState(1), "target:one", 20)
            .expect("first creature"),
    );
    let before = carrier.slots.clone();
    assert_eq!(
        carrier.admit_creature(&continuity, ActorState(2), "target:two", 20),
        Err(CarrierError::CapacityExceeded)
    );
    assert_eq!(carrier.slots, before);
    let mut maximum = b"cast:1\0".to_vec();
    maximum.resize(MAX_OWNER_COMMIT_BINDING_BYTES, b'x');
    assert_eq!(
        carrier.commit_creature_damage_inner(
            &continuity,
            actor.0,
            command(b"cast:1", &maximum, 1),
            None,
            true,
        ),
        Err(CarrierError::InjectedCommitFailure)
    );
    assert_eq!(carrier.slots, before);
    maximum.push(b'x');
    assert_eq!(
        carrier.commit_creature_damage_inner(
            &continuity,
            actor.0,
            command(b"cast:1", &maximum, 1),
            None,
            false,
        ),
        Err(CarrierError::CommitBindingTooLarge)
    );
    assert_eq!(carrier.slots, before);
}

#[test]
fn target_marker_4096_accepts_and_4097_rejects_before_slot_mutation() {
    let mut continuity = NamespaceContinuityGuard::from_pre_production_grant(grant(90, 1));
    let mut carrier =
        ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 1).expect("finite carrier");
    let before = carrier.slots.clone();
    let maximum = "a".repeat(MAX_OWNER_COMMIT_BINDING_BYTES);
    let excessive = "a".repeat(MAX_OWNER_COMMIT_BINDING_BYTES + 1);
    assert_eq!(
        carrier.admit_creature(&continuity, ActorState(1), &excessive, 20),
        Err(CarrierError::InvalidCreatureTarget)
    );
    assert_eq!(carrier.slots, before);
    assert!(
        carrier
            .admit_creature(&continuity, ActorState(1), &maximum, 20)
            .is_ok()
    );
    assert_eq!(health(&carrier), Some(20));
}

// --- D4 (D140-D144): bounded multi-occurrence damage receipts ---------------------------------

fn who(seed: u8) -> CharacterId {
    let mut bytes = [seed; 16];
    bytes[6] = 0x70;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    CharacterId::decode(&bytes).expect("uuid v7 character")
}

fn session(seed: u8) -> GameSessionId {
    let mut bytes = [seed; 16];
    bytes[6] = 0x70;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    GameSessionId::decode(&bytes).expect("uuid v7 session")
}

fn attack(character: u8, session_seed: u8, sequence: u64, sub_ordinal: u16) -> AttackerCommand {
    // The lease generation is the session seed: a higher session seed is a newer session.
    AttackerCommand::new(
        who(character),
        u64::from(session_seed),
        CommandRef::new(
            session(session_seed),
            super::super::CommandId::new(sequence).expect("non-zero command"),
        ),
        sub_ordinal,
    )
}

fn tall_fixture(
    seed: u64,
    health: i64,
) -> (NamespaceContinuityGuard, ChannelActorCarrier, ExactActorRef) {
    let mut continuity = NamespaceContinuityGuard::from_pre_production_grant(grant(seed, 1));
    let mut carrier = ChannelActorCarrier::bootstrap_pre_production(&mut continuity, 1)
        .expect("one explicit carrier slot");
    let actor = ExactActorRef(
        carrier
            .admit_creature(&continuity, ActorState(1), "target:one", health)
            .expect("fixture HP"),
    );
    (continuity, carrier, actor)
}

/// Deterministic per-command binding: an exact resubmission of the same command carries the
/// same bytes, a different command carries different bytes.
fn binding_of(command: AttackerCommand, damage: i64) -> Vec<u8> {
    format!(
        "plan:{}:{}:{}:{}\0damage:{damage}",
        command.character.as_bytes()[0],
        command.session.as_bytes()[0],
        command.sequence,
        command.sub_ordinal
    )
    .into_bytes()
}

fn hit(
    carrier: &mut ChannelActorCarrier,
    continuity: &NamespaceContinuityGuard,
    actor: ExactActorRef,
    command: AttackerCommand,
    damage: i64,
) -> Result<OwnerDamageResult, CarrierError> {
    // The caller-supplied `occurrence` bytes are deliberately junk: an attributed commit's
    // identity is derived from `command` alone.
    carrier.commit_creature_damage_inner(
        continuity,
        actor.0,
        command_with(
            b"caller-supplied-junk",
            &binding_of(command, damage),
            damage,
        ),
        Some(command),
        false,
    )
}

fn unsequenced(
    carrier: &mut ChannelActorCarrier,
    continuity: &NamespaceContinuityGuard,
    actor: ExactActorRef,
    tag: &str,
    damage: i64,
) -> Result<OwnerDamageResult, CarrierError> {
    let binding = format!("{tag}\0unsequenced").into_bytes();
    carrier.commit_creature_damage_inner(
        continuity,
        actor.0,
        command_with(tag.as_bytes(), &binding, damage),
        None,
        false,
    )
}

fn command_with<'a>(
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

fn receipts(carrier: &ChannelActorCarrier) -> &DamageReceipts {
    match &carrier.slots[0] {
        Slot::CreatureOccupied { committed, .. } => Some(committed),
        _ => None,
    }
    .expect("expected the creature slot")
}

fn origins(carrier: &ChannelActorCarrier) -> Vec<Option<DamageOrigin>> {
    receipts(carrier)
        .entries
        .iter()
        .map(|record| record.origin)
        .collect()
}

fn charm_child(parent: &EffectPlan, generated: i64) -> EffectPlan {
    EffectPlan::ordered_sequential(
        parent.occurrence().clone(),
        parent.intent().clone(),
        vec![
            parent.effects()[0].clone(),
            Effect::damage(parent.effects()[0].target().as_str(), generated).expect("generated"),
        ],
        vec![CalculationStage::new("charm:generated").expect("stage")],
        parent.commit_group().owner_scope(),
        parent.commit_group().group_id(),
    )
    .expect("declared child entries")
}

fn charm_magnitudes(plan: &EffectPlan) -> Vec<i64> {
    plan.effects().iter().map(Effect::magnitude).collect()
}

#[test]
fn charm_native_declared_child_order_preserves_canonical_constructor() {
    let parent = plan(occurrence("charm:order", "rules:1"), "target:one", 3);
    let ordered = charm_child(&parent, 9);
    let canonical = EffectPlan::new(
        ordered.occurrence().clone(),
        ordered.intent().clone(),
        ordered.effects().to_vec(),
        vec![],
        ordered.commit_group().clone(),
    )
    .expect("legacy constructor");
    assert_eq!(charm_magnitudes(&ordered), [3, 9]);
    assert_eq!(charm_magnitudes(&canonical), [9, 3]);
    assert_eq!(
        ordered
            .sub_occurrence(1)
            .expect("real child index")
            .ordinal(),
        1
    );
}

#[test]
fn receipt_and_sub_ordinal_bounds_match_the_registered_rows() {
    assert_eq!(COMBAT01_DAMAGE_RECEIPTS_PER_CREATURE_GENERATION_MAX, 16);
    assert_eq!(
        usize::from(ABILITY01_EFFECT_PLAN_ENTRIES_MAX),
        super::super::exact_actor_test_ability::MAX_EFFECT_PLAN_ENTRIES
    );
}

#[test]
fn every_retained_occurrence_replays_and_an_evicted_one_is_refused_never_reapplied() {
    let (continuity, mut carrier, actor) = tall_fixture(200, 1_000);
    let mut results = Vec::new();
    for sequence in 1..=16_u64 {
        results.push(
            hit(
                &mut carrier,
                &continuity,
                actor,
                attack(1, 1, sequence, 0),
                3,
            )
            .expect("hit"),
        );
    }
    assert_eq!(receipts(&carrier).entries.len(), 16);
    assert_eq!(health(&carrier), Some(1_000 - 48));
    let full = carrier.slots.clone();
    // Replay of each retained occurrence: byte-identical original transition, nothing mutated.
    for (index, original) in results.iter().enumerate() {
        let sequence = u64::try_from(index).expect("index") + 1;
        let replay = hit(
            &mut carrier,
            &continuity,
            actor,
            attack(1, 1, sequence, 0),
            3,
        )
        .expect("retained replay");
        assert_eq!(
            replay,
            OwnerDamageResult {
                applied: false,
                ..*original
            }
        );
        assert_eq!(carrier.slots, full);
    }
    // The 17th distinct occurrence evicts the oldest and still applies.
    let seventeenth = hit(&mut carrier, &continuity, actor, attack(1, 1, 17, 0), 3).expect("17th");
    assert!(seventeenth.applied);
    assert_eq!(receipts(&carrier).entries.len(), 16);
    assert_eq!(health(&carrier), Some(1_000 - 51));
    assert!(
        !origins(&carrier).contains(&Some(attack(1, 1, 1, 0).origin())),
        "the oldest receipt (sequence 1) was evicted"
    );
    // The evicted occurrence's resubmission is refused as stale and mutates nothing: no
    // double-apply.
    let after_eviction = carrier.slots.clone();
    assert_eq!(
        hit(&mut carrier, &continuity, actor, attack(1, 1, 1, 0), 3),
        Err(CarrierError::StaleAttackerSequence)
    );
    assert_eq!(carrier.slots, after_eviction);
    assert_eq!(health(&carrier), Some(1_000 - 51));
    // A still-retained occurrence keeps replaying after the eviction.
    assert!(
        !hit(&mut carrier, &continuity, actor, attack(1, 1, 2, 0), 3)
            .expect("retained")
            .applied
    );
}

#[test]
fn conflict_and_dead_creature_are_distinct_refusals() {
    let (continuity, mut carrier, actor) = tall_fixture(210, 10);
    hit(&mut carrier, &continuity, actor, attack(1, 1, 1, 0), 4).expect("first");
    let before = carrier.slots.clone();
    // Same occurrence identity, different damage or different binding: PlanConflict.
    assert_eq!(
        hit(&mut carrier, &continuity, actor, attack(1, 1, 1, 0), 5),
        Err(CarrierError::PlanConflict)
    );
    assert_eq!(
        carrier.commit_creature_damage_inner(
            &continuity,
            actor.0,
            command_with(b"x", b"a-different-binding", 4),
            Some(attack(1, 1, 1, 0)),
            false,
        ),
        Err(CarrierError::PlanConflict)
    );
    assert_eq!(carrier.slots, before);
    // Kill it; a genuinely new occurrence is CreatureNotActionable, a retained replay still
    // returns its original transition.
    let lethal = hit(&mut carrier, &continuity, actor, attack(1, 1, 2, 0), 100).expect("lethal");
    assert_eq!((lethal.health_before, lethal.health_after), (6, 0));
    let dead = carrier.slots.clone();
    assert_eq!(
        hit(&mut carrier, &continuity, actor, attack(1, 1, 3, 0), 1),
        Err(CarrierError::CreatureNotActionable)
    );
    assert_eq!(
        unsequenced(&mut carrier, &continuity, actor, "cast:later", 1),
        Err(CarrierError::CreatureNotActionable)
    );
    assert_eq!(carrier.slots, dead);
    assert!(
        !hit(&mut carrier, &continuity, actor, attack(1, 1, 1, 0), 4)
            .expect("replay of a retained non-lethal hit")
            .applied
    );
    assert_eq!(carrier.slots, dead);
}

#[test]
fn a_sequence_at_or_below_the_session_mark_is_stale_and_lexicographic_with_sub_ordinal() {
    let (continuity, mut carrier, actor) = tall_fixture(220, 1_000);
    hit(&mut carrier, &continuity, actor, attack(1, 1, 5, 0), 1).expect("mark (5,0)");
    let marked = carrier.slots.clone();
    for stale in [attack(1, 1, 4, 0), attack(1, 1, 4, 1), attack(1, 1, 1, 0)] {
        assert_eq!(
            hit(&mut carrier, &continuity, actor, stale, 1),
            Err(CarrierError::StaleAttackerSequence)
        );
        assert_eq!(carrier.slots, marked);
    }
    // The second sub-occurrence of the same command is above the mark: (5,0) < (5,1).
    assert!(
        hit(&mut carrier, &continuity, actor, attack(1, 1, 5, 1), 1)
            .expect("(5,1)")
            .applied
    );
    // The retained (5,0) receipt still replays; a new (6,0) is above the mark.
    assert!(
        !hit(&mut carrier, &continuity, actor, attack(1, 1, 5, 0), 1)
            .expect("retained replay")
            .applied
    );
    assert!(
        hit(&mut carrier, &continuity, actor, attack(1, 1, 6, 0), 1)
            .expect("(6,0)")
            .applied
    );
    // A different attacker has its own mark and is unaffected.
    assert!(
        hit(&mut carrier, &continuity, actor, attack(2, 1, 1, 0), 1)
            .expect("other attacker")
            .applied
    );
}

#[test]
fn reconnect_accepts_a_new_sessions_low_sequence_and_the_fence_refuses_the_old_session() {
    use super::super::{ConnectionFence, ConnectionGeneration};
    let (continuity, mut carrier, actor) = tall_fixture(230, 1_000);
    let mut fence = ConnectionFence::fresh_admission();
    let session_a_generation = fence.current();
    assert_eq!(
        session_a_generation,
        ConnectionGeneration::new(1).expect("gen")
    );

    let original =
        hit(&mut carrier, &continuity, actor, attack(1, 1, 5, 0), 2).expect("A, sequence 5");
    // Within the same session a retained sequence replays and a lower one is stale.
    let marked = carrier.slots.clone();
    assert_eq!(
        hit(&mut carrier, &continuity, actor, attack(1, 1, 5, 0), 2),
        Ok(OwnerDamageResult {
            applied: false,
            ..original
        })
    );
    assert_eq!(
        hit(&mut carrier, &continuity, actor, attack(1, 1, 3, 0), 2),
        Err(CarrierError::StaleAttackerSequence)
    );
    assert_eq!(carrier.slots, marked);

    // The attacker reconnects under a genuinely new GameSessionId: the fence advances...
    let session_b_generation = fence.rebind(session_a_generation).expect("rebind");
    // ...and its low starting sequence is accepted, not compared against A's mark.
    let b = hit(&mut carrier, &continuity, actor, attack(1, 2, 1, 0), 2).expect("B, sequence 1");
    assert!(b.applied);
    assert_eq!(health(&carrier), Some(1_000 - 4));
    // B's own mark now governs.
    assert!(
        !hit(&mut carrier, &continuity, actor, attack(1, 2, 1, 0), 2)
            .expect("B replay")
            .applied
    );
    // The old session's late command never reaches the carrier: the session-generation fence
    // refuses it upstream (also proven by `reconnect_advances_generation_and_fences_stale_transport`
    // in `foundation/mod.rs`; repeated here for the exact reconnect shape).
    assert!(!fence.accepts(session_a_generation));
    assert!(fence.accepts(session_b_generation));
}

#[test]
fn a_callers_opaque_bytes_can_never_alias_or_split_a_commands_identity() {
    let (continuity, mut carrier, actor) = tall_fixture(240, 1_000);
    let a1 = attack(1, 1, 1, 0);
    let original = carrier
        .commit_creature_damage_inner(
            &continuity,
            actor.0,
            command_with(b"cast:shared", b"shared-binding", 2),
            Some(a1),
            false,
        )
        .expect("session A, sequence 1");
    assert!(original.applied);
    // Same quadruple, different caller `occurrence` bytes: still the same replay.
    let replay = carrier
        .commit_creature_damage_inner(
            &continuity,
            actor.0,
            command_with(b"entirely-other-bytes", b"shared-binding", 2),
            Some(a1),
            false,
        )
        .expect("exact quadruple replays");
    assert_eq!(
        replay,
        OwnerDamageResult {
            applied: false,
            ..original
        }
    );
    // Push A1 out of the retained set with 16 more A hits.
    for sequence in 2..=17_u64 {
        hit(
            &mut carrier,
            &continuity,
            actor,
            attack(1, 1, sequence, 0),
            2,
        )
        .expect("filler");
    }
    assert!(!origins(&carrier).contains(&Some(a1.origin())));
    // Session B, sequence 1, reusing A's old literal bytes: its own new occurrence, applied.
    let before = health(&carrier);
    let b1 = carrier
        .commit_creature_damage_inner(
            &continuity,
            actor.0,
            command_with(b"cast:shared", b"shared-binding", 2),
            Some(attack(1, 2, 1, 0)),
            false,
        )
        .expect("session B is its own occurrence");
    assert!(b1.applied);
    assert_eq!(health(&carrier), before.map(|value| value - 2));
}

#[test]
fn two_sub_occurrences_of_one_command_are_distinct_receipts_and_the_bound_is_enforced() {
    let (continuity, mut carrier, actor) = tall_fixture(250, 1_000);
    let first = hit(&mut carrier, &continuity, actor, attack(1, 1, 1, 0), 3).expect("sub 0");
    let second = hit(&mut carrier, &continuity, actor, attack(1, 1, 1, 1), 4).expect("sub 1");
    assert!(first.applied && second.applied);
    assert_eq!(receipts(&carrier).entries.len(), 2);
    assert_eq!(health(&carrier), Some(1_000 - 7));
    let both = carrier.slots.clone();
    assert_eq!(
        hit(&mut carrier, &continuity, actor, attack(1, 1, 1, 0), 3).expect("replay sub 0"),
        OwnerDamageResult {
            applied: false,
            ..first
        }
    );
    assert_eq!(carrier.slots, both);
    // At ABILITY01_EFFECT_PLAN_ENTRIES_MAX the ordinal is refused before any other check.
    let out_of_range = attack(1, 1, 1, ABILITY01_EFFECT_PLAN_ENTRIES_MAX);
    assert_eq!(
        hit(&mut carrier, &continuity, actor, out_of_range, 3),
        Err(CarrierError::SubOrdinalOutOfRange)
    );
    assert_eq!(
        carrier.commit_creature_damage_inner(
            &continuity,
            actor.0,
            command_with(b"", b"", 0),
            Some(out_of_range),
            false,
        ),
        Err(CarrierError::SubOrdinalOutOfRange)
    );
    assert_eq!(carrier.slots, both);
}

#[test]
fn a_reconnect_orphaned_receipt_is_evicted_and_never_starves_new_hits() {
    let (continuity, mut carrier, actor) = tall_fixture(260, 1_000);
    hit(&mut carrier, &continuity, actor, attack(1, 1, 1, 0), 1).expect("A1");
    for sequence in 1..=15_u64 {
        hit(
            &mut carrier,
            &continuity,
            actor,
            attack(1, 2, sequence, 0),
            1,
        )
        .expect("B hit");
    }
    assert_eq!(receipts(&carrier).entries.len(), 16);
    assert_eq!(origins(&carrier)[0], Some(attack(1, 1, 1, 0).origin()));
    // B16 needs room: the superseded A1 receipt is the oldest evictable one.
    assert!(
        hit(&mut carrier, &continuity, actor, attack(1, 2, 16, 0), 1)
            .expect("B16 admitted")
            .applied
    );
    let retained = origins(&carrier);
    assert_eq!(retained.len(), 16);
    assert!(!retained.contains(&Some(attack(1, 1, 1, 0).origin())));
    assert_eq!(retained[0], Some(attack(1, 2, 1, 0).origin()));
    assert_eq!(health(&carrier), Some(1_000 - 17));
}

#[test]
fn unsequenced_receipts_are_never_evicted_and_can_fill_the_list() {
    let (continuity, mut carrier, actor) = tall_fixture(270, 1_000);
    for tag in ["cast:env:1", "cast:env:2", "cast:env:3"] {
        unsequenced(&mut carrier, &continuity, actor, tag, 1).expect("unsequenced hit");
    }
    for sequence in 1..=40_u64 {
        hit(
            &mut carrier,
            &continuity,
            actor,
            attack(1, 1, sequence, 0),
            1,
        )
        .expect("attributed");
    }
    let retained = origins(&carrier);
    assert_eq!(retained.len(), 16);
    assert_eq!(retained.iter().filter(|origin| origin.is_none()).count(), 3);
    assert!(
        !unsequenced(&mut carrier, &continuity, actor, "cast:env:1", 1)
            .expect("the oldest unsequenced receipt still replays")
            .applied
    );

    // Max: sixteen simultaneously non-evictable receipts retain; max+1 is refused fail-closed
    // (GAME-ABILITY-01 section 12) with HP and every receipt untouched.
    let (continuity, mut carrier, actor) = tall_fixture(271, 1_000);
    for index in 0..16 {
        unsequenced(
            &mut carrier,
            &continuity,
            actor,
            &format!("cast:env:{index}"),
            1,
        )
        .expect("max non-evictable receipts retain");
    }
    let full = carrier.slots.clone();
    assert_eq!(
        unsequenced(&mut carrier, &continuity, actor, "cast:env:16", 1),
        Err(CarrierError::DamageReceiptCapacityExceeded)
    );
    // An attributed 17th finds no evictable receipt either.
    assert_eq!(
        hit(&mut carrier, &continuity, actor, attack(1, 1, 1, 0), 1),
        Err(CarrierError::DamageReceiptCapacityExceeded)
    );
    assert_eq!(carrier.slots, full);
    assert_eq!(health(&carrier), Some(1_000 - 16));
}

#[test]
fn receipts_of_untracked_seventeenth_attackers_are_not_evictable() {
    let (continuity, mut carrier, actor) = tall_fixture(280, 1_000);
    // 16 tracked attackers fill both the contributor map and the receipt list.
    for seed in 1..=16_u8 {
        hit(&mut carrier, &continuity, actor, attack(seed, 1, 1, 0), 1).expect("tracked");
    }
    // Each untracked 17th-plus attacker evicts one tracked (evictable) receipt and lands its
    // own, non-evictable one.
    for seed in 17..=32_u8 {
        assert!(
            hit(&mut carrier, &continuity, actor, attack(seed, 1, 1, 0), 1)
                .expect("evicts a tracked receipt")
                .applied
        );
    }
    assert!(
        origins(&carrier)
            .iter()
            .all(|origin| origin.is_some_and(|(character, ..)| character >= who(17)))
    );
    assert_eq!(
        hit(&mut carrier, &continuity, actor, attack(33, 1, 1, 0), 1),
        Err(CarrierError::DamageReceiptCapacityExceeded)
    );
    assert_eq!(health(&carrier), Some(1_000 - 32));
}

fn attack_with_lease(
    character: u8,
    lease_generation: u64,
    session_seed: u8,
    sequence: u64,
) -> AttackerCommand {
    AttackerCommand::new(
        who(character),
        lease_generation,
        CommandRef::new(
            session(session_seed),
            super::super::CommandId::new(sequence).expect("non-zero command"),
        ),
        0,
    )
}

#[test]
fn a_delayed_command_of_a_superseded_session_is_refused_even_after_its_receipt_was_evicted() {
    let (continuity, mut carrier, actor) = tall_fixture(290, 1_000);
    // Session A (lease generation 1) lands a hit, then the attacker reconnects: session B (lease
    // generation 2) replaces the mark and 16 more B hits evict A's receipt.
    hit(&mut carrier, &continuity, actor, attack(1, 1, 1, 0), 2).expect("A1");
    for sequence in 1..=17_u64 {
        hit(
            &mut carrier,
            &continuity,
            actor,
            attack(1, 2, sequence, 0),
            1,
        )
        .expect("B hit");
    }
    assert!(!origins(&carrier).contains(&Some(attack(1, 1, 1, 0).origin())));
    let before = carrier.slots.clone();
    let hp = health(&carrier);
    // The delayed A command (its receipt is gone) and a never-seen A command are both refused.
    for delayed in [attack(1, 1, 1, 0), attack(1, 1, 99, 0)] {
        assert_eq!(
            hit(&mut carrier, &continuity, actor, delayed, 2),
            Err(CarrierError::SupersededAttackerSession)
        );
        assert_eq!(carrier.slots, before);
    }
    assert_eq!(health(&carrier), hp);
    // Equal lease generation with a differing session is superseded too.
    assert_eq!(
        hit(
            &mut carrier,
            &continuity,
            actor,
            attack_with_lease(1, 2, 3, 1),
            1
        ),
        Err(CarrierError::SupersededAttackerSession)
    );
    assert_eq!(carrier.slots, before);
    // A strictly higher lease generation is a newer session and replaces the mark.
    assert!(
        hit(
            &mut carrier,
            &continuity,
            actor,
            attack_with_lease(1, 3, 3, 1),
            1
        )
        .expect("newer lease")
        .applied
    );
    assert_eq!(
        hit(&mut carrier, &continuity, actor, attack(1, 2, 18, 0), 1),
        Err(CarrierError::SupersededAttackerSession)
    );
}

#[test]
fn an_unsequenced_occurrence_containing_the_nul_delimiter_is_rejected_before_mutation() {
    let (continuity, mut carrier, actor) = tall_fixture(295, 1_000);
    let before = carrier.slots.clone();
    assert_eq!(
        carrier.commit_creature_damage_inner(
            &continuity,
            actor.0,
            command_with(b"cast\0ambiguous", b"cast\0ambiguous\0plan", 1),
            None,
            false,
        ),
        Err(CarrierError::InvalidCommitBinding)
    );
    assert_eq!(carrier.slots, before);
}
