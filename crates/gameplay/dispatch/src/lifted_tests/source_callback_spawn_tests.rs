#![allow(clippy::expect_used)]
use crate::content::{ProjectV2DefinitionRef as Ref, ProjectV2Draft, ProjectV2Family as Family};
use crate::creature_auto_attack::AttackError;
use crate::creature_auto_attack::ProfileAbilityProposal;
use crate::foundation::DotElement;
use crate::foundation::owner_timer::SemanticTimeMicros;
use crate::foundation::owner_timer::VirtualOwnerClock;
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, MovementLocalPosition, RuntimeWorkStamp,
    ScopeOwnershipGeneration, ScopeRuntimeFence,
};
use crate::gameplay_transport::actor_spell::tests::runtime_with_capacity;
use crate::source_callback_spawn::*;
use oteryn_gameplay_foundation::foundation::profile_schedule::ScheduleList;
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
use {
    crate::foundation::{
        ActorConditionTransition, ApplicationFacts, ConditionDefinition, ConditionSource,
        ConditionSourceKind, ConditionValues,
    },
    oteryn_gameplay_content::content::creature_policy::CompiledCreaturePolicies,
};

fn runtime_with_player(
    tag: u8,
) -> (
    ChannelRuntimeV1,
    ExactActorRef,
    crate::foundation::GameSessionId,
) {
    let (mut runtime, player, session) = runtime_with_capacity(tag, 32);
    runtime
        .initialize_source_pinned_lab_player_position(
            player,
            session,
            MovementLocalPosition {
                x: 90,
                y: 90,
                floor: 7,
            },
        )
        .expect("qualified fixture");
    (runtime, player, session)
}
fn draft(r: &ChannelRuntimeV1) -> ProjectV2Draft {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../apps/game-server/src/source_callback_spawn_native_fixture.json"
    ))
    .expect("qualified fixture");
    ProjectV2Draft {
        core: crate::content::ProjectDraft {
            project_revision: "g4-npc-wave-a-r9".into(),
            package_key: "oteryn:content.world-project".into(),
            semantic_schema_version: "reference-schema-v1".into(),
            licensing_metadata: "PENDING".into(),
            world_id: hex(*r.binding().world_id().as_bytes()),
            coordinate_frame: "global-target-2026-09-27".into(),
            records: serde_json::from_value(v["records"].clone()).expect("qualified fixture"),
            imports: vec![],
            metadata: vec![],
        },
        state: crate::content::ProjectV2State {
            declarations: serde_json::from_value(v["declarations"].clone())
                .expect("qualified fixture"),
            authoring_profiles: serde_json::from_value(v["authoring_profiles"].clone())
                .expect("qualified fixture"),
            source_identity_bindings: serde_json::from_value(v["source_identity_bindings"].clone())
                .expect("qualified fixture"),
            sources: serde_json::from_value(v["sources"].clone()).expect("qualified fixture"),
            ..Default::default()
        },
    }
}
fn policies(r: &ChannelRuntimeV1) -> CompiledCreaturePolicies {
    // Cache only the authentic immutable catalogue. Each fixture runtime has its
    // own qualified outer artifact; rebinding must never be cached across pins.
    static BASE: std::sync::OnceLock<CompiledCreaturePolicies> = std::sync::OnceLock::new();
    let mut policies = BASE
        .get_or_init(|| {
            let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../content/spells.manifest.json");
            crate::content::native_gameplay::callback_native_policy_fixture_at(&manifest)
                .expect("actual manifest-pinned native policy fixture")
                .policies()
                .clone()
        })
        .clone();
    policies
        .bind_qualified_outer_artifact(r.content_pin().server_artifact_digest())
        .expect("actual native policies preserve sealed metadata under fixture outer pin");
    policies
}
fn admit_source_parent(
    runtime: &mut ChannelRuntimeV1,
    at: MovementLocalPosition,
    key: &str,
) -> ExactActorRef {
    let maximum_health = runtime
        .companion_policy(key)
        .expect("actual native parent policy")
        .maximum_health;
    let actor = runtime
        .admit_source_pinned_lab_creature(at, key, maximum_health)
        .expect("actual native parent physical slot");
    runtime
        .install_creature_policy(actor, key)
        .expect("actual native parent metadata and policy");
    actor
}

