use super::*;

fn creatures() -> Vec<TaskBoardCreature> {
    (0..48)
        .map(|i| TaskBoardCreature {
            race: format!("oteryn:creature.taskboard_test_{i}"),
            bestiary_stars: (i % 6) as u8,
            experience: 100,
            preyable: true,
            prey_exclusive: false,
        })
        .collect()
}

fn context(creatures: &[TaskBoardCreature]) -> TaskBoardContext<'_> {
    TaskBoardContext {
        level: 300,
        has_vocation: true,
        now_unix_seconds: 1_800_000_000,
        period: 42,
        creatures,
        available_gold: 5_000,
        reroll_tokens: 0,
    }
}

fn apply(
    data: &mut TaskBoardData,
    balances: &mut TaskBoardBalances,
    action: TaskBoardAction,
    context: &TaskBoardContext<'_>,
) -> Result<TaskBoardEffects, TaskBoardRefusal> {
    apply_action(data, balances, &action, context, &mut |_| 0)
}

#[test]
fn difficulty_gates_counts_and_caps_match_crystal() -> Result<(), TaskBoardRefusal> {
    let catalogue = creatures();
    let mut ctx = context(&catalogue);
    for (difficulty, minimum, kills, any, cap) in [
        (0, 8, 50, 1000, 200000),
        (1, 30, 100, 2000, 800000),
        (2, 150, 200, 3000, 3000000),
        (3, 300, 300, 4000, 799999880000008),
    ] {
        let mut data = TaskBoardData::default();
        let mut balances = TaskBoardBalances::default();
        ctx.level = minimum - 1;
        assert_eq!(
            apply(
                &mut data,
                &mut balances,
                TaskBoardAction::StartWeekly { difficulty },
                &ctx
            ),
            Err(TaskBoardRefusal::LevelTooLow)
        );
        assert_eq!(data, TaskBoardData::default());
        ctx.level = minimum;
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::StartWeekly { difficulty },
            &ctx,
        )?;
        let board = data.weekly.as_ref().ok_or(TaskBoardRefusal::WrongState)?;
        assert_eq!(board.kills.len(), 5);
        assert_eq!(board.deliveries.len(), 6);
        assert_eq!(board.any_required, any);
        assert!(board.kills.iter().all(|task| task.required == kills));
        assert_eq!(weekly_experience(20_000_000, difficulty)?, cap);
        let races: BTreeSet<_> = board.kills.iter().map(|task| &task.race).collect();
        let items: BTreeSet<_> = board.deliveries.iter().map(|task| task.item_id).collect();
        assert_eq!(races.len(), 5);
        assert_eq!(items.len(), 6);
        for task in &board.kills {
            let creature = catalogue
                .iter()
                .find(|creature| creature.race == task.race)
                .ok_or(TaskBoardRefusal::MissingCatalogue)?;
            assert!(stars_match(difficulty, creature.bestiary_stars, false));
        }
        assert_eq!(
            apply(
                &mut data,
                &mut balances,
                TaskBoardAction::StartWeekly { difficulty },
                &ctx
            ),
            Err(TaskBoardRefusal::WrongState)
        );
    }
    assert_eq!(minimum_level(4), None);
    assert_eq!(weekly_experience(8, 4), Err(TaskBoardRefusal::InvalidInput));
    Ok(())
}

#[test]
fn weekly_experience_uses_current_level_and_exact_rounding() -> Result<(), TaskBoardRefusal> {
    for (level, amount) in [
        (1, 50),
        (8, 1100),
        (10, 1850),
        (82, 162050),
        (83, 165503),
        (999, 1992014),
        (1000, 1994008),
        (2000, 7988008),
    ] {
        assert_eq!(weekly_experience(level, 3)?, amount, "level {level}");
    }
    assert_eq!(weekly_experience(1000, 0)?, 200_000);
    assert_eq!(weekly_experience(1000, 1)?, 800_000);
    assert_eq!(weekly_experience(2000, 2)?, 3_000_000);
    assert_eq!(weekly_experience(0, 0), Err(TaskBoardRefusal::InvalidInput));
    assert_eq!(
        weekly_experience(u32::MAX, 3),
        Err(TaskBoardRefusal::ArithmeticOverflow)
    );
    Ok(())
}

#[test]
fn completion_multiplier_uses_executable_thirteen_task_rule() -> Result<(), TaskBoardRefusal> {
    for (count, multiplier) in [
        (0, 1),
        (4, 1),
        (5, 2),
        (8, 2),
        (9, 3),
        (12, 3),
        (13, 5),
        (16, 5),
        (17, 8),
        (18, 8),
    ] {
        assert_eq!(completion_multiplier(count), multiplier);
    }
    let board = WeeklyBoard {
        period: 1,
        difficulty: 3,
        any_required: 4000,
        any_kills: 4000,
        kills: (0..8)
            .map(|i| WeeklyKillTask {
                race: format!("race{i}"),
                required: 300,
                current: 300,
            })
            .collect(),
        deliveries: (0..9)
            .map(|_| WeeklyDeliveryTask {
                item_id: 811,
                required: 20,
                delivered: true,
            })
            .collect(),
        finished: false,
    };
    assert_eq!(board.rewards()?, (13_320, 18));
    Ok(())
}

