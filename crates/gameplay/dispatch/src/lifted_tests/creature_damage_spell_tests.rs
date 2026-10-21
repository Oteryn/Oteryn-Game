#![allow(clippy::expect_used)]
use crate::ai_think::{
    ThinkSequenceTracker,
    profile_schedule::{AttackTarget, ProfileScheduleState},
};
use crate::content::{
    ProjectReferenceRecord, ProjectV2AbilityEffect, ProjectV2AuthoringProfile,
    ProjectV2AuthoringProfileData as Data, ProjectV2DefinitionRef as Ref, ProjectV2Family,
};
use crate::creature_auto_attack::ProfileAbilityProposal;
use crate::creature_auto_attack::{AttackError, AttackFacts, CurrentConditionPolicy};
use crate::creature_damage_spell::*;
use crate::foundation::MovementLocalPosition;
use crate::foundation::owner_timer::SemanticTimeMicros;
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, RuntimeWorkStamp, ScopeRuntimeFence,
};
use crate::gameplay_transport::actor_spell::{ChannelSpellStates, tests::FACTS};
use crate::player_lethal::PlayerLethalVitals;
use oteryn_gameplay_content::content::creature_attack_geometry::Facing;
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
struct CurrentWorld {
    facts: SpellCombatFacts,
    missing: bool,
    blocked: bool,
    facing: Option<Facing>,
    extra: Vec<(ExactActorRef, GameSessionId, u32)>,
}
impl SpellWorldReader for CurrentWorld {
    fn current_players(
        &mut self,
        r: &ChannelRuntimeV1,
        _: RuntimeWorkStamp,
    ) -> Option<Vec<(ExactActorRef, GameSessionId)>> {
        r.player_control_facts(self.facts.attack.target, self.facts.attack.session)
            .ok()?;
        let mut result = vec![(self.facts.attack.target, self.facts.attack.session)];
        for (a, s, _) in &self.extra {
            r.player_control_facts(*a, *s).ok()?;
            result.push((*a, *s));
        }
        Some(result)
    }
    fn current_facing(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: RuntimeWorkStamp,
    ) -> Option<Facing> {
        self.facing
    }
    fn tile_allowed(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: i32,
        _: i32,
        _: i16,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        (!self.missing).then_some(!self.blocked)
    }
    fn combat(
        &mut self,
        r: &ChannelRuntimeV1,
        i: ExactActorRef,
        t: ExactActorRef,
        s: GameSessionId,
        _: Option<&str>,
        _: RuntimeWorkStamp,
    ) -> Option<SpellCombatFacts> {
        r.read_actor_position(i).ok()?;
        r.player_control_facts(t, s).ok()?;
        if self.missing || self.facts.attack.issuer != i {
            return None;
        }
        let mut facts = self.facts.clone();
        if self.facts.attack.target == t && self.facts.attack.session == s {
            return Some(facts);
        }
        let (_, _, m) = self
            .extra
            .iter()
            .find(|(a, session, _)| *a == t && *session == s)?;
        facts.attack.target = t;
        facts.attack.session = s;
        facts.multiplier_ppm = *m;
        Some(facts)
    }
}

fn test_uuid(tag: u8) -> [u8; 16] {
    [
        0x01, 0x90, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag,
    ]
}
fn runtime_three_slots() -> (ChannelRuntimeV1, ExactActorRef, GameSessionId) {
    use {
        crate::foundation::{ChannelId, NodeId, WorldId},
        oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin,
    };
    let w = WorldId::decode(&test_uuid(0x60)).expect("qualified fixture");
    let mut r = ChannelRuntimeV1::from_committed_assignment(
        w,
        ChannelId::decode(&test_uuid(0x61)).expect("qualified fixture"),
        NodeId::decode(&test_uuid(0x62)).expect("qualified fixture"),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        3,
        ChannelContentPin::test(w),
    )
    .expect("qualified fixture");
    let s = GameSessionId::decode(&test_uuid(0x71)).expect("qualified fixture");
    let reservation = r.reserve_fresh_session(s).expect("qualified fixture");
    let a = r
        .commit_fresh_session(reservation)
        .expect("qualified fixture");
    (r, a, s)
}
fn native() -> (Vec<ProjectReferenceRecord>, Vec<ProjectV2AuthoringProfile>) {
    let v: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../apps/game-server/src/creature_auto_attack_test_data.json"
    )))
    .expect("qualified fixture");
    (
        serde_json::from_value(v["records"].clone()).expect("qualified fixture"),
        serde_json::from_value(v["authoring_profiles"].clone()).expect("qualified fixture"),
    )
}
fn setup() -> (
    ChannelRuntimeV1,
    ChannelSpellStates,
    SpellSource,
    ProfileAbilityProposal,
    CurrentWorld,
    ScopeRuntimeFence,
    RuntimeWorkStamp,
) {
    setup_named("1st_mate_ratticus", 1)
}
fn setup_named(
    name: &str,
    index: usize,
) -> (
    ChannelRuntimeV1,
    ChannelSpellStates,
    SpellSource,
    ProfileAbilityProposal,
    CurrentWorld,
    ScopeRuntimeFence,
    RuntimeWorkStamp,
) {
    let (mut r, t, s) = runtime_three_slots();
    r.initialize_source_pinned_lab_player_position(
        t,
        s,
        MovementLocalPosition {
            x: 100,
            y: 100,
            floor: 7,
        },
    )
    .expect("qualified fixture");
    let (records, profiles) = if name == "bony_sea_devil" {
        let packet = native_windup_packet();
        (
            serde_json::from_value(packet["records"].clone()).expect("qualified fixture"),
            serde_json::from_value(packet["authoring_profiles"].clone())
                .expect("qualified fixture"),
        )
    } else if matches!(
        name,
        "barbarian_brutetamer" | "demon_outcast" | "werewolf" | "warlock" | "undead_dragon"
    ) {
        let packet = native_variant_packet();
        (
            serde_json::from_value(packet["records"].clone()).expect("qualified fixture"),
            serde_json::from_value(packet["authoring_profiles"].clone())
                .expect("qualified fixture"),
        )
    } else if matches!(name, "blue_djinn" | "green_djinn") {
        let packet:serde_json::Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../docs/agents/evidence/monster-full-mechanics-20261004/lanes/conditions/remaining-source-families/dispel/native-fixture.json"))).expect("qualified fixture");
        (
            serde_json::from_value(packet["records"].clone()).expect("qualified fixture"),
            serde_json::from_value(packet["authoring_profiles"].clone())
                .expect("qualified fixture"),
        )
    } else {
        native()
    };
    let cref = Ref {
        family: ProjectV2Family::Creature,
        key: format!("oteryn:creature.{name}"),
        revision: "definition-r1".into(),
    };
    let health = profiles
        .iter()
        .find_map(|p| match &p.data {
            Data::Creature(c) if p.target == cref => c.health,
            _ => None,
        })
        .expect("qualified fixture");
    let issuer = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: 101,
                y: 100,
                floor: 7,
            },
            &cref.key,
            health as i64,
        )
        .expect("qualified fixture");
    let source = SpellSource::from_native(
        &cref,
        index,
        &records,
        &profiles,
        r.content_pin().server_artifact_digest(),
    )
    .expect("qualified fixture");
    let abilities = profiles
        .iter()
        .filter_map(|p| match &p.data {
            Data::Ability(a) => Some((p.target.clone(), a.clone())),
            _ => None,
        })
        .collect();
    let behavior = profiles
        .iter()
        .find_map(|p| match &p.data {
            Data::Behavior(b) if p.target.key == format!("oteryn:behavior.creature.{name}") => {
                Some(b)
            }
            _ => None,
        })
        .expect("qualified fixture");
    let root = GameplayDecisionRoot::from_bytes(r.content_pin().server_artifact_digest());
    let revisions = crate::ability::RevisionSet::new(
        "rules-r1",
        "content-r1",
        "policy-r1",
        "definition-r1",
        "sim-r1",
    )
    .expect("qualified fixture");
    let mut schedule = ProfileScheduleState::new(issuer);
    let mut tracker = ThinkSequenceTracker::new();
    let mut found = None;
    for _ in 0..256 {
        let occurrence = tracker.next_occurrence(issuer);
        let summon_facts = behavior.summons.as_ref().map(|summons| {
            crate::ai_think::profile_schedule::MonsterSummonFacts {
                occurrence,
                is_summon: r
                    .native_summon_role(issuer)
                    .expect("qualified fixture")
                    .is_some(),
                target_with_path: None,
                total_count: r.native_summon_count(issuer, None),
                entry_counts: summons
                    .entries
                    .iter()
                    .map(
                        |entry| crate::ai_think::profile_schedule::ObservedSummonCount {
                            creature: entry.creature.clone(),
                            count: r.native_summon_count(issuer, Some(&entry.creature.key)),
                        },
                    )
                    .collect(),
            }
        });
        let plan = schedule
            .prepare_with_summons(
                crate::ai_think::profile_schedule::ProfileScheduleInput {
                    occurrence,
                    behavior,
                    abilities: &abilities,
                    target: Some(AttackTarget {
                        actor: t,
                        same_floor_distance: Some(1),
                    }),
                    revisions: &revisions,
                    root: &root,
                },
                summon_facts.as_ref(),
            )
            .expect("native source schedule with independently current carrier summon counts");
        if let Some(p) = plan
            .proposals
            .into_iter()
            .find(|p| p.ability == source.ability)
        {
            found = Some(p);
            break;
        }
    }
    let proposal = found.expect("actual source chance/timing eventually prepares native spell");
    let mut states = ChannelSpellStates::default();
    states
        .initialize(
            &r,
            t,
            s,
            FACTS,
            (0, 0),
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
        )
        .expect("qualified fixture");
    let facts = SpellCombatFacts {
        attack: AttackFacts {
            issuer,
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
        condition_policy: None,
    };
    let b = r.binding();
    let (fence, stamp) = crate::foundation::crystal_timer_fixture(
        crate::foundation::RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
        b.scope_generation(),
    )
    .expect("qualified fixture");
    (
        r,
        states,
        source,
        proposal,
        CurrentWorld {
            facts,
            missing: false,
            blocked: false,
            facing: None,
            extra: vec![],
        },
        fence,
        stamp,
    )
}

