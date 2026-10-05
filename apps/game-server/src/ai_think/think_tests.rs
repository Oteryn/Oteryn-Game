#![allow(
    clippy::expect_used,
    reason = "test fixtures and assertions must fail on missing evidence"
)]

//! CREATURE-AI-1 §2.1: the think engine (targeting, steps, schedule) and its limits
//! (`CREATUREAI0-RL-05`, `-06`, `-07`, `-18`).

use super::behaviour_profile::{CREATUREAI0_PROPOSALS_PER_THINK, ProfileRefusal};
use super::profile_schedule::ScheduleList;
use super::*;
use crate::ability::{AbilityOccurrence, AiAbilityAdapter};
use crate::content::{
    ProjectV2AbilityAuthoring, ProjectV2BehaviorAuthoring, ProjectV2DefinitionRef,
};
use serde_json::{Value, json};

type Abilities = BTreeMap<ProjectV2DefinitionRef, ProjectV2AbilityAuthoring>;

fn actor_n(seed: u16) -> ExactActorRef {
    let mut bytes = [0; 16];
    bytes[0] = 1;
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    let world = crate::foundation::WorldId::decode(&bytes).expect("world fixture");
    bytes[14..].copy_from_slice(&seed.to_be_bytes());
    ExactActorRef::transport_fixture(
        world,
        crate::foundation::ChannelId::decode(&bytes).expect("channel fixture"),
    )
}

fn rat() -> ExactActorRef {
    actor_n(1)
}

fn revisions() -> RevisionSet {
    RevisionSet::new("rules-r1", "content-r1", "world-r1", "formula-r1", "sim-r1")
        .expect("revisions")
}

fn root() -> GameplayDecisionRoot {
    GameplayDecisionRoot::from_bytes([9; 32])
}

fn at(x: i32, y: i32) -> MovementLocalPosition {
    MovementLocalPosition { x, y, floor: 7 }
}

fn player(seed: u16, position: MovementLocalPosition) -> TargetCandidate {
    TargetCandidate {
        actor: actor_n(seed),
        position,
        eligible: true,
        health: 100,
        damage: 0,
    }
}

fn reference(key: &str) -> Value {
    json!({"family":"Ability","key":format!("oteryn:ability.{key}"),"revision":"definition-r1"})
}

fn abilities() -> Abilities {
    [("bite", "Melee", 1), ("bolt", "Spell", 3), ("heal", "Spell", 0)]
        .into_iter()
        .map(|(key, kind, range)| {
            (
                serde_json::from_value(reference(key)).expect("reference"),
                serde_json::from_value(json!({"details":{"kind":kind,"range_tiles":range,"needs_target":true,"needs_direction":false}}))
                    .expect("ability profile"),
            )
        })
        .collect()
}

fn entry(key: &str, interval_ms: u64, maximum: u64) -> Value {
    json!({"ability":reference(key),"interval_ms":interval_ms,"chance_ppm":1_000_000,"magnitude":{"minimum":0,"maximum":maximum}})
}

fn rat_targeting() -> Value {
    json!({"hostile":true,"can_target":true,"sense_invisible":false,"target_distance_tiles":1,
        "static_attack_chance_ppm":900_000,"flee_health":5,
        "change_target":{"interval_ms":4000,"chance_ppm":0},
        "strategy_weights":{"nearest":100,"damage":0,"health":0,"random":0}})
}

fn behavior(
    targeting: Value,
    attacks: Vec<Value>,
    defenses: Vec<Value>,
    wander: Option<Value>,
) -> ProjectV2BehaviorAuthoring {
    let mut movement = json!({"can_walk":true,"pass_through":false,"pushable":false,"push_items":false,"push_creatures":false,"walks_on_energy":false,"walks_on_fire":false,"walks_on_poison":false});
    if let Some(wander) = wander {
        movement["wander"] = wander;
    }
    serde_json::from_value(
        json!({"movement":movement,"targeting":targeting,"attacks":attacks,"defenses":defenses}),
    )
    .expect("behaviour fixture")
}

fn project(
    behavior: &ProjectV2BehaviorAuthoring,
) -> Result<CreatureBehaviourProfile, ProfileRefusal> {
    CreatureBehaviourProfile::project(Some(behavior), &abilities())
}

/// The D116 rat: one melee entry, 2000 ms, {0, 8}; flees at 5 hp.
fn rat_profile() -> CreatureBehaviourProfile {
    project(&behavior(
        rat_targeting(),
        vec![entry("bite", 2000, 8)],
        vec![],
        None,
    ))
    .expect("rat")
}

