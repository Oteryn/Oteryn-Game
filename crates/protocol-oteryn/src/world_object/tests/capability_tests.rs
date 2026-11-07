use super::*;

#[test]
fn fields_4_and_5_round_trip_under_capability_15() {
    let handle = ItemHandle::new(u64::MAX).expect("non-zero");
    for intent in [
        plain(UseTarget::Item(handle)),
        plain(definition(1)),
        plain(definition(u32::MAX)),
        UseIntent {
            target: UseTarget::Item(handle),
            use_with: Some(creature(u64::MAX)),
            use_with_map_item: None,
            use_with_position: None,
        },
        UseIntent {
            target: definition(u32::MAX),
            use_with: Some(creature(1)),
            use_with_map_item: None,
            use_with_position: None,
        },
    ] {
        let bytes = encode_item_use_intent(&intent).expect("encode");
        assert!(bytes.len() <= 44, "{intent:?}");
        assert_eq!(
            decode_use_intent_target(&ITEM_USE, &bytes),
            Ok(intent.clone())
        );
        // Without capability 15 (or without its requirement 4) fields 4 and 5 fail closed.
        let plain_item = matches!(intent.target, UseTarget::Item(_)) && intent.use_with.is_none();
        for selected in [
            &[CAPABILITY_ITEM_VIEW_MOVE_V1][..],
            &[CAPABILITY_ITEM_USE_V1],
        ] {
            if plain_item && selected == [CAPABILITY_ITEM_VIEW_MOVE_V1] {
                // A plain field 2 stays valid under capability 4 alone.
                assert_eq!(
                    decode_use_intent_target(selected, &bytes),
                    Ok(intent.clone())
                );
                continue;
            }
            assert_eq!(
                decode_use_intent_target(selected, &bytes),
                Err(WorldObjectError::Malformed),
                "{selected:?} {bytes:?}"
            );
        }
    }
    // The largest intent with fields 4 and 5 is 44 bytes, far within ITEMUSE0-RL-03.
    let worst = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(handle),
        use_with: Some(creature(u64::MAX)),
        use_with_map_item: None,
        use_with_position: None,
    })
    .expect("encode");
    assert_eq!(worst.len(), 44);
    // A world-object target still round trips under capability 15.
    let object = encode_use_intent(&target(b"door", 3)).expect("encode");
    assert_eq!(
        decode_use_intent_target(&ITEM_USE, &object),
        Ok(plain(UseTarget::WorldObject(target(b"door", 3))))
    );
}
