#![allow(clippy::expect_used, clippy::panic)]
use super::*;
use crate::foundation::{ChannelContentPin, ChannelId, MovementLocalPosition, NodeId, WorldId};
fn fixture() -> (ChannelRuntimeV1, ProjectV2Draft, Ref) {
    let v:serde_json::Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../docs/agents/evidence/monster-full-mechanics-20261004/lanes/encounters/area-heal-native-fixture.json"))).expect("qualified fixture");
    let id = |t: u8| [1, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, t];
    let w = WorldId::decode(&id(1)).expect("qualified fixture");
    let r = ChannelRuntimeV1::from_committed_assignment(
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
    // Fixture envelope only; each source definition/binding below is retained native readback.
    let draft = ProjectV2Draft {
        core: crate::content::ProjectDraft {
            project_revision: "fixture-r1".into(),
            package_key: "oteryn:fixture".into(),
            semantic_schema_version: "v2".into(),
            licensing_metadata: "fixture".into(),
            world_id: r
                .content_pin()
                .world_id()
                .as_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
            coordinate_frame: "fixture".into(),
            records: serde_json::from_value(v["records"].clone()).expect("qualified fixture"),
            imports: vec![],
            metadata: vec![],
        },
        state: crate::content::ProjectV2State {
            authoring_profiles: serde_json::from_value(v["authoring_profiles"].clone())
                .expect("qualified fixture"),
            source_identity_bindings: serde_json::from_value(v["source_identity_bindings"].clone())
                .expect("qualified fixture"),
            sources: serde_json::from_value(v["sources"].clone()).expect("qualified fixture"),
            ..Default::default()
        },
    };
    (
        r,
        draft,
        Ref {
            family: ProjectV2Family::Creature,
            key: "oteryn:creature.frozen_minion".into(),
            revision: "definition-r1".into(),
        },
    )
}
struct FixturePolicy {
    missing: bool,
}
impl AreaHealPolicy for FixturePolicy {
    fn facing(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: RuntimeWorkStamp,
    ) -> Option<Facing> {
        Some(Facing::North)
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
        (!self.missing).then_some(true)
    }
    fn target_allowed(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: ExactActorRef,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        Some(true)
    }
    fn non_player_side(
        &mut self,
        r: &ChannelRuntimeV1,
        target: ExactActorRef,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        r.native_summon_role(target).ok().map(|_| true)
    }
    fn masterless(
        &mut self,
        r: &ChannelRuntimeV1,
        target: ExactActorRef,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        r.native_summon_role(target).ok().map(|m| m.is_none())
    }
    fn is_top_creature(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        None
    }
}
fn proposal_fixture(actor: ExactActorRef, source: &AreaHealSource) -> ProfileAbilityProposal {
    // Explicit owner-boundary proposal fixture, not an autonomous scheduler/chance proof.
    let hex = actor
        .placement_identity()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let atom = format!("fixture:{hex}");
    let revisions = crate::ability::RevisionSet::new(
        "rules-r1",
        "content-r1",
        "policy-r1",
        "definition-r1",
        "sim-r1",
    )
    .expect("qualified fixture");
    ProfileAbilityProposal {
        issuer: actor,
        target: actor,
        ability: source.ability.clone(),
        list: ScheduleList::Defence,
        entry_index: 0,
        magnitude: None,
        range_tiles: 0,
        occurrence: crate::ability::AbilityOccurrence::new(
            &format!("ai-profile:{hex}:0:defence:0"),
            revisions,
        )
        .expect("qualified fixture"),
        intent: crate::ability::AiAbilityAdapter::normalize(&atom, &[&atom])
            .expect("qualified fixture"),
    }
}
#[test]
fn exact_frozen_source_and_current_native_hp_crosscaster_sequence_zero_replay() {
    let (mut r, d, c) = fixture();
    let catalog = CreatureHealCatalog::from_native(&d, [1; 32]).expect("qualified fixture");
    let source = AreaHealSource::from_native(&d, &c, 0, [1; 32]).expect("qualified fixture");
    assert_eq!((source.minimum, source.maximum), (100, 200));
    assert_eq!(catalog.maximum[&c.key], 3725);
    let a = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
            &c.key,
            1000,
        )
        .expect("qualified fixture");
    let b = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: 101,
                y: 100,
                floor: 7,
            },
            &c.key,
            1000,
        )
        .expect("qualified fixture");
    let target = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: 100,
                y: 101,
                floor: 7,
            },
            &c.key,
            1000,
        )
        .expect("qualified fixture");
    let bind = r.binding();
    let (fence, stamp) = crate::foundation::crystal_timer_fixture(
        RuntimeScopeRefV1::channel(bind.world_id(), bind.channel_id()),
        bind.scope_generation(),
    )
    .expect("qualified fixture");
    let pa = proposal_fixture(a, &source);
    let pb = proposal_fixture(b, &source);
    let mut states = crate::gameplay_transport::actor_spell::ChannelSpellStates::default();
    let mut owner = AreaHealOwner::default();
    let mut policy = FixturePolicy { missing: false };
    let first = owner
        .execute(
            &mut r,
            &fence,
            stamp,
            &catalog,
            &source,
            &pa,
            &mut policy,
            &mut states,
            0,
        )
        .expect("qualified fixture");
    let hp1 = first
        .targets
        .iter()
        .find(|(t, _)| *t == target)
        .expect("qualified fixture")
        .1
        .health_after;
    let second = owner
        .execute(
            &mut r,
            &fence,
            stamp,
            &catalog,
            &source,
            &pb,
            &mut policy,
            &mut states,
            0,
        )
        .expect("qualified fixture");
    let hp2 = second
        .targets
        .iter()
        .find(|(t, _)| *t == target)
        .expect("qualified fixture")
        .1
        .health_after;
    assert!(hp2 > hp1);
    assert_eq!(
        owner
            .execute(
                &mut r,
                &fence,
                stamp,
                &catalog,
                &source,
                &pa,
                &mut policy,
                &mut states,
                1
            )
            .expect("qualified fixture"),
        first
    );
    assert_eq!(owner.memos.len(), 2);
    assert!(owner.memos.iter().all(|m| m.sequence == 0));
    let probe = r
        .commit_source_creature_heal_batch([1; 32], &[(target, c.key.clone(), 3725, 0)])
        .expect("qualified fixture");
    assert_eq!(probe[0].health_after, hp2);
}
#[test]
fn independent_missing_area_policy_refuses_before_hp_then_source_retry_can_execute() {
    let (mut r, d, c) = fixture();
    let catalog = CreatureHealCatalog::from_native(&d, [1; 32]).expect("qualified fixture");
    let source = AreaHealSource::from_native(&d, &c, 0, [1; 32]).expect("qualified fixture");
    let a = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
            &c.key,
            1000,
        )
        .expect("qualified fixture");
    let bind = r.binding();
    let (fence, stamp) = crate::foundation::crystal_timer_fixture(
        RuntimeScopeRefV1::channel(bind.world_id(), bind.channel_id()),
        bind.scope_generation(),
    )
    .expect("qualified fixture");
    let p = proposal_fixture(a, &source);
    let mut states = crate::gameplay_transport::actor_spell::ChannelSpellStates::default();
    let mut owner = AreaHealOwner::default();
    let mut policy = FixturePolicy { missing: true };
    assert_eq!(
        owner.execute(
            &mut r,
            &fence,
            stamp,
            &catalog,
            &source,
            &p,
            &mut policy,
            &mut states,
            0
        ),
        Err(AreaHealError::MissingCurrentPolicy)
    );
    assert!(owner.memos.is_empty());
    let probe = r
        .commit_source_creature_heal_batch([1; 32], &[(a, c.key.clone(), 3725, 0)])
        .expect("qualified fixture");
    assert_eq!(probe[0].health_after, 1000);
    policy.missing = false;
    assert!(
        owner
            .execute(
                &mut r,
                &fence,
                stamp,
                &catalog,
                &source,
                &p,
                &mut policy,
                &mut states,
                1
            )
            .expect("qualified fixture")
            .targets[0]
            .1
            .health_after
            > 1000
    );
}

