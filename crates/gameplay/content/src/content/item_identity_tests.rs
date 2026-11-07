use super::{ItemIdentityError, validate_appearance_items};
use std::collections::BTreeMap;

const INDEX: &[u8] =
    include_bytes!("../../../../../imports/official/appearance-membership/admitted.json");
const MANIFEST: &[u8] = include_bytes!(
    "../../../../../imports/official/appearance-membership/appearances-2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.json"
);

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