#[test]
fn all_weekly_tasks_progress_together_and_xp_is_immediate_once() -> Result<(), TaskBoardRefusal> {
    let catalogue = creatures();
    let mut ctx = context(&catalogue);
    ctx.level = 10;
    let mut data = TaskBoardData::default();
    let mut balances = TaskBoardBalances::default();
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::StartWeekly { difficulty: 0 },
        &ctx,
    )?;
    let board = data.weekly.as_mut().ok_or(TaskBoardRefusal::WrongState)?;
    board.any_kills = board.any_required - 1;
    board.kills[0].current = board.kills[0].required - 1;
    let race = board.kills[0].race.clone();
    let effects = apply(
        &mut data,
        &mut balances,
        TaskBoardAction::CreditKill { race: race.clone() },
        &ctx,
    )?;
    assert_eq!(effects.experience, 3_700);
    assert_eq!(
        effects.weekly_completions, 2,
        "the progression owner must plan any, then specific, using the level after each award"
    );
    assert_eq!((balances.points, balances.soulseals), (0, 0));
    let repeated = apply(
        &mut data,
        &mut balances,
        TaskBoardAction::CreditKill { race },
        &ctx,
    )?;
    assert_eq!((repeated.experience, repeated.weekly_completions), (0, 0));
    ctx.level = 83;
    let board = data.weekly.as_mut().ok_or(TaskBoardRefusal::WrongState)?;
    board.kills[1].current = board.kills[1].required - 1;
    let race = board.kills[1].race.clone();
    let specific = apply(
        &mut data,
        &mut balances,
        TaskBoardAction::CreditKill { race },
        &ctx,
    )?;
    assert_eq!(
        (specific.experience, specific.weekly_completions),
        (165_503, 1)
    );
    let board = data.weekly.as_ref().ok_or(TaskBoardRefusal::WrongState)?;
    assert_eq!(board.completed_kills(), 3);
    Ok(())
}

#[test]
fn delivery_requests_exact_custody_debit_and_cannot_repeat() -> Result<(), TaskBoardRefusal> {
    let catalogue = creatures();
    let ctx = context(&catalogue);
    let mut data = TaskBoardData::default();
    let mut balances = TaskBoardBalances::default();
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::Deliver { slot: 0 },
            &ctx
        ),
        Err(TaskBoardRefusal::WrongState)
    );
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::StartWeekly { difficulty: 0 },
        &ctx,
    )?;
    let task = data
        .weekly
        .as_ref()
        .ok_or(TaskBoardRefusal::WrongState)?
        .deliveries[0]
        .clone();
    let effects = apply(
        &mut data,
        &mut balances,
        TaskBoardAction::Deliver { slot: 0 },
        &ctx,
    )?;
    assert_eq!(
        effects.consume_items,
        vec![TaskBoardItemQuantity {
            item_id: task.item_id,
            amount: task.required
        }]
    );
    assert_eq!(effects.experience, 200_000);
    assert_eq!(effects.weekly_completions, 1);
    assert_eq!(balances.points, 0);
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::Deliver { slot: 0 },
            &ctx
        ),
        Err(TaskBoardRefusal::WrongState)
    );
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::Deliver { slot: 9 },
            &ctx
        ),
        Err(TaskBoardRefusal::InvalidInput)
    );
    Ok(())
}

#[test]
fn only_server_save_period_settles_points_and_soulseals_once() -> Result<(), TaskBoardRefusal> {
    let catalogue = creatures();
    let mut ctx = context(&catalogue);
    let mut data = TaskBoardData::default();
    let mut balances = TaskBoardBalances::default();
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::StartWeekly { difficulty: 0 },
        &ctx,
    )?;
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::Deliver { slot: 0 },
        &ctx,
    )?;
    ctx.now_unix_seconds += 100 * 86_400;
    apply(&mut data, &mut balances, TaskBoardAction::Query, &ctx)?;
    assert_eq!(
        (balances.points, balances.soulseals),
        (0, 0),
        "node/database wall-time is not the reset authority"
    );
    ctx.period += 1;
    apply(&mut data, &mut balances, TaskBoardAction::Query, &ctx)?;
    assert_eq!((balances.points, balances.soulseals), (75, 1));
    apply(&mut data, &mut balances, TaskBoardAction::Query, &ctx)?;
    assert_eq!((balances.points, balances.soulseals), (75, 1));
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::Deliver { slot: 1 },
            &ctx
        ),
        Err(TaskBoardRefusal::WrongState)
    );
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::StartWeekly { difficulty: 1 },
        &ctx,
    )?;
    assert_eq!(
        data.weekly
            .as_ref()
            .ok_or(TaskBoardRefusal::WrongState)?
            .period,
        ctx.period
    );
    ctx.period -= 1;
    assert_eq!(
        apply(&mut data, &mut balances, TaskBoardAction::Query, &ctx),
        Err(TaskBoardRefusal::WrongState)
    );
    Ok(())
}

