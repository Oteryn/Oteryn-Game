//! Real native source-owner regressions for the accepted charm incoming order.
//! This file is auto-mounted; the cases are in-memory owner tests, not DB coverage.
#![allow(
    clippy::expect_used,
    reason = "test fixture invariants use named assertions"
)]
use crate::ai_think::{
    ThinkSequenceTracker,
    profile_schedule::{AttackTarget, ProfileScheduleState},
};
use crate::content::{
    ProjectReferenceRecord, ProjectV2AuthoringProfile, ProjectV2AuthoringProfileData as Data,
    ProjectV2DefinitionRef as Ref, ProjectV2Family,
};
use crate::creature_auto_attack::{AttackFacts, CurrentConditionPolicy, ProfileAbilityProposal};
use crate::creature_damage_spell::*;
use crate::foundation::MovementLocalPosition;
use crate::foundation::owner_timer::SemanticTimeMicros;
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, RuntimeWorkStamp, ScopeRuntimeFence,
};
use crate::gameplay_transport::actor_spell::{ChannelSpellStates, tests::FACTS};
use oteryn_gameplay_content::content::creature_attack_geometry::Facing;
use oteryn_simulation_determinism::GameplayDecisionRoot;

struct CurrentWorld {
    facts: SpellCombatFacts,
    missing: bool,
    blocked: bool,
    facing: Option<Facing>,
}
impl SpellWorldReader for CurrentWorld {
    fn current_players(
        &mut self,
        r: &ChannelRuntimeV1,
        _: RuntimeWorkStamp,
    ) -> Option<Vec<(ExactActorRef, GameSessionId)>> {
        r.player_control_facts(self.facts.attack.target, self.facts.attack.session)
            .ok()?;
        Some(vec![(self.facts.attack.target, self.facts.attack.session)])
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
        (!self.missing
            && self.facts.attack.issuer == i
            && self.facts.attack.target == t
            && self.facts.attack.session == s)
            .then(|| self.facts.clone())
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
fn setup_named_at(
    name: &str,
    index: usize,
    at: MovementLocalPosition,
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
    r.initialize_source_pinned_lab_player_position(t, s, at)
        .expect("qualified fixture");
    let (records, profiles) = if matches!(
        name,
        "radiant_acolyte"
            | "radiant_paragon"
            | "radiant_templar"
            | "phosphorus_final"
            | "cake_golem"
    ) {
        let document: crate::content::native_gameplay::CreatureProfilesDocument =
            serde_json::from_slice(include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/creatures/definitions/spell-native-profiles.json"
            )))
            .expect("actual native profiles");
        let definitions = document
            .records
            .into_iter()
            .find(|record| record.profile.target.key == format!("oteryn:creature.{name}"))
            .and_then(|record| record.attack_definitions)
            .expect("actual Summer closure");
        (definitions.records, definitions.profiles)
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
            MovementLocalPosition { x: at.x + 1, ..at },
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
            player_level: 1,
            reference_level_party_lowest: None,
        },
        multiplier_ppm: 1_000_000,
        equipment_protection: None,
        immune: false,
        condition_policy: None,
    };
    let b = r.binding();
    let (fence, stamp) =
        crate::foundation::crystal_timer_fixture(b.runtime_scope(), b.scope_generation())
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
        },
        fence,
        stamp,
    )
}

fn qualify_hazard_monster(runtime: &mut ChannelRuntimeV1, actor: ExactActorRef, key: &str) {
    let policies = crate::content::native_gameplay::callback_source_native_test_policies(
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/creatures/definitions/spell-native-profiles.json"
        )),
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/presentations/definitions/spell-native-profiles.json"
        )),
        runtime.content_pin().server_artifact_digest(),
    )
    .expect("actual native Creature policies");
    runtime
        .install_companion_policies(policies)
        .expect("pinned policy table");
    runtime
        .install_creature_policy(actor, key)
        .expect("actual wild Creature policy");
    assert!(
        runtime
            .companion_snapshot_including_dead(actor)
            .expect("current monster")
            .state
            .master
            .is_none()
    );
}