#[test]
fn composed_readonly_preflight_defers_actual_hp_and_rejects_missing_policy() {
    let (mut r, mut states, source, p, mut world, _, stamp) = setup();
    let owner = DamageSpellOwner::default();
    let prepared = owner
        .prepare_target(&r, &source, &p, &mut world, stamp, None)
        .expect("qualified fixture");
    assert!(!prepared.targets.is_empty());
    // A real one-point owner probe proves no primary HP was committed during preparation.
    assert_eq!(
        states
            .apply_attack_damage(
                &mut r,
                p.target,
                world.facts.attack.session,
                1,
                "preflight-health-probe",
                crate::foundation::owner_timer::OwnerClock::now(
                    &crate::foundation::owner_timer::VirtualOwnerClock::new(
                        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
                    )
                )
            )
            .expect("qualified fixture")
            .health_after,
        184
    );
    world.missing = true;
    assert!(matches!(
        owner.prepare_target(&r, &source, &p, &mut world, stamp, None),
        Err(AttackError::MissingCombatFacts)
    ));
    assert_eq!(
        states
            .apply_attack_damage(
                &mut r,
                p.target,
                world.facts.attack.session,
                1,
                "preflight-second-probe",
                crate::foundation::owner_timer::OwnerClock::now(
                    &crate::foundation::owner_timer::VirtualOwnerClock::new(
                        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
                    )
                )
            )
            .expect("qualified fixture")
            .health_after,
        183
    );
}
fn native_windup_packet() -> serde_json::Value {
    serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../docs/agents/evidence/monster-full-mechanics-20261004/lanes/conditions/remaining-source-families/windup/native-fixture.json"))).expect("qualified fixture")
}
fn windup_setup() -> (
    ChannelRuntimeV1,
    ChannelSpellStates,
    SpellSource,
    ProfileAbilityProposal,
    CurrentWorld,
    ScopeRuntimeFence,
    RuntimeWorkStamp,
) {
    let packet = native_windup_packet();
    let index = packet["cases"]
        .as_array()
        .expect("qualified fixture")
        .iter()
        .find(|c| c["creature"]["key"] == "oteryn:creature.bony_sea_devil")
        .expect("qualified fixture")["entry"]
        .as_u64()
        .expect("qualified fixture") as usize;
    let mut setup = setup_named("bony_sea_devil", index);
    setup.4.facts.condition_policy = Some(CurrentConditionPolicy {
        immunities: vec![],
        base_speed: 180,
    });
    setup
}
#[test]
fn native_all6_delayed_fear_sources_lower_without_immediate_condition() {
    let packet = native_windup_packet();
    let records: Vec<ProjectReferenceRecord> =
        serde_json::from_value(packet["records"].clone()).expect("qualified fixture");
    let profiles: Vec<ProjectV2AuthoringProfile> =
        serde_json::from_value(packet["authoring_profiles"].clone()).expect("qualified fixture");
    for case in packet["cases"].as_array().expect("qualified fixture") {
        let creature: Ref =
            serde_json::from_value(case["creature"].clone()).expect("qualified fixture");
        let source = SpellSource::from_native(
            &creature,
            case["entry"].as_u64().expect("qualified fixture") as usize,
            &records,
            &profiles,
            [1; 32],
        )
        .expect("qualified fixture");
        assert_eq!(
            source
                .windup
                .as_ref()
                .expect("qualified fixture")
                .windup
                .as_ref()
                .expect("qualified fixture")
                .delay_ms,
            2000
        );
        assert_eq!(source.conditions.len(), 1);
    }
}
#[test]
fn native_delayed_fear_exact2000ms_real_store_scope_and_single_due_replay() {
    let (mut r, mut v, s, p, mut w, mut f, stamp) = windup_setup();
    let session = w.facts.attack.session;
    let before = v
        .read_owned_player_state_test_snapshot(&r, p.target, session)
        .expect("qualified fixture")
        .clone();
    let mut owner = DamageSpellOwner::default();
    let now = SemanticTimeMicros::from_micros(2_000_000);
    let start = owner
        .execute(&mut r, &f, stamp, &mut v, &s, &p, &mut w, now)
        .expect("qualified fixture");
    assert_eq!(
        start
            .delayed_cast
            .as_ref()
            .expect("qualified fixture")
            .due
            .get(),
        4_000_000
    );
    assert!(start.targets.is_empty());
    assert!(start.presentation.is_some());
    assert!(v.native_condition_movement_allowed(
        &r,
        p.target,
        w.facts.attack.session,
        oteryn_simulation_determinism::SemanticTimeMicros::from_micros(2_000_000)
    ));
    assert_eq!(
        owner
            .execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(3_000_000)
            )
            .expect("qualified fixture"),
        start
    );
    assert_eq!(
        owner
            .delayed
            .as_ref()
            .expect("qualified fixture")
            .pending_len(),
        1
    );
    let clock = crate::foundation::owner_timer::VirtualOwnerClock::new(
        SemanticTimeMicros::from_micros(3_999_999),
    );
    assert!(
        owner
            .run_windup_due(&mut r, &mut f, &mut v, &mut w, &clock)
            .expect("qualified fixture")
            .is_empty()
    );
    assert!(v.native_condition_movement_allowed(
        &r,
        p.target,
        w.facts.attack.session,
        oteryn_simulation_determinism::SemanticTimeMicros::from_micros(3_999_999)
    ));
    clock.advance(1);
    let fired = owner
        .run_windup_due(&mut r, &mut f, &mut v, &mut w, &clock)
        .expect("qualified fixture");
    assert_eq!(fired.len(), 1);
    let hit = fired[0].1.as_ref().expect("qualified fixture");
    assert!(hit.delayed_cast.is_none());
    assert!(
        hit.targets[0]
            .1
            .as_ref()
            .expect("qualified fixture")
            .condition_only_applied
    );
    assert!(!v.native_condition_movement_allowed(
        &r,
        p.target,
        w.facts.attack.session,
        oteryn_simulation_determinism::SemanticTimeMicros::from_micros(4_000_000)
    ));
    let committed = v
        .read_owned_player_state_test_snapshot(&r, p.target, session)
        .expect("qualified fixture")
        .clone();
    assert_eq!(committed.vitals(), before.vitals());
    assert_eq!(committed.revision(), before.revision() + 1);
    assert!(
        owner
            .run_windup_due(&mut r, &mut f, &mut v, &mut w, &clock)
            .expect("qualified fixture")
            .is_empty()
    );
    assert_eq!(
        owner
            .delayed
            .as_ref()
            .expect("qualified fixture")
            .pending_len(),
        0
    );
    assert!(v.native_condition_movement_allowed(
        &r,
        p.target,
        w.facts.attack.session,
        oteryn_simulation_determinism::SemanticTimeMicros::from_micros(7_000_000)
    ));
    assert_eq!(
        v.read_owned_player_state_test_snapshot(&r, p.target, session)
            .expect("qualified fixture"),
        &committed
    );
}
#[test]
fn native_delayed_fear_current_pz_missing_facts_and_source_death_never_install() {
    for mode in 0..3 {
        let (mut r, mut v, s, p, mut w, mut f, stamp) = windup_setup();
        let mut owner = DamageSpellOwner::default();
        owner
            .execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(2_000_000),
            )
            .expect("qualified fixture");
        match mode {
            0 => w.facts.attack.target_pz = true,
            1 => w.missing = true,
            _ => {
                r.remove_test_actor(p.issuer).expect("qualified fixture");
            }
        }
        let clock = crate::foundation::owner_timer::VirtualOwnerClock::new(
            SemanticTimeMicros::from_micros(4_000_000),
        );
        let fired = owner
            .run_windup_due(&mut r, &mut f, &mut v, &mut w, &clock)
            .expect("qualified fixture");
        assert_eq!(fired.len(), 1);
        match mode {
            0 => assert!(
                fired[0]
                    .1
                    .as_ref()
                    .expect("qualified fixture")
                    .targets
                    .is_empty()
            ),
            1 => assert_eq!(fired[0].1, Err(AttackError::MissingCombatFacts)),
            _ => assert_eq!(fired[0].1, Err(AttackError::StaleIssuer)),
        };
        assert!(v.native_condition_movement_allowed(
            &r,
            p.target,
            w.facts.attack.session,
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(4_000_000)
        ));
        assert_eq!(
            owner
                .delayed
                .as_ref()
                .expect("qualified fixture")
                .pending_len(),
            0
        );
        assert!(
            owner
                .run_windup_due(&mut r, &mut f, &mut v, &mut w, &clock)
                .expect("qualified fixture")
                .is_empty()
        );
    }
}
#[test]
fn native_delayed_fear_overflow_pending_cap_and_retired_scope_are_explicit() {
    let (mut r, mut v, s, p, mut w, mut f, stamp) = windup_setup();
    let mut owner = DamageSpellOwner::default();
    assert_eq!(
        owner.execute(
            &mut r,
            &f,
            stamp,
            &mut v,
            &s,
            &p,
            &mut w,
            SemanticTimeMicros::from_micros(u64::MAX - 1)
        ),
        Err(AttackError::NumericOverflow)
    );
    assert!(owner.delayed.is_none());
    let mut owner = DamageSpellOwner::default();
    owner
        .execute(
            &mut r,
            &f,
            stamp,
            &mut v,
            &s,
            &p,
            &mut w,
            SemanticTimeMicros::from_micros(2_000_000),
        )
        .expect("qualified fixture");
    let queued = owner
        .delayed
        .as_ref()
        .expect("qualified fixture")
        .pending_len();
    // Adversarial owner admission probe of the private cap; no fabricated think or wire grant.
    assert_eq!(
        owner.delayed.as_mut().expect("qualified fixture").schedule(
            &r,
            &f,
            stamp,
            &s,
            &p,
            w.facts.attack.session,
            SemanticTimeMicros::from_micros(3_000_000),
            SemanticTimeMicros::from_micros(5_000_000)
        ),
        Err(AttackError::LedgerFull)
    );
    assert_eq!(
        owner
            .delayed
            .as_ref()
            .expect("qualified fixture")
            .pending_len(),
        queued
    );
    f.apply_external_grant(
        crate::foundation::ScopeOwnershipGeneration::new(2).expect("qualified fixture"),
    )
    .expect("qualified fixture");
    let clock = crate::foundation::owner_timer::VirtualOwnerClock::new(
        SemanticTimeMicros::from_micros(4_000_000),
    );
    assert!(matches!(
        owner.run_windup_due(&mut r, &mut f, &mut v, &mut w, &clock),
        Err(AttackError::StaleOwner)
    ));
    assert_eq!(
        owner
            .delayed
            .as_ref()
            .expect("qualified fixture")
            .pending_len(),
        queued
    );
    assert!(v.native_condition_movement_allowed(
        &r,
        p.target,
        w.facts.attack.session,
        oteryn_simulation_determinism::SemanticTimeMicros::from_micros(4_000_000)
    ));
}