#[test]
fn second_weekly_start_preserves_progress_and_accrued_rewards_until_reset()
-> Result<(), TaskBoardRefusal> {
    let catalogue = creatures();
    let mut ctx = context(&catalogue);
    let mut data = TaskBoardData::default();
    let mut balances = TaskBoardBalances::default();
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::StartWeekly { difficulty: 0 },
        &ctx,
    )?;
    let board = data.weekly.as_mut().ok_or(TaskBoardRefusal::WrongState)?;
    board.any_kills = 17;
    board.kills[0].current = board.kills[0].required - 1;
    let race = board.kills[0].race.clone();
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::CreditKill { race },
            &ctx
        )?
        .weekly_completions,
        1
    );
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::Deliver { slot: 0 },
        &ctx,
    )?;
    let before = data.clone();
    let funds = balances.clone();
    assert_eq!(
        before
            .weekly
            .as_ref()
            .ok_or(TaskBoardRefusal::WrongState)?
            .rewards()?,
        (100, 2)
    );
    let mut draws = 0;
    let refused = apply_action(
        &mut data,
        &mut balances,
        &TaskBoardAction::StartWeekly { difficulty: 3 },
        &ctx,
        &mut |_| {
            draws += 1;
            0
        },
    );
    assert_eq!(refused, Err(TaskBoardRefusal::WrongState));
    assert_eq!(
        draws, 0,
        "an active-board refusal must not generate replacement tasks"
    );
    assert_eq!(data, before);
    assert_eq!(balances, funds);
    ctx.period += 1;
    apply(&mut data, &mut balances, TaskBoardAction::Query, &ctx)?;
    assert_eq!((balances.points, balances.soulseals), (100, 2));
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::StartWeekly { difficulty: 3 },
        &ctx,
    )?;
    let next = data.weekly.as_ref().ok_or(TaskBoardRefusal::WrongState)?;
    assert_eq!(
        (
            next.period,
            next.difficulty,
            next.any_kills,
            next.rewards()?
        ),
        (ctx.period, 3, 0, (0, 0))
    );
    Ok(())
}

#[test]
fn expansion_entitlement_adds_three_slots_and_survives_the_reset() -> Result<(), TaskBoardRefusal> {
    let catalogue = creatures();
    let mut ctx = context(&catalogue);
    let mut data = TaskBoardData::default();
    let mut balances = TaskBoardBalances::default();
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::StartWeekly { difficulty: 2 },
        &ctx,
    )?;
    let first = data
        .weekly
        .as_ref()
        .ok_or(TaskBoardRefusal::WrongState)?
        .kills
        .clone();
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::GrantExpansion,
        &ctx,
    )?;
    let board = data.weekly.as_ref().ok_or(TaskBoardRefusal::WrongState)?;
    assert_eq!((board.kills.len() + 1, board.deliveries.len()), (9, 9));
    assert_eq!(&board.kills[..5], first.as_slice());
    assert!(data.expansion);
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::GrantExpansion,
            &ctx
        ),
        Err(TaskBoardRefusal::AlreadyPurchased)
    );
    ctx.period += 1;
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::StartWeekly { difficulty: 0 },
        &ctx,
    )?;
    assert_eq!(
        data.weekly
            .as_ref()
            .ok_or(TaskBoardRefusal::WrongState)?
            .kills
            .len(),
        8
    );
    assert_eq!(
        SHOP_OFFERS.len(),
        24,
        "commented 50000-point expansion is not an active offer"
    );
    assert!(stars_match(2, 2, true) && stars_match(2, 5, true));
    assert!(!stars_match(2, 2, false) && !stars_match(2, 5, false));
    Ok(())
}

#[test]
fn bounty_selection_completion_and_claim_use_own_state_and_rewards() -> Result<(), TaskBoardRefusal>
{
    let catalogue = creatures();
    let mut ctx = context(&catalogue);
    let mut data = TaskBoardData::default();
    let mut balances = TaskBoardBalances::default();
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::SelectBounty { slot: 0 },
            &ctx
        ),
        Err(TaskBoardRefusal::WrongState)
    );
    apply(&mut data, &mut balances, TaskBoardAction::OpenBounty, &ctx)?;
    assert_eq!(data.bounty.offers.len(), 3);
    assert_eq!(
        (
            data.bounty.offers[0].grade,
            data.bounty.offers[0].points,
            data.bounty.offers[0].experience
        ),
        (2, 12, 12000)
    );
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::SelectBounty { slot: 3 },
            &ctx
        ),
        Err(TaskBoardRefusal::InvalidInput)
    );
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::SelectBounty { slot: 0 },
        &ctx,
    )?;
    assert_eq!(data.bounty.state, BountyStatus::Active);
    assert!(data.bounty.offers.is_empty());
    assert_eq!(
        apply(&mut data, &mut balances, TaskBoardAction::ClaimBounty, &ctx),
        Err(TaskBoardRefusal::WrongState)
    );
    let task = data
        .bounty
        .active
        .as_mut()
        .ok_or(TaskBoardRefusal::WrongState)?;
    task.current = task.required - 1;
    let race = task.race.clone();
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::CreditKill { race },
        &ctx,
    )?;
    assert_eq!(data.bounty.state, BountyStatus::Completed);
    assert_eq!(balances.bounty_points, 0);
    // Crystal caps the daily claim at ten, not a completed bounty's +1 token.
    ctx.reroll_tokens = 10;
    let effects = apply(&mut data, &mut balances, TaskBoardAction::ClaimBounty, &ctx)?;
    assert_eq!(
        (
            effects.experience,
            effects.reroll_tokens_granted,
            balances.bounty_points
        ),
        (12000, 1, 12)
    );
    assert_eq!(data.bounty.state, BountyStatus::Selection);
    assert!(data.bounty.active.is_none());
    assert_eq!(
        apply(&mut data, &mut balances, TaskBoardAction::ClaimBounty, &ctx),
        Err(TaskBoardRefusal::WrongState)
    );
    Ok(())
}

