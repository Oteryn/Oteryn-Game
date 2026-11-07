use super::*;
use crate::charm_wire::push_varint_field;
use crate::container_tree::{
    ContainerView, ContainerViews, MAX_CONTAINER_VIEW_BYTES, decode_container_views,
    decode_container_views_for, encode_container_views, encode_container_views_for,
};
use crate::item_view::{ItemEntry, ItemHandle};
use std::num::NonZeroU32;

fn handle(value: u64) -> Result<ItemHandle, String> {
    ItemHandle::new(value).ok_or_else(|| "test handle must be nonzero".to_owned())
}

fn page(view_id: u8, index: u16, total: u16) -> Result<ContainerView, String> {
    let base = u64::from(view_id) * 100;
    let count = total
        .saturating_sub(index)
        .min(u16::from(BROWSE_FIELD_CAPACITY));
    let entries = (0..count)
        .map(|entry| {
            Ok(ItemEntry {
                handle: handle(u64::MAX - base - u64::from(entry))?,
                item_definition_ref: NonZeroU32::MAX,
                count: NonZeroU32::MAX,
                sub_type: u32::MAX,
            })
        })
        .collect::<Result<_, String>>()?;
    Ok(ContainerView {
        view_id,
        container_handle: handle(u64::MAX - base - 99)?,
        parent_handle: None,
        capacity: BROWSE_FIELD_CAPACITY,
        entries,
        browse: Some(BrowseFieldPage {
            position: WorldTilePosition {
                x: i32::from(u16::MAX),
                y: i32::from(u16::MAX),
                floor: BOTTOM_FLOOR,
            },
            index,
            total,
        }),
    })
}

fn assert_views_rejected(views: &ContainerViews, browse_field: bool) -> Result<(), String> {
    // Every view is valid alone; concatenate its repeated-message wrapper to exercise
    // the decoder's aggregate ownership rules independently of the encoder's refusal.
    let mut encoded = Vec::new();
    for view in &views.views {
        encoded.extend(
            encode_container_views_for(
                &ContainerViews {
                    views: vec![view.clone()],
                },
                browse_field,
            )
            .map_err(|error| format!("individual view: {error:?}"))?,
        );
    }
    assert_eq!(
        encode_container_views_for(views, browse_field),
        Err(ItemViewWireError::Malformed)
    );
    assert_eq!(
        decode_container_views_for(&encoded, browse_field),
        Err(ItemViewWireError::Malformed)
    );
    Ok(())
}

#[test]
fn browse_field_commands_round_trip_native_positions_and_u16_indexes() -> Result<(), String> {
    for intent in [
        BrowseFieldIntent::Open {
            position: WorldTilePosition {
                x: 0,
                y: 0,
                floor: 0,
            },
        },
        BrowseFieldIntent::Open {
            position: WorldTilePosition {
                x: 65_535,
                y: 65_535,
                floor: BOTTOM_FLOOR,
            },
        },
        BrowseFieldIntent::Seek {
            view_id: 0,
            index: 0,
        },
        BrowseFieldIntent::Seek {
            view_id: 15,
            index: 30,
        },
        // Alignment and the live container size are authoritative gameplay rules, not wire rules.
        BrowseFieldIntent::Seek {
            view_id: 15,
            index: u16::MAX,
        },
    ] {
        let encoded = encode_browse_field_intent(&intent).map_err(|error| format!("{error:?}"))?;
        assert!(encoded.len() <= MAX_BROWSE_FIELD_INTENT_BYTES);
        assert_eq!(decode_browse_field_intent(&encoded), Ok(intent));
    }
    let largest = BrowseFieldIntent::Open {
        position: WorldTilePosition {
            x: 65_535,
            y: 65_535,
            floor: BOTTOM_FLOOR,
        },
    };
    assert_eq!(
        encode_browse_field_intent(&largest)
            .map_err(|error| format!("{error:?}"))?
            .len(),
        MAX_BROWSE_FIELD_INTENT_BYTES
    );
    Ok(())
}

