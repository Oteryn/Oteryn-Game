#![allow(clippy::expect_used)]
use super::*;

fn context() -> Context {
    Context {
        content: [1; 32],
        cosmetics: [2; 32],
    }
}
fn selection() -> Selection {
    Selection {
        outfit: "oteryn:outfit.starter".into(),
        colours: [0, 1, 2, 132],
        addons: 0,
        mount: None,
    }
}
fn change() -> Change {
    Change {
        context: context(),
        expected_character_revision: 7,
        before: Some(Selected {
            selection: selection(),
            revision: 3,
            provenance: "content-old".into(),
        }),
        selection: Selection {
            addons: 1,
            ..selection()
        },
        content_revision: "content-1".into(),
    }
}
fn catalogue() -> Catalogue {
    Catalogue {
        context: context(),
        character_revision: 7,
        selected: change().before,
        palette: vec![[10, 20, 30]; 133],
        outfits: vec![OutfitOption {
            key: selection().outfit,
            look_type: 128,
            allowed_addons: 1,
        }],
        mounts: vec!["mount.horse".into()],
    }
}
fn binding(reference: u32) -> Binding {
    Binding {
        reference,
        entity: EntityRef {
            identity: [3; 16],
            generation: 4,
        },
        context: context(),
        character_revision: 7,
        selection_revision: Some(3),
        condition_revision: 9,
        owner_time_us: 100,
        look: Look::Outfit {
            look_type: 128,
            colours: [0, 1, 2, 132],
            addons: 1,
        },
    }
}
fn mappings(binding: Binding) -> Mappings {
    Mappings {
        context: context(),
        bindings: vec![binding],
        forget: Vec::new(),
    }
}
fn snapshot() -> Snapshot {
    Snapshot {
        catalogue: catalogue(),
        bindings: vec![binding(1)],
        high_water: 1,
    }
}

#[test]
fn complete_payloads_roundtrip_including_item_transform_and_stored_mount() {
    let mut command = change();
    command.selection.mount = Some("mount.horse".into());
    assert_eq!(
        decode_change(&encode_change(&command).expect("valid fixture or native operation"))
            .expect("valid fixture or native operation"),
        command
    );
    for result in [
        ChangeResult::Rejected,
        ChangeResult::Committed { revision: u64::MAX },
    ] {
        assert_eq!(
            decode_result(&encode_result(result).expect("valid fixture or native operation"))
                .expect("valid fixture or native operation"),
            result
        );
    }
    let full = snapshot();
    assert_eq!(
        decode_snapshot(&encode_snapshot(&full).expect("valid fixture or native operation"))
            .expect("valid fixture or native operation"),
        full
    );
    let mut item = binding(1);
    item.look = Look::Item { definition_ref: 91 };
    let delta = mappings(item);
    assert_eq!(
        decode_mappings(&encode_mappings(&delta).expect("valid fixture or native operation"))
            .expect("valid fixture or native operation"),
        delta
    );
    assert_eq!(
        decode_catalogue(
            &encode_catalogue(&catalogue()).expect("valid fixture or native operation")
        )
        .expect("valid fixture or native operation"),
        catalogue()
    );
}

#[test]
fn all_decoders_refuse_every_truncated_prefix_unknown_and_duplicate_fields() {
    type Decoder = fn(&[u8]) -> bool;
    let mut required_binding = binding(1);
    required_binding.selection_revision = None;
    let cases: Vec<(Vec<u8>, Decoder)> = vec![
        (
            encode_change(&change()).expect("valid fixture or native operation"),
            |b| decode_change(b).is_err(),
        ),
        (
            encode_result(ChangeResult::Committed { revision: 7 })
                .expect("valid fixture or native operation"),
            |b| decode_result(b).is_err(),
        ),
        (
            encode_binding(&required_binding).expect("valid fixture or native operation"),
            |b| decode_binding(b).is_err(),
        ),
        (
            encode_selection(&selection()).expect("valid fixture or native operation"),
            |b| decode_selection(b).is_err(),
        ),
    ];
    for (bytes, refuses) in cases {
        for end in 0..bytes.len() {
            assert!(refuses(&bytes[..end]), "accepted truncated prefix {end}");
        }
        let mut unknown = bytes.clone();
        push_varint_field(&mut unknown, 99, 1);
        assert!(refuses(&unknown));
        let mut duplicate = bytes.clone();
        duplicate.extend_from_slice(&bytes);
        assert!(refuses(&duplicate));
        let mut bad_wire = bytes;
        bad_wire[0] = (bad_wire[0] & !7) | 5;
        assert!(refuses(&bad_wire));
    }
}

