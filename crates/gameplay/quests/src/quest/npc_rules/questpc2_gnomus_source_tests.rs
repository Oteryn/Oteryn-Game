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

const LEGACY: &str = include_str!(
    "../../../../../../content/dialogues/quest-rules/quest-batch3-qn3-dangerous_depths.json"
);

#[test]
fn gnomus_measurements_and_ordnance_keep_their_distinct_source_custody()
-> Result<(), Box<dyn std::error::Error>> {
    let catalogue = NpcRuleCatalog::from_json_documents(&[LEGACY, DOC])?;
    assert_eq!(catalogue.programs.len(), 2);
    assert!(catalogue.programs[0].document.program_key.is_none());
    assert_eq!(
        catalogue.programs[1].document.program_key.as_deref(),
        Some(super::PROGRAM)
    );
    Ok(())
}

#[test]
fn gnomus_legacy_custody_rejects_cross_program_and_changed_source()
-> Result<(), Box<dyn std::error::Error>> {
    let original: serde_json::Value = serde_json::from_str(LEGACY)?;
    for case in 0..8 {
        let mut document = original.clone();
        match case {
            0 => document["program_key"] = super::PROGRAM.into(),
            1 => {
                document["source_files"][0]["revision"] =
                    "9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d".into()
            }
            2 => document["source_files"][0]["path"] = "data-global/npc/pemaret.lua".into(),
            3 => document["source_files"][0]["sha256"] = "0".repeat(64).into(),
            4 => document["rules"][0]["npc"] = "oteryn:npc.mr_morris".into(),
            5 => {
                document["rules"][0]["sources"][0] =
                    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/pemaret.lua:88-97".into()
            }
            6 => document["source_files"] = serde_json::json!([]),
            _ => {
                document["tracks"][0]["sources"][0] =
                    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/gnomus.lua:119-176".into()
            }
        }
        let text = serde_json::to_string(&document)?;
        assert!(
            matches!(NpcRuleCatalog::from_json_documents(&[&text, DOC]),
                Err(NpcRuleError::Invalid(message)) if message == "Gnomus source custody"),
            "legacy source custody case {case}"
        );
    }
    Ok(())
}

#[test]
fn gnomus_supplement_cannot_drop_its_program_or_source_custody()
-> Result<(), Box<dyn std::error::Error>> {
    let original: serde_json::Value = serde_json::from_str(DOC)?;
    for case in 0..4 {
        let mut document = original.clone();
        match case {
            0 => document["program_key"] = serde_json::Value::Null,
            1 => document["source_files"] = serde_json::json!([]),
            2 => {
                document["program_key"] = serde_json::Value::Null;
                document["source_files"] = serde_json::json!([]);
            }
            _ => {
                document["program_key"] = serde_json::Value::Null;
                document["source_files"][0]["revision"] =
                    "201189f871d23597b82caf88fd611454a2ddefb8".into();
            }
        }
        let text = serde_json::to_string(&document)?;
        assert!(
            matches!(NpcRuleCatalog::from_json_documents(&[LEGACY, &text]),
                Err(NpcRuleError::Invalid(message)) if message == "Gnomus source custody"),
            "supplement source custody case {case}"
        );
    }
    Ok(())
}