#[test]
fn browse_field_commands_reject_malformed_or_out_of_range_values() {
    for malformed in [
        &[][..],
        &[0x1a, 0x00],
        &[0x0a, 0x00, 0x12, 0x00],
        &[0x0a, 0x04, 0x08, 0x00, 0x08, 0x01],
        &[0x0a, 0x02, 0x20, 0x01],
        &[0x12, 0x02, 0x08, 0x10],
        &[0x12, 0x04, 0x10, 0x80, 0x80, 0x04],
        &[0x12, 0x04, 0x10, 0x00, 0x10, 0x00],
        &[0x0a, 0x04, 0x18, 0x80, 0x80, 0x04],
        &[0x0a, 0x02, 0x08, 0x01],
        &[0x0a, 0x04, 0x08, 0x80, 0x80, 0x08],
        &[0x0a, 0x02, 0x18, 0x02],
        &[0x0a, 0x02, 0x18, 0x1f],
    ] {
        assert!(
            decode_browse_field_intent(malformed).is_err(),
            "{malformed:?}"
        );
    }
    assert_eq!(
        encode_browse_field_intent(&BrowseFieldIntent::Seek {
            view_id: 16,
            index: 0
        }),
        Err(ItemViewWireError::LimitExceeded)
    );
    assert_eq!(
        decode_browse_field_intent(&[0; MAX_BROWSE_FIELD_INTENT_BYTES + 1]),
        Err(ItemViewWireError::LimitExceeded)
    );
    for position in [
        WorldTilePosition {
            x: -1,
            y: 0,
            floor: 0,
        },
        WorldTilePosition {
            x: 0,
            y: -1,
            floor: 0,
        },
        WorldTilePosition {
            x: 65_536,
            y: 0,
            floor: 0,
        },
        WorldTilePosition {
            x: 0,
            y: 65_536,
            floor: 0,
        },
        WorldTilePosition {
            x: 0,
            y: 0,
            floor: 1,
        },
        WorldTilePosition {
            x: 0,
            y: 0,
            floor: -16,
        },
    ] {
        assert_eq!(
            encode_browse_field_intent(&BrowseFieldIntent::Open { position }),
            Err(ItemViewWireError::Malformed)
        );
    }
}

#[test]
fn browse_field_pages_require_selection_and_preserve_legacy_limits() -> Result<(), String> {
    for (index, total) in [(0, 0), (0, 30), (30, 31), (65_520, u16::MAX)] {
        let views = ContainerViews {
            views: vec![page(7, index, total)?],
        };
        let encoded =
            encode_container_views_for(&views, true).map_err(|error| format!("{error:?}"))?;
        assert_eq!(
            decode_container_views_for(&encoded, true),
            Ok(views.clone())
        );
        assert!(encode_container_views(&views).is_err());
        assert!(decode_container_views(&encoded).is_err());
    }
    let mut ordinary = page(0, 0, 20)?;
    ordinary.browse = None;
    ordinary.capacity = 20;
    let views = ContainerViews {
        views: vec![ordinary.clone()],
    };
    assert_eq!(
        encode_container_views(&views),
        encode_container_views_for(&views, true)
    );
    ordinary.capacity = 21;
    assert_eq!(
        encode_container_views_for(
            &ContainerViews {
                views: vec![ordinary]
            },
            true
        ),
        Err(ItemViewWireError::LimitExceeded)
    );
    Ok(())
}

#[test]
fn browse_field_shared_container_views_keep_independent_pages() -> Result<(), String> {
    let first = page(7, 0, 31)?;
    let mut last = page(8, 30, 31)?;
    last.container_handle = first.container_handle;
    let views = ContainerViews {
        views: vec![first, last],
    };
    let encoded = encode_container_views_for(&views, true).map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        decode_container_views_for(&encoded, true),
        Ok(views.clone())
    );
    assert!(encode_container_views(&views).is_err());
    assert!(decode_container_views(&encoded).is_err());
    Ok(())
}

