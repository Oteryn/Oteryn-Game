use super::{ItemIdentityError, validate_appearance_items};
use std::collections::BTreeMap;

const INDEX: &[u8] =
    include_bytes!("../../../../../imports/official/appearance-membership/admitted.json");
const MANIFEST: &[u8] = include_bytes!(
    "../../../../../imports/official/appearance-membership/appearances-2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.json"
);

#[test]
fn m57_canonical_bridge_resolves_through_the_sorted_native_index()
-> Result<(), Box<dyn std::error::Error>> {
    let index = crate::content::item_ref::ItemDefinitionIndex::decode(include_bytes!(
        "../../../../../tools/content-schema/native-gameplay/item-keys.json"
    ))?;
    let key = "oteryn:item.tibia.i48227";
    let reference = index
        .definition_ref("Item", key, "definition-r1")
        .ok_or("admitted Cake bridge identity missing")?;
    assert_eq!(index.definition_at(reference), Some((key, "definition-r1")));
    assert!(
        index
            .definition_ref("Terrain", key, "definition-r1")
            .is_none()
    );
    assert!(index.definition_ref("Item", key, "definition-r2").is_none());
    Ok(())
}

#[test]
fn admission_proof_cannot_be_replaced_by_self_consistent_unpinned_bytes() {
    let mut unpinned = INDEX.to_vec();
    unpinned.push(b'\n');
    assert_eq!(
        validate_appearance_items(&[], &BTreeMap::new(), &unpinned, MANIFEST),
        Err(ItemIdentityError::AliasTable(
            "admitted appearance index digest"
        ))
    );
}

#[test]
fn current_manifest_bytes_are_bound_to_the_admitted_index() {
    let mut unpinned = MANIFEST.to_vec();
    unpinned.push(b'\n');
    assert_eq!(
        validate_appearance_items(&[], &BTreeMap::new(), INDEX, &unpinned),
        Err(ItemIdentityError::AliasTable(
            "current appearance manifest digest"
        ))
    );
}
