use super::super::{NpcRuleCatalog, NpcRuleError};

const DOC: &str = include_str!(
    "../../../../../../content/dialogues/quest-rules-supplemental/questpc2-gnomus.json"
);

#[test]
fn gnomus_exact_retained_source_program_is_admitted() -> Result<(), Box<dyn std::error::Error>> {
    NpcRuleCatalog::from_json_documents(&[DOC])?;
    Ok(())
}

#[test]
fn gnomus_source_custody_rejects_other_hash_npc_and_reference()
-> Result<(), Box<dyn std::error::Error>> {
    let original: serde_json::Value = serde_json::from_str(DOC)?;
    for case in 0..5 {
        let mut document = original.clone();
        match case {
            0 => document["source_files"][0]["sha256"] = "0".repeat(64).into(),
            1 => document["rules"][0]["npc"] = "oteryn:npc.mr_morris".into(),
            2 => document["rules"][0]["sources"][0] =
                "crystal:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/npc/pemaret.lua:102"
                    .into(),
            3 => {
                document["source_files"][0]["revision"] =
                    "201189f871d23597b82caf88fd611454a2ddefb8".into()
            }
            _ => document["source_files"][0]["path"] = "data-global/npc/pemaret.lua".into(),
        }
        let text = serde_json::to_string(&document)?;
        assert!(
            matches!(NpcRuleCatalog::from_json_documents(&[&text]),
            Err(NpcRuleError::Invalid(message)) if message == "Gnomus source custody"),
            "source custody case {case}"
        );
    }
    Ok(())
}

const MEASUREMENTS: &str = include_str!(
    "../../../../../../content/dialogues/quest-rules/quest-batch3-qn3-dangerous_depths.json"
);
const PREFIX: &str = "oteryn:quest-progress/quest/u11_50/dangerous_depths/";

#[test]
fn gnomus_measurements_and_ordnance_keep_their_distinct_exact_source_custody()
-> Result<(), Box<dyn std::error::Error>> {
    NpcRuleCatalog::from_json_documents(&[MEASUREMENTS])?;
    for documents in [[MEASUREMENTS, DOC], [DOC, MEASUREMENTS]] {
        let catalogue = NpcRuleCatalog::from_json_documents(&documents)?;
        assert_eq!(catalogue.programs().len(), 2);
    }
    let original: serde_json::Value = serde_json::from_str(MEASUREMENTS)?;
    for case in 0..10 {
        let mut document = original.clone();
        match case {
            0 => document["source_files"][0]["sha256"] = "0".repeat(64).into(),
            1 => document["rules"][0]["npc"] = "oteryn:npc.mr_morris".into(),
            2 => document["rules"][0]["sources"][0] =
                "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/pemaret.lua:88-97"
                    .into(),
            3 => document["source_files"][0]["revision"] =
                "9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d".into(),
            4 => document["source_files"][0]["path"] = "data-global/npc/pemaret.lua".into(),
            5 => document["program_key"] = super::PROGRAM.into(),
            6 => document["rules"][0]["guards"] = serde_json::json!([]),
            7 => document["rules"][0]["effects"][1]["count"] = 2.into(),
            8 => document["rules"][0]["any_message"] = true.into(),
            _ => document["rules"][0]["topics"] = serde_json::json!([22]),
        }
        let text = serde_json::to_string(&document)?;
        assert!(
            matches!(NpcRuleCatalog::from_json_documents(&[&text]),
            Err(NpcRuleError::Invalid(message)) if message == "Gnomus source custody"),
            "measurements source custody case {case}"
        );
    }
    Ok(())
}

fn selected_plan(
    reversed: bool,
    message: &str,
    topic: i32,
    measurements: i64,
    locations: i64,
    cooldown: i64,
    storage_only: bool,
) -> Result<Option<super::super::NpcRulePlan>, NpcRuleError> {
    use super::super::{NpcInventorySnapshot, NpcRuleInput, NpcRuleTrigger};
    use std::collections::BTreeMap;
    let documents = if reversed {
        [DOC, MEASUREMENTS]
    } else {
        [MEASUREMENTS, DOC]
    };
    let catalogue = NpcRuleCatalog::from_json_documents(&documents)?;
    let mut tracks: BTreeMap<_, _> = catalogue
        .programs()
        .iter()
        .flat_map(|program| &program.document.tracks)
        .map(|track| (track.key.clone(), track.initial))
        .collect();
    for (suffix, value) in [
        ("gnomes/measurements", measurements),
        ("gnomes/location_count", locations),
        ("gnomes/time_task_measurements", cooldown),
        ("gnomes/points", 0),
        ("questline", 0),
        ("gnomes/ordnance", 0),
    ] {
        tracks.insert(format!("{PREFIX}{suffix}"), value);
    }
    let inventory = NpcInventorySnapshot::default();
    let outfits = BTreeMap::new();
    let input = NpcRuleInput {
        npc: "oteryn:npc.gnomus",
        trigger: NpcRuleTrigger::Say,
        message,
        player_name: None,
        topic,
        tracks: &tracks,
        inventory: &inventory,
        now_epoch_seconds: 1_700_000_000,
        calendar_month: 11,
        premium_current: false,
        level: 100,
        vocation: "knight",
        sex: 0,
        outfits: &outfits,
    };
    if storage_only {
        catalogue.plan_storage(&input)
    } else {
        catalogue.plan(&input)
    }
}

