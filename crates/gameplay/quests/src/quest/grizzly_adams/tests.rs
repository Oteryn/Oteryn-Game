use super::*;

fn actor(level: u32) -> GrizzlyActor {
    GrizzlyActor {
        level,
        premium: true,
        level_after_reward: Some(level),
    }
}

fn apply(
    state: &mut GrizzlyState,
    level: u32,
    action: GrizzlyAction,
) -> Result<GrizzlyTransition, GrizzlyRefusal> {
    let result = plan_action(state, actor(level), &action)?;
    for change in &result.changes {
        state.insert(change.track.clone(), change.after);
    }
    Ok(result)
}

fn member() -> Result<GrizzlyState, GrizzlyRefusal> {
    let mut state = GrizzlyState::new();
    apply(&mut state, 6, GrizzlyAction::Join)?;
    Ok(state)
}

fn set(state: &mut GrizzlyState, id: u32, value: i64) {
    state.insert(storage_track(id), value);
}

#[test]
fn grizzly_catalogue_has_all_tasks_bosses_and_unique_ordinary_creatures()
-> Result<(), GrizzlyRefusal> {
    let data = catalogue()?;
    assert_eq!(data.tasks.len(), 42);
    assert_eq!(data.bosses.len(), 30);
    assert_eq!(task(1).ok_or(GrizzlyRefusal::InvalidTask)?.required, 300);
    assert_eq!(task(42).ok_or(GrizzlyRefusal::InvalidTask)?.required, 6666);
    assert_eq!(task(11).ok_or(GrizzlyRefusal::InvalidTask)?.points, None);
    assert_eq!(task(27).ok_or(GrizzlyRefusal::InvalidTask)?.points, None);
    let mut seen = std::collections::BTreeSet::new();
    for task in &data.tasks {
        for creature in &task.creatures {
            assert!(
                seen.insert(&creature.key),
                "a death must not target two tasks"
            );
        }
    }
    assert!(
        data.bosses
            .iter()
            .all(|boss| !seen.contains(&boss.creature_key))
    );
    assert_eq!(task_by_name("MEDUSA").map(|task| task.id), Some(34));
    assert_eq!(task_by_name("werewolve").map(|task| task.id), Some(25));
    assert!(task_by_name("unknown").is_none());
    Ok(())
}

#[test]
fn grizzly_join_is_explicit_and_initial_source_storage_is_minus_one() -> Result<(), GrizzlyRefusal>
{
    let mut state = GrizzlyState::new();
    assert_eq!(storage_value(&state, POINTS_STORAGE), -1);
    assert_eq!(rank(&state), 0);
    assert_eq!(
        can_start_task(&state, actor(6), 1),
        Err(GrizzlyRefusal::NotJoined)
    );
    apply(&mut state, 6, GrizzlyAction::Join)?;
    assert!(joined(&state));
    assert_eq!(rank(&state), 1);
    assert_eq!(storage_value(&state, POINTS_STORAGE), 0);
    assert_eq!(storage_value(&state, BOSS_POINTS_STORAGE), 0);
    assert_eq!(
        plan_action(&state, actor(6), &GrizzlyAction::Join),
        Err(GrizzlyRefusal::AlreadyJoined)
    );
    Ok(())
}

#[test]
fn grizzly_task_eligibility_rejects_level_premium_rank_and_invalid_ids()
-> Result<(), GrizzlyRefusal> {
    let mut state = member()?;
    for id in [0, 43, u8::MAX] {
        assert_eq!(
            can_start_task(&state, actor(6), id),
            Err(GrizzlyRefusal::InvalidTask)
        );
    }
    for level in [0, 5, 50, 9999] {
        assert_eq!(
            can_start_task(&state, actor(level), 1),
            Err(GrizzlyRefusal::Level)
        );
    }
    for level in [6, 49] {
        assert_eq!(can_start_task(&state, actor(level), 1), Ok(()));
    }
    assert_eq!(
        can_start_task(
            &state,
            GrizzlyActor {
                premium: false,
                ..actor(6)
            },
            1
        ),
        Err(GrizzlyRefusal::Premium)
    );
    assert_eq!(
        can_start_task(&state, actor(130), 42),
        Err(GrizzlyRefusal::Rank)
    );
    set(&mut state, POINTS_STORAGE, 100);
    assert_eq!(can_start_task(&state, actor(130), 42), Ok(()));
    assert_eq!(
        can_start_task(
            &state,
            GrizzlyActor {
                premium: false,
                ..actor(130)
            },
            42
        ),
        Err(GrizzlyRefusal::Premium)
    );
    Ok(())
}

#[test]
fn grizzly_three_tasks_cancel_resets_count_without_spending_repeat() -> Result<(), GrizzlyRefusal> {
    let mut state = member()?;
    for id in [1, 2, 3] {
        apply(&mut state, 6, GrizzlyAction::StartTask(id))?;
    }
    assert_eq!(started_tasks(&state), vec![1, 2, 3]);
    assert_eq!(
        can_start_task(&state, actor(6), 1),
        Err(GrizzlyRefusal::AlreadyStarted)
    );
    assert_eq!(
        can_start_task(&state, actor(6), 4),
        Err(GrizzlyRefusal::TaskLimit)
    );
    apply(
        &mut state,
        6,
        GrizzlyAction::KillCreature {
            task_id: 1,
            creature: "crocodile".into(),
            observed_revision: 1,
        },
    )?;
    apply(&mut state, 6, GrizzlyAction::CancelTask(1))?;
    assert_eq!(storage_value(&state, KILL_COUNTER_BASE + 1), 0);
    assert_eq!(storage_value(&state, KILLS_STORAGE_BASE + 1), -1);
    assert_eq!(storage_value(&state, REPEAT_STORAGE_BASE + 1), -1);
    assert_eq!(
        plan_action(&state, actor(6), &GrizzlyAction::CancelTask(1)),
        Err(GrizzlyRefusal::NotStarted)
    );
    apply(&mut state, 6, GrizzlyAction::StartTask(4))?;
    Ok(())
}

#[test]
fn grizzly_kills_match_exact_creatures_update_species_and_keep_counting_until_report()
-> Result<(), GrizzlyRefusal> {
    let mut state = member()?;
    apply(&mut state, 6, GrizzlyAction::StartTask(9))?;
    for creature in ["kongra", "oteryn:creature.merlkin", "SIBANG"] {
        apply(
            &mut state,
            6,
            GrizzlyAction::KillCreature {
                task_id: 9,
                creature: creature.into(),
                observed_revision: 1,
            },
        )?;
    }
    assert_eq!(storage_value(&state, KILL_COUNTER_BASE + 9), 3);
    for id in 42450..=42452 {
        assert_eq!(storage_value(&state, id), 1);
    }
    assert_eq!(
        plan_action(
            &state,
            actor(6),
            &GrizzlyAction::KillCreature {
                task_id: 9,
                creature: "rat".into(),
                observed_revision: 1
            }
        ),
        Err(GrizzlyRefusal::CreatureMismatch)
    );
    assert_eq!(
        plan_action(&state, actor(6), &GrizzlyAction::CompleteTask(9)),
        Err(GrizzlyRefusal::KillsIncomplete)
    );
    set(&mut state, KILL_COUNTER_BASE + 9, 300);
    apply(
        &mut state,
        6,
        GrizzlyAction::KillCreature {
            task_id: 9,
            creature: "kongra".into(),
            observed_revision: 1,
        },
    )?;
    assert_eq!(storage_value(&state, KILL_COUNTER_BASE + 9), 301);
    apply(&mut state, 6, GrizzlyAction::CompleteTask(9))?;
    assert_eq!(
        plan_action(
            &state,
            actor(6),
            &GrizzlyAction::KillCreature {
                task_id: 9,
                creature: "kongra".into(),
                observed_revision: 1
            }
        ),
        Err(GrizzlyRefusal::NotStarted)
    );
    Ok(())
}

