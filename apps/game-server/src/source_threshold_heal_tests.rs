#![allow(clippy::expect_used, clippy::panic)]
use super::*;
use crate::foundation::{ChannelContentPin, ChannelId, MovementLocalPosition, NodeId, WorldId};
fn fixture(name: &str) -> (ChannelRuntimeV1, ProjectV2Draft, Ref, ThresholdHealSource) {
    let v:serde_json::Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../docs/agents/evidence/monster-full-mechanics-20261004/lanes/encounters/callback18/threshold-heal-native-fixture-v2.json"))).expect("qualified fixture");
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
    let draft = ProjectV2Draft {
        core: crate::content::ProjectDraft {
            project_revision: "g4-npc-wave-a-r9".into(),
            package_key: "oteryn:content.world-project".into(),
            semantic_schema_version: "reference-schema-v1".into(),
            licensing_metadata: "PENDING".into(),
            world_id: r
                .content_pin()
                .world_id()
                .as_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
            coordinate_frame: v["coordinate_frame"]
                .as_str()
                .expect("qualified fixture")
                .into(),
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
    };
    let caster = Ref {
        family: Family::Creature,
        key: format!("oteryn:creature.{name}"),
        revision: "definition-r1".into(),
    };
    let i = match name {
        "tyrn" => 2,
        "lisa" => 0,
        "professor_maxxen" => 4,
        _ => panic!(),
    };
    let source =
        ThresholdHealSource::from_native(&draft, &caster, i, [1; 32]).expect("qualified fixture");
    (r, draft, caster, source)
}
fn proposal(actor: ExactActorRef, s: &ThresholdHealSource, seq: u64) -> ProfileAbilityProposal {
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
        ability: s.ability.clone(),
        list: ScheduleList::Defence,
        entry_index: s.index,
        magnitude: None,
        range_tiles: 0,
        occurrence: crate::ability::AbilityOccurrence::new(
            &format!("ai-profile:{hex}:{seq}:defence:{}", s.index),
            revisions,
        )
        .expect("qualified fixture"),
        intent: crate::ability::AiAbilityAdapter::normalize(&atom, &[&atom])
            .expect("qualified fixture"),
    }
}
fn fence(r: &ChannelRuntimeV1) -> (ScopeRuntimeFence, RuntimeWorkStamp) {
    let b = r.binding();
    crate::foundation::crystal_timer_fixture(
        RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
        b.scope_generation(),
    )
    .expect("qualified fixture")
}
#[test]
fn three_source_exact_callback_memberships_and_spoofed_encounter_rejected() {
    for name in ["tyrn", "lisa", "professor_maxxen"] {
        let (_, mut d, c, s) = fixture(name);
        assert!(s.minimum > 0);
        let ec = d
            .state
            .authoring_profiles
            .iter_mut()
            .find(|p| {
                p.target.family == Family::Encounter
                    && p.target.key
                        == match name {
                            "tyrn" => "oteryn:encounter.field_fill_tyrn",
                            "lisa" => "oteryn:encounter.field_fill_lisa",
                            _ => "oteryn:encounter.professor_maxxen",
                        }
            })
            .expect("qualified fixture");
        let Data::Encounter(p) = &mut ec.data else {
            panic!()
        };
        p.details = None;
        assert!(ThresholdHealSource::from_native(&d, &c, s.index, [1; 32]).is_err());
    }
}
#[test]
fn tyrn_strict_threshold_actual_hp_replay_and_marker_duplication() {
    let (mut r, _, c, s) = fixture("tyrn");
    let a = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
            &c.key,
            2399,
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
            2400,
        )
        .expect("qualified fixture");
    let (f, stamp) = fence(&r);
    let mut owner = ThresholdHealOwner::default();
    let p = proposal(a, &s, 0);
    let hit = owner
        .cast(&mut r, &f, stamp, &s, &p, 0)
        .expect("qualified fixture");
    assert!(hit.started);
    assert!((7399..=9899).contains(&hit.healed.expect("qualified fixture").health_after));
    let hp = r
        .read_source_creature_health(a, &c.key, 12000)
        .expect("qualified fixture");
    assert_eq!(
        owner
            .cast(&mut r, &f, stamp, &s, &p, 1)
            .expect("qualified fixture"),
        hit
    );
    assert_eq!(
        r.read_source_creature_health(a, &c.key, 12000)
            .expect("qualified fixture"),
        hp
    );
    assert!(
        !owner
            .cast(&mut r, &f, stamp, &s, &proposal(a, &s, 1), 2)
            .expect("qualified fixture")
            .started
    );
    assert!(
        !owner
            .cast(&mut r, &f, stamp, &s, &proposal(b, &s, 0), 0)
            .expect("qualified fixture")
            .started
    );
    assert_eq!(owner.markers.len(), 2);
}
#[test]
fn delayed_lisa_and_maxxen_timer_actual_hp_once_and_private_marker() {
    for name in ["lisa", "professor_maxxen"] {
        let (mut r, _, c, s) = fixture(name);
        let a = r
            .admit_source_pinned_lab_creature(
                MovementLocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
                &c.key,
                1,
            )
            .expect("qualified fixture");
        let (f, stamp) = fence(&r);
        let mut owner = ThresholdHealOwner::default();
        let p = proposal(a, &s, 0);
        let begin = owner
            .cast(&mut r, &f, stamp, &s, &p, 0)
            .expect("qualified fixture");
        assert!(begin.started && begin.pending && begin.healed.is_none());
        assert_eq!(
            r.read_source_creature_health(a, &c.key, s.maximum)
                .expect("qualified fixture"),
            1
        );
        assert!(
            !owner
                .cast(&mut r, &f, stamp, &s, &proposal(a, &s, 1), 1)
                .expect("qualified fixture")
                .started
        );
        assert_eq!(
            owner.tick(&mut r, &f, stamp, &s, a, s.delay_us - 1),
            Ok(None)
        );
        let finish = owner
            .tick(&mut r, &f, stamp, &s, a, s.delay_us)
            .expect("qualified fixture")
            .expect("qualified fixture");
        let hp = finish.healed.expect("qualified fixture").health_after;
        assert!((s.minimum + 1..=s.maximum_draw + 1).contains(&(hp as u64)));
        assert_eq!(
            owner.tick(&mut r, &f, stamp, &s, a, s.delay_us + 1),
            Ok(None)
        );
        assert_eq!(
            r.read_source_creature_health(a, &c.key, s.maximum)
                .expect("qualified fixture"),
            hp as u64
        );
        assert!(
            owner
                .tick(&mut r, &f, stamp, &s, a, s.delay_us - 1)
                .is_err()
        );
    }
}
#[test]
fn pending_dead_or_stale_generation_cannot_heal_replacement() {
    let (mut r, _, c, s) = fixture("lisa");
    let a = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
            &c.key,
            1,
        )
        .expect("qualified fixture");
    let (f, stamp) = fence(&r);
    let mut owner = ThresholdHealOwner::default();
    owner
        .cast(&mut r, &f, stamp, &s, &proposal(a, &s, 0), 0)
        .expect("qualified fixture");
    r.remove_test_actor(a).expect("qualified fixture");
    let replacement = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
            &c.key,
            1,
        )
        .expect("qualified fixture");
    assert!(owner.tick(&mut r, &f, stamp, &s, a, s.delay_us).is_err());
    assert_eq!(
        r.read_source_creature_health(replacement, &c.key, s.maximum)
            .expect("qualified fixture"),
        1
    );
    assert!(owner.markers[0].pending.is_some());
}
#[test]
fn overflow_clock_and_wrong_world_refuse_before_hp_and_marker_write() {
    let (mut r, _, c, mut s) = fixture("tyrn");
    let a = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
            &c.key,
            1,
        )
        .expect("qualified fixture");
    let (f, stamp) = fence(&r);
    let mut owner = ThresholdHealOwner::default();
    let p = proposal(a, &s, 0);
    assert_eq!(
        owner.cast(&mut r, &f, stamp, &s, &p, u64::MAX),
        Err(ThresholdHealError::Clock)
    );
    assert!(owner.markers.is_empty());
    assert_eq!(
        r.read_source_creature_health(a, &c.key, 12000)
            .expect("qualified fixture"),
        1
    );
    s.world = "bad".into();
    assert_eq!(
        owner.cast(&mut r, &f, stamp, &s, &p, 0),
        Err(ThresholdHealError::Fence)
    );
    assert!(owner.markers.is_empty());
}

