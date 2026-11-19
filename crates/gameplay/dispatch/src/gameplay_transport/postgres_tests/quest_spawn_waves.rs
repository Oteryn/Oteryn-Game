//! Scheduler and literal USE coverage with bindings authored by the fixture.
use super::fixture::{Harness, debug, id, run};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition, NodeId,
    owner_timer::SemanticTimeMicros,
};
use crate::gameplay_transport::{FenceHold, FenceStep, TransitionFence};
use crate::map::view::MapFacts;
use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
use oteryn_gameplay_quests::quest::world_callbacks::{
    Callback, Encounter, EncounterMonster, EncounterPlayer, ItemOnTarget, Offering, Position,
    Storage, TargetTransform, Trigger,
    periodic_bosses::{PeriodicBoss, PeriodicBossPlacement, WeightedBoss},
    reactive_waves::{Chance, EarthHit, FinishArea, Retaliation, SpecialWave},
    waves::{Wave, WaveCredit, WaveCreditEffect, WaveRun, WaveSpawn, WaveVictory},
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::time::Duration;

const TRIGGER: Position = Position {
    x: 33294,
    y: 31677,
    floor: -15,
};
const fn row(offset: u16) -> Position {
    Position {
        x: TRIGGER.x + offset,
        ..TRIGGER
    }
}
const START: Position = Position {
    x: TRIGGER.x - 1,
    ..TRIGGER
};
const PEER_START: Position = Position {
    y: START.y - 1,
    ..START
};
const LANDING: Position = row(2);
const PEER_LANDING: Position = Position {
    y: LANDING.y + 1,
    ..LANDING
};
const FIRST: Position = row(3);
const SECOND: Position = row(4);
const EXIT: Position = row(6);
const VICTORY_EXIT: Position = row(7);
const HATCH: Position = row(8);
const RAT: &str = "oteryn:creature.rat";
const CAVE_RAT: &str = "oteryn:creature.cave_rat";
const CREDIT: &str = "oteryn:quest-transition/wave-egress/fixture";
const TRACK: &str = "oteryn:quest-progress/fixture/wave-survived";
const CUT: &str = "oteryn:quest-progress/fixture/weak-point";
const OTHER_CUT: &str = "oteryn:quest-progress/fixture/other-weak-point";
const CHAMPION: &str = "oteryn:achievement/champion_of_chazorai";
const OAK_TOOL: &str = "oteryn:item.tibia.i919";

#[test]
fn periodic_boss_waits_for_interval_and_cooldown_and_keeps_one_live_actor() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("quest-periodic-boss", move || {
        scenario(admin, Case::Periodic)
    })
}

#[test]
fn periodic_boss_forced_placement_keeps_an_occupied_center() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("quest-periodic-forced-center", move || {
        scenario(admin, Case::PeriodicOccupiedCenter)
    })
}

#[test]
fn periodic_boss_with_a_missing_center_uses_the_extended_neighborhood() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("quest-periodic-extended-neighbor", move || {
        scenario(admin, Case::PeriodicMissingCenter)
    })
}

#[test]
fn quest_use_admits_two_staged_players_then_ejects_and_credits_both_survivors() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("quest-timed-waves", move || {
        scenario(admin, Case::Waves)
    })
}

#[test]
fn quest_use_refuses_wave_credit_bound_to_a_world_occurrence_before_relocation() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("quest-invalid-wave-credit", move || {
        scenario(admin, Case::WorldCredit)
    })
}

#[test]
fn quest_use_refuses_a_wave_room_with_an_unreachable_timeout_exit() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("quest-invalid-wave-exit", move || {
        scenario(admin, Case::UnreachableExit)
    })
}

#[test]
fn quest_use_refuses_an_unknown_victory_achievement_before_relocation() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("quest-unknown-victory-achievement", move || {
        scenario(admin, Case::UnknownAchievement)
    })
}

#[test]
fn quest_step_admits_guarded_survival_trial_and_credits_at_deadline() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("quest-survival-step", move || {
        scenario(admin, Case::SurvivalStep)
    })
}

#[test]
fn quest_station_step_starts_waves_without_relocating_staged_players() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("quest-station-step", move || {
        scenario(admin, Case::Stations)
    })
}

#[test]
fn quest_wave_victory_requires_the_terminal_spawn_and_uses_reward_egress() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("quest-victory-egress", move || {
        scenario(admin, Case::Victory)
    })
}

#[test]
fn quest_wave_victory_awards_both_survivors_even_without_quest_progress() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("quest-victory-achievement", move || {
        scenario(admin, Case::VictoryWithoutProgress)
    })
}

#[test]
fn quest_wave_uses_an_in_time_terminal_death_observed_after_timeout() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("quest-wave-in-time-death", move || {
        scenario(admin, Case::VictoryBeforeTimeout)
    })
}

#[test]
fn quest_wave_uses_a_pre_cutoff_reflection_while_corpse_projection_is_pending() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("quest-wave-pending-reflection", move || {
        scenario(admin, Case::VictoryWithPendingReflection)
    })
}

#[test]
fn quest_wave_terminal_death_at_timeout_ejects_both_without_victory_credit() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("quest-wave-death-at-timeout", move || {
        scenario(admin, Case::DeathAtTimeout)
    })
}

#[test]
fn quest_step_earth_hit_updates_current_vitals_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("quest-earth-step", move || {
        scenario(admin, Case::EarthStep)
    })
}

#[test]
fn quest_axe_use_spawns_last_retaliation_and_requires_clearance_for_final_cut() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("quest-retaliation", move || {
        scenario(admin, Case::Retaliation)
    })
}

#[test]
fn quest_axe_use_advances_an_unset_weak_point_to_two() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("quest-retaliation-first-blow", move || {
        scenario(admin, Case::RetaliationFirstBlow)
    })
}

#[test]
fn quest_axe_use_recovers_lost_ack_without_repeating_retaliation() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("quest-retaliation-lost-ack", move || {
        scenario(admin, Case::RetaliationLostAck)
    })
}

#[derive(Clone, Copy, Debug)]
enum Case {
    Periodic,
    PeriodicMissingCenter,
    PeriodicOccupiedCenter,
    Waves,
    DeathAtTimeout,
    Victory,
    VictoryBeforeTimeout,
    VictoryWithPendingReflection,
    VictoryWithoutProgress,
    SurvivalStep,
    Stations,
    EarthStep,
    Retaliation,
    RetaliationFirstBlow,
    RetaliationLostAck,
    UnreachableExit,
    UnknownAchievement,
    WorldCredit,
}