#[test]
fn grizzly_points_boundaries_follow_pre_reward_caps_and_level_130_bonuses() {
    for (level, current, base, expected) in [
        (49, 39, 3, 3),
        (49, 40, 3, 0),
        (50, 69, 3, 3),
        (79, 70, 3, 0),
        (80, 99, 4, 4),
        (129, 100, 4, 0),
        (130, 20, 3, 6),
        (130, 21, 3, 5),
        (130, 40, 3, 5),
        (130, 41, 3, 4),
        (130, 70, 3, 4),
        (130, 71, 3, 3),
        // Source dialogue says no points after Elite, but the actual sourced branch adds them.
        (130, 100, 3, 3),
    ] {
        assert_eq!(
            points_reward(level, current, base),
            expected,
            "level={level} points={current}"
        );
    }
}

#[test]
fn grizzly_completion_uses_post_xp_level_and_refuses_absent_projection()
-> Result<(), GrizzlyRefusal> {
    for (before, after, points, expected) in [
        (49, 50, 40, 41),
        (79, 80, 70, 71),
        (129, 130, 100, 101),
        (129, 130, 20, 24),
    ] {
        let mut state = member()?;
        apply(&mut state, 6, GrizzlyAction::StartTask(1))?;
        set(&mut state, KILL_COUNTER_BASE + 1, 300);
        set(&mut state, POINTS_STORAGE, points);
        let mut hunter = GrizzlyActor {
            level_after_reward: None,
            ..actor(before)
        };
        assert_eq!(
            plan_action(&state, hunter, &GrizzlyAction::CompleteTask(1)),
            Err(GrizzlyRefusal::ProgressionUnavailable)
        );
        hunter.level_after_reward = Some(before - 1);
        assert_eq!(
            plan_action(&state, hunter, &GrizzlyAction::CompleteTask(1)),
            Err(GrizzlyRefusal::ProgressionUnavailable)
        );
        hunter.level_after_reward = Some(after);
        let plan = plan_action(&state, hunter, &GrizzlyAction::CompleteTask(1))?;
        for change in plan.changes {
            state.insert(change.track, change.after);
        }
        assert_eq!(storage_value(&state, POINTS_STORAGE), expected);
    }
    Ok(())
}

#[test]
fn grizzly_three_completions_lock_low_task_and_existing_boss_becomes_a_boss_point()
-> Result<(), GrizzlyRefusal> {
    let mut state = member()?;
    for repetition in 1..=3 {
        apply(&mut state, 6, GrizzlyAction::StartTask(1))?;
        assert_eq!(storage_value(&state, KILL_COUNTER_BASE + 1), 0);
        set(&mut state, KILL_COUNTER_BASE + 1, 300);
        let reward = apply(&mut state, 6, GrizzlyAction::CompleteTask(1))?;
        assert_eq!(reward.experience, Some(800));
        assert!(reward.achievement.is_none());
        assert_eq!(storage_value(&state, REPEAT_STORAGE_BASE + 1), repetition);
        assert_eq!(storage_value(&state, POINTS_STORAGE), repetition);
        assert_eq!(storage_value(&state, BOSS_POINTS_STORAGE), repetition - 1);
        assert_eq!(storage_value(&state, 34100), 1);
    }
    assert_eq!(storage_value(&state, KILLS_STORAGE_BASE + 1), 2);
    assert_eq!(
        can_start_task(&state, actor(6), 1),
        Err(GrizzlyRefusal::RepeatLimit)
    );
    assert_eq!(
        plan_action(&state, actor(6), &GrizzlyAction::CompleteTask(1)),
        Err(GrizzlyRefusal::NotStarted)
    );
    Ok(())
}

#[test]
fn grizzly_high_tasks_repeat_and_demons_award_holy_icon_once() -> Result<(), GrizzlyRefusal> {
    let mut state = member()?;
    for _ in 0..4 {
        apply(&mut state, 130, GrizzlyAction::StartTask(32))?;
        set(&mut state, KILL_COUNTER_BASE + 32, 650);
        apply(&mut state, 130, GrizzlyAction::CompleteTask(32))?;
        assert_eq!(storage_value(&state, REPEAT_STORAGE_BASE + 32), 0);
    }
    set(&mut state, POINTS_STORAGE, 100);
    apply(&mut state, 130, GrizzlyAction::StartTask(42))?;
    set(&mut state, KILL_COUNTER_BASE + 42, 6666);
    let reward = apply(&mut state, 130, GrizzlyAction::CompleteTask(42))?;
    assert_eq!(
        reward.item,
        Some(GrizzlyItemReward {
            source_id: 9388,
            count: 1
        })
    );
    assert_eq!(reward.experience, None);
    assert_eq!(storage_value(&state, 41300), 1);
    assert_eq!(storage_value(&state, QUEST_STORAGE_BASE + 42), 2);
    assert_eq!(
        can_start_task(&state, actor(130), 42),
        Err(GrizzlyRefusal::AlreadyStarted)
    );
    Ok(())
}

#[test]
fn grizzly_point_rank_and_confirmed_rank_advance_independently() -> Result<(), GrizzlyRefusal> {
    let mut state = member()?;
    for (points, expected) in [(0, 1), (10, 2), (20, 3), (40, 4), (70, 5), (100, 6)] {
        set(&mut state, POINTS_STORAGE, points);
        assert_eq!(rank(&state), expected);
    }
    for expected in [0, 2, 4, 6, 7] {
        apply(&mut state, 130, GrizzlyAction::Promote)?;
        assert_eq!(storage_value(&state, PROMOTION_STORAGE), expected);
    }
    assert_eq!(
        plan_action(&state, actor(130), &GrizzlyAction::Promote),
        Err(GrizzlyRefusal::NoPromotion)
    );
    for (rank, low, allowed, next) in [
        (-1, 5, 6, 0),
        (0, 5, 6, 2),
        (2, 49, 50, 4),
        (4, 79, 80, 6),
        (6, 129, 130, 7),
    ] {
        set(&mut state, PROMOTION_STORAGE, rank);
        assert_eq!(promotion_due(&state, low), None);
        assert_eq!(promotion_due(&state, allowed), Some(next));
    }
    Ok(())
}

#[test]
fn grizzly_boss_selection_is_level_gated_and_free_first_lower_bracket_visit_keeps_point()
-> Result<(), GrizzlyRefusal> {
    let mut state = member()?;
    assert_eq!(
        plan_action(&state, actor(6), &GrizzlyAction::SelectBoss(1)),
        Err(GrizzlyRefusal::BossPoints)
    );
    set(&mut state, BOSS_POINTS_STORAGE, 2);
    assert_eq!(
        plan_action(&state, actor(49), &GrizzlyAction::SelectBoss(5)),
        Err(GrizzlyRefusal::Level)
    );
    apply(&mut state, 50, GrizzlyAction::SelectBoss(1))?;
    assert_eq!(storage_value(&state, BOSS_POINTS_STORAGE), 2);
    assert_eq!(
        plan_action(&state, actor(50), &GrizzlyAction::SelectBoss(1)),
        Err(GrizzlyRefusal::BossActive)
    );
    set(&mut state, 34100, 3);
    apply(&mut state, 50, GrizzlyAction::SelectBoss(1))?;
    assert_eq!(storage_value(&state, BOSS_POINTS_STORAGE), 1);
    apply(&mut state, 130, GrizzlyAction::SelectBoss(19))?;
    assert_eq!(storage_value(&state, BOSS_POINTS_STORAGE), 0);
    for id in [20, 2] {
        assert_eq!(
            apply(&mut state, 130, GrizzlyAction::SelectBoss(id)),
            Err(GrizzlyRefusal::BossPoints),
            "Q17 covers both paid and free lower-bracket selections"
        );
        assert_eq!(storage_value(&state, BOSS_POINTS_STORAGE), 0);
        assert_eq!(
            storage_value(&state, boss(id).ok_or(GrizzlyRefusal::InvalidBoss)?.storage),
            -1
        );
    }
    assert_eq!(
        plan_action(&state, actor(130), &GrizzlyAction::SelectBoss(29)),
        Err(GrizzlyRefusal::InvalidBoss)
    );
    Ok(())
}

