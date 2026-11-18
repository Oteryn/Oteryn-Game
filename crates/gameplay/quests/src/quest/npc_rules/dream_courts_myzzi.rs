//! Crystal's focused Court-entry dialogue writes the existing Main.Questline track.
use super::{NpcRuleCatalog, NpcRuleError};
use serde_json::{Value, json};

pub(super) const PROGRAM: &str = "oteryn:quest-npc-program.dream-courts-myzzi";
pub(super) const QUEST: &str = "oteryn:quest.the_dream_courts_quest";
const TRACK: &str = "oteryn:quest-progress/quest/u12_00/the_dream_courts/main/questline";
const NPC: &str = "oteryn:npc.myzzi";
const FILE: &str = "data-global/npc/myzzi.lua";
const REVISION: &str = "201189f871d23597b82caf88fd611454a2ddefb8";

fn reference(lines: &str) -> String {
    format!("crystal:{REVISION}:{FILE}:{lines}")
}

pub(super) fn source() -> Result<NpcRuleCatalog, NpcRuleError> {
    let mut rules = Vec::new();
    for (trigger, reply, lines) in [
        ("greet", "Adventurer, it is good to see you.", "102"),
        ("farewell", "Farewell.", "103"),
        ("walkaway", "Farewell.", "104"),
    ] {
        rules.push(rule(trigger, trigger, reply, lines));
    }
    for (word, topic, next, reply, lines) in [
        (
            "good",
            None,
            2,
            "I'm looking for adventurers who can help my friends.",
            "68-70",
        ),
        (
            "help",
            Some(2),
            3,
            "Lady Alivar and Lord Cadion need help to avert a threat to the world.",
            "71-73",
        ),
        (
            "threat",
            Some(3),
            4,
            "I know little about it. I can grant access to the Courts, where Vanys and Undal can tell you more.",
            "74-79",
        ),
        (
            "courts",
            Some(4),
            5,
            "The entrances are in the mountains of Tyrsung and the meadows of Feyrist.",
            "80-82",
        ),
        (
            "entrances",
            Some(5),
            6,
            "Will you help my friends in their time of need?",
            "83-85",
        ),
        (
            "yes",
            Some(6),
            0,
            "Take my enchantment. You can now pass the Court portals.",
            "87-90",
        ),
    ] {
        let mut row = question(word, reply, lines, topic);
        row["keywords"] = json!([word]);
        row["effects"] = json!([{"kind":"set_topic", "value":next}]);
        if word == "yes" {
            row["effects"] = json!([
                {"kind":"set", "track":TRACK, "value":1},
                {"kind":"set_topic", "value":next}
            ]);
        }
        rules.push(row);
    }
    let mut unconfirmed = question("unconfirmed", "What do you mean?", "91-93", Some(6));
    unconfirmed["any_message"] = json!(true);
    unconfirmed["exclude_keywords"] = json!(["yes"]);
    rules.push(unconfirmed);

    let document = json!({
        "schema": 1,
        "program_key": PROGRAM,
        "quest": QUEST,
        "source_files": [{
            "path": FILE,
            "revision": REVISION,
            "sha256": "8a362f3651f4a15643cce9aad258195050d75ad6a3c3b79b8869d548bec27db1"
        }],
        "tracks": [{
            "key": TRACK, "quest": QUEST, "initial": -1, "min": -1, "max": 1,
            "sources": [reference("67-89")]
        }],
        "rules": rules
    });
    NpcRuleCatalog::from_json_documents(&[&document.to_string()])
}

fn rule(suffix: &str, trigger: &str, reply: &str, lines: &str) -> Value {
    json!({
        "key": format!("oteryn:quest-npc/the_dream_courts/myzzi/{suffix}"),
        "quest": QUEST,
        "npc": NPC,
        "trigger": trigger,
        "reply": [reply],
        "sources": [reference(lines)]
    })
}

fn question(suffix: &str, reply: &str, lines: &str, topic: Option<i32>) -> Value {
    let mut row = rule(suffix, "say", reply, lines);
    row["guards"] = json!([{"kind":"storage", "track":TRACK, "op":"lt", "value":1}]);
    if let Some(topic) = topic {
        row["topics"] = json!([topic]);
    }
    row
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quest::npc_rules::{NpcInventorySnapshot, NpcRuleInput, NpcRuleTrigger};
    use std::collections::BTreeMap;

    #[test]
    fn myzzi_grants_the_main_track_only_after_the_source_topic_sequence()
    -> Result<(), Box<dyn std::error::Error>> {
        let lowered = crate::quest::loader::load_embedded_quest_state("content-1")
            .map_err(|error| format!("{error:?}"))?;
        let catalogue = NpcRuleCatalog::runtime()?
            .admitted_for_npc(NPC, lowered.catalogue())?
            .ok_or("Myzzi programme is not admitted")?;
        let inventory = NpcInventorySnapshot::default();
        let tracks = BTreeMap::new();
        let outfits = BTreeMap::new();
        let mut input = NpcRuleInput {
            npc: NPC,
            trigger: NpcRuleTrigger::Say,
            message: "yes",
            player_name: None,
            topic: 0,
            tracks: &tracks,
            inventory: &inventory,
            now_epoch_seconds: 0,
            calendar_month: 1,
            premium_current: false,
            level: 1,
            vocation: "",
            sex: 0,
            outfits: &outfits,
        };
        assert!(catalogue.plan_storage(&input)?.is_none());
        for (message, next) in [
            ("good", 2),
            ("help", 3),
            ("threat", 4),
            ("courts", 5),
            ("entrances", 6),
        ] {
            input.message = message;
            let plan = catalogue.plan_storage(&input)?.ok_or("source topic")?;
            assert!(plan.track_changes.is_empty());
            assert_eq!(plan.next_topic, next);
            input.topic = next;
        }
        input.message = "no";
        let plan = catalogue
            .plan_storage(&input)?
            .ok_or("source confirmation fallback")?;
        assert_eq!(plan.next_topic, 6);
        assert!(plan.track_changes.is_empty());
        input.message = "yes";
        let plan = catalogue
            .plan_storage(&input)?
            .ok_or("source confirmation")?;
        assert_eq!(plan.track_changes.len(), 1);
        assert_eq!(
            (&*plan.track_changes[0].key, plan.track_changes[0].after),
            (TRACK, 1)
        );
        assert_eq!(plan.next_topic, 0);
        let mut completed = input;
        completed.topic = 6;
        let completed_tracks = BTreeMap::from([(TRACK.into(), 1)]);
        completed.tracks = &completed_tracks;
        assert!(catalogue.plan_storage(&completed)?.is_none());
        Ok(())
    }
}