impl Case {
    fn has_retaliation_binding(self) -> bool {
        matches!(self, Self::Retaliation | Self::RetaliationFirstBlow | Self::RetaliationLostAck)
    }

    fn has_victory_binding(self) -> bool {
        matches!(
            self,
            Self::Victory
                | Self::VictoryBeforeTimeout
                | Self::VictoryWithPendingReflection
                | Self::VictoryWithoutProgress
                | Self::DeathAtTimeout
                | Self::UnknownAchievement
        )
    }
}

fn scenario(admin: String, case: Case) -> TestResult {
    run(async move |_| {
        let started = std::time::Instant::now();
        let mut callback = super::authored_world_rewards::placeholder(TRIGGER)?;
        callback.key = "oteryn:quest-callback/fixture/quest-spawn".into();
        let initial_storage = std::mem::take(&mut callback.storage);
        callback.requires_world_admission = false;
        let victory = case.has_victory_binding();
        let credit_stage = i64::from(matches!(case, Case::VictoryWithoutProgress));
        let credit = || WaveCredit::Bound {
            key: CREDIT.into(),
            effects: vec![WaveCreditEffect {
                track: TRACK.into(),
                minimum: credit_stage,
                maximum: credit_stage,
                set: 1,
            }],
        };
        let first_creature = if victory { CAVE_RAT } else { RAT };
        let terminal_position = if victory { LANDING } else { SECOND };
        callback.effects.encounter = Some(Encounter {
            minimum_level: 0,
            room_min: LANDING,
            room_max: Position {
                x: SECOND.x,
                ..PEER_LANDING
            },
            players: vec![
                EncounterPlayer {
                    from: START,
                    to: LANDING,
                },
                EncounterPlayer {
                    from: PEER_START,
                    to: PEER_LANDING,
                },
            ],
            monsters: Vec::new(),
            timing: None,
            phase: None,
            // Crystal the_new_frontier/action_arena.lua:39-82. The PG fixture
            // uses shorter durations and two waves to exercise both deadlines.
            wave_run: Some(WaveRun {
                timeout_ms: 10_000,
                timeout_exit: if matches!(case, Case::UnreachableExit) {
                    row(40)
                } else {
                    EXIT
                },
                timeout_credit: match case {
                    Case::WorldCredit => Some(WaveCredit::Reference(CREDIT.into())),
                    _ if victory => None,
                    _ => Some(credit()),
                },
                // Crystal the_new_frontier/creaturescripts_tirecz_kill.lua:19-34.
                victory: victory.then(|| WaveVictory {
                    creature: CAVE_RAT.into(),
                    exit: VICTORY_EXIT,
                    credit: Some(credit()),
                    achievement: Some(if matches!(case, Case::UnknownAchievement) {
                        "oteryn:achievement/unregistered_wave_victory"
                    } else {
                        CHAMPION
                    }.into()),
                }),
                eject_room: false,
                stay_at_stations: false,
                hatch: None,
                allow_unstaged_activator: false,
                member_guard: Some(
                    oteryn_gameplay_quests::quest::reward_callbacks::TrackRequirement {
                        track: TRACK.into(),
                        minimum: 0,
                        maximum: 0,
                    },
                ),
                waves: [(1_000, FIRST, first_creature), (3_000, terminal_position, CAVE_RAT)]
                    .into_iter()
                    .map(|(at_ms, position, creature)| Wave {
                        at_ms,
                        spawn: WaveSpawn::Fixed {
                            monsters: vec![EncounterMonster {
                                position,
                                creature: creature.into(),
                            }],
                        },
                    })
                    .collect(),
            }),
        });
        if matches!(case, Case::SurvivalStep | Case::Stations) {
            callback.trigger = Trigger::StepIn;
            callback.return_on_failure = matches!(case, Case::SurvivalStep);
            callback.storage = initial_storage
                .into_iter()
                .map(|storage| Storage {
                    track: TRACK.into(),
                    read_only: true,
                    set: 0,
                    ..storage
                })
                .collect();
            let room = callback.effects.encounter.as_mut().ok_or("fixture room")?;
            let wave = room.wave_run.as_mut().ok_or("fixture wave run")?;
            wave.member_guard = None;
            if matches!(case, Case::Stations) {
                // Crystal movements_click.lua:84-117 activates at the stations;
                // players later cross into the room by their own movement.
                room.players = [TRIGGER, PEER_START].map(|position| EncounterPlayer {
                    from: position,
                    to: position,
                }).into();
                wave.stay_at_stations = true;
                wave.eject_room = true;
                wave.hatch = Some(TargetTransform {
                    position: HATCH,
                    from: 1897,
                    to: 1977,
                    reset_after_ms: 3_000,
                });
                wave.waves[0].spawn = WaveSpawn::Scatter {
                    creature: RAT.into(),
                    count: 1,
                    attempts_per_creature: 10,
                    min: FIRST,
                    max: FIRST,
                };
            } else {
                // Crystal movement_minotaur_boss.lua:7-15,22-40 decrees a
                // survival deadline without summoning any timed waves.
                room.players = vec![EncounterPlayer { from: TRIGGER, to: LANDING }];
                wave.waves.clear();
            }
        }
        if matches!(case, Case::EarthStep) {
            // Crystal movements_area_damage.lua:3-16. The fixture forces a
            // fixed roll so it can verify the real HP owner deterministically.
            callback.trigger = Trigger::StepIn;
            callback.selector.position = None;
            callback.selector.unique = None;
            callback.effects.encounter = None;
            callback.effects.earth_hit = Some(EarthHit {
                chance: Chance { wins: 1, rolls: 1 },
                minimum: 35,
                maximum: 35,
                effect: 51,
            });
        }
        if case.has_retaliation_binding() {
            retaliation_binding(&mut callback, !matches!(case, Case::Retaliation));
        }
        // Use the real typed callback decode in addition to the composed PG fixture.
        let callback: Callback = serde_json::from_value(serde_json::to_value(callback)?)?;
        let world = super::quest_world_callbacks::world_with_cells(
            &callback,
            TRIGGER,
            &[FIRST, SECOND, EXIT, PEER_START, PEER_LANDING],
        )?;
        let h =
            Harness::create_with_admission_seed(admin, "quest_spawn_waves", false, false).await?;
        let admission = prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("content generation")?;
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            world.map_revision_digest(),
            (
                i32::from(START.x),
                i32::from(START.y),
                -i16::from(START.floor),
            ),
        );
        super::boss_activity::issue_content_and_reset(&h, &pin).await?;
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            16,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        deps.admitted.runtime_actor = Some(deps.actor);
        runtime
            .initialize_pinned_test_position(deps.actor, local(START))
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        if !matches!(case, Case::Periodic | Case::PeriodicMissingCenter | Case::PeriodicOccupiedCenter) {
            assert!(
                owner.states.quest_callbacks.periodic_boss_fixture.set(Vec::new()).is_ok()
            );
        }
        if victory || matches!(case, Case::Waves | Case::SurvivalStep | Case::Stations | Case::UnreachableExit | Case::WorldCredit) {
            owner.quest_catalogue = Some(std::sync::Arc::new(credit_catalogue(&callback, matches!(case, Case::WorldCredit))?));
        } else if case.has_retaliation_binding() {
            owner.quest_catalogue = Some(std::sync::Arc::new(retaliation_catalogue(&callback)?));
        } else if matches!(case, Case::EarthStep) {
            assert!(
                owner
                    .states
                    .quest_callbacks
                    .fixture
                    .set(vec![callback])
                    .is_ok()
            );
        } else {
            // Crystal globalevents_pits_of_inferno_bosses.lua:12-14,23. This
            // fixture uses a nonzero cooldown to prove the shared owner gate.
            let (placement, choices): (_, &[_]) = match case {
                Case::PeriodicMissingCenter => (
                    PeriodicBossPlacement::ExtendedForced,
                    &[(row(10), RAT, 1)],
                ),
                Case::PeriodicOccupiedCenter => (
                    PeriodicBossPlacement::ExtendedForced,
                    &[(START, RAT, 1)],
                ),
                _ => (
                    PeriodicBossPlacement::Exact,
                    &[(FIRST, RAT, 1), (SECOND, CAVE_RAT, 3)],
                ),
            };
            let plan = PeriodicBoss {
                key: "oteryn:periodic-boss/fixture".into(),
                interval_ms: 1_000,
                cooldown_ms: 4_000,
                placement,
                choices: choices
                    .iter()
                    .map(|&(position, creature, weight)| WeightedBoss {
                        weight,
                        spawn: EncounterMonster {
                            creature: creature.into(),
                            position,
                        },
                    })
                    .collect(),
            };
            assert!(
                owner
                    .states
                    .quest_callbacks
                    .periodic_boss_fixture
                    .set(vec![plan])
                    .is_ok()
            );
        }
        let mut admitted = deps.admitted;
        super::srx3_activity_source_fixture::prepare_player_with_operation(
            &h,
            &owner,
            &mut admitted,
            100,
            "knight",
            admission.operation(),
        )
        .await?;
        match case {
            Case::Waves
            | Case::Victory
            | Case::VictoryBeforeTimeout
            | Case::VictoryWithPendingReflection
            | Case::VictoryWithoutProgress
            | Case::DeathAtTimeout => {
                wave_scenario(&h, &owner, &mut admitted, deps.actor, case).await?
            }
            Case::SurvivalStep => survival_step_scenario(&owner, &mut admitted, deps.actor).await?,
            Case::Stations => stations_scenario(&h, &owner, &mut admitted, deps.actor).await?,
            Case::Periodic => periodic_scenario(&owner).await?,
            Case::PeriodicMissingCenter | Case::PeriodicOccupiedCenter => {
                // Crystal map.cpp:243-307. A player occupies START. The other
                // layout ends at row(8), two tiles from the missing row(10).
                let expected = if matches!(case, Case::PeriodicMissingCenter) {
                    row(8)
                } else {
                    START
                };
                tokio::time::pause();
                owner.drain_quest_encounter_deadlines().await;
                advance(&owner, 1_000).await;
                let actors = creatures(&owner).await?;
                assert_eq!(actors.len(), 1);
                assert_eq!(actors[0].1, local(expected));
                assert_eq!(position(&owner, deps.actor).await?, local(START));
                tokio::time::resume();
            }
            Case::EarthStep => earth_step_scenario(&owner, &mut admitted, deps.actor).await?,
            Case::Retaliation => retaliation_scenario(&h, &owner, &mut admitted, deps.actor).await?,
            Case::RetaliationFirstBlow | Case::RetaliationLostAck => {
                retaliation_first_blow(&h, &owner, &mut admitted, deps.actor, matches!(case, Case::RetaliationLostAck)).await?
            }
            Case::UnreachableExit | Case::UnknownAchievement | Case::WorldCredit => {
                let peer = stage_peer(&h, &owner, &admitted).await?;
                let peer_actor = peer.runtime_actor.ok_or("staged peer")?;
                let expected = if matches!(case, Case::UnreachableExit) {
                    UseDisposition::NotPossible
                } else {
                    UseDisposition::Rejected
                };
                assert_eq!(use_room(&owner, &mut admitted, 1).await?, expected);
                assert_eq!(position(&owner, deps.actor).await?, local(START));
                assert_eq!(position(&owner, peer_actor).await?, local(PEER_START));
            }
        }
        eprintln!(
            "POSTGRES_CASE_COMPLETED quest_spawn case={case:?} duration={:?}",
            started.elapsed()
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

fn credit_catalogue(
    callback: &Callback,
    world_credit: bool,
) -> TestResult<oteryn_gameplay_quests::quest::QuestStateCatalogue> {
    use oteryn_gameplay_quests::quest::{
        QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
        QuestTransition, authored_bindings::Bindings,
    };
    // A World-occurrence transition is valid in the quest catalogue, but this
    // scheduler's survivor receipt cannot settle that type of transition.
    let (tracks, transitions) = if world_credit {
        (vec![QuestTrack {
            key: TRACK.into(), quest: callback.quest.clone(), initial: 0, min: 0, max: 1,
        }], vec![QuestTransition {
            key: CREDIT.into(), quest: callback.quest.clone(), completes: false,
            experience: None, world_change: Some("hive_born".into()),
            effects: vec![QuestEffect {
                track: TRACK.into(), from: QuestComparison::Eq(0), effect: QuestEffectKind::Set(1),
            }],
        }])
    } else {
        (Vec::new(), Vec::new())
    };
    let base = QuestStateCatalogue::new("content-1", tracks, transitions).map_err(debug)?;
    let kind = match callback.trigger {
        Trigger::StepIn => "step_in",
        _ => "map_use",
    };
    let document = serde_json::json!({"schema":1,"quests":[{
        "quest": callback.quest,
        "recipe_sha256":"2222222222222222222222222222222222222222222222222222222222222222",
        "sources":["Crystal 201189f8 the_new_frontier/action_arena.lua:39-82 and children_of_the_revolution/movements_click.lua:29-43"],
        "tracks":[{"key":TRACK,"initial":0,"minimum":0,"maximum":1}],
        "stages":[{"key":"waves","completes":false,"trigger":{"kind":kind,"callback":callback}}]
    }]});
    let bindings = Bindings::parse(&serde_json::to_vec(&document)?)?;
    Ok(bindings.attach(&base)?)
}

fn retaliation_binding(callback: &mut Callback, first_blow: bool) {
    // Crystal actions_demon_oak.lua:67-70,80-126 reads each character's weak
    // points. Retain a completed point while staging the last special blow.
    callback.effects.encounter = None;
    callback.requires_world_admission = true;
    callback.item_on_target = Some(ItemOnTarget {
        item: OAK_TOOL.into(),
        alternatives: vec![],
        count: 1,
        replacement: None,
        target_replacement: None,
        target_remove: false,
        reusable: true,
        on_step: false,
        selected_instance: false,
        ingredients: vec![],
    });
    callback.storage.push(Storage {
        track: CUT.into(),
        minimum: if first_blow { -1 } else { 10 },
        maximum: 10,
        set: 11,
        add: Some(1),
        set_if: None,
        preserve: false,
        increment: false,
        at_least: false,
        read_only: false,
    });
    callback.storage.push(Storage {
        track: OTHER_CUT.into(),
        minimum: 0,
        maximum: 11,
        set: 11,
        add: None,
        set_if: None,
        preserve: true,
        increment: false,
        at_least: false,
        read_only: false,
    });
    callback.effects.retaliation = Some(Retaliation {
        progress: CUT.into(),
        weak_points: vec![CUT.into(), OTHER_CUT.into()],
        completed_at: 11,
        progress_floor: first_blow.then_some(1),
        positions: vec![FIRST],
        ordinary_chance: if first_blow { Chance { wins: 1, rolls: 1 } } else { Chance { wins: 11, rolls: 100 } },
        ordinary: vec![RAT.into()],
        special: vec![SpecialWave {
            before: 10,
            creatures: vec![RAT.into()],
        }],
        last: CAVE_RAT.into(),
    });
    callback.effects.earth_hit = Some(EarthHit {
        chance: Chance { wins: 1, rolls: 1 },
        minimum: 35,
        maximum: 35,
        effect: 51,
    });
}

fn retaliation_catalogue(
    hit: &Callback,
) -> TestResult<oteryn_gameplay_quests::quest::QuestStateCatalogue> {
    use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
    let initial = hit.storage.first().ok_or("fixture cut track")?.minimum;
    let mut finish = hit.clone();
    finish.key = "oteryn:quest-callback/fixture/finish-weak-point".into();
    finish.teleport = Some(EXIT);
    finish.effects.retaliation = None;
    finish.effects.earth_hit = None;
    finish.effects.finish_area = Some(FinishArea {
        minimum: FIRST,
        maximum: SECOND,
    });
    let cut = finish.storage.first_mut().ok_or("fixture cut track")?;
    cut.minimum = 11;
    cut.maximum = 11;
    cut.add = None;
    cut.preserve = true;
    let done = Storage {
        track: TRACK.into(),
        minimum: 0,
        maximum: 0,
        set: 1,
        preserve: false,
        ..cut.clone()
    };
    finish.storage[1].minimum = 11;
    finish.storage.push(done);
    let bindings = Bindings::parse(&serde_json::to_vec(&serde_json::json!({"schema":1,"quests":[{
        "quest":hit.quest,
        "recipe_sha256":"2222222222222222222222222222222222222222222222222222222222222222",
        "sources":["Crystal 201189f8 demon_oak/actions_demon_oak.lua:62-126"],
        "tracks":[
            {"key":CUT,"initial":initial,"minimum":initial,"maximum":11},
            {"key":OTHER_CUT,"initial":11,"minimum":0,"maximum":11},
            {"key":TRACK,"initial":0,"minimum":0,"maximum":1}
        ],
        "stages":[
            {"key":"hit","completes":false,"trigger":{"kind":"map_use","callback":hit}},
            {"key":"finish","completes":false,"trigger":{"kind":"map_use","callback":finish}}
        ]
    }]}))?)?;
    let base = QuestStateCatalogue::new("content-1", Vec::new(), Vec::new()).map_err(debug)?;
    Ok(bindings.attach(&base)?)
}

async fn retaliation_first_blow(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    player: ExactActorRef,
    lose_ack: bool,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::decode_use_result;

    let fence = admitted.item_fence.ok_or("axe item fence")?;
    super::item_look::seed_main_backpack_for(h, fence.character_id.as_bytes(), 70, &[]).await?;
    super::quest_world_callbacks::input_item(
        owner,
        fence,
        &Offering { position: TRIGGER, item: OAK_TOOL.into(), count: 1 },
        60,
        true,
    ).await?;
    let session = admitted.game_session_id;
    let health = async || -> TestResult<_> {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        Ok(states.get(&runtime, player, session).ok_or("axe player vitals")?.vitals().health)
    };
    let read_state = async || {
        owner.root.read_character_quest_state(owner.character, fence.character_id).await.map_err(debug)
    };
    let before = health().await?;
    let payload = use_room_payload(owner)?;
    if lose_ack {
        owner.states.quest_callbacks.interrupt_after_commit.store(true, std::sync::atomic::Ordering::Release);
        assert!(super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload.clone()).await.is_err());
        assert!(owner.has_pending_quest_callback(session));
        assert_eq!(read_state().await?.tracks().get(CUT), Some(&2));
        assert_eq!(health().await?, before);
        assert!(creatures(owner).await?.is_empty());
    }

    let reply = super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload.clone()).await?;
    assert_eq!(decode_use_result(&reply).map_err(debug)?, UseDisposition::Committed);
    assert!(!owner.has_pending_quest_callback(session));
    assert_eq!(position(owner, player).await?, local(START));
    // Crystal actions_demon_oak.lua:94-95,115-124. The unset track counts
    // as one when striking; the durable result is two and the wave is ordinary.
    let state = read_state().await?;
    assert_eq!(state.tracks().get(CUT), Some(&2));
    let spawned = creatures(owner).await?;
    let [(actor, at)] = spawned.as_slice() else {
        return Err("first blow should spawn the fixture's ordinary monster".into());
    };
    assert_eq!(*at, local(FIRST));
    {
        let runtime = owner.runtime.lock().await;
        assert_eq!(runtime.companion_snapshot(*actor).map_err(debug)?.state.policy.definition_key, RAT);
    }
    assert_eq!(before - health().await?, 35);
    // Crystal actions_demon_oak.lua:115-126 applies one wave roll and one
    // self-hit per accepted blow. A repeated transport command is the same blow.
    assert_eq!(super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload).await?, reply);
    assert_eq!(read_state().await?, state);
    assert_eq!(creatures(owner).await?, spawned);
    assert_eq!(before - health().await?, 35);
    Ok(())
}

