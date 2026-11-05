//! Bounded Spike rules from Crystal 201189f; callers own randomness and durable effects.
//! Source: data-global/scripts/quests/spike_tasks/*, the_spike_tasks/* and
//! data-global/npc/{gnomilly,gnombold,gnomargery}.lua at that exact commit.
pub use super::world_callbacks::Position;
pub mod dialogue;
pub mod storage_only;
use serde::Deserialize;

pub const QUEST: &str = "oteryn:quest.spike_task";
pub const EMBEDDED: &str = include_str!("../../../../../content/quests/spike-tasks.json");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub tier: String,
    pub task: String,
    pub npc: String,
    pub start_keyword: String,
    pub report_keyword: String,
    pub min_level: u32,
    pub max_level: Option<u32>,
    pub threshold: i64,
    pub experience: i64,
    pub cooldown_seconds: i64,
    pub grant_item: Option<u16>,
    pub grant_count: u16,
    pub storage_prefix: String,
    pub source: Vec<String>,
}

pub fn load() -> Result<Vec<Task>, String> {
    let tasks: Vec<Task> = serde_json::from_str(EMBEDDED).map_err(|e| e.to_string())?;
    let mut keys = std::collections::BTreeSet::new();
    if tasks.len() != 12 {
        return Err("Spike requires twelve tasks".into());
    }
    for task in &tasks {
        if !keys.insert((&task.tier, &task.task))
            || task.threshold < 1
            || task.experience < 1
            || task.cooldown_seconds != 72000
            || task.max_level.is_some_and(|level| level < task.min_level)
            || task.grant_item.is_some() != (task.grant_count > 0)
            || task.source.is_empty()
            || task.source.iter().any(|source| {
                !source
                    .starts_with("zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:")
            })
        {
            return Err(format!("invalid Spike task {}/{}", task.tier, task.task));
        }
    }
    Ok(tasks)
}

#[derive(Debug, PartialEq, Eq)]
pub struct TaskCompletion {
    pub main: i64,
    pub daily: i64,
    pub fame: i64,
    pub experience: i64,
}

