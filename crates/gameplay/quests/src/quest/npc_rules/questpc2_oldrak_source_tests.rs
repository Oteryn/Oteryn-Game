use super::super::{NpcRuleCatalog, NpcRuleError};

const DOC: &str = include_str!(
    "../../../../../../content/dialogues/quest-rules-supplemental/questpc2-oldrak.json"
);

#[test]
fn oldrak_exact_retained_source_program_is_admitted() -> Result<(), Box<dyn std::error::Error>> {
    NpcRuleCatalog::from_json_documents(&[DOC])?;
    Ok(())
}

#[test]
fn oldrak_source_custody_rejects_other_hash_npc_and_reference()
-> Result<(), Box<dyn std::error::Error>> {
    let original: serde_json::Value = serde_json::from_str(DOC)?;
    for case in 0..3 {
        let mut document = original.clone();
        match case {
            0 => document["source_files"][0]["sha256"] = "0".repeat(64).into(),
            1 => document["rules"][0]["npc"] = "oteryn:npc.mr_morris".into(),
            _ => document["rules"][0]["sources"][0] =
                "crystal:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/npc/pemaret.lua:102"
                    .into(),
        }
        let text = serde_json::to_string(&document)?;
        assert!(
            matches!(NpcRuleCatalog::from_json_documents(&[&text]),
            Err(NpcRuleError::Invalid(message)) if message == "Oldrak source custody"),
            "source custody case {case}"
        );
    }
    Ok(())
}