#[test]
fn gnomus_task_selection_uses_source_guards_and_is_independent_of_document_order()
-> Result<(), Box<dyn std::error::Error>> {
    let source = NpcRuleCatalog::from_json_documents(&[MEASUREMENTS, DOC])?;
    // These are the accepted bounded programs: one selected plan per input.
    // The donor Lua has separate task/status blocks, not a single global if/elseif.
    for (message, topic, progress, locations, cooldown, expected, next, gift) in [
        ("measurements", 22, 1, 5, 0, "measurements-report", 1, 1),
        ("measurements", 1, 1, 5, 0, "measurements-report", 1, 1),
        ("measurements", 22, 2, 5, 10, "measurements-cooldown", 1, 0),
        ("measurements", 22, 1, 4, 0, "unsupported-task-0", 0, 0),
        ("measurements yes", 22, 1, 4, 0, "unsupported-task-0", 0, 0),
        (
            "measurements ordnance",
            22,
            1,
            5,
            0,
            "measurements-report",
            1,
            1,
        ),
        ("measurements yes", 22, 1, 5, 0, "measurements-report", 1, 1),
        (
            "status measurements",
            22,
            1,
            5,
            0,
            "measurements-report",
            1,
            1,
        ),
        (
            "gnomes measurements",
            1,
            1,
            5,
            0,
            "measurements-report",
            1,
            1,
        ),
        (
            "status measurements",
            22,
            1,
            4,
            0,
            "unsupported-task-0",
            0,
            0,
        ),
        ("ordnance", 1, 1, 5, 0, "initial-offer", 22, 0),
        ("ordnance yes", 22, 1, 5, 0, "initial-offer", 22, 0),
        ("yes", 22, 1, 5, 0, "accept-first", 1, 0),
    ] {
        let forward = selected_plan(false, message, topic, progress, locations, cooldown, false)?;
        let reversed = selected_plan(true, message, topic, progress, locations, cooldown, false)?;
        assert_eq!(
            forward, reversed,
            "document order changed {message} at topic {topic}"
        );
        let plan = forward.ok_or("source plan")?;
        assert_eq!(
            plan.selected_rule_keys,
            vec![format!(
                "oteryn:quest-npc/dangerous_depths/gnomus/{expected}"
            )]
        );
        assert_eq!(plan.next_topic, next);
        assert_eq!(
            plan.inventory_after.items.values().copied().sum::<u32>(),
            gift
        );
        let measurements_owner = expected.starts_with("measurements-");
        let program = source
            .programs()
            .iter()
            .find(|program| program.document.program_key.is_none() == measurements_owner)
            .ok_or("actual source programme")?;
        assert_eq!(plan.program_digest, program.digest);
        if expected.starts_with("unsupported-") {
            assert!(plan.track_changes.is_empty());
        }
    }
    for reversed in [false, true] {
        assert!(
            matches!(
                selected_plan(reversed, "measurements", 22, 1, 5, 0, true),
                Err(NpcRuleError::MissingNativeResolution(_))
            ),
            "storage-only planning must not discard an unowned item effect and choose a reset"
        );
        assert!(selected_plan(reversed, "yes", 1, 1, 5, 0, false)?.is_none());
        let refused = selected_plan(reversed, "measurements", 22, 1, 4, 0, false)?
            .ok_or("unowned-task reset")?;
        assert_eq!(refused.next_topic, 0);
        assert!(
            selected_plan(reversed, "yes", refused.next_topic, 1, 4, 0, false)?.is_none(),
            "an unfinished measurements task must not retain ordnance confirmation"
        );
    }
    Ok(())
}