fn native_variant_packet() -> serde_json::Value {
    serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../docs/agents/evidence/monster-full-mechanics-20261004/lanes/conditions/remaining-source-families/variants/native-fixture.json"))).expect("qualified fixture")
}
#[test]
fn native_all42_parent_variants_have_exact_child_membership_without_creature_aliases() {
    let packet = native_variant_packet();
    let records: Vec<ProjectReferenceRecord> =
        serde_json::from_value(packet["records"].clone()).expect("qualified fixture");
    let profiles: Vec<ProjectV2AuthoringProfile> =
        serde_json::from_value(packet["authoring_profiles"].clone()).expect("qualified fixture");
    for case in packet["cases"].as_array().expect("qualified fixture") {
        let creature: Ref =
            serde_json::from_value(case["creature"].clone()).expect("qualified fixture");
        let source = SpellSource::from_native(
            &creature,
            case["entry"].as_u64().expect("qualified fixture") as usize,
            &records,
            &profiles,
            [1; 32],
        )
        .expect("qualified fixture");
        assert_eq!(
            source.variants.len(),
            case["children"]
                .as_array()
                .expect("qualified fixture")
                .len()
        );
        for (index, branch) in source.variants.iter().enumerate() {
            match branch {
                Ok(b) => {
                    assert_eq!(b.ability, source.ability);
                    assert_eq!(
                        b.variant_child.as_ref().expect("qualified fixture").key,
                        format!("{}.variant-{}", source.ability.key, index + 1)
                    );
                }
                Err(e) => assert_eq!(*e, AttackError::UnsupportedShape),
            }
        }
    }
}
#[test]
fn native_skill_variant_once_per_parent_cast_real_owner_and_replay_no_fake_damage() {
    let packet = native_variant_packet();
    let cases = packet["cases"].as_array().expect("qualified fixture");
    for name in [
        "barbarian_brutetamer",
        "demon_outcast",
        "werewolf",
        "warlock",
    ] {
        let case = cases
            .iter()
            .find(|c| c["creature"]["key"] == format!("oteryn:creature.{name}"))
            .expect("qualified fixture");
        let index = case["entry"].as_u64().expect("qualified fixture") as usize;
        let (mut r, mut v, s, p, mut w, f, stamp) = setup_named(name, index);
        w.facing = Some(Facing::West);
        w.facts.condition_policy = Some(CurrentConditionPolicy {
            immunities: vec![],
            base_speed: 180,
        });
        let mut owner = DamageSpellOwner::default();
        let hit = owner
            .execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(2_000_000),
            )
            .expect("qualified fixture");
        let selected = hit.source_variant.as_ref().expect("qualified fixture");
        assert!(selected.index < s.variants.len());
        assert_eq!(selected.parent, p.ability);
        assert_eq!(
            selected.child.key,
            format!("{}.variant-{}", p.ability.key, selected.index + 1)
        );
        assert_eq!(hit.targets.len(), 1);
        assert!(
            hit.targets[0]
                .1
                .as_ref()
                .expect("qualified fixture")
                .condition_only_applied
        );
        assert!(
            hit.targets[0]
                .1
                .as_ref()
                .expect("qualified fixture")
                .damage
                .is_none()
        );
        assert_eq!(
            owner
                .execute(
                    &mut r,
                    &f,
                    stamp,
                    &mut v,
                    &s,
                    &p,
                    &mut w,
                    SemanticTimeMicros::from_micros(9_000_000)
                )
                .expect("qualified fixture"),
            hit
        );
        assert_eq!(
            v.apply_attack_damage(
                &mut r,
                p.target,
                w.facts.attack.session,
                1,
                "variant-no-hp-probe",
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(9_000_000)
            )
            .expect("qualified fixture")
            .health_after,
            184
        );
    }
}

