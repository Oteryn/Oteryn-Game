#![allow(clippy::expect_used)]
use crate::ability::RevisionSet;
use crate::content::{
    ProjectReferenceRecord, ProjectV2AuthoringProfile, ProjectV2AuthoringProfileData as Data,
    ProjectV2DefinitionRef as Ref, ProjectV2Family,
};
use crate::creature_auto_attack::*;
use crate::foundation::owner_timer::SemanticTimeMicros;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition};
use crate::gameplay_transport::actor_spell::{
    ChannelSpellStates,
    tests::{FACTS, runtime_with_player},
};
use crate::player_lethal::PlayerLethalVitals;
use oteryn_simulation_determinism::GameplayDecisionRoot;

// Positive-only fixture grant; real runtime paths receive the independently current owner fence.
trait AutoAttackOwnerFixtureExt {
    #[allow(clippy::too_many_arguments)]
    fn fixture_swing(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        vitals: &mut dyn PlayerLethalVitals,
        source: &MeleeSource,
        facts: AttackFacts,
        sequence: u64,
        revisions: RevisionSet,
        root: &GameplayDecisionRoot,
        now: SemanticTimeMicros,
    ) -> Result<SwingOutcome, AttackError>;
}

impl AutoAttackOwnerFixtureExt for AutoAttackOwner {
    #[allow(clippy::too_many_arguments)]
    fn fixture_swing(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        vitals: &mut dyn PlayerLethalVitals,
        source: &MeleeSource,
        facts: AttackFacts,
        sequence: u64,
        revisions: RevisionSet,
        root: &GameplayDecisionRoot,
        now: SemanticTimeMicros,
    ) -> Result<SwingOutcome, AttackError> {
        let (fence, stamp) = current_fence(runtime);
        self.swing(
            runtime, vitals, source, facts, sequence, revisions, root, now, &fence, stamp,
        )
    }
}

#[test]
fn source_swing_superseded_external_grant_refuses_before_primary_or_sequence_write() {
    let (mut runtime, mut states, source, facts, root) = setup("1st_mate_ratticus", false);
    let mut attack = owner(&runtime, &source, facts);
    let before = crate::gameplay_transport::actor_spell::observe_vitals(
        &runtime,
        &states,
        facts.target,
        facts.session,
    );
    let sequence = attack.next_sequence(facts.issuer, source.entry_index);
    let (mut fence, stamp) = current_fence(&runtime);
    fence
        .apply_external_grant(
            crate::foundation::ScopeOwnershipGeneration::new(2).expect("qualified fixture"),
        )
        .expect("qualified fixture");
    assert_eq!(
        attack.swing(
            &mut runtime,
            &mut states,
            &source,
            facts,
            1,
            revisions(),
            &root,
            clock(2_000_000),
            &fence,
            stamp
        ),
        Err(AttackError::StaleOwner)
    );
    assert_eq!(
        crate::gameplay_transport::actor_spell::observe_vitals(
            &runtime,
            &states,
            facts.target,
            facts.session
        ),
        before
    );
    assert_eq!(
        attack.next_sequence(facts.issuer, source.entry_index),
        sequence
    );
}