// Test-only independent approved arena policy. No corresponding permissive product default.
struct AggregateAreaWorld;
impl crate::creature_auto_attack::CurrentCombatFactsReader for AggregateAreaWorld {
    fn read_attack(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: ExactActorRef,
        _: crate::foundation::GameSessionId,
        _: RuntimeWorkStamp,
    ) -> Option<crate::creature_auto_attack::AttackFacts> {
        None
    }
}
impl crate::creature_damage_spell::SpellWorldReader for AggregateAreaWorld {
    fn current_players(
        &mut self,
        _: &ChannelRuntimeV1,
        _: RuntimeWorkStamp,
    ) -> Option<Vec<(ExactActorRef, crate::foundation::GameSessionId)>> {
        Some(vec![])
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
        if p.context() != r.pinned_movement_context() {
            return None;
        }
        Some(z == p.position().floor)
    }
    fn combat(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: ExactActorRef,
        _: crate::foundation::GameSessionId,
        _: Option<&str>,
        _: RuntimeWorkStamp,
    ) -> Option<crate::creature_damage_spell::SpellCombatFacts> {
        None
    }
}
impl crate::gameplay_transport::actor_spell::InvisibleTileCombatPolicy for AggregateAreaWorld {
    fn current_tile_allowed(&mut self, _: &ChannelRuntimeV1, _: ExactActorRef) -> Option<bool> {
        None
    }
}
impl crate::monster_combat_lane::MonsterCombatReader for AggregateAreaWorld {
    fn selected_player(
        &mut self,
        r: &ChannelRuntimeV1,
        _: RuntimeWorkStamp,
        o: crate::ai_think::ThinkOccurrence,
        _: &crate::content::ProjectV2BehaviorAuthoring,
    ) -> Option<Option<(ExactActorRef, crate::foundation::GameSessionId)>> {
        r.contains_live_creature(o.actor).then_some(None)
    }
    fn condition_tick_facts(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: crate::foundation::GameSessionId,
        _: RuntimeWorkStamp,
    ) -> Option<crate::foundation::TickFacts> {
        None
    }
    fn area_heal_target_allowed(
        &mut self,
        r: &ChannelRuntimeV1,
        c: ExactActorRef,
        t: ExactActorRef,
        a: &Ref,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        if a.key != "oteryn:ability.spell.frozen_minion_heal"
            || !r.contains_live_creature(c)
            || !r.contains_live_creature(t)
        {
            return None;
        }
        let cpos = r.read_actor_position(c).ok()?;
        let tpos = r.read_actor_position(t).ok()?;
        if cpos.context() != r.pinned_movement_context() || tpos.context() != cpos.context() {
            return None;
        }
        Some(cpos.position().floor == tpos.position().floor)
    }
    fn area_heal_non_player_side(
        &mut self,
        r: &ChannelRuntimeV1,
        t: ExactActorRef,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        r.native_summon_role(t).ok().and_then(|m| match m {
            None => Some(true),
            Some(parent) => r.contains_live_creature(parent).then_some(true),
        })
    }
}
#[test]
fn actual_profile_schedule_defense_chance_native_think_pulse_heals_neighbor_creature() {
    use crate::foundation::owner_timer::{SemanticTimeMicros, VirtualOwnerClock};
    let (mut r, d, c) = fixture();
    let actor = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
            &c.key,
            1000,
        )
        .expect("qualified fixture");
    let neighbor = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: 100,
                y: 101,
                floor: 7,
            },
            &c.key,
            1000,
        )
        .expect("qualified fixture");
    let binding = r.binding();
    let (mut fence, _) = crate::foundation::crystal_timer_fixture(
        RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
        binding.scope_generation(),
    )
    .expect("qualified fixture");
    let mut lane =
        crate::monster_combat_lane::MonsterCombatLane::new(&r, &fence).expect("qualified fixture");
    lane.register_project(
        &r,
        &mut fence,
        actor,
        &c,
        &d,
        [1; 32],
        SemanticTimeMicros::from_micros(0),
    )
    .expect("qualified fixture");
    let mut states = crate::gameplay_transport::actor_spell::ChannelSpellStates::default();
    let mut world = AggregateAreaWorld;
    let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0));
    let revisions = crate::ability::RevisionSet::new(
        "rules-r1",
        "content-r1",
        "policy-r1",
        "definition-r1",
        "sim-r1",
    )
    .expect("qualified fixture");
    let mut committed = false;
    for _ in 0..256 {
        let pulse = lane
            .run(
                &mut r,
                &mut states,
                &mut fence,
                &clock,
                &mut world,
                revisions.clone(),
            )
            .expect("qualified fixture");
        for think in pulse.thinks {
            for (_, effect) in think.defenses {
                if let Ok(crate::monster_combat_lane::DefenseOutcome::AreaHeal(result)) = effect {
                    let hp = result
                        .targets
                        .iter()
                        .find(|(t, _)| *t == neighbor)
                        .expect("actual current neighboring Creature receives heal")
                        .1;
                    assert!(hp.health_after > hp.health_before);
                    committed = true;
                }
            }
        }
        if committed {
            break;
        }
        clock.advance(1_000_000);
    }
    assert!(
        committed,
        "actual source2000ms/15percent defense schedule must eventually cast"
    );
}
#[test]
#[ignore = "requires retained current source-corrected native eleven-document capture; run explicitly locally"]
fn actual_corrected_native_world_has_1863_hp_catalog_19_area_and_1_project_prophet_membership() {
    use crate::content::*;
    let retained_native_capture_path = std::env::var_os("OTERYN_MONSTER_NATIVE_CAPTURE_ROOT")
        .filter(|value| !value.is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| panic!("OTERYN_MONSTER_NATIVE_CAPTURE_ROOT must explicitly name the current final native eleven-document capture"));
    let path = retained_native_capture_path.as_path();
    let project = capture_world_project(
        path.parent().expect("qualified fixture"),
        path.file_name().expect("qualified fixture"),
        ProjectFilesystemLimits {
            project: ProjectEvidenceLimits {
                max_documents: 11,
                max_document_bytes: 96_000_000,
                max_total_bytes: 160_000_000,
                max_json_depth: 24,
                max_decoded_fields: 2_120_000,
                max_string_bytes: 43_000_000,
                max_locator_bytes: 160,
                max_locator_segments: 8,
                max_reference_records: 70000,
                max_import_records: 16,
                max_reimport_states: 404,
            },
            max_entries_per_directory_scan: 32,
            max_total_directory_entries_scanned: 201,
        },
    )
    .expect("qualified fixture");
    let draft = project.migrate_to_v2();
    let source = project.lower_reference_source().expect("qualified fixture");
    let catalog = CreatureHealCatalog::from_native(&draft, [1; 32]).expect("qualified fixture");
    assert_eq!(catalog.world, source.world_id);
    assert_eq!(catalog.maximum.len(), 1863);
    let mut memberships = Vec::new();
    for p in &draft.state.authoring_profiles {
        let Data::Creature(_) = &p.data else { continue };
        let Some(Record::Creature{behavior,..})=draft.core.records.iter().find(|r|matches!(r,Record::Creature{identity,..}if identity.key==p.target.key&&identity.revision==p.target.revision))else{panic!("actual linked Creature")};
        let Some(Data::Behavior(b)) = draft
            .state
            .authoring_profiles
            .iter()
            .find(|v| v.target.key == behavior.key && v.target.revision == behavior.revision)
            .map(|p| &p.data)
        else {
            panic!("actual linked Behavior")
        };
        for (index, _) in b.defenses.iter().enumerate() {
            if let Ok(s) = AreaHealSource::from_native(&draft, &p.target, index, [1; 32]) {
                memberships.push((
                    p.target.key.clone(),
                    s.ability.key.clone(),
                    s.requires_paralysis_removal(),
                ));
            }
        }
    }
    assert_eq!(memberships.len(), 20);
    // Nineteen previous area primitives plus ONE explicit local source-load RNG
    // substitute; do not count the latter as donor RNG or Global parity proof.
    assert_eq!(
        memberships
            .iter()
            .filter(|(c, _, _)| c != "oteryn:creature.minotaur_cult_prophet")
            .count(),
        19
    );
    let prophet = Ref {
        family: ProjectV2Family::Creature,
        key: "oteryn:creature.minotaur_cult_prophet".into(),
        revision: "definition-r1".into(),
    };
    let qualified =
        AreaHealSource::from_native(&draft, &prophet, 0, [1; 32]).expect("qualified fixture");
    assert_eq!((qualified.minimum, qualified.maximum), (200, 350));
    let load = qualified
        .source_load_magnitude
        .expect("PROJECT source-load snapshot");
    assert!((200..=350).contains(&load));
    assert_eq!(
        AreaHealSource::from_native(&draft, &prophet, 0, [1; 32])
            .expect("qualified fixture")
            .source_load_magnitude,
        Some(load)
    );
    let filter = qualified
        .filter
        .as_ref()
        .expect("exact named source filter");
    assert!(filter.includes_caster && filter.top_creature_only && !filter.excludes_caster_name);
    assert_eq!(filter.creatures.len(), 3);
    assert_eq!(
        memberships
            .iter()
            .filter(|(c, _, _)| c == "oteryn:creature.knowledge_elemental")
            .count(),
        2
    );
    assert!(
        memberships
            .iter()
            .any(|(c, _, r)| c == "oteryn:creature.soul_cage" && *r)
    );
    assert!(memberships.iter().any(
        |(c, ability, _)| c == "oteryn:creature.minotaur_cult_prophet"
            && ability == "oteryn:ability.creature.minotaur_cult_prophet.field-fill-defenses-1"
    ));
}

