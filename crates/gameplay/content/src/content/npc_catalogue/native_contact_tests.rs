//! A real normal package must admit the contact before its World anchor is useful.
#[test]
fn m38_spy_rat_is_admitted_by_the_real_npc_service_model() -> Result<(), Box<dyn std::error::Error>>
{
    let catalogue = crate::content::NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        crate::content::npc_catalogue_preproduction_limits(),
    )?;
    let model = catalogue.service_model_for_project()?;
    let (identity, contact) = model
        .npcs()
        .iter()
        .find(|(identity, _)| identity.key == "oteryn:npc.eustacios_spy_rat")
        .ok_or("Spy Rat missing from admitted runtime model")?;
    assert_eq!(identity.revision, "definition-r1");
    assert_eq!(contact.name, "Eustacio's Spy Rat");
    assert!(contact.dialogue.is_none());
    assert!(contact.bank_dialogue.is_none());
    assert!(contact.services.is_empty());
    Ok(())
}