#[test]
fn bounty_grade_boundaries_and_difficulty_reward_tables_match_crystal()
-> Result<(), TaskBoardRefusal> {
    let catalogue = creatures();
    let ctx = context(&catalogue);
    for (difficulty, min_exp, points) in [(0, 30, 3), (1, 70, 7), (2, 150, 16), (3, 300, 27)] {
        for (roll, grade, multiplier) in [
            (0, 2, 4),
            (1, 2, 4),
            (2, 1, 2),
            (9, 1, 2),
            (10, 0, 1),
            (99, 0, 1),
        ] {
            let mut data = TaskBoardData::default();
            let mut balances = TaskBoardBalances::default();
            data.bounty.difficulty = difficulty;
            let mut count = 0;
            apply_action(
                &mut data,
                &mut balances,
                &TaskBoardAction::OpenBounty,
                &ctx,
                &mut |_| {
                    count += 1;
                    if count % 4 == 0 { roll } else { 0 }
                },
            )?;
            for offer in &data.bounty.offers {
                assert_eq!(
                    (offer.grade, offer.points, offer.experience),
                    (
                        grade,
                        points * multiplier,
                        100 * min_exp * u64::from(multiplier)
                    )
                );
            }
        }
    }
    Ok(())
}

#[test]
fn bounty_completion_preserves_existing_wide_token_owner() -> Result<(), TaskBoardRefusal> {
    let catalogue = creatures();
    let mut ctx = context(&catalogue);
    ctx.reroll_tokens = u64::from(u32::MAX) + 10;
    let mut data = TaskBoardData::default();
    let mut balances = TaskBoardBalances::default();
    apply(&mut data, &mut balances, TaskBoardAction::OpenBounty, &ctx)?;
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::SelectBounty { slot: 0 },
        &ctx,
    )?;
    let task = data
        .bounty
        .active
        .as_mut()
        .ok_or(TaskBoardRefusal::WrongState)?;
    task.current = task.required - 1;
    let race = task.race.clone();
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::CreditKill { race },
        &ctx,
    )?;
    let effects = apply(&mut data, &mut balances, TaskBoardAction::ClaimBounty, &ctx)?;
    assert_eq!(effects.reroll_tokens_granted, 1);
    assert_eq!(
        ctx.reroll_tokens + u64::from(effects.reroll_tokens_granted),
        u64::from(u32::MAX) + 11
    );
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::ClaimDailyReroll,
            &ctx
        ),
        Err(TaskBoardRefusal::LimitReached)
    );
    Ok(())
}

#[test]
fn difficulty_changes_wait_for_reroll_and_preserve_active_reward() -> Result<(), TaskBoardRefusal> {
    let catalogue = creatures();
    let mut ctx = context(&catalogue);
    let mut data = TaskBoardData::default();
    let mut balances = TaskBoardBalances::default();
    apply(&mut data, &mut balances, TaskBoardAction::OpenBounty, &ctx)?;
    let offers = data.bounty.offers.clone();
    ctx.level = 149;
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::ChangeBountyDifficulty { difficulty: 2 },
            &ctx
        ),
        Err(TaskBoardRefusal::LevelTooLow)
    );
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::ChangeBountyDifficulty { difficulty: 4 },
            &ctx
        ),
        Err(TaskBoardRefusal::InvalidInput)
    );
    assert_eq!(data.bounty.difficulty, 0);
    ctx.level = 300;
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::ChangeBountyDifficulty { difficulty: 2 },
        &ctx,
    )?;
    assert_eq!(data.bounty.offers, offers);
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::SelectBounty { slot: 0 },
        &ctx,
    )?;
    let mut selected = offers[0].clone();
    selected.difficulty = 2;
    assert_eq!(data.bounty.active, Some(selected.clone()));
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::ChangeBountyDifficulty { difficulty: 3 },
        &ctx,
    )?;
    assert_eq!(data.bounty.active, Some(selected));
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::RerollBounty,
        &ctx,
    )?;
    assert!(data.bounty.active.is_none());
    assert!(
        data.bounty
            .offers
            .iter()
            .all(|task| task.difficulty == 3 && task.required == 300 && task.points == 108)
    );
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::SetPreferred {
                slot: 1,
                race: Some(catalogue[0].race.clone())
            },
            &ctx
        ),
        Err(TaskBoardRefusal::WrongState)
    );
    Ok(())
}

#[test]
fn token_owner_boundary_refuses_before_consuming_completed_bounty() -> Result<(), TaskBoardRefusal>
{
    let catalogue = creatures();
    let mut ctx = context(&catalogue);
    let mut data = TaskBoardData::default();
    let mut balances = TaskBoardBalances::default();
    apply(&mut data, &mut balances, TaskBoardAction::OpenBounty, &ctx)?;
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::SelectBounty { slot: 0 },
        &ctx,
    )?;
    let task = data
        .bounty
        .active
        .as_mut()
        .ok_or(TaskBoardRefusal::WrongState)?;
    task.current = task.required - 1;
    let race = task.race.clone();
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::CreditKill { race },
        &ctx,
    )?;
    ctx.reroll_tokens = i64::MAX.unsigned_abs();
    let before = (data.clone(), balances.clone());
    assert_eq!(
        apply(&mut data, &mut balances, TaskBoardAction::ClaimBounty, &ctx),
        Err(TaskBoardRefusal::ArithmeticOverflow)
    );
    assert_eq!((data.clone(), balances.clone()), before);
    ctx.reroll_tokens -= 1;
    let effects = apply(&mut data, &mut balances, TaskBoardAction::ClaimBounty, &ctx)?;
    assert_eq!(
        ctx.reroll_tokens + u64::from(effects.reroll_tokens_granted),
        i64::MAX.unsigned_abs()
    );
    assert_eq!(data.bounty.state, BountyStatus::Selection);
    Ok(())
}