fn clock(us: u64) -> SemanticTimeMicros {
    SemanticTimeMicros::from_micros(us)
}
fn revisions() -> RevisionSet {
    RevisionSet::new(
        "rules-r1",
        "content-r1",
        "policy-r1",
        "definition-r1",
        "sim-r1",
    )
    .expect("revisions")
}
fn native_data() -> (Vec<ProjectReferenceRecord>, Vec<ProjectV2AuthoringProfile>) {
    let value: serde_json::Value =
        serde_json::from_str(include_str!("../creature_auto_attack_test_data.json"))
            .expect("source fixture");
    (
        serde_json::from_value(value["records"].clone()).expect("records"),
        serde_json::from_value(value["authoring_profiles"].clone()).expect("profiles"),
    )
}
fn setup(
    name: &str,
    far: bool,
) -> (
    ChannelRuntimeV1,
    ChannelSpellStates,
    MeleeSource,
    AttackFacts,
    GameplayDecisionRoot,
) {
    let (mut runtime, target, session) = runtime_with_player(0x71);
    runtime
        .initialize_movement_test_position(
            target,
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
        )
        .expect("actual position");
    let (r, p) = native_data();
    let key = format!("oteryn:creature.{name}");
    let health = p
        .iter()
        .find_map(|p| match &p.data {
            Data::Creature(c) if p.target.key == key => c.health,
            _ => None,
        })
        .expect("native source health");
    let issuer = runtime
        .admit_monster_lab_creature(
            MovementLocalPosition {
                x: if far { 110 } else { 101 },
                y: 100,
                floor: 7,
            },
            &key,
            i64::try_from(health).expect("source health bound"),
        )
        .expect("actual native admission");
    let mut states = ChannelSpellStates::default();
    states
        .initialize(
            &runtime,
            target,
            session,
            FACTS,
            (0, 0),
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
        )
        .expect("actual vitals");
    let cref = Ref {
        family: ProjectV2Family::Creature,
        key: format!("oteryn:creature.{name}"),
        revision: "definition-r1".into(),
    };
    let source = MeleeSource::from_native(
        &cref,
        0,
        &r,
        &p,
        runtime.content_pin().server_artifact_digest(),
    )
    .expect("source-bound melee");
    let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
    let facts = AttackFacts {
        issuer,
        target,
        session,
        revision: 1,
        visible: true,
        issuer_pz: false,
        target_pz: false,
        issuer_protected: false,
        target_protected: false,
        defense: 0,
        armor: 0,
    };
    (runtime, states, source, facts, root)
}
fn owner(runtime: &ChannelRuntimeV1, s: &MeleeSource, f: AttackFacts) -> AutoAttackOwner {
    let mut o = AutoAttackOwner::default();
    o.set_target(
        runtime,
        f.issuer,
        f.target,
        f.session,
        s,
        "target-command:1",
        clock(0),
    )
    .expect("current target");
    o
}
#[test]
fn native_range_auto_attack_commits_lethal_real_vitals_once() {
    let (mut runtime, mut states, source, facts, root) = setup("1st_mate_ratticus", false);
    let mut owner = owner(&runtime, &source, facts);
    assert_eq!(
        owner.fixture_swing(
            &mut runtime,
            &mut states,
            &source,
            facts,
            1,
            revisions(),
            &root,
            clock(1)
        ),
        Err(AttackError::NotDue)
    );
    let hit = owner
        .fixture_swing(
            &mut runtime,
            &mut states,
            &source,
            facts,
            1,
            revisions(),
            &root,
            clock(2_000_000),
        )
        .expect("real owner hit");
    assert!((200..=300).contains(&hit.requested));
    let damage = hit.damage.expect("actual damage receipt");
    assert_eq!(damage.applied, 185);
    assert_eq!(damage.health_after, 0);
    assert!(damage.death.is_some());
    assert_eq!(
        owner
            .fixture_swing(
                &mut runtime,
                &mut states,
                &source,
                facts,
                1,
                revisions(),
                &root,
                clock(9_000_000)
            )
            .expect("replay"),
        hit
    );
    let mut changed = facts;
    changed.revision = 2;
    assert_eq!(
        owner.fixture_swing(
            &mut runtime,
            &mut states,
            &source,
            changed,
            1,
            revisions(),
            &root,
            clock(9_000_000)
        ),
        Err(AttackError::OccurrenceConflict)
    );
}
#[test]
fn actual_armor_shield_and_stall_do_not_replay_backlog() {
    let (mut runtime, mut states, source, mut facts, root) = setup("1st_mate_ratticus", false);
    facts.defense = 1000;
    let mut owner = owner(&runtime, &source, facts);
    let blocked = owner
        .fixture_swing(
            &mut runtime,
            &mut states,
            &source,
            facts,
            1,
            revisions(),
            &root,
            clock(100_000_000),
        )
        .expect("blocked hit");
    assert_eq!(blocked.blocked, blocked.requested);
    assert!(blocked.damage.is_none());
    assert_eq!(
        owner.next_sequence(facts.issuer, source.entry_index),
        Some(2)
    );
    assert_eq!(
        owner.fixture_swing(
            &mut runtime,
            &mut states,
            &source,
            facts,
            2,
            revisions(),
            &root,
            clock(100_000_000)
        ),
        Err(AttackError::NotDue)
    );
    facts.defense = 0;
    facts.armor = 3;
    facts.revision = 2;
    let hit = owner
        .fixture_swing(
            &mut runtime,
            &mut states,
            &source,
            facts,
            2,
            revisions(),
            &root,
            clock(102_000_000),
        )
        .expect("next actual deadline");
    assert_eq!(hit.blocked, 1);
    assert_eq!(hit.damage.expect("lethal").health_after, 0);
    assert_eq!(
        owner.fixture_swing(
            &mut runtime,
            &mut states,
            &source,
            facts,
            1,
            revisions(),
            &root,
            clock(104_000_000)
        ),
        Err(AttackError::OccurrenceSuperseded)
    );
}
#[test]
fn protected_due_occurrence_is_retained_not_buffered() {
    let (mut runtime, mut states, source, mut facts, root) = setup("1st_mate_ratticus", false);
    facts.target_protected = true;
    let mut owner = owner(&runtime, &source, facts);
    assert_eq!(
        owner.fixture_swing(
            &mut runtime,
            &mut states,
            &source,
            facts,
            1,
            revisions(),
            &root,
            clock(2_000_000)
        ),
        Err(AttackError::Protected)
    );
    facts.target_protected = false;
    facts.revision = 2;
    assert_eq!(
        owner.fixture_swing(
            &mut runtime,
            &mut states,
            &source,
            facts,
            1,
            revisions(),
            &root,
            clock(3_000_000)
        ),
        Err(AttackError::OccurrenceConflict)
    );
    assert_eq!(
        owner.fixture_swing(
            &mut runtime,
            &mut states,
            &source,
            facts,
            2,
            revisions(),
            &root,
            clock(3_000_000)
        ),
        Err(AttackError::NotDue)
    );
    assert_eq!(
        owner
            .fixture_swing(
                &mut runtime,
                &mut states,
                &source,
                facts,
                2,
                revisions(),
                &root,
                clock(4_000_000)
            )
            .expect("fresh swing")
            .damage
            .expect("HP receipt")
            .health_after,
        0
    );
}
#[test]
fn independent_current_range_generation_and_content_guards() {
    let (mut runtime, mut states, mut source, facts, root) = setup("1st_mate_ratticus", true);
    let mut owner = owner(&runtime, &source, facts);
    assert_eq!(
        owner.fixture_swing(
            &mut runtime,
            &mut states,
            &source,
            facts,
            1,
            revisions(),
            &root,
            clock(2_000_000)
        ),
        Err(AttackError::OutOfRange)
    );
    source.content_digest = [0; 32];
    assert_eq!(
        owner.fixture_swing(
            &mut runtime,
            &mut states,
            &source,
            facts,
            2,
            revisions(),
            &root,
            clock(4_000_000)
        ),
        Err(AttackError::ContentChanged)
    );
    runtime
        .remove_test_actor(facts.issuer)
        .expect("despawn current actor");
    assert_eq!(
        owner.fixture_swing(
            &mut runtime,
            &mut states,
            &source,
            facts,
            2,
            revisions(),
            &root,
            clock(4_000_000)
        ),
        Err(AttackError::StaleIssuer)
    );
}
#[test]
fn donor_melee_attack_skill_formula_and_missing_native_ref_refusal() {
    let (runtime, _states, source, _facts, _root) = setup("abyssador", false);
    assert_eq!((source.minimum, source.maximum), (0, 2325));
    let (r, mut p) = native_data();
    let cref = Ref {
        family: ProjectV2Family::Creature,
        key: "oteryn:creature.abyssador".into(),
        revision: "definition-r1".into(),
    };
    p.retain(|p| p.target.family != ProjectV2Family::Formula);
    assert!(matches!(
        MeleeSource::from_native(
            &cref,
            0,
            &r,
            &p,
            runtime.content_pin().server_artifact_digest()
        ),
        Err(AttackError::InvalidSource)
    ));
}
#[test]
fn native_two_melee_entries_have_independent_sequences_and_occurrences() {
    let (mut runtime, mut states, source0, mut facts, root) = setup("an_astral_glyph", false);
    facts.defense = 10000;
    let mut owner = owner(&runtime, &source0, facts);
    let (r, p) = native_data();
    let cref = Ref {
        family: ProjectV2Family::Creature,
        key: "oteryn:creature.an_astral_glyph".into(),
        revision: "definition-r1".into(),
    };
    let source1 = MeleeSource::from_native(
        &cref,
        1,
        &r,
        &p,
        runtime.content_pin().server_artifact_digest(),
    )
    .expect("actual second melee");
    owner
        .set_target(
            &runtime,
            facts.issuer,
            facts.target,
            facts.session,
            &source1,
            "target-command:1",
            clock(0),
        )
        .expect("second native source entry");
    let first = owner
        .fixture_swing(
            &mut runtime,
            &mut states,
            &source0,
            facts,
            1,
            revisions(),
            &root,
            clock(2_000_000),
        )
        .expect("first entry");
    assert_eq!(owner.next_sequence(facts.issuer, 0), Some(2));
    assert_eq!(owner.next_sequence(facts.issuer, 1), Some(1));
    let second = owner
        .fixture_swing(
            &mut runtime,
            &mut states,
            &source1,
            facts,
            1,
            revisions(),
            &root,
            clock(2_000_000),
        )
        .expect("independent due entry");
    assert_ne!(first.occurrence, second.occurrence);
    assert!(first.damage.is_none());
    assert!(second.damage.is_none());
    assert_eq!(
        owner
            .fixture_swing(
                &mut runtime,
                &mut states,
                &source0,
                facts,
                1,
                revisions(),
                &root,
                clock(4_000_000)
            )
            .expect("first replay"),
        first
    );
    assert_eq!(
        owner
            .fixture_swing(
                &mut runtime,
                &mut states,
                &source1,
                facts,
                1,
                revisions(),
                &root,
                clock(4_000_000)
            )
            .expect("second replay"),
        second
    );
}
fn test_uuid(tag: u8) -> [u8; 16] {
    [
        0x01, 0x90, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag,
    ]
}
#[test]
fn shield_budget_65_session_turnovers_purge_stale_and_preserve_live_other_tokens() {
    use crate::foundation::{ChannelContentPin, ChannelId, NodeId, WorldId};
    let world = WorldId::decode(&test_uuid(0x60)).expect("world");
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&test_uuid(0x61)).expect("channel"),
        NodeId::decode(&test_uuid(0x62)).expect("node"),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        3,
        ChannelContentPin::test(world),
    )
    .expect("real three-slot runtime");
    let persistent_session = GameSessionId::decode(&test_uuid(0x71)).expect("session");
    let reservation = runtime
        .reserve_fresh_session(persistent_session)
        .expect("reserve");
    let persistent = runtime.commit_fresh_session(reservation).expect("commit");
    runtime
        .initialize_movement_test_position(
            persistent,
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
        )
        .expect("position");
    let (r, p) = native_data();
    let cref = Ref {
        family: ProjectV2Family::Creature,
        key: "oteryn:creature.1st_mate_ratticus".into(),
        revision: "definition-r1".into(),
    };
    let source = MeleeSource::from_native(
        &cref,
        0,
        &r,
        &p,
        runtime.content_pin().server_artifact_digest(),
    )
    .expect("native melee");
    let issuer = runtime
        .admit_monster_lab_creature(
            MovementLocalPosition {
                x: 101,
                y: 100,
                floor: 7,
            },
            &cref.key,
            200000,
        )
        .expect("actual creature source health");
    let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
    let mut states = ChannelSpellStates::default();
    let init_time = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0);
    states
        .initialize(
            &runtime,
            persistent,
            persistent_session,
            FACTS,
            (0, 0),
            init_time,
        )
        .expect("actual persistent vitals");
    let mut facts = AttackFacts {
        issuer,
        target: persistent,
        session: persistent_session,
        revision: 1,
        visible: true,
        issuer_pz: false,
        target_pz: false,
        issuer_protected: false,
        target_protected: false,
        defense: 10000,
        armor: 0,
    };
    let mut owner = owner(&runtime, &source, facts);
    owner
        .fixture_swing(
            &mut runtime,
            &mut states,
            &source,
            facts,
            1,
            revisions(),
            &root,
            clock(2_000_000),
        )
        .expect("actual first shield use");
    assert_eq!(u32::from(owner.blocks[0].budget.blocks()), 1);
    for n in 0..65u8 {
        let session = GameSessionId::decode(&test_uuid(0x80 + n)).expect("new session");
        let reservation = runtime
            .reserve_fresh_session(session)
            .expect("turnover reserve");
        let target = runtime
            .commit_fresh_session(reservation)
            .expect("turnover commit");
        runtime
            .initialize_movement_test_position(
                target,
                MovementLocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
            )
            .expect("current position");
        states
            .initialize(&runtime, target, session, FACTS, (0, 0), init_time)
            .expect("actual new vitals");
        facts.target = target;
        facts.session = session;
        facts.revision = u64::from(n) + 2;
        let now = (u64::from(n) + 1) * 2_000_000;
        owner
            .set_target(
                &runtime,
                issuer,
                target,
                session,
                &source,
                &format!("target-command:{}", n + 2),
                clock(now),
            )
            .expect("new current target");
        let sequence = owner.next_sequence(issuer, 0).expect("retained sequence");
        owner
            .fixture_swing(
                &mut runtime,
                &mut states,
                &source,
                facts,
                sequence,
                revisions(),
                &root,
                clock(now + 2_000_000),
            )
            .expect("65-turnover guard regression");
        assert!(owner.blocks.len() <= 2);
        assert_eq!(
            owner
                .blocks
                .iter()
                .find(|b| b.target == persistent && b.session == persistent_session)
                .expect("live other budget retained")
                .budget
                .blocks(),
            1
        );
        runtime
            .remove_terminal_session(session, target)
            .expect("current owner removes terminal actor");
    }
}