#[test]
fn grizzly_boss_enter_report_recovery_and_achievement_follow_donor_stages()
-> Result<(), GrizzlyRefusal> {
    let mut state = member()?;
    assert_eq!(
        check_boss_room(&state, 1, false),
        Err(GrizzlyRefusal::BossNotEntered)
    );
    set(&mut state, 34100, 1);
    set(&mut state, 42350, 0);
    assert_eq!(
        check_boss_room(&state, 1, true),
        Err(GrizzlyRefusal::RoomOccupied)
    );
    apply(&mut state, 6, GrizzlyAction::EnterBoss(1))?;
    apply(&mut state, 6, GrizzlyAction::ReportBoss(1))?;
    assert_eq!(storage_value(&state, 34100), 1);
    apply(&mut state, 6, GrizzlyAction::EnterBoss(1))?;
    apply(
        &mut state,
        6,
        GrizzlyAction::BossKill {
            boss_id: 1,
            observed_revision: 1,
        },
    )?;
    let report = apply(&mut state, 6, GrizzlyAction::ReportBoss(1))?;
    assert_eq!(report.achievement.as_deref(), Some("Blood-Red Snapper"));
    assert_eq!(storage_value(&state, 34100), 3);
    assert_eq!(storage_value(&state, 42350), 0);
    assert_eq!(
        plan_action(&state, actor(6), &GrizzlyAction::ReportBoss(1)),
        Err(GrizzlyRefusal::BossNotEntered)
    );
    set(&mut state, 34100, 2);
    set(&mut state, 42350, 2);
    assert_eq!(
        plan_action(&state, actor(6), &GrizzlyAction::ReportBoss(1)),
        Err(GrizzlyRefusal::BossNotKilled)
    );
    Ok(())
}

#[test]
fn grizzly_special_tasks_are_sequential_recoverable_and_once_only() -> Result<(), GrizzlyRefusal> {
    let mut state = member()?;
    assert_eq!(
        plan_action(&state, actor(79), &GrizzlyAction::StartSpecialTask),
        Err(GrizzlyRefusal::Level)
    );
    assert_eq!(
        plan_action(&state, actor(80), &GrizzlyAction::StartSpecialTask),
        Err(GrizzlyRefusal::Rank)
    );
    set(&mut state, POINTS_STORAGE, 70);
    for id in [29, 30] {
        let boss = boss(id).ok_or(GrizzlyRefusal::InvalidBoss)?;
        apply(&mut state, 80, GrizzlyAction::StartSpecialTask)?;
        assert_eq!(storage_value(&state, boss.storage), 1);
        apply(&mut state, 80, GrizzlyAction::EnterBoss(id))?;
        apply(&mut state, 80, GrizzlyAction::StartSpecialTask)?;
        assert_eq!(storage_value(&state, boss.storage), 1);
        apply(&mut state, 80, GrizzlyAction::EnterBoss(id))?;
        apply(
            &mut state,
            80,
            GrizzlyAction::BossKill {
                boss_id: id,
                observed_revision: 1,
            },
        )?;
        apply(&mut state, 80, GrizzlyAction::StartSpecialTask)?;
        assert_eq!(storage_value(&state, boss.storage), 3);
    }
    assert_eq!(
        plan_action(&state, actor(80), &GrizzlyAction::StartSpecialTask),
        Err(GrizzlyRefusal::SpecialTasksComplete)
    );
    Ok(())
}

#[test]
fn grizzly_incidental_tiquanda_kill_preserves_source_fallthrough_to_demodras()
-> Result<(), GrizzlyRefusal> {
    let mut state = member()?;
    set(&mut state, POINTS_STORAGE, 70);
    apply(&mut state, 80, GrizzlyAction::StartSpecialTask)?;
    // Boss credit is unconditional: another encounter can credit a player who has not
    // entered their own lair. Crystal's unmatched Tiquanda branch then tries Demodras.
    apply(
        &mut state,
        80,
        GrizzlyAction::BossKill {
            boss_id: 29,
            observed_revision: 1,
        },
    )?;
    apply(&mut state, 80, GrizzlyAction::StartSpecialTask)?;
    assert_eq!(
        storage_value(&state, boss(29).ok_or(GrizzlyRefusal::InvalidBoss)?.storage),
        1
    );
    assert_eq!(
        storage_value(&state, boss(30).ok_or(GrizzlyRefusal::InvalidBoss)?.storage),
        1
    );
    apply(
        &mut state,
        80,
        GrizzlyAction::BossKill {
            boss_id: 30,
            observed_revision: 1,
        },
    )?;
    assert_eq!(
        plan_action(&state, actor(80), &GrizzlyAction::StartSpecialTask),
        Err(GrizzlyRefusal::BossNotKilled)
    );
    Ok(())
}

#[test]
fn grizzly_all_task_transitions_fit_existing_eight_effect_receipt_bound()
-> Result<(), GrizzlyRefusal> {
    for task in &catalogue()?.tasks {
        let mut state = member()?;
        set(&mut state, POINTS_STORAGE, 100);
        let start = apply(
            &mut state,
            task.minimum_level,
            GrizzlyAction::StartTask(task.id),
        )?;
        assert!(start.changes.len() <= crate::quest::QUESTSTATE0_RL_02);
        set(
            &mut state,
            KILL_COUNTER_BASE + u32::from(task.id),
            task.required,
        );
        let complete = apply(
            &mut state,
            task.minimum_level,
            GrizzlyAction::CompleteTask(task.id),
        )?;
        assert!(complete.changes.len() <= crate::quest::QUESTSTATE0_RL_02);
        let unique: std::collections::BTreeSet<_> = complete
            .changes
            .iter()
            .map(|change| &change.track)
            .collect();
        assert_eq!(unique.len(), complete.changes.len());
    }
    Ok(())
}

#[test]
fn grizzly_storage_overflow_refuses_the_entire_transition() -> Result<(), GrizzlyRefusal> {
    let mut state = member()?;
    apply(&mut state, 6, GrizzlyAction::StartTask(1))?;
    set(&mut state, KILL_COUNTER_BASE + 1, i64::from(i32::MAX));
    let original = state.clone();
    assert_eq!(
        plan_action(
            &state,
            actor(6),
            &GrizzlyAction::KillCreature {
                task_id: 1,
                creature: "crocodile".into(),
                observed_revision: 1
            }
        ),
        Err(GrizzlyRefusal::OutOfRange)
    );
    assert_eq!(state, original);
    Ok(())
}

#[test]
fn grizzly_receipt_binding_distinguishes_creature_and_captured_task_run() {
    let a = GrizzlyAction::KillCreature {
        task_id: 9,
        creature: "kongra".into(),
        observed_revision: 1,
    };
    let b = GrizzlyAction::KillCreature {
        task_id: 9,
        creature: "sibang".into(),
        observed_revision: 1,
    };
    assert_eq!(a.transition_key(), b.transition_key());
    assert_ne!(a.binding_bytes(), b.binding_bytes());
    let restarted = GrizzlyAction::KillCreature {
        task_id: 9,
        creature: "kongra".into(),
        observed_revision: 2,
    };
    assert_eq!(a.transition_key(), restarted.transition_key());
    assert_ne!(a.binding_bytes(), restarted.binding_bytes());
    let boss = GrizzlyAction::BossKill {
        boss_id: 1,
        observed_revision: 7,
    };
    let after_reset = GrizzlyAction::BossKill {
        boss_id: 1,
        observed_revision: 8,
    };
    assert_eq!(boss.transition_key(), after_reset.transition_key());
    assert_ne!(
        boss.binding_bytes(),
        after_reset.binding_bytes(),
        "a retried death cannot substitute a capture from after the NPC counter reset"
    );
    assert_ne!(
        GrizzlyAction::CompleteTask(1).transition_key(),
        GrizzlyAction::CompleteTask(2).transition_key()
    );
}

#[test]
fn grizzly_native_catalogue_keeps_old_keys_and_refuses_contextless_actions() -> Result<(), String> {
    let lowered = crate::quest::loader::load_embedded_quest_state("grizzly-test")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = lowered.catalogue();
    let points = storage_track(BOSS_POINTS_STORAGE);
    assert_eq!(
        points,
        "oteryn:quest-progress/quest/u8_5/killing_in_the_name_of/boss_points"
    );
    assert_eq!(
        catalogue.track(&points).ok_or("missing points")?.initial,
        -1
    );
    let first = catalogue
        .definition_hash(QUEST)
        .ok_or("missing quest hash")?;
    let extended =
        crate::quest::task_board::extend(catalogue).map_err(|error| format!("{error:?}"))?;
    assert_eq!(extended.definition_hash(QUEST), Some(first));
    assert!(
        catalogue
            .transition(&GrizzlyAction::StartTask(1).transition_key())
            .ok_or("missing action")?
            .effects
            .iter()
            .all(|effect| effect.effect == crate::quest::QuestEffectKind::Computed)
    );
    Ok(())
}