fn move_existing_player_through_native_owner_to_curse_cone(
    r: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
) {
    use crate::content::static_cell_engine::{
        EngineeringCollisionClaim, EngineeringStaticCellClaim, EngineeringStaticCellIndex,
        EngineeringStaticCellScope,
    };
    use crate::content::{
        CollisionClass, ContentLockBinding, ContentLockEntry, CoordinateFrameRef, LogicalCell,
        MapRevisionRef, ProductionAtom, ProductionKey, Sha256HexDigest,
    };
    use crate::movement::{
        CardinalStep, MovementEngineeringSelection, MovementOwnerTurn, MovementTurnOutcome,
    };
    let scope = EngineeringStaticCellScope {
        world_id: r
            .read_actor_position(actor)
            .expect("qualified fixture")
            .world_id(),
        coordinate_frame: CoordinateFrameRef::new("movement-engineering-frame")
            .expect("qualified fixture"),
        map_revision: MapRevisionRef::new("movement-engineering-map").expect("qualified fixture"),
        generation_digest: [7; 32],
        content_lock: ContentLockBinding {
            revision_digest_token: ProductionAtom::new("lock", "movement-engineering-lock")
                .expect("qualified fixture"),
            entries: vec![ContentLockEntry::exact(
                ProductionKey::new("engineering:variant-curse").expect("qualified fixture"),
                ProductionAtom::new("revision", "variant-curse-r1").expect("qualified fixture"),
                Sha256HexDigest::new(&"a".repeat(64)).expect("qualified fixture"),
            )],
        },
    };
    // Explicit local map claims, not a live/Global map authority. Every relocation below
    // commits through the actual existing four-cardinal movement owner and current carrier.
    for direction in [
        CardinalStep::North,
        CardinalStep::North,
        CardinalStep::North,
        CardinalStep::North,
        CardinalStep::North,
        CardinalStep::East,
    ] {
        let expected = r.read_actor_position(actor).expect("qualified fixture");
        let selected = MovementEngineeringSelection {
            owner_context: expected.context(),
            content_scope: &scope,
        };
        let p = expected.position();
        let cell = match direction {
            CardinalStep::North => LogicalCell {
                x: p.x,
                y: p.y - 1,
                z: i32::from(p.floor),
            },
            CardinalStep::East => LogicalCell {
                x: p.x + 1,
                y: p.y,
                z: i32::from(p.floor),
            },
            _ => unreachable!("explicit North/East source-fixture route"),
        };
        // One exact destination claim per owner turn stays below existing native cap4.
        let index = EngineeringStaticCellIndex::from_claims(vec![EngineeringStaticCellClaim {
            scope: scope.clone(),
            cell,
            collision: EngineeringCollisionClaim::Qualified(CollisionClass::Walkable),
        }])
        .expect("qualified fixture");
        let result = MovementOwnerTurn::begin(r, std::num::NonZeroUsize::MIN)
            .try_step(actor, expected, &selected, &index, direction);
        assert!(
            matches!(result, Ok(MovementTurnOutcome::Applied(_))),
            "native source-fixture relocation refused: {result:?}"
        );
    }
    assert_eq!(
        r.read_actor_position(actor)
            .expect("qualified fixture")
            .position(),
        MovementLocalPosition {
            x: 101,
            y: 95,
            floor: 7
        }
    );
}

#[test]
fn native_curse_variant_one_choice_shared_across_real_player_roster_and_retry() {
    let packet = native_variant_packet();
    let case = packet["cases"]
        .as_array()
        .expect("qualified fixture")
        .iter()
        .find(|c| c["creature"]["key"] == "oteryn:creature.undead_dragon")
        .expect("qualified fixture");
    let (mut r, mut v, s, p, mut w, f, stamp) = setup_named(
        "undead_dragon",
        case["entry"].as_u64().expect("qualified fixture") as usize,
    );
    w.facing = Some(Facing::North);
    w.facts.condition_policy = Some(CurrentConditionPolicy {
        immunities: vec![],
        base_speed: 180,
    });
    move_existing_player_through_native_owner_to_curse_cone(&mut r, p.target);
    let session = GameSessionId::decode(&test_uuid(0x75)).expect("qualified fixture");
    let reservation = r.reserve_fresh_session(session).expect("qualified fixture");
    let second = r
        .commit_fresh_session(reservation)
        .expect("qualified fixture");
    r.initialize_source_pinned_lab_player_position(
        second,
        session,
        MovementLocalPosition {
            x: 102,
            y: 95,
            floor: 7,
        },
    )
    .expect("qualified fixture");
    v.initialize(
        &r,
        second,
        session,
        FACTS,
        (0, 0),
        oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
    )
    .expect("qualified fixture");
    w.extra.push((second, session, 1_000_000));
    let mut owner = DamageSpellOwner::default();
    let hit = owner
        .execute(
            &mut r,
            &f,
            stamp,
            &mut v,
            &s,
            &p,
            &mut w,
            SemanticTimeMicros::from_micros(2_000_000),
        )
        .expect("qualified fixture");
    assert_eq!(hit.targets.len(), 2);
    let chosen = hit.source_variant.as_ref().expect("qualified fixture");
    assert!(chosen.index < 2);
    assert_eq!(chosen.parent, p.ability);
    for (_, target) in &hit.targets {
        let target = target.as_ref().expect("qualified fixture");
        assert!(target.condition_only_applied);
        assert!(target.damage.is_none());
    }
    assert_eq!(
        owner
            .execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(9_000_000)
            )
            .expect("qualified fixture"),
        hit
    );
    assert_eq!(
        v.apply_attack_damage(
            &mut r,
            p.target,
            w.facts.attack.session,
            1,
            "variant-first-hp-probe",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(9_000_000)
        )
        .expect("qualified fixture")
        .health_after,
        184
    );
    assert_eq!(
        v.apply_attack_damage(
            &mut r,
            second,
            session,
            1,
            "variant-second-hp-probe",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(9_000_000)
        )
        .expect("qualified fixture")
        .health_after,
        184
    );
}

#[test]
fn native_variant_child_substitution_missing_closure_and_current_pin_refuse() {
    let packet = native_variant_packet();
    let records: Vec<ProjectReferenceRecord> =
        serde_json::from_value(packet["records"].clone()).expect("qualified fixture");
    let profiles: Vec<ProjectV2AuthoringProfile> =
        serde_json::from_value(packet["authoring_profiles"].clone()).expect("qualified fixture");
    let creature = Ref {
        family: ProjectV2Family::Creature,
        key: "oteryn:creature.barbarian_brutetamer".into(),
        revision: "definition-r1".into(),
    };
    let case = packet["cases"]
        .as_array()
        .expect("qualified fixture")
        .iter()
        .find(|c| c["creature"]["key"] == creature.key)
        .expect("qualified fixture");
    let index = case["entry"].as_u64().expect("qualified fixture") as usize;
    let mut wrong = profiles.clone();
    for p in &mut wrong {
        if p.target.key == case["parent"]
            && let Data::Ability(a) = &mut p.data
        {
            a.details.as_mut().expect("qualified fixture").variants[0].key =
                "oteryn:ability.spell.haste".into();
        }
    }
    assert_eq!(
        SpellSource::from_native(&creature, index, &records, &wrong, [1; 32])
            .expect_err("expected fixture rejection"),
        AttackError::InvalidSource
    );
    let missing = profiles
        .into_iter()
        .filter(|p| {
            p.target.key
                != format!(
                    "{}.variant-1",
                    case["parent"].as_str().expect("qualified fixture")
                )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        SpellSource::from_native(&creature, index, &records, &missing, [1; 32])
            .expect_err("expected fixture rejection"),
        AttackError::InvalidSource
    );
    let (mut r, mut v, mut source, proposal, mut reader, f, stamp) =
        setup_named("barbarian_brutetamer", index);
    source.content_digest = [9; 32];
    assert_eq!(
        DamageSpellOwner::default().execute(
            &mut r,
            &f,
            stamp,
            &mut v,
            &source,
            &proposal,
            &mut reader,
            SemanticTimeMicros::from_micros(2_000_000)
        ),
        Err(AttackError::ContentChanged)
    );
}

#[test]
fn native_djinn_source_area_dispel_invisible_player_without_primary_hp_and_replay() {
    for name in ["blue_djinn", "green_djinn"] {
        let (mut r, mut v, s, p, mut w, f, stamp) = setup_named(name, 6);
        assert!(s.dispel_invisible);
        let session = w.facts.attack.session;
        let before = v
            .read_owned_player_state_test_snapshot(&r, p.target, session)
            .expect("qualified fixture")
            .clone();
        let native_root =
            GameplayDecisionRoot::from_bytes(r.content_pin().server_artifact_digest());
        let invisible =
            oteryn_gameplay_foundation::foundation::condition::ConditionDefinition::new(
                "test.source.djinn.invisible",
                1,
                oteryn_gameplay_domain::domain::condition::ConditionValues::Invisible {
                    duration_ms: 5000,
                },
            )
            .expect("qualified fixture");
        let seed_facts = oteryn_gameplay_foundation::foundation::condition::ApplicationFacts {
            now: 1_000_000,
            base_speed: 180,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &native_root,
            occurrence: DecisionOccurrenceId::from_bytes([91; 16]),
        };
        // Native player owned store, explicit current self-use and shared vitals CAS.
        assert!(v.install_owned_player_self_use_test_condition(
            &r,
            &f,
            stamp,
            p.target,
            session,
            &invisible,
            &seed_facts,
        ));
        let seeded = v
            .read_owned_player_state_test_snapshot(&r, p.target, session)
            .expect("qualified fixture")
            .clone();
        assert_eq!(seeded.vitals(), before.vitals());
        assert_eq!(seeded.revision(), before.revision() + 1);
        assert_eq!(
            v.native_actor_invisible(
                &r,
                p.target,
                Some(session),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(2_000_000)
            ),
            Some(true)
        );
        assert_eq!(
            v.read_owned_player_state_test_snapshot(&r, p.target, session)
                .expect("qualified fixture"),
            &seeded
        );
        w.facts.attack.visible = false;
        let mut o = DamageSpellOwner::default();
        let result = o
            .execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(2_000_000),
            )
            .expect("qualified fixture");
        assert_eq!(result.targets.len(), 1);
        let hit = result.targets[0].1.as_ref().expect("qualified fixture");
        assert!(hit.condition_only_applied);
        assert!(hit.damage.is_none() && hit.mana.is_none());
        assert_eq!(result.requested, 0);
        assert_eq!(
            v.native_actor_invisible(
                &r,
                p.target,
                Some(session),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(2_000_000)
            ),
            Some(false)
        );
        let cured = v
            .read_owned_player_state_test_snapshot(&r, p.target, session)
            .expect("qualified fixture")
            .clone();
        assert_eq!(cured.vitals(), before.vitals());
        assert_eq!(cured.revision(), seeded.revision() + 1);

        assert_eq!(
            o.execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(9_000_000)
            )
            .expect("qualified fixture"),
            result
        );
        assert_eq!(
            v.read_owned_player_state_test_snapshot(&r, p.target, session)
                .expect("qualified fixture"),
            &cured
        );
        assert_eq!(
            v.apply_attack_damage(
                &mut r,
                p.target,
                w.facts.attack.session,
                1,
                "dispel-hp-probe",
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(9_000_000)
            )
            .expect("qualified fixture")
            .health_after,
            184
        );
    }
}
#[test]
fn native_djinn_dispel_visibility_exception_keeps_current_pz_and_source_guards() {
    let (mut r, mut v, s, p, mut w, f, stamp) = setup_named("blue_djinn", 6);
    let mut o = DamageSpellOwner::default();
    w.facts.attack.visible = false;
    w.facts.attack.target_pz = true;
    assert!(
        o.execute(
            &mut r,
            &f,
            stamp,
            &mut v,
            &s,
            &p,
            &mut w,
            SemanticTimeMicros::from_micros(2_000_000)
        )
        .expect("qualified fixture")
        .targets
        .is_empty()
    );
    let mut profiles = native();
    assert!(
        SpellSource::from_native(
            &Ref {
                family: ProjectV2Family::Creature,
                key: "oteryn:creature.blue_djinn".into(),
                revision: "definition-r1".into()
            },
            6,
            &profiles.0,
            &profiles.1,
            [1; 32]
        )
        .is_err()
    );
    let packet:serde_json::Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../docs/agents/evidence/monster-full-mechanics-20261004/lanes/conditions/remaining-source-families/dispel/native-fixture.json"))).expect("qualified fixture");
    profiles = (
        serde_json::from_value(packet["records"].clone()).expect("qualified fixture"),
        serde_json::from_value(packet["authoring_profiles"].clone()).expect("qualified fixture"),
    );
    for profile in &mut profiles.1 {
        if profile.target.key == s.ability.key
            && let Data::Ability(a) = &mut profile.data
        {
            a.details.as_mut().expect("qualified fixture").range_tiles = 1;
        }
    }
    assert!(matches!(
        SpellSource::from_native(
            &Ref {
                family: ProjectV2Family::Creature,
                key: "oteryn:creature.blue_djinn".into(),
                revision: "definition-r1".into()
            },
            6,
            &profiles.0,
            &profiles.1,
            [1; 32]
        ),
        Err(AttackError::UnsupportedShape)
    ));
}

