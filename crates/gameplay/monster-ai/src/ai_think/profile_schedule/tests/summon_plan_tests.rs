#![allow(clippy::panic)]
//! Reuse the existing schedule fixture; test the opt-in projection and one selected-target pass.
use super::*;
use crate::ai_think::behaviour_profile::{CreatureBehaviourProfile, ProfileRefusal};
use crate::ai_think::summon_plan::MonsterThinkReader;
use crate::ai_think::targeting::TargetCandidate;
use crate::ai_think::{CreatureAiState, CreatureThinkFacts};
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};

fn ready<T>(future: impl Future<Output = T>) -> T {
    let mut future = std::pin::pin!(future);
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    match future.as_mut().poll(&mut cx) {
        std::task::Poll::Ready(value) => value,
        std::task::Poll::Pending => panic!("fixture reader must be immediately ready"),
    }
}
#[test]
fn summon_projection_is_opt_in_and_enforces_eight_and_sixteen_bounds() {
    let (mut behavior, abilities) = summon_fixture();
    assert!(
        CreatureBehaviourProfile::project(Some(&behavior), &abilities)
            .expect("legacy")
            .behavior()
            .summons
            .is_none()
    );
    let summons = behavior.summons.as_mut().expect("summons");
    summons.max_summons = 16;
    let entry = summons.entries[0].clone();
    summons.entries = (0..8)
        .map(|index| {
            let mut entry = entry.clone();
            entry.creature.key = format!("oteryn:creature.summon-{index}");
            entry.count = 16;
            entry
        })
        .collect();
    assert_eq!(
        CreatureBehaviourProfile::project_with_summons(Some(&behavior), &abilities)
            .expect("8 / 16")
            .behavior()
            .summons,
        behavior.summons
    );
    behavior
        .summons
        .as_mut()
        .expect("summons")
        .entries
        .push(entry);
    assert_eq!(
        CreatureBehaviourProfile::project_with_summons(Some(&behavior), &abilities),
        Err(ProfileRefusal::Invalid)
    );
    behavior.summons.as_mut().expect("summons").entries.pop();
    behavior.summons.as_mut().expect("summons").max_summons = 17;
    assert_eq!(
        CreatureBehaviourProfile::project_with_summons(Some(&behavior), &abilities),
        Err(ProfileRefusal::Invalid)
    );
    behavior.summons.as_mut().expect("summons").max_summons = 16;
    behavior.summons.as_mut().expect("summons").entries[0].count = 17;
    assert_eq!(
        CreatureBehaviourProfile::project_with_summons(Some(&behavior), &abilities),
        Err(ProfileRefusal::Invalid)
    );
}
struct Reader {
    facts: Option<MonsterSummonFacts>,
    selected: Vec<Option<ExactActorRef>>,
}
impl MonsterThinkReader for Reader {
    async fn summon_facts(
        &mut self,
        _: &ChannelRuntimeV1,
        occurrence: ThinkOccurrence,
        target: Option<ExactActorRef>,
        _: Option<&[MovementLocalPosition]>,
    ) -> Option<MonsterSummonFacts> {
        self.selected.push(target);
        self.facts.clone().map(|mut facts| {
            facts.occurrence = occurrence;
            facts
        })
    }
}
#[test]
fn one_think_reports_abilities_and_summons_and_changed_path_conflicts_without_redraw() {
    let (mut behavior, abilities) = summon_fixture();
    behavior.summons.as_mut().expect("summons").entries[0].interval_ms = 1000;
    let profile = CreatureBehaviourProfile::project_with_summons(Some(&behavior), &abilities)
        .expect("profile");
    let mut state = CreatureAiState::new(
        actor(1),
        profile,
        MovementLocalPosition {
            x: 100,
            y: 100,
            floor: 7,
        },
    );
    let runtime = crate::monster_summon::summon_execution_fixture_for_world(actor(1).world_id()).0;
    let players = [TargetCandidate {
        actor: actor(2),
        position: MovementLocalPosition {
            x: 101,
            y: 100,
            floor: 7,
        },
        eligible: true,
        invisible: false,
        health: 100,
        damage: 0,
    }];
    let revisions = revisions();
    let root = root();
    let facts = CreatureThinkFacts {
        position: MovementLocalPosition {
            x: 100,
            y: 100,
            floor: 7,
        },
        health: 100,
        has_active_condition: false,
        players: &players,
        now_us: 0,
        root: &root,
        revisions: &revisions,
    };
    let mut reader = Reader {
        facts: Some(summon_facts(&behavior, 0)),
        selected: Vec::new(),
    };
    let first = ready(state.think_current(0, &facts, &runtime, &mut reader, None)).expect("think");
    assert_eq!(reader.selected, [Some(actor(2))]);
    assert_eq!(first.proposals.len(), 2);
    assert_eq!(first.summon_proposals.len(), 1);
    let replay =
        ready(state.think_current(0, &facts, &runtime, &mut reader, None)).expect("replay");
    assert_eq!(replay.proposals, first.proposals);
    assert_eq!(replay.summon_proposals, first.summon_proposals);
    reader.facts.as_mut().expect("facts").target_with_path = None;
    let changed = ready(state.think_current(0, &facts, &runtime, &mut reader, None))
        .expect("conflicting facts");
    assert_eq!(changed.refusal, Some(ScheduleError::OccurrenceConflict));
    assert!(changed.proposals.is_empty() && changed.summon_proposals.is_empty());
}
#[test]
fn unknown_path_refuses_without_advancing_the_shared_clock() {
    let (behavior, abilities) = summon_fixture();
    let mut schedule = ProfileScheduleState::new(actor(1));

    // Reuse the existing wrappers so their borrowed revision/root storage stays valid.
    assert_eq!(
        run(&mut schedule, 0, &behavior, &abilities, target(Some(1))),
        Err(ScheduleError::SummonClockDependency)
    );
    let plan = summon_run(
        &mut schedule,
        &behavior,
        &abilities,
        &summon_facts(&behavior, 1),
    )
    .expect("retry at next think");
    assert!(
        plan.summon_proposals.is_empty(),
        "a refused occurrence never advances the 3000ms clock"
    );
}