#[test]
fn grizzly_real_quest_log_shows_live_counts() -> Result<(), String> {
    use crate::quest::log::{QuestLogFacts, QuestLogState};
    struct Facts(GrizzlyState);
    impl QuestLogFacts for Facts {
        fn track_value(&self, key: &str) -> Option<i64> {
            self.0.get(key).copied()
        }
        fn quest_state(&self, _: &str) -> Option<QuestLogState<'_>> {
            None
        }
    }
    let lowered = crate::quest::loader::load_embedded_quest_state("grizzly-test")
        .map_err(|error| format!("{error:?}"))?;
    let loaded = crate::quest::log::loader::load_embedded_quest_log(&lowered)?;
    let index = loaded
        .content
        .active()
        .quests()
        .iter()
        .position(|quest| quest.key == QUEST)
        .ok_or("missing journal")?;
    let quest = &loaded.content.active().quests()[index];
    assert_eq!(quest.start, Some((storage_track(QUEST_LOG_STORAGE), 0)));
    assert_eq!(quest.missions.len(), 87);
    assert!(!loaded.exclusions.iter().any(|entry| entry.quest == QUEST));
    let index = u32::try_from(index).map_err(|error| error.to_string())?;
    assert!(
        loaded
            .content
            .line(&Facts(GrizzlyState::new()), index)
            .is_none()
    );
    let mut state = member().map_err(|error| format!("{error:?}"))?;
    apply(&mut state, 6, GrizzlyAction::StartTask(9)).map_err(|error| format!("{error:?}"))?;
    apply(
        &mut state,
        6,
        GrizzlyAction::KillCreature {
            task_id: 9,
            creature: "kongra".into(),
            observed_revision: 1,
        },
    )
    .map_err(|error| format!("{error:?}"))?;
    use crate::quest::npc_rules::{NpcRuleCatalog, NpcRuleInput, NpcRuleTrigger};
    let catalogue = lowered.catalogue();
    let npc = "oteryn:npc.lugri";
    state.insert(
        "oteryn:quest-progress/hunting-tasks/necromancers/phase".into(),
        3,
    );
    state.insert(
        "oteryn:quest-progress/hunting-tasks/minotaurs/phase".into(),
        3,
    );
    for (task, phase, repeat_count, expected, done) in [
        (
            "necromancers",
            2,
            0,
            "finding and fighting Necropharus",
            false,
        ),
        ("necromancers", 3, 0, "You faced Necropharus", false),
        ("necromancers", 3, 1000, "You've finished this task", true),
        ("minotaurs", 2, 0, "hideout of The Horned Fox", false),
    ] {
        let mut scenario = state.clone();
        let track = format!("oteryn:quest-progress/hunting-tasks/{task}/phase");
        scenario.insert(track.clone(), phase);
        scenario.insert(
            "oteryn:quest-progress/hunting-tasks/necromancers-repeat/count".into(),
            repeat_count,
        );
        let slot = quest
            .missions
            .iter()
            .position(|m| m.track == track)
            .ok_or("declared hunt")?;
        let line = loaded
            .content
            .line(&Facts(scenario), index)
            .ok_or("listed hunt")?;
        let mission = line
            .missions
            .iter()
            .find(|m| m.index as usize == slot)
            .ok_or("projected hunt")?;
        assert!(mission.text.contains(expected));
        assert_eq!(mission.done, done);
    }
    let programme = NpcRuleCatalog::runtime()
        .map_err(|e| e.to_string())?
        .admitted_for_npc(npc, catalogue)
        .map_err(|e| e.to_string())?
        .ok_or("Lugri programme")?;
    let request = NpcRuleInput {
        npc,
        trigger: NpcRuleTrigger::Say,
        message: "task",
        topic: 0,
        player_name: None,
        tracks: &state,
        inventory: &Default::default(),
        now_epoch_seconds: 1_700_000_000,
        calendar_month: 11,
        premium_current: true,
        level: 60,
        vocation: "oteryn:vocation.knight",
        sex: 0,
        outfits: &Default::default(),
    };
    let quote = programme
        .plan_storage(&request)
        .map_err(|e| e.to_string())?
        .ok_or("repeat offer")?;
    let accept = programme
        .plan_storage(&NpcRuleInput {
            message: "yes",
            topic: quote.next_topic,
            ..request
        })
        .map_err(|e| e.to_string())?
        .ok_or("repeat acceptance")?;
    state.extend(
        accept
            .track_changes
            .into_iter()
            .map(|change| (change.key, change.after)),
    );
    let death =
        "oteryn:quest-transition/native-kills/hunting-tasks/necromancers-repeat/necromancer";
    assert!(
        crate::quest::native_kills::active_for(catalogue, &state)
            .iter()
            .any(|key| key == death)
    );
    let changes = catalogue
        .evaluate(catalogue.transition(death).ok_or("repeat kill")?, &state, 0)
        .map_err(|e| format!("{e:?}"))?;
    state.extend(
        changes
            .into_iter()
            .map(|change| (change.track, change.after)),
    );
    let line = loaded
        .content
        .line(&Facts(state), index)
        .ok_or("missing listed quest")?;
    let mission = line
        .missions
        .iter()
        .find(|mission| mission.name == "Paw and Fur: Apes")
        .ok_or("missing Apes")?;
    assert!(mission.text.contains("1/300"));
    assert!(mission.text.contains("kongra: 1"));
    for (task, expected, done) in [
        (
            "necromancers",
            "Hunted 1/1000. Report to Lugri after the hunt.",
            false,
        ),
        (
            "minotaurs",
            "You have slain 5000 minotaurs and fought The Horned Fox for Budrik and the whole dwarven kind.",
            true,
        ),
    ] {
        let track = format!("oteryn:quest-progress/hunting-tasks/{task}/phase");
        let slot = quest
            .missions
            .iter()
            .position(|mission| mission.track == track)
            .ok_or("declared mission")?;
        let projected = line
            .missions
            .iter()
            .find(|mission| mission.index as usize == slot)
            .ok_or("missing mission")?;
        assert_eq!((projected.text.as_str(), projected.done), (expected, done));
    }
    assert!(!mission.done);
    Ok(())
}

