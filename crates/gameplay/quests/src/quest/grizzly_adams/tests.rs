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
fn grizzly_real_quest_log_shows_live_counts_and_preserves_other_npc_exclusions()
-> Result<(), String> {
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
    // The source has 87 missions: W15c admits all 78 Grizzly missions. The nine other
    // NPCs' journals still lack supported ordered template bindings in the existing loader.
    assert_eq!(quest.missions.len(), 78);
    let excluded: std::collections::BTreeSet<_> = loaded
        .exclusions
        .iter()
        .filter(|entry| entry.quest == QUEST)
        .filter_map(|entry| entry.mission.as_deref())
        .collect();
    assert_eq!(
        excluded,
        std::collections::BTreeSet::from([
            "the_marid_green_djinns",
            "the_efreet_blue_djinns",
            "meriana_rebels_pirates",
            "turmoil_of_war",
            "lugri_necromancers_and_priestesses",
            "edron_city_trolls",
            "edron_city_goblins",
            "edron_city_rotworms",
            "edron_city_cyclops",
        ])
    );
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
    assert!(!mission.done);
    Ok(())
}