fn fence(r: &ChannelRuntimeV1, g: u64) -> (ScopeRuntimeFence, RuntimeWorkStamp) {
    let b = r.binding();
    crate::foundation::crystal_timer_fixture(
        b.runtime_scope(),
        ScopeOwnershipGeneration::new(g).expect("qualified fixture"),
    )
    .expect("qualified fixture")
}
fn plan(actor: ExactActorRef, s: &SourceCallbackSpawn, n: u64) -> ProfileAbilityProposal {
    let atom = format!("actor:{}", hex(actor.placement_identity()));
    let list = if s.list == ScheduleList::Attack {
        "attack"
    } else {
        "defence"
    };
    ProfileAbilityProposal {
        issuer: actor,
        target: actor,
        ability: s.parent.clone(),
        list: s.list,
        entry_index: s.index,
        magnitude: s.magnitude,
        range_tiles: s.range,
        occurrence: crate::ability::AbilityOccurrence::new(
            &format!(
                "ai-profile:{}:{n}:{list}:{}",
                hex(actor.placement_identity()),
                s.index
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
        .expect("qualified fixture"),
        intent: crate::ability::AiAbilityAdapter::normalize(&atom, &[&atom])
            .expect("qualified fixture"),
    }
}
struct Map {
    present: bool,
    allowed: bool,
    anchor: bool,
    next: i32,
}
impl SourceSpawnFacts for Map {
    fn source_spawn_position(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: &SourceSpawnPlacement,
        _: u64,
        _: usize,
        _: RuntimeWorkStamp,
    ) -> Option<MovementLocalPosition> {
        if !self.present {
            return None;
        }
        self.next += 1;
        Some(MovementLocalPosition {
            x: 100 + self.next,
            y: 100,
            floor: 7,
        })
    }
    fn source_spawn_tile_allowed(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: MovementLocalPosition,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        self.present.then_some(self.allowed)
    }
    fn source_spawn_anchor_contains(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: &serde_json::Value,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        self.present.then_some(self.anchor)
    }
}
fn poison(r: &mut ChannelRuntimeV1, a: ExactActorRef) {
    let root = GameplayDecisionRoot::from_bytes(r.content_pin().server_artifact_digest());
    let definition = ConditionDefinition::new(
        "fixture.source-soulcatcher.poison",
        1,
        ConditionValues::DamageOverTime {
            element: DotElement::Poison,
            total_min: 10,
            total_max: 10,
            per_tick: 1,
            interval_ms: 2000,
            delayed: true,
        },
    )
    .expect("qualified fixture");
    let facts = ApplicationFacts {
        now: 0,
        decision_root: &root,
        occurrence: DecisionOccurrenceId::from_bytes([0x35; 16]),
        target_is_player: false,
        target_reentry_protected: false,
        source_reentry_protected: false,
        mana_shield_capacity: 0,
        base_speed: 220,
    };
    let plan = r
        .prepare_actor_condition(
            a,
            None,
            ActorConditionTransition::Apply {
                definition: &definition,
                source: ConditionSource {
                    actor: a,
                    session: None,
                    kind: ConditionSourceKind::Creature,
                },
                immunities: &[],
                facts,
            },
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
        )
        .expect("qualified fixture");
    r.commit_actor_condition(
        &plan,
        oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
    )
    .expect("qualified fixture");
}
#[test]
fn sixteen_exact_source_spawns_actual_policy_health_origin_delay_replay_and_cleanup() {
    for case in fixture().expect("qualified fixture").cases {
        let (mut r, _, _) = runtime_with_player(0x86);
        r.install_companion_policies(policies(&r))
            .expect("qualified fixture");
        let parent = admit_source_parent(
            &mut r,
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
            &case.caster,
        );
        let d = draft(&r);
        let list = if case.phase == "attacks" {
            ScheduleList::Attack
        } else {
            ScheduleList::Defence
        };
        let source = SourceCallbackSpawn::qualify(
            &d,
            &Ref {
                family: Family::Creature,
                key: case.caster.clone(),
                revision: "definition-r1".into(),
            },
            case.index,
            list,
            r.content_pin().server_artifact_digest(),
        )
        .expect("qualified fixture")
        .expect("qualified fixture");
        if source.kind == Kind::Soulcatcher {
            poison(&mut r, parent)
        }
        let (mut f, stamp) = fence(&r, 1);
        let mut owner = CallbackSpawnOwner::new(&r, &f).expect("qualified fixture");
        let proposal = plan(parent, &source, 0);
        let mut forged = proposal.clone();
        forged.intent =
            crate::ability::AiAbilityAdapter::normalize("actor:forged", &["actor:forged"])
                .expect("qualified fixture");
        assert!(matches!(
            owner.schedule(
                &r,
                &f,
                stamp,
                &source,
                &forged,
                SemanticTimeMicros::from_micros(0)
            ),
            Err(AttackError::InvalidPlan)
        ));
        assert!(owner.pending.is_empty());
        assert!(owner.memos.is_empty());
        let foreign = runtime_with_player(0x89).1;
        let mut forged = proposal.clone();
        forged.target = foreign;
        assert!(
            owner
                .schedule(
                    &r,
                    &f,
                    stamp,
                    &source,
                    &forged,
                    SemanticTimeMicros::from_micros(0)
                )
                .is_err()
        );
        assert!(owner.pending.is_empty());
        assert!(owner.memos.is_empty());
        let now = SemanticTimeMicros::from_micros(0);
        assert!(
            owner
                .schedule(&r, &f, stamp, &source, &proposal, now)
                .expect("qualified fixture")
        );
        assert!(
            !owner
                .schedule(&r, &f, stamp, &source, &proposal, now)
                .expect("qualified fixture")
        );
        let mut map = Map {
            present: true,
            allowed: true,
            anchor: true,
            next: 0,
        };
        if source.kind == Kind::GeneratorRelease {
            assert!(
                owner
                    .drain(
                        &mut r,
                        &mut f,
                        stamp,
                        &mut map,
                        &VirtualOwnerClock::new(SemanticTimeMicros::from_micros(13999000))
                    )
                    .expect("qualified fixture")
                    .is_empty()
            );
        }
        let receipt = owner
            .drain(
                &mut r,
                &mut f,
                stamp,
                &mut map,
                &VirtualOwnerClock::new(SemanticTimeMicros::from_micros(14000000)),
            )
            .expect("qualified fixture")
            .remove(0)
            .expect("qualified fixture");
        assert!(receipt.requested > 0, "{}", case.caster);
        assert_eq!(receipt.children.len(), receipt.requested as usize);
        let lines = owner.take_source_speech();
        let expected = match source.kind {
            Kind::Gaz => receipt.requested as usize,
            Kind::GeneratorRelease => 1,
            Kind::Tenebris | Kind::Maxxen => {
                receipt.children.iter().filter(|c| c.result.is_ok()).count()
            }
            _ => 0,
        };
        assert_eq!(lines.len(), expected, "{}", case.caster);
        assert!(owner.take_source_speech().is_empty());
        assert!(owner.take_speech_errors().is_empty());
        for child in &receipt.children {
            let actor = child.result.as_ref().expect("qualified fixture");
            let cp = source
                .children
                .iter()
                .find(|c| c.creature == child.creature)
                .expect("qualified fixture");
            assert_eq!(
                r.read_source_creature_health(*actor, &cp.creature.key, cp.health as u64)
                    .expect("qualified fixture"),
                cp.health as u64
            );
            assert!(r.companion_snapshot(*actor).is_ok());
            if child.intrinsic {
                assert!(
                    matches!(r.native_summon_origin(*actor).expect("qualified fixture"),Some(crate::foundation::NativeSummonOrigin::SourceCallback{ability,..})if ability.as_ref()==source.parent.key)
                );
            } else {
                assert!(
                    r.native_summon_role(*actor)
                        .expect("qualified fixture")
                        .is_none()
                );
            }
        }
        assert!(
            owner
                .drain(
                    &mut r,
                    &mut f,
                    stamp,
                    &mut map,
                    &VirtualOwnerClock::new(SemanticTimeMicros::from_micros(15000000))
                )
                .expect("qualified fixture")
                .is_empty()
        );
        if source.kind == Kind::GeneratorRelease {
            assert_eq!(receipt.remove, Some(Ok(())));
            assert!(!r.contains_live_creature(parent));
        } else {
            assert!(
                !owner
                    .schedule(
                        &r,
                        &f,
                        stamp,
                        &source,
                        &proposal,
                        SemanticTimeMicros::from_micros(15000000)
                    )
                    .expect("qualified fixture")
            );
            r.remove_native_encounter_subject(&f, stamp, parent, &source.creature, source.content)
                .expect("qualified fixture");
        }
        for child in receipt.children {
            let actor = child.result.expect("qualified fixture");
            assert_eq!(r.contains_live_creature(actor), !child.intrinsic);
        }
    }
}
#[test]
fn all_sixteen_sources_missing_map_stale_fence_and_provenance_never_spawn() {
    for case in fixture().expect("qualified fixture").cases {
        let (mut r, _, _) = runtime_with_player(0x87);
        r.install_companion_policies(policies(&r))
            .expect("qualified fixture");
        let parent = admit_source_parent(
            &mut r,
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
            &case.caster,
        );
        let mut d = draft(&r);
        let cr = Ref {
            family: Family::Creature,
            key: case.caster.clone(),
            revision: "definition-r1".into(),
        };
        let list = if case.phase == "attacks" {
            ScheduleList::Attack
        } else {
            ScheduleList::Defence
        };
        let source = SourceCallbackSpawn::qualify(
            &d,
            &cr,
            case.index,
            list,
            r.content_pin().server_artifact_digest(),
        )
        .expect("qualified fixture")
        .expect("qualified fixture");
        if source.kind == Kind::Soulcatcher {
            poison(&mut r, parent)
        }
        let (mut f, stamp) = fence(&r, 1);
        let (wrong, wrong_stamp) = fence(&r, 2);
        let mut owner = CallbackSpawnOwner::new(&r, &f).expect("qualified fixture");
        let proposal = plan(parent, &source, 0);
        assert!(matches!(
            owner.schedule(
                &r,
                &wrong,
                wrong_stamp,
                &source,
                &proposal,
                SemanticTimeMicros::from_micros(0)
            ),
            Err(AttackError::StaleOwner)
        ));
        owner
            .schedule(
                &r,
                &f,
                stamp,
                &source,
                &proposal,
                SemanticTimeMicros::from_micros(0),
            )
            .expect("qualified fixture");
        let mut map = Map {
            present: false,
            allowed: true,
            anchor: true,
            next: 0,
        };
        let result = owner
            .drain(
                &mut r,
                &mut f,
                stamp,
                &mut map,
                &VirtualOwnerClock::new(SemanticTimeMicros::from_micros(14000000)),
            )
            .expect("qualified fixture")
            .remove(0);
        assert!(
            matches!(result, Err(AttackError::MissingCombatFacts)),
            "{} {:?}",
            case.caster,
            result
        );
        assert_eq!(
            r.current_live_creature_candidates(64)
                .expect("qualified fixture")
                .len(),
            1
        );
        d.state
            .source_identity_bindings
            .iter_mut()
            .find(|b| b.target.key == case.encounter)
            .expect("qualified fixture")
            .source_revision = "wrong".into();
        assert!(SourceCallbackSpawn::qualify(&d, &cr, case.index, list, source.content).is_err());
        r.remove_test_actor(parent).expect("qualified fixture");
        assert!(matches!(
            owner.schedule(
                &r,
                &f,
                stamp,
                &source,
                &proposal,
                SemanticTimeMicros::from_micros(15000000)
            ),
            Err(AttackError::StaleIssuer)
        ));
    }
}

fn source_spawn_fixture(caster: &str) -> (ChannelRuntimeV1, ExactActorRef, SourceCallbackSpawn) {
    let (mut runtime, _, _) = runtime_with_player(0xa6);
    runtime
        .install_companion_policies(policies(&runtime))
        .expect("actual child policies");
    let case = fixture()
        .expect("reviewed sources")
        .cases
        .into_iter()
        .find(|case| case.caster == caster)
        .expect("Gaz source");
    let actor = admit_source_parent(
        &mut runtime,
        MovementLocalPosition {
            x: 100,
            y: 100,
            floor: 7,
        },
        &case.caster,
    );
    let source = SourceCallbackSpawn::qualify(
        &draft(&runtime),
        &Ref {
            family: Family::Creature,
            key: case.caster,
            revision: "definition-r1".into(),
        },
        case.index,
        if case.phase == "attacks" {
            ScheduleList::Attack
        } else {
            ScheduleList::Defence
        },
        runtime.content_pin().server_artifact_digest(),
    )
    .expect("source qualification")
    .expect("Gaz source descriptor");
    (runtime, actor, source)
}

fn gaz_source_fixture() -> (ChannelRuntimeV1, ExactActorRef, SourceCallbackSpawn) {
    source_spawn_fixture("oteryn:creature.gaz_haragoth")
}

#[test]
fn srx3_gaz_source_minions_remain_masterless_when_sum_is_unbound() {
    let (mut runtime, actor, source) = gaz_source_fixture();
    let (mut fence, stamp) = fence(&runtime, 1);
    let mut owner = CallbackSpawnOwner::new(&runtime, &fence).expect("owner");
    owner
        .schedule(
            &runtime,
            &fence,
            stamp,
            &source,
            &plan(actor, &source, 0),
            SemanticTimeMicros::from_micros(0),
        )
        .expect("source cast");
    let mut map = Map {
        present: true,
        allowed: true,
        anchor: true,
        next: 0,
    };
    let receipt = owner
        .drain(
            &mut runtime,
            &mut fence,
            stamp,
            &mut map,
            &VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0)),
        )
        .expect("source drain")
        .remove(0)
        .expect("actual source children");
    assert_eq!(receipt.requested, 2);
    for child in receipt.children {
        let actor = child.result.expect("actual admitted minion");
        assert!(
            !child.intrinsic,
            "pinned Gaz Lua calls new_minion:setSummon(nil), not caster:setSummon(new_minion)"
        );
        assert!(
            runtime
                .native_summon_role(actor)
                .expect("actual role")
                .is_none()
        );
    }
}