#[test]
fn pending_actual_dead_health_is_not_revived() {
    let (mut r, _, c, s) = fixture("lisa");
    let a = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
            &c.key,
            1,
        )
        .expect("qualified fixture");
    let (f, stamp) = fence(&r);
    let mut owner = ThresholdHealOwner::default();
    owner
        .cast(&mut r, &f, stamp, &s, &proposal(a, &s, 0), 0)
        .expect("qualified fixture");
    r.crystal_router_fixture_commit_lethal(a, &c.key, 1);
    assert_eq!(r.crystal_router_fixture_health(a), 0);
    assert!(owner.tick(&mut r, &f, stamp, &s, a, s.delay_us).is_err());
    assert_eq!(r.crystal_router_fixture_health(a), 0);
    assert!(owner.markers[0].pending.is_some());
}

// Test-only reader: none of these self-only callbacks needs invented player/map facts.
struct ThresholdWorld;
impl crate::creature_auto_attack::CurrentCombatFactsReader for ThresholdWorld {
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
impl crate::creature_damage_spell::SpellWorldReader for ThresholdWorld {
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
    ) -> Option<crate::creature_attack_geometry::Facing> {
        None
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
        None
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
impl crate::gameplay_transport::actor_spell::InvisibleTileCombatPolicy for ThresholdWorld {
    fn current_tile_allowed(&mut self, _: &ChannelRuntimeV1, _: ExactActorRef) -> Option<bool> {
        None
    }
}
impl crate::monster_combat_lane::MonsterCombatReader for ThresholdWorld {
    fn selected_player(
        &mut self,
        _: &ChannelRuntimeV1,
        _: RuntimeWorkStamp,
        _: crate::ai_think::ThinkOccurrence,
        _: &crate::content::ProjectV2BehaviorAuthoring,
    ) -> Option<Option<(ExactActorRef, crate::foundation::GameSessionId)>> {
        Some(None)
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
}
#[test]
fn actual_native_aggregate_three_boss_schedule_threshold_delayed_hp_and_marker() {
    use crate::foundation::owner_timer::{OwnerClock, SemanticTimeMicros, VirtualOwnerClock};
    for name in ["tyrn", "lisa", "professor_maxxen"] {
        let (mut r, mut d, c, s) = fixture(name);
        complete_retained_threshold_aggregate_fixture(&mut d).expect("qualified fixture");
        let actor = r
            .admit_source_pinned_lab_creature(
                MovementLocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
                &c.key,
                1,
            )
            .expect("qualified fixture");
        let (mut f, _) = fence(&r);
        let mut lane =
            crate::monster_combat_lane::MonsterCombatLane::new(&r, &f).expect("qualified fixture");
        lane.register_project(
            &r,
            &mut f,
            actor,
            &c,
            &d,
            [1; 32],
            SemanticTimeMicros::from_micros(0),
        )
        .expect("qualified fixture");
        let mut states = crate::gameplay_transport::actor_spell::ChannelSpellStates::default();
        let mut world = ThresholdWorld;
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0));
        let revisions = crate::ability::RevisionSet::new(
            "rules-r1",
            "content-r1",
            "policy-r1",
            "definition-r1",
            "sim-r1",
        )
        .expect("qualified fixture");
        let mut first = None;
        let mut committed = None;
        let mut starts = 0;
        for _ in 0..32 {
            let pulse = lane
                .run(
                    &mut r,
                    &mut states,
                    &mut f,
                    &clock,
                    &mut world,
                    revisions.clone(),
                )
                .expect("qualified fixture");
            for think in pulse.thinks {
                assert!(
                    think.schedule.is_ok(),
                    "{name} actual source schedule {:?}",
                    think.schedule
                );
                for (ability, result) in think.defenses {
                    if ability == s.ability {
                        let crate::monster_combat_lane::DefenseOutcome::ThresholdHeal(outcome) =
                            result.expect("qualified fixture")
                        else {
                            panic!("wrong actual native dispatch")
                        };
                        if outcome.started {
                            starts += 1;
                            first.get_or_insert(clock.now().get());
                        }
                        if let Some(hp) = outcome.healed {
                            assert!(hp.health_after > hp.health_before);
                            let applied = (hp.health_after - hp.health_before) as u64;
                            assert!(
                                (s.minimum..=s.maximum_draw).contains(&applied),
                                "{name} exact threshold receipt delta {applied}"
                            );
                            assert!(
                                clock.now().get() >= first.expect("source began") + s.delay_us,
                                "threshold callback healed before source delay"
                            );
                            committed = Some(clock.now().get());
                        }
                    }
                }
            }
            if committed.is_some() {
                break;
            }
            clock.advance(1_000_000);
        }
        let Some(begin) = first else {
            panic!("1000ms/100percent source producer must cast")
        };
        let Some(end) = committed else {
            panic!("actual owning pulse must consume source timer")
        };
        assert_eq!(end - begin, s.delay_us);
        assert_eq!(
            starts, 1,
            "pending source cast must not duplicate marker/heal"
        );
        let hp = r
            .read_source_creature_health(actor, &c.key, s.maximum)
            .expect("qualified fixture");
        let duplicate = lane
            .run(
                &mut r,
                &mut states,
                &mut f,
                &clock,
                &mut world,
                revisions.clone(),
            )
            .expect("qualified fixture");
        assert!(duplicate.thinks.is_empty());
        assert_eq!(
            r.read_source_creature_health(actor, &c.key, s.maximum)
                .expect("qualified fixture"),
            hp
        );
    }
}

#[test]
fn private_marker_reinjured_native_hp_blocks_then_exact_source_expiry_reopens() {
    let (mut r, _, c, s) = fixture("tyrn");
    let a = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
            &c.key,
            1,
        )
        .expect("qualified fixture");
    let (f, stamp) = fence(&r);
    let mut owner = ThresholdHealOwner::default();
    let hit = owner
        .cast(&mut r, &f, stamp, &s, &proposal(a, &s, 0), 0)
        .expect("qualified fixture")
        .healed
        .expect("qualified fixture");
    // Existing test helper calls actual native commit_damage; despite its historical
    // name, this damage intentionally leaves1HP and creates no lethal projection.
    r.crystal_router_fixture_commit_lethal(a, &c.key, hit.health_after - 1);
    assert_eq!(
        r.read_source_creature_health(a, &c.key, s.maximum)
            .expect("qualified fixture"),
        1
    );
    let locked = owner
        .cast(
            &mut r,
            &f,
            stamp,
            &s,
            &proposal(a, &s, 1),
            s.cooldown_us - 1,
        )
        .expect("qualified fixture");
    assert!(!locked.started);
    assert_eq!(
        r.read_source_creature_health(a, &c.key, s.maximum)
            .expect("qualified fixture"),
        1
    );
    let reopened = owner
        .cast(&mut r, &f, stamp, &s, &proposal(a, &s, 2), s.cooldown_us)
        .expect("qualified fixture");
    assert!(reopened.started);
    assert!((5001..=7501).contains(&reopened.healed.expect("qualified fixture").health_after));
    assert_eq!(owner.markers.len(), 1);
}