#[test]
fn browse_field_shared_page_entries_can_appear_in_multiple_views() -> Result<(), String> {
    let first = page(7, 0, 30)?;
    let mut same_page = first.clone();
    same_page.view_id = 8;
    let views = ContainerViews {
        views: vec![first, same_page],
    };
    let encoded = encode_container_views_for(&views, true).map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        decode_container_views_for(&encoded, true),
        Ok(views.clone())
    );
    assert!(encode_container_views(&views).is_err());
    assert!(decode_container_views(&encoded).is_err());
    Ok(())
}

#[test]
fn browse_field_shared_container_metadata_must_agree() -> Result<(), String> {
    let first = page(7, 0, 31)?;
    let mut wrong_position = page(8, 30, 31)?;
    wrong_position.container_handle = first.container_handle;
    wrong_position
        .browse
        .as_mut()
        .ok_or("browse page")?
        .position
        .x -= 1;
    let mut wrong_total = page(8, 0, 30)?;
    wrong_total.container_handle = first.container_handle;
    for other in [wrong_position, wrong_total] {
        assert_views_rejected(
            &ContainerViews {
                views: vec![first.clone(), other],
            },
            true,
        )?;
    }
    Ok(())
}

#[test]
fn browse_field_entries_cannot_belong_to_different_containers() -> Result<(), String> {
    let first = page(7, 0, 1)?;
    // A different synthetic owner cannot reuse an entry, even if its tile matches.
    for x in [i32::from(u16::MAX), i32::from(u16::MAX) - 1] {
        let mut other = page(8, 0, 1)?;
        other.browse.as_mut().ok_or("browse page")?.position.x = x;
        other.entries = first.entries.clone();
        assert_views_rejected(
            &ContainerViews {
                views: vec![first.clone(), other],
            },
            true,
        )?;
    }
    Ok(())
}

#[test]
fn browse_field_and_ordinary_views_cannot_share_container_or_entry_ownership() -> Result<(), String>
{
    let browse = page(7, 0, 1)?;
    let mut ordinary = page(8, 0, 1)?;
    ordinary.browse = None;
    ordinary.capacity = 20;
    let mut same_container = ordinary.clone();
    same_container.container_handle = browse.container_handle;
    ordinary.entries = browse.entries.clone();
    for mut other in [same_container, ordinary] {
        assert_views_rejected(
            &ContainerViews {
                views: vec![browse.clone(), other.clone()],
            },
            true,
        )?;
        // Check both registration orders: neither kind may adopt the other's owner.
        other.view_id = 6;
        assert_views_rejected(
            &ContainerViews {
                views: vec![other, browse.clone()],
            },
            true,
        )?;
    }
    Ok(())
}

#[test]
fn browse_field_selection_keeps_ordinary_ownership_unique() -> Result<(), String> {
    let mut first = page(7, 0, 1)?;
    first.browse = None;
    first.capacity = 20;
    let mut other = page(8, 0, 1)?;
    other.browse = None;
    other.capacity = 20;
    let mut same_container = other.clone();
    same_container.container_handle = first.container_handle;
    other.entries = first.entries.clone();
    for other in [same_container, other] {
        for selected in [false, true] {
            assert_views_rejected(
                &ContainerViews {
                    views: vec![first.clone(), other.clone()],
                },
                selected,
            )?;
        }
    }
    Ok(())
}

