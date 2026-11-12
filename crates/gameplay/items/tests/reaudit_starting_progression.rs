use oteryn_gameplay_domain::domain::item_definition::TypedDefinitionRef;
use oteryn_gameplay_items::starter_grant::StartingLoadout;

#[test]
fn reaudit_starting_progression_rejects_tampered_and_unsupported_loadouts()
-> Result<(), &'static str> {
    let resolve = |id| {
        Some(TypedDefinitionRef {
            family: "Item".into(),
            production_key: format!("oteryn:item.tibia.i{id}"),
            revision_ref: "definition-r1".into(),
        })
    };
    for vocation in ["none", "sorcerer", "druid", "paladin", "knight", "monk"] {
        let original = StartingLoadout::resolve(vocation, resolve).ok_or("donor kit")?;
        assert!(original.matches_source());
        let mut quantity = original.clone();
        quantity.items[0].1 += 1;
        assert!(
            !quantity.matches_source(),
            "caller cannot increase a donor quantity"
        );
        let mut equipment = original.clone();
        equipment.items[0].2 = 8;
        assert!(
            !equipment.matches_source(),
            "caller cannot change item custody"
        );
        let mut definition = original;
        definition.items[0].0.production_key = "oteryn:item.tibia.i3031".into();
        assert!(
            !definition.matches_source(),
            "caller cannot substitute gold for tools"
        );
    }
    assert!(StartingLoadout::resolve("elite_knight", resolve).is_none());
    assert!(StartingLoadout::resolve("unknown", resolve).is_none());
    Ok(())
}