#[test]
fn srx3_gaz_source_counts_named_nearby_minions_instead_of_only_own_summons() {
    let (mut runtime, actor, source) = gaz_source_fixture();
    for x in 101..108 {
        runtime
            .admit_source_pinned_lab_creature(
                MovementLocalPosition {
                    x,
                    y: 100,
                    floor: 7,
                },
                "oteryn:creature.minion_of_gaz_haragoth",
                5500,
            )
            .expect("real independent nearby minion");
    }
    assert_eq!(runtime.native_summon_count(actor, None), 0);
    let (mut fence, stamp) = fence(&runtime, 1);
    let mut owner = CallbackSpawnOwner::new(&runtime, &fence).expect("owner");
    owner
        .schedule(
            &runtime,
            &fence,
            stamp,
            &source,
            &plan(actor, &source, 0),
            SemanticTimeMicros::from_micros(0),
        )
        .expect("source cast");
    let mut map = Map {
        present: true,
        allowed: true,
        anchor: true,
        next: 20,
    };
    let receipt = owner
        .drain(
            &mut runtime,
            &mut fence,
            stamp,
            &mut map,
            &VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0)),
        )
        .expect("source drain")
        .remove(0)
        .expect("source count outcome");
    assert_eq!(
        receipt.requested, 0,
        "source getSpectators ±25 counts all7namedminions before the seven-minion cap"
    );
}

