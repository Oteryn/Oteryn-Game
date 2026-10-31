use super::*;

/// Catalogue requirements, repeat count and the three-active-task limit are rechecked by
/// the durable owner. PR2770 Q1 accepts the catalogue requirements over the donor's
/// contradictory non-Premium branch, which also bypassed the Demon rank requirement.
/// Q16 requires the explicit join/menu contract for direct-name task starts too.
pub fn can_start_task(
    state: &GrizzlyState,
    actor: GrizzlyActor,
    id: u8,
) -> Result<(), GrizzlyRefusal> {
    let task = task(id).ok_or(GrizzlyRefusal::InvalidTask)?;
    if !joined(state) {
        return Err(GrizzlyRefusal::NotJoined);
    }
    if storage_value(state, QUEST_STORAGE_BASE + u32::from(id)) > 0 {
        return Err(GrizzlyRefusal::AlreadyStarted);
    }
    if started_tasks(state).len() >= ACTIVE_TASK_LIMIT {
        return Err(GrizzlyRefusal::TaskLimit);
    }
    if !(task.minimum_level..=task.maximum_level).contains(&actor.level) {
        return Err(GrizzlyRefusal::Level);
    }
    if task.premium && !actor.premium {
        return Err(GrizzlyRefusal::Premium);
    }
    if rank(state) < task.minimum_rank {
        return Err(GrizzlyRefusal::Rank);
    }
    if storage_value(state, REPEAT_STORAGE_BASE + u32::from(id))
        >= i64::from(task.repeat_limit.unwrap_or(3))
    {
        return Err(GrizzlyRefusal::RepeatLimit);
    }
    Ok(())
}

/// grizzly_adams.lua:485-504. Caps test the pre-reward points; a reward may cross
/// a threshold. At level 130 the catch-up bonus is added after the configured rate (1).
#[must_use]
pub fn points_reward(level: u32, current: i64, reward: i64) -> i64 {
    if (level < 50 && current >= 40)
        || (level < 80 && current >= 70)
        || (level < 130 && current >= 100)
    {
        return 0;
    }
    let bonus = if level >= 130 {
        match current {
            ..=20 => 3,
            21..=40 => 2,
            41..=70 => 1,
            _ => 0,
        }
    } else {
        0
    };
    reward + bonus
}

/// grizzly_adams.lua:741-774. Promotions advance one rank per conversation.
#[must_use]
pub fn promotion_due(state: &GrizzlyState, level: u32) -> Option<i64> {
    let points = storage_value(state, POINTS_STORAGE);
    match storage_value(state, PROMOTION_STORAGE) {
        ..=-1 if points >= 10 && level >= 6 => Some(0),
        0 if points >= 20 && level >= 6 => Some(2),
        2 if points >= 40 && level >= 50 => Some(4),
        4 if points >= 70 && level >= 80 => Some(6),
        6 if points >= 100 && level >= 130 => Some(7),
        _ => None,
    }
}

struct Plan<'a> {
    state: &'a GrizzlyState,
    transition: GrizzlyTransition,
}

impl Plan<'_> {
    fn set(&mut self, id: u32, after: i64) -> Result<(), GrizzlyRefusal> {
        if !(-1..=i64::from(i32::MAX)).contains(&after) {
            return Err(GrizzlyRefusal::OutOfRange);
        }
        self.transition.changes.push(QuestTrackChange {
            track: storage_track(id),
            before: storage_value(self.state, id),
            after,
        });
        Ok(())
    }

    fn add(&mut self, id: u32, value: i64) -> Result<(), GrizzlyRefusal> {
        self.set(
            id,
            storage_value(self.state, id)
                .checked_add(value)
                .ok_or(GrizzlyRefusal::OutOfRange)?,
        )
    }
}

