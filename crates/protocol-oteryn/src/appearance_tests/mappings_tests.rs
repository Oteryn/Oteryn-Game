#![allow(clippy::expect_used)]
use crate::appearance::*;
const GOLDEN: &[u8] = include_bytes!("../../tests/fixtures/appearance_snapshot_v1.bin");

#[test]
fn appearance_dictionary_has_an_independent_byte_golden() {
    let snapshot = Snapshot::decode(GOLDEN).expect("valid appearance fixture");
    assert_eq!(snapshot.encode().expect("valid appearance fixture"), GOLDEN);
    assert_eq!(snapshot.catalogue.root_revision, 1);
    assert_eq!(snapshot.bindings[0].entity.identity, [3; 16]);
    assert_eq!(
        snapshot.bindings[0].look,
        Look::Outfit {
            look_type: 128,
            colours: [0, 1, 2, 0],
            addons: 0
        }
    );
}
#[test]
fn appearance_catalogue_palette_and_list_bounds_are_symmetric() {
    let mut catalogue = Snapshot::decode(GOLDEN)
        .expect("valid appearance fixture")
        .catalogue;
    catalogue.palette = vec![[0; 3]; 256];
    assert_eq!(
        Catalogue::decode(&catalogue.encode().expect("valid appearance fixture"))
            .expect("valid appearance fixture"),
        catalogue
    );
    catalogue.palette.push([0; 3]);
    assert!(catalogue.encode().is_err());
    catalogue.palette.pop();
    catalogue.outfits.push(catalogue.outfits[0].clone());
    assert!(catalogue.encode().is_err());
    catalogue.outfits.clear();
    catalogue.mounts = vec!["x".into(); 257];
    assert!(catalogue.encode().is_err());
}
#[test]
fn appearance_mapping_rejects_mixed_context_repeated_refs_and_kind_fields() {
    let original = Snapshot::decode(GOLDEN).expect("valid appearance fixture");
    let mut changed = original.clone();
    changed.bindings[0].context.cosmetics_digest = [9; 32];
    assert!(changed.encode().is_err());
    changed = original.clone();
    changed.bindings.push(changed.bindings[0].clone());
    assert!(changed.encode().is_err());
    changed = original.clone();
    changed.bindings[0].entity.generation = 0;
    assert!(changed.encode().is_err());
    changed = original.clone();
    changed.bindings[0].selection_revision = Some(2);
    assert!(changed.encode().is_err());
    changed = original.clone();
    changed.high_water = 0;
    assert!(changed.encode().is_err());
    let mappings = Mappings {
        context: original.catalogue.context,
        upserts: original.bindings.clone(),
        forgets: vec![original.bindings[0].entity],
    };
    assert!(mappings.encode().is_err());
    let encoded = original.bindings[0]
        .encode()
        .expect("valid appearance fixture");
    for suffix in [&[24, 1][..], &[80, 0], &[34, 0]] {
        let mut invalid = encoded.clone();
        invalid.extend_from_slice(suffix);
        assert!(Binding::decode(&invalid).is_err());
    }
}