#[test]
fn srx3_gaz_source_says_once_per_minion_attempt_including_failed_creation() {
    for allowed in [true, false] {
        let (mut runtime, actor, source) = gaz_source_fixture();
        let (mut fence, stamp) = fence(&runtime, 1);
        let mut owner = CallbackSpawnOwner::new(&runtime, &fence).expect("owner");
        owner
            .schedule(
                &runtime,
                &fence,
                stamp,
                &source,
                &plan(actor, &source, 0),
                SemanticTimeMicros::from_micros(0),
            )
            .expect("source cast");
        let mut map = Map {
            present: true,
            allowed,
            anchor: true,
            next: 0,
        };
        let receipt = owner
            .drain(
                &mut runtime,
                &mut fence,
                stamp,
                &mut map,
                &VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0)),
            )
            .expect("source drain")
            .remove(0)
            .expect("source attempts");
        assert_eq!(receipt.requested, 2);
        assert_eq!(
            owner.take_source_speech().len(),
            2,
            "Gaz Lua says inside each attempt, before testing monster creation success"
        );
    }
}

#[test]
fn srx3_glooth_generator_retires_and_speaks_after_failed_energy_pulse_creation() {
    let (mut runtime, actor, source) = source_spawn_fixture("oteryn:creature.glooth_generator");
    let (mut fence, stamp) = fence(&runtime, 1);
    let mut owner = CallbackSpawnOwner::new(&runtime, &fence).expect("owner");
    owner
        .schedule(
            &runtime,
            &fence,
            stamp,
            &source,
            &plan(actor, &source, 0),
            SemanticTimeMicros::from_micros(0),
        )
        .expect("source cast");
    let mut map = Map {
        present: true,
        allowed: false,
        anchor: true,
        next: 0,
    };
    let receipt = owner
        .drain(
            &mut runtime,
            &mut fence,
            stamp,
            &mut map,
            &VirtualOwnerClock::new(SemanticTimeMicros::from_micros(14_000_000)),
        )
        .expect("source drain")
        .remove(0)
        .expect("source attempt");
    assert_eq!(receipt.requested, 1);
    assert!(receipt.children[0].result.is_err());
    assert!(
        receipt.remove.is_some_and(|result| result.is_ok()),
        "source removal is unconditional after the 14s creation attempt"
    );
    assert!(!runtime.contains_live_creature(actor));
    assert_eq!(
        owner.take_source_speech().len(),
        1,
        "source speech must be captured before unconditional removal"
    );
}

