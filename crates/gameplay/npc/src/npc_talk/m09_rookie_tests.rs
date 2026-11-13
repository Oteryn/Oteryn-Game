use super::*;

#[test]
fn m09_rookie_reward_has_nonempty_wire_safe_greeting() -> Result<(), String> {
    let catalogue = crate::content::NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        crate::content::npc_catalogue_preproduction_limits(),
    )
    .map_err(|e| format!("{e:?}"))?;
    let model = catalogue
        .service_model_for_project()
        .map_err(|e| format!("{e:?}"))?;
    let key = "oteryn:npc.vascalir";
    for book in [
        NpcTalkBook::from_model_with_native_quests(&model, &[key]),
        NpcTalkBook::from_model(&model),
    ] {
        let entry = book
            .get(key)
            .ok_or("Rookie reward NPC must be admitted at runtime")?;
        assert!(
            !entry.greeting_reply.is_empty(),
            "conditional source greeting must not emit an empty wire line"
        );
        let rendered = entry.render_source_text(&entry.greeting_reply, "Quest witness");
        assert!(!rendered.chars().any(char::is_control));
        assert!(rendered.len() <= 2048);
    }
    Ok(())
}
