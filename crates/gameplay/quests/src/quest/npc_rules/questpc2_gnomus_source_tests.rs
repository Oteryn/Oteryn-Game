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