#[test]
fn actual_source_schedule_area_damage_real_hp_and_replay() {
    let (mut r, mut v, s, p, mut w, f, stamp) = setup();
    let mut o = DamageSpellOwner::default();
    let hit = o
        .execute(
            &mut r,
            &f,
            stamp,
            &mut v,
            &s,
            &p,
            &mut w,
            SemanticTimeMicros::from_micros(2_000_000),
        )
        .expect("qualified fixture");
    assert_eq!(hit.targets.len(), 1);
    let receipt = hit.targets[0]
        .1
        .as_ref()
        .expect("qualified fixture")
        .damage
        .expect("qualified fixture");
    assert!(receipt.applied > 0);
    assert_eq!(
        o.execute(
            &mut r,
            &f,
            stamp,
            &mut v,
            &s,
            &p,
            &mut w,
            SemanticTimeMicros::from_micros(9_000_000)
        )
        .expect("qualified fixture"),
        hit
    );
    let mut conflict = p.clone();
    conflict.target = p.issuer;
    assert_eq!(
        o.execute(
            &mut r,
            &f,
            stamp,
            &mut v,
            &s,
            &conflict,
            &mut w,
            SemanticTimeMicros::from_micros(9_000_000)
        ),
        Err(AttackError::OccurrenceConflict)
    );
}
#[test]
fn independently_missing_los_policy_refuses_entire_cast_no_buffer() {
    let (mut r, mut v, s, p, mut w, f, stamp) = setup();
    let mut o = DamageSpellOwner::default();
    w.missing = true;
    assert_eq!(
        o.execute(
            &mut r,
            &f,
            stamp,
            &mut v,
            &s,
            &p,
            &mut w,
            SemanticTimeMicros::from_micros(2_000_000)
        ),
        Err(AttackError::MissingCombatFacts)
    );
    w.missing = false;
    assert_eq!(
        o.execute(
            &mut r,
            &f,
            stamp,
            &mut v,
            &s,
            &p,
            &mut w,
            SemanticTimeMicros::from_micros(4_000_000)
        ),
        Err(AttackError::MissingCombatFacts)
    );
    assert_eq!(
        v.apply_attack_damage(
            &mut r,
            p.target,
            w.facts.attack.session,
            1,
            "independent-spell-probe",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(4_000_000)
        )
        .expect("qualified fixture")
        .health_after,
        184
    );
}
#[test]
fn independent_tile_combat_rejection_and_retired_scope() {
    let (mut r, mut v, s, p, mut w, mut f, stamp) = setup();
    w.blocked = true;
    let mut o = DamageSpellOwner::default();
    assert!(
        o.execute(
            &mut r,
            &f,
            stamp,
            &mut v,
            &s,
            &p,
            &mut w,
            SemanticTimeMicros::from_micros(2_000_000)
        )
        .expect("qualified fixture")
        .targets
        .is_empty()
    );
    f.apply_external_grant(
        crate::foundation::ScopeOwnershipGeneration::new(2).expect("qualified fixture"),
    )
    .expect("qualified fixture");
    assert_eq!(
        o.execute(
            &mut r,
            &f,
            stamp,
            &mut v,
            &s,
            &p,
            &mut w,
            SemanticTimeMicros::from_micros(3_000_000)
        ),
        Err(AttackError::StaleOwner)
    );
    assert_eq!(
        v.apply_attack_damage(
            &mut r,
            p.target,
            w.facts.attack.session,
            1,
            "tile-refusal-probe",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(3_000_000)
        )
        .expect("qualified fixture")
        .health_after,
        184
    );
}
#[test]
fn actual_source_inline_energy_condition_has_no_fake_primary_hp() {
    let (mut r, mut v, s, p, mut w, f, stamp) = setup_named("abyssador", 1);
    assert_eq!((s.minimum, s.maximum), (0, 0));
    assert!(s.element.is_none());
    assert!(!s.conditions.is_empty());
    w.facts.condition_policy = Some(CurrentConditionPolicy {
        immunities: vec![],
        base_speed: 180,
    });
    let mut o = DamageSpellOwner::default();
    let result = o
        .execute(
            &mut r,
            &f,
            stamp,
            &mut v,
            &s,
            &p,
            &mut w,
            SemanticTimeMicros::from_micros(2_000_000),
        )
        .expect("actual typed condition owner admission");
    assert_eq!(result.targets.len(), 1);
    let target = result.targets[0].1.as_ref().expect("actual store commit");
    assert!(target.condition_only_applied);
    assert!(target.damage.is_none());
    assert_eq!(
        o.execute(
            &mut r,
            &f,
            stamp,
            &mut v,
            &s,
            &p,
            &mut w,
            SemanticTimeMicros::from_micros(9_000_000)
        )
        .expect("retained condition-only cast"),
        result
    );
    assert_eq!(
        v.apply_attack_damage(
            &mut r,
            p.target,
            w.facts.attack.session,
            1,
            "condition-no-primary-hp-probe",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(9_000_000)
        )
        .expect("real HP store")
        .health_after,
        184
    );
}