#[test]
fn srx3_gaz_failed_chance_branch_retains_source_desired_count_across_incarnations() {
    let (mut runtime, actor, source) = gaz_source_fixture();
    for x in 101..103 {
        runtime
            .admit_source_pinned_lab_creature(
                MovementLocalPosition {
                    x,
                    y: 100,
                    floor: 7,
                },
                "oteryn:creature.minion_of_gaz_haragoth",
                5500,
            )
            .expect("actual independent minion");
    }
    let (mut fence, stamp) = fence(&runtime, 1);
    let mut owner = CallbackSpawnOwner::new(&runtime, &fence).expect("source owner");
    let mut map = Map {
        present: true,
        allowed: false,
        anchor: true,
        next: 30,
    };
    let mut selected = None;
    for sequence in 0..128 {
        let now = SemanticTimeMicros::from_micros(sequence * 2_000_000);
        owner
            .schedule(
                &runtime,
                &fence,
                stamp,
                &source,
                &plan(actor, &source, sequence),
                now,
            )
            .expect("actual source occurrence");
        let receipt = owner
            .drain(
                &mut runtime,
                &mut fence,
                stamp,
                &mut map,
                &VirtualOwnerClock::new(now),
            )
            .expect("source timer")
            .remove(0)
            .expect("source result");
        if receipt.requested == 1 {
            assert!(receipt.children.iter().all(|child| child.result.is_err()));
            selected = Some(sequence);
            break;
        }
    }
    let sequence = selected.expect("fixed deterministic fixture selects source chance branch");
    // The global source desired count increments even though createMonster failed.
    // A replacement boss in the SAME physical runtime must not reset it to two.
    let parent_health = runtime
        .companion_snapshot(actor)
        .expect("actual source health")
        .health;
    let replacement = runtime
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: 100,
                y: 101,
                floor: 7,
            },
            &source.creature,
            parent_health,
        )
        .expect("new exact source incarnation");
    runtime
        .install_creature_policy(replacement, &source.creature)
        .expect("replacement retains actual source policy and metadata");
    for replacement_sequence in 0..16 {
        let now =
            SemanticTimeMicros::from_micros((sequence + replacement_sequence + 1) * 2_000_000);
        owner
            .schedule(
                &runtime,
                &fence,
                stamp,
                &source,
                &plan(replacement, &source, replacement_sequence),
                now,
            )
            .expect("replacement source occurrence");
        let receipt = owner
            .drain(
                &mut runtime,
                &mut fence,
                stamp,
                &mut map,
                &VirtualOwnerClock::new(now),
            )
            .expect("source timer")
            .remove(0)
            .expect("retained desired count");
        assert_eq!(
            receipt.requested, 1,
            "two existing minions must refill the retained desired count of three"
        );
    }
}