#[test]
fn browse_field_page_shape_and_parent_are_strict() -> Result<(), String> {
    let valid = page(7, 30, 31)?;
    let mut invalid = Vec::new();
    let mut wrong = valid.clone();
    wrong.parent_handle = Some(handle(1)?);
    invalid.push(wrong);
    let mut wrong = valid.clone();
    wrong.capacity = 20;
    invalid.push(wrong);
    let mut wrong = valid.clone();
    wrong.entries.clear();
    invalid.push(wrong);
    let mut wrong = valid.clone();
    wrong.browse.as_mut().ok_or("browse page")?.position.x = -1;
    invalid.push(wrong);
    for (index, total) in [(1, 31), (30, 30), (30, 0)] {
        invalid.push(page(7, index, total)?);
    }
    for wrong in invalid {
        assert!(encode_container_views_for(&ContainerViews { views: vec![wrong] }, true).is_err());
    }
    Ok(())
}

#[test]
fn browse_field_metadata_fields_fail_closed() {
    for malformed in [
        &[0x08, 0x01, 0x08, 0x01][..],
        &[0x20, 0x00, 0x20, 0x00],
        &[0x28, 0x00, 0x28, 0x00],
        &[0x30, 0x01],
        &[0x20, 0x80, 0x80, 0x04],
        &[0x28, 0x80, 0x80, 0x04],
        &[0x18, 0x80, 0x80, 0x04],
        &[0x08, 0xff],
    ] {
        assert!(decode_page(malformed).is_err(), "{malformed:?}");
    }
    assert_eq!(
        decode_page(&[0; MAX_BROWSE_FIELD_PAGE_BYTES + 1]),
        Err(ItemViewWireError::LimitExceeded)
    );
}

#[test]
fn browse_field_duplicate_outer_metadata_is_refused() {
    let mut body = Vec::new();
    push_varint_field(&mut body, 1, 7);
    push_varint_field(&mut body, 2, 1);
    push_varint_field(&mut body, 4, u64::from(BROWSE_FIELD_CAPACITY));
    // An empty page message is the valid empty tile at native position (0, 0, 0).
    push_message_field(&mut body, 6, &[]);
    let mut valid = Vec::new();
    push_message_field(&mut valid, 1, &body);
    assert!(decode_container_views_for(&valid, true).is_ok());

    push_message_field(&mut body, 6, &[]);
    let mut repeated = Vec::new();
    push_message_field(&mut repeated, 1, &body);
    assert_eq!(
        decode_container_views_for(&repeated, true),
        Err(ItemViewWireError::Malformed)
    );
}

#[test]
fn browse_field_selection_keeps_ordinary_view_byte_bound() -> Result<(), String> {
    let mut ordinary = page(15, 0, 20)?;
    ordinary.browse = None;
    ordinary.capacity = 20;
    ordinary.parent_handle = Some(handle(u64::MAX - 1598)?);
    let canonical = encode_container_views(&ContainerViews {
        views: vec![ordinary.clone()],
    })
    .map_err(|error| format!("{error:?}"))?;
    assert!(decode_container_views_for(&canonical, true).is_ok());

    let mut body = Vec::new();
    push_varint_field(&mut body, 1, u64::from(ordinary.view_id));
    push_varint_field(&mut body, 2, ordinary.container_handle.get());
    push_varint_field(
        &mut body,
        3,
        ordinary.parent_handle.ok_or("ordinary parent")?.get(),
    );
    push_varint_field(&mut body, 4, u64::from(ordinary.capacity));
    for entry in &ordinary.entries {
        let mut encoded = Vec::new();
        crate::item_view::encode_entry(&mut encoded, 5, entry);
        assert_eq!(encoded.first(), Some(&0x2a));
        // The existing protobuf reader accepts an overlong varint tag. Inflate only the
        // wrapper tag so every item still respects its own byte bound and remains valid.
        body.extend_from_slice(&[0xaa, 0x00]);
        body.extend_from_slice(&encoded[1..]);
    }
    assert_eq!(body.len(), MAX_CONTAINER_VIEW_BYTES + 20);
    assert!(body.len() < MAX_BROWSE_FIELD_VIEW_BYTES);
    let mut inflated = Vec::new();
    push_message_field(&mut inflated, 1, &body);
    for selected in [false, true] {
        assert_eq!(
            decode_container_views_for(&inflated, selected),
            Err(ItemViewWireError::LimitExceeded)
        );
    }
    Ok(())
}