async fn retaliation_scenario(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    player: ExactActorRef,
) -> TestResult {
    let fence = admitted.item_fence.ok_or("axe item fence")?;
    super::item_look::seed_main_backpack_for(h, fence.character_id.as_bytes(), 70, &[]).await?;
    super::quest_world_callbacks::input_item(
        owner,
        fence,
        &Offering { position: TRIGGER, item: OAK_TOOL.into(), count: 1 },
        60,
        true,
    ).await?;

    let session = admitted.game_session_id;
    let health = async || -> TestResult<_> {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        Ok(states.get(&runtime, player, session).ok_or("axe player vitals")?.vitals().health)
    };
    let before = health().await?;
    // Forced retaliation can spawn at a selected position occupied by a player.
    let blocker = position_session_before_lease(owner, FIRST).await?;
    assert_eq!(use_room(owner, admitted, 1).await?, UseDisposition::Committed);
    let actors = creatures(owner).await?;
    let [(last, at)] = actors.as_slice() else {
        return Err("the last blow must spawn exactly one retaliation".into());
    };
    assert_eq!(*at, local(FIRST));
    assert_eq!(position(owner, blocker).await?, local(FIRST));
    {
        let runtime = owner.runtime.lock().await;
        assert_eq!(
            runtime.companion_snapshot(*last).map_err(debug)?.state.policy.definition_key,
            CAVE_RAT,
        );
    }
    assert_eq!(before - health().await?, 35);
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(use_room(owner, admitted, 2).await?, UseDisposition::Rejected);
    assert_eq!(position(owner, player).await?, local(START));
    assert_eq!(before - health().await?, 35);
    owner.runtime.lock().await.remove_encounter_actor(*last).map_err(debug)?;
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(use_room(owner, admitted, 3).await?, UseDisposition::Committed);
    assert_eq!(position(owner, player).await?, local(EXIT));
    let state = owner.root.read_character_quest_state(owner.character, fence.character_id)
        .await.map_err(debug)?;
    assert_eq!(state.tracks().get(CUT), Some(&11));
    assert_eq!(state.tracks().get(TRACK), Some(&1));
    assert_eq!(before - health().await?, 35);

    // Crystal actions_demon_oak.lua:67-70,80-96 uses the axe wielder's stored
    // blows. Another character still owes the last blow after the first exits.
    let mut peer = stage_peer(h, owner, admitted).await?;
    let peer_actor = peer.runtime_actor.ok_or("peer actor")?;
    let peer_fence = peer.item_fence.ok_or("peer axe item fence")?;
    assert_ne!(peer_fence.character_id, fence.character_id);
    super::item_look::seed_main_backpack_for(h, peer_fence.character_id.as_bytes(), 73, &[]).await?;
    super::quest_world_callbacks::input_item(
        owner,
        peer_fence,
        &Offering { position: TRIGGER, item: OAK_TOOL.into(), count: 1 },
        61,
        true,
    ).await?;
    assert_eq!(use_room(owner, &mut peer, 1).await?, UseDisposition::Committed);
    assert_eq!(position(owner, peer_actor).await?, local(PEER_START));
    assert_eq!(position(owner, player).await?, local(EXIT));
    let peer_state = owner.root
        .read_character_quest_state(owner.character, peer_fence.character_id)
        .await.map_err(debug)?;
    assert_eq!(peer_state.tracks().get(CUT), Some(&11));
    assert_eq!(peer_state.tracks().get(TRACK).copied().unwrap_or(0), 0);
    let spawned = creatures(owner).await?;
    assert_eq!(spawned.len(), 1);
    assert_ne!(spawned[0].0, *last);
    Ok(())
}

