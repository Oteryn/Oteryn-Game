#![allow(clippy::expect_used, clippy::panic)]
#![allow(clippy::unwrap_used)]
use super::*;
use crate::ai_think::{
    ThinkSequenceTracker,
    profile_schedule::{
        AttackTarget, MonsterSummonFacts, ObservedSummonCount, ProfileScheduleInput,
        ProfileScheduleState,
    },
};
use crate::content::native_gameplay::{NativeGameplayInput, NativeGameplayState};
use crate::foundation::{ChannelContentPin, ChannelId, MovementLocalPosition, NodeId};
use crate::gameplay_transport::actor_spell::{ChannelSpellStates, tests::FACTS};
fn actual_content() -> (NativeGameplayState, ChannelContentPin) {
    let path = std::env::var_os("OTERYN_FULL_SPELL_TEST_MANIFEST")
        .expect("explicit actual source artifact manifest");
    let input = NativeGameplayInput::from_manifest(std::path::Path::new(&path)).unwrap();
    let world = crate::content::native_gameplay_test_world_id().unwrap();
    let room = crate::content::qualify_selected_native_gameplay_room(world, &input).unwrap();
    let compiled = room.compiled();
    let native = crate::content::qualified_native_gameplay_test_state(compiled).unwrap();
    let at = room.entry_start();
    let pin = ChannelContentPin::from_activation(
        world,
        1,
        compiled.server_digest(),
        compiled.client_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (at.x, at.y, at.floor),
    );
    (native, pin)
}
fn packet() -> serde_json::Value {
    serde_json::from_str(include_str!("creature_appearance_execution_data.json")).unwrap()
}
struct World<'a> {
    native: &'a NativeGameplayState,
    target: ExactActorRef,
    session: GameSessionId,
    missing: bool,
    blocked: bool,
    content_present: bool,
}
impl SpellWorldReader for World<'_> {
    fn native_gameplay<'a>(&'a self, _: &ChannelRuntimeV1) -> Option<&'a NativeGameplayState> {
        self.content_present.then_some(self.native)
    }
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
        i: ExactActorRef,
        _: i32,
        _: i32,
        z: i16,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        let p = r.read_actor_position(i).ok()?;
        (!self.missing).then_some(
            !self.blocked && p.position().floor == z && p.context() == r.pinned_movement_context(),
        )
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
        r.player_control_facts(t, s).ok()?;
        r.read_actor_position(i).ok()?;
        if self.missing {
            return None;
        }
        Some(SpellCombatFacts {
            attack: AttackFacts {
                issuer: i,
                target: t,
                session: s,
                revision: 1,
                visible: true,
                issuer_pz: false,
                target_pz: self.blocked,
                issuer_protected: false,
                target_protected: false,
                defense: 0,
                armor: 0,
            },
            multiplier_ppm: 1_000_000,
            immune: false,
            condition_policy: Some(CurrentConditionPolicy {
                base_speed: 180,
                immunities: vec![],
            }),
        })
    }
}
fn fixture<'a>(
    native: &'a NativeGameplayState,
    pin: ChannelContentPin,
    name: &str,
    index: usize,
    defense: bool,
) -> (
    ChannelRuntimeV1,
    ChannelSpellStates,
    SpellSource,
    ProfileAbilityProposal,
    World<'a>,
    ScopeRuntimeFence,
    RuntimeWorkStamp,
) {
    let id = |t: u8| [1, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, t];
    let mut r = ChannelRuntimeV1::from_committed_assignment(
        pin.world_id(),
        ChannelId::decode(&id(2)).unwrap(),
        NodeId::decode(&id(3)).unwrap(),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        8,
        pin,
    )
    .unwrap();
    let s = GameSessionId::decode(&id(4)).unwrap();
    let slot = r.reserve_fresh_session(s).unwrap();
    let target = r.commit_fresh_session(slot).unwrap();
    r.initialize_first_entry_position(target).unwrap();
    let at = r.read_actor_position(target).unwrap().position();
    let v = packet();
    let records: Vec<ProjectReferenceRecord> =
        serde_json::from_value(v["records"].clone()).unwrap();
    let profiles: Vec<ProjectV2AuthoringProfile> =
        serde_json::from_value(v["authoring_profiles"].clone()).unwrap();
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
        .unwrap();
    let issuer = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: at.x + 1,
                y: at.y,
                floor: at.floor,
            },
            &creature.key,
            health as i64,
        )
        .unwrap();
    let source = if defense {
        SpellSource::from_native_appearance_defense(
            &creature,
            index,
            &records,
            &profiles,
            native.source_digest(),
        )
    } else {
        SpellSource::from_native(
            &creature,
            index,
            &records,
            &profiles,
            native.source_digest(),
        )
    }
    .unwrap();
    let behavior = profiles
        .iter()
        .find_map(|p| match &p.data {
            Data::Behavior(b) if p.target.key == format!("oteryn:behavior.creature.{name}") => {
                Some(b)
            }
            _ => None,
        })
        .unwrap();
    let abilities = profiles
        .iter()
        .filter_map(|p| match &p.data {
            Data::Ability(a) => Some((p.target.clone(), a.clone())),
            _ => None,
        })
        .collect();
    let root = GameplayDecisionRoot::from_bytes(native.source_digest());
    let revisions = crate::ability::RevisionSet::new(
        "rules-r1",
        "content-r1",
        "policy-r1",
        "definition-r1",
        "sim-r1",
    )
    .unwrap();
    let mut schedule = ProfileScheduleState::new(issuer);
    let mut tracker = ThinkSequenceTracker::new();
    let mut found = None;
    for _ in 0..4096 {
        let occurrence = tracker.next_occurrence(issuer);
        let summons = behavior.summons.as_ref().map(|b| MonsterSummonFacts {
            occurrence,
            is_summon: false,
            target_with_path: None,
            total_count: 0,
            entry_counts: b
                .entries
                .iter()
                .map(|e| ObservedSummonCount {
                    creature: e.creature.clone(),
                    count: 0,
                })
                .collect(),
        });
        let plan = schedule
            .prepare_with_summons(
                ProfileScheduleInput {
                    occurrence,
                    behavior,
                    abilities: &abilities,
                    target: Some(AttackTarget {
                        actor: target,
                        same_floor_distance: Some(1),
                    }),
                    revisions: &revisions,
                    root: &root,
                },
                summons.as_ref(),
            )
            .unwrap();
        found = plan
            .proposals
            .into_iter()
            .find(|p| p.ability == source.ability);
        if found.is_some() {
            break;
        }
    }
    let proposal = found.expect("actual source schedule occurrence");
    let mut states = ChannelSpellStates::default();
    states
        .initialize(
            &r,
            target,
            s,
            FACTS,
            (0, 0),
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
        )
        .unwrap();
    let b = r.binding();
    let (fence, stamp) = crate::foundation::crystal_timer_fixture(
        crate::foundation::RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
        b.scope_generation(),
    )
    .unwrap();
    (
        r,
        states,
        source,
        proposal,
        World {
            native,
            target,
            session: s,
            missing: false,
            blocked: false,
            content_present: true,
        },
        fence,
        stamp,
    )
}
#[test]
#[ignore = "requires actual OTERYN_FULL_SPELL_TEST_MANIFEST"]
fn appearance_all120_source_bodies_resolve_current_native110_creature10_items() {
    let (native, pin) = actual_content();
    let v = packet();
    let records: Vec<ProjectReferenceRecord> =
        serde_json::from_value(v["records"].clone()).unwrap();
    let profiles: Vec<ProjectV2AuthoringProfile> =
        serde_json::from_value(v["authoring_profiles"].clone()).unwrap();
    let (mut creatures, mut items, mut defenses) = (0, 0, 0);
    for case in v["cases"].as_array().unwrap() {
        let creature = serde_json::from_value(case["creature"].clone()).unwrap();
        let i = case["entry"].as_u64().unwrap() as usize;
        let source = if case["list"] == "defenses" {
            defenses += 1;
            SpellSource::from_native_appearance_defense(
                &creature,
                i,
                &records,
                &profiles,
                native.source_digest(),
            )
        } else {
            SpellSource::from_native(&creature, i, &records, &profiles, native.source_digest())
        }
        .unwrap();
        let id = |t: u8| [1, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, t];
        let r = ChannelRuntimeV1::from_committed_assignment(
            pin.world_id(),
            ChannelId::decode(&id(2)).unwrap(),
            NodeId::decode(&id(3)).unwrap(),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            1,
            pin.clone(),
        )
        .unwrap();
        let defs = source.current_appearance_conditions(&r, &native).unwrap();
        assert_eq!(defs.len(), 1);
        match defs[0].values() {
            crate::foundation::ConditionValues::Outfit { .. } => {
                creatures += 1;
                assert!(defs[0].source_appearance_selection().is_some());
            }
            crate::foundation::ConditionValues::ItemOutfit { .. } => {
                items += 1;
                assert_eq!(
                    defs[0].source_item_appearance().unwrap().artifact_digest(),
                    native.source_digest()
                );
            }
            _ => panic!("wrong source primitive"),
        }
    }
    assert_eq!((creatures, items, defenses), (110, 10, 56));
}
#[test]
#[ignore = "requires actual OTERYN_FULL_SPELL_TEST_MANIFEST"]
fn scheduled_appearance_real_player_store_projection_expiry_and_replay() {
    let (native, pin) = actual_content();
    for (index, item) in [(4, false), (5, true)] {
        let (mut r, mut states, source, p, mut world, fence, stamp) =
            fixture(&native, pin.clone(), "dark_apprentice", index, false);
        let before = states
            .read_owned_player_state_test_snapshot(&r, p.target, world.session)
            .unwrap()
            .clone();
        let mut owner = DamageSpellOwner::default();
        let now = SemanticTimeMicros::from_micros(1);
        let first = owner
            .execute(
                &mut r,
                &fence,
                stamp,
                &mut states,
                &source,
                &p,
                &mut world,
                now,
            )
            .unwrap();
        assert_eq!(first.targets.len(), if item { 2 } else { 1 });
        assert!(first.targets[0].1.as_ref().unwrap().condition_only_applied);
        assert_eq!(
            states
                .read_owned_player_state_test_snapshot(&r, p.target, world.session)
                .unwrap()
                .vitals(),
            before.vitals()
        );
        assert!(
            r.actor_conditions(p.target, Some(world.session))
                .unwrap()
                .instances()
                .is_empty()
        );
        let store = states
            .read_owned_player_state_test_snapshot(&r, p.target, world.session)
            .unwrap()
            .owned_conditions();
        let shown = store.displayed_temporary_appearance_at(1).unwrap();
        match shown {
            crate::ability::condition::TemporaryDisplayedAppearance::Item(v) => {
                assert!(item);
                assert_eq!(v.definition_key(), "oteryn:item.tibia.i2324");
                assert_eq!(v.artifact_digest(), native.source_digest());
            }
            crate::ability::condition::TemporaryDisplayedAppearance::Outfit(v) => {
                assert!(!item);
                assert_eq!(
                    *v,
                    native
                        .spell_appearances()
                        .unwrap()
                        .for_creature("oteryn:creature.cyclops", "definition-r1")
                        .unwrap()
                        .selection()
                );
            }
        }
        assert!(store.displayed_temporary_appearance_at(2_000_001).is_none());
        let applied = states
            .read_owned_player_state_test_snapshot(&r, p.target, world.session)
            .unwrap()
            .clone();
        let retry = owner
            .execute(
                &mut r,
                &fence,
                stamp,
                &mut states,
                &source,
                &p,
                &mut world,
                now,
            )
            .unwrap();
        assert_eq!(first, retry);
        assert_eq!(
            states.read_owned_player_state_test_snapshot(&r, p.target, world.session),
            Some(&applied)
        );
    }
}
#[test]
#[ignore = "requires actual OTERYN_FULL_SPELL_TEST_MANIFEST"]
fn scheduled_appearance_missing_artifact_tile_refusal_wrong_revision_and_stale_owner_no_mutation() {
    let (native, pin) = actual_content();
    let (mut r, mut states, source, p, mut world, fence, stamp) =
        fixture(&native, pin.clone(), "dark_apprentice", 4, false);
    let before = states
        .read_owned_player_state_test_snapshot(&r, p.target, world.session)
        .unwrap()
        .clone();
    let now = SemanticTimeMicros::from_micros(1);
    let mut owner = DamageSpellOwner::default();
    world.content_present = false;
    assert_eq!(
        owner.execute(
            &mut r,
            &fence,
            stamp,
            &mut states,
            &source,
            &p,
            &mut world,
            now
        ),
        Err(AttackError::MissingCombatFacts)
    );
    assert!(owner.casts.is_empty());
    assert_eq!(
        states.read_owned_player_state_test_snapshot(&r, p.target, world.session),
        Some(&before)
    );
    world.content_present = true;
    let mut wrong = source.clone();
    if let crate::content::ProjectV2InlineEffectOperation::AppearanceTransform {
        creature: Some(c),
        ..
    } = &mut wrong.appearances[0].operation
    {
        c.revision = "definition-r2".into();
    }
    assert_eq!(
        owner.execute(
            &mut r,
            &fence,
            stamp,
            &mut states,
            &wrong,
            &p,
            &mut world,
            now
        ),
        Err(AttackError::InvalidSource)
    );
    assert!(owner.casts.is_empty());
    assert_eq!(
        states.read_owned_player_state_test_snapshot(&r, p.target, world.session),
        Some(&before)
    );
    world.blocked = true;
    let refused = owner.execute(
        &mut r,
        &fence,
        stamp,
        &mut states,
        &source,
        &p,
        &mut world,
        now,
    );
    assert!(
        refused.is_err()
            || refused
                .unwrap()
                .targets
                .iter()
                .all(|(_, o)| o.as_ref().map_or(true, |o| !o.condition_only_applied))
    );
    assert_eq!(
        states.read_owned_player_state_test_snapshot(&r, p.target, world.session),
        Some(&before)
    );
}
#[test]
#[ignore = "requires actual OTERYN_FULL_SPELL_TEST_MANIFEST"]
fn source_target_flag_defense_still_mutates_sole_native_creature_slot() {
    let (native, pin) = actual_content();
    let (mut r, mut states, source, p, mut world, fence, stamp) =
        fixture(&native, pin, "dark_apprentice", 1, true);
    assert_eq!(p.target, p.issuer);
    let player = states
        .read_owned_player_state_test_snapshot(&r, world.target, world.session)
        .unwrap()
        .clone();
    let mut owner = DamageSpellOwner::default();
    let now = SemanticTimeMicros::from_micros(1);
    let first = owner
        .execute(
            &mut r,
            &fence,
            stamp,
            &mut states,
            &source,
            &p,
            &mut world,
            now,
        )
        .unwrap();
    assert_eq!(first.targets.len(), 1);
    assert_eq!(first.targets[0].0, p.issuer);
    let expected = native
        .spell_appearances()
        .unwrap()
        .for_creature("oteryn:creature.green_frog", "definition-r1")
        .unwrap()
        .look_type();
    assert_eq!(
        r.actor_conditions(p.issuer, None).unwrap().outfit_at(1),
        Some(expected)
    );
    assert_eq!(
        r.actor_conditions(p.issuer, None)
            .unwrap()
            .outfit_at(3_000_001),
        None
    );
    assert_eq!(
        owner
            .execute(
                &mut r,
                &fence,
                stamp,
                &mut states,
                &source,
                &p,
                &mut world,
                now
            )
            .unwrap(),
        first
    );
    assert_eq!(
        states.read_owned_player_state_test_snapshot(&r, world.target, world.session),
        Some(&player)
    );
}
#[test]
#[ignore = "requires actual OTERYN_FULL_SPELL_TEST_MANIFEST"]
fn appearance_area_late_player_clock_refusal_rolls_back_creature_and_players() {
    let (native, pin) = actual_content();
    let (mut r, mut states, source, p, mut world, fence, stamp) =
        fixture(&native, pin, "dark_apprentice", 5, false);
    let root = GameplayDecisionRoot::from_bytes(native.source_digest());
    let def = crate::ability::condition::ConditionDefinition::new(
        "fixture:future",
        1,
        crate::ability::condition::ConditionValues::Invisible { duration_ms: 1000 },
    )
    .unwrap();
    let facts = crate::ability::condition::ApplicationFacts {
        now: 10,
        base_speed: 180,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &root,
        occurrence: DecisionOccurrenceId::from_bytes([7; 16]),
    };
    assert!(states.install_owned_player_self_use_test_condition(
        &r,
        &fence,
        stamp,
        p.target,
        world.session,
        &def,
        &facts
    ));
    let player = states
        .read_owned_player_state_test_snapshot(&r, p.target, world.session)
        .unwrap()
        .clone();
    let creature = r.actor_conditions(p.issuer, None).unwrap().clone();
    let mut owner = DamageSpellOwner::default();
    assert_eq!(
        owner.execute(
            &mut r,
            &fence,
            stamp,
            &mut states,
            &source,
            &p,
            &mut world,
            SemanticTimeMicros::from_micros(1)
        ),
        Err(AttackError::MissingVitals)
    );
    assert_eq!(
        states.read_owned_player_state_test_snapshot(&r, p.target, world.session),
        Some(&player)
    );
    assert_eq!(r.actor_conditions(p.issuer, None).unwrap(), &creature);
    let binding = r.binding();
    let (stale, wrong_stamp) = crate::foundation::crystal_timer_fixture(
        crate::foundation::RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
        crate::foundation::ScopeOwnershipGeneration::new(binding.scope_generation().get() + 1)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        owner.execute(
            &mut r,
            &stale,
            wrong_stamp,
            &mut states,
            &source,
            &p,
            &mut world,
            SemanticTimeMicros::from_micros(10)
        ),
        Err(AttackError::StaleOwner)
    );
    assert_eq!(
        states.read_owned_player_state_test_snapshot(&r, p.target, world.session),
        Some(&player)
    );
    assert_eq!(r.actor_conditions(p.issuer, None).unwrap(), &creature);
}