#[test]
fn srx3_source_spawn_reselects_cached_candidates_against_each_prior_child_admission() {
    struct CurrentOccupancy;
    impl SourceSpawnFacts for CurrentOccupancy {
        fn source_spawn_position(
            &mut self,
            runtime: &ChannelRuntimeV1,
            parent: ExactActorRef,
            _: &SourceSpawnPlacement,
            _: u64,
            _: usize,
            _: RuntimeWorkStamp,
        ) -> Option<MovementLocalPosition> {
            let origin = runtime.read_actor_position(parent).ok()?.position();
            // A prepared, stable source candidate order. Only current physical
            // occupancy changes between the two ordinary source creations.
            (1..=3)
                .map(|dx| MovementLocalPosition {
                    x: origin.x + dx,
                    ..origin
                })
                .find(|at| runtime.position_occupied_by_other(parent, *at) == Ok(false))
        }
        fn source_spawn_tile_allowed(
            &mut self,
            runtime: &ChannelRuntimeV1,
            parent: ExactActorRef,
            at: MovementLocalPosition,
            _: RuntimeWorkStamp,
        ) -> Option<bool> {
            runtime
                .position_occupied_by_other(parent, at)
                .ok()
                .map(|occupied| !occupied)
        }
        fn source_spawn_anchor_contains(
            &mut self,
            _: &ChannelRuntimeV1,
            _: ExactActorRef,
            _: &serde_json::Value,
            _: RuntimeWorkStamp,
        ) -> Option<bool> {
            None
        }
    }
    let (mut runtime, actor, source) = gaz_source_fixture();
    let (mut fence, stamp) = fence(&runtime, 1);
    let mut owner = CallbackSpawnOwner::new(&runtime, &fence).expect("actual owner");
    let now = SemanticTimeMicros::from_micros(0);
    owner
        .schedule(
            &runtime,
            &fence,
            stamp,
            &source,
            &plan(actor, &source, 0),
            now,
        )
        .expect("actual source occurrence");
    let receipt = owner
        .drain(
            &mut runtime,
            &mut fence,
            stamp,
            &mut CurrentOccupancy,
            &VirtualOwnerClock::new(now),
        )
        .expect("source drain")
        .remove(0)
        .expect("source result");
    assert_eq!(receipt.requested, 2);
    let children = receipt
        .children
        .iter()
        .map(|child| {
            *child.result.as_ref().expect(
                "each source creation must choose against the immediately current occupancy",
            )
        })
        .collect::<Vec<_>>();
    assert_ne!(
        runtime
            .read_actor_position(children[0])
            .expect("first native child")
            .position(),
        runtime
            .read_actor_position(children[1])
            .expect("second native child")
            .position()
    );
}

