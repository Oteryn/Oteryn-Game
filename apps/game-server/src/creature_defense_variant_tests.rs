#![allow(clippy::expect_used)]
use super::*;
use crate::ai_think::{
    ThinkSequenceTracker,
    profile_schedule::{
        MonsterSummonFacts, ObservedSummonCount, ProfileScheduleInput, ProfileScheduleState,
    },
};
use crate::foundation::{
    ChannelContentPin, ChannelId, MovementLocalPosition, NodeId, RuntimeScopeRefV1, WorldId,
};
use crate::gameplay_transport::actor_spell::{ChannelSpellStates, tests::FACTS};
fn native() -> (Vec<ProjectReferenceRecord>, Vec<ProjectV2AuthoringProfile>) {
    let v:serde_json::Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../docs/agents/evidence/monster-full-mechanics-20261004/lanes/conditions/remaining-source-families/defense4/native-fixture.json"))).expect("qualified fixture");
    (
        serde_json::from_value(v["records"].clone()).expect("qualified fixture"),
        serde_json::from_value(v["authoring_profiles"].clone()).expect("qualified fixture"),
    )
}
struct World {
    target: ExactActorRef,
    session: GameSessionId,
    missing: bool,
}
impl SpellWorldReader for World {
    fn current_players(
        &mut self,
        r: &ChannelRuntimeV1,
        _: RuntimeWorkStamp,
    ) -> Option<Vec<(ExactActorRef, GameSessionId)>> {
        r.player_control_facts(self.target, self.session).ok()?;
        Some(vec![(self.target, self.session)])
    }
    fn current_facing(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: RuntimeWorkStamp,
    ) -> Option<Facing> {
        None
    }
    fn tile_allowed(
        &mut self,
        r: &ChannelRuntimeV1,
        c: ExactActorRef,
        _: i32,
        _: i32,
        z: i16,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        let p = r.read_actor_position(c).ok()?;
        if self.missing || p.context() != r.pinned_movement_context() {
            return None;
        }
        Some(z == p.position().floor)
    }
    fn combat(
        &mut self,
        r: &ChannelRuntimeV1,
        c: ExactActorRef,
        t: ExactActorRef,
        s: GameSessionId,
        _: Option<&str>,
        _: RuntimeWorkStamp,
    ) -> Option<SpellCombatFacts> {
        if self.missing {
            return None;
        }
        r.player_control_facts(t, s).ok()?;
        r.read_actor_position(c).ok()?;
        Some(SpellCombatFacts {
            attack: AttackFacts {
                issuer: c,
                target: t,
                session: s,
                revision: 1,
                visible: true,
                issuer_pz: false,
                target_pz: false,
                issuer_protected: false,
                target_protected: false,
                defense: 0,
                armor: 0,
            },
            multiplier_ppm: 1_000_000,
            immune: false,
            condition_policy: Some(CurrentConditionPolicy {
                immunities: vec![],
                base_speed: 180,
            }),
        })
    }
}
fn setup(
    name: &str,
) -> (
    ChannelRuntimeV1,
    ChannelSpellStates,
    SpellSource,
    ProfileAbilityProposal,
    World,
    ScopeRuntimeFence,
    RuntimeWorkStamp,
) {
    let id = |t: u8| [1, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, t];
    let w = WorldId::decode(&id(1)).expect("qualified fixture");
    let mut r = ChannelRuntimeV1::from_committed_assignment(
        w,
        ChannelId::decode(&id(2)).expect("qualified fixture"),
        NodeId::decode(&id(3)).expect("qualified fixture"),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        8,
        ChannelContentPin::test(w),
    )
    .expect("qualified fixture");
    let session = GameSessionId::decode(&id(8)).expect("qualified fixture");
    let reservation = r.reserve_fresh_session(session).expect("qualified fixture");
    let target = r
        .commit_fresh_session(reservation)
        .expect("qualified fixture");
    r.initialize_first_entry_position(target)
        .expect("qualified fixture");
    let at = r
        .read_actor_position(target)
        .expect("qualified fixture")
        .position();
    let (records, profiles) = native();
    let creature = Ref {
        family: ProjectV2Family::Creature,
        key: format!("oteryn:creature.{name}"),
        revision: "definition-r1".into(),
    };
    let health = profiles
        .iter()
        .find_map(|p| match &p.data {
            Data::Creature(c) if p.target == creature => c.health,
            _ => None,
        })
        .expect("qualified fixture");
    let actor = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: at.x + 1,
                y: at.y,
                floor: at.floor,
            },
            &creature.key,
            health as i64,
        )
        .expect("qualified fixture");
    let source = SpellSource::from_native_defense(&creature, 1, &records, &profiles, [1; 32])
        .expect("qualified fixture");
    let behavior = profiles
        .iter()
        .find_map(|p| match &p.data {
            Data::Behavior(b) if p.target.key == format!("oteryn:behavior.creature.{name}") => {
                Some(b)
            }
            _ => None,
        })
        .expect("qualified fixture");
    let abilities = profiles
        .iter()
        .filter_map(|p| match &p.data {
            Data::Ability(a) => Some((p.target.clone(), a.clone())),
            _ => None,
        })
        .collect();
    let root = GameplayDecisionRoot::from_bytes([1; 32]);
    let revisions = crate::ability::RevisionSet::new(
        "rules-r1",
        "content-r1",
        "policy-r1",
        "definition-r1",
        "sim-r1",
    )
    .expect("qualified fixture");
    let mut schedule = ProfileScheduleState::new(actor);
    let mut tracker = ThinkSequenceTracker::new();
    let mut found = None;
    for _ in 0..256 {
        let occurrence = tracker.next_occurrence(actor);
        let summons = behavior.summons.as_ref().map(|entries| MonsterSummonFacts {
            occurrence,
            is_summon: r
                .native_summon_role(actor)
                .expect("qualified fixture")
                .is_some(),
            target_with_path: None,
            total_count: r.native_summon_count(actor, None),
            entry_counts: entries
                .entries
                .iter()
                .map(|e| ObservedSummonCount {
                    creature: e.creature.clone(),
                    count: r.native_summon_count(actor, Some(&e.creature.key)),
                })
                .collect(),
        });
        let plan = schedule
            .prepare_with_summons(
                ProfileScheduleInput {
                    occurrence,
                    behavior,
                    abilities: &abilities,
                    target: None,
                    revisions: &revisions,
                    root: &root,
                },
                summons.as_ref(),
            )
            .expect("native defense schedule with independently current carrier summon counts");
        found = plan
            .proposals
            .into_iter()
            .find(|p| p.list == ScheduleList::Defence && p.ability == source.ability);
        if found.is_some() {
            break;
        }
    }
    let p = found.expect("sourcechance prepares genuine Defense proposal");
    assert_eq!(p.target, actor);
    let mut states = ChannelSpellStates::default();
    states
        .initialize(
            &r,
            target,
            session,
            crate::spell::cast::CharacterCastFacts {
                magic_level: 53,
                ..FACTS
            },
            (0, 0),
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
        )
        .expect("qualified fixture");
    let b = r.binding();
    let (fence, stamp) = crate::foundation::crystal_timer_fixture(
        RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
        b.scope_generation(),
    )
    .expect("qualified fixture");
    (
        r,
        states,
        source,
        p,
        World {
            target,
            session,
            missing: false,
        },
        fence,
        stamp,
    )
}
#[test]
fn both_native_defense_variants_execute_real_attribute_owner_shared_oncecast_and_replay() {
    for name in ["the_sinister_hermit", "the_sinister_hermit_dirty"] {
        let (mut r, mut states, s, p, mut world, fence, stamp) = setup(name);
        let mut owner = DamageSpellOwner::default();
        let before = states
            .source_player_health(&r, world.target, world.session)
            .expect("qualified fixture");
        let hit = owner
            .execute(
                &mut r,
                &fence,
                stamp,
                &mut states,
                &s,
                &p,
                &mut world,
                SemanticTimeMicros::from_micros(1_000_000),
            )
            .expect("qualified fixture");
        assert!(
            hit.targets
                .iter()
                .any(|(t, r)| *t == world.target
                    && r.as_ref().is_ok_and(|r| r.condition_only_applied))
        );
        let variant = hit.source_variant.as_ref().expect("qualified fixture");
        assert!(variant.index < 31);
        let percent = 20 + variant.index as u32;
        let expected = 53 - (53 * (100 - percent) / 100);
        assert_eq!(
            states
                .native_effective_attributes(
                    &r,
                    world.target,
                    world.session,
                    oteryn_simulation_determinism::SemanticTimeMicros::from_micros(1_000_000)
                )
                .expect("qualified fixture")
                .magic_level(53),
            Some(expected)
        );
        assert_eq!(
            states
                .source_player_health(&r, world.target, world.session)
                .expect("qualified fixture"),
            (before.0, before.1, before.2 + 1)
        );
        let committed = states
            .read_owned_player_state_test_snapshot(&r, world.target, world.session)
            .expect("qualified fixture")
            .clone();
        assert_eq!(
            owner
                .execute(
                    &mut r,
                    &fence,
                    stamp,
                    &mut states,
                    &s,
                    &p,
                    &mut world,
                    SemanticTimeMicros::from_micros(2_000_000)
                )
                .expect("qualified fixture"),
            hit
        );
        assert_eq!(
            states
                .native_effective_attributes(
                    &r,
                    world.target,
                    world.session,
                    oteryn_simulation_determinism::SemanticTimeMicros::from_micros(7_000_000)
                )
                .expect("qualified fixture")
                .magic_level(53),
            Some(53)
        );
        assert_eq!(
            states
                .read_owned_player_state_test_snapshot(&r, world.target, world.session)
                .expect("qualified fixture"),
            &committed
        );
        assert_eq!(owner.casts[0].list, ScheduleList::Defence);
    }
}
#[test]
fn defense_source_rejects_attack_occurrence_child_substitution_and_missing_current_facts() {
    let (mut r, mut states, s, p, mut world, fence, stamp) = setup("the_sinister_hermit");
    let mut owner = DamageSpellOwner::default();
    let mut attack = p.clone();
    attack.list = ScheduleList::Attack;
    assert_eq!(
        owner.execute(
            &mut r,
            &fence,
            stamp,
            &mut states,
            &s,
            &attack,
            &mut world,
            SemanticTimeMicros::from_micros(1)
        ),
        Err(AttackError::InvalidSource)
    );
    assert!(owner.casts.is_empty());
    let mut suffix = p.clone();
    suffix.occurrence = crate::ability::AbilityOccurrence::new(
        &format!(
            "ai-profile:{}:0:attack:1",
            hex(&p.issuer.placement_identity())
        ),
        crate::ability::RevisionSet::new(
            "rules-r1",
            "content-r1",
            "policy-r1",
            "definition-r1",
            "sim-r1",
        )
        .expect("qualified fixture"),
    )
    .expect("qualified fixture");
    assert_eq!(
        owner.execute(
            &mut r,
            &fence,
            stamp,
            &mut states,
            &s,
            &suffix,
            &mut world,
            SemanticTimeMicros::from_micros(1)
        ),
        Err(AttackError::InvalidPlan)
    );
    let (records, mut profiles) = native();
    let child = profiles
        .iter_mut()
        .find(|p| p.target.key == "oteryn:ability.spell.shock_head_skill_reducer_2.variant-1")
        .expect("qualified fixture");
    if let Data::Ability(a) = &mut child.data {
        a.details.as_mut().expect("qualified fixture").needs_target = true
    };
    let creature = Ref {
        family: ProjectV2Family::Creature,
        key: "oteryn:creature.the_sinister_hermit".into(),
        revision: "definition-r1".into(),
    };
    assert!(matches!(
        SpellSource::from_native_defense(&creature, 1, &records, &profiles, [1; 32]),
        Err(AttackError::InvalidSource)
    ));
    world.missing = true;
    assert_eq!(
        owner.execute(
            &mut r,
            &fence,
            stamp,
            &mut states,
            &s,
            &p,
            &mut world,
            SemanticTimeMicros::from_micros(1)
        ),
        Err(AttackError::MissingCombatFacts)
    );
    assert_eq!(
        states
            .source_player_health(&r, world.target, world.session)
            .expect("qualified fixture")
            .0,
        185
    );
    assert_eq!(
        states
            .native_effective_attributes(
                &r,
                world.target,
                world.session,
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(1)
            )
            .expect("qualified fixture")
            .magic_level(53),
        Some(53)
    );
}