#[test]
fn regional_boss_handoffs_require_quota_and_consume_one_entry() -> Result<(), String> {
    use crate::quest::reward_callbacks::{load_for, offered_for};
    let loaded = crate::quest::loader::load_embedded_quest_state("regional-boss-partial")
        .map_err(|error| format!("{error:?}"))?;
    let historical = load_for(loaded.catalogue())?;
    let offered = offered_for(loaded.catalogue())?;
    for task in ["blue-djinns", "green-djinns", "minotaurs", "necromancers"] {
        let key = format!("oteryn:quest-reward/hunting-tasks/{task}/report");
        assert!(
            historical.iter().any(|callback| callback.key == key),
            "{key}"
        );
        let report = offered.iter().find(|cb| cb.key == key).ok_or(key)?;
        let world = crate::quest::world_callbacks::load()?;
        let entry = world
            .iter()
            .find(|cb| cb.key == format!("oteryn:quest-callback/hunting-tasks/{task}/boss-entry"))
            .ok_or(task)?;
        assert_eq!(entry.storage[0].track, report.storage[0].track);
        assert_eq!(entry.storage[0].minimum, report.storage[0].set);
        assert_eq!(entry.storage[0].maximum, report.storage[0].set);
        assert_ne!(entry.storage[0].set, report.storage[0].set);
        assert!(entry.personal_room_entry().is_some());
        if task.ends_with("djinns") {
            assert_eq!(report.experience, Some(10_000));
            let coins = report.give.as_ref().ok_or("djinn report coins")?;
            assert_eq!((&*coins.item, coins.count), ("oteryn:item.tibia.i3035", 50));
        }
    }
    let pirate_report = offered
        .iter()
        .find(|callback| callback.key == "oteryn:quest-reward/hunting-tasks/pirates/report")
        .ok_or("Pirate quota pays before the boss stage")?;
    assert!(pirate_report.give.is_some());
    assert!(pirate_report.experience.is_some());
    let programme = crate::quest::npc_rules::NpcRuleCatalog::runtime()
        .map_err(|error| error.to_string())?
        .admitted_for_npc(&pirate_report.npc, loaded.catalogue())
        .map_err(|error| error.to_string())?
        .ok_or("Pirate hunting programme")?;
    let mut tracks: BTreeMap<_, _> = pirate_report
        .requires
        .iter()
        .map(|requirement| (requirement.track.clone(), requirement.minimum))
        .collect();
    tracks.extend(
        pirate_report
            .storage
            .iter()
            .map(|storage| (storage.track.clone(), storage.set)),
    );
    let input = crate::quest::npc_rules::NpcRuleInput {
        npc: &pirate_report.npc,
        trigger: crate::quest::npc_rules::NpcRuleTrigger::Say,
        message: "task",
        player_name: None,
        topic: 0,
        tracks: &tracks,
        inventory: &Default::default(),
        now_epoch_seconds: 1_700_000_000,
        calendar_month: 11,
        premium_current: true,
        level: 60,
        vocation: "oteryn:vocation.knight",
        sex: 0,
        outfits: &BTreeMap::new(),
    };
    assert!(
        programme
            .plan_storage(&input)
            .map_err(|error| error.to_string())?
            .is_none(),
        "earned boss access must be consumed before a new bounty"
    );
    let encounters = crate::quest::world_callbacks::load()?;
    let entry = encounters
        .iter()
        .find(|c| c.key.ends_with("pirates/boss-entry"))
        .ok_or("pirate entry")?;
    assert_eq!(
        (
            entry.storage[1].minimum,
            entry.storage[1].maximum,
            entry.storage[1].increment
        ),
        (0, 2, true)
    );
    let boss = &entry
        .effects
        .encounter
        .as_ref()
        .ok_or("pirate room")?
        .monsters[0];
    assert_eq!(boss.alternatives.len(), 3);
    let repeat = offered
        .iter()
        .find(|c| c.key.ends_with("pirates/repeat-report"))
        .ok_or("repeat pirates")?;
    assert_eq!(repeat.storage[0].set, 0);
    assert!(repeat.per_occurrence_mint);
    let repeat = offered
        .iter()
        .find(|c| c.key.ends_with("necromancers-repeat/report"))
        .ok_or("repeat necromancers")?;
    assert_eq!(
        (repeat.storage[1].minimum, repeat.experience),
        (1000, Some(40_000))
    );
    Ok(())
}

#[test]
fn non_paw_and_fur_tasks_load_with_native_counters_and_journals() -> Result<(), String> {
    let lowered = crate::quest::loader::load_embedded_quest_state("killing-in-the-name-test")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = lowered.catalogue();
    let log = crate::quest::log::loader::load_embedded_quest_log(&lowered)?;
    let journal = log
        .content
        .active()
        .quests()
        .iter()
        .find(|quest| quest.key == QUEST)
        .ok_or("missing Killing in the Name Of journal")?;
    assert_eq!(journal.missions.len(), 87);
    // Validate retained report transitions as well as live offers. The partial boss
    // handoffs have a separate admission test.
    let callbacks = crate::quest::reward_callbacks::load_for(catalogue)?;
    let rules =
        crate::quest::npc_rules::NpcRuleCatalog::runtime().map_err(|error| error.to_string())?;
    for (task, quota, creatures) in [
        ("trolls", 100, &["troll", "troll_champion"][..]),
        (
            "goblins",
            150,
            &["goblin", "goblin_scavenger", "goblin_assassin"],
        ),
        ("rotworms", 300, &["rotworm", "carrion_worm"]),
        (
            "cyclops",
            500,
            &["cyclops", "cyclops_drone", "cyclops_smith"],
        ),
        ("green-djinns", 500, &["green_djinn", "efreet"]),
        ("blue-djinns", 500, &["blue_djinn", "marid"]),
        (
            "pirates",
            3000,
            &[
                "pirate_marauder",
                "pirate_cutthroat",
                "pirate_buccaneer",
                "pirate_corsair",
            ],
        ),
        (
            "minotaurs",
            5000,
            &[
                "minotaur",
                "minotaur_guard",
                "minotaur_mage",
                "minotaur_archer",
            ],
        ),
        (
            "necromancers",
            4000,
            &[
                "necromancer",
                "priestess",
                "blood_priest",
                "blood_hand",
                "shadow_pupil",
            ],
        ),
    ] {
        let report_key = format!("oteryn:quest-reward/hunting-tasks/{task}/report");
        let report = callbacks
            .iter()
            .find(|c| c.key == report_key)
            .ok_or("report callback")?;
        let programme = rules
            .admitted_for_npc(&report.npc, catalogue)
            .map_err(|error| error.to_string())?
            .ok_or("hunting programme")?;
        let acceptance_key = format!("oteryn:quest-npc/hunting-tasks/{task}/accept");
        let acceptance = programme
            .programs()
            .iter()
            .flat_map(|program| &program.document.rules)
            .find(|rule| rule.key == acceptance_key)
            .ok_or("acceptance rule")?;
        let phase = format!("oteryn:quest-progress/hunting-tasks/{task}/phase");
        let count = format!("oteryn:quest-progress/hunting-tasks/{task}/count");
        assert!(
            journal
                .missions
                .iter()
                .any(|mission| mission.track == phase)
        );
        assert!(
            report
                .storage
                .iter()
                .any(|storage| storage.track == count && storage.minimum == quota)
        );
        let mut tracks = BTreeMap::from([
            (
                "oteryn:quest-progress/quest/u8_5/killing_in_the_name_of/quest_log_entry".into(),
                -1,
            ),
            (
                "oteryn:quest-progress/quest/u7_4/djinn_war/marid_faction/mission03".into(),
                3,
            ),
            (
                "oteryn:quest-progress/quest/u7_4/djinn_war/efreet_faction/mission03".into(),
                3,
            ),
            (
                "oteryn:quest-progress/quest/u7_8/the_shattered_isles/rays_mission4".into(),
                5,
            ),
            (
                "oteryn:quest-progress/quest/u8_1/to_outfox_afox_quest".into(),
                2,
            ),
        ]);
        if task == "goblins" {
            tracks.insert("oteryn:quest-progress/hunting-tasks/trolls/phase".into(), 2);
        }
        let level = match task {
            "trolls" | "goblins" => 6,
            "rotworms" => 20,
            "cyclops" => 40,
            "necromancers" => 60,
            _ => 10,
        };
        let input = crate::quest::npc_rules::NpcRuleInput {
            npc: &report.npc,
            trigger: crate::quest::npc_rules::NpcRuleTrigger::Say,
            message: "yes",
            player_name: None,
            topic: acceptance.topics.as_ref().ok_or("acceptance topic")?[0],
            tracks: &tracks,
            inventory: &Default::default(),
            now_epoch_seconds: 1_700_000_000,
            calendar_month: 11,
            premium_current: true,
            level,
            vocation: "oteryn:vocation.knight",
            sex: 0,
            outfits: &BTreeMap::new(),
        };
        // Crystal matches "task" within a phrase and Budrik uses the exact "mission" opener.
        let ask = if task == "minotaurs" {
            "mission"
        } else {
            "can I take the task?"
        };
        let quote = programme
            .plan_storage(&crate::quest::npc_rules::NpcRuleInput {
                message: ask,
                topic: 0,
                ..input
            })
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("{task} phrased offer refused"))?;
        assert_eq!(quote.next_topic, input.topic, "{task}");
        if task != "minotaurs" {
            for message in ["yes, about the task", "no, about the task"] {
                let repeated = programme
                    .plan_storage(&crate::quest::npc_rules::NpcRuleInput { message, ..input })
                    .map_err(|error| error.to_string())?
                    .ok_or_else(|| format!("{task} did not repeat the prompt for {message:?}"))?;
                assert_eq!(repeated.next_topic, input.topic, "{task}: {message}");
                assert!(repeated.track_changes.is_empty(), "{task}: {message}");
                assert_eq!(repeated.selected_rule_keys, quote.selected_rule_keys);
            }
        }
        let phrased_accept = programme
            .plan_storage(&crate::quest::npc_rules::NpcRuleInput {
                message: "yes please",
                ..input
            })
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("{task} phrased acceptance refused"))?;
        assert_eq!(
            phrased_accept.selected_rule_keys.as_slice(),
            std::slice::from_ref(&acceptance_key)
        );
        let report_request = if task == "minotaurs" {
            "mission"
        } else {
            "I finished the task"
        };
        assert!(report.matches_keyword(report_request), "{task}");
        let prefix = format!("oteryn:quest-transition/native-kills/hunting-tasks/{task}/");
        assert!(
            !crate::quest::native_kills::active_for(catalogue, &tracks)
                .iter()
                .any(|key| key.starts_with(&prefix))
        );
        let plan = programme
            .plan_storage(&input)
            .map_err(|error| error.to_string())?
            .ok_or("eligible task refused")?;
        assert_eq!(plan.selected_rule_keys, [acceptance_key]);
        assert!(
            plan.track_changes
                .iter()
                .any(|change| change.key == storage_track(JOIN_STORAGE) && change.after == 1)
        );
        assert!(
            plan.track_changes
                .iter()
                .any(|change| change.key == phase && change.after == 1)
        );
        tracks.extend(
            plan.track_changes
                .into_iter()
                .map(|change| (change.key, change.after)),
        );
        let active = crate::quest::native_kills::active_for(catalogue, &tracks);
        let report_transition = crate::quest::reward_callbacks::primary_transition(report)?;
        assert!(matches!(
            catalogue.evaluate(&report_transition, &tracks, 0),
            Err(crate::quest::QuestRefusal::StageMismatch)
        ));
        if report.npc == "oteryn:npc.daniel_steelsoul" {
            tracks.insert(count.clone(), 37);
            let progress = programme
                .plan_storage(&crate::quest::npc_rules::NpcRuleInput {
                    message: "task",
                    topic: 0,
                    tracks: &tracks,
                    inventory: &Default::default(),
                    outfits: &Default::default(),
                    npc: &report.npc,
                    trigger: crate::quest::npc_rules::NpcRuleTrigger::Say,
                    player_name: None,
                    now_epoch_seconds: 1_700_000_000,
                    calendar_month: 11,
                    premium_current: true,
                    level,
                    vocation: "oteryn:vocation.knight",
                    sex: 0,
                })
                .map_err(|error| error.to_string())?
                .ok_or("missing hunt progress")?;
            assert!(
                progress
                    .reply
                    .iter()
                    .any(|line| line.contains("killed 37 of them"))
            );
        }
        tracks.insert(count.clone(), quota - 1);
        for creature in creatures {
            let key =
                format!("oteryn:quest-transition/native-kills/hunting-tasks/{task}/{creature}");
            assert!(active.contains(&key), "inactive counter {key}");
            let counter = crate::quest::native_kills::counter_for(catalogue, &key).ok_or(key)?;
            assert_eq!(counter.creature, format!("oteryn:creature.{creature}"));
            assert_eq!(counter.maximum, quota);
            let death = catalogue
                .transition(counter.transition)
                .ok_or("death transition")?;
            let changes = catalogue
                .evaluate(death, &tracks, 0)
                .map_err(|refusal| format!("{refusal:?}"))?;
            assert!(
                changes
                    .iter()
                    .any(|change| change.track == count && change.after == quota)
            );
        }
        tracks.insert(count, quota);
        let reported = catalogue
            .evaluate(&report_transition, &tracks, 0)
            .map_err(|refusal| format!("{refusal:?}"))?;
        assert!(
            reported
                .iter()
                .any(|change| change.track == phase && change.after == 2)
        );
        tracks.insert(phase, 2);
        assert!(matches!(
            catalogue.evaluate(&report_transition, &tracks, 0),
            Err(crate::quest::QuestRefusal::StageMismatch)
        ));
    }
    Ok(())
}