#[test]
fn all25_native_dodge_cancels_real_source_spells_mana_and_attached_conditions() {
    std::thread::Builder::new()
        .name("all25-native-dodge".into())
        .stack_size(64 << 20)
        .spawn(dodge_source_scenario)
        .expect("thread")
        .join()
        .expect("scenario");
}
fn dodge_source_scenario() {
    for (name, index, current) in [
        ("1st_mate_ratticus", 1, false),
        ("radiant_acolyte", 2, true),
        ("radiant_acolyte", 1, true),
        ("radiant_paragon", 2, true),
        ("an_astral_glyph", 3, false),
        ("abyssador", 1, false),
    ] {
        let at = if name == "abyssador" {
            MovementLocalPosition {
                x: 33502,
                y: 32740,
                floor: 13,
            }
        } else {
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            }
        };
        let (mut runtime, mut states, source, proposal, mut world, fence, fixture_stamp) =
            setup_named_at(name, index, at);
        let session = world.facts.attack.session;
        qualify_hazard_monster(
            &mut runtime,
            proposal.issuer,
            &format!("oteryn:creature.{name}"),
        );
        runtime
            .bind_attacker_lease(
                proposal.target,
                session,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(&test_uuid(0x72)).expect("character"),
                    1,
                )
                .expect("lease"),
            )
            .expect("current charm principal");
        if !source.conditions.is_empty() {
            world.facts.condition_policy = Some(CurrentConditionPolicy {
                immunities: vec![],
                base_speed: 180,
            });
        }
        let stamp = if current {
            runtime.issue_owner_work().expect("current work")
        } else {
            fixture_stamp
        };
        let before = crate::gameplay_transport::actor_spell::observe_vitals(
            &runtime,
            &states,
            proposal.target,
            session,
        )
        .expect("before");
        let mut owner = DamageSpellOwner::default();
        let execute = if source.element() == Some("undefined") {
            DamageSpellOwner::execute_current
        } else if source.area.is_some() {
            DamageSpellOwner::execute_current_elemental_area
        } else {
            DamageSpellOwner::execute_current_target_damage
        };
        let hit = states
            .with_source_charm_test(
                &mut runtime,
                proposal.issuer,
                proposal.target,
                session,
                "dodge",
                |runtime, vitals| {
                    if current {
                        execute(
                            &mut owner,
                            runtime,
                            stamp,
                            vitals,
                            &source,
                            &proposal,
                            &mut world,
                            SemanticTimeMicros::from_micros(2_000_000),
                        )
                    } else {
                        owner.execute(
                            runtime,
                            &fence,
                            stamp,
                            vitals,
                            &source,
                            &proposal,
                            &mut world,
                            SemanticTimeMicros::from_micros(2_000_000),
                        )
                    }
                },
            )
            .expect("actual admitted source cast");
        assert_eq!(hit.targets.len(), 1, "{name}");
        let outcome = hit.targets[0]
            .1
            .as_ref()
            .expect("cancelled cast is admitted, not MissingVitals");
        assert!(
            outcome.damage.is_none() && outcome.mana.is_none() && !outcome.condition_only_applied,
            "{name}/{index}: Dodge must cancel HP, MP and attached conditions, got {outcome:?}"
        );
        assert_eq!(
            crate::gameplay_transport::actor_spell::observe_vitals(
                &runtime,
                &states,
                proposal.target,
                session
            ),
            Some(before),
            "{name}: unchanged vitals and revision"
        );
        // Change selection: retry must retain the already-cancelled occurrence,
        // not interpret current Parry as a fresh incoming attack.
        let replay = states
            .with_source_charm_test(
                &mut runtime,
                proposal.issuer,
                proposal.target,
                session,
                "parry",
                |runtime, vitals| {
                    if current {
                        execute(
                            &mut owner,
                            runtime,
                            stamp,
                            vitals,
                            &source,
                            &proposal,
                            &mut world,
                            SemanticTimeMicros::from_micros(9_000_000),
                        )
                    } else {
                        owner.execute(
                            runtime,
                            &fence,
                            stamp,
                            vitals,
                            &source,
                            &proposal,
                            &mut world,
                            SemanticTimeMicros::from_micros(9_000_000),
                        )
                    }
                },
            )
            .expect("same admitted source occurrence");
        assert_eq!(replay, hit, "{name}: retained cancellation");
        assert_eq!(
            crate::gameplay_transport::actor_spell::observe_vitals(
                &runtime,
                &states,
                proposal.target,
                session
            ),
            Some(before)
        );
    }
}