#[test]
fn bounty_open_uses_vocation_and_reroll_preserves_difficulty_after_level_loss()
-> Result<(), TaskBoardRefusal> {
    let catalogue = creatures();
    let mut ctx = context(&catalogue);
    let mut data = TaskBoardData::default();
    let mut balances = TaskBoardBalances::default();
    ctx.level = 8;
    ctx.has_vocation = false;
    assert_eq!(
        apply(&mut data, &mut balances, TaskBoardAction::OpenBounty, &ctx),
        Err(TaskBoardRefusal::WrongState)
    );
    assert_eq!(data, TaskBoardData::default());
    ctx.has_vocation = true;
    ctx.level = 7;
    apply(&mut data, &mut balances, TaskBoardAction::OpenBounty, &ctx)?;
    assert_eq!(
        data.bounty.offers.len(),
        3,
        "a mainland character may open after losing a level"
    );
    ctx.level = 30;
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::ChangeBountyDifficulty { difficulty: 1 },
        &ctx,
    )?;
    ctx.level = 29;
    apply(&mut data, &mut balances, TaskBoardAction::OpenBounty, &ctx)?;
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::RerollBounty,
        &ctx,
    )?;
    assert!(
        data.bounty
            .offers
            .iter()
            .all(|task| task.difficulty == 1 && task.required == 100)
    );
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::ChangeBountyDifficulty { difficulty: 1 },
            &ctx
        ),
        Err(TaskBoardRefusal::LevelTooLow)
    );
    Ok(())
}

#[test]
fn bounty_rerolls_use_twenty_hour_free_timer_before_spending_one_token()
-> Result<(), TaskBoardRefusal> {
    let catalogue = creatures();
    let mut ctx = context(&catalogue);
    let mut data = TaskBoardData::default();
    let mut balances = TaskBoardBalances::default();
    ctx.reroll_tokens = 2;
    let effects = apply(
        &mut data,
        &mut balances,
        TaskBoardAction::RerollBounty,
        &ctx,
    )?;
    assert_eq!(effects.reroll_tokens_spent, 0);
    assert_eq!(
        data.bounty.next_free_reroll_unix,
        ctx.now_unix_seconds + 72_000
    );
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::RerollBounty,
            &ctx
        )?
        .reroll_tokens_spent,
        1
    );
    ctx.reroll_tokens = 0;
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::RerollBounty,
            &ctx
        ),
        Err(TaskBoardRefusal::Cooldown)
    );
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::ClaimDailyReroll,
            &ctx
        ),
        Err(TaskBoardRefusal::Cooldown)
    );
    ctx.now_unix_seconds += 72_000;
    ctx.reroll_tokens = 10;
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::ClaimDailyReroll,
            &ctx
        ),
        Err(TaskBoardRefusal::LimitReached)
    );
    ctx.reroll_tokens = 9;
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::ClaimDailyReroll,
            &ctx
        )?
        .reroll_tokens_granted,
        1
    );
    Ok(())
}

#[test]
fn preferred_lists_unlock_in_order_and_clear_costs_ten() -> Result<(), TaskBoardRefusal> {
    let catalogue = creatures();
    let ctx = context(&catalogue);
    let mut data = TaskBoardData::default();
    let mut balances = TaskBoardBalances::default();
    assert!(data.bounty.lists[0].unlocked);
    assert_eq!(
        apply(&mut data, &mut balances, TaskBoardAction::UnlockList, &ctx),
        Err(TaskBoardRefusal::InsufficientResources)
    );
    balances.bounty_points = 3_000;
    for (slot, balance) in [(1, 2700), (2, 2100), (3, 1200), (4, 0)] {
        apply(&mut data, &mut balances, TaskBoardAction::UnlockList, &ctx)?;
        assert!(data.bounty.lists[slot].unlocked);
        assert_eq!(balances.bounty_points, balance);
    }
    assert_eq!(
        apply(&mut data, &mut balances, TaskBoardAction::UnlockList, &ctx),
        Err(TaskBoardRefusal::LimitReached)
    );
    let race = catalogue[5].race.clone();
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::SetPreferred {
            slot: 0,
            race: Some(race.clone()),
        },
        &ctx,
    )?;
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::SetPreferred {
                slot: 0,
                race: None
            },
            &ctx
        ),
        Err(TaskBoardRefusal::InsufficientResources)
    );
    balances.bounty_points = 10;
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::SetPreferred {
            slot: 0,
            race: None,
        },
        &ctx,
    )?;
    assert_eq!(balances.bounty_points, 0);
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::SetPreferred {
            slot: 0,
            race: None,
        },
        &ctx,
    )?;
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::SetPreferred {
                slot: 5,
                race: Some(race)
            },
            &ctx
        ),
        Err(TaskBoardRefusal::InvalidInput)
    );
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::SetUnwanted {
                slot: 0,
                race: Some("unknown".to_owned())
            },
            &ctx
        ),
        Err(TaskBoardRefusal::InvalidInput)
    );
    Ok(())
}