/// One source mutation. XP, items and achievements are effects for the existing durable
/// owners, never applied here. A receipt replay is answered before this function is called.
pub fn plan_action(
    state: &GrizzlyState,
    actor: GrizzlyActor,
    action: &GrizzlyAction,
) -> Result<GrizzlyTransition, GrizzlyRefusal> {
    catalogue()?;
    let mut plan = Plan {
        state,
        transition: GrizzlyTransition::default(),
    };
    match action {
        GrizzlyAction::Join => {
            if joined(state) {
                return Err(GrizzlyRefusal::AlreadyJoined);
            }
            // grizzly_adams.lua:412-416.
            for (id, value) in [
                (JOIN_STORAGE, 1),
                (BOSS_POINTS_STORAGE, 0),
                (QUEST_LOG_STORAGE, 0),
                (POINTS_STORAGE, 0),
            ] {
                plan.set(id, value)?;
            }
        }
        GrizzlyAction::StartTask(id) => {
            can_start_task(state, actor, *id)?;
            let task = task(*id).ok_or(GrizzlyRefusal::InvalidTask)?;
            let id = u32::from(*id);
            // grizzly_adams.lua:675-718: starting a new run resets all its counters.
            plan.set(QUEST_STORAGE_BASE + id, 1)?;
            plan.set(KILL_COUNTER_BASE + id, 0)?;
            for creature in &task.creatures {
                if let Some(storage) = creature.alt_storage {
                    plan.set(storage, 0)?;
                }
            }
            let previous = storage_value(state, KILLS_STORAGE_BASE + id);
            plan.set(
                KILLS_STORAGE_BASE + id,
                if previous == 1 {
                    0
                } else {
                    previous.checked_add(1).ok_or(GrizzlyRefusal::OutOfRange)?
                },
            )?;
        }
        GrizzlyAction::CancelTask(id) => {
            active_task(state, *id)?;
            let id = u32::from(*id);
            // grizzly_adams.lua:895-898. Cancellation does not spend a repeat.
            plan.set(QUEST_STORAGE_BASE + id, -1)?;
            plan.add(KILLS_STORAGE_BASE + id, -1)?;
            plan.set(KILL_COUNTER_BASE + id, 0)?;
        }
        GrizzlyAction::CompleteTask(id) => complete(&mut plan, actor, *id)?,
        GrizzlyAction::Promote => {
            let next = promotion_due(state, actor.level).ok_or(GrizzlyRefusal::NoPromotion)?;
            plan.set(PROMOTION_STORAGE, next)?;
        }
        GrizzlyAction::SelectBoss(id) => select_boss(&mut plan, actor, *id)?,
        GrizzlyAction::ReportBoss(id) => {
            let boss = boss(*id)
                .filter(|boss| boss.task_id.is_some())
                .ok_or(GrizzlyRefusal::InvalidBoss)?;
            if storage_value(state, boss.storage) != 2 {
                return Err(GrizzlyRefusal::BossNotEntered);
            }
            // grizzly_adams.lua:382-395: a failed visit is rearmed; exactly one kill
            // completes the report and grants the achievement.
            match storage_value(state, boss.kill_storage) {
                0 => plan.set(boss.storage, 1)?,
                1 => {
                    plan.set(boss.storage, 3)?;
                    plan.set(boss.kill_storage, 0)?;
                    plan.transition.achievement = boss.achievement.clone();
                }
                _ => return Err(GrizzlyRefusal::BossNotKilled),
            }
        }
        GrizzlyAction::StartSpecialTask => special_task(&mut plan, actor)?,
        GrizzlyAction::EnterBoss(id) => {
            check_boss_room(state, *id, false)?;
            plan.set(boss(*id).ok_or(GrizzlyRefusal::InvalidBoss)?.storage, 2)?;
        }
        GrizzlyAction::KillCreature {
            task_id, creature, ..
        } => {
            let task = active_task(state, *task_id)?;
            let member = task
                .creatures
                .iter()
                .find(|member| member.matches(creature))
                .ok_or(GrizzlyRefusal::CreatureMismatch)?;
            // creaturescripts_monster_kill.lua:25-56. Source counters keep increasing
            // after reaching the requirement until the task is reported.
            if let Some(storage) = member.alt_storage {
                plan.add(storage, 1)?;
            }
            plan.add(KILL_COUNTER_BASE + u32::from(*task_id), 1)?;
            let tracker = KILLS_STORAGE_BASE + u32::from(*task_id);
            plan.set(tracker, storage_value(state, tracker))?;
        }
        GrizzlyAction::BossKill { boss_id, .. } => {
            let boss = boss(*boss_id).ok_or(GrizzlyRefusal::InvalidBoss)?;
            // creaturescripts_boss_kill.lua:43-52 counts a matching boss even before a task.
            plan.set(
                boss.kill_storage,
                storage_value(state, boss.kill_storage)
                    .max(0)
                    .checked_add(1)
                    .ok_or(GrizzlyRefusal::OutOfRange)?,
            )?;
        }
    }
    if plan.transition.changes.len() > super::super::QUESTSTATE0_RL_02 {
        return Err(GrizzlyRefusal::OutOfRange);
    }
    Ok(plan.transition)
}

fn active_task(state: &GrizzlyState, id: u8) -> Result<&'static GrizzlyTask, GrizzlyRefusal> {
    let task = task(id).ok_or(GrizzlyRefusal::InvalidTask)?;
    if storage_value(state, QUEST_STORAGE_BASE + u32::from(id)) != 1 {
        return Err(GrizzlyRefusal::NotStarted);
    }
    Ok(task)
}

