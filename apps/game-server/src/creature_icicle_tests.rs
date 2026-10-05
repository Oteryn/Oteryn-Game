use super::*;
use crate::ai_think::{ThinkSequenceTracker, profile_schedule::ProfileScheduleState};
use crate::content::{ProjectDraft, ProjectV2Family as Family, ProjectV2State};
use crate::foundation::{ChannelContentPin, ChannelId, MovementLocalPosition, NodeId, WorldId};
fn fixture() -> (
    ChannelRuntimeV1,
    ProjectV2Draft,
    ExactActorRef,
    ExactActorRef,
    GameSessionId,
    IcicleSource,
    ProfileAbilityProposal,
    ScopeRuntimeFence,
    RuntimeWorkStamp,
) {
    let id = |t| [1, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, t];
    let w = WorldId::decode(&id(1)).unwrap();
    let mut r = ChannelRuntimeV1::from_committed_assignment(
        w,
        ChannelId::decode(&id(2)).unwrap(),
        NodeId::decode(&id(3)).unwrap(),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        8,
        ChannelContentPin::test(w),
    )
    .unwrap();
    let session = GameSessionId::decode(&id(8)).unwrap();
    let reserve = r.reserve_fresh_session(session).unwrap();
    let player = r.commit_fresh_session(reserve).unwrap();
    r.initialize_first_entry_position(player).unwrap();
    let at = r.read_actor_position(player).unwrap().position();
    let caster = r
        .admit_source_pinned_lab_creature(
            MovementLocalPosition {
                x: at.x + 1,
                y: at.y,
                floor: at.floor,
            },
            CASTER,
            8200,
        )
        .unwrap();
    let v: serde_json::Value =
        serde_json::from_str(include_str!("icicle_source_fixture.json")).unwrap();
    let d = ProjectV2Draft {
        core: ProjectDraft {
            project_revision: "fixture-r1".into(),
            package_key: "oteryn:fixture".into(),
            semantic_schema_version: "v2".into(),
            licensing_metadata: "fixture".into(),
            world_id: hex(w.as_bytes()),
            coordinate_frame: "fixture".into(),
            records: serde_json::from_value(v["test_records"].clone()).unwrap(),
            imports: vec![],
            metadata: vec![],
        },
        state: ProjectV2State {
            authoring_profiles: serde_json::from_value(v["test_profiles"].clone()).unwrap(),
            ..Default::default()
        },
    };
    let cref = Ref {
        family: Family::Creature,
        key: CASTER.into(),
        revision: "definition-r1".into(),
    };
    let source = IcicleSource::qualify(&d, &cref, 0, [1; 32]).unwrap();
    let behavior = d
        .state
        .authoring_profiles
        .iter()
        .find_map(|p| match &p.data {
            Data::Behavior(b) if p.target.key == "oteryn:behavior.creature.icicle" => Some(b),
            _ => None,
        })
        .unwrap();
    let abilities = d
        .state
        .authoring_profiles
        .iter()
        .filter_map(|p| match &p.data {
            Data::Ability(a) => Some((p.target.clone(), a.clone())),
            _ => None,
        })
        .collect();
    let revisions = crate::ability::RevisionSet::new(
        "rules-r1",
        "content-r1",
        "policy-r1",
        "definition-r1",
        "sim-r1",
    )
    .unwrap();
    let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes([1; 32]);
    let mut schedule = ProfileScheduleState::new(caster);
    let mut tracker = ThinkSequenceTracker::new();
    let mut proposal = None;
    for _ in 0..256 {
        let plan = schedule
            .prepare(
                tracker.next_occurrence(caster),
                behavior,
                &abilities,
                None,
                &revisions,
                &root,
            )
            .unwrap();
        proposal = plan
            .proposals
            .into_iter()
            .find(|p| p.list == ScheduleList::Defence && p.ability == source.ability);
        if proposal.is_some() {
            break;
        }
    }
    let b = r.binding();
    let (fence, stamp) = crate::foundation::crystal_timer_fixture(
        RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
        b.scope_generation(),
    )
    .unwrap();
    (
        r,
        d,
        caster,
        player,
        session,
        source,
        proposal.unwrap(),
        fence,
        stamp,
    )
}
fn egg(r: &mut ChannelRuntimeV1, caster: ExactActorRef, x: i32, y: i32, hp: i64) -> ExactActorRef {
    let at = r.read_actor_position(caster).unwrap().position();
    r.admit_source_pinned_lab_creature(
        MovementLocalPosition {
            x: at.x + x,
            y: at.y + y,
            floor: at.floor,
        },
        EGG,
        hp,
    )
    .unwrap()
}
struct Facts {
    missing: Option<ExactActorRef>,
    allow: bool,
}
impl IcicleWorldReader for Facts {
    fn icicle_callback_allowed(
        &mut self,
        r: &ChannelRuntimeV1,
        _: ExactActorRef,
        target: ExactActorRef,
        session: Option<GameSessionId>,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        if self.missing == Some(target) {
            return None;
        }
        let p = r.read_actor_position(target).ok()?;
        if p.context() != r.pinned_movement_context() {
            return None;
        }
        if let Some(s) = session {
            r.player_control_facts(target, s).ok()?;
        } else if !r.contains_live_creature(target) {
            return None;
        }
        Some(self.allow)
    }
}
fn active_ai(
    r: &ChannelRuntimeV1,
    d: &ProjectV2Draft,
    caster: ExactActorRef,
    player: ExactActorRef,
    session: GameSessionId,
) -> AutoAttackOwner {
    let source = crate::creature_auto_attack::MeleeSource::from_native(
        &Ref {
            family: Family::Creature,
            key: CASTER.into(),
            revision: "definition-r1".into(),
        },
        0,
        &d.core.records,
        &d.state.authoring_profiles,
        [1; 32],
    )
    .unwrap();
    let mut ai = AutoAttackOwner::default();
    ai.set_target(
        r,
        caster,
        player,
        session,
        &source,
        "icicle-test-current-player",
        SemanticTimeMicros::from_micros(0),
    )
    .unwrap();
    assert!(ai.has_active_target_test(caster));
    ai
}
#[test]
fn icicle_source_raw_hp_death_ai_clear_and_same_occurrence_replay() {
    let (mut r, d, c, p, s, source, proposal, f, stamp) = fixture();
    let a = egg(&mut r, c, 1, 0, 120);
    let b = egg(&mut r, c, 2, 0, 50);
    let mut ai = active_ai(&r, &d, c, p, s);
    let mut owner = IcicleOwner::default();
    let mut facts = Facts {
        missing: None,
        allow: true,
    };
    let result = owner
        .execute(
            &mut r,
            &f,
            stamp,
            &source,
            &proposal,
            &mut ai,
            &mut facts,
            SemanticTimeMicros::from_micros(1_000_000),
        )
        .unwrap();
    assert_eq!(r.read_source_creature_health(a, EGG, 5000).unwrap(), 20);
    assert!(!r.contains_live_creature(b));
    assert_eq!(
        result
            .eggs
            .iter()
            .find(|(actor, _)| *actor == b)
            .unwrap()
            .1
            .health_after,
        0
    );
    r.borrow_combat_death().committed_lethal_receipt(b).unwrap();
    assert!(result.cleared_target);
    assert!(!ai.has_active_target_test(c));
    assert_eq!(
        owner
            .execute(
                &mut r,
                &f,
                stamp,
                &source,
                &proposal,
                &mut ai,
                &mut facts,
                SemanticTimeMicros::from_micros(2_000_000)
            )
            .unwrap(),
        result
    );
    assert_eq!(r.read_source_creature_health(a, EGG, 5000).unwrap(), 20);
    let mut forged = proposal.clone();
    forged.target = p;
    assert_eq!(
        owner.execute(
            &mut r,
            &f,
            stamp,
            &source,
            &forged,
            &mut ai,
            &mut facts,
            SemanticTimeMicros::from_micros(2_000_000)
        ),
        Err(IcicleError::Source)
    );
}
#[test]
fn icicle_missing_late_callback_and_wrong_phase_leave_hp_ai_and_occurrence_unwritten() {
    let (mut r, d, c, p, s, source, proposal, f, stamp) = fixture();
    let a = egg(&mut r, c, 1, 0, 500);
    let b = egg(&mut r, c, 2, 0, 500);
    let mut ai = active_ai(&r, &d, c, p, s);
    let mut owner = IcicleOwner::default();
    let mut facts = Facts {
        missing: Some(b),
        allow: true,
    };
    assert_eq!(
        owner.execute(
            &mut r,
            &f,
            stamp,
            &source,
            &proposal,
            &mut ai,
            &mut facts,
            SemanticTimeMicros::from_micros(1_000_000)
        ),
        Err(IcicleError::MissingFacts)
    );
    assert_eq!(r.read_source_creature_health(a, EGG, 5000).unwrap(), 500);
    assert_eq!(r.read_source_creature_health(b, EGG, 5000).unwrap(), 500);
    assert!(ai.has_active_target_test(c));
    assert!(owner.memos.is_empty());
    let mut substituted = source.clone();
    substituted.content = [9; 32];
    assert_eq!(
        owner.execute(
            &mut r,
            &f,
            stamp,
            &substituted,
            &proposal,
            &mut ai,
            &mut facts,
            SemanticTimeMicros::from_micros(1_000_000)
        ),
        Err(IcicleError::Actor)
    );
    facts.missing = None;
    owner
        .execute(
            &mut r,
            &f,
            stamp,
            &source,
            &proposal,
            &mut ai,
            &mut facts,
            SemanticTimeMicros::from_micros(1_000_000),
        )
        .unwrap();
    assert_eq!(r.read_source_creature_health(a, EGG, 5000).unwrap(), 400);
    let current = r.binding();
    let mut foreign = *current.world_id().as_bytes();
    foreign[15] = 99;
    let (foreign_fence, _) = crate::foundation::crystal_timer_fixture(
        RuntimeScopeRefV1::channel(WorldId::decode(&foreign).unwrap(), current.channel_id()),
        current.scope_generation(),
    )
    .unwrap();
    assert_eq!(
        owner.execute(
            &mut r,
            &foreign_fence,
            stamp,
            &source,
            &proposal,
            &mut ai,
            &mut facts,
            SemanticTimeMicros::from_micros(2_000_000)
        ),
        Err(IcicleError::Fence)
    );
    let mut wrong = d.clone();
    let ix = wrong
        .state
        .authoring_profiles
        .iter()
        .position(|p| p.target.key == source.ability.key)
        .unwrap();
    wrong.state.authoring_profiles.remove(ix);
    assert!(
        IcicleSource::qualify(
            &wrong,
            &Ref {
                family: Family::Creature,
                key: CASTER.into(),
                revision: "definition-r1".into()
            },
            0,
            [1; 32]
        )
        .is_err()
    );
}
#[test]
fn icicle_rectangle_spectator_outside37cells_clears_ai_without_hp_or_damage_pipeline() {
    let (mut r, d, c, p, s, source, proposal, f, stamp) = fixture();
    let a = egg(&mut r, c, 3, 3, 500);
    let mut ai = active_ai(&r, &d, c, p, s);
    let mut owner = IcicleOwner::default();
    let mut facts = Facts {
        missing: None,
        allow: true,
    };
    let hit = owner
        .execute(
            &mut r,
            &f,
            stamp,
            &source,
            &proposal,
            &mut ai,
            &mut facts,
            SemanticTimeMicros::from_micros(1_000_000),
        )
        .unwrap();
    assert!(hit.cleared_target);
    assert!(hit.eggs.is_empty());
    assert!(!ai.has_active_target_test(c));
    assert_eq!(r.read_source_creature_health(a, EGG, 5000).unwrap(), 500);
}