async fn periodic_scenario(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    tokio::time::pause();
    owner.drain_quest_encounter_deadlines().await;
    assert_eq!(next_spawn_delay(owner).await, Some(Duration::from_secs(1)));
    advance(owner, 999).await;
    assert_eq!(next_spawn_delay(owner).await, Some(Duration::from_millis(1)));
    assert!(creatures(owner).await?.is_empty());
    advance(owner, 1).await;
    let actors = creatures(owner).await?;
    let [(actor, at)] = actors.as_slice() else {
        return Err("the interval should produce exactly one boss".into());
    };
    assert!([local(FIRST), local(SECOND)].contains(at));
    advance(owner, 1_000).await;
    assert_eq!(
        creatures(owner).await?,
        actors,
        "no replacement while the boss lives"
    );
    owner
        .runtime
        .lock()
        .await
        .remove_encounter_actor(*actor)
        .map_err(debug)?;
    advance(owner, 1_000).await;
    assert!(
        creatures(owner).await?.is_empty(),
        "the next interval is inside cooldown"
    );
    // Crystal globalevents_pits_of_inferno_bosses.lua:12-23 is autonomous.
    // Let the same loop as serve_gameplay observe cooldown expiry without a
    // player packet or a test-driven call to the quest deadline drain.
    let stop = oteryn_foundation::CancellationToken::new();
    let observe = async {
        tokio::time::advance(Duration::from_secs(2)).await;
        // Keep the scheduler's unrelated durable passes on real time.
        tokio::time::resume();
        loop {
            let observed = creatures(owner).await?;
            if !observed.is_empty() {
                stop.cancel();
                return Ok::<_, Box<dyn std::error::Error>>(observed);
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    };
    let respawn = tokio::time::timeout(Duration::from_secs(10), async {
        let (_, observed) = super::fixture::join_two(owner.run_source_owner_cycles(&stop), observe).await;
        observed
    })
    .await??;
    assert_eq!(respawn.len(), 1);
    assert_ne!(respawn[0].0, *actor);
    tokio::time::pause();
    advance(owner, 5_000).await;
    assert_eq!(
        creatures(owner).await?,
        respawn,
        "live identity holds after cooldown too"
    );
    tokio::time::resume();
    Ok(())
}

async fn wave_scenario(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    player: ExactActorRef,
    case: Case,
) -> TestResult {
    let victory = case.has_victory_binding();
    let timeout_edge = matches!(
        case,
        Case::VictoryBeforeTimeout | Case::VictoryWithPendingReflection | Case::DeathAtTimeout
    );
    let timed_out = matches!(case, Case::DeathAtTimeout);
    assert_eq!(
        use_room(owner, admitted, 1).await?,
        UseDisposition::Rejected
    );
    assert_eq!(position(owner, player).await?, local(START));
    let peer = stage_peer(h, owner, admitted).await?;
    let peer_actor = peer.runtime_actor.ok_or("staged peer actor")?;
    let blocker = position_session_before_lease(owner, SECOND).await?;
    assert_eq!(
        use_room(owner, admitted, 2).await?,
        UseDisposition::Occupied
    );
    assert_eq!(position(owner, player).await?, local(START));
    assert_eq!(position(owner, peer_actor).await?, local(PEER_START));
    owner
        .runtime
        .lock()
        .await
        .remove_test_actor(blocker)
        .map_err(debug)?;

    // An occupied USE consumes the per-player world action cooldown.
    tokio::time::sleep(Duration::from_millis(200)).await;
    let (release, released) = std::sync::mpsc::channel::<()>();
    let (ready, started) = tokio::sync::oneshot::channel();
    let blocking = tokio::task::spawn_blocking(move || {
        let _ = ready.send(());
        let _ = released.recv_timeout(Duration::from_secs(45));
    });
    started.await?;
    tokio::time::pause();
    let clock = timeout_edge.then(|| owner.freeze_owner_clock_for_test());
    let start_us = owner.owner_now().get();
    let result = use_room(owner, admitted, 3).await;
    drop(release);
    blocking.await?;
    assert_eq!(result?, UseDisposition::Committed);
    assert_eq!(position(owner, player).await?, local(LANDING));
    assert_eq!(position(owner, peer_actor).await?, local(PEER_LANDING));
    assert_eq!(next_spawn_delay(owner).await, Some(Duration::from_secs(1)));
    advance(owner, 999).await;
    assert_eq!(next_spawn_delay(owner).await, Some(Duration::from_millis(1)));
    assert!(creatures(owner).await?.is_empty());
    if victory {
        advance(owner, 1).await;
        let first = creatures(owner).await?;
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].1, local(FIRST));
        project_wave_death(owner, first[0].0, b"first-wave").await?;
        owner.drain_quest_encounter_deadlines().await;
        assert_eq!(position(owner, player).await?, local(LANDING));
        assert_eq!(position(owner, peer_actor).await?, local(PEER_LANDING));
        advance(owner, 2_000).await;
    } else {
        // Both independent timers are due when the node next drains its
        // scheduler (Crystal action_arena.lua:80-83).
        advance(owner, 2_001).await;
        assert_eq!(next_spawn_delay(owner).await, Some(Duration::from_secs(7)));
    }
    let active = creatures(owner).await?;
    assert_eq!(active.len(), if victory { 1 } else { 2 });
    let exit = if victory {
        let [(terminal, at)] = active.as_slice() else {
            return Err("terminal wave spawn".into());
        };
        // Crystal map.cpp:272-307 tries adjacent cells when the bound center
        // is occupied. Stay inside the room and clear of both staged players.
        assert_eq!(at.x, local(FIRST).x);
        assert!((local(FIRST).y..=local(PEER_LANDING).y).contains(&at.y));
        if timeout_edge {
            // Delay the scheduler across cleanup (Crystal action_arena.lua:71).
            // Stamp the lethal turn independently at the boundary or just before it.
            let lethal_ms = if timed_out { 10_000 } else { 9_999 };
            tokio::time::advance(Duration::from_millis(lethal_ms - 3_000)).await;
            owner.states.test_owner_time.store(
                start_us + lethal_ms * 1_000 + 1,
                std::sync::atomic::Ordering::Relaxed,
            );
        }
        if matches!(case, Case::VictoryWithPendingReflection) {
            let mut runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            runtime.set_test_creature_health(*terminal, 1).map_err(debug)?;
            states
                .get_mut(&runtime, player, admitted.game_session_id)
                .ok_or("primary combat state")?
                .install_equipped_offence(
                    oteryn_gameplay_combat_rules::resolution::equipped_offence::EquippedOffence {
                        reflect_percent: 10_000,
                        ..Default::default()
                    },
                );
            assert!(states.reflect_equipment(
                &mut runtime,
                *terminal,
                player,
                admitted.game_session_id,
                Some("physical"),
                1,
                b"terminal-wave-reflection",
                SemanticTimeMicros::from_micros(owner.owner_now().get()),
            ));
            assert!(!runtime.contains_live_creature(*terminal));
            assert_eq!(states.equipment_reflection_kills.len(), 1);
            assert_eq!(states.equipment_reflection_kills[0].0, *terminal);
            assert!(
                runtime
                    .borrow_combat_death()
                    .projected_death_at_ms(*terminal)
                    .is_err()
            );
        } else {
            project_wave_death(owner, *terminal, b"terminal-wave").await?;
        }
        if timeout_edge {
            tokio::time::advance(Duration::from_millis(2)).await;
            assert_eq!(next_spawn_delay(owner).await, Some(Duration::ZERO));
        }
        owner.drain_quest_encounter_deadlines().await;
        tokio::time::resume();
        if timed_out { EXIT } else { VICTORY_EXIT }
    } else {
        assert!(active.iter().any(|(_, at)| *at == local(FIRST)));
        assert!(active.iter().any(|(_, at)| *at == local(SECOND)));
        advance(owner, 6_999).await;
        assert_eq!(next_spawn_delay(owner).await, Some(Duration::from_millis(1)));
        assert_eq!(position(owner, player).await?, local(LANDING));
        assert_eq!(position(owner, peer_actor).await?, local(PEER_LANDING));
        assert_eq!(creatures(owner).await?, active);

        // Crystal action_arena.lua:39-50,71 arms cleanup without a client
        // packet. Cross the deadline on the paused clock and let the actual
        // source owner perform egress and cleanup.
        tokio::time::advance(Duration::from_millis(1)).await;
        tokio::time::resume();
        let stop = oteryn_foundation::CancellationToken::new();
        let observe = async {
            loop {
                if position(owner, player).await? == local(EXIT)
                    && position(owner, peer_actor).await? == local(EXIT)
                    && creatures(owner).await?.is_empty()
                {
                    stop.cancel();
                    return Ok::<_, Box<dyn std::error::Error>>(());
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        };
        tokio::time::timeout(Duration::from_secs(10), async {
            let (_, result) =
                super::fixture::join_two(owner.run_source_owner_cycles(&stop), observe).await;
            result
        })
        .await??;
        EXIT
    };
    assert_eq!(position(owner, player).await?, local(exit));
    assert_eq!(position(owner, peer_actor).await?, local(exit));
    assert!(creatures(owner).await?.is_empty());
    assert_eq!(next_spawn_delay(owner).await, None);
    drop(clock);

    let primary = admitted
        .item_fence
        .ok_or("primary item fence")?
        .character_id;
    let partner = peer.item_fence.ok_or("peer item fence")?.character_id;
    assert_ne!(primary, partner);
    owner.drain_quest_encounter_deadlines().await;
    owner.drain_quest_encounter_deadlines().await;
    let read_credit = async |character| {
        owner
            .root
            .read_character_quest_state(owner.character, character)
            .await
            .map_err(debug)
    };
    let primary_after = read_credit(primary).await?;
    let partner_after = read_credit(partner).await?;
    let progress = (!timed_out).then_some(i64::from(!matches!(case, Case::VictoryWithoutProgress)));
    assert_eq!(primary_after.tracks().get(TRACK).copied(), progress);
    assert_eq!(partner_after.tracks().get(TRACK).copied(), progress);
    owner.drain_quest_encounter_deadlines().await;
    assert_eq!(read_credit(primary).await?, primary_after);
    assert_eq!(read_credit(partner).await?, partner_after);
    let grants: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_account_achievement_grant_requests
         WHERE achievement_key = $1 AND source_kind = 'oteryn:quest-wave-victory'",
    )
    .bind(CHAMPION)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(grants, if victory && !timed_out { 2 } else { 0 });
    Ok(())
}

async fn project_wave_death(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    occurrence: &[u8],
) -> TestResult {
    let mut runtime = owner.runtime.lock().await;
    let health = runtime.companion_snapshot(actor).map_err(debug)?.health;
    let hit = runtime
        .borrow_exact_actor_commit_at_us(owner.owner_now().get())
        .commit_damage(actor, crate::foundation::OwnerDamageCommand {
            target: CAVE_RAT.as_bytes(),
            occurrence,
            binding: occurrence,
            damage: health,
        })
        .map_err(debug)?;
    assert!(hit.applied && hit.health_before > 0 && hit.health_after == 0);
    crate::combat::project_fixed_one_creature_death(&mut runtime.borrow_combat_death(), actor)
        .map_err(debug)?;
    Ok(())
}

async fn stage_peer(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    leader: &AdmittedSession,
) -> TestResult<AdmittedSession> {
    use crate::gameplay_transport::connection::{
        ControllerBinding, FirstEntryOutcome, FreshAdmissionAuthority, SessionContinuity,
    };
    use crate::world_party_runtime_postgres_cases::{
        FreshPlayerOwners, committed_fresh_player_with_operation,
    };
    use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let (current, operation) = committed_fresh_player_with_operation(
        &FreshPlayerOwners {
            root: h.root.clone(),
            node: h.node.clone(),
            recovery: &h.recovery,
        },
        60,
        63,
        171,
        now,
    )
    .await
    .map_err(debug)?;
    let fence = crate::gameplay_transport::item_fence_of(current).ok_or("peer item fence")?;
    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime
            .reserve_fresh_session(fence.game_session_id)
            .map_err(debug)?;
        runtime.commit_fresh_session(reservation).map_err(debug)?
    };
    let mut peer = AdmittedSession {
        game_session_id: fence.game_session_id,
        world_id: leader.world_id,
        channel_id: leader.channel_id,
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: Some(ControllerBinding {
            transport: current.current_transport().ok_or("peer transport")?,
            account_id: id(63),
        }),
        continuity: SessionContinuity::FRESH,
        item_fence: Some(fence),
    };
    super::srx3_activity_source_fixture::prepare_player_with_operation(
        h, owner, &mut peer, 100, "knight", &operation,
    )
    .await?;
    let (step, _) = owner
        .paced_step(actor, peer.game_session_id, StepDirection::North)
        .await;
    assert_eq!(step.disposition, StepDisposition::Moved);
    assert_eq!(position(owner, actor).await?, local(PEER_START));
    Ok(peer)
}

