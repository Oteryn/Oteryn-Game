use super::*;

#[test]
fn cook_material_bindings_admit_only_the_three_exact_wiki_targets() -> Result<(), ContentError> {
    let mut bindings = BTreeMap::new();
    questpc5_cook_bindings::extend(&mut bindings)?;
    assert_eq!(bindings.len(), 3);
    for (id, revision) in [
        ("44525", "fandom-r1047591"),
        ("44527", "fandom-r1047595"),
        ("44528", "fandom-r1046764"),
    ] {
        let binding = bindings
            .get(id)
            .ok_or_else(|| invalid("missing Cook binding"))?;
        assert_eq!(
            binding.target.family,
            super::super::project::ProjectV2Family::Item
        );
        assert_eq!(binding.target.key, format!("oteryn:item.tibia.i{id}"));
        assert_eq!(binding.target.revision, "definition-r1");
        assert_eq!(binding.source_revision, revision);
        assert_eq!(binding.identity_namespace, "tibiawiki/item_id");
        assert_eq!(
            binding.disposition,
            super::super::project::ProjectV2SourceIdentityDisposition::Exact
        );
    }
    assert!(!bindings.contains_key("5884"));
    Ok(())
}

#[test]
fn cook_material_binding_collision_refuses_the_generation() -> Result<(), ContentError> {
    let mut bindings = BTreeMap::new();
    questpc5_cook_bindings::extend(&mut bindings)?;
    assert!(questpc5_cook_bindings::extend(&mut bindings).is_err());
    assert_eq!(bindings.len(), 3);
    Ok(())
}