#[test]
fn wrath_mission_five_loads_admission_handoff_and_city_routes() -> Result<(), String> {
    let lowered = crate::quest::loader::load_embedded_quest_state("wrath-city-test")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = lowered.catalogue();
    let callbacks = crate::quest::reward_callbacks::offered_for(catalogue)?;
    for action in [
        "briefing",
        "supplies-accept",
        "supplies-report",
        "zlak-handoff",
    ] {
        assert!(
            callbacks
                .iter()
                .any(|c| c.key == format!("oteryn:quest-reward/wrath-city/{action}"))
        );
    }
    let report = callbacks
        .iter()
        .find(|c| c.key.ends_with("/wrath-city/supplies-report"))
        .ok_or("report")?;
    assert!(report.topic > 0 && report.matches_keyword("yes"));
    assert!(!report.matches_keyword("supplies"));
    let rewards = (0..100)
        .map(|roll| {
            report
                .reward_for_percentile(roll)
                .map(|reward| (reward.item.as_str(), reward.count))
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        rewards,
        [3038, 3039, 3041, 10384]
            .map(|id| format!("oteryn:item.tibia.i{id}"))
            .iter()
            .map(|key| Some((key.as_str(), 1)))
            .collect()
    );
    let briefing = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-reward/wrath-city/briefing")
        .ok_or("briefing")?;
    assert!(briefing.requires.iter().any(|guard| guard.track
        == "oteryn:quest-progress/wrath-city/supplies"
        && guard.minimum == 2
        && guard.maximum == 2));
    assert!(briefing.matches_keyword("tell me about the mission"));
    let handoff = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-reward/wrath-city/zlak-handoff")
        .ok_or("handoff")?;
    assert!(handoff.matches_keyword("tell me about the mission"));
    let gates = crate::quest::npc_rules::NpcRuleCatalog::runtime()
        .and_then(|rules| rules.admitted_for_npc("oteryn:npc.gate_guardian", catalogue))
        .map_err(|error| error.to_string())?
        .ok_or("Gate Guardian programme")?;
    let questline = "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/questline";
    let mission = "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/mission05";
    for (stage, step, can_enter) in [(12, 0, false), (13, 1, true), (22, 2, false)] {
        let facts = BTreeMap::from([(questline.into(), stage), (mission.into(), step)]);
        let input = crate::quest::npc_rules::NpcRuleInput {
            npc: "oteryn:npc.gate_guardian",
            trigger: crate::quest::npc_rules::NpcRuleTrigger::Say,
            message: "yes",
            player_name: None,
            topic: 1,
            tracks: &facts,
            inventory: &Default::default(),
            now_epoch_seconds: 1_700_000_000,
            calendar_month: 11,
            premium_current: true,
            level: 80,
            vocation: "oteryn:vocation.knight",
            sex: 0,
            outfits: &BTreeMap::new(),
        };
        let offer = gates
            .plan_storage(&crate::quest::npc_rules::NpcRuleInput {
                message: "can I have a pass?",
                topic: 0,
                ..input
            })
            .map_err(|error| error.to_string())?;
        assert_eq!(offer.map(|plan| plan.next_topic), can_enter.then_some(1));
        for message in ["yes, I need a pass", "yes, about the mission"] {
            let combined = crate::quest::npc_rules::NpcRuleInput { message, ..input };
            assert!(
                gates
                    .plan_storage_teleport(&combined)
                    .map_err(|error| error.to_string())?
                    .is_none(),
                "prompt keyword should win at stage {stage}: {message}"
            );
            let repeated = gates
                .plan_storage(&combined)
                .map_err(|error| error.to_string())?;
            assert_eq!(repeated.map(|plan| plan.next_topic), can_enter.then_some(1));
        }
        let confirmed = gates
            .plan_storage_teleport(&crate::quest::npc_rules::NpcRuleInput {
                message: "yes please",
                ..input
            })
            .map_err(|error| error.to_string())?;
        assert_eq!(confirmed.is_some(), can_enter);
        let entry = gates
            .plan_storage_teleport(&input)
            .map_err(|error| error.to_string())?;
        assert_eq!(entry.is_some(), can_enter);
        if let Some(entry) = entry {
            assert_eq!(
                (
                    entry.destination.x,
                    entry.destination.y,
                    entry.destination.floor
                ),
                (33114, 31197, -7)
            );
            assert!(
                entry
                    .source
                    .track_changes
                    .iter()
                    .any(|change| change.key == questline && change.after == 22)
            );
        }
    }
    let world = crate::quest::world_callbacks::load_for(catalogue)?;
    for action in [
        "supply-crate",
        "supply-fish",
        "feed-prisoners",
        "rebel-to-city",
        "city-to-rebel-north",
        "city-to-rebel-south",
    ] {
        let key = format!("oteryn:quest-callback/authored-wrath-city-{action}");
        let callback = world.iter().find(|c| c.key == key).ok_or(key)?;
        assert!(callback.selector.native.is_some());
        if callback.teleport.is_some() {
            assert!(callback.is_read_only());
            assert!(callback.return_on_failure);
        }
    }
    Ok(())
}

#[test]
fn chartan_reopens_supplies_after_the_daily_cooldown() -> Result<(), String> {
    use crate::quest::reward_callbacks::{offered_for, primary_transition};

    let loaded = crate::quest::loader::load_embedded_quest_state("chartan-supplies")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let callbacks = offered_for(catalogue)?;
    let chartan = callbacks
        .iter()
        .find(|row| row.key.ends_with("/wrath-city/chartan-supplies-repeat"))
        .ok_or("Chartan's repeat offer")?;
    assert_eq!(chartan.npc, "oteryn:npc.chartan");
    assert!(chartan.requires.iter().any(|guard| {
        guard.track.ends_with("/wrath_of_the_emperor/questline")
            && guard.minimum == 34
            && guard.maximum == 34
    }));
    let report = callbacks
        .iter()
        .find(|row| row.key.ends_with("/wrath-city/supplies-report"))
        .ok_or("Zalamon's report")?;
    let key = |name: &str| format!("oteryn:quest-progress/wrath-city/{name}");
    let mut stored = BTreeMap::from([
        (key("crate"), 1),
        (key("fish"), 1),
        (key("prisoners"), 1),
        (key("supplies"), 1),
    ]);
    let now = 1_700_000_000;
    let turn_in = catalogue
        .evaluate(&primary_transition(report)?, &stored, now)
        .map_err(|error| format!("{error:?}"))?;
    stored.extend(
        turn_in
            .into_iter()
            .map(|change| (change.track, change.after)),
    );
    assert_eq!(stored.get(&key("completed")), Some(&1));
    assert_eq!(stored.get(&key("last-report")), Some(&now));
    let repeat = primary_transition(chartan)?;
    assert!(matches!(
        catalogue.evaluate(&repeat, &stored, now + 86399),
        Err(crate::quest::QuestRefusal::StageMismatch)
    ));
    let accepted = catalogue
        .evaluate(&repeat, &stored, now + 86400)
        .map_err(|error| format!("{error:?}"))?;
    stored.extend(
        accepted
            .into_iter()
            .map(|change| (change.track, change.after)),
    );
    for objective in ["crate", "fish", "prisoners"] {
        assert_eq!(stored.get(&key(objective)), Some(&0), "{objective}");
    }
    assert_eq!(stored.get(&key("supplies")), Some(&1));
    assert_eq!(stored.get(&key("completed")), Some(&1));
    Ok(())
}

#[test]
fn daniel_repeats_a_reported_starter_hunt_only_from_the_choice_prompt() -> Result<(), String> {
    use crate::quest::npc_rules::{NpcRuleCatalog, NpcRuleInput, NpcRuleTrigger};
    let loaded = crate::quest::loader::load_embedded_quest_state("daniel-repeat")
        .map_err(|error| format!("{error:?}"))?;
    let npc = "oteryn:npc.daniel_steelsoul";
    let programme = NpcRuleCatalog::runtime()
        .and_then(|rules| rules.admitted_for_npc(npc, loaded.catalogue()))
        .map_err(|error| error.to_string())?
        .ok_or("Daniel hunting programme")?;
    let phase = |task| format!("oteryn:quest-progress/hunting-tasks/{task}/phase");
    let count = |task| format!("oteryn:quest-progress/hunting-tasks/{task}/count");
    let tracks = BTreeMap::from([
        (phase("trolls"), 2),
        (phase("goblins"), 2),
        (count("trolls"), 100),
        (count("goblins"), 150),
    ]);
    let input = NpcRuleInput {
        npc,
        trigger: NpcRuleTrigger::Say,
        message: "task",
        player_name: None,
        topic: 0,
        tracks: &tracks,
        inventory: &Default::default(),
        now_epoch_seconds: 1_700_000_000,
        calendar_month: 11,
        premium_current: true,
        level: 19,
        vocation: "oteryn:vocation.knight",
        sex: 0,
        outfits: &BTreeMap::new(),
    };
    let choice = programme
        .plan_storage(&input)
        .map_err(|error| error.to_string())?
        .ok_or("repeat choice")?;
    assert!(choice.track_changes.is_empty());
    for task in ["trolls", "goblins"] {
        assert!(
            choice
                .reply
                .iter()
                .any(|line| line.contains(&format!("{{{task}}}")))
        );
        assert!(
            programme
                .plan_storage(&NpcRuleInput {
                    message: task,
                    ..input
                })
                .map_err(|error| error.to_string())?
                .is_none()
        );
        let start = programme
            .plan_storage(&NpcRuleInput {
                message: task,
                topic: choice.next_topic,
                ..input
            })
            .map_err(|error| error.to_string())?
            .ok_or("repeat start")?;
        assert_eq!(start.next_topic, 0);
        assert!(
            start
                .track_changes
                .iter()
                .any(|change| change.key == phase(task) && change.after == 1)
        );
        assert!(
            start
                .track_changes
                .iter()
                .any(|change| change.key == count(task) && change.after == 0)
        );
        assert!(start.track_changes.iter().all(|change| {
            !change
                .key
                .starts_with("oteryn:quest-progress/hunting-tasks/")
                || change.key == phase(task)
                || change.key == count(task)
        }));
    }
    assert!(
        programme
            .plan_storage(&NpcRuleInput {
                message: "yes",
                topic: choice.next_topic,
                ..input
            })
            .map_err(|error| error.to_string())?
            .is_none()
    );
    Ok(())
}

#[test]
fn regional_hunt_reports_and_zlak_require_the_donor_keywords() -> Result<(), String> {
    use crate::quest::reward_callbacks::{load_for, offered_for};
    let loaded = crate::quest::loader::load_embedded_quest_state("regional-report-keywords")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let callbacks = load_for(catalogue)?;
    let offered = offered_for(catalogue)?;
    let mut unexpected = Vec::new();
    for (hunt, creature) in [
        ("trolls", "trolls"),
        ("goblins", "goblins"),
        ("cyclops", "cyclops"),
        ("rotworms", "rotworms"),
        ("pirates", "pirates"),
        ("blue-djinns", "blue djinns"),
        ("green-djinns", "green djinns"),
        ("minotaurs", "minotaurs"),
        ("necromancers", "necromancers"),
    ] {
        let key = format!("oteryn:quest-reward/hunting-tasks/{hunt}/report");
        let callback = callbacks
            .iter()
            .find(|callback| callback.key == key)
            .ok_or(key)?;
        assert!(
            offered.iter().any(|entry| entry.key == callback.key),
            "{hunt}"
        );
        let positive = if hunt == "minotaurs" {
            "mission"
        } else {
            "I finished the task"
        };
        assert!(callback.matches_keyword(positive), "{hunt}");
        for text in [
            format!("the {creature} are nearby"),
            "I have a report".into(),
        ] {
            if callback.matches_keyword(&text) {
                unexpected.push((callback.key.as_str(), text));
            }
        }
    }
    let zlak = offered
        .iter()
        .find(|entry| entry.key == "oteryn:quest-reward/wrath-city/zlak-handoff")
        .ok_or("Zlak handoff")?;
    assert!(zlak.matches_keyword("tell me about the mission"));
    if zlak.matches_keyword("tell me about imperial service") {
        unexpected.push((zlak.key.as_str(), "tell me about imperial service".into()));
    }
    assert!(
        unexpected.is_empty(),
        "unexpected reward triggers: {unexpected:?}"
    );
    Ok(())
}

#[test]
fn daniel_declines_to_an_eligible_alternate_hunt() -> Result<(), String> {
    use crate::quest::npc_rules::{NpcRuleCatalog, NpcRuleInput, NpcRuleTrigger};
    let loaded = crate::quest::loader::load_embedded_quest_state("daniel-alternate")
        .map_err(|error| format!("{error:?}"))?;
    let npc = "oteryn:npc.daniel_steelsoul";
    let programme = NpcRuleCatalog::runtime()
        .and_then(|rules| rules.admitted_for_npc(npc, loaded.catalogue()))
        .map_err(|error| error.to_string())?
        .ok_or("Daniel hunting programme")?;
    let phase = |task| format!("oteryn:quest-progress/hunting-tasks/{task}/phase");
    for (level, trolls, offered, alternate) in [
        (19, 0, 1, Some(("goblins", 2))),
        (19, 2, 2, Some(("trolls", 1))),
        (29, 0, 4, None),
        (30, 0, 3, Some(("rotworms", 4))),
        (35, 0, 3, Some(("rotworms", 4))),
        (39, 0, 3, Some(("rotworms", 4))),
        (40, 0, 3, None),
        (59, 0, 3, None),
    ] {
        let tracks = BTreeMap::from([(phase("trolls"), trolls)]);
        let input = NpcRuleInput {
            npc,
            trigger: NpcRuleTrigger::Say,
            message: "task",
            player_name: None,
            topic: 0,
            tracks: &tracks,
            inventory: &Default::default(),
            now_epoch_seconds: 1_700_000_000,
            calendar_month: 11,
            premium_current: true,
            level,
            vocation: "oteryn:vocation.knight",
            sex: 0,
            outfits: &BTreeMap::new(),
        };
        let plan = |message, topic| {
            programme
                .plan_storage(&NpcRuleInput {
                    message,
                    topic,
                    ..input
                })
                .map_err(|error| error.to_string())
        };
        let ask = plan("task", 0)?.ok_or("missing hunt offer")?;
        assert!(ask.track_changes.is_empty());
        assert_eq!(ask.next_topic, offered);
        let decline = plan("no", ask.next_topic)?;
        if let Some((hunt, topic)) = alternate {
            let switch = decline.ok_or("missing eligible alternate")?;
            assert!(switch.track_changes.is_empty());
            assert_eq!(switch.next_topic, topic);
            if (30..40).contains(&level) {
                let back = plan("no", switch.next_topic)?.ok_or("missing cyclops alternate")?;
                assert_eq!(back.next_topic, ask.next_topic);
            }
            let acceptance = plan("yes", switch.next_topic)?.ok_or("missing acceptance")?;
            assert_eq!(acceptance.next_topic, 0);
            assert!(
                acceptance
                    .track_changes
                    .iter()
                    .any(|change| change.key == phase(hunt) && change.after == 1)
            );
        } else {
            assert!(
                decline.is_none_or(|plan| plan.next_topic == 0 && plan.track_changes.is_empty())
            );
        }
        for (topic, eligible) in [
            (2, level < 20),
            (3, level >= 30),
            (4, (20..40).contains(&level)),
        ] {
            assert_eq!(
                plan("yes", topic)?.is_some(),
                eligible,
                "level {level}, topic {topic}"
            );
        }
    }
    Ok(())
}

#[test]
fn regional_hunt_reports_yield_to_earlier_npc_keywords() -> Result<(), String> {
    use crate::quest::reward_callbacks::{load_for, offered_for};
    let loaded = crate::quest::loader::load_embedded_quest_state("hunt-mission-precedence")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let offered = offered_for(catalogue)?;
    let historical = load_for(catalogue)?;
    let mut intercepted = Vec::new();
    for (hunt, earlier, held) in [
        ("cyclops", &["mission"][..], false),
        ("goblins", &["mission"][..], false),
        ("rotworms", &["mission"][..], false),
        ("trolls", &["mission"][..], false),
        (
            "pirates",
            &["eleonore", "mission", "mermaid", "pirate outfit"][..],
            false,
        ),
        ("blue-djinns", &["mission"][..], true),
        ("green-djinns", &["mission"][..], true),
    ] {
        let key = format!("oteryn:quest-reward/hunting-tasks/{hunt}/report");
        let callbacks = if held { &historical } else { &offered };
        let callback = callbacks
            .iter()
            .find(|callback| callback.key == key)
            .ok_or(key)?;
        assert!(callback.matches_keyword("I finished the task"), "{hunt}");
        for keyword in earlier {
            if callback.matches_keyword(&format!("{keyword} task")) {
                intercepted.push((hunt, *keyword));
            }
        }
    }
    assert!(
        intercepted.is_empty(),
        "reports intercepted earlier branches: {intercepted:?}"
    );
    Ok(())
}

#[test]
fn review_batch_regional_hunts_follow_crystal_branch_order() -> Result<(), String> {
    use crate::quest::npc_rules::{NpcRuleCatalog, NpcRuleInput, NpcRuleTrigger};
    let loaded = crate::quest::loader::load_embedded_quest_state("regional-branch-order")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let callbacks = crate::quest::reward_callbacks::load_for(catalogue)?;
    let rules = NpcRuleCatalog::runtime().map_err(|error| error.to_string())?;
    let mut failures = Vec::new();
    for (task, alternate, level) in [
        ("trolls", "goblins", 19),
        ("goblins", "trolls", 19),
        ("rotworms", "cyclops", 40),
        ("cyclops", "rotworms", 30),
    ] {
        let npc = "oteryn:npc.daniel_steelsoul";
        let programme = rules
            .admitted_for_npc(npc, catalogue)
            .map_err(|error| error.to_string())?
            .ok_or("Daniel hunting programme")?;
        let phase = |hunt| format!("oteryn:quest-progress/hunting-tasks/{hunt}/phase");
        let topic = |hunt| {
            let key = format!("oteryn:quest-npc/hunting-tasks/{hunt}/accept");
            programme
                .programs()
                .iter()
                .flat_map(|program| &program.document.rules)
                .find(|rule| rule.key == key)
                .and_then(|rule| rule.topics.as_ref()?.first().copied())
                .ok_or(key)
        };
        let tracks = BTreeMap::from([(phase("trolls"), i64::from(level == 19) * 2)]);
        let input = NpcRuleInput {
            npc,
            trigger: NpcRuleTrigger::Say,
            message: "no",
            player_name: None,
            topic: topic(task)?,
            tracks: &tracks,
            inventory: &Default::default(),
            now_epoch_seconds: 1_700_000_000,
            calendar_month: 11,
            premium_current: true,
            level,
            vocation: "oteryn:vocation.knight",
            sex: 0,
            outfits: &BTreeMap::new(),
        };
        let decline = programme
            .plan_storage(&input)
            .map_err(|error| error.to_string())?;
        let alternate_topic = topic(alternate)?;
        if decline.as_ref().map(|plan| plan.next_topic) != Some(alternate_topic) {
            failures.push(format!("declining {task} did not offer {alternate}"));
        }
        let confirmation = format!("yes {task}");
        for message in ["yes", confirmation.as_str()] {
            let accept = programme
                .plan_storage(&NpcRuleInput {
                    message,
                    topic: alternate_topic,
                    ..input
                })
                .map_err(|error| error.to_string())?;
            if !accept.is_some_and(|plan| {
                plan.track_changes
                    .iter()
                    .any(|change| change.key == phase(alternate) && change.after == 1)
            }) {
                failures.push(format!(
                    "{message:?} did not accept alternate {alternate} after declining {task}"
                ));
            }
        }
    }
    for (npc, earlier) in [
        ("daniel_steelsoul", "mission"),
        ("raymond_striker", "mission"),
        ("raymond_striker", "eleonore"),
        ("raymond_striker", "mermaid"),
        ("raymond_striker", "pirate outfit"),
        ("malor", "mission"),
        ("gabel", "mission"),
        ("lugri", "outfit"),
    ] {
        let npc = format!("oteryn:npc.{npc}");
        let message = format!("I finished the {earlier} task");
        let matches = callbacks.iter().any(|callback| {
            callback.npc == npc
                && callback
                    .key
                    .starts_with("oteryn:quest-reward/hunting-tasks/")
                && callback.matches_keyword(&message)
        });
        if matches {
            failures.push(format!("{npc} Rewards consumed {message:?}"));
        }
    }
    assert!(
        failures.is_empty(),
        "Crystal branch mismatches: {failures:#?}"
    );
    Ok(())
}