#[test]
fn fixture_native_reference_parse_lower_link_and_separate_encounter_declarations() {
    let (_, d, _, _) = fixture("tyrn");
    assert_eq!(d.state.declarations.len(), 3);
    assert_eq!(d.core.records.len(), 127);
    let limits = crate::content::ProjectEvidenceLimits {
        max_documents: 11,
        max_document_bytes: 4_000_000,
        max_total_bytes: 16_000_000,
        max_json_depth: 32,
        max_decoded_fields: 200_000,
        max_string_bytes: 2_000_000,
        max_locator_bytes: 160,
        max_locator_segments: 8,
        max_reference_records: 10000,
        max_import_records: 14,
        max_reimport_states: 108,
    };
    // Actual native canonical writer -> strict snapshot parser -> reference lower/link.
    // Encounter declarations stay in separately typed state; they are not executable
    // reference records. Authoring fixture is a fragment, not a complete native v2 world.
    let docs = crate::content::CanonicalProjectDocuments::from_draft(d.core.clone(), limits)
        .expect("qualified fixture");
    let project = docs
        .into_snapshot(limits)
        .expect("qualified fixture")
        .parse(limits)
        .expect("qualified fixture");
    let source = project.lower_reference_source().expect("qualified fixture");
    assert_eq!(source.definitions.len(), 127);
    project.link().expect("qualified fixture");
    let encoded = serde_json::to_vec(&d.state.declarations).expect("qualified fixture");
    let declarations: Vec<crate::content::ProjectV2Declaration> =
        serde_json::from_slice(&encoded).expect("qualified fixture");
    assert_eq!(declarations, d.state.declarations);
}
#[test]
fn retired_65_generations_prune_only_stale_markers_preserving_live_pending_and_cooldown() {
    let (mut r, d, tyrn, ts) = fixture("tyrn");
    let lisa = Ref {
        family: Family::Creature,
        key: "oteryn:creature.lisa".into(),
        revision: "definition-r1".into(),
    };
    let ls = ThresholdHealSource::from_native(&d, &lisa, 0, [1; 32]).expect("qualified fixture");
    let (f, stamp) = fence(&r);
    let mut owner = ThresholdHealOwner::default();
    let keeper = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
            &tyrn.key,
            1,
        )
        .expect("qualified fixture");
    let keeper_heal = owner
        .cast(&mut r, &f, stamp, &ts, &proposal(keeper, &ts, 0), 0)
        .expect("qualified fixture")
        .healed
        .expect("qualified fixture");
    r.crystal_router_fixture_commit_lethal(keeper, &tyrn.key, keeper_heal.health_after - 1);
    let pending = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: 101,
                y: 100,
                floor: 7,
            },
            &lisa.key,
            1,
        )
        .expect("qualified fixture");
    assert!(
        owner
            .cast(&mut r, &f, stamp, &ls, &proposal(pending, &ls, 0), 0)
            .expect("qualified fixture")
            .pending
    );
    for generation in 0..65 {
        let retired = r
            .admit_source_pinned_lab_creature(
                MovementLocalPosition {
                    x: 102,
                    y: 100,
                    floor: 7,
                },
                &tyrn.key,
                12000,
            )
            .expect("qualified fixture");
        let no_heal = owner
            .cast(&mut r, &f, stamp, &ts, &proposal(retired, &ts, 0), 0)
            .expect("qualified fixture");
        assert!(!no_heal.started, "full-health churn generation{generation}");
        assert_eq!(
            owner.markers.len(),
            3,
            "must retain exactly2live protected actors plus current generation"
        );
        r.remove_test_actor(retired).expect("qualified fixture");
        assert!(!r.contains_live_creature(retired));
    }
    // A valid new cast can clear the last retired generation without erasing live timers.
    let fresh = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: 102,
                y: 100,
                floor: 7,
            },
            &tyrn.key,
            12000,
        )
        .expect("qualified fixture");
    owner
        .cast(&mut r, &f, stamp, &ts, &proposal(fresh, &ts, 0), 0)
        .expect("qualified fixture");
    assert_eq!(owner.markers.len(), 3);
    let locked = owner
        .cast(&mut r, &f, stamp, &ts, &proposal(keeper, &ts, 1), 1)
        .expect("qualified fixture");
    assert!(!locked.started);
    assert_eq!(
        r.read_source_creature_health(keeper, &tyrn.key, 12000)
            .expect("qualified fixture"),
        1
    );
    assert!(
        owner
            .markers
            .iter()
            .any(|m| m.actor == pending && m.pending.is_some())
    );
    let end = owner
        .tick(&mut r, &f, stamp, &ls, pending, ls.delay_us)
        .expect("qualified fixture")
        .expect("qualified fixture");
    assert!((18001..=23001).contains(&end.healed.expect("qualified fixture").health_after));
}
#[test]
fn stale_content_activation_rows_pruned_but_invalid_source_cannot_prune() {
    let (mut r, _, c, s) = fixture("tyrn");
    let (f, stamp) = fence(&r);
    let mut owner = ThresholdHealOwner::default();
    let a = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
            &c.key,
            12000,
        )
        .expect("qualified fixture");
    owner
        .cast(&mut r, &f, stamp, &s, &proposal(a, &s, 0), 0)
        .expect("qualified fixture");
    let b = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: 101,
                y: 100,
                floor: 7,
            },
            &c.key,
            12000,
        )
        .expect("qualified fixture");
    owner
        .cast(&mut r, &f, stamp, &s, &proposal(b, &s, 0), 0)
        .expect("qualified fixture");
    // Mutate only stale ledger metadata (not current native pin or caller authority).
    owner.markers[0].activation = r
        .content_pin()
        .activation_sequence()
        .checked_add(1)
        .expect("qualified fixture");
    owner.markers[1].content = [2; 32];
    let fresh = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: 102,
                y: 100,
                floor: 7,
            },
            &c.key,
            12000,
        )
        .expect("qualified fixture");
    let mut wrong = s.clone();
    wrong.content = [9; 32];
    assert_eq!(
        owner.cast(&mut r, &f, stamp, &wrong, &proposal(fresh, &wrong, 0), 0),
        Err(ThresholdHealError::Fence)
    );
    assert_eq!(
        owner.markers.len(),
        2,
        "untrusted source cannot prune ledger"
    );
    owner
        .cast(&mut r, &f, stamp, &s, &proposal(fresh, &s, 0), 0)
        .expect("qualified fixture");
    assert_eq!(owner.markers.len(), 1);
    assert_eq!(owner.markers[0].actor, fresh);
    assert_eq!(
        owner.markers[0].content,
        r.content_pin().server_artifact_digest()
    );
    assert_eq!(
        owner.markers[0].activation,
        r.content_pin().activation_sequence()
    );
}

