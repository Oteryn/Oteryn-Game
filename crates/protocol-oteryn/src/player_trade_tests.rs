use super::*;
#[test]
fn player_trade_wire_roundtrip_and_bounds() -> Result<(), Error> {
    let partner = EntityRef {
        identity: [1; 16],
        generation: 7,
    };
    for intent in [
        Intent::Offer {
            partner,
            item: u64::MAX,
        },
        Intent::Accept { trade_id: [2; 16] },
        Intent::Cancel,
        Intent::Look {
            trade_id: [3; 16],
            counter_offer: true,
        },
    ] {
        assert_eq!(decode_intent(&encode_intent(intent)?)?, intent);
    }
    for bytes in [
        vec![],
        vec![26, 0, 26, 0],
        vec![18, 16, 0],
        vec![42, 0],
        vec![0; MAX_INTENT_BYTES + 1],
    ] {
        assert!(decode_intent(&bytes).is_err());
    }
    assert!(encode_intent(Intent::Offer { partner, item: 0 }).is_err());
    assert!(encode_intent(Intent::Accept { trade_id: [0; 16] }).is_err());
    assert_eq!(ResultCode::Completed.payload(), [8, 3]);
    let state = State {
        contents: Default::default(),
        trade_id: Some([3; 16]),
        partner: Some(partner),
        own: Some(Offer {
            definition: 1,
            count: 100,
            sub_type: 0,
        }),
        counter: Some(Offer {
            definition: 2,
            count: 1,
            sub_type: 7,
        }),
        accepted: [true, false],
        phase: 2,
        reason: 0,
        message: String::new(),
        look: String::new(),
    };
    let encoded = encode_state(&state)?;
    assert_eq!(decode_state(&encoded)?, state);
    for code in [
        ResultCode::Offered,
        ResultCode::Accepted,
        ResultCode::Completed,
        ResultCode::Cancelled,
        ResultCode::TooFar,
        ResultCode::NotTradeable,
        ResultCode::PartnerBusy,
        ResultCode::NoRoom,
        ResultCode::NoBackpack,
        ResultCode::Stale,
        ResultCode::NotSupported,
        ResultCode::Rejected,
    ] {
        assert_eq!(decode_result(&code.payload())?, code);
    }
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/player_trade_ready_v1.bin"
    );
    if std::env::var_os("OTERYN_GENERATE_TRADE_GOLDEN").is_some() {
        std::fs::write(fixture, &encoded).map_err(|_| Error::Malformed)?;
    }
    assert_eq!(
        encoded,
        std::fs::read(fixture).map_err(|_| Error::Malformed)?
    );
    assert!(
        encode_state(&State {
            own: None,
            ..state.clone()
        })
        .is_err()
    );
    assert!(
        encode_state(&State {
            look: "a".repeat(4097),
            ..state
        })
        .is_err()
    );
    Ok(())
}

#[test]
fn trade_contents_are_negotiated_bounded_and_parent_indexed() -> Result<(), Error> {
    let root = Offer {
        definition: 1,
        count: 1,
        sub_type: 0,
    };
    let mut state = State {
        own: Some(root),
        partner: Some(EntityRef {
            identity: [1; 16],
            generation: 1,
        }),
        phase: 1,
        reason: 0,
        ..Default::default()
    };
    state.contents[0] = vec![
        ContentsItem {
            item: root,
            parent: None,
            name: "backpack".into(),
        },
        ContentsItem {
            item: Offer {
                definition: 2,
                count: 9,
                sub_type: 0,
            },
            parent: Some(0),
            name: "fish".into(),
        },
    ];
    let mut omitted_root_parent = encode_state(&state)?;
    let mut body = Vec::new();
    push_message_field(&mut body, 1, &offer(root)?);
    push_message_field(&mut body, 3, b"backpack");
    push_message_field(&mut omitted_root_parent, 11, &body);
    assert_eq!(
        decode_state_for(&omitted_root_parent, true)?.contents[0],
        state.contents[0][..1]
    );
    let encoded = encode_state_for(&state, true)?;
    assert!(decode_state(&encoded).is_err());
    assert_eq!(decode_state_for(&encoded, true)?, state);
    assert!(decode_state_for(&encode_state(&state)?, true)?.contents[0].is_empty());
    state.contents[0][1].parent = Some(1);
    assert!(encode_state_for(&state, true).is_err());
    state.contents[0][1].parent = None;
    assert!(encode_state_for(&state, true).is_err());
    state.contents[0][1].parent = Some(0);
    let child = state.contents[0][1].clone();
    state.contents[0].resize(MAX_CONTENTS_ITEMS, child.clone());
    let maximum = encode_state_for(&state, true)?;
    assert_eq!(decode_state_for(&maximum, true)?, state);
    state.contents[0].push(child);
    assert!(encode_state_for(&state, true).is_err());
    Ok(())
}

#[test]
fn player_trade_own_offer_look_accepts_omitted_false() -> Result<(), Error> {
    let trade_id = [9; 16];
    let mut bytes = vec![34, 18, 10, 16];
    bytes.extend_from_slice(&trade_id);
    assert_eq!(
        decode_intent(&bytes)?,
        Intent::Look {
            trade_id,
            counter_offer: false
        }
    );
    Ok(())
}

#[test]
fn player_trade_state_rejects_one_byte_above_registered_ceiling() {
    let payload = vec![0; 65_537];
    assert_eq!(
        decode_state_for(&payload[..65_536], true),
        Err(Error::Malformed)
    );
    assert_eq!(decode_state_for(&payload, true), Err(Error::LimitExceeded));
    assert_eq!(MAX_CONTENTS_STATE_BYTES, 65_536);
}