#[test]
fn unwanted_overrides_preferred_and_preferred_bypasses_only_stars() -> Result<(), TaskBoardRefusal>
{
    let mut catalogue = creatures();
    // Put a hard, preferred creature first so the deterministic draw selects it.
    catalogue[0].bestiary_stars = 5;
    let ctx = context(&catalogue);
    let mut data = TaskBoardData::default();
    let mut balances = TaskBoardBalances::default();
    let race = catalogue[0].race.clone();
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::SetPreferred {
            slot: 0,
            race: Some(race.clone()),
        },
        &ctx,
    )?;
    apply(&mut data, &mut balances, TaskBoardAction::OpenBounty, &ctx)?;
    assert_eq!(data.bounty.offers[0].race, race);
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::SetUnwanted {
            slot: 0,
            race: Some(race.clone()),
        },
        &ctx,
    )?;
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::RerollBounty,
        &ctx,
    )?;
    assert!(data.bounty.offers.iter().all(|task| task.race != race));
    Ok(())
}

#[test]
fn shop_items_cosmetics_and_progressive_promotions_use_sourced_prices()
-> Result<(), TaskBoardRefusal> {
    let catalogue = creatures();
    let ctx = context(&catalogue);
    let mut data = TaskBoardData::default();
    let mut balances = TaskBoardBalances::default();
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::BuyShop { offer: 10 },
            &ctx
        ),
        Err(TaskBoardRefusal::InsufficientResources)
    );
    balances.points = 1_000_000;
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::BuyShop { offer: 1 },
            &ctx
        ),
        Err(TaskBoardRefusal::RequiresOutfit)
    );
    let effects = apply(
        &mut data,
        &mut balances,
        TaskBoardAction::BuyShop { offer: 0 },
        &ctx,
    )?;
    assert_eq!(effects.outfit, Some((1907, 0)));
    assert_eq!(balances.points, 900_000);
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::BuyShop { offer: 0 },
            &ctx
        ),
        Err(TaskBoardRefusal::AlreadyPurchased)
    );
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::BuyShop { offer: 1 },
            &ctx
        )?
        .outfit,
        Some((1907, 1))
    );
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::BuyShop { offer: 6 },
            &ctx
        )?
        .mount,
        Some(1908)
    );
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::BuyShop { offer: 10 },
            &ctx
        )?
        .grant_items,
        vec![TaskBoardItemQuantity {
            item_id: 36753,
            amount: 1
        }]
    );
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::BuyShop { offer: 22 },
            &ctx
        )?
        .grant_items,
        vec![TaskBoardItemQuantity {
            item_id: 32795,
            amount: 1
        }],
        "one headboard kit places both bed parts"
    );
    let points = balances.points;
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::BuyShop { offer: 23 },
            &ctx
        )?
        .promotion_points,
        1
    );
    assert_eq!(balances.points, points - 100);
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::BuyShop { offer: 23 },
        &ctx,
    )?;
    assert_eq!(balances.points, points - 300);
    assert_eq!(data.promotion_points, 2);
    data.promotion_points = 50;
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::BuyShop { offer: 23 },
            &ctx
        ),
        Err(TaskBoardRefusal::LimitReached)
    );
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::BuyShop { offer: 24 },
            &ctx
        ),
        Err(TaskBoardRefusal::InvalidInput)
    );
    Ok(())
}

#[test]
fn talisman_purchase_upgrade_and_active_gate_are_bounty_specific() -> Result<(), TaskBoardRefusal> {
    let catalogue = creatures();
    let mut ctx = context(&catalogue);
    let mut data = TaskBoardData::default();
    let mut balances = TaskBoardBalances::default();
    ctx.available_gold = 4_999;
    assert_eq!(
        apply(&mut data, &mut balances, TaskBoardAction::BuyTalisman, &ctx),
        Err(TaskBoardRefusal::InsufficientResources)
    );
    ctx.available_gold = 5_000;
    let purchase = apply(&mut data, &mut balances, TaskBoardAction::BuyTalisman, &ctx)?;
    assert_eq!(purchase.gold_cost, 5_000);
    assert_eq!(
        purchase.grant_items,
        vec![TaskBoardItemQuantity {
            item_id: 51978,
            amount: 1
        }]
    );
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::UpgradeTalisman { path: 0 },
            &ctx
        ),
        Err(TaskBoardRefusal::InsufficientResources)
    );
    balances.bounty_points = 10_000;
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::UpgradeTalisman { path: 0 },
        &ctx,
    )?;
    assert_eq!(
        (balances.talisman_levels[0], balances.bounty_points),
        (1, 9995)
    );
    // PR #2764 answer 6542770501 preserves the previously accepted NEWS-HIST-TALISMAN-1 profile.
    balances.talisman_levels[0] = 355;
    balances.talisman_levels[3] = 355;
    for path in [0, 3] {
        assert_eq!(
            apply(
                &mut data,
                &mut balances,
                TaskBoardAction::UpgradeTalisman { path },
                &ctx
            ),
            Err(TaskBoardRefusal::LimitReached)
        );
    }
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::UpgradeTalisman { path: 4 },
            &ctx
        ),
        Err(TaskBoardRefusal::InvalidInput)
    );
    let mut tracks = BTreeMap::new();
    balances.store_into_tracks(&mut tracks);
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::StartWeekly { difficulty: 0 },
        &ctx,
    )?;
    let race = data
        .weekly
        .as_ref()
        .ok_or(TaskBoardRefusal::WrongState)?
        .kills[0]
        .race
        .clone();
    assert_eq!(
        data.active_talisman_bonuses(&tracks, &race, true),
        [0; 4],
        "a weekly target never activates the talisman"
    );
    apply(&mut data, &mut balances, TaskBoardAction::OpenBounty, &ctx)?;
    apply(
        &mut data,
        &mut balances,
        TaskBoardAction::SelectBounty { slot: 0 },
        &ctx,
    )?;
    let task = data
        .bounty
        .active
        .as_ref()
        .ok_or(TaskBoardRefusal::WrongState)?;
    let race = task.race.clone();
    assert_eq!(
        data.active_talisman_bonuses(&tracks, &race, true),
        [5000, 0, 0, 10000]
    );
    assert_eq!(data.active_talisman_bonuses(&tracks, &race, false), [0; 4]);
    assert_eq!(
        data.active_talisman_bonuses(&tracks, "unrelated", true),
        [0; 4]
    );
    data.bounty.state = BountyStatus::Completed;
    assert_eq!(data.active_talisman_bonuses(&tracks, &race, true), [0; 4]);
    Ok(())
}