#[test]
fn maximum_native_keys_in_both_prior_and_new_selection_fit_the_command() {
    let mut value = change();
    value.selection.outfit = "n".repeat(MAX_KEY_BYTES);
    value.selection.mount = Some("m".repeat(MAX_KEY_BYTES));
    value.content_revision = "c".repeat(MAX_PROVENANCE_BYTES);
    let before = value
        .before
        .as_mut()
        .expect("valid fixture or native operation");
    before.selection.outfit = "o".repeat(MAX_KEY_BYTES);
    before.selection.mount = Some("p".repeat(MAX_KEY_BYTES));
    before.provenance = "r".repeat(MAX_PROVENANCE_BYTES);
    let bytes = encode_change(&value).expect("valid fixture or native operation");
    assert!(bytes.len() > 2_048);
    assert!(bytes.len() <= MAX_CHANGE_BYTES);
    assert_eq!(
        decode_change(&bytes).expect("valid fixture or native operation"),
        value
    );
}

#[test]
fn implicit_fallback_has_no_fabricated_selection_revision_and_item_requires_its_capability() {
    let mut full = snapshot();
    full.catalogue.selected = None;
    full.bindings[0].selection_revision = None;
    let store = ReferenceStore::from_snapshot(1, 1, &[6, 25], full.clone())
        .expect("valid fixture or native operation");
    assert_eq!(
        store
            .resolve(1, binding(1).entity, 1)
            .expect("valid fixture or native operation")
            .selection_revision,
        None
    );
    full.bindings[0].look = Look::Item { definition_ref: 1 };
    assert!(ReferenceStore::from_snapshot(1, 1, &[6, 25], full.clone()).is_err());
    assert!(ReferenceStore::from_snapshot(1, 1, &[4, 6, 25], full.clone()).is_ok());
    assert!(ReferenceStore::from_snapshot(1, 1, &[25], full).is_err());
}

#[test]
fn key_utf8_provenance_colour_and_addon_shape_boundaries_are_strict() {
    let mut value = selection();
    value.outfit = "x".repeat(MAX_KEY_BYTES);
    value.mount = Some("y".repeat(MAX_KEY_BYTES));
    value.colours = [u16::MAX; 4];
    value.addons = u8::MAX;
    assert_eq!(
        decode_selection(&encode_selection(&value).expect("valid fixture or native operation"))
            .expect("valid fixture or native operation"),
        value
    );
    value.outfit.push('x');
    assert_eq!(encode_selection(&value), Err(Error::LimitExceeded));
    value.outfit = "bad\nkey".into();
    assert_eq!(encode_selection(&value), Err(Error::Malformed));
    let mut raw = Vec::new();
    push_message_field(&mut raw, 1, &[0xff]);
    put_colours(&mut raw, 2, [0; 4]);
    push_varint_field(&mut raw, 3, 0);
    assert_eq!(decode_selection(&raw), Err(Error::Malformed));
    let mut raw = Vec::new();
    push_message_field(&mut raw, 1, b"key");
    put_colours(&mut raw, 2, [0; 4]);
    push_varint_field(&mut raw, 3, 256);
    assert_eq!(decode_selection(&raw), Err(Error::Malformed));
    let mut command = change();
    command.content_revision = "é".repeat(64);
    assert!(encode_change(&command).is_ok());
    command.content_revision.push('a');
    assert_eq!(encode_change(&command), Err(Error::LimitExceeded));
    command = change();
    command
        .before
        .as_mut()
        .expect("valid fixture or native operation")
        .revision = 8;
    assert_eq!(encode_change(&command), Err(Error::Malformed));
    command = change();
    command.expected_character_revision = 0;
    assert_eq!(encode_change(&command), Err(Error::Malformed));
}

#[test]
fn result_variants_are_closed_and_never_represent_unknown_commit_as_rejected() {
    for raw in [
        vec![],
        vec![8, 0],
        vec![8, 3],
        vec![8, 1],
        vec![8, 2, 16, 7],
        vec![8, 1, 16, 0],
        vec![8, 2, 8, 2],
    ] {
        assert!(decode_result(&raw).is_err(), "{raw:?}");
    }
    assert_eq!(
        encode_result(ChangeResult::Committed { revision: 0 }),
        Err(Error::Malformed)
    );
}

