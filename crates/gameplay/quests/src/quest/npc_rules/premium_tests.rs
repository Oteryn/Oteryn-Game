use super::*;

#[test]
fn npc_premium_requirement_is_bound_without_changing_legacy_program_hashes()
-> Result<(), NpcRuleError> {
    let original = include_str!("test-fixtures/hero_greeting.json");
    let legacy = NpcRuleCatalog::from_json_documents(&[original])?;
    let mut document: serde_json::Value =
        serde_json::from_str(original).map_err(|error| NpcRuleError::Json(error.to_string()))?;
    document["premium_required"] = false.into();
    let explicit_free = NpcRuleCatalog::from_json_documents(&[&document.to_string()])?;
    assert_eq!(
        legacy.programs()[0].digest,
        explicit_free.programs()[0].digest
    );
    document["premium_required"] = true.into();
    let premium = NpcRuleCatalog::from_json_documents(&[&document.to_string()])?;
    assert_ne!(legacy.programs()[0].digest, premium.programs()[0].digest);
    Ok(())
}