struct ProphetPolicy {
    missing_top: bool,
    top: ExactActorRef,
}
impl AreaHealPolicy for ProphetPolicy {
    fn facing(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: RuntimeWorkStamp,
    ) -> Option<Facing> {
        Some(Facing::North)
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
        Some(true)
    }
    fn target_allowed(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: ExactActorRef,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        Some(true)
    }
    fn non_player_side(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        None
    }
    fn masterless(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        None
    }
    fn is_top_creature(
        &mut self,
        _: &ChannelRuntimeV1,
        t: ExactActorRef,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        (!self.missing_top).then_some(t == self.top)
    }
}
#[test]
fn prophet_source_load_snapshot_named_top_target_self_and_atomic_missing_policy() {
    let (mut r, mut d, _) = fixture();
    let v:serde_json::Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../docs/agents/evidence/monster-full-mechanics-20261004/reconcile-main/lanes/main95-source-compositions/prophet-native-fixture.json"))).expect("qualified fixture");
    d.core.records.extend(
        serde_json::from_value::<Vec<Record>>(v["records"].clone()).expect("qualified fixture"),
    );
    d.state.authoring_profiles.extend(
        serde_json::from_value::<Vec<crate::content::ProjectV2AuthoringProfile>>(
            v["authoring_profiles"].clone(),
        )
        .expect("qualified fixture"),
    );
    d.state.source_identity_bindings.extend(
        serde_json::from_value::<Vec<crate::content::ProjectV2SourceIdentityBinding>>(
            v["source_identity_bindings"].clone(),
        )
        .expect("qualified fixture"),
    );
    // The retained base fixture already includes the same pinned Canary source generation.
    let c = Ref {
        family: ProjectV2Family::Creature,
        key: "oteryn:creature.minotaur_cult_prophet".into(),
        revision: "definition-r1".into(),
    };
    let catalog = CreatureHealCatalog::from_native(&d, [1; 32]).expect("qualified fixture");
    let source = AreaHealSource::from_native(&d, &c, 0, [1; 32]).expect("qualified fixture");
    let load = source.source_load_magnitude.expect("qualified fixture");
    assert!((200..=350).contains(&load));
    assert_eq!(
        AreaHealSource::from_native(&d, &c, 0, [1; 32])
            .expect("qualified fixture")
            .source_load_magnitude,
        Some(load)
    );
    let a = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
            &c.key,
            100,
        )
        .expect("qualified fixture");
    let key = "oteryn:creature.minotaur_cult_follower";
    let b = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: 100,
                y: 101,
                floor: 7,
            },
            key,
            100,
        )
        .expect("qualified fixture");
    let bind = r.binding();
    let (fence, stamp) = crate::foundation::crystal_timer_fixture(
        RuntimeScopeRefV1::channel(bind.world_id(), bind.channel_id()),
        bind.scope_generation(),
    )
    .expect("qualified fixture");
    let proposal = proposal_fixture(a, &source);
    let mut owner = AreaHealOwner::default();
    let mut states = crate::gameplay_transport::actor_spell::ChannelSpellStates::default();
    let mut policy = ProphetPolicy {
        missing_top: true,
        top: b,
    };
    assert_eq!(
        owner.execute(
            &mut r,
            &fence,
            stamp,
            &catalog,
            &source,
            &proposal,
            &mut policy,
            &mut states,
            0
        ),
        Err(AreaHealError::MissingCurrentPolicy)
    );
    assert_eq!(
        r.read_source_creature_health(a, &c.key, catalog.maximum[&c.key])
            .expect("qualified fixture"),
        100
    );
    assert_eq!(
        r.read_source_creature_health(b, key, catalog.maximum[key])
            .expect("qualified fixture"),
        100
    );
    policy.missing_top = false;
    let result = owner
        .execute(
            &mut r,
            &fence,
            stamp,
            &catalog,
            &source,
            &proposal,
            &mut policy,
            &mut states,
            0,
        )
        .expect("qualified fixture");
    assert_eq!(result.targets.len(), 2);
    assert!(result.qualification.contains("GLOBAL_UNVERIFIED"));
    assert_eq!(
        r.read_source_creature_health(a, &c.key, catalog.maximum[&c.key])
            .expect("qualified fixture"),
        100 + load
    );
    assert_eq!(
        r.read_source_creature_health(b, key, catalog.maximum[key])
            .expect("qualified fixture"),
        100 + load
    );
    assert_eq!(
        owner
            .execute(
                &mut r,
                &fence,
                stamp,
                &catalog,
                &source,
                &proposal,
                &mut policy,
                &mut states,
                0
            )
            .expect("qualified fixture"),
        result
    );
}