async fn earth_step_scenario(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    player: ExactActorRef,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::{
        StepDirection, StepDisposition, decode_step_result, encode_step_intent,
    };
    let health = async |session| -> TestResult<_> {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        Ok(states
            .get(&runtime, player, session)
            .ok_or("vitals")?
            .vitals()
            .health)
    };
    let before = health(admitted.game_session_id).await?;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        1,
        1,
        encode_step_intent(StepDirection::East),
    )
    .await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    assert_eq!(position(owner, player).await?, local(TRIGGER));
    assert_eq!(before - health(admitted.game_session_id).await?, 35);
    owner.drain_quest_encounter_deadlines().await;
    assert_eq!(before - health(admitted.game_session_id).await?, 35);
    Ok(())
}

async fn survival_step_scenario(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    player: ExactActorRef,
) -> TestResult {
    guarded_wave_step(owner, admitted).await?;
    assert_eq!(position(owner, player).await?, local(LANDING));
    advance(owner, 9_999).await;
    assert_eq!(position(owner, player).await?, local(LANDING));
    assert!(creatures(owner).await?.is_empty());
    advance(owner, 1).await;
    assert_eq!(position(owner, player).await?, local(EXIT));
    tokio::time::resume();

    let character = admitted.item_fence.ok_or("trial item fence")?.character_id;
    owner.drain_quest_encounter_deadlines().await;
    let state = owner
        .root
        .read_character_quest_state(owner.character, character)
        .await
        .map_err(debug)?;
    assert_eq!(state.tracks().get(TRACK), Some(&1));
    Ok(())
}