fn complete(plan: &mut Plan<'_>, actor: GrizzlyActor, id: u8) -> Result<(), GrizzlyRefusal> {
    let task = active_task(plan.state, id)?;
    let id = u32::from(id);
    if storage_value(plan.state, KILL_COUNTER_BASE + id) < task.required {
        return Err(GrizzlyRefusal::KillsIncomplete);
    }
    let reward_level = if task.experience.is_some() {
        actor
            .level_after_reward
            .filter(|level| *level >= actor.level)
            .ok_or(GrizzlyRefusal::ProgressionUnavailable)?
    } else {
        actor.level
    };
    // grizzly_adams.lua:460-507. Achievements wait for the boss report.
    plan.transition.experience = task.experience;
    plan.transition.item = task.item.clone();
    if let Some(reward) = &task.completion_storage {
        plan.set(reward.storage, reward.value)?;
    }
    if let Some(boss_id) = task.boss_id {
        let boss = boss(boss_id).ok_or(GrizzlyRefusal::InvalidBoss)?;
        if matches!(storage_value(plan.state, boss.storage), 1 | 2) {
            plan.add(BOSS_POINTS_STORAGE, 1)?;
        } else {
            plan.set(boss.storage, 1)?;
            plan.set(boss.kill_storage, 0)?;
        }
    }
    if let Some(base_points) = task.points {
        let current = storage_value(plan.state, POINTS_STORAGE);
        let reward = points_reward(reward_level, current, base_points);
        if reward > 0 {
            plan.set(
                POINTS_STORAGE,
                current
                    .max(0)
                    .checked_add(reward)
                    .ok_or(GrizzlyRefusal::OutOfRange)?,
            )?;
        }
    }
    // grizzly_adams.lua:515-527. Unlimited tasks do not advance the repeat counter.
    plan.set(
        QUEST_STORAGE_BASE + id,
        if task.repeat_limit == Some(1) { 2 } else { 0 },
    )?;
    let previous = storage_value(plan.state, REPEAT_STORAGE_BASE + id).max(0);
    let repeats = if task.repeat_limit.is_none() {
        previous
    } else {
        previous.checked_add(1).ok_or(GrizzlyRefusal::OutOfRange)?
    };
    plan.set(REPEAT_STORAGE_BASE + id, repeats)?;
    if repeats == 3 {
        plan.set(KILLS_STORAGE_BASE + id, 2)?;
    } else {
        plan.add(KILLS_STORAGE_BASE + id, 1)?;
    }
    // Preserve the donor's write to the base counter, not the completed task's counter.
    plan.set(KILL_COUNTER_BASE, 0)?;
    Ok(())
}

fn select_boss(plan: &mut Plan<'_>, actor: GrizzlyActor, id: u8) -> Result<(), GrizzlyRefusal> {
    let boss = boss(id)
        .filter(|boss| boss.task_id.is_some())
        .ok_or(GrizzlyRefusal::InvalidBoss)?;
    let minimum_level = match id {
        1..=4 => 0,
        5..=9 => 50,
        10..=18 => 80,
        _ => 130,
    };
    if actor.level < minimum_level {
        return Err(GrizzlyRefusal::Level);
    }
    // grizzly_adams.lua:776-779: entering the selection dialogue requires a point,
    // including the free first visit to a boss from an outgrown level bracket.
    // PR2770 Q17 rechecks this gate after every spend, closing the retained-topic bypass.
    if storage_value(plan.state, BOSS_POINTS_STORAGE) <= 0 {
        return Err(GrizzlyRefusal::BossPoints);
    }
    let previous = storage_value(plan.state, boss.storage);
    if previous != 3 && previous >= 0 {
        return Err(GrizzlyRefusal::BossActive);
    }
    let free_above = match id {
        1..=4 => 50,
        5..=9 => 80,
        10..=18 => 130,
        _ => u32::MAX,
    };
    plan.set(boss.storage, 1)?;
    plan.set(boss.kill_storage, 0)?;
    if !(previous < 0 && actor.level >= free_above) {
        plan.add(BOSS_POINTS_STORAGE, -1)?;
    }
    Ok(())
}

fn special_task(plan: &mut Plan<'_>, actor: GrizzlyActor) -> Result<(), GrizzlyRefusal> {
    // grizzly_adams.lua:917-940: use the first matching branch, considering Tiquanda
    // before Demodras. An incidental boss kill can make the first task fall through.
    if actor.level < 80 {
        return Err(GrizzlyRefusal::Level);
    }
    if storage_value(plan.state, POINTS_STORAGE) < 70 {
        return Err(GrizzlyRefusal::Rank);
    }
    let mut all_completed = true;
    for id in [29, 30] {
        let boss = boss(id).ok_or(GrizzlyRefusal::InvalidBoss)?;
        let stage = storage_value(plan.state, boss.storage);
        let kills = storage_value(plan.state, boss.kill_storage);
        if stage < 1 {
            plan.set(boss.kill_storage, 0)?;
            plan.set(boss.storage, 1)?;
            return Ok(());
        }
        if stage <= 2 && kills == 0 {
            plan.set(boss.storage, 1)?;
            return Ok(());
        }
        if stage == 2 && kills == 1 {
            plan.set(boss.storage, 3)?;
            return Ok(());
        }
        if stage != 3 {
            all_completed = false;
        }
    }
    Err(if all_completed {
        GrizzlyRefusal::SpecialTasksComplete
    } else {
        GrizzlyRefusal::BossNotKilled
    })
}