impl Task {
    #[must_use]
    pub fn can_offer(&self, main: i64, daily: i64, level: u32, now: i64) -> bool {
        main == -1
            && daily < now
            && level >= self.min_level
            && self.max_level.is_none_or(|maximum| level <= maximum)
    }
    #[must_use]
    pub fn complete(&self, main: i64, fame: i64, now: i64) -> Option<TaskCompletion> {
        if main != self.threshold {
            return None;
        }
        // NPC report branches have no level check. The cooldown starts at report,
        // not acceptance; Player.addFamePoint clamps the unset -1 to zero.
        Some(TaskCompletion {
            main: -1,
            daily: now.checked_add(self.cooldown_seconds)?,
            fame: fame.max(0).checked_add(1)?,
            experience: self.experience,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct CounterChange {
    pub main: i64,
    pub consume_tool: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub struct ShovelChange {
    pub main: i64,
    pub consume_tool: bool,
    pub monster: Option<&'static str>,
}
#[derive(Debug, PartialEq, Eq)]
pub struct DetectorChange {
    pub main: i64,
    pub target: Option<Position>,
    pub consume_tool: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub struct MonolithChange {
    pub main: i64,
    pub tool_replacement: Option<u16>,
    pub scatter: bool,
    pub magic_effect: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub struct NestChange {
    pub main: i64,
    pub monster: Option<&'static str>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Geomantic {
    Bounce,
    Apply {
        outfit_seconds: Option<u16>,
        charge: Option<i64>,
    },
}

#[must_use]
pub fn pacifier(main: i64, target: u16) -> Option<CounterChange> {
    // actions_tuning_fork.lua:3-21; target replacement is 19213 plus decay.
    if main == -1 || main == 7 || target != 19208 {
        return None;
    }
    let next = main.checked_add(1)?;
    Some(CounterChange {
        main: next,
        consume_tool: next == 7,
    })
}
#[must_use]
pub fn shovel(main: i64, target: u16, roll: u8) -> Option<ShovelChange> {
    // actions_spirit_shovel.lua:1-38; the first interval includes both 90 and 100.
    if main == -1 || main == 1 || target != 19211 || !(1..=100).contains(&roll) {
        return None;
    }
    let monster = match roll {
        90..=100 => Some("Enraged Soul"),
        80..=89 => Some("Ghost"),
        70..=79 => Some("Demon Skeleton"),
        50..=69 => Some("Zombie"),
        _ => None,
    };
    let next = if roll >= 90 {
        main.checked_add(1)?
    } else {
        main
    };
    Some(ShovelChange {
        main: next,
        consume_tool: roll >= 90 && next == 1,
        monster,
    })
}
#[must_use]
pub fn detector_found(player: Position, target: Position) -> bool {
    // actions_ghost_detector.lua:82-87; square tolerance, not Euclidean radius.
    player.x.abs_diff(target.x) <= 4
        && player.y.abs_diff(target.y) <= 4
        && player.floor == target.floor
}
#[must_use]
pub fn detector_advance(main: i64, xy: (u16, u16)) -> Option<DetectorChange> {
    // actions_ghost_detector.lua:88-101. The caller supplies a sampled next target;
    // the final success must not sample or retain another target.
    if main == 2 {
        return Some(DetectorChange {
            main: 3,
            target: None,
            consume_tool: true,
        });
    }
    if !(0..=1).contains(&main)
        || !(32171..=32352).contains(&xy.0)
        || !(32512..=32671).contains(&xy.1)
    {
        return None;
    }
    let next = main + 1;
    Some(DetectorChange {
        main: next,
        target: Some(Position {
            x: xy.0,
            y: xy.1,
            floor: -8 - i8::try_from(next).ok()?,
        }),
        consume_tool: false,
    })
}
#[must_use]
pub fn detector_hint(from: Position, to: Position) -> String {
    // actions_ghost_detector.lua:10-64. Native negative floors reverse donor z.
    let dx = i32::from(from.x) - i32::from(to.x);
    let dy = i32::from(from.y) - i32::from(to.y);
    let level = if from.floor < to.floor {
        0
    } else if from.floor > to.floor {
        1
    } else {
        2
    };
    if dx.abs() < 5 && dy.abs() < 5 {
        return ["above you", "below you", "next to you"][level].into();
    }
    let tangent = if dx == 0 {
        10.0
    } else {
        f64::from(dy) / f64::from(dx)
    };
    let direction = if tangent.abs() < 0.4142 {
        if dx > 0 { "west" } else { "east" }
    } else if tangent.abs() < 2.4142 {
        if tangent > 0.0 {
            if dy > 0 { "north-west" } else { "south-east" }
        } else if dx > 0 {
            "south-west"
        } else {
            "north-east"
        }
    } else if dy > 0 {
        "north"
    } else {
        "south"
    };
    let distance = i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy);
    let prefix = if distance < 10000 {
        [
            "on a higher level to the ",
            "on a lower level to the ",
            "to the ",
        ][level]
    } else if distance < 75625 {
        "far to the "
    } else {
        "very far to the "
    };
    format!("{prefix}{direction}")
}
#[must_use]
pub fn monolith(main: i64, tool: u16, target: u16, roll: u8) -> Option<MonolithChange> {
    // actions_lode_stone.lua:9-49. In particular -1 is accepted, and the fresh
    // tool resets progress to one. Every accepted monolith becomes 19379+decay.
    if main == 3 || target != 19217 {
        return None;
    }
    let (next, replacement) = match tool {
        19207 => (1, Some(19216)),
        19216 => ((if main == -1 { 0 } else { main }).checked_add(1)?, None),
        _ => return None,
    };
    if tool == 19216 && next == 3 {
        return Some(MonolithChange {
            main: 3,
            tool_replacement: Some(19218),
            scatter: false,
            magic_effect: false,
        });
    }
    if !(1..=100).contains(&roll) {
        return None;
    }
    Some(MonolithChange {
        main: next,
        tool_replacement: replacement,
        scatter: roll > 60,
        magic_effect: true,
    })
}
#[must_use]
pub fn fertilizer(main: i64, target: u16, visited: bool) -> Option<CounterChange> {
    // actions_fertilizer.lua:6-31. Persist the position in the transient visited
    // set only after the operation succeeds; this rule does not reset that set.
    if main == -1 || main == 4 || target != 19215 || visited {
        return None;
    }
    let next = main.checked_add(1)?;
    Some(CounterChange {
        main: next,
        consume_tool: next == 4,
    })
}
#[must_use]
pub fn nest(main: i64, look_type: u16, roll: u8, choice: u8) -> Option<NestChange> {
    // actions_nests.lua:4-28. The action and report stop at five, despite the
    // transformer's distinct eight check and the introductory NPC sentence.
    if main == -1 || main == 5 || look_type != 307 || !(1..=100).contains(&roll) {
        return None;
    }
    let monster = if roll > 60 {
        Some(*["Spider", "Larva", "Scarab", "Tarantula"].get(usize::from(choice).checked_sub(1)?)?)
    } else {
        None
    };
    Some(NestChange {
        main: main.checked_add(1)?,
        monster,
    })
}
#[must_use]
pub fn geomantic(nest: i64, charge: i64, outfit: bool, look_type: u16) -> Geomantic {
    // the_spike_tasks/movements_geomantic_charges.lua:13-38. Outfit rejection
    // occurs before the charge branch and therefore suppresses both effects.
    let nest_active = nest != -1 && nest != 8;
    let charge_active = charge == 1;
    if (!nest_active && !charge_active) || (nest_active && (outfit || look_type == 307)) {
        return Geomantic::Bounce;
    }
    Geomantic::Apply {
        outfit_seconds: nest_active.then_some(120),
        charge: charge_active.then_some(2),
    }
}
#[must_use]
pub fn hottest_pool(main: i64, selection: u8, position: Position) -> bool {
    // actions_thermometer.lua:1-10. The caller retains ONE shared world selection,
    // not one random choice per use or per character.
    let pools = [
        ((32263, 32481, -13), (32274, 32490, -13)),
        ((32163, 32558, -13), (32173, 32567, -13)),
        ((32201, 32667, -13), (32211, 32672, -13)),
        ((32135, 32606, -14), (32143, 32614, -14)),
        ((32330, 32519, -14), (32339, 32521, -14)),
        ((32260, 32697, -14), (32272, 32705, -14)),
        ((32176, 32493, -15), (32186, 32502, -15)),
        ((32341, 32577, -15), (32347, 32586, -15)),
        ((32220, 32643, -15), (32223, 32652, -15)),
    ];
    if main == -1 || main == 1 {
        return false;
    }
    selection
        .checked_sub(1)
        .and_then(|index| pools.get(usize::from(index)))
        .is_some_and(|(from, to)| inside(position, *from, *to))
}
#[must_use]
pub fn kill_in_area(tier: &str, position: Position) -> bool {
    // creaturescripts_{upper,middle,lower}_spike_kill.lua:3-6. This is only the
    // spatial predicate; retain most-damage + active-shared-XP party selection.
    let (from, to) = match tier {
        "upper" => ((32008, 32522, -8), (32365, 32759, -10)),
        "middle" => ((32100, 32470, -11), (32380, 32725, -12)),
        "lower" => ((32120, 32470, -13), (32345, 32710, -15)),
        _ => return false,
    };
    inside(position, from, to)
}
#[must_use]
pub fn contact(main: i64, threshold: i64, visited: bool) -> Option<i64> {
    // gnome_trooper.lua:49-105 and the nine undercover greet callbacks:51-73.
    if main == -1 || main == threshold || visited {
        return None;
    }
    main.checked_add(1)
}

fn inside(position: Position, from: (u16, u16, i8), to: (u16, u16, i8)) -> bool {
    (from.0..=to.0).contains(&position.x)
        && (from.1..=to.1).contains(&position.y)
        && (from.2.min(to.2)..=from.2.max(to.2)).contains(&position.floor)
}

#[cfg(test)]
#[path = "spike_tasks/tests.rs"]
mod tests;