async fn stations_scenario(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    player: ExactActorRef,
) -> TestResult {
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;
    use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

    let peer = stage_peer(h, owner, admitted).await?;
    let peer_actor = peer.runtime_actor.ok_or("staged peer")?;
    let map = owner.bundle_map().ok_or("station map")?;
    let hatch_key = oteryn_world_bundle::bundle::placement_key(HATCH.floor, HATCH.x, HATCH.y, 1)
        .ok_or("station hatch key")?;
    let hatch_tile = crate::map::overlay::TilePos {
        x: HATCH.x,
        y: HATCH.y,
        floor: HATCH.floor,
    };
    let hatch_palette = *map
        .overlay
        .base()
        .tile(HATCH.x, HATCH.y, HATCH.floor)
        .and_then(|tile| tile.ids().get(1))
        .ok_or("station hatch placement")?;
    let hatch_appearance = || {
        map.facts
            .base_entry(hatch_tile, 1, hatch_palette)
            .map(|facts| facts.appearance_id)
    };
    assert_eq!(map.facts.object_revision(hatch_key), 0);
    assert_eq!(hatch_appearance(), Some(1897));
    guarded_wave_step(owner, admitted).await?;
    assert_eq!(position(owner, player).await?, local(TRIGGER));
    assert_eq!(position(owner, peer_actor).await?, local(PEER_START));
    assert_eq!(map.facts.object_revision(hatch_key), 1);
    assert_eq!(hatch_appearance(), Some(1977));
    let close_at = map.facts.next_movement_decay_time().ok_or("station hatch timer")?;
    assert_eq!(map.facts.expire_movement_items(close_at - 1), 0);
    assert_eq!(map.facts.object_revision(hatch_key), 1);
    advance(owner, 999).await;
    assert!(creatures(owner).await?.is_empty());
    advance(owner, 1).await;
    let spawned = creatures(owner).await?;
    assert_eq!(spawned.len(), 1);
    assert_eq!(spawned[0].1, local(FIRST));

    // Crystal movements_click.lua:106-125 opens a separate route. Move one
    // station member into the room through the normal movement owner.
    let _clock = owner.freeze_owner_clock_for_test();
    let (release, released) = std::sync::mpsc::channel::<()>();
    let (ready, started) = tokio::sync::oneshot::channel();
    let blocking = tokio::task::spawn_blocking(move || {
        let _ = ready.send(());
        let _ = released.recv_timeout(Duration::from_secs(45));
    });
    started.await?;
    for expected in [row(1), LANDING] {
        owner.states.test_owner_time.store(
            owner.owner_now().get() + 1_000_001,
            std::sync::atomic::Ordering::Relaxed,
        );
        let (step, _) = owner.paced_step(player, admitted.game_session_id, StepDirection::East).await;
        assert_eq!(step.disposition, StepDisposition::Moved);
        assert_eq!(position(owner, player).await?, local(expected));
    }
    drop(release);
    blocking.await?;
    advance(owner, 2_000).await;
    assert_eq!(map.facts.expire_movement_items(close_at), 1);
    assert_eq!(map.facts.object_revision(hatch_key), 2);
    assert_eq!(hatch_appearance(), Some(1897));
    advance(owner, 6_999).await;
    assert_eq!(position(owner, player).await?, local(LANDING));

    let arriving = position_session_before_lease(owner, PEER_LANDING).await?;
    advance(owner, 1).await;
    assert_eq!(position(owner, player).await?, local(EXIT));
    assert_eq!(position(owner, peer_actor).await?, local(PEER_START));
    assert_eq!(position(owner, arriving).await?, local(PEER_LANDING));
    assert!(!creatures(owner).await?.is_empty());

    tokio::time::resume();
    // The exit has committed, but the scheduler has not settled its credit.
    // A normal logout must preserve the session fence needed by the writer.
    assert!(owner.logout_preflight(*admitted).await.is_ok());
    let token = owner
        .runtime
        .lock()
        .await
        .mint_transition_fence()
        .map_err(debug)?;
    let mut hold = FenceHold::new(admitted.game_session_id, TransitionFence::Transition(token));
    assert!(matches!(
        owner.fence_transition(player, &mut hold, true).await,
        FenceStep::Wait
    ));
    assert!(matches!(
        owner.logout_session(*admitted).await,
        crate::gameplay_transport::logout::LogoutReply::Result(
            oteryn_protocol_oteryn::logout::LogoutResult::Busy
        )
    ));
    owner.drain_quest_encounter_deadlines().await;
    for (staged, credit) in [(admitted.item_fence, 1), (peer.item_fence, 0)] {
        let character = staged.ok_or("staging item fence")?.character_id;
        let state = owner
            .root
            .read_character_quest_state(owner.character, character)
            .await
            .map_err(debug)?;
        assert_eq!(state.tracks().get(TRACK).copied().unwrap_or(0), credit);
    }
    assert!(!owner.has_pending_wave_credit(admitted.game_session_id));

    owner.runtime.lock().await.remove_test_actor(arriving).map_err(debug)?;
    owner.drain_quest_encounter_deadlines().await;
    assert!(creatures(owner).await?.is_empty());
    Ok(())
}