#[test]
fn malformed_random_or_missing_catalogue_never_mutates_state() -> Result<(), TaskBoardRefusal> {
    let catalogue = creatures();
    let ctx = context(&catalogue);
    let mut data = TaskBoardData::default();
    let mut balances = TaskBoardBalances::default();
    assert_eq!(
        apply_action(
            &mut data,
            &mut balances,
            &TaskBoardAction::StartWeekly { difficulty: 0 },
            &ctx,
            &mut |upper| upper
        ),
        Err(TaskBoardRefusal::InvalidRandomDraw)
    );
    assert_eq!(data, TaskBoardData::default());
    assert_eq!(balances, TaskBoardBalances::default());
    let ctx = context(&[]);
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::StartWeekly { difficulty: 0 },
            &ctx
        ),
        Err(TaskBoardRefusal::MissingCatalogue)
    );
    assert_eq!(data, TaskBoardData::default());
    assert_eq!(
        apply(
            &mut data,
            &mut balances,
            TaskBoardAction::RefuseLegacy { action: 1, task: 1 },
            &ctx
        ),
        Err(TaskBoardRefusal::RetiredAction)
    );
    Ok(())
}

#[test]
fn all_delivery_definitions_are_unique_and_quantities_are_positive() {
    assert_eq!(DELIVERY_ITEMS.len(), 436);
    assert!(
        DELIVERY_ITEMS
            .iter()
            .all(|item| item.item_id > 0 && item.amount > 0)
    );
    assert_eq!(
        DELIVERY_ITEMS
            .iter()
            .map(|item| item.item_id)
            .collect::<BTreeSet<_>>()
            .len(),
        436
    );
}

#[test]
fn grappling_hook_delivery_uses_approved_portable_source_id() -> Result<(), TaskBoardRefusal> {
    let catalogue = creatures();
    let ctx = context(&catalogue);
    let mut data = TaskBoardData::default();
    let mut balances = TaskBoardBalances::default();
    assert!(!DELIVERY_ITEMS.iter().any(|item| item.item_id == 5800));
    let hook_index = DELIVERY_ITEMS
        .iter()
        .position(|item| item.item_id == 35588)
        .ok_or(TaskBoardRefusal::MissingCatalogue)?;
    let hook_index = u32::try_from(hook_index).map_err(|_| TaskBoardRefusal::InvalidInput)?;
    apply_action(
        &mut data,
        &mut balances,
        &TaskBoardAction::StartWeekly { difficulty: 0 },
        &ctx,
        &mut |upper| if upper == 436 { hook_index } else { 0 },
    )?;
    let board = data.weekly.as_ref().ok_or(TaskBoardRefusal::WrongState)?;
    assert_eq!(
        board
            .deliveries
            .first()
            .map(|task| (task.item_id, task.required)),
        Some((35588, 50))
    );
    let effects = apply(
        &mut data,
        &mut balances,
        TaskBoardAction::Deliver { slot: 0 },
        &ctx,
    )?;
    assert_eq!(
        effects.consume_items,
        vec![TaskBoardItemQuantity {
            item_id: 35588,
            amount: 50
        }]
    );
    Ok(())
}

#[test]
fn shop_kit_targets_include_non_pickupable_cerberus() {
    let targets: Vec<_> = SHOP_OFFERS
        .iter()
        .filter(|offer| offer.requires_decoration_kit())
        .map(|offer| (offer.id, offer.appearance_or_item))
        .collect();
    assert_eq!(
        targets,
        vec![
            (10, 36753),
            (11, 31465),
            (12, 32753),
            (13, 32756),
            (14, 36748),
            (15, 36749),
            (16, 32752),
            (17, 32755),
            (18, 32754),
            (19, 32751),
            (20, 32918),
            (21, 32943),
            (22, 32795)
        ]
    );
    assert_eq!(SHOP_OFFERS[11].appearance_or_item, 31465);
    assert!(SHOP_OFFERS[11].requires_decoration_kit());
    assert_eq!(SHOP_OFFERS[11].points, 75_000);
}

