//! Source modifications1 / ReducedHealth6 or ReducedHealthExclamation7.
use super::*;
#[test]
fn srx3_soulpit_creature_icon_field14_requires_negotiated_native_carrier()
-> Result<(), WorldSpatialError> {
    let entity = WorldSpatialEntity {
        display_name: None,
        kind: EntityKind::Creature,
        entity: EntityRef {
            identity: [9; 16],
            generation: 1,
        },
        position: ActorPosition {
            x: 32373,
            y: 31151,
            floor: -8,
        },
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    let fields = EntityFields::selected(&[6, 371]);
    let mut encoded = encode_entity(&entity, fields, None)?;
    // New field14; explicit category1/icon6/count15, from pinned C++ protocol.
    push_message(&mut encoded, 14, &[8, 1, 16, 6, 24, 15]);
    assert!(
        decode_entity(&encoded, fields).is_ok(),
        "current source badge must decode under selected371"
    );
    assert!(decode_entity(&encoded, EntityFields::selected(&[6])).is_err());
    let mut duplicate = encoded.clone();
    push_message(&mut duplicate, 14, &[8, 1, 16, 6, 24, 15]);
    assert!(
        decode_entity(&duplicate, fields).is_ok(),
        "native ordered lists preserve duplicate icon IDs"
    );
    push_message(&mut duplicate, 14, &[8, 1, 16, 6, 24, 15]);
    push_message(&mut duplicate, 14, &[8, 1, 16, 6, 24, 15]);
    assert!(
        decode_entity(&duplicate, fields).is_err(),
        "source visible list is bounded to three"
    );
    Ok(())
}
#[test]
fn srx3_soulpit_creature_icon_refuses_wrong_category_shape_count_and_object()
-> Result<(), WorldSpatialError> {
    let entity = WorldSpatialEntity {
        display_name: None,
        kind: EntityKind::Creature,
        entity: EntityRef {
            identity: [9; 16],
            generation: 1,
        },
        position: ActorPosition {
            x: 32373,
            y: 31151,
            floor: -8,
        },
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    let fields = EntityFields::selected(&[6, 371]);
    for bad in [
        &[8, 2, 16, 6, 24, 15][..],
        &[8, 1, 16, 6, 24, 40][..],
        &[8, 1, 16, 7, 24, 1][..],
        &[8, 1, 16, 7][..],
    ] {
        let mut encoded = encode_entity(&entity, fields, None)?;
        push_message(&mut encoded, 14, bad);
        assert!(decode_entity(&encoded, fields).is_err());
    }
    Ok(())
}

#[test]
fn srx3_native_creature_icons_preserve_order_duplicate_ids_and_visible_zero()
-> Result<(), WorldSpatialError> {
    let entity = WorldSpatialEntity {
        display_name: None,
        kind: EntityKind::Player,
        entity: EntityRef {
            identity: [9; 16],
            generation: 1,
        },
        position: ActorPosition {
            x: 32373,
            y: 31151,
            floor: -8,
        },
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    let fields = EntityFields::selected(&[6, 371]);
    let mut encoded = encode_entity(&entity, fields, None)?;
    for icon in [
        &[8, 0, 16, 2, 24, 0][..],
        &[8, 0, 16, 2, 24, 111][..],
        &[8, 0, 16, 17, 24, 100][..],
    ] {
        push_message(&mut encoded, 14, icon);
    }
    let (decoded, _) = decode_entity(&encoded, fields)?;
    assert_eq!(
        encode_entity(&decoded, fields, None)?,
        encoded,
        "visible zero and duplicate RedCross IDs must survive exact source order"
    );
    Ok(())
}

#[test]
fn srx3_native_creature_icons_refuse_unqualified_counts_and_four_visible_entries() {
    for icon in [
        NativeCreatureIcon {
            category: 0,
            id: 2,
            count: 112,
        },
        NativeCreatureIcon {
            category: 0,
            id: 17,
            count: 0,
        },
        NativeCreatureIcon {
            category: 0,
            id: 17,
            count: 101,
        },
        NativeCreatureIcon {
            category: 0,
            id: 22,
            count: 0,
        },
        NativeCreatureIcon {
            category: 1,
            id: 6,
            count: 40,
        },
        NativeCreatureIcon {
            category: 1,
            id: 7,
            count: 1,
        },
        NativeCreatureIcon {
            category: 2,
            id: 2,
            count: 1,
        },
    ] {
        assert!(
            NativeCreatureIcons::new(&[icon]).is_err(),
            "unqualified source icon {icon:?}"
        );
    }
    let icon = NativeCreatureIcon {
        category: 0,
        id: 2,
        count: 0,
    };
    assert!(NativeCreatureIcons::new(&[icon; 4]).is_err());
    assert_eq!(
        NativeCreatureIcons::new(&[]).map(|icons| icons.iter().count()),
        Ok(0)
    );
}

#[test]
fn srx3_native_creature_icons_general_hazard_preserves_source_u16_points()
-> Result<(), WorldSpatialError> {
    // Crystal player.cpp5573-5583 / Canary player.cpp5628-5638 publish uint16
    // nonzero points as Quests/Hazard22; hazard_primal.lua1-5 admits level12.
    // Rotten's separate named rotten-hazard source setter remains capped at4.
    let entity = WorldSpatialEntity {
        display_name: None,
        kind: EntityKind::Player,
        entity: EntityRef {
            identity: [9; 16],
            generation: 1,
        },
        position: ActorPosition {
            x: 32373,
            y: 31151,
            floor: -8,
        },
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    let fields = EntityFields::selected(&[6, 371]);
    for count in [5, 12, u16::MAX] {
        let icon = NativeCreatureIcon {
            category: 0,
            id: 22,
            count,
        };
        let mut bytes = encode_entity(&entity, fields, None)?;
        push_message(&mut bytes, 14, &native_icons::encode(icon));
        let (decoded, _) = decode_entity(&bytes, fields)?;
        let EntityDetail::ActorWithIcons { icons, .. } = decoded.detail else {
            return Err(WorldSpatialError::Malformed);
        };
        assert_eq!(icons.iter().collect::<Vec<_>>(), [icon]);
        assert_eq!(encode_entity(&decoded, fields, None)?, bytes);
    }
    let mut outside_u16 = vec![8, 0, 16, 22, 24];
    push_varint(&mut outside_u16, u64::from(u16::MAX) + 1);
    let mut bytes = encode_entity(&entity, fields, None)?;
    push_message(&mut bytes, 14, &outside_u16);
    assert_eq!(
        decode_entity(&bytes, fields),
        Err(WorldSpatialError::Malformed)
    );
    Ok(())
}