#[test]
fn native_hunter_armor_only_blocks_real_arrow_without_vital_commit() {
    let (mut r, mut v, s, p, mut w, f, stamp) = setup_named("hunter", 1);
    assert!(s.armor);
    w.facts.attack.armor = 10_000;
    let mut o = DamageSpellOwner::default();
    let result = o
        .execute(
            &mut r,
            &f,
            stamp,
            &mut v,
            &s,
            &p,
            &mut w,
            SemanticTimeMicros::from_micros(2_000_000),
        )
        .expect("native arrow source");
    assert!(result.armor_only_attack1_approximation);
    assert_eq!(result.targets.len(), 1);
    assert!(
        result.targets[0]
            .1
            .as_ref()
            .expect("qualified fixture")
            .damage
            .is_none()
    );
    assert!(
        !result.targets[0]
            .1
            .as_ref()
            .expect("qualified fixture")
            .condition_only_applied
    );
    assert_eq!(
        v.apply_attack_damage(
            &mut r,
            p.target,
            w.facts.attack.session,
            1,
            "armor-arrow-probe",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(2_000_000)
        )
        .expect("qualified fixture")
        .health_after,
        184
    );
}
#[test]
fn primary_native_immunity_skips_source_curse_secondary_condition() {
    let (mut r, mut v, s, p, mut w, f, stamp) = setup_named("blightwalker", 3);
    assert!(s.element.is_some());
    assert!(!s.conditions.is_empty());
    w.facts.immune = true;
    w.facts.condition_policy = Some(CurrentConditionPolicy {
        immunities: vec![],
        base_speed: 180,
    });
    let mut o = DamageSpellOwner::default();
    let result = o
        .execute(
            &mut r,
            &f,
            stamp,
            &mut v,
            &s,
            &p,
            &mut w,
            SemanticTimeMicros::from_micros(2_000_000),
        )
        .expect("source combat immunity admission");
    assert_eq!(result.targets.len(), 1);
    let target = result.targets[0].1.as_ref().expect("qualified fixture");
    assert!(target.damage.is_none());
    assert!(!target.condition_only_applied);
    assert_eq!(
        v.apply_attack_damage(
            &mut r,
            p.target,
            w.facts.attack.session,
            1,
            "immune-curse-probe",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(2_000_000)
        )
        .expect("qualified fixture")
        .health_after,
        184
    );
}

#[test]
fn all_target_amounts_preflight_before_first_real_hp_even_when_second_overflows() {
    let (mut r, mut v, mut s, p, mut w, f, stamp) = setup();
    let second_session = GameSessionId::decode(&test_uuid(0x72)).expect("qualified fixture");
    let reserve = r
        .reserve_fresh_session(second_session)
        .expect("qualified fixture");
    let second = r.commit_fresh_session(reserve).expect("qualified fixture");
    r.initialize_source_pinned_lab_player_position(
        second,
        second_session,
        MovementLocalPosition {
            x: 102,
            y: 100,
            floor: 7,
        },
    )
    .expect("qualified fixture");
    v.initialize(
        &r,
        second,
        second_session,
        FACTS,
        (0, 0),
        oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
    )
    .expect("qualified fixture");
    if p.target.placement_identity() < second.placement_identity() {
        w.extra.push((second, second_session, 10_000_000));
    } else {
        w.facts.multiplier_ppm = 10_000_000;
        w.extra.push((second, second_session, 1_000_000));
    }
    // Adversarial private descriptor bounds test, not a claim that the donor uses this damage.
    // Both victims remain genuine ChannelSpellStates185HP; no model-health substitute.
    s.minimum = u32::MAX;
    s.maximum = u32::MAX;
    let mut o = DamageSpellOwner::default();
    assert_eq!(
        o.execute(
            &mut r,
            &f,
            stamp,
            &mut v,
            &s,
            &p,
            &mut w,
            SemanticTimeMicros::from_micros(2_000_000)
        ),
        Err(AttackError::NumericOverflow)
    );
    assert_eq!(
        v.apply_attack_damage(
            &mut r,
            p.target,
            w.facts.attack.session,
            1,
            "overflow-A-probe",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(2_000_000)
        )
        .expect("qualified fixture")
        .health_after,
        184
    );
    assert_eq!(
        v.apply_attack_damage(
            &mut r,
            second,
            second_session,
            1,
            "overflow-B-probe",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(2_000_000)
        )
        .expect("qualified fixture")
        .health_after,
        184
    );
}

#[test]
fn native_glyph_mana_drain_actual_mp_clamp_revision_no_hp_and_replay() {
    let (mut r, mut v, s, p, mut w, f, stamp) = setup_named("an_astral_glyph", 3);
    assert_eq!(s.element.as_deref(), Some("mana_drain"));
    let mut o = DamageSpellOwner::default();
    let hit = o
        .execute(
            &mut r,
            &f,
            stamp,
            &mut v,
            &s,
            &p,
            &mut w,
            SemanticTimeMicros::from_micros(2_000_000),
        )
        .expect("qualified fixture");
    let t = hit.targets[0].1.as_ref().expect("qualified fixture");
    assert!(t.damage.is_none());
    assert!(!t.condition_only_applied);
    let mp = t.mana.expect("qualified fixture");
    assert_eq!(
        (
            mp.applied,
            mp.mana_before,
            mp.mana_after,
            mp.vitals_revision
        ),
        (90, 90, 0, 2)
    );
    let (revision, current) = crate::gameplay_transport::actor_spell::observe_vitals(
        &r,
        &v,
        p.target,
        w.facts.attack.session,
    )
    .expect("qualified fixture");
    assert_eq!((revision, current.health, current.mana), (2, 185, 0));
    assert_eq!(
        o.execute(
            &mut r,
            &f,
            stamp,
            &mut v,
            &s,
            &p,
            &mut w,
            SemanticTimeMicros::from_micros(9_000_000)
        )
        .expect("qualified fixture"),
        hit
    );
    let (revision, current) = crate::gameplay_transport::actor_spell::observe_vitals(
        &r,
        &v,
        p.target,
        w.facts.attack.session,
    )
    .expect("qualified fixture");
    assert_eq!((revision, current.health, current.mana), (2, 185, 0));
}
#[test]
fn actual_mana_owner_replay_conflict_session_and_current_death_guards() {
    let (mut r, mut v, _s, p, w, _f, _stamp) = setup_named("an_astral_glyph", 3);
    let session = w.facts.attack.session;
    let first = v
        .apply_attack_mana_drain(
            &mut r,
            p.target,
            session,
            40,
            "mana-owner-1",
            crate::foundation::owner_timer::OwnerClock::now(
                &crate::foundation::owner_timer::VirtualOwnerClock::new(
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
                ),
            ),
        )
        .expect("qualified fixture");
    assert_eq!(
        (
            first.applied,
            first.mana_before,
            first.mana_after,
            first.vitals_revision
        ),
        (40, 90, 50, 2)
    );
    assert_eq!(
        v.apply_attack_mana_drain(
            &mut r,
            p.target,
            session,
            40,
            "mana-owner-1",
            crate::foundation::owner_timer::OwnerClock::now(
                &crate::foundation::owner_timer::VirtualOwnerClock::new(
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
                )
            )
        ),
        Some(first)
    );
    assert!(
        v.apply_attack_mana_drain(
            &mut r,
            p.target,
            session,
            41,
            "mana-owner-1",
            crate::foundation::owner_timer::OwnerClock::now(
                &crate::foundation::owner_timer::VirtualOwnerClock::new(
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
                )
            )
        )
        .is_none()
    );
    assert!(
        v.apply_attack_mana_drain(
            &mut r,
            p.target,
            GameSessionId::decode(&test_uuid(0x79)).expect("qualified fixture"),
            1,
            "wrong-session",
            crate::foundation::owner_timer::OwnerClock::now(
                &crate::foundation::owner_timer::VirtualOwnerClock::new(
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
                )
            )
        )
        .is_none()
    );
    let death = v
        .apply_attack_damage(
            &mut r,
            p.target,
            session,
            185,
            "independent-lethal-owner",
            crate::foundation::owner_timer::OwnerClock::now(
                &crate::foundation::owner_timer::VirtualOwnerClock::new(
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
                ),
            ),
        )
        .expect("qualified fixture");
    assert_eq!(death.health_after, 0);
    assert!(death.death.is_some());
    assert!(
        v.apply_attack_mana_drain(
            &mut r,
            p.target,
            session,
            1,
            "mana-after-death",
            crate::foundation::owner_timer::OwnerClock::now(
                &crate::foundation::owner_timer::VirtualOwnerClock::new(
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
                )
            )
        )
        .is_none()
    );
    assert!(
        v.apply_attack_mana_drain(
            &mut r,
            p.target,
            session,
            40,
            "mana-owner-1",
            crate::foundation::owner_timer::OwnerClock::now(
                &crate::foundation::owner_timer::VirtualOwnerClock::new(
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
                )
            )
        )
        .is_none()
    );
    let (revision, current) =
        crate::gameplay_transport::actor_spell::observe_vitals(&r, &v, p.target, session)
            .expect("qualified fixture");
    assert_eq!((revision, current.health, current.mana), (3, 0, 50));
}
#[test]
fn missing_current_world_facts_refuses_mana_without_hp_or_mp_buffer() {
    let (mut r, mut v, s, p, mut w, f, stamp) = setup_named("an_astral_glyph", 3);
    w.missing = true;
    let mut o = DamageSpellOwner::default();
    assert_eq!(
        o.execute(
            &mut r,
            &f,
            stamp,
            &mut v,
            &s,
            &p,
            &mut w,
            SemanticTimeMicros::from_micros(2_000_000)
        ),
        Err(AttackError::MissingCombatFacts)
    );
    w.missing = false;
    assert_eq!(
        o.execute(
            &mut r,
            &f,
            stamp,
            &mut v,
            &s,
            &p,
            &mut w,
            SemanticTimeMicros::from_micros(4_000_000)
        ),
        Err(AttackError::MissingCombatFacts)
    );
    let (revision, current) = crate::gameplay_transport::actor_spell::observe_vitals(
        &r,
        &v,
        p.target,
        w.facts.attack.session,
    )
    .expect("qualified fixture");
    assert_eq!((revision, current.health, current.mana), (1, 185, 90));
}
#[test]
fn actual_exhausted_mp_noop_retains_revision_and_health() {
    let (mut r, mut v, _s, p, w, _f, _stamp) = setup_named("an_astral_glyph", 3);
    let session = w.facts.attack.session;
    v.apply_attack_mana_drain(
        &mut r,
        p.target,
        session,
        900,
        "mana-full-clamp",
        crate::foundation::owner_timer::OwnerClock::now(
            &crate::foundation::owner_timer::VirtualOwnerClock::new(
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
            ),
        ),
    )
    .expect("qualified fixture");
    let zero = v
        .apply_attack_mana_drain(
            &mut r,
            p.target,
            session,
            1,
            "mana-exhausted",
            crate::foundation::owner_timer::OwnerClock::now(
                &crate::foundation::owner_timer::VirtualOwnerClock::new(
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
                ),
            ),
        )
        .expect("qualified fixture");
    assert_eq!(
        (
            zero.applied,
            zero.mana_before,
            zero.mana_after,
            zero.vitals_revision
        ),
        (0, 0, 0, 2)
    );
    let (revision, current) =
        crate::gameplay_transport::actor_spell::observe_vitals(&r, &v, p.target, session)
            .expect("qualified fixture");
    assert_eq!((revision, current.health, current.mana), (2, 185, 0));
}