#[test]
#[ignore = "requires actual OTERYN_FULL_SPELL_TEST_MANIFEST"]
fn mixed_appearance_receipts_keep_longer_player_outfit_and_apply_new_creature_item() {
    let (native, pin) = actual_content();
    let (mut r, mut states, source, p, mut world, fence, stamp) =
        fixture(&native, pin, "dark_apprentice", 5, false);
    let root = GameplayDecisionRoot::from_bytes(native.source_digest());
    let member = native
        .spell_appearances()
        .unwrap()
        .for_creature("oteryn:creature.cyclops", "definition-r1")
        .unwrap();
    let selection = member.selection();
    let def = crate::ability::condition::ConditionDefinition::new(
        "fixture:longer-outfit",
        1,
        crate::ability::condition::ConditionValues::Outfit {
            duration_ms: 10000,
            look_type: member.look_type(),
        },
    )
    .unwrap()
    .with_appearance(selection)
    .unwrap();
    let facts = crate::ability::condition::ApplicationFacts {
        now: 1,
        base_speed: 180,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &root,
        occurrence: DecisionOccurrenceId::from_bytes([8; 16]),
    };
    assert!(states.install_owned_player_self_use_test_condition(
        &r,
        &fence,
        stamp,
        p.target,
        world.session,
        &def,
        &facts
    ));
    let player = states
        .read_owned_player_state_test_snapshot(&r, p.target, world.session)
        .unwrap()
        .clone();
    let mut owner = DamageSpellOwner::default();
    let now = SemanticTimeMicros::from_micros(2);
    let out = owner
        .execute(
            &mut r,
            &fence,
            stamp,
            &mut states,
            &source,
            &p,
            &mut world,
            now,
        )
        .unwrap();
    assert_eq!(out.targets.len(), 2);
    assert!(
        !out.targets
            .iter()
            .find(|(actor, _)| *actor == p.target)
            .unwrap()
            .1
            .as_ref()
            .unwrap()
            .condition_only_applied
    );
    assert!(
        out.targets
            .iter()
            .find(|(actor, _)| *actor == p.issuer)
            .unwrap()
            .1
            .as_ref()
            .unwrap()
            .condition_only_applied
    );
    assert_eq!(
        states
            .read_owned_player_state_test_snapshot(&r, p.target, world.session)
            .unwrap()
            .owned_conditions()
            .get(crate::ability::condition::ConflictKey::Outfit),
        player
            .owned_conditions()
            .get(crate::ability::condition::ConflictKey::Outfit)
    );
    assert!(matches!(
        r.actor_conditions(p.issuer, None)
            .unwrap()
            .displayed_temporary_appearance_at(2),
        Some(_)
    ));
    assert_eq!(
        owner
            .execute(
                &mut r,
                &fence,
                stamp,
                &mut states,
                &source,
                &p,
                &mut world,
                now
            )
            .unwrap(),
        out
    );
}