#[test]
fn browse_field_decoders_are_bounded_under_mutation_and_truncation() -> Result<(), String> {
    let command_seeds = [
        BrowseFieldIntent::Open {
            position: WorldTilePosition {
                x: 65_535,
                y: 65_535,
                floor: BOTTOM_FLOOR,
            },
        },
        BrowseFieldIntent::Seek {
            view_id: 15,
            index: u16::MAX,
        },
    ]
    .iter()
    .map(|intent| encode_browse_field_intent(intent).map_err(|error| format!("{error:?}")))
    .collect::<Result<_, _>>()?;
    crate::decoder_fuzz_support::decoder("browse_field_intent", command_seeds, |bytes| {
        decode_browse_field_intent(bytes).map(|_| ())
    });
    let views = ContainerViews {
        views: vec![page(7, 30, 31)?],
    };
    let encoded = encode_container_views_for(&views, true).map_err(|error| format!("{error:?}"))?;
    crate::decoder_fuzz_support::decoder("browse_field_views", vec![encoded], |bytes| {
        decode_container_views_for(bytes, true).map(|_| ())
    });
    Ok(())
}

#[test]
fn browse_field_full_pages_have_a_measured_bounded_payload() -> Result<(), String> {
    let views = ContainerViews {
        views: (0..16)
            .map(|id| page(id, 60_000, u16::MAX))
            .collect::<Result<_, _>>()?,
    };
    let encoded = encode_container_views_for(&views, true).map_err(|error| format!("{error:?}"))?;
    // View zero omits its proto3 default id. Thirty entries and the signed native coordinates
    // account for the capability-specific increase; legacy domain14 codecs retain their bound.
    assert_eq!(encoded.len(), MAX_BROWSE_FIELD_VIEWS_BYTES - 2);
    assert_eq!(MAX_BROWSE_FIELD_VIEWS_BYTES, 15_488);
    assert_eq!(MAX_LIVE_ITEM_HANDLES_BROWSE_FIELD, 811);
    assert_eq!(decode_container_views_for(&encoded, true), Ok(views));
    assert_eq!(
        decode_container_views_for(&vec![0; MAX_BROWSE_FIELD_VIEWS_BYTES + 1], true),
        Err(ItemViewWireError::LimitExceeded)
    );
    Ok(())
}

#[test]
fn browse_field_registry_reuses_container_views_without_a_second_domain() -> Result<(), String> {
    let registry: serde_json::Value = serde_json::from_str(crate::PROTOCOL_OTERYN_V1_REGISTRY_JSON)
        .map_err(|error| error.to_string())?;
    let capabilities = registry["capabilities"].as_array().ok_or("capabilities")?;
    let capability = capabilities
        .iter()
        .find(|entry| entry["id"] == CAPABILITY_BROWSE_FIELD_V1)
        .ok_or("browse capability")?;
    assert_eq!(capability["requires"], serde_json::json!([14, 18]));
    assert_eq!(
        capability["command_types"],
        serde_json::json!([COMMAND_TYPE_BROWSE_FIELD_INTENT])
    );
    assert_eq!(capability["state_domains"], serde_json::json!([]));
    assert_eq!(capability["offered"], false);
    assert!(
        capability["offer_gate"]
            .as_str()
            .is_some_and(|value| value.contains("BUNDLE_WORLD_OFFERED_CAPABILITIES"))
    );
    let commands = registry["command_types"].as_array().ok_or("commands")?;
    let command = commands
        .iter()
        .find(|entry| entry["id"] == COMMAND_TYPE_BROWSE_FIELD_INTENT)
        .ok_or("browse command")?;
    assert_eq!(command["max_payload_bytes"], MAX_BROWSE_FIELD_INTENT_BYTES);
    assert_eq!(command["max_result_payload_bytes"], 4);
    Ok(())
}