#[test]
fn balances_round_trip_existing_quest_tracks_without_overwriting_other_quests() {
    let balances = TaskBoardBalances {
        points: 42,
        bounty_points: 7,
        soulseals: 3,
        talisman_levels: [1, 2, 3, 4],
    };
    let mut tracks = BTreeMap::from([("oteryn:other/quest".to_owned(), 99)]);
    balances.store_into_tracks(&mut tracks);
    assert_eq!(TaskBoardBalances::from_tracks(&tracks), balances);
    assert_eq!(tracks.get("oteryn:other/quest"), Some(&99));
}

#[test]
fn weekly_balances_cross_legacy_billion_bound_and_refuse_owner_overflow()
-> Result<(), TaskBoardRefusal> {
    let catalogue = creatures();
    let mut ctx = context(&catalogue);
    let mut pending = TaskBoardData::default();
    let mut initial = TaskBoardBalances::default();
    apply(
        &mut pending,
        &mut initial,
        TaskBoardAction::StartWeekly { difficulty: 0 },
        &ctx,
    )?;
    apply(
        &mut pending,
        &mut initial,
        TaskBoardAction::Deliver { slot: 0 },
        &ctx,
    )?;
    ctx.period += 1;
    for (points, soulseals, succeeds) in [
        (1_000_000_000 - 25, 0, true),
        (i64::MAX - 75, i64::from(MAX_SOULSEALS) - 1, true),
        (i64::MAX - 74, i64::from(MAX_SOULSEALS) - 1, false),
        (100, i64::from(MAX_SOULSEALS), false),
    ] {
        let mut data = pending.clone();
        let mut balances = TaskBoardBalances {
            points,
            soulseals,
            ..TaskBoardBalances::default()
        };
        let before = (data.clone(), balances.clone());
        let result = apply(&mut data, &mut balances, TaskBoardAction::Query, &ctx);
        if succeeds {
            result?;
            assert_eq!(
                (balances.points, balances.soulseals),
                (points + 75, soulseals + 1)
            );
            let paid = (data.clone(), balances.clone());
            apply(&mut data, &mut balances, TaskBoardAction::Query, &ctx)?;
            assert_eq!(
                (data, balances),
                paid,
                "settled rewards are not credited twice"
            );
        } else {
            assert_eq!(result, Err(TaskBoardRefusal::ArithmeticOverflow));
            assert_eq!(
                (&data, &balances),
                (&before.0, &before.1),
                "seal overflow must also roll back the hunting points addition"
            );
            assert_eq!(
                apply(&mut data, &mut balances, TaskBoardAction::Query, &ctx),
                Err(TaskBoardRefusal::ArithmeticOverflow)
            );
            assert_eq!((data, balances), before);
        }
    }
    Ok(())
}

#[test]
fn bounty_points_last_valid_reward_then_overflow_keeps_completed_task()
-> Result<(), TaskBoardRefusal> {
    let catalogue = creatures();
    let ctx = context(&catalogue);
    let mut completed = TaskBoardData::default();
    let mut initial = TaskBoardBalances::default();
    apply(
        &mut completed,
        &mut initial,
        TaskBoardAction::OpenBounty,
        &ctx,
    )?;
    apply(
        &mut completed,
        &mut initial,
        TaskBoardAction::SelectBounty { slot: 0 },
        &ctx,
    )?;
    let task = completed
        .bounty
        .active
        .as_mut()
        .ok_or(TaskBoardRefusal::WrongState)?;
    task.current = task.required - 1;
    let race = task.race.clone();
    let reward = i64::from(task.points);
    apply(
        &mut completed,
        &mut initial,
        TaskBoardAction::CreditKill { race },
        &ctx,
    )?;
    for succeeds in [true, false] {
        let mut data = completed.clone();
        let mut balances = TaskBoardBalances {
            bounty_points: i64::from(MAX_BOUNTY_POINTS) - reward + i64::from(!succeeds),
            ..TaskBoardBalances::default()
        };
        let before = (data.clone(), balances.clone());
        let result = apply(&mut data, &mut balances, TaskBoardAction::ClaimBounty, &ctx);
        if succeeds {
            let effects = result?;
            assert_eq!(balances.bounty_points, i64::from(MAX_BOUNTY_POINTS));
            assert_eq!(effects.reroll_tokens_granted, 1);
            let claimed = (data.clone(), balances.clone());
            assert_eq!(
                apply(&mut data, &mut balances, TaskBoardAction::ClaimBounty, &ctx),
                Err(TaskBoardRefusal::WrongState)
            );
            assert_eq!((data, balances), claimed);
        } else {
            assert_eq!(result, Err(TaskBoardRefusal::ArithmeticOverflow));
            assert_eq!((&data, &balances), (&before.0, &before.1));
            assert_eq!(
                apply(&mut data, &mut balances, TaskBoardAction::ClaimBounty, &ctx),
                Err(TaskBoardRefusal::ArithmeticOverflow)
            );
            assert_eq!(
                (data, balances),
                before,
                "overflow preserves completed reward and all balances"
            );
        }
    }
    Ok(())
}
