//! Source branch order and topics. A held branch never returns success text or a transition.
use super::{Book, KV_PREFIX, PREFIX, transition_key};
use std::collections::BTreeMap;

// Crystal 201189f data/npclib/npc.lua:13-20. The authored keyword set is literal
// ASCII. A prefixed occurrence anywhere rejects a message, even if another
// occurrence has a word boundary; Lua %w does not include underscore.
fn contains(message: &str, keyword: &str) -> bool {
    if keyword.is_empty() {
        return false;
    }
    let message = message.to_ascii_lowercase();
    let keyword = keyword.to_ascii_lowercase();
    let mut found = false;
    for (offset, candidate) in message.as_bytes().windows(keyword.len()).enumerate() {
        if candidate != keyword.as_bytes() {
            continue;
        }
        found = true;
        if offset > 0 && message.as_bytes()[offset - 1].is_ascii_alphanumeric() {
            return false;
        }
    }
    found
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Decision {
    pub transition: Option<String>,
    pub next_topic: Option<u16>,
    pub lines: Vec<String>,
    pub held: bool,
}

impl Decision {
    fn held() -> Self {
        Self {
            held: true,
            ..Self::default()
        }
    }
}

impl Book {
    fn say(
        &self,
        npc: &str,
        line: u16,
        topic: Option<u16>,
        transition: Option<&str>,
    ) -> Option<Decision> {
        Some(Decision {
            transition: transition.map(transition_key),
            next_topic: topic,
            lines: self.reply(&format!("{npc}:{line}"))?.lines.clone(),
            held: false,
        })
    }

    #[must_use]
    pub fn respond(
        &self,
        npc: &str,
        message: &str,
        topic: u16,
        tracks: &BTreeMap<String, i64>,
    ) -> Option<Decision> {
        let npc = npc.strip_prefix("oteryn:npc.")?;
        if !matches!(
            npc,
            "rapanaio" | "rapanaio_boat" | "rapanaio_isle_of_evil" | "scutty" | "mortimer"
        ) {
            return None;
        }
        // The dedicated Rapanaio KeywordHandler roots have one literal keyword;
        // source KeywordNode uses substring matching, without MsgContains' prefix rule.
        let lower = message.to_ascii_lowercase();
        if let Some(keyword) = self
            .text
            .static_keywords
            .iter()
            .find(|keyword| keyword.npc == npc && lower.contains(&keyword.keyword))
        {
            return Some(Decision {
                lines: self.reply(&keyword.reply)?.lines.clone(),
                ..Decision::default()
            });
        }
        let read = |suffix| {
            tracks
                .get(&format!("{PREFIX}{suffix}"))
                .copied()
                .unwrap_or(0)
        };
        let kv = |key| {
            tracks
                .get(&format!("{KV_PREFIX}{key}"))
                .copied()
                .unwrap_or(0)
        };
        let has = |word| contains(message, word);
        match npc {
            "rapanaio" => self.rapanaio(message, topic, read("questline")),
            "rapanaio_boat" => {
                if has("evil isle") {
                    return self.say(
                        npc,
                        if kv("rapanaio-travel") == 1 { 79 } else { 82 },
                        Some(if kv("rapanaio-travel") == 1 { 9 } else { 0 }),
                        None,
                    );
                }
                if has("mission") && topic == 0 {
                    return self.say(
                        npc,
                        if read("mission06") == 1 { 94 } else { 90 },
                        (read("mission06") == 1).then_some(1),
                        None,
                    );
                }
                if has("yes") && (1..=8).contains(&topic) {
                    let line = match topic {
                        1 => 104,
                        2 => 110,
                        3 => 116,
                        4 => 119,
                        5 => 122,
                        6 => 125,
                        7 => 128,
                        // Mission06, Questline and teleport are one compound source action.
                        _ => return Some(Decision::held()),
                    };
                    return self.say(npc, line, Some(topic + 1), None);
                }
                // Source topic9 is outside the enclosing 1..8 test (boat.lua:102,137).
                // Do not activate its unreachable teleport without the travel owner.
                if has("no") && topic >= 1 {
                    return self.say(npc, 146, Some(0), None);
                }
                None
            }
            "rapanaio_isle_of_evil" => {
                if has("mission") {
                    return match (read("questline"), read("mission07")) {
                        (13, _) => self.say(npc, 85, Some(0), Some("begin-final-mission")),
                        (14, 1) => self.say(npc, 91, Some(0), None),
                        (14, 2) => self.say(npc, 94, Some(1), Some("report-evil-master")),
                        _ => None,
                    };
                }
                if has("back") && topic == 1 {
                    return self.say(npc, 106, Some(2), None);
                }
                if has("yes") && topic == 2 {
                    return Some(Decision::held());
                }
                None
            }
            "scutty" => {
                // Shadows of Yalahar owns the preceding source branch. Refuse a mixed
                // message instead of guessing its foreign guard or taking its topic.
                if has("research notes") {
                    return Some(Decision::held());
                }
                if has("machine") && read("mission04") == 1 {
                    return self.say(npc, 74, Some(1), None);
                }
                if has("mission") && read("mission04") == 2 {
                    // donor bug fix: the referenced legacy constants are absent from
                    // storages.lua:1573-1585. Use the exact existing scoped boolean key
                    // in actions_heated_worm_punisher.lua:3-5,56, not an invented ID.
                    // QUESTION https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589174504
                    // ANSWER https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728 (3).
                    return self.say(
                        npc,
                        if kv("mission04-mustpay") == 1 { 82 } else { 85 },
                        Some(if kv("mission04-mustpay") == 1 { 3 } else { 0 }),
                        None,
                    );
                }
                if has("mission") && read("mission04") == 3 {
                    return Some(Decision::held());
                }
                if has("yes") {
                    return match topic {
                        1 => self.say(npc, 96, Some(2), None),
                        2 | 3 => Some(Decision::held()),
                        _ => None,
                    };
                }
                if has("no") && topic == 3 {
                    return self.say(npc, 126, Some(0), None);
                }
                None
            }
            "mortimer" => {
                if !has("nautical map") || read("mission05") != 1 {
                    return None;
                }
                // Earlier foreign branches / static roots may win such a mixed message.
                // Their unknown guards stay outside this batch; do not bypass them.
                if topic != 0
                    || self
                        .data
                        .mortimer_prior_keywords
                        .iter()
                        .any(|keyword| has(keyword))
                    || self
                        .data
                        .mortimer_static_keywords
                        .iter()
                        .any(|keyword| lower.contains(keyword))
                {
                    return Some(Decision::held());
                }
                self.say(npc, 627, Some(0), Some("mortimer-map"))
            }
            _ => None,
        }
    }

    fn rapanaio(&self, message: &str, old_topic: u16, questline: i64) -> Option<Decision> {
        let has = |word| contains(message, word);
        let mut initial = if has("mission") {
            match questline {
                value if value < 1 => self.say("rapanaio", 57, Some(1), None),
                1 => self.say("rapanaio", 60, Some(8), None),
                2 => self.say("rapanaio", 77, Some(9), None),
                3 => self.say("rapanaio", 80, Some(0), Some("mission-3")),
                4 => self.say("rapanaio", 90, Some(10), None),
                5 => self.say("rapanaio", 93, Some(0), Some("mission-5")),
                6 => self.say("rapanaio", 98, Some(11), None),
                7 => self.say("rapanaio", 101, Some(0), Some("mission-7")),
                8 => self.say("rapanaio", 109, Some(12), None),
                9 => self.say("rapanaio", 112, Some(0), Some("mission-9")),
                10 => self.say("rapanaio", 127, Some(13), None),
                11 => return self.say("rapanaio", 130, Some(0), Some("prepare-departure")),
                _ => None,
            }
        } else {
            None
        };
        let topic = initial
            .as_ref()
            .and_then(|plan| plan.next_topic)
            .unwrap_or(old_topic);
        let after = if has("fight") && topic == 1 {
            self.say("rapanaio", 142, Some(2), None)
        } else if has("evil") && topic == 2 {
            self.say("rapanaio", 149, Some(3), None)
        } else if has("hunt") && topic == 3 {
            self.say("rapanaio", 155, Some(4), None)
        } else if has("ridicule") && topic == 4 {
            self.say("rapanaio", 162, Some(5), None)
        } else if has("truth") && topic == 5 {
            self.say("rapanaio", 165, Some(6), None)
        } else if has("isle of evil") && topic == 6 {
            self.say("rapanaio", 172, Some(7), None)
        } else if has("yes") && topic == 7 {
            self.say("rapanaio", 183, Some(0), Some("accept-fight"))
        } else if has("yes") && (8..=13).contains(&topic) {
            Some(Decision::held())
        } else {
            None
        };
        match after {
            Some(next) if next.held => Some(next),
            Some(mut next) => {
                if let Some(first) = initial.take() {
                    let mut lines = first.lines;
                    lines.extend(next.lines);
                    next.lines = lines;
                    next.transition = next.transition.or(first.transition);
                }
                Some(next)
            }
            None => initial,
        }
    }
}