#[cfg(test)]
pub(crate) fn complete_retained_threshold_aggregate_fixture(
    draft: &mut ProjectV2Draft,
) -> Result<(), String> {
    // Exact immutable Seven/Spawn qualification witnesses used by production owners.
    // The historical three-boss capture predates these added consumers; no
    // source validator is disabled and no authored schedule chance is modified.
    fn profiles(
        d: &mut ProjectV2Draft,
        values: &[serde_json::Value],
        replace: bool,
    ) -> Result<(), String> {
        for value in values {
            let profile: crate::content::ProjectV2AuthoringProfile =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            if let Some(current) = d
                .state
                .authoring_profiles
                .iter_mut()
                .find(|p| p.target == profile.target)
            {
                if *current != profile {
                    if !replace {
                        return Err(format!(
                            "conflicting native fixture profile {}",
                            profile.target.key
                        ));
                    }
                    *current = profile;
                }
            } else {
                d.state.authoring_profiles.push(profile);
            }
        }
        Ok(())
    }
    fn records(
        d: &mut ProjectV2Draft,
        values: &[serde_json::Value],
        replace: bool,
    ) -> Result<(), String> {
        for value in values {
            let record: crate::content::ProjectReferenceRecord =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            let id = &value["identity"];
            let mut selected = None;
            for (index, current) in d.core.records.iter().enumerate() {
                if serde_json::to_value(current).map_err(|e| e.to_string())?["identity"] == *id {
                    selected = Some(index);
                    break;
                }
            }
            if let Some(index) = selected {
                if d.core.records[index] != record {
                    if !replace {
                        return Err("conflicting native fixture record".into());
                    }
                    d.core.records[index] = record;
                }
            } else {
                d.core.records.push(record);
            }
        }
        Ok(())
    }
    if draft
        .state
        .authoring_profiles
        .iter()
        .any(|p| p.target.key == "oteryn:creature.professor_maxxen")
    {
        let seven: serde_json::Value =
            serde_json::from_str(include_str!("source_encounter_seven_fixture.json"))
                .map_err(|e| e.to_string())?;
        profiles(
            draft,
            seven["profiles"]
                .as_array()
                .ok_or("actual Seven profiles")?,
            false,
        )?;
        records(
            draft,
            seven["records"].as_array().ok_or("actual Seven records")?,
            false,
        )?;
        for value in seven["source_identity_bindings"]
            .as_array()
            .ok_or("actual Seven bindings")?
        {
            let binding = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            if !draft.state.source_identity_bindings.contains(&binding) {
                draft.state.source_identity_bindings.push(binding);
            }
        }
        for value in seven["sources"].as_array().ok_or("actual Seven sources")? {
            let source: crate::content::ProjectV2Source =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            if !draft
                .state
                .sources
                .iter()
                .any(|s| s.key == source.key && s.revision == source.revision)
            {
                draft.state.sources.push(source);
            }
        }
        // Add only already selected callback parents (Maxxen generator and its
        // retained children), never the unrelated sixteen-source population.
        let spawn: serde_json::Value =
            serde_json::from_str(include_str!("source_callback_spawn_fixture.json"))
                .map_err(|e| e.to_string())?;
        for case in spawn["cases"].as_array().ok_or("actual callback cases")? {
            let parent = case["parent"].as_str().ok_or("actual callback parent")?;
            if !draft
                .state
                .authoring_profiles
                .iter()
                .any(|p| p.target.key == parent)
            {
                continue;
            }
            for (key, family) in [
                (parent, "Ability"),
                (
                    case["encounter"]
                        .as_str()
                        .ok_or("actual callback Encounter")?,
                    "Encounter",
                ),
            ] {
                let data = spawn["profiles"]
                    .get(key)
                    .ok_or("actual callback profile")?;
                profiles(
                    draft,
                    &[
                        serde_json::json!({"target":{"family":family,"key":key,"revision":"definition-r1"},"data":data}),
                    ],
                    false,
                )?;
            }
            for record in spawn["records"]
                .as_array()
                .ok_or("actual callback records")?
            {
                if record["identity"]["key"] == parent {
                    records(draft, &[record.clone()], false)?;
                }
            }
            for binding in spawn["bindings"]
                .as_array()
                .ok_or("actual callback bindings")?
            {
                if binding["target"]["key"] != case["encounter"] {
                    continue;
                }
                let binding = serde_json::from_value(binding.clone()).map_err(|e| e.to_string())?;
                if !draft.state.source_identity_bindings.contains(&binding) {
                    draft.state.source_identity_bindings.push(binding);
                }
            }
        }
        // The selected energy_pulse child executes a source callback cast. Its
        // production qualifier deliberately checks the entire pinned callback
        // witness, so retain that witness too rather than bypass its closure.
        let casts: serde_json::Value =
            serde_json::from_str(include_str!("source_callback_cast_fixture.json"))
                .map_err(|e| e.to_string())?;
        for (key, data) in casts["profiles"]
            .as_object()
            .ok_or("actual callback cast profiles")?
        {
            let family = data["kind"].as_str().ok_or("actual callback cast family")?;
            profiles(
                draft,
                &[
                    serde_json::json!({"target":{"family":family,"key":key,"revision":"definition-r1"},"data":data}),
                ],
                false,
            )?;
        }
        records(
            draft,
            casts["records"]
                .as_array()
                .ok_or("actual callback cast records")?,
            false,
        )?;
        for value in casts["bindings"]
            .as_array()
            .ok_or("actual callback cast bindings")?
        {
            let binding = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            if !draft.state.source_identity_bindings.contains(&binding) {
                draft.state.source_identity_bindings.push(binding);
            }
        }
    }
    Ok(())
}