fn with_targeting(edit: impl FnOnce(&mut Value)) -> CreatureBehaviourProfile {
    let mut targeting = rat_targeting();
    edit(&mut targeting);
    project(&behavior(
        targeting,
        vec![entry("bite", 2000, 8)],
        vec![],
        None,
    ))
    .expect("profile")
}

struct Facts {
    root: GameplayDecisionRoot,
    revisions: RevisionSet,
}

impl Facts {
    fn new() -> Self {
        Self {
            root: root(),
            revisions: revisions(),
        }
    }

    fn at<'a>(
        &'a self,
        position: MovementLocalPosition,
        health: u64,
        players: &'a [TargetCandidate],
        now_us: u64,
    ) -> CreatureThinkFacts<'a> {
        CreatureThinkFacts {
            position,
            health,
            has_active_condition: false,
            players,
            now_us,
            root: &self.root,
            revisions: &self.revisions,
        }
    }
}

fn think(
    state: &mut CreatureAiState,
    sequence: u64,
    facts: &CreatureThinkFacts<'_>,
) -> ThinkReport {
    state.think(sequence, facts, |_| true).expect("think")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn profile_missing_and_invalid_profiles_are_refused() {
    assert_eq!(
        CreatureBehaviourProfile::project(None, &abilities()),
        Err(ProfileRefusal::Missing)
    );
    let refused = |attacks: Vec<Value>, defenses: Vec<Value>| {
        project(&behavior(rat_targeting(), attacks, defenses, None))
    };
    assert_eq!(
        refused(vec![entry("bite", 2000, 8), entry("bite", 2000, 8)], vec![]),
        Err(ProfileRefusal::Invalid)
    );
    assert_eq!(
        refused(vec![], vec![entry("bite", 2000, 8)]),
        Err(ProfileRefusal::Invalid)
    );
    assert_eq!(
        refused(vec![entry("bite", 2000, 0)], vec![]),
        Err(ProfileRefusal::Invalid)
    );
    assert_eq!(
        refused(vec![entry("bolt", 2000, 8); 17], vec![]),
        Err(ProfileRefusal::Invalid)
    );
    assert_eq!(
        refused(vec![], vec![entry("heal", 2000, 8); 9]),
        Err(ProfileRefusal::Invalid)
    );
    let mut attacks = vec![entry("bolt", 2000, 8); 15];
    attacks.push(entry("bite", 2000, 8));
    let at_max = refused(attacks, vec![entry("heal", 2000, 8); 8]).expect("RL-07 at max");
    assert_eq!(at_max.entry_count(), CREATUREAI0_PROPOSALS_PER_THINK);
    assert_eq!(CREATUREAI0_PROPOSALS_PER_THINK, 24);
    assert_eq!(at_max.melee().map(|melee| melee.index), Some(15));
    let melee = rat_profile().melee().expect("melee");
    assert_eq!(
        (
            melee.interval_ms,
            melee.chance_ppm,
            melee.minimum,
            melee.maximum
        ),
        (2000, 1_000_000, 0, 8)
    );
}

#[test]
fn rat_targets_the_nearer_player_and_chases_until_adjacent() {
    let facts = Facts::new();
    let players = [player(10, at(105, 100)), player(11, at(103, 100))];
    let mut state = CreatureAiState::new(rat(), rat_profile(), at(100, 100));
    let report = think(&mut state, 1, &facts.at(at(100, 100), 20, &players, 0));
    assert_eq!(report.target, Some(actor_n(11)));
    assert_eq!(
        report.step,
        Some(CreatureStep {
            kind: StepKind::Chase,
            step: CardinalStep::East,
            destination: at(101, 100),
        })
    );
    assert!(report.proposals.is_empty() && !report.fleeing && !report.idle);
    assert_eq!(report.units, 1 + 2 + 2 + 1);
    let adjacent = think(
        &mut state,
        2,
        &facts.at(at(102, 100), 20, &players, 1_000_000),
    );
    assert_eq!(adjacent.target, Some(actor_n(11)));
    assert_eq!(adjacent.step, None);
    // A refused destination is no step.
    let mut blocked = CreatureAiState::new(rat(), rat_profile(), at(100, 100));
    let report = blocked
        .think(1, &facts.at(at(100, 100), 20, &players, 0), |_| false)
        .expect("think");
    assert_eq!((report.target, report.step), (Some(actor_n(11)), None));
}

#[test]
fn ineligible_and_other_floor_players_are_never_targeted_and_targets_are_dropped() {
    let facts = Facts::new();
    let mut protected = player(10, at(101, 100));
    protected.eligible = false;
    let other_floor = TargetCandidate {
        position: MovementLocalPosition {
            floor: 6,
            ..at(101, 100)
        },
        ..player(11, at(0, 0))
    };
    let mut state = CreatureAiState::new(rat(), rat_profile(), at(100, 100));
    for sequence in 1..=3 {
        let report = think(
            &mut state,
            sequence,
            &facts.at(
                at(100, 100),
                20,
                &[protected, other_floor],
                sequence * 1_000_000,
            ),
        );
        assert_eq!((report.target, report.step), (None, None));
    }
    let mut eligible = player(12, at(102, 100));
    let report = think(
        &mut state,
        4,
        &facts.at(at(100, 100), 20, &[eligible], 4_000_000),
    );
    assert_eq!(report.target, Some(actor_n(12)));
    eligible.eligible = false;
    let report = think(
        &mut state,
        5,
        &facts.at(at(100, 100), 20, &[eligible], 5_000_000),
    );
    assert_eq!((report.target, report.step), (None, None));
    assert_eq!(state.target(), None);
    // An unseen player is not perceived: the think is idle.
    let report = think(
        &mut state,
        6,
        &facts.at(at(100, 100), 20, &[player(13, at(200, 200))], 6_000_000),
    );
    assert!(report.idle);
    assert_eq!(report.units, 1 + 1);
}

#[test]
fn hurt_rat_flees_away_and_keeps_its_target() {
    let facts = Facts::new();
    let players = [player(10, at(102, 100))];
    let mut state = CreatureAiState::new(rat(), rat_profile(), at(100, 100));
    let report = think(&mut state, 1, &facts.at(at(100, 100), 5, &players, 0));
    assert!(report.fleeing);
    assert_eq!(report.target, Some(actor_n(10)));
    assert_eq!(
        report.step,
        Some(CreatureStep {
            kind: StepKind::Flee,
            step: CardinalStep::West,
            destination: at(99, 100),
        })
    );
    // Adjacent while fleeing: still the target (the swing bites), and still stepping away.
    let report = think(
        &mut state,
        2,
        &facts.at(at(101, 100), 5, &players, 1_000_000),
    );
    assert_eq!(report.target, Some(actor_n(10)));
    assert_eq!(report.step.map(|step| step.kind), Some(StepKind::Flee));
    let report = think(
        &mut state,
        3,
        &facts.at(at(101, 100), 6, &players, 2_000_000),
    );
    assert!(!report.fleeing);
}

#[test]
fn timed_target_change_fires_every_second_think_and_replays_identically() {
    let facts = Facts::new();
    let profile = with_targeting(|targeting| {
        targeting["change_target"] = json!({"interval_ms":2000,"chance_ppm":1_000_000});
    });
    let players = [player(10, at(101, 100)), player(11, at(99, 100))];
    let run = || {
        let mut state = CreatureAiState::new(rat(), profile.clone(), at(100, 100));
        (1..=8)
            .map(|sequence| {
                think(
                    &mut state,
                    sequence,
                    &facts.at(at(100, 100), 20, &players, sequence * 1_000_000),
                )
            })
            .collect::<Vec<_>>()
    };
    let reports = run();
    assert_eq!(
        reports
            .iter()
            .map(|report| report.target_changed)
            .collect::<Vec<_>>(),
        [false, true, false, true, false, true, false, true]
    );
    assert!(reports.iter().all(|report| report.target.is_some()));
    assert_eq!(reports, run());
    // chance 0 never changes.
    let mut state = CreatureAiState::new(rat(), rat_profile(), at(100, 100));
    for sequence in 1..=8 {
        let report = think(
            &mut state,
            sequence,
            &facts.at(at(100, 100), 20, &players, 0),
        );
        assert!(!report.target_changed);
    }
}

#[test]
fn strategy_weight_100_picks_that_strategy_and_ties_break_by_distance_then_identity() {
    let facts = Facts::new();
    let lowest_health = TargetCandidate {
        health: 10,
        ..player(10, at(103, 100))
    };
    let nearest = TargetCandidate {
        health: 50,
        ..player(11, at(101, 100))
    };
    let most_damage = TargetCandidate {
        health: 90,
        damage: 70,
        ..player(12, at(104, 100))
    };
    let players = [lowest_health, nearest, most_damage];
    for (strategy, expected) in [
        ("nearest", Some(actor_n(11))),
        ("health", Some(actor_n(10))),
        ("damage", Some(actor_n(12))),
        ("random", None),
    ] {
        let profile = with_targeting(|targeting| {
            targeting["strategy_weights"] = json!({"nearest":0,"damage":0,"health":0,"random":0});
            targeting["strategy_weights"][strategy] = json!(100);
        });
        let mut state = CreatureAiState::new(rat(), profile, at(100, 100));
        let report = think(&mut state, 1, &facts.at(at(100, 100), 20, &players, 0));
        let expected = expected.or_else(|| {
            let draw = deterministic_decision_u64(
                &facts.root,
                decision_occurrence(ThinkOccurrence {
                    actor: rat(),
                    sequence: 1,
                }),
                "AI_TARGET_SEARCH",
                1,
            )
            .expect("draw");
            targeting::select(
                Strategy::Random,
                at(100, 100),
                &targeting::perceive(at(100, 100), &players),
                draw,
            )
        });
        assert_eq!(report.target, expected, "{strategy}");
    }
    // Health tie: the nearer; health and distance tie: the canonical (identity) order.
    let near = TargetCandidate {
        health: 10,
        ..player(20, at(102, 100))
    };
    let far = TargetCandidate {
        health: 10,
        ..player(21, at(103, 100))
    };
    let eligible = targeting::perceive(at(100, 100), &[far, near]);
    assert_eq!(
        targeting::select(Strategy::LowestHealth, at(100, 100), &eligible, 0),
        Some(actor_n(20))
    );
    let twin = TargetCandidate {
        health: 10,
        ..player(22, at(98, 100))
    };
    let eligible = targeting::perceive(at(100, 100), &[twin, near]);
    let first = if actor_n(20).placement_identity() < actor_n(22).placement_identity() {
        actor_n(20)
    } else {
        actor_n(22)
    };
    assert_eq!(
        targeting::select(Strategy::LowestHealth, at(100, 100), &eligible, 0),
        Some(first)
    );
    assert_eq!(
        targeting::select(Strategy::Nearest, at(100, 100), &eligible, 0),
        Some(first)
    );
}

#[test]
fn spell_attacks_and_defence_emit_exact_proposals_and_skip_melee() {
    let facts = Facts::new();
    let behaviour = behavior(
        rat_targeting(),
        vec![
            entry("bolt", 1000, 8),
            entry("bolt", 1000, 9),
            entry("bolt", 1000, 10),
            entry("bite", 1000, 8),
        ],
        vec![entry("heal", 1000, 5)],
        None,
    );
    let profile = project(&behaviour).expect("profile");
    let mut state = CreatureAiState::new(rat(), profile, at(100, 100));
    let players = [player(10, at(102, 100))];
    let report = think(&mut state, 1, &facts.at(at(100, 100), 20, &players, 0));
    let identity = hex(&rat().placement_identity());
    let atom = |actor: ExactActorRef| format!("actor:{}", hex(&actor.placement_identity()));
    let expected = [
        (ScheduleList::Attack, 0, actor_n(10)),
        (ScheduleList::Attack, 1, actor_n(10)),
        (ScheduleList::Attack, 2, actor_n(10)),
        (ScheduleList::Defence, 0, rat()),
    ];
    assert_eq!(report.proposals.len(), expected.len());
    for (proposal, (list, index, target)) in report.proposals.iter().zip(expected) {
        let kind = if list == ScheduleList::Attack {
            "attack"
        } else {
            "defence"
        };
        assert_eq!(
            (
                proposal.list,
                proposal.entry_index,
                proposal.issuer,
                proposal.target
            ),
            (list, index, rat(), target)
        );
        assert_eq!(
            proposal.occurrence,
            AbilityOccurrence::new(
                &format!("ai-profile:{identity}:1:{kind}:{index}"),
                revisions()
            )
            .expect("occurrence")
        );
        assert_eq!(
            proposal.intent,
            AiAbilityAdapter::normalize(&atom(rat()), &[&atom(target)]).expect("intent")
        );
    }
    assert_eq!(report.refusal, None);
    // Out of the spells' range 3: only the defence.
    let far = [player(10, at(104, 100))];
    let report = think(&mut state, 2, &facts.at(at(100, 100), 20, &far, 1_000_000));
    assert_eq!(
        report
            .proposals
            .iter()
            .map(|proposal| proposal.list)
            .collect::<Vec<_>>(),
        [ScheduleList::Defence]
    );
}

#[test]
fn evaluation_units_at_max_think_and_above_max_mutate_nothing() {
    let facts = Facts::new();
    let players = (0..63)
        .map(|seed| player(100 + seed, at(103, 101)))
        .collect::<Vec<_>>();
    let mut state = CreatureAiState::new(rat(), rat_profile(), at(100, 100));
    let mut protected = player(99, at(103, 101));
    protected.eligible = false;
    let mut over = players.clone();
    over.push(protected);
    let refused = think(&mut state, 1, &facts.at(at(100, 100), 20, &over, 0));
    assert!(refused.over_budget);
    assert_eq!(refused.units, CREATUREAI0_EVALUATION_UNITS_PER_THINK + 1);
    assert_eq!((refused.target, refused.step), (None, None));
    assert_eq!(state.target(), None);
    // Nothing was recorded: the same sequence thinks afresh at the maximum.
    let report = think(&mut state, 1, &facts.at(at(100, 100), 20, &players, 0));
    assert!(!report.over_budget);
    assert_eq!(report.units, CREATUREAI0_EVALUATION_UNITS_PER_THINK);
    assert!(report.target.is_some() && report.step.is_some());
}

#[test]
fn thinks_per_window_at_max_start_now_and_the_rest_next_window() {
    let profile = rat_profile();
    let table_of = |count: u16| {
        let mut table = CreatureAiTable::default();
        for seed in 0..count {
            table.admit(
                CreatureAiState::new(actor_n(1000 + seed), profile.clone(), at(100, 100)),
                0,
            );
        }
        table
    };
    let mut table = table_of(1024);
    assert_eq!(table.take_due(0).len(), CREATUREAI0_THINKS_PER_WINDOW);
    let mut table = table_of(1025);
    assert_eq!(table.pending(), 1025);
    let first = table.take_due(0);
    assert_eq!(first.len(), 1024);
    assert!(
        table
            .take_due(CREATUREAI0_THINK_WINDOW_MICROS - 1)
            .is_empty()
    );
    // The started thinks run (idle: nothing perceived); the 1025th starts next window.
    let facts = Facts::new();
    for creature in &first {
        table
            .think(*creature, &facts.at(at(100, 100), 20, &[], 0), |_| true)
            .expect("present")
            .expect("think");
    }
    assert_eq!(table.pending(), 1);
    let next = table.take_due(CREATUREAI0_THINK_WINDOW_MICROS);
    assert_eq!(next.len(), 1);
    assert!(!first.contains(&next[0]));
    let last = (0..1025)
        .map(|seed| actor_n(1000 + seed))
        .max_by_key(|actor| actor.placement_identity())
        .expect("last");
    assert_eq!(next[0], last);
}

#[test]
fn overrides_hold_replace_and_expire() {
    let facts = Facts::new();
    let near = player(10, at(101, 100));
    let far = player(11, at(104, 100));
    let players = [near, far];
    let mut table = CreatureAiTable::default();
    table.admit(CreatureAiState::new(rat(), rat_profile(), at(100, 100)), 0);
    assert!(!table.set_override(
        actor_n(2),
        CreatureAiOverride::ForcedDistanceOne { until_us: 1 }
    ));
    for value in [
        CreatureAiOverride::ForcedTarget {
            actor: actor_n(10),
            until_us: 1,
        },
        CreatureAiOverride::ForcedDistanceOne {
            until_us: 5_000_000,
        },
        CreatureAiOverride::ForcedTarget {
            actor: actor_n(11),
            until_us: 5_000_000,
        },
    ] {
        assert!(table.set_override(rat(), value));
        assert!(
            table.state(rat()).expect("rat").override_count() <= CREATUREAI0_OVERRIDES_PER_CREATURE
        );
    }
    assert_eq!(table.state(rat()).expect("rat").override_count(), 2);
    // Hurt and with a nearer player: the forced target holds, no search and no flee.
    for now in [0, 4_000_000] {
        let report = table
            .think(rat(), &facts.at(at(100, 100), 5, &players, now), |_| true)
            .expect("present")
            .expect("think");
        assert_eq!(report.target, Some(actor_n(11)));
        assert!(!report.fleeing);
        assert_eq!(report.step.map(|step| step.kind), Some(StepKind::Chase));
    }
    let report = table
        .think(
            rat(),
            &facts.at(at(100, 100), 5, &players, 5_000_000),
            |_| true,
        )
        .expect("present")
        .expect("think");
    assert_eq!(report.target, Some(actor_n(10)));
    assert!(report.fleeing);
    assert_eq!(table.state(rat()).expect("rat").override_count(), 0);
}

#[test]
fn retried_think_is_identical_and_an_older_sequence_is_superseded() {
    let facts = Facts::new();
    let players = [player(10, at(103, 100))];
    let mut state = CreatureAiState::new(rat(), rat_profile(), at(100, 100));
    let report = think(&mut state, 2, &facts.at(at(100, 100), 20, &players, 0));
    assert_eq!(
        report,
        think(&mut state, 2, &facts.at(at(100, 100), 20, &players, 0))
    );
    assert_eq!(
        state.think(1, &facts.at(at(100, 100), 20, &players, 0), |_| true),
        Err(ThinkError::OccurrenceSuperseded)
    );
}

#[test]
fn idle_creatures_wake_only_in_view_and_at_most_the_wake_cap() {
    let facts = Facts::new();
    let profile = rat_profile();
    let mut table = CreatureAiTable::default();
    table.admit(
        CreatureAiState::new(rat(), profile.clone(), at(100, 100)),
        0,
    );
    // A creature admitted inside a stationary player's view targets them on its first think.
    let stationary = [player(10, at(103, 102))];
    assert_eq!(table.take_due(0), [rat()]);
    let report = table
        .think(rat(), &facts.at(at(100, 100), 20, &stationary, 0), |_| true)
        .expect("present")
        .expect("think");
    assert_eq!(report.target, Some(actor_n(10)));
    assert_eq!(table.due(rat()), Some(1_000_000));
    // Out of view: idle, no next think.
    let report = table
        .think(rat(), &facts.at(at(100, 100), 20, &[], 1_000_000), |_| true)
        .expect("present")
        .expect("think");
    assert!(report.idle);
    assert_eq!((table.due(rat()), table.pending()), (None, 0));
    let position = |_| Some(at(100, 100));
    assert!(table.wake(at(109, 100), position, 2_000_000).is_empty());
    assert!(table.wake(at(100, 107), position, 2_000_000).is_empty());
    assert_eq!(table.wake(at(108, 100), position, 2_000_000), [rat()]);
    assert_eq!(table.due(rat()), Some(2_000_000));
    // A creature with a pending think is not woken again.
    assert!(table.wake(at(101, 100), position, 3_000_000).is_empty());
    assert_eq!(table.due(rat()), Some(2_000_000));

    let mut table = CreatureAiTable::default();
    let creatures = (0..1025)
        .map(|seed| actor_n(1000 + seed))
        .collect::<Vec<_>>();
    for creature in &creatures {
        table.admit(
            CreatureAiState::new(*creature, profile.clone(), at(100, 100)),
            0,
        );
        table
            .think(*creature, &facts.at(at(100, 100), 20, &[], 0), |_| true)
            .expect("present")
            .expect("think");
    }
    assert_eq!(table.pending(), 0);
    let woken = table.wake(at(101, 100), |_| Some(at(100, 100)), 1);
    assert_eq!(woken.len(), CREATUREAI1_WAKE_CANDIDATES);
    assert_eq!(table.pending(), CREATUREAI1_WAKE_CANDIDATES);
    assert!(table.remove(creatures[0]));
    assert!(!table.remove(creatures[0]));
}

#[test]
fn wander_steps_near_the_anchor_only_while_a_player_is_perceived() {
    let facts = Facts::new();
    let profile = project(&behavior(
        rat_targeting(),
        vec![entry("bite", 2000, 8)],
        vec![],
        Some(json!({"interval_ms":2000,"radius_tiles":1})),
    ))
    .expect("profile");
    let mut protected = player(10, at(104, 100));
    protected.eligible = false;
    let mut state = CreatureAiState::new(rat(), profile.clone(), at(100, 100));
    let first = think(&mut state, 1, &facts.at(at(100, 100), 20, &[protected], 0));
    assert_eq!(first.step, None);
    let second = think(
        &mut state,
        2,
        &facts.at(at(100, 100), 20, &[protected], 1_000_000),
    );
    let step = second.step.expect("wander step");
    assert_eq!(step.kind, StepKind::Wander);
    assert_eq!(chebyshev_distance(step.destination, at(100, 100)), Some(1));
    // Nothing perceived: idle, no wander.
    let mut state = CreatureAiState::new(rat(), profile, at(100, 100));
    for sequence in 1..=4 {
        let report = think(&mut state, sequence, &facts.at(at(100, 100), 20, &[], 0));
        assert!(report.idle && report.step.is_none());
    }
}
