//! Closed purchase causes use the existing fee/audit fixtures and preserve Charm encoding.
use super::*;

#[test]
fn promotion_and_blessing_fee_causes_round_trip_under_the_registered_tags() {
    for (cause, tag) in [
        (
            FeeBurnCauseV1::PromotionPurchase(OneItemNpcPurchaseV1 {
                service_key: "oteryn:service.promotion.eremo".into(),
                occurrence: uuid(70),
            }),
            0x12,
        ),
        (
            FeeBurnCauseV1::BlessingPurchase(OneItemNpcPurchaseV1 {
                service_key: "oteryn:service.blessing.the_wisdom_of_solitude".into(),
                occurrence: uuid(70),
            }),
            0x1a,
        ),
    ] {
        let mut value = burn(vec![coin_line(Coin::Crystal, 3, 1, 2, 0)], 20_000);
        let cause = OneItemFeeBurnCauseV1 { cause: Some(cause) };
        assert_eq!(cause.encode_to_vec()[0], tag);
        value.cause = Some(cause);
        let envelope =
            encode_fee_burn_event_with(identity(), value.clone(), Type2EventTuple::V1).unwrap();
        assert_eq!(decode_fee_burn_envelope(&envelope).unwrap().1, value);
    }
}

#[test]
fn promotion_audit_refuses_a_bank_part_and_wrong_service_family() {
    let mut value = coins_then_bank();
    value.cause = Some(OneItemFeeBurnCauseV1 {
        cause: Some(FeeBurnCauseV1::PromotionPurchase(OneItemNpcPurchaseV1 {
            service_key: "oteryn:service.promotion.eremo".into(),
            occurrence: uuid(70),
        })),
    });
    assert!(check_fee_burn(&value).is_err());
    value.bank_debit = None;
    value.cause = Some(OneItemFeeBurnCauseV1 {
        cause: Some(FeeBurnCauseV1::BlessingPurchase(OneItemNpcPurchaseV1 {
            service_key: "oteryn:service.promotion.eremo".into(),
            occurrence: uuid(70),
        })),
    });
    assert!(check_fee_burn(&value).is_err());
}

#[test]
fn cyclopedia_area_donation_audit_binds_world_area_and_occurrence() {
    let mut value = burn(vec![coin_line(Coin::Crystal, 3, 1, 2, 0)], 20_000);
    value.cause = Some(OneItemFeeBurnCauseV1 {
        cause: Some(FeeBurnCauseV1::AreaDonation(OneItemAreaDonationV1 {
            world: value.world_id.clone(),
            area: 1,
            occurrence: uuid(70),
        })),
    });
    assert_eq!(value.cause.as_ref().unwrap().encode_to_vec()[0], 0x22);
    let envelope =
        encode_fee_burn_event_with(identity(), value.clone(), Type2EventTuple::V1).unwrap();
    assert_eq!(decode_fee_burn_envelope(&envelope).unwrap().1, value);
    for (world, area) in [(uuid(2), 1), (uuid(1), 0), (uuid(1), 2)] {
        value.cause = Some(OneItemFeeBurnCauseV1 {
            cause: Some(FeeBurnCauseV1::AreaDonation(OneItemAreaDonationV1 {
                world,
                area,
                occurrence: uuid(70),
            })),
        });
        assert!(check_fee_burn(&value).is_err());
    }
}
