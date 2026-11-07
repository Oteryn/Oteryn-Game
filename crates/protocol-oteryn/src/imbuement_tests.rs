#![allow(clippy::expect_used)]
use super::*;

fn handle(value: u64) -> ItemHandle {
    ItemHandle::new(value).expect("nonzero")
}

#[test]
fn imbuement_request_strict_codec() {
    for action in [
        ImbuementAction::Query,
        ImbuementAction::Apply {
            slot: 2,
            imbuement: 59,
        },
        ImbuementAction::Clear { slot: 0 },
    ] {
        let r = ImbuementRequest {
            item: handle(u64::MAX),
            action,
        };
        let bytes = encode_imbuement_request(&r).expect("request");
        assert!(bytes.len() <= MAX_REQUEST_BYTES);
        assert_eq!(decode_imbuement_request(&bytes), Ok(r));
        for end in 0..bytes.len() {
            assert!(decode_imbuement_request(&bytes[..end]).is_err());
        }
        let mut duplicate = bytes.clone();
        duplicate.extend_from_slice(&[8, 1]);
        assert!(decode_imbuement_request(&duplicate).is_err());
        let mut unknown = bytes;
        unknown.extend_from_slice(&[40, 1]);
        assert!(decode_imbuement_request(&unknown).is_err());
    }
    assert!(decode_imbuement_request(&[0; MAX_REQUEST_BYTES + 1]).is_err());
    // Zero handle, unknown action, slot or index out of range, extra or missing operands,
    // non-canonical field order.
    for bad in [
        &[8, 1, 16, 0][..],
        &[8, 4, 16, 1],
        &[8, 2, 16, 1, 24, 3, 32, 0],
        &[8, 2, 16, 1, 24, 0, 32, 60],
        &[8, 1, 16, 1, 24, 0],
        &[8, 3, 16, 1, 24, 0, 32, 0],
        &[8, 2, 16, 1, 24, 0],
        &[8, 3, 16, 1],
        &[16, 1, 8, 1],
    ] {
        assert!(decode_imbuement_request(bad).is_err(), "{bad:?}");
    }
    assert!(
        encode_imbuement_request(&ImbuementRequest {
            item: handle(1),
            action: ImbuementAction::Clear { slot: MAX_SLOTS },
        })
        .is_err()
    );
}

fn full_window() -> ImbuementWindow {
    ImbuementWindow {
        slot_count: MAX_SLOTS,
        applied: (0..MAX_SLOTS)
            .map(|slot| AppliedImbuement {
                slot,
                family: MAX_FAMILY - slot,
                tier: 3,
                remaining_ms: 72_000_000,
            })
            .collect(),
        offers: (1..=MAX_FAMILY)
            .flat_map(|family| {
                (1..=3).map(move |tier| ImbuementOffer {
                    family,
                    tier,
                    gold: u32::MAX,
                    requires_premium: tier > 1,
                })
            })
            .collect(),
        clear_gold: u32::MAX,
    }
}

#[test]
fn imbuement_reply_strict_codec_and_bound() {
    let window = full_window();
    assert_eq!(window.offers.len(), MAX_OFFERS);
    let full = ImbuementReply {
        result: ImbuementResult::Ok,
        window: Some(window),
    };
    let bytes = encode_imbuement_reply(&full).expect("largest reply fits");
    assert!(bytes.len() <= MAX_REPLY_BYTES);
    assert_eq!(decode_imbuement_reply(&bytes), Ok(full.clone()));
    for end in 0..bytes.len() {
        assert!(decode_imbuement_reply(&bytes[..end]).is_err() || end == 2);
    }
    for result in 1..=7 {
        let r = ImbuementReply {
            result: result_of(result).expect("known result"),
            window: None,
        };
        let bytes = encode_imbuement_reply(&r).expect("bare result");
        assert_eq!(decode_imbuement_reply(&bytes), Ok(r));
    }
    assert!(decode_imbuement_reply(&[8, 0]).is_err());
    assert!(decode_imbuement_reply(&[8, 8]).is_err());
    assert!(decode_imbuement_reply(&[8, 1, 8, 1]).is_err());
    assert!(decode_imbuement_reply(&[8, 1, 16, 1]).is_err());
    assert!(decode_imbuement_reply(&[0; MAX_REPLY_BYTES + 1]).is_err());

    // A window only on OK, every row inside its domain and in canonical order.
    let mut wrong_result = full.clone();
    wrong_result.result = ImbuementResult::NotAllowed;
    assert!(encode_imbuement_reply(&wrong_result).is_err());
    let invalid: [fn(&mut ImbuementWindow); 6] = [
        |w| w.slot_count = 4,
        |w| w.applied[0].slot = 3,
        |w| w.applied.swap(0, 1),
        |w| w.applied[1].family = 0,
        |w| w.offers.swap(0, 1),
        |w| w.offers.push(w.offers[0]),
    ];
    for mutate in invalid {
        let mut r = full.clone();
        mutate(r.window.as_mut().expect("window"));
        assert!(encode_imbuement_reply(&r).is_err());
    }
}
