//! Independent pinned-Lua source witnesses, not earned quest or runtime-owner facts.
use super::*;
use serde_json::Value;
#[derive(Deserialize)]
struct Cases {
    cases: Vec<Case>,
}
#[derive(Deserialize)]
struct Case {
    npc: String,
    source_transition: String,
    negative: bool,
    message: String,
    topic: u16,
    before: Value,
    expected: Option<String>,
}
struct SourceFacts<'a> {
    program: &'a Program,
    case: &'a Case,
}
impl SourceFacts<'_> {
    fn number(&self, bucket: &str, key: &str, default: i64) -> i64 {
        self.case
            .before
            .get(bucket)
            .and_then(|v| v.get(key))
            .and_then(Value::as_i64)
            .unwrap_or(default)
    }
    fn boolean(&self, bucket: &str, key: &str) -> bool {
        self.case
            .before
            .get(bucket)
            .and_then(|v| v.get(key))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }
}
impl Facts for SourceFacts<'_> {
    fn now(&self) -> i64 {
        self.case.before["now"].as_i64().unwrap_or(0)
    }
    fn track(&self, storage: &str) -> Result<i64, Blocked> {
        self.program
            .binding(storage)
            .ok_or(Blocked::MissingSourceFact)?;
        Ok(self.number("tracks", storage, -1))
    }
    fn inventory(&self, item: u32) -> Result<u32, Blocked> {
        u32::try_from(self.number("inventory", &item.to_string(), 0))
            .map_err(|_| Blocked::ArithmeticBound)
    }
    fn premium(&self) -> Result<bool, Blocked> {
        self.case.before["premium"]
            .as_bool()
            .ok_or(Blocked::MissingSourceFact)
    }
    fn sex(&self) -> Sex {
        if self.case.before["sex"] == "female" {
            Sex::Female
        } else {
            Sex::Male
        }
    }
    fn owned_look_addon(&self, look: u32, addon: u8) -> Result<bool, Blocked> {
        Ok(self.boolean("looks", &format!("{look}:{addon}")))
    }
    fn kv_bool(&self, scope: &str, key: &str) -> Result<bool, Blocked> {
        Ok(self.boolean("kv", &format!("{scope}/{key}")))
    }
    fn level(&self) -> u32 {
        self.case.before["level"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .unwrap_or(0)
    }
    fn owner_points(&self, owner: &str) -> Result<i64, Blocked> {
        Ok(self.number("owner_points", owner, 0).max(0))
    }
    fn owned_mount(&self, id: u16) -> Result<bool, Blocked> {
        Ok(self.boolean("mounts", &id.to_string()))
    }
    fn selected_integer(&self, name: &str) -> Option<i64> {
        (name == "hazard_level")
            .then(|| {
                let bytes = self.case.message.as_bytes();
                let start = bytes.iter().position(u8::is_ascii_digit)?;
                let count = bytes[start..]
                    .iter()
                    .take_while(|b| b.is_ascii_digit())
                    .count();
                self.case.message[start..start + count].parse().ok()
            })
            .flatten()
    }
    fn hazard_can_select(&self, owner: &str, _selection: i64) -> Result<bool, Blocked> {
        Ok(self.boolean("hazard", owner))
    }
    fn world_storage(&self, id: u32, missing: i64) -> Result<i64, Blocked> {
        Ok(self.number("world_storage", &id.to_string(), missing))
    }
}
fn programme_for_test() -> Result<Program, String> {
    let bytes = match std::env::var_os("OTERYN_SRV7_NPC_POLICY_PROGRAM") {
        Some(path) => std::fs::read(path).map_err(|e| e.to_string())?,
        None => EMBEDDED.as_bytes().to_vec(),
    };
    Program::parse(&bytes).map_err(|e| format!("{e:?}"))
}
#[test]
fn srv7_source_oracle_all_452_policy_branches_and_negative_boundaries() -> Result<(), String> {
    let cases: Cases = serde_json::from_str(include_str!("source_program_policy_cases.json"))
        .map_err(|e| e.to_string())?;
    assert_eq!(cases.cases.len(), 904);
    let loaded_program = programme_for_test()?;
    let program = &loaded_program;
    let mut positives = std::collections::BTreeSet::new();
    let mut failures = Vec::new();
    for case in &cases.cases {
        let facts = SourceFacts { program, case };
        let trigger = if case.before["fresh_greeting"] == true {
            Trigger::Greet
        } else {
            Trigger::Say
        };
        let selected = program.choose_for_trigger(
            &case.npc,
            &case.message,
            case.topic,
            &facts,
            crate::quest::classic_talk::contains,
            trigger,
        );
        match selected {
            Ok(selected) if selected.map(|t| t.id.as_str()) == case.expected.as_deref() => {}
            Ok(selected) => failures.push(format!(
                "{}:{} negative={} expected={:?} actual={:?}",
                case.npc,
                case.source_transition,
                case.negative,
                case.expected,
                selected.map(|t| t.id.as_str())
            )),
            Err(error) => failures.push(format!(
                "{}:{} negative={} blocked={error:?}",
                case.npc, case.source_transition, case.negative
            )),
        }
        if !case.negative {
            positives.insert((case.npc.as_str(), case.source_transition.as_str()));
        }
    }
    assert_eq!(positives.len(), 452);
    if !failures.is_empty() {
        return Err(format!(
            "{} of904 policy cases failed: {}",
            failures.len(),
            failures
                .iter()
                .take(24)
                .cloned()
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }
    Ok(())
}
#[test]
fn srv7_numeric_source_keyword_capture_uses_actual_message_not_stale_integer_facts()
-> Result<(), String> {
    let loaded_program = programme_for_test()?;
    let program = &loaded_program;
    let case = Case {
        npc: "oteryn:npc.gnomadness".into(),
        source_transition: "numeric-input".into(),
        negative: true,
        message: "not a number".into(),
        topic: 1,
        before: serde_json::json!({"now":1800000000,"premium":true,"sex":"male","level":100,"selected":{"hazard_level":6},"hazard":{"hazard.gnomprona-gardens":true}}),
        expected: None,
    };
    let facts = SourceFacts {
        program,
        case: &case,
    };
    assert_eq!(facts.selected_integer("hazard_level"), None);
    let choice = program
        .choose(
            &case.npc,
            &case.message,
            case.topic,
            &facts,
            crate::quest::classic_talk::contains,
        )
        .map_err(|e| format!("{e:?}"))?
        .ok_or("source invalid-input refusal")?;
    assert_eq!(choice.id, "ripptor-select-invalid");
    assert_eq!(choice.topic_after, 2);
    assert!(
        choice.effects.is_empty(),
        "source invalidinput cannotspend/grant/select"
    );
    Ok(())
}