#[test]
fn newer_mana_occurrence_with_nonadvancing_clock_never_commits_mp() {
    let (mut r, mut v, s, p, mut w, f, stamp) = setup_named("an_astral_glyph", 3);
    let mut o = DamageSpellOwner::default();
    o.execute(
        &mut r,
        &f,
        stamp,
        &mut v,
        &s,
        &p,
        &mut w,
        SemanticTimeMicros::from_micros(2_000_000),
    )
    .expect("qualified fixture");
    // Deliberately inconsistent newer event/current clock input, not a source scheduler fixture.
    let mut forged = p.clone();
    let old = o.casts[0].sequence;
    forged.occurrence = crate::ability::AbilityOccurrence::new(
        &format!(
            "ai-profile:{}:{}:attack:{}",
            hex(&p.issuer.placement_identity()),
            old + 1,
            s.entry_index
        ),
        p.occurrence.revisions().clone(),
    )
    .expect("qualified fixture");
    assert_eq!(
        o.execute(
            &mut r,
            &f,
            stamp,
            &mut v,
            &s,
            &forged,
            &mut w,
            SemanticTimeMicros::from_micros(1_000_000)
        ),
        Err(AttackError::NotDue)
    );
    let (revision, current) = crate::gameplay_transport::actor_spell::observe_vitals(
        &r,
        &v,
        p.target,
        w.facts.attack.session,
    )
    .expect("qualified fixture");
    assert_eq!((revision, current.health, current.mana), (2, 185, 0));
}

struct CurrentChainMap {
    missing: bool,
}
impl crate::creature_chain_attack::ChainMapFacts for CurrentChainMap {
    fn sight(
        &mut self,
        _: &ChannelRuntimeV1,
        _: crate::spell::chain::TilePosition,
        _: crate::spell::chain::TilePosition,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        (!self.missing).then_some(true)
    }
    fn path(
        &mut self,
        _: &ChannelRuntimeV1,
        _: crate::spell::chain::TilePosition,
        to: crate::spell::chain::TilePosition,
        _: RuntimeWorkStamp,
    ) -> Option<Vec<crate::spell::chain::TilePosition>> {
        (!self.missing).then_some(vec![to])
    }
}
fn chain_second(
    r: &mut ChannelRuntimeV1,
    v: &mut ChannelSpellStates,
    w: &mut CurrentWorld,
) -> (ExactActorRef, GameSessionId) {
    let session = GameSessionId::decode(&test_uuid(0x72)).expect("qualified fixture");
    let reserve = r.reserve_fresh_session(session).expect("qualified fixture");
    let actor = r.commit_fresh_session(reserve).expect("qualified fixture");
    r.initialize_source_pinned_lab_player_position(
        actor,
        session,
        MovementLocalPosition {
            x: 102,
            y: 100,
            floor: 7,
        },
    )
    .expect("qualified fixture");
    v.initialize(
        r,
        actor,
        session,
        FACTS,
        (0, 0),
        oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
    )
    .expect("qualified fixture");
    w.extra.push((actor, session, 1_000_000));
    (actor, session)
}
#[test]
fn chain_native_hp_two_targets_not_before_fifty_ms_and_retry_once() {
    use crate::creature_chain_attack::ChainOwner;
    use crate::foundation::owner_timer::VirtualOwnerClock;
    let (mut r, mut v, s, p, mut w, mut f, stamp) = setup_named("bony_sea_devil", 5);
    let (second, session) = chain_second(&mut r, &mut v, &mut w);
    let mut map = CurrentChainMap { missing: false };
    let mut chain = ChainOwner::new(&r, &f).expect("qualified fixture");
    let damage = DamageSpellOwner::default();
    let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0));
    let token = chain
        .schedule(
            &r,
            &f,
            stamp,
            &s,
            &p,
            &mut w,
            &mut map,
            SemanticTimeMicros::from_micros(0),
        )
        .expect("qualified fixture");
    assert_eq!(
        chain
            .schedule(
                &r,
                &f,
                stamp,
                &s,
                &p,
                &mut w,
                &mut map,
                SemanticTimeMicros::from_micros(0)
            )
            .expect("qualified fixture"),
        token
    );
    let first = chain
        .run_due(&mut r, &mut f, &mut v, &damage, &mut w, &mut map, &clock)
        .expect("qualified fixture");
    assert_eq!(first.len(), 1);
    assert!(
        first[0].2.as_ref().expect("qualified fixture").targets[0]
            .1
            .as_ref()
            .expect("qualified fixture")
            .damage
            .expect("qualified fixture")
            .applied
            > 0
    );
    let (_, before) =
        crate::gameplay_transport::actor_spell::observe_vitals(&r, &v, second, session)
            .expect("qualified fixture");
    assert_eq!(before.health, 185);
    clock.advance(49_000);
    assert!(
        chain
            .run_due(&mut r, &mut f, &mut v, &damage, &mut w, &mut map, &clock)
            .expect("qualified fixture")
            .is_empty()
    );
    clock.advance(1_000);
    let next = chain
        .run_due(&mut r, &mut f, &mut v, &damage, &mut w, &mut map, &clock)
        .expect("qualified fixture");
    assert_eq!(next.len(), 1);
    assert!(
        next[0].2.as_ref().expect("qualified fixture").targets[0]
            .1
            .as_ref()
            .expect("qualified fixture")
            .damage
            .expect("qualified fixture")
            .applied
            > 0
    );
    assert_eq!(chain.receipts(token).expect("qualified fixture").len(), 2);
    assert!(
        chain
            .run_due(&mut r, &mut f, &mut v, &damage, &mut w, &mut map, &clock)
            .expect("qualified fixture")
            .is_empty()
    );
}
#[test]
fn chain_native_mana_two_actual_player_stores_no_hp() {
    use crate::creature_chain_attack::ChainOwner;
    use crate::foundation::owner_timer::VirtualOwnerClock;
    let (mut r, mut v, s, p, mut w, mut f, stamp) = setup_named("timira_the_many_headed", 3);
    let (second, session) = chain_second(&mut r, &mut v, &mut w);
    let mut map = CurrentChainMap { missing: false };
    let mut chain = ChainOwner::new(&r, &f).expect("qualified fixture");
    let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0));
    let damage = DamageSpellOwner::default();
    chain
        .schedule(
            &r,
            &f,
            stamp,
            &s,
            &p,
            &mut w,
            &mut map,
            SemanticTimeMicros::from_micros(0),
        )
        .expect("qualified fixture");
    let first = chain
        .run_due(&mut r, &mut f, &mut v, &damage, &mut w, &mut map, &clock)
        .expect("qualified fixture");
    let receipt = first[0].2.as_ref().expect("qualified fixture").targets[0]
        .1
        .as_ref()
        .expect("qualified fixture");
    assert!(receipt.damage.is_none());
    assert!(receipt.mana.expect("qualified fixture").applied > 0);
    clock.advance(49_000);
    assert!(
        chain
            .run_due(&mut r, &mut f, &mut v, &damage, &mut w, &mut map, &clock)
            .expect("qualified fixture")
            .is_empty()
    );
    let (_, before) =
        crate::gameplay_transport::actor_spell::observe_vitals(&r, &v, second, session)
            .expect("qualified fixture");
    assert_eq!((before.health, before.mana), (185, 90));
    clock.advance(1_000);
    let next = chain
        .run_due(&mut r, &mut f, &mut v, &damage, &mut w, &mut map, &clock)
        .expect("qualified fixture");
    assert!(
        next[0].2.as_ref().expect("qualified fixture").targets[0]
            .1
            .as_ref()
            .expect("qualified fixture")
            .mana
            .expect("qualified fixture")
            .applied
            > 0
    );
    let (_, after) =
        crate::gameplay_transport::actor_spell::observe_vitals(&r, &v, second, session)
            .expect("qualified fixture");
    assert_eq!(after.health, 185);
    assert!(after.mana < 90);
}
#[test]
fn chain_due_missing_current_los_retains_failure_and_never_retries_step() {
    use crate::creature_chain_attack::ChainOwner;
    use crate::foundation::owner_timer::VirtualOwnerClock;
    let (mut r, mut v, s, p, mut w, mut f, stamp) = setup_named("bony_sea_devil", 5);
    let (second, session) = chain_second(&mut r, &mut v, &mut w);
    let mut map = CurrentChainMap { missing: false };
    let mut chain = ChainOwner::new(&r, &f).expect("qualified fixture");
    let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0));
    let damage = DamageSpellOwner::default();
    let token = chain
        .schedule(
            &r,
            &f,
            stamp,
            &s,
            &p,
            &mut w,
            &mut map,
            SemanticTimeMicros::from_micros(0),
        )
        .expect("qualified fixture");
    chain
        .run_due(&mut r, &mut f, &mut v, &damage, &mut w, &mut map, &clock)
        .expect("qualified fixture");
    clock.advance(50_000);
    map.missing = true;
    let next = chain
        .run_due(&mut r, &mut f, &mut v, &damage, &mut w, &mut map, &clock)
        .expect("qualified fixture");
    assert_eq!(next[0].2, Err(AttackError::MissingCombatFacts));
    assert_eq!(chain.receipts(token).expect("qualified fixture").len(), 2);
    map.missing = false;
    assert!(
        chain
            .run_due(&mut r, &mut f, &mut v, &damage, &mut w, &mut map, &clock)
            .expect("qualified fixture")
            .is_empty()
    );
    let (_, after) =
        crate::gameplay_transport::actor_spell::observe_vitals(&r, &v, second, session)
            .expect("qualified fixture");
    assert_eq!(after.health, 185);
    f.apply_external_grant(
        crate::foundation::ScopeOwnershipGeneration::new(2).expect("qualified fixture"),
    )
    .expect("qualified fixture");
    assert_eq!(
        chain.run_due(&mut r, &mut f, &mut v, &damage, &mut w, &mut map, &clock),
        Err(AttackError::StaleOwner)
    );
}