#[test]
fn srx3_generator_source_forced_center_creates_energy_pulse_before_retiring_occupying_caster()
-> Result<(), Box<dyn std::error::Error>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            struct ExistingCenter<'a> {
                native: &'a crate::content::native_gameplay::NativeGameplayState,
            }
            impl SourceSpawnFacts for ExistingCenter<'_> {
                fn current_native(
                    &self,
                ) -> Option<&crate::content::native_gameplay::NativeGameplayState> {
                    Some(self.native)
                }

                fn source_spawn_position(
                    &mut self,
                    runtime: &ChannelRuntimeV1,
                    parent: ExactActorRef,
                    placement: &SourceSpawnPlacement,
                    _: u64,
                    _: usize,
                    _: RuntimeWorkStamp,
                ) -> Option<MovementLocalPosition> {
                    assert_eq!(
                        self.native.source_digest(),
                        runtime.content_pin().server_artifact_digest()
                    );
                    assert!(matches!(placement, SourceSpawnPlacement::Subject));
                    runtime
                        .read_actor_position(parent)
                        .ok()
                        .map(|snapshot| snapshot.position())
                }
                fn source_spawn_tile_allowed(
                    &mut self,
                    _: &ChannelRuntimeV1,
                    _: ExactActorRef,
                    _: MovementLocalPosition,
                    _: RuntimeWorkStamp,
                ) -> Option<bool> {
                    Some(true)
                }
                fn source_spawn_anchor_contains(
                    &mut self,
                    _: &ChannelRuntimeV1,
                    _: ExactActorRef,
                    _: &serde_json::Value,
                    _: RuntimeWorkStamp,
                ) -> Option<bool> {
                    None
                }
            }
            let world =
                crate::content::native_gameplay_test_world_id().map_err(|e| format!("{e:?}"))?;
            let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../content/spells.manifest.json");
            let native = crate::content::native_gameplay::retained_callback_native_fixture_at(
                world, &manifest,
            );
            let uuid = |tag| {
                let mut bytes = [tag; 16];
                bytes[6] = 0x70;
                bytes[8] = 0x80;
                bytes
            };
            let mut runtime = ChannelRuntimeV1::from_committed_assignment(
                world,
                crate::foundation::ChannelId::decode(&uuid(0x48)).map_err(|e| format!("{e:?}"))?,
                crate::foundation::NodeId::decode(&uuid(0x49)).map_err(|e| format!("{e:?}"))?,
                1,
                1,
                1,
                "runtime-scope-assignment:1",
                8,
                crate::foundation::ChannelContentPin::from_activation(
                    world,
                    1,
                    native.source_digest(),
                    [2; 32],
                    [3; 32],
                    [4; 32],
                    (100, 100, 7),
                ),
            )
            .map_err(|e| format!("{e:?}"))?;
            runtime
                .install_companion_policies(native.policies().clone())
                .map_err(|e| format!("{e:?}"))?;
            let record = native
                .creature_profiles()
                .records
                .iter()
                .find(|r| r.profile.target.key == "oteryn:creature.glooth_generator")
                .ok_or("actual native Generator")?;
            let source = SourceCallbackSpawn::qualify_native(
                &native,
                &record.profile.target,
                0,
                ScheduleList::Defence,
            )
            .map_err(|e| format!("{e:?}"))?
            .ok_or("actual native callback")?;
            let child = native
                .creature_profiles()
                .records
                .iter()
                .find(|r| r.profile.target.key == "oteryn:creature.energy_pulse")
                .ok_or("actual native Energy Pulse")?;
            let token = native
                .callback_spawn_source(
                    &record.profile.target,
                    0,
                    crate::content::native_monster_callbacks::NativeCallbackList::Defence,
                    &child.profile.target,
                )
                .map_err(|e| format!("{e:?}"))?
                .ok_or("actual sealed force token")?;
            assert!(token.extended() && token.forced());
            let policy = runtime
                .companion_policy(&record.profile.target.key)
                .map_err(|e| format!("{e:?}"))?;
            let actor = runtime
                .admit_source_pinned_lab_creature(
                    MovementLocalPosition {
                        x: 100,
                        y: 100,
                        floor: 7,
                    },
                    &record.profile.target.key,
                    policy.maximum_health,
                )
                .map_err(|e| format!("{e:?}"))?;
            runtime
                .install_creature_policy(actor, &record.profile.target.key)
                .map_err(|e| format!("{e:?}"))?;
            let before = runtime
                .read_actor_position(actor)
                .expect("actual source center")
                .position();
            let (mut fence, stamp) = fence(&runtime, 1);
            let mut owner = CallbackSpawnOwner::new(&runtime, &fence).expect("owner");
            owner
                .schedule(
                    &runtime,
                    &fence,
                    stamp,
                    &source,
                    &plan(actor, &source, 0),
                    SemanticTimeMicros::from_micros(0),
                )
                .expect("source closure");
            let receipt = owner
                .drain(
                    &mut runtime,
                    &mut fence,
                    stamp,
                    &mut ExistingCenter { native: &native },
                    &VirtualOwnerClock::new(SemanticTimeMicros::from_micros(14_000_000)),
                )
                .expect("actual timer")
                .remove(0)
                .expect("source create/remove result");
            let pulse = *receipt.children[0].result.as_ref().expect(
                "source createMonster extended=true force=true must stack on its live generator",
            );
            assert_eq!(
                runtime
                    .read_actor_position(pulse)
                    .expect("actual Energy Pulse")
                    .position(),
                before
            );
            assert_eq!(
                runtime.read_source_creature_health(pulse, "oteryn:creature.energy_pulse", 5000),
                Ok(5000)
            );
            assert!(!runtime.contains_live_creature(actor));
            assert!(receipt.remove.is_some_and(|result| result.is_ok()));
            Ok(())
        })?
        .join()
        .map_err(|_| "actual source force test thread panicked")??;
    Ok(())
}