#[test]
fn catalogue_lists_and_palette_are_bounded_unique_and_in_canonical_order() {
    let mut value = catalogue();
    value.outfits.push(value.outfits[0].clone());
    assert_eq!(encode_catalogue(&value), Err(Error::Malformed));
    value = catalogue();
    value.mounts = vec!["z".into(), "a".into()];
    assert_eq!(encode_catalogue(&value), Err(Error::Malformed));
    value = catalogue();
    value.palette.clear();
    assert_eq!(encode_catalogue(&value), Err(Error::LimitExceeded));
    value = catalogue();
    value.palette = vec![[0; 3]; MAX_PALETTE_COLOURS + 1];
    assert_eq!(encode_catalogue(&value), Err(Error::LimitExceeded));
    value = catalogue();
    value.outfits[0].look_type = 0;
    assert_eq!(encode_catalogue(&value), Err(Error::Malformed));
    value = catalogue();
    value.mounts = (0..=MAX_CATALOGUE_MOUNTS)
        .map(|n| format!("mount.{n:04}"))
        .collect();
    assert_eq!(encode_catalogue(&value), Err(Error::LimitExceeded));
}

#[test]
fn binding_batch_rejects_cross_content_duplicates_conflicting_variants_and_overflow() {
    let mut delta = mappings(binding(1));
    let mut other = binding(2);
    other.entity.identity = [4; 16];
    delta.bindings.push(other.clone());
    assert!(encode_mappings(&delta).is_ok());
    delta.bindings[1].context.cosmetics = [9; 32];
    assert_eq!(encode_mappings(&delta), Err(Error::Malformed));
    delta.bindings[1] = other;
    delta.bindings[1].reference = 1;
    assert_eq!(encode_mappings(&delta), Err(Error::Malformed));
    delta = mappings(binding(1));
    delta.forget.push(binding(1).entity);
    assert_eq!(encode_mappings(&delta), Err(Error::Malformed));
    let mut bad = binding(1);
    bad.entity.generation = 0;
    assert_eq!(encode_binding(&bad), Err(Error::Malformed));
    let mut raw = Vec::new();
    push_varint_field(&mut raw, 1, 2);
    push_varint_field(&mut raw, 5, 7);
    push_varint_field(&mut raw, 2, 128);
    assert_eq!(decode_look(&raw), Err(Error::Malformed));
    for bytes in [
        vec![0; MAX_CHANGE_BYTES + 1],
        vec![0; MAX_SNAPSHOT_BYTES + 1],
    ] {
        assert!(decode_change(&bytes).is_err());
        assert!(decode_snapshot(&bytes).is_err());
    }
}

#[test]
fn connection_generation_reset_cannot_resolve_old_reference_or_old_entity_generation() {
    let old = ReferenceStore::from_snapshot(4, 1, &[4, 6, 25], snapshot())
        .expect("valid fixture or native operation");
    assert!(old.resolve(4, binding(1).entity, 1).is_ok());
    assert!(old.resolve(5, binding(1).entity, 1).is_err());
    let mut fresh = snapshot();
    fresh.bindings[0].entity.generation = 5;
    let new = ReferenceStore::from_snapshot(5, 1, &[4, 6, 25], fresh.clone())
        .expect("valid fixture or native operation");
    assert!(new.resolve(5, binding(1).entity, 1).is_err());
    assert!(new.resolve(5, fresh.bindings[0].entity, 1).is_ok());
}

#[test]
fn mapping_before_entity_unknown_reference_and_retired_reference_fail_closed() {
    let mut start = snapshot();
    start.bindings.clear();
    start.high_water = 0;
    let mut store = ReferenceStore::from_snapshot(1, 1, &[4, 6, 25], start)
        .expect("valid fixture or native operation");
    assert!(store.resolve(1, binding(1).entity, 1).is_err());
    store
        .apply(1, mappings(binding(1)))
        .expect("valid fixture or native operation");
    assert!(store.resolve(1, binding(1).entity, 1).is_ok());
    store
        .apply(
            1,
            Mappings {
                context: context(),
                bindings: Vec::new(),
                forget: vec![binding(1).entity],
            },
        )
        .expect("valid fixture or native operation");
    assert!(store.resolve(1, binding(1).entity, 1).is_err());
    let before = store.clone();
    assert!(store.apply(1, mappings(binding(1))).is_err());
    assert_eq!(store, before);
    store
        .apply(1, mappings(binding(2)))
        .expect("valid fixture or native operation");
    assert!(store.resolve(1, binding(1).entity, 2).is_ok());
}

