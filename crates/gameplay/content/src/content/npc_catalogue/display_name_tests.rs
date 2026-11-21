use super::*;

fn model(
    key: &str,
    fields: serde_json::Value,
) -> Result<(ProjectV2DefinitionRef, NpcServiceModel), Box<dyn std::error::Error>> {
    let reference: ProjectV2DefinitionRef = serde_json::from_value(serde_json::json!({
        "family":"NPC","key":key,"revision":"definition-r1"
    }))?;
    let declaration: ProjectV2Declaration = serde_json::from_value(serde_json::json!({
        "kind":"NPC","identity":{"key":key,"revision":"definition-r1"},
        "presentation":null,"behavior":null,"dialogue":null,"services":[],"fields":fields
    }))?;
    let result = build(
        &[(&reference, &declaration)],
        "display-test",
        "display-test",
        &ReferenceNpcItemFacts::default(),
    );
    Ok((reference, result))
}

#[test]
fn npcfix_preserved_display_names_reach_the_service_model_without_changing_identity()
-> Result<(), Box<dyn std::error::Error>> {
    for (key, name) in [
        ("oteryn:npc.ferryman_kamil_meluna", "Ferryman Kamil"),
        ("oteryn:npc.harlow_vengoth", "Harlow"),
        ("oteryn:npc.junkar_mines", "Junkar"),
        ("oteryn:npc.maris_mistrock", "Maris"),
    ] {
        let (reference, model) = model(
            key,
            serde_json::json!([{
                "field_path":"oteryn:source.npc.display_name","value":{"type":"Text","value":name}
            }]),
        )?;
        let entry = model
            .npcs()
            .get(&reference)
            .ok_or("NPC identity changed or held")?;
        assert_eq!(entry.name, name);
        assert_eq!(reference.key, key);
    }
    Ok(())
}

#[test]
fn npcfix_display_names_keep_fallback_for_absent_or_unusable_values()
-> Result<(), Box<dyn std::error::Error>> {
    let key = "oteryn:npc.maris_mistrock";
    for name in ["", "   ", "bad\nname", &"x".repeat(65)] {
        let (reference, model) = model(
            key,
            serde_json::json!([{
                "field_path":"oteryn:source.npc.display_name","value":{"type":"Text","value":name}
            }]),
        )?;
        assert_eq!(
            model.npcs().get(&reference).ok_or("NPC")?.name,
            replies::npc_display_name(key)
        );
    }
    let (reference, model) = model(key, serde_json::json!([]))?;
    assert_eq!(
        model.npcs().get(&reference).ok_or("NPC")?.name,
        replies::npc_display_name(key)
    );
    Ok(())
}
