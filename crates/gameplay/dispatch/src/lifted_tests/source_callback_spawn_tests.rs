#![allow(clippy::expect_used)]
use crate::content::{ProjectV2DefinitionRef as Ref, ProjectV2Draft, ProjectV2Family as Family};
use crate::creature_auto_attack::AttackError;
use crate::creature_auto_attack::ProfileAbilityProposal;
use crate::foundation::DotElement;
use crate::foundation::owner_timer::SemanticTimeMicros;
use crate::foundation::owner_timer::VirtualOwnerClock;
use crate::foundation::{
    ActorConditionTransition, ApplicationFacts, CompiledCreaturePolicies, ConditionDefinition,
    ConditionSource, ConditionSourceKind, ConditionValues,
};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, MovementLocalPosition, RuntimeScopeRefV1, RuntimeWorkStamp,
    ScopeOwnershipGeneration, ScopeRuntimeFence,
};
use crate::gameplay_transport::actor_spell::tests::runtime_with_capacity;
use crate::source_callback_spawn::*;
use oteryn_gameplay_foundation::foundation::profile_schedule::ScheduleList;
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

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
    crate::content::native_gameplay::callback_source_native_test_policies(
        include_bytes!("../../../../../apps/game-server/src/callback_native_child_profiles.json"),
        include_bytes!("../../../../../apps/game-server/src/callback_native_child_looks.json"),
        r.content_pin().server_artifact_digest(),
    )
    .expect("qualified fixture")
}
fn fence(r: &ChannelRuntimeV1, g: u64) -> (ScopeRuntimeFence, RuntimeWorkStamp) {
    let b = r.binding();
    crate::foundation::crystal_timer_fixture(
        RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
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
        let parent = r
            .admit_source_pinned_lab_creature(
                MovementLocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
                &case.caster,
                300000,
            )
            .expect("qualified fixture");
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
            Kind::Gaz | Kind::GeneratorRelease => 1,
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
        let parent = r
            .admit_source_pinned_lab_creature(
                MovementLocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
                &case.caster,
                300000,
            )
            .expect("qualified fixture");
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
