use super::*;
#[test]
fn podium_display_visibility_codec_preserves_legacy_and_rejects_unknown_bits()
-> Result<(), &'static str> {
    let value = SetRequest {
        item: [7; 16],
        expected_revision: 3,
        display_kind: 3,
        direction: 2,
        key: "oteryn:creature.rat".into(),
    };
    let old = value.encode().ok_or("old encoding")?;
    for bits in 0..4 {
        let request = Request::Set {
            value: value.clone(),
            visibility: Visibility::decode(bits).ok_or("flags")?,
        };
        let bytes = request.encode().ok_or("encoding")?;
        assert_eq!(Request::decode(&bytes), Some(request));
        assert!(SetRequest::decode(&bytes).is_none());
    }
    assert_eq!(SetRequest::decode(&old), Some(value));
    assert!(Request::decode(&[1, 4]).is_none());
    assert!(Request::decode(&[0; 18]).is_none());
    let reply = Reply::State(State {
        committed: true,
        podium_kind: 3,
        item_definition_key: "oteryn:item.tibia.i42367".into(),
        item_definition_revision: "definition-r1".into(),
        platform_appearance: 42367,
        appearance_kind: 1,
        appearance: 21,
        visibility: Visibility {
            display: false,
            platform: true,
        },
        projection: Some(Projection {
            podium_kind: 3,
            display_kind: 3,
            direction: 2,
            revision: 4,
            key: "oteryn:creature.rat".into(),
        }),
    });
    let bytes = reply.encode().ok_or("reply")?;
    assert_eq!(Reply::decode(&bytes), Some(reply));
    for length in 0..bytes.len() {
        assert!(Reply::decode(&bytes[..length]).is_none());
    }
    Ok(())
}

#[test]
fn podium_live_handle_targets_are_bounded_and_cannot_smuggle_a_uuid() -> Result<(), &'static str> {
    let value = SetRequest {
        item: [0; 16],
        expected_revision: 1,
        display_kind: 3,
        direction: 1,
        key: "oteryn:creature.rat".into(),
    };
    for request in [
        Request::QueryHandle(9),
        Request::SetHandle {
            handle: 9,
            value: value.clone(),
            visibility: Visibility::default(),
        },
        Request::PreviewHandle {
            handle: 9,
            value: value.clone(),
            visibility: Visibility::default(),
        },
    ] {
        let bytes = request.encode().ok_or("encoding")?;
        assert_eq!(Request::decode(&bytes), Some(request));
    }
    assert!(Request::QueryHandle(0).encode().is_none());
    assert!(Request::decode(&[3; 10]).is_none());
    assert!(
        Request::SetHandle {
            handle: 1,
            value: SetRequest {
                item: [7; 16],
                ..value
            },
            visibility: Visibility::default()
        }
        .encode()
        .is_none()
    );
    Ok(())
}