#[test]
fn srx3_mazoran_source_spawn_waits_three_seconds_before_resolving_current_creature() {
    // Canary47df mazoran_fire.lua:956-958 schedules lavaGrounds(cid) after3s.
    let (runtime, actor, source) = source_spawn_fixture("oteryn:creature.mazoran");
    let (mut fence, stamp) = fence(&runtime, 1);
    let mut owner = CallbackSpawnOwner::new(&runtime, &fence).expect("existing source timer owner");
    let now = SemanticTimeMicros::from_micros(0);
    owner
        .schedule(
            &runtime,
            &fence,
            stamp,
            &source,
            &plan(actor, &source, 0),
            now,
        )
        .expect("actual source proposal");
    assert_eq!(
        owner.next_due_time(),
        Some(3_000_000),
        "source delayed Creature(cid) lookup must not run synchronously"
    );
    assert_eq!(owner.pending_due(&owner.pending[0]), Some(3_000_000));
    fence.invalidate();
}

#[test]
fn srx3_source_spawn_loops_return_after_first_failed_create_monster() {
    // Canary47df: mazoran_fire.lua:920-922; plagirath_summon.lua:9-11;
    // razzagorn_summon.lua:6-8; shulgrax_summon.lua:8-10,15-17.
    for caster in ["mazoran", "plagirath", "razzagorn", "shulgrax"] {
        let (mut runtime, actor, source) =
            source_spawn_fixture(&format!("oteryn:creature.{caster}"));
        let (mut fence, stamp) = fence(&runtime, 1);
        let mut owner = CallbackSpawnOwner::new(&runtime, &fence).expect("actual source owner");
        owner
            .schedule(
                &runtime,
                &fence,
                stamp,
                &source,
                &plan(actor, &source, 0),
                SemanticTimeMicros::from_micros(0),
            )
            .expect("actual source proposal");
        let mut map = Map {
            present: true,
            allowed: false,
            anchor: true,
            next: 0,
        };
        let results = owner
            .drain(
                &mut runtime,
                &mut fence,
                stamp,
                &mut map,
                &VirtualOwnerClock::new(SemanticTimeMicros::from_micros(3_000_000)),
            )
            .expect("source timer");
        let receipt = results
            .into_iter()
            .next()
            .expect("one source closure")
            .expect("source failure receipt");
        assert_eq!(
            receipt.children.len(),
            1,
            "{caster}: source returns after its first nil createMonster"
        );
        assert_eq!(
            map.next, 1,
            "{caster}: later loop iterations must not evaluate positions after source return"
        );
        assert!(receipt.children[0].result.is_err());
    }
}

#[test]
fn srx3_heart_source_counts_creation_attempts_even_when_every_creation_fails() {
    for caster in [
        "the_destruction",
        "the_hunger",
        "the_rage",
        "world_devourer",
    ] {
        let (mut runtime, actor, source) =
            source_spawn_fixture(&format!("oteryn:creature.{caster}"));
        let (mut fence, stamp) = fence(&runtime, 1);
        let mut owner = CallbackSpawnOwner::new(&runtime, &fence).expect("actual source owner");
        for sequence in 0..4 {
            let now = SemanticTimeMicros::from_micros(sequence * 15_000_000);
            owner
                .schedule(
                    &runtime,
                    &fence,
                    stamp,
                    &source,
                    &plan(actor, &source, sequence),
                    now,
                )
                .expect("actual source occurrence");
            let mut map = Map {
                present: true,
                allowed: false,
                anchor: true,
                next: 0,
            };
            let receipt = owner
                .drain(
                    &mut runtime,
                    &mut fence,
                    stamp,
                    &mut map,
                    &VirtualOwnerClock::new(now),
                )
                .expect("actual source timer")
                .into_iter()
                .next()
                .expect("one callback")
                .expect("failed source create receipt");
            assert_eq!(
                receipt.requested,
                u32::from(sequence < 3),
                "{caster}: source increments its counter after attempted createMonster, regardless of nil result"
            );
        }
    }
}

#[test]
fn srx3_heart_source_fifteen_second_delay_starts_even_after_failed_creation() {
    for caster in ["the_destruction", "the_hunger"] {
        let (mut runtime, actor, source) =
            source_spawn_fixture(&format!("oteryn:creature.{caster}"));
        let (mut fence, stamp) = fence(&runtime, 1);
        let mut owner = CallbackSpawnOwner::new(&runtime, &fence).expect("actual source owner");
        for sequence in 0..2 {
            let now = SemanticTimeMicros::from_micros(sequence * 1_000_000);
            owner
                .schedule(
                    &runtime,
                    &fence,
                    stamp,
                    &source,
                    &plan(actor, &source, sequence),
                    now,
                )
                .expect("actual source occurrence");
            let mut map = Map {
                present: true,
                allowed: false,
                anchor: true,
                next: 0,
            };
            let receipt = owner
                .drain(
                    &mut runtime,
                    &mut fence,
                    stamp,
                    &mut map,
                    &VirtualOwnerClock::new(now),
                )
                .expect("source timer")
                .into_iter()
                .next()
                .expect("one callback")
                .expect("failed source create receipt");
            assert_eq!(
                receipt.requested,
                u32::from(sequence == 0),
                "{caster}: source schedules removeDelay15000 independently of creation success"
            );
        }
    }
}