struct ActualFactsReader(AttackFacts);
impl CurrentCombatFactsReader for ActualFactsReader {
    fn read_attack(
        &mut self,
        runtime: &ChannelRuntimeV1,
        issuer: ExactActorRef,
        target: ExactActorRef,
        session: GameSessionId,
        _stamp: crate::foundation::RuntimeWorkStamp,
    ) -> Option<AttackFacts> {
        runtime.player_control_facts(target, session).ok()?;
        runtime.read_actor_position(issuer).ok()?;
        runtime.read_actor_position(target).ok()?;
        (issuer == self.0.issuer && target == self.0.target && session == self.0.session)
            .then_some(self.0)
    }
}
struct MissingFacts;
impl CurrentCombatFactsReader for MissingFacts {
    fn read_attack(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: ExactActorRef,
        _: GameSessionId,
        _: crate::foundation::RuntimeWorkStamp,
    ) -> Option<AttackFacts> {
        None
    }
}
fn current_fence(
    runtime: &ChannelRuntimeV1,
) -> (
    crate::foundation::ScopeRuntimeFence,
    crate::foundation::RuntimeWorkStamp,
) {
    let b = runtime.binding();
    crate::foundation::crystal_timer_fixture(
        crate::foundation::RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
        b.scope_generation(),
    )
    .expect("existing test-only native owner grant")
}
#[test]
fn typed_first_ai_sequence_zero_valid_replay_and_later_think_does_not_starve_melee() {
    let (mut runtime, mut states, source, facts, _root) = setup("1st_mate_ratticus", false);
    let mut owner = AutoAttackOwner::default();
    let (fence, stamp) = current_fence(&runtime);
    let mut tracker = crate::ai_think::ThinkSequenceTracker::new();
    let zero = tracker.next_occurrence(facts.issuer);
    assert_eq!(zero.sequence, 0);
    owner
        .accept_ai_target(
            &runtime,
            &fence,
            stamp,
            zero,
            facts.target,
            facts.session,
            std::slice::from_ref(&source),
            clock(0),
        )
        .expect("lawful first zero");
    owner
        .accept_ai_target(
            &runtime,
            &fence,
            stamp,
            zero,
            facts.target,
            facts.session,
            std::slice::from_ref(&source),
            clock(500_000),
        )
        .expect("duplicate selection");
    let one = tracker.next_occurrence(facts.issuer);
    owner
        .accept_ai_target(
            &runtime,
            &fence,
            stamp,
            one,
            facts.target,
            facts.session,
            std::slice::from_ref(&source),
            clock(1_000_000),
        )
        .expect("same target later think");
    assert_eq!(
        owner.accept_ai_target(
            &runtime,
            &fence,
            stamp,
            zero,
            facts.target,
            facts.session,
            std::slice::from_ref(&source),
            clock(1_500_000)
        ),
        Err(AttackError::StaleAiSelection)
    );
    let got = owner
        .dispatch_due(
            &mut runtime,
            &fence,
            stamp,
            &mut states,
            &[(facts.issuer, source)],
            &mut ActualFactsReader(facts),
            revisions(),
            clock(2_000_000),
        )
        .expect("actual due dispatcher");
    assert_eq!(got.len(), 1);
    assert_eq!(
        got[0]
            .1
            .as_ref()
            .expect("native HP commit")
            .damage
            .expect("actual HP receipt")
            .health_after,
        0
    );
}
#[test]
fn missing_current_combat_facts_consumes_due_without_hp_and_stale_owner_refuses() {
    let (mut runtime, mut states, source, facts, _root) = setup("1st_mate_ratticus", false);
    let mut owner = AutoAttackOwner::default();
    let (mut fence, stamp) = current_fence(&runtime);
    let mut tracker = crate::ai_think::ThinkSequenceTracker::new();
    let occurrence = tracker.next_occurrence(facts.issuer);
    owner
        .accept_ai_target(
            &runtime,
            &fence,
            stamp,
            occurrence,
            facts.target,
            facts.session,
            std::slice::from_ref(&source),
            clock(0),
        )
        .expect("target");
    let sources = [(facts.issuer, source.clone())];
    let got = owner
        .dispatch_due(
            &mut runtime,
            &fence,
            stamp,
            &mut states,
            &sources,
            &mut MissingFacts,
            revisions(),
            clock(2_000_000),
        )
        .expect("failclosed due");
    assert_eq!(got[0].1, Err(AttackError::MissingCombatFacts));
    assert_eq!(owner.next_sequence(facts.issuer, 0), Some(2));
    assert!(
        owner
            .dispatch_due(
                &mut runtime,
                &fence,
                stamp,
                &mut states,
                &sources,
                &mut ActualFactsReader(facts),
                revisions(),
                clock(2_000_000)
            )
            .expect("no buffered damage")
            .is_empty()
    );
    fence
        .apply_external_grant(
            crate::foundation::ScopeOwnershipGeneration::new(2).expect("generation"),
        )
        .expect("actual fence handoff");
    assert!(matches!(
        owner.dispatch_due(
            &mut runtime,
            &fence,
            stamp,
            &mut states,
            &sources,
            &mut ActualFactsReader(facts),
            revisions(),
            clock(4_000_000)
        ),
        Err(AttackError::StaleOwner)
    ));
    // Read through a real one-point vital commit: prior refused work left all185HP intact.
    let probe = states
        .apply_attack_damage(
            &mut runtime,
            facts.target,
            facts.session,
            1,
            "independent-vitals-probe",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(4_000_000),
        )
        .expect("real vital store");
    assert_eq!(probe.health_after, 184);
}

#[test]
fn native_cat_zero_damage_is_legal_noop_without_vital_or_death_commit() {
    let (mut runtime, mut states, source, facts, root) = setup("cat", false);
    assert_eq!((source.minimum, source.maximum), (0, 0));
    let mut owner = owner(&runtime, &source, facts);
    let hit = owner
        .fixture_swing(
            &mut runtime,
            &mut states,
            &source,
            facts,
            1,
            revisions(),
            &root,
            clock(2_000_000),
        )
        .expect("source-native zero magnitude");
    assert_eq!(hit.requested, 0);
    assert!(hit.damage.is_none());
    assert!(hit.chance_passed);
    assert_eq!(
        owner
            .fixture_swing(
                &mut runtime,
                &mut states,
                &source,
                facts,
                1,
                revisions(),
                &root,
                clock(9_000_000)
            )
            .expect("noop replay"),
        hit
    );
    assert_eq!(
        states
            .apply_attack_damage(
                &mut runtime,
                facts.target,
                facts.session,
                1,
                "zero-melee-probe",
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(9_000_000)
            )
            .expect("real unchanged vital store")
            .health_after,
        184
    );
}
