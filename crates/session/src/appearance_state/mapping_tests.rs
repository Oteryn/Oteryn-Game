#![allow(clippy::expect_used)]
use super::*;
const GOLDEN: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../protocol-oteryn/tests/fixtures/appearance_snapshot_v1.bin"
));

#[test]
fn appearance_mapping_reducer_is_atomic_on_stale_gapped_and_reused_refs() {
    let snapshot = wire::Snapshot::decode(GOLDEN).expect("valid appearance fixture");
    let mut state = AppearanceState {
        items: None,
        revision: 1,
        high_water: snapshot.high_water,
        catalogue: snapshot.catalogue,
        bindings: snapshot
            .bindings
            .into_iter()
            .map(|b| (b.entity, b))
            .collect(),
    };
    let before = state.clone();
    let mut binding = state
        .bindings
        .values()
        .next()
        .expect("valid appearance fixture")
        .clone();
    binding.root_revision = 2;
    let payload = wire::Mappings {
        context: state.catalogue.context,
        upserts: vec![binding.clone()],
        forgets: Vec::new(),
    }
    .encode()
    .expect("valid appearance fixture");
    assert!(state.apply(0, 1, 1, &payload, false).is_err());
    assert!(state.apply(1, 3, 1, &payload, false).is_err());
    assert_eq!(state.bindings, before.bindings);
    assert_eq!(state.revision, 1);
    state
        .apply(1, 2, 1, &payload, false)
        .expect("valid appearance fixture");
    assert!(state.apply(2, 3, 1, &payload, false).is_err());
    let entity = binding.entity;
    let forget = wire::Mappings {
        context: state.catalogue.context,
        upserts: Vec::new(),
        forgets: vec![entity],
    }
    .encode()
    .expect("valid appearance fixture");
    state
        .apply(2, 3, 1, &forget, false)
        .expect("valid appearance fixture");
    assert!(state.apply(3, 4, 1, &payload, false).is_err());
    binding.appearance_ref = 2;
    let reappear = wire::Mappings {
        context: state.catalogue.context,
        upserts: vec![binding],
        forgets: Vec::new(),
    }
    .encode()
    .expect("valid appearance fixture");
    state
        .apply(3, 4, 1, &reappear, false)
        .expect("valid appearance fixture");
    assert_eq!(state.high_water, 2);
}
#[test]
fn appearance_mapping_context_palette_and_item_downgrades_refuse() {
    let mut snapshot = wire::Snapshot::decode(GOLDEN).expect("valid appearance fixture");
    let selected = [6, 25];
    let domain = GatedSnapshot {
        domain_id: 21,
        snapshot_type: 1,
        revision: 1,
        payload: GOLDEN.to_vec(),
    };
    assert!(from_snapshots(&selected, std::slice::from_ref(&domain), [9; 32], None).is_err());
    assert!(from_snapshots(&selected, &[], [1; 32], None).is_err());
    assert!(from_snapshots(&selected, &[domain], [1; 32], None).is_ok());
    snapshot.bindings[0].look = wire::Look::Item { definition_ref: 1 };
    let domain = GatedSnapshot {
        domain_id: 21,
        snapshot_type: 1,
        revision: 1,
        payload: snapshot.encode().expect("valid appearance fixture"),
    };
    assert!(from_snapshots(&selected, std::slice::from_ref(&domain), [1; 32], None).is_err());
    assert!(from_snapshots(&[4, 6, 25], std::slice::from_ref(&domain), [1; 32], None).is_err());
    let items = AppearanceItemDictionary {
        content_generation: [1; 32],
        definition_refs: &[1],
    };
    assert!(from_snapshots(&[4, 6, 25], &[domain], [1; 32], Some(items)).is_ok());
}
