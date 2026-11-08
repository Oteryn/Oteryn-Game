use super::*;

#[test]
fn lane_qengine_reward_npcs_open_only_source_conversations() -> Result<(), String> {
    let catalogue = crate::content::NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        crate::content::npc_catalogue_preproduction_limits(),
    )
    .map_err(|e| format!("{e:?}"))?;
    let model = catalogue
        .service_model_for_project()
        .map_err(|e| format!("{e:?}"))?;
    let book = NpcTalkBook::from_model(&model);
    for name in ["morgan", "navigator", "costello", "budrik", "sam"] {
        let key = format!("oteryn:npc.{name}");
        let entry = book
            .get(&key)
            .ok_or_else(|| format!("missing reward conversation {key}"))?;
        assert!(!entry.greeting_reply.is_empty());
        assert!(
            entry.dialogue_keywords.is_empty(),
            "no scripted reward branch opens through talk rules"
        );
    }
    assert!(!reward_callback_npc("oteryn:npc.unqualified"));
    Ok(())
}