async fn position_session_before_lease(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    position: Position,
) -> TestResult<ExactActorRef> {
    let mut runtime = owner.runtime.lock().await;
    let session = GameSessionId::decode(&id(58)).map_err(debug)?;
    let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
    let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_pinned_test_position(actor, local(position))
        .map_err(debug)?;
    Ok(actor)
}

async fn guarded_wave_step(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::{
        StepDirection, StepDisposition, decode_step_result, encode_step_intent,
    };

    // Keep paused time from auto-advancing during the first-entry database reads.
    let (release, released) = std::sync::mpsc::channel::<()>();
    let (ready, started) = tokio::sync::oneshot::channel();
    let blocking = tokio::task::spawn_blocking(move || {
        let _ = ready.send(());
        let _ = released.recv_timeout(Duration::from_secs(45));
    });
    started.await?;
    tokio::time::pause();
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        1,
        1,
        encode_step_intent(StepDirection::East),
    )
    .await;
    drop(release);
    blocking.await?;
    assert_eq!(
        decode_step_result(&reply?).map_err(debug)?,
        StepDisposition::Moved
    );
    Ok(())
}

async fn use_room(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
) -> TestResult<UseDisposition> {
    let reply = super::quest_world_callbacks::wire(owner, admitted, id, 2, use_room_payload(owner)?).await?;
    oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)
}