#[test]
fn chain_retired_target_generation_refuses_frozen_second_step() {
    use crate::creature_chain_attack::ChainOwner;
    use crate::foundation::owner_timer::VirtualOwnerClock;
    let (mut r, mut v, s, p, mut w, mut f, stamp) = setup_named("bony_sea_devil", 5);
    let (second, _) = chain_second(&mut r, &mut v, &mut w);
    let mut map = CurrentChainMap { missing: false };
    let mut chain = ChainOwner::new(&r, &f).expect("qualified fixture");
    let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0));
    let damage = DamageSpellOwner::default();
    let token = chain
        .schedule(
            &r,
            &f,
            stamp,
            &s,
            &p,
            &mut w,
            &mut map,
            SemanticTimeMicros::from_micros(0),
        )
        .expect("qualified fixture");
    chain
        .run_due(&mut r, &mut f, &mut v, &damage, &mut w, &mut map, &clock)
        .expect("qualified fixture");
    r.remove_test_actor(second).expect("qualified fixture");
    clock.advance(50_000);
    let result = chain
        .run_due(&mut r, &mut f, &mut v, &damage, &mut w, &mut map, &clock)
        .expect("qualified fixture");
    assert_eq!(result[0].2, Err(AttackError::StaleTarget));
    assert_eq!(chain.receipts(token).expect("qualified fixture").len(), 2);
    assert!(
        chain
            .run_due(&mut r, &mut f, &mut v, &damage, &mut w, &mut map, &clock)
            .expect("qualified fixture")
            .is_empty()
    );
}
#[test]
fn native_presentation_only_produces_source_cues_geometry_without_hp_mp_commit() {
    let (mut r, mut v, s, p, mut w, f, stamp) = setup_named("apocalypse", 3);
    assert_eq!((s.minimum, s.maximum), (0, 0));
    let mut owner = DamageSpellOwner::default();
    let result = owner
        .execute(
            &mut r,
            &f,
            stamp,
            &mut v,
            &s,
            &p,
            &mut w,
            SemanticTimeMicros::from_micros(0),
        )
        .expect("qualified fixture");
    let event = result.presentation.as_ref().expect("qualified fixture");
    assert!(!event.tiles.is_empty());
    assert!(event.source.impact_cue.is_some());
    assert!(event.source.effects.iter().all(|e|matches!(e,ProjectV2AbilityEffect::Inline(i)if matches!(i.operation,crate::content::ProjectV2InlineEffectOperation::PresentationOnly))));
    let (revision, before) = crate::gameplay_transport::actor_spell::observe_vitals(
        &r,
        &v,
        p.target,
        w.facts.attack.session,
    )
    .expect("qualified fixture");
    assert_eq!((revision, before.health, before.mana), (1, 185, 90));
    assert_eq!(
        owner
            .execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(999)
            )
            .expect("qualified fixture"),
        result
    );
}
#[test]
fn fairy_combat_none_source_preserves_presentation_does_not_invent_ice_damage() {
    for index in [1, 2] {
        let (mut r, mut v, s, p, mut w, f, stamp) = setup_named("angry_sugar_fairy", index);
        assert!(s.source_none_correction);
        assert!(s.element.is_none());
        assert_eq!((s.minimum, s.maximum), (0, 0));
        let result = DamageSpellOwner::default()
            .execute(
                &mut r,
                &f,
                stamp,
                &mut v,
                &s,
                &p,
                &mut w,
                SemanticTimeMicros::from_micros(0),
            )
            .expect("qualified fixture");
        assert_eq!(result.requested, 0);
        let event = result.presentation.expect("qualified fixture");
        assert!(
            event
                .qualification
                .contains("SOURCE_COMBAT_NONE_NO_HP_CORRECTION")
        );
        assert!(event.source.effects.iter().all(|e|matches!(e,ProjectV2AbilityEffect::Inline(i)if matches!(i.operation,crate::content::ProjectV2InlineEffectOperation::PresentationOnly))));
        let (revision, current) = crate::gameplay_transport::actor_spell::observe_vitals(
            &r,
            &v,
            p.target,
            w.facts.attack.session,
        )
        .expect("qualified fixture");
        assert_eq!((revision, current.health, current.mana), (1, 185, 90));
    }
}
#[test]
fn presentation_current_tile_policy_unavailable_refuses_and_no_vital_commit() {
    let (mut r, mut v, s, p, mut w, f, stamp) = setup_named("apocalypse", 3);
    w.missing = true;
    let mut owner = DamageSpellOwner::default();
    assert_eq!(
        owner.execute(
            &mut r,
            &f,
            stamp,
            &mut v,
            &s,
            &p,
            &mut w,
            SemanticTimeMicros::from_micros(0)
        ),
        Err(AttackError::MissingCombatFacts)
    );
    let (revision, current) = crate::gameplay_transport::actor_spell::observe_vitals(
        &r,
        &v,
        p.target,
        w.facts.attack.session,
    )
    .expect("qualified fixture");
    assert_eq!((revision, current.health, current.mana), (1, 185, 90));
    w.missing = false;
    assert_eq!(
        owner.execute(
            &mut r,
            &f,
            stamp,
            &mut v,
            &s,
            &p,
            &mut w,
            SemanticTimeMicros::from_micros(2)
        ),
        Err(AttackError::MissingCombatFacts)
    );
}
