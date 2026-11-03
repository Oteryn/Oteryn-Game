//! The three officers retain one donor talkState and the original branch order.
use super::*;
use crate::quest::dark_trails::message_contains as matches;
use std::collections::BTreeMap;

pub const PREFIX: &str = "oteryn:quest-progress/quest/u10_20/spike_task_quest/";
pub const FAME: &str = "oteryn:quest-progress/quest/u10_20/spike_task_quest/spike_fame_points";
pub const EMBEDDED_DIALOGUE: &str =
    include_str!("../../../../../../content/dialogues/quests/spike-tasks.json");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reply {
    pub kind: String,
    pub lines: Vec<String>,
    pub source: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Info {
    pub keyword: String,
    pub reply: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskText {
    pub task: String,
    pub topic: String,
    pub cooldown: String,
    pub level: String,
    pub offer: String,
    pub started: String,
    pub accept: String,
    pub decline: String,
    pub report_absent: String,
    pub report_success: String,
    pub report_incomplete: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Officer {
    pub npc: String,
    pub tier: String,
    pub info: Vec<Info>,
    pub mission_refusal: String,
    pub mission_offer: String,
    pub report_prompt: String,
    pub invalid_report: String,
    pub greeting: String,
    pub tasks: Vec<TaskText>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contact {
    pub npc: String,
    pub reply: String,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParcelReply {
    pub state: u8,
    pub reply: String,
}

pub struct Book {
    pub tasks: Vec<Task>,
    pub replies: BTreeMap<String, Reply>,
    pub officers: Vec<Officer>,
    pub contacts: Vec<Contact>,
    pub parcel_replies: Vec<ParcelReply>,
}

pub struct Facts<'a> {
    pub tracks: &'a BTreeMap<String, i64>,
    pub level: u32,
    pub now: i64,
}

#[derive(Default, Debug)]
pub struct Plan {
    pub next_topic: Option<String>,
    pub reply: Vec<String>,
    pub starts: Vec<String>,
    pub report: Option<String>,
    pub needs_inventory: bool,
}

#[must_use]
pub fn main_track(task: &Task) -> String {
    format!("{PREFIX}spike_{}_{}_main", task.tier, task.task)
}

#[must_use]
pub fn report_track(task: &Task) -> String {
    format!("{PREFIX}spike_{}_{}_reported_at", task.tier, task.task)
}

/// Native SET_NOW stores the report instant. Projection to Crystal's absolute
/// deadline is exact: Daily=reported_at+72000; an unset timestamp stays unset.
#[must_use]
pub fn source_daily(task: &Task, tracks: &BTreeMap<String, i64>) -> Option<i64> {
    let reported = tracks.get(&report_track(task)).copied().unwrap_or(-1);
    if reported == -1 {
        Some(-1)
    } else {
        reported.checked_add(task.cooldown_seconds)
    }
}

pub fn load() -> Result<Book, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Document {
        schema: String,
        replies: BTreeMap<String, Reply>,
        officers: Vec<Officer>,
        contacts: Vec<Contact>,
        parcel_replies: Vec<ParcelReply>,
    }
    let doc: Document = serde_json::from_str(EMBEDDED_DIALOGUE).map_err(|e| e.to_string())?;
    if doc.schema != "OTERYN_SPIKE_DIALOGUE_V1"
        || doc.officers.len() != 3
        || doc.contacts.len() != 9
        || doc.parcel_replies.len() != 4
    {
        return Err("Spike dialogue bounds".into());
    }
    let book = Book {
        tasks: super::load()?,
        replies: doc.replies,
        officers: doc.officers,
        contacts: doc.contacts,
        parcel_replies: doc.parcel_replies,
    };
    for (key, reply) in &book.replies {
        if !["static", "wait_time", "level_range", "minimum_level"].contains(&reply.kind.as_str())
            || reply.lines.is_empty()
            || reply.lines.iter().any(String::is_empty)
            || !reply
                .source
                .starts_with("zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:")
        {
            return Err(format!("invalid Spike reply {key}"));
        }
    }
    for officer in &book.officers {
        if officer.tasks.len() != 4
            || book
                .tasks
                .iter()
                .filter(|task| task.tier == officer.tier)
                .count()
                != 4
        {
            return Err("Spike officer tasks".into());
        }
        let mut refs = vec![
            &officer.mission_refusal,
            &officer.mission_offer,
            &officer.report_prompt,
            &officer.invalid_report,
            &officer.greeting,
        ];
        refs.extend(officer.info.iter().map(|info| &info.reply));
        for text in &officer.tasks {
            if !book
                .tasks
                .iter()
                .any(|task| task.tier == officer.tier && task.task == text.task)
            {
                return Err("Spike task text identity".into());
            }
            refs.extend([
                &text.cooldown,
                &text.level,
                &text.offer,
                &text.started,
                &text.accept,
                &text.decline,
                &text.report_absent,
                &text.report_success,
                &text.report_incomplete,
            ]);
        }
        if refs.into_iter().any(|key| !book.replies.contains_key(key)) {
            return Err("missing Spike dialogue reference".into());
        }
    }
    if book
        .contacts
        .iter()
        .any(|contact| !book.replies.contains_key(&contact.reply))
        || book
            .parcel_replies
            .iter()
            .any(|reply| reply.state > 3 || !book.replies.contains_key(&reply.reply))
    {
        return Err("Spike contact dialogue reference".into());
    }
    Ok(book)
}

impl Book {
    fn render(&self, key: &str, task: &Task, remaining: i64) -> Option<Vec<String>> {
        let reply = self.replies.get(key)?;
        if reply.kind == "static" {
            return Some(reply.lines.clone());
        }
        let value = match reply.kind.as_str() {
            "wait_time" => reply
                .lines
                .first()?
                .replace("{remaining}", &time_text(remaining)?),
            "level_range" => reply
                .lines
                .first()?
                .replace("{minimum}", &task.min_level.to_string())
                .replace("{maximum}", &task.max_level?.to_string()),
            "minimum_level" => reply
                .lines
                .first()?
                .replace("{minimum}", &task.min_level.to_string()),
            _ => return None,
        };
        Some(vec![value])
    }

    #[must_use]
    pub fn task(&self, tier: &str, name: &str) -> Option<&Task> {
        self.tasks
            .iter()
            .find(|task| task.tier == tier && task.task == name)
    }

    #[must_use]
    pub fn respond(
        &self,
        npc: &str,
        topic: Option<&str>,
        message: &str,
        facts: &Facts<'_>,
    ) -> Option<Plan> {
        let officer = self.officers.iter().find(|officer| officer.npc == npc)?;
        let first = self.task(&officer.tier, &officer.tasks.first()?.task)?;
        let mut plan = Plan {
            next_topic: topic.map(str::to_string),
            ..Plan::default()
        };
        for info in &officer.info {
            if matches(message, &info.keyword) {
                plan.reply = self.render(&info.reply, first, 0)?;
                return Some(plan);
            }
        }
        if matches(message, "mission") {
            // gnomilly/gnombold only reject above their maximum in this menu;
            // the actual offer below checks both ends. Gnomargery checks minimum.
            let refused = first
                .max_level
                .map_or(facts.level < first.min_level, |max| facts.level > max);
            plan.reply = self.render(
                if refused {
                    &officer.mission_refusal
                } else {
                    &officer.mission_offer
                },
                first,
                0,
            )?;
            return Some(plan);
        }
        if matches(message, "report") {
            plan.next_topic = Some("report".into());
            plan.reply = self.render(&officer.report_prompt, first, 0)?;
            return Some(plan);
        }
        if topic == Some("report") {
            plan.next_topic = None;
            for text in &officer.tasks {
                let task = self.task(&officer.tier, &text.task)?;
                if matches(message, &task.report_keyword) {
                    let main = facts.tracks.get(&main_track(task)).copied().unwrap_or(-1);
                    let reply = if main == -1 {
                        &text.report_absent
                    } else if main == task.threshold {
                        plan.report = Some(task.task.clone());
                        &text.report_success
                    } else {
                        &text.report_incomplete
                    };
                    plan.reply = self.render(reply, task, 0)?;
                    return Some(plan);
                }
            }
            plan.reply = self.render(&officer.invalid_report, first, 0)?;
            return Some(plan);
        }
        // These are separate source if blocks, not an elseif chain: a single
        // utterance may both offer and accept, or start two tasks in source order.
        for text in &officer.tasks {
            let task = self.task(&officer.tier, &text.task)?;
            if matches(message, &task.start_keyword) {
                let daily = source_daily(task, facts.tracks)?;
                if daily >= facts.now {
                    plan.reply.extend(self.render(
                        &text.cooldown,
                        task,
                        daily.checked_sub(facts.now)?,
                    )?);
                    return Some(plan);
                }
                if facts.level < task.min_level
                    || task.max_level.is_some_and(|max| facts.level > max)
                {
                    plan.reply.extend(self.render(&text.level, task, 0)?);
                    return Some(plan);
                }
                if facts.tracks.get(&main_track(task)).copied().unwrap_or(-1) == -1 {
                    plan.reply.extend(self.render(&text.offer, task, 0)?);
                    plan.next_topic = Some(text.topic.clone());
                } else {
                    plan.reply.extend(self.render(&text.started, task, 0)?);
                }
            }
            if plan.next_topic.as_deref() == Some(text.topic.as_str()) {
                if matches(message, "yes") {
                    plan.starts.push(task.task.clone());
                    plan.needs_inventory |= task.grant_item.is_some();
                    plan.reply.extend(self.render(&text.accept, task, 0)?);
                    plan.next_topic = None;
                } else if matches(message, "no") {
                    plan.reply.extend(self.render(&text.decline, task, 0)?);
                    plan.next_topic = None;
                }
            }
        }
        (!plan.reply.is_empty()).then_some(plan)
    }
}

fn time_text(seconds: i64) -> Option<String> {
    if seconds < 0 {
        return None;
    }
    // Crystal201189f data/libs/functions/string.lua:38-59, including an empty
    // duration at exact deadline and the source's comma/and formatting.
    let values = [
        seconds / 86400,
        seconds / 3600 % 24,
        seconds / 60 % 60,
        seconds % 60,
    ];
    let mut text = String::new();
    for (index, (name, value)) in ["day", "hour", "minute", "second"]
        .into_iter()
        .zip(values)
        .enumerate()
    {
        if value == 0 {
            continue;
        }
        if index == 3 {
            text.push_str(" and ");
        } else if !text.is_empty() {
            text.push_str(", ");
        }
        text.push_str(&format!(
            "{value} {name}{}",
            if value == 1 { "" } else { "s" }
        ));
    }
    if text.len() < 16 && text.contains("second") {
        text = text.strip_prefix(" and ")?.to_string();
    }
    Some(text)
}

#[cfg(test)]
mod tests;