#[test]
fn icicle_native_mixed_targets_and_receipt_failure_preflight_leave_first_slot_unchanged() {
    let (mut r, _, c, _, _, _, _, _, _) = fixture();
    let a = egg(&mut r, c, 1, 0, 500);
    assert!(
        r.borrow_exact_actor_commit()
            .commit_icicle_egg_raw_batch(
                &[a, c],
                b"mixed-cast",
                b"mixed-cast\0native-source",
                |_| Ok(())
            )
            .is_err()
    );
    assert_eq!(r.read_source_creature_health(a, EGG, 5000).unwrap(), 500);
    assert!(
        r.borrow_exact_actor_commit()
            .commit_icicle_egg_raw_batch(
                &[a],
                b"receipt-refusal",
                b"receipt-refusal\0native-source",
                |_| Err::<(), _>(crate::foundation::CarrierError::AllocationFailed)
            )
            .is_err()
    );
    assert_eq!(r.read_source_creature_health(a, EGG, 5000).unwrap(), 500);
    let result = r
        .borrow_exact_actor_commit()
        .commit_icicle_egg_raw_batch(
            &[a],
            b"receipt-refusal",
            b"receipt-refusal\0native-source",
            |r| Ok(r.to_vec()),
        )
        .unwrap();
    assert_eq!(result[0].health_after, 400);
}
