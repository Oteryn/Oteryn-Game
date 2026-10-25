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