#[test]
fn all25_native_dodge_cancels_melee_before_block_and_retains_replay() {
    std::thread::Builder::new()
        .name("all25-melee-dodge".into())
        .stack_size(64 << 20)
        .spawn(|| {
            let (mut runtime, mut states, _, proposal, world, _, _) = setup_named_at(
                "1st_mate_ratticus",
                1,
                MovementLocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
            );
            let facts = world.facts.attack;
            let (records, profiles) = native();
            let source = crate::creature_auto_attack::MeleeSource::from_native(
                &Ref {
                    family: ProjectV2Family::Creature,
                    key: "oteryn:creature.1st_mate_ratticus".into(),
                    revision: "definition-r1".into(),
                },
                0,
                &records,
                &profiles,
                runtime.content_pin().server_artifact_digest(),
            )
            .expect("actual melee source");
            qualify_hazard_monster(
                &mut runtime,
                facts.issuer,
                "oteryn:creature.1st_mate_ratticus",
            );
            let character = crate::foundation::CharacterId::decode(facts.session.as_bytes())
                .expect("character");
            runtime
                .bind_attacker_lease(
                    facts.target,
                    facts.session,
                    crate::foundation::CharacterLease::new(character, 1).expect("lease"),
                )
                .expect("principal");
            let mut owner = crate::creature_auto_attack::AutoAttackOwner::default();
            owner
                .set_target(
                    &runtime,
                    proposal.issuer,
                    proposal.target,
                    facts.session,
                    &source,
                    "all25-target:1",
                    SemanticTimeMicros::from_micros(0),
                )
                .expect("target");
            let before = crate::gameplay_transport::actor_spell::observe_vitals(
                &runtime,
                &states,
                facts.target,
                facts.session,
            );
            let root =
                GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
            let revisions = || {
                crate::ability::RevisionSet::new(
                    "rules-r1",
                    "content-r1",
                    "policy-r1",
                    "definition-r1",
                    "sim-r1",
                )
                .expect("revisions")
            };
            let (fence, stamp) = crate::foundation::crystal_timer_fixture(
                runtime.binding().runtime_scope(),
                runtime.binding().scope_generation(),
            )
            .expect("owner fence");
            let hit = states
                .with_source_charm_test(
                    &mut runtime,
                    facts.issuer,
                    facts.target,
                    facts.session,
                    "dodge",
                    |runtime, vitals| {
                        owner.swing(
                            runtime,
                            vitals,
                            &source,
                            facts,
                            1,
                            revisions(),
                            &root,
                            SemanticTimeMicros::from_micros(2_000_000),
                            &fence,
                            stamp,
                        )
                    },
                )
                .expect("admitted cancelled native swing");
            assert!(hit.chance_passed && hit.requested > 0);
            assert!(
                hit.damage.is_none(),
                "Dodge must not create damage or death"
            );
            assert_eq!(hit.blocked, hit.requested);
            assert_eq!(
                crate::gameplay_transport::actor_spell::observe_vitals(
                    &runtime,
                    &states,
                    facts.target,
                    facts.session
                ),
                before
            );
            let replay = states
                .with_source_charm_test(
                    &mut runtime,
                    facts.issuer,
                    facts.target,
                    facts.session,
                    "parry",
                    |runtime, vitals| {
                        owner.swing(
                            runtime,
                            vitals,
                            &source,
                            facts,
                            1,
                            revisions(),
                            &root,
                            SemanticTimeMicros::from_micros(9_000_000),
                            &fence,
                            stamp,
                        )
                    },
                )
                .expect("native swing replay");
            assert_eq!(replay, hit);
            assert_eq!(
                crate::gameplay_transport::actor_spell::observe_vitals(
                    &runtime,
                    &states,
                    facts.target,
                    facts.session
                ),
                before
            );
        })
        .expect("thread")
        .join()
        .expect("scenario");
}
