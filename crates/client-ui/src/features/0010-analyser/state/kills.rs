//! Local kill targets and an independent view of confirmed hunting-task progress.
use super::Panel;
use std::fmt::Write;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskProgress {
    pub label: String,
    pub current: u64,
    pub required: u64,
    pub complete: bool,
}
pub fn goal(panel: &mut Panel, race: &str, target: Option<&str>) -> bool {
    let Some(race) = race.trim().parse::<u32>().ok().filter(|id| *id != 0) else {
        panel.feedback = Some("Enter a creature race number from Kill Tracker");
        return false;
    };
    if let Some(target) = target {
        let Some(target) = target.trim().parse::<u64>().ok().filter(|n| *n != 0) else {
            panel.feedback = Some("Enter a positive kill target");
            return false;
        };
        if !panel.kills_by_race.contains_key(&race) && !panel.kill_goals.contains_key(&race) {
            panel.feedback = Some("Choose a creature race listed in Kill Tracker");
            return false;
        }
        if panel.kill_goals.len() >= 128 && !panel.kill_goals.contains_key(&race) {
            panel.feedback = Some("Track up to 128 creature goals");
            return false;
        }
        panel.kill_goals.insert(race, target);
    } else {
        panel.kill_goals.remove(&race);
    }
    panel.feedback = None;
    true
}
pub fn task_rows(board: &crate::task_board::TaskBoardPanel) -> Option<Vec<TaskProgress>> {
    let mut rows = Vec::new();
    if let Some(reply) = &board.weekly {
        if reply.refusal != 0 {
            return None;
        }
        if reply.weekly.started {
            for (i, task) in reply.weekly.kills.iter().enumerate() {
                if task.required == 0 {
                    continue;
                }
                let name = if task.race.is_empty() {
                    "Any creature".into()
                } else {
                    crate::task_board::creature_name(&task.race)
                };
                rows.push(TaskProgress {
                    label: format!("Week {} · kill {} · {name}", reply.period, i + 1),
                    current: u64::from(task.current),
                    required: u64::from(task.required),
                    complete: task.current >= task.required,
                });
            }
            for task in &reply.weekly.deliveries {
                rows.push(TaskProgress {
                    label: format!(
                        "Week {} · delivery item #{} ({})",
                        reply.period,
                        task.item,
                        if task.delivered {
                            "delivered"
                        } else {
                            "available for delivery"
                        }
                    ),
                    current: u64::from(task.current),
                    required: u64::from(task.required),
                    complete: task.delivered,
                });
            }
        }
        if let Some(task) = &reply.bounty.active {
            rows.push(TaskProgress {
                label: format!("Bounty · {}", crate::task_board::creature_name(&task.race)),
                current: u64::from(reply.bounty.current),
                required: u64::from(task.required),
                complete: reply.bounty.current >= task.required,
            });
        }
        return Some(rows);
    }
    let reply = board.state.as_ref()?;
    if reply.refused {
        return None;
    }
    if let Some(task) = usize::from(reply.selected)
        .checked_sub(1)
        .and_then(|i| reply.tasks.get(i))
    {
        let current = u64::try_from(reply.kills).ok()?;
        let required = u64::try_from(task.required).ok()?;
        rows.push(TaskProgress {
            label: format!(
                "Hunting task · {}",
                crate::task_board::creature_name(&task.race)
            ),
            current,
            required,
            complete: current >= required,
        });
    }
    Some(rows)
}
pub fn text(panel: &Panel) -> String {
    let mut text = format!(
        "Kill Tracker\r\nKills: {}\r\nConfirmed session kills by creature race\r\n",
        panel.kills
    );
    let races: std::collections::BTreeSet<_> = panel
        .kills_by_race
        .keys()
        .chain(panel.kill_goals.keys())
        .copied()
        .collect();
    for race in races {
        let count = panel.kills_by_race.get(&race).copied().unwrap_or(0);
        let name = if race == 0 {
            "Race unavailable".into()
        } else {
            format!("Race #{race}")
        };
        let _ = write!(text, "{name}: {count}");
        if let Some(goal) = panel.kill_goals.get(&race) {
            let remaining = goal.saturating_sub(count);
            let _ = write!(text, " / {goal} · {remaining} remaining");
            if remaining == 0 {
                text.push_str(" · target reached");
            }
        }
        text.push_str("\r\n");
    }
    text.push_str("\r\nLast confirmed hunting tasks\r\n");
    match &panel.task_progress {
        None => text.push_str("Refresh hunting tasks to read server progress.\r\n"),
        Some(rows) if rows.is_empty() => text.push_str("No active hunting tasks\r\n"),
        Some(rows) => {
            for row in rows {
                let _ = write!(
                    text,
                    "{}: {}/{} · {}\r\n",
                    row.label,
                    row.current,
                    row.required,
                    if row.complete { "complete" } else { "active" }
                );
            }
        }
    }
    if let Some(feedback) = panel.task_feedback {
        let _ = write!(text, "{feedback}\r\n");
    }
    if let Some(feedback) = panel.feedback {
        text.push_str(feedback);
    }
    text
}
