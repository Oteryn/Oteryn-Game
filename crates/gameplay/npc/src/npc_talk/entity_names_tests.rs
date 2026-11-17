//! S-ENTNAME: visibility names must not depend on native dialogue support.
use super::*;

#[test]
fn entity_names_cover_catalogue_npcs_without_native_dialogue() -> Result<(), String> {
    let catalogue = crate::content::NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        crate::content::npc_catalogue_preproduction_limits(),
    )
    .map_err(|error| format!("{error:?}"))?;
    let model = catalogue
        .service_model_for_project()
        .map_err(|error| format!("{error:?}"))?;
    let book = NpcTalkBook::from_model(&model);
    let mut without_talk = 0;
    for (reference, npc) in model.npcs() {
        assert_eq!(book.display_name(&reference.key), Some(npc.name.as_str()));
        if book.get(&reference.key).is_none() {
            without_talk += 1;
        }
    }
    assert!(
        without_talk > 0,
        "exercise named NPCs without native talk support"
    );
    Ok(())
}