#[test]
fn ordered_newer_observation_supersedes_without_repainting_an_old_result() {
    let mut store = ReferenceStore::from_snapshot(1, 1, &[4, 6, 25], snapshot())
        .expect("valid fixture or native operation");
    let committed = ChangeResult::Committed { revision: 8 };
    assert_eq!(
        decode_result(&encode_result(committed).expect("valid fixture or native operation"))
            .expect("valid fixture or native operation"),
        committed
    );
    // A result is settlement only: it cannot touch this dictionary.
    assert_eq!(
        store
            .resolve(1, binding(1).entity, 1)
            .expect("valid fixture or native operation")
            .character_revision,
        7
    );
    let mut next = binding(2);
    next.character_revision = 8;
    next.owner_time_us = 101;
    store
        .apply(1, mappings(next.clone()))
        .expect("valid fixture or native operation");
    store
        .apply(1, mappings(next))
        .expect("valid fixture or native operation");
    let before = store.clone();
    assert!(store.apply(1, mappings(binding(1))).is_err());
    assert_eq!(store, before);
    let mut bad = binding(3);
    bad.character_revision = 8;
    bad.owner_time_us = 102;
    if let Look::Outfit { colours, .. } = &mut bad.look {
        colours[0] = 133;
    }
    assert!(store.apply(1, mappings(bad)).is_err());
    assert_eq!(store, before);
}

#[test]
fn rejected_batch_is_atomic_and_catalogue_context_cannot_be_rebound() {
    let mut store = ReferenceStore::from_snapshot(1, 1, &[4, 6, 25], snapshot())
        .expect("valid fixture or native operation");
    let before = store.clone();
    let mut new = binding(2);
    new.character_revision = 8;
    new.owner_time_us = 101;
    let mut bad = binding(3);
    bad.entity.identity = [8; 16];
    bad.context.content = [9; 32];
    assert!(
        store
            .apply(
                1,
                Mappings {
                    context: context(),
                    bindings: vec![new, bad],
                    forget: Vec::new()
                }
            )
            .is_err()
    );
    assert_eq!(store, before);
    let mut changed = catalogue();
    changed.context.cosmetics = [8; 32];
    assert!(store.update_catalogue(1, changed).is_err());
    assert_eq!(store, before);
}

#[test]
fn same_character_revision_catalogue_change_uses_actual_domain_sequence() {
    let mut store =
        ReferenceStore::from_snapshot(1, 8, &[6, 25], snapshot()).expect("valid snapshot");
    let mut next = catalogue();
    next.mounts.push("mount.zebra".into());
    assert_eq!(
        next.character_revision,
        store.catalogue().character_revision
    );
    let bytes = encode_catalogue(&next).expect("catalogue payload");
    let delta = crate::StateDeltaView {
        domain_id: STATE_DOMAIN_APPEARANCE,
        base_revision: 8,
        new_revision: 9,
        delta_type: DELTA_TYPE_APPEARANCE_CATALOGUE_V1,
        payload: &bytes,
    };
    store
        .apply_delta(1, delta)
        .expect("next domain revision updates own allowed choices");
    assert_eq!(store.catalogue(), &next);
    let unchanged = store.clone();
    assert!(store.apply_delta(1, delta).is_err());
    assert_eq!(store, unchanged);
    assert!(
        store
            .apply_delta(
                1,
                crate::StateDeltaView {
                    base_revision: 9,
                    new_revision: 11,
                    ..delta
                }
            )
            .is_err()
    );
    assert_eq!(store, unchanged);
}

#[test]
fn resync_retains_retired_reference_high_water_and_refuses_rebinding() {
    let mut store = ReferenceStore::from_snapshot(1, 1, &[6, 25], snapshot()).expect("snapshot");
    let mut empty = snapshot();
    empty.bindings.clear();
    empty.high_water = 7;
    store
        .replace_snapshot(1, 3, empty)
        .expect("authoritative resync");
    let unchanged = store.clone();
    assert!(store.apply(1, mappings(binding(2))).is_err());
    assert_eq!(store, unchanged);
    assert!(store.replace_snapshot(1, 4, snapshot()).is_err());
    assert_eq!(store, unchanged);
    let mut next = snapshot();
    next.bindings[0].reference = 8;
    next.high_water = 8;
    store
        .replace_snapshot(1, 4, next)
        .expect("new immutable reference after resync");
    assert!(store.resolve(1, binding(1).entity, 8).is_ok());
}
