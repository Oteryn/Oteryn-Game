#![allow(clippy::expect_used, clippy::panic)]
use super::*;
use crate::foundation::owner_timer::VirtualOwnerClock;
use crate::foundation::{
    CompiledCreaturePolicies, CompiledCreaturePolicy, ConditionType, CreatureExactRatio,
    CreatureFlags, CreatureResistance, MovementLocalPosition,
};
use crate::gameplay_transport::actor_spell::tests::runtime_with_player;
fn draft(r: &ChannelRuntimeV1) -> ProjectV2Draft {
    let v: serde_json::Value =
        serde_json::from_str(include_str!("source_encounter_seven_fixture.json"))
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
            authoring_profiles: serde_json::from_value(v["profiles"].clone())
                .expect("qualified fixture"),
            sources: serde_json::from_value(v["sources"].clone()).expect("qualified fixture"),
            source_identity_bindings: serde_json::from_value(v["source_identity_bindings"].clone())
                .expect("qualified fixture"),
            ..Default::default()
        },
    }
}
pub(crate) fn source(r: &ChannelRuntimeV1, d: &ProjectV2Draft, index: usize) -> SevenSource {
    SevenSource::qualify(
        d,
        &Ref {
            family: Family::Creature,
            key: BASE.into(),
            revision: "definition-r1".into(),
        },
        index,
        ScheduleList::Defence,
        r.content_pin().server_artifact_digest(),
    )
    .expect("qualified fixture")
    .expect("qualified fixture")
}
// Fixture policies come from the exact canonical native source profiles, not donor/default HP.
fn policy(d: &ProjectV2Draft, key: &str) -> CompiledCreaturePolicy {
    let p = d
        .state
        .authoring_profiles
        .iter()
        .find(|p| p.target.key == key)
        .expect("qualified fixture");
    let Data::Creature(cp) = &p.data else {
        panic!("Creature")
    };
    let detail = cp.details.as_ref().expect("qualified fixture");
    let rec = d
        .core
        .records
        .iter()
        .find(|r| matches!(r,Record::Creature{identity,..}if identity.key==key))
        .expect("qualified fixture");
    let Record::Creature {
        presentation,
        behavior,
        ..
    } = rec
    else {
        panic!("Creature")
    };
    let Data::Presentation(appearance) = &d
        .state
        .authoring_profiles
        .iter()
        .find(|p| p.target.key == presentation.key)
        .expect("qualified fixture")
        .data
    else {
        panic!("Presentation")
    };
    let Data::Behavior(behavior) = &d
        .state
        .authoring_profiles
        .iter()
        .find(|p| p.target.key == behavior.key)
        .expect("qualified fixture")
        .data
    else {
        panic!("Behavior")
    };
    let ratio = |v: crate::content::ProjectV2ExactRatio| CreatureExactRatio {
        numerator: v.numerator,
        denominator: v.denominator,
    };
    CompiledCreaturePolicy {
        definition_key: key.into(),
        definition_revision: "definition-r1".into(),
        display_name: detail.display_name.clone(),
        maximum_health: i64::try_from(cp.health.expect("qualified fixture"))
            .expect("qualified fixture"),
        base_speed: i32::try_from(cp.speed.expect("qualified fixture")).expect("qualified fixture"),
        outfit_look_type: appearance
            .asset_binding
            .as_ref()
            .expect("qualified fixture")
            .strip_prefix("canary.appearance:outfit/")
            .expect("qualified fixture")
            .parse()
            .expect("qualified fixture"),
        object_look_type: None,
        summonable: detail.summoning.summonable,
        convinceable: detail.summoning.convinceable,
        mana_cost: detail.summoning.mana_cost,
        is_familiar: detail.summoning.is_familiar,
        condition_immunities: detail
            .condition_immunities
            .iter()
            .map(|s| match s.as_str() {
                "invisible" => ConditionType::Invisible,
                "paralyze" => ConditionType::Paralysis,
                _ => panic!("exact source fixture condition"),
            })
            .collect(),
        armor: cp.armor,
        mitigation: cp.mitigation.map(ratio),
        resistances: cp
            .resistances
            .iter()
            .map(|v| CreatureResistance {
                damage_type: v.damage_type.clone(),
                percent: ratio(v.percent),
            })
            .collect(),
        damage_immunities: cp.immunities.clone(),
        healing_from_damage: detail
            .healing_from_damage
            .iter()
            .map(|v| CreatureResistance {
                damage_type: v.damage_type.clone(),
                percent: ratio(v.percent),
            })
            .collect(),
        flags: CreatureFlags {
            attackable: detail.flags.attackable,
            illusionable: detail.flags.illusionable,
            health_hidden: detail.flags.health_hidden,
        },
        preferred_distance: Some(u32::from(behavior.targeting.target_distance_tiles)),
        reward_boss: Some(detail.system_eligibility.reward_boss),
    }
}
pub(crate) fn fixture() -> (
    ChannelRuntimeV1,
    ProjectV2Draft,
    ExactActorRef,
    ScopeRuntimeFence,
    RuntimeWorkStamp,
) {
    let (mut r, _, _) = runtime_with_player(0x72);
    let d = draft(&r);
    let policies = [BASE, BLAZING, FREEZING]
        .iter()
        .map(|k| policy(&d, k))
        .collect();
    r.install_companion_policies(
        CompiledCreaturePolicies::from_active_artifact(
            r.content_pin().server_artifact_digest(),
            policies,
        )
        .expect("qualified fixture"),
    )
    .expect("qualified fixture");
    let actor = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 14,
            },
            BASE,
            290000,
        )
        .expect("qualified fixture");
    r.install_creature_policy(actor, BASE)
        .expect("qualified fixture");
    r.commit_monster_lab_damage(actor, b"source-initial-wound", 280000)
        .expect("qualified fixture");
    let b = r.binding();
    let (f, stamp) = crate::foundation::crystal_timer_fixture(
        RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
        b.scope_generation(),
    )
    .expect("qualified fixture");
    (r, d, actor, f, stamp)
}
fn proposal(actor: ExactActorRef, s: &SevenSource, seq: u64) -> ProfileAbilityProposal {
    let a = format!("actor:{}", hex(actor.placement_identity()));
    ProfileAbilityProposal {
        issuer: actor,
        target: actor,
        ability: s.ability(),
        list: s.phase(),
        entry_index: s.case.index,
        magnitude: None,
        range_tiles: 0,
        occurrence: crate::ability::AbilityOccurrence::new(
            &format!(
                "ai-profile:{}:{seq}:defence:{}",
                hex(actor.placement_identity()),
                s.case.index
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
        intent: crate::ability::AiAbilityAdapter::normalize(&a, &[&a]).expect("qualified fixture"),
    }
}
#[test]
fn actual_time_profiles_share_actor_hp_history_and_exact30s_return_not_global_swap() {
    let (mut r, d, a, f, stamp) = fixture();
    let s = source(&r, &d, 0);
    let p = proposal(a, &s, 0);
    let mut o = TimeGuardianOwner::new(&r).expect("qualified fixture");
    let before = r.companion_snapshot(a).expect("qualified fixture");
    let plan = o
        .prepare_cast(&r, &f, stamp, &s, &p, SemanticTimeMicros::from_micros(0))
        .expect("qualified fixture")
        .expect("qualified fixture");
    let selected = TimeGuardianOwner::planned_definition(&plan).to_owned();
    let receipt = o
        .commit_cast(&mut r, &f, stamp, plan)
        .expect("qualified fixture");
    assert_eq!(receipt.actor, a);
    assert_eq!(receipt.health, before.health);
    assert_eq!(receipt.definition, selected);
    assert_eq!(
        r.current_live_creature_identity(a)
            .expect("qualified fixture"),
        selected.as_bytes()
    );
    assert_eq!(
        r.companion_snapshot(a)
            .expect("qualified fixture")
            .state
            .policy
            .as_ref(),
        &policy(&d, &selected)
    );
    let replay = o
        .prepare_cast(&r, &f, stamp, &s, &p, SemanticTimeMicros::from_micros(1))
        .expect("qualified fixture")
        .err()
        .expect("qualified fixture");
    assert_eq!(replay, receipt);
    assert_eq!(o.pending.len(), 1);
    r.commit_monster_lab_damage(a, b"source-phase-current-damage", 100)
        .expect("qualified fixture");
    assert!(
        o.due_returns(
            &r,
            &f,
            &VirtualOwnerClock::new(SemanticTimeMicros::from_micros(29_999_999))
        )
        .expect("qualified fixture")
        .is_empty()
    );
    let due = o
        .due_returns(
            &r,
            &f,
            &VirtualOwnerClock::new(SemanticTimeMicros::from_micros(30_000_000)),
        )
        .expect("qualified fixture");
    assert_eq!(due.len(), 1);
    let result = o
        .commit_return(
            &mut r,
            &f,
            stamp,
            due.into_iter()
                .next()
                .expect("qualified fixture")
                .expect("qualified fixture"),
        )
        .expect("qualified fixture");
    assert!(result.returned);
    assert_eq!(result.health, before.health - 100);
    assert_eq!(
        r.current_live_creature_identity(a)
            .expect("qualified fixture"),
        BASE.as_bytes()
    );
    assert_eq!(result.epoch, before.state.lifecycle_epoch + 2);
    assert!(o.pending.is_empty());
    assert!(
        o.due_returns(
            &r,
            &f,
            &VirtualOwnerClock::new(SemanticTimeMicros::from_micros(30_000_000))
        )
        .expect("qualified fixture")
        .is_empty()
    );
    assert!(matches!(
        o.prepare_cast(
            &r,
            &f,
            stamp,
            &s,
            &proposal(a, &s, 1),
            SemanticTimeMicros::from_micros(5)
        ),
        Err(AttackError::InvalidPlan)
    ));
}
#[test]
fn distinct_time_source_entries_qualify_and_foreign_owner_source_substitution_never_write() {
    let (mut r, d, a, f, stamp) = fixture();
    let s = source(&r, &d, 1);
    assert_eq!(s.kind(), SevenKind::TimeGuardiann);
    let before = r.companion_snapshot(a).expect("qualified fixture");
    let mut o = TimeGuardianOwner::new(&r).expect("qualified fixture");
    let b = r.binding();
    let mut world = *b.world_id().as_bytes();
    world[15] ^= 1;
    let (foreign, _) = crate::foundation::crystal_timer_fixture(
        RuntimeScopeRefV1::channel(
            WorldId::decode(&world).expect("qualified fixture"),
            b.channel_id(),
        ),
        b.scope_generation(),
    )
    .expect("qualified fixture");
    assert!(matches!(
        o.prepare_cast(
            &r,
            &foreign,
            stamp,
            &s,
            &proposal(a, &s, 0),
            SemanticTimeMicros::from_micros(0)
        ),
        Err(AttackError::StaleOwner)
    ));
    assert!(o.pending.is_empty());
    assert_eq!(r.companion_snapshot(a).expect("qualified fixture"), before);
    let mut forged = d.clone();
    forged
        .state
        .source_identity_bindings
        .iter_mut()
        .find(|b| b.target.key == s.case.encounter)
        .expect("qualified fixture")
        .source_revision = "forged-source-pin".into();
    assert!(matches!(
        SevenSource::qualify(
            &forged,
            &Ref {
                family: Family::Creature,
                key: BASE.into(),
                revision: "definition-r1".into()
            },
            1,
            ScheduleList::Defence,
            s.content
        ),
        Err(AttackError::InvalidSource)
    ));
    let mut malformed = proposal(a, &s, 0);
    malformed.target = crate::gameplay_transport::actor_spell::tests::runtime_with_player(0x78).1;
    assert!(
        o.prepare_cast(
            &r,
            &f,
            stamp,
            &s,
            &malformed,
            SemanticTimeMicros::from_micros(0)
        )
        .is_err()
    );
    assert!(o.pending.is_empty());
    assert_eq!(r.companion_snapshot(a).expect("qualified fixture"), before);
    let p = proposal(a, &s, 0);
    let plan = o
        .prepare_cast(&r, &f, stamp, &s, &p, SemanticTimeMicros::from_micros(0))
        .expect("qualified fixture")
        .expect("qualified fixture");
    o.commit_cast(&mut r, &f, stamp, plan)
        .expect("qualified fixture");
    r.remove_test_actor(a).expect("qualified fixture");
    assert!(
        o.due_returns(
            &r,
            &f,
            &VirtualOwnerClock::new(SemanticTimeMicros::from_micros(30_000_000))
        )
        .expect("qualified fixture")
        .is_empty()
    );
    assert!(o.pending.is_empty());
}
#[test]
fn exact_seven_descriptors_preserve_map_dependency_and_original_two_actor_rules() {
    let (r, d, _, _, _) = fixture();
    let w: Witness = serde_json::from_str(include_str!("source_encounter_seven_fixture.json"))
        .expect("qualified fixture");
    assert_eq!(w.cases.len(), 7);
    for c in &w.cases {
        let reference = Ref {
            family: Family::Creature,
            key: c.creature.clone(),
            revision: "definition-r1".into(),
        };
        let list = if c.list == "attacks" {
            ScheduleList::Attack
        } else {
            ScheduleList::Defence
        };
        assert!(
            SevenSource::qualify(
                &d,
                &reference,
                c.index,
                list,
                r.content_pin().server_artifact_digest()
            )
            .expect("qualified fixture")
            .is_some()
        );
    }
    let raw: serde_json::Value =
        serde_json::from_str(include_str!("source_encounter_seven_fixture.json"))
            .expect("qualified fixture");
    let cases = raw["cases"].as_array().expect("qualified fixture");
    assert!(cases.iter().filter(|c| c["creature"] == BASE).all(|c| {
        c["raw_rules_preserved"]
            .as_array()
            .is_some_and(|r| !r.is_empty())
    }));
}
#[test]
fn sole_return_deadline_survives_real_reservation_refused_commit_and_foreign_scope() {
    use crate::foundation::{CharacterId, CommandId, CommandRef};
    use crate::spell::combat_batch::{
        OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
    };
    let (mut r, player, session) = runtime_with_player(0x72);
    let d = draft(&r);
    r.install_companion_policies(
        CompiledCreaturePolicies::from_active_artifact(
            r.content_pin().server_artifact_digest(),
            [BASE, BLAZING, FREEZING]
                .iter()
                .map(|k| policy(&d, k))
                .collect(),
        )
        .expect("qualified fixture"),
    )
    .expect("qualified fixture");
    let a = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 14,
            },
            BASE,
            290000,
        )
        .expect("qualified fixture");
    r.install_creature_policy(a, BASE)
        .expect("qualified fixture");
    let binding = r.binding();
    let (f, stamp) = crate::foundation::crystal_timer_fixture(
        RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
        binding.scope_generation(),
    )
    .expect("qualified fixture");
    let s = source(&r, &d, 0);
    let mut owner = TimeGuardianOwner::new(&r).expect("qualified fixture");
    let cast = owner
        .prepare_cast(
            &r,
            &f,
            stamp,
            &s,
            &proposal(a, &s, 0),
            SemanticTimeMicros::from_micros(0),
        )
        .expect("qualified fixture")
        .expect("qualified fixture");
    owner
        .commit_cast(&mut r, &f, stamp, cast)
        .expect("qualified fixture");
    // The real existing SQL-coupled batch reservation has no HP write; it blocks physical profile mutation.

    let batch = OwnerCombatBatch {
        caster: player,
        attacker: CharacterId::decode(&[1, 144, 0, 0, 0, 5, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 5])
            .expect("qualified fixture"),
        current_lease_generation: 1,
        command: CommandRef::new(session, CommandId::new(7).expect("qualified fixture")),
        occurrence: SpellOccurrenceBinding {
            id: "test:time-return-reservation".into(),
            revisions: ["rules:1", "content:1", "world:1", "formula:1", "sim:1"].map(str::to_owned),
        },
        binding: b"test:time-return-reservation".to_vec(),
        anchor: None,
        now_ms: 30000,
        effects: vec![OwnerCombatEffect {
            target: a,
            sub_ordinal: 0,
            change: OwnerCombatChange::Damage {
                target_atom: String::from_utf8(
                    r.current_live_creature_identity(a)
                        .expect("qualified fixture")
                        .to_vec(),
                )
                .expect("qualified fixture"),
                magnitude: 1,
            },
        }],
        deferred: None,
    };
    let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(30_000_000));
    let mut reserved = r.stage_spell_batch(&batch).expect("qualified fixture");
    r.reserve_spell_batch(&mut reserved)
        .expect("qualified fixture");
    let before = r.companion_snapshot(a).expect("qualified fixture");
    let refused = owner
        .due_returns(&r, &f, &clock)
        .expect("qualified fixture");
    assert_eq!(refused.len(), 1);
    assert!(refused[0].is_err());
    assert_eq!(owner.pending.len(), 1);
    assert_eq!(r.companion_snapshot(a).expect("qualified fixture"), before);
    r.release_definitely_uncommitted_spell_batch(&reserved)
        .expect("qualified fixture");
    let stale_plan = owner
        .due_returns(&r, &f, &clock)
        .expect("qualified fixture")
        .pop()
        .expect("qualified fixture")
        .expect("qualified fixture");
    r.commit_monster_lab_damage(a, b"time-return-current-health-changed", 1)
        .expect("qualified fixture");
    assert!(owner.commit_return(&mut r, &f, stamp, stale_plan).is_err());
    assert_eq!(owner.pending.len(), 1);
    let b = r.binding();
    let (foreign, foreign_stamp) = crate::foundation::crystal_timer_fixture(
        RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
        crate::foundation::ScopeOwnershipGeneration::new(2).expect("qualified fixture"),
    )
    .expect("qualified fixture");
    let plan = owner
        .due_returns(&r, &f, &clock)
        .expect("qualified fixture")
        .pop()
        .expect("qualified fixture")
        .expect("qualified fixture");
    assert!(matches!(
        owner.commit_return(&mut r, &foreign, foreign_stamp, plan),
        Err(AttackError::StaleOwner)
    ));
    assert_eq!(owner.pending.len(), 1);
    let retry = owner
        .due_returns(&r, &f, &clock)
        .expect("qualified fixture")
        .pop()
        .expect("qualified fixture")
        .expect("qualified fixture");
    let receipt = owner
        .commit_return(&mut r, &f, stamp, retry)
        .expect("qualified fixture");
    assert_eq!(receipt.health, before.health - 1);
    assert!(owner.pending.is_empty());
    assert!(
        owner
            .due_returns(&r, &f, &clock)
            .expect("qualified fixture")
            .is_empty()
    );
}
#[test]
fn two_retained_casts_and_returns_cannot_rewind_the_same_source_owner_clock() {
    use crate::foundation::{ChannelContentPin, NodeId};
    let (base, _, _, _, _) = fixture();
    let b = base.binding();
    let pin: ChannelContentPin = base.content_pin().clone();
    let mut r = ChannelRuntimeV1::from_committed_assignment(
        b.world_id(),
        b.channel_id(),
        NodeId::decode(&[1, 144, 0, 0, 0, 0x62, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 0x62])
            .expect("qualified fixture"),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        3,
        pin,
    )
    .expect("qualified fixture");
    let d = draft(&r);
    r.install_companion_policies(
        CompiledCreaturePolicies::from_active_artifact(
            r.content_pin().server_artifact_digest(),
            [BASE, BLAZING, FREEZING]
                .iter()
                .map(|k| policy(&d, k))
                .collect(),
        )
        .expect("qualified fixture"),
    )
    .expect("qualified fixture");
    let a = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 14,
            },
            BASE,
            290000,
        )
        .expect("qualified fixture");
    let b = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: 101,
                y: 100,
                floor: 14,
            },
            BASE,
            290000,
        )
        .expect("qualified fixture");
    r.install_creature_policy(a, BASE)
        .expect("qualified fixture");
    r.install_creature_policy(b, BASE)
        .expect("qualified fixture");
    let binding = r.binding();
    let (f, stamp) = crate::foundation::crystal_timer_fixture(
        RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
        binding.scope_generation(),
    )
    .expect("qualified fixture");
    let s = source(&r, &d, 0);
    let mut owner = TimeGuardianOwner::new(&r).expect("qualified fixture");
    let old = owner
        .prepare_cast(
            &r,
            &f,
            stamp,
            &s,
            &proposal(a, &s, 0),
            SemanticTimeMicros::from_micros(100),
        )
        .expect("qualified fixture")
        .expect("qualified fixture");
    let newer = owner
        .prepare_cast(
            &r,
            &f,
            stamp,
            &s,
            &proposal(b, &s, 0),
            SemanticTimeMicros::from_micros(200),
        )
        .expect("qualified fixture")
        .expect("qualified fixture");
    owner
        .commit_cast(&mut r, &f, stamp, newer)
        .expect("qualified fixture");
    let before = r.companion_snapshot(a).expect("qualified fixture");
    assert!(matches!(
        owner.commit_cast(&mut r, &f, stamp, old),
        Err(AttackError::InvalidPlan)
    ));
    assert_eq!(r.companion_snapshot(a).expect("qualified fixture"), before);
    assert_eq!(owner.last_time, Some(SemanticTimeMicros::from_micros(200)));
    let current = owner
        .prepare_cast(
            &r,
            &f,
            stamp,
            &s,
            &proposal(a, &s, 0),
            SemanticTimeMicros::from_micros(200),
        )
        .expect("qualified fixture")
        .expect("qualified fixture");
    owner
        .commit_cast(&mut r, &f, stamp, current)
        .expect("qualified fixture");
    let mut due = owner
        .due_returns(
            &r,
            &f,
            &VirtualOwnerClock::new(SemanticTimeMicros::from_micros(30_000_200)),
        )
        .expect("qualified fixture");
    assert_eq!(due.len(), 2);
    let old_return = due.remove(0).expect("qualified fixture");
    let later = owner
        .due_returns(
            &r,
            &f,
            &VirtualOwnerClock::new(SemanticTimeMicros::from_micros(30_000_300)),
        )
        .expect("qualified fixture")
        .into_iter()
        .map(Result::unwrap)
        .find(|p| p.actor != old_return.actor)
        .expect("qualified fixture");
    owner
        .commit_return(&mut r, &f, stamp, later)
        .expect("qualified fixture");
    let retained_actor = old_return.actor;
    let before = r
        .companion_snapshot(retained_actor)
        .expect("qualified fixture");
    assert!(matches!(
        owner.commit_return(&mut r, &f, stamp, old_return),
        Err(AttackError::InvalidPlan)
    ));
    assert_eq!(
        r.companion_snapshot(retained_actor)
            .expect("qualified fixture"),
        before
    );
    assert_eq!(
        owner.last_time,
        Some(SemanticTimeMicros::from_micros(30_000_300))
    );
    let retry = owner
        .due_returns(
            &r,
            &f,
            &VirtualOwnerClock::new(SemanticTimeMicros::from_micros(30_000_300)),
        )
        .expect("qualified fixture")
        .pop()
        .expect("qualified fixture")
        .expect("qualified fixture");
    owner
        .commit_return(&mut r, &f, stamp, retry)
        .expect("qualified fixture");
    assert!(owner.pending.is_empty());
}