fn use_room_payload(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult<Vec<u8>> {
    use oteryn_protocol_oteryn::world_object::encode_use_intent;
    let key = oteryn_world_bundle::bundle::placement_key(TRIGGER.floor, TRIGGER.x, TRIGGER.y, 1)
        .ok_or("fixture USE placement")?;
    let map = owner.bundle_map().ok_or("fixture map")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
    .map_err(debug)?)
}

async fn advance(owner: &ComposedFreshAdmission<'_, '_, '_>, millis: u64) {
    tokio::time::advance(Duration::from_millis(millis)).await;
    owner.drain_quest_encounter_deadlines().await;
}

async fn next_spawn_delay(owner: &ComposedFreshAdmission<'_, '_, '_>) -> Option<Duration> {
    let runtime = owner.runtime.lock().await;
    owner.states.quest_callbacks
        .next_spawn_deadline(&runtime)
        .map(|due| due.saturating_duration_since(tokio::time::Instant::now()))
}

async fn creatures(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult<Vec<(ExactActorRef, MovementLocalPosition)>> {
    let runtime = owner.runtime.lock().await;
    let mut actors = Vec::new();
    for actor in runtime.current_live_creature_census().map_err(debug)? {
        actors.push((
            actor,
            runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
        ));
    }
    Ok(actors)
}

async fn position(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
) -> TestResult<MovementLocalPosition> {
    Ok(owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position())
}

fn local(position: Position) -> MovementLocalPosition {
    MovementLocalPosition {
        x: i32::from(position.x),
        y: i32::from(position.y),
        floor: -i16::from(position.floor),
    }
}
