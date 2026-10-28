#![allow(clippy::unwrap_used)]

use super::*;
use crate::store::{AdmittedOffer, CurrencyKind, Eligibility, OfferKind};

fn uuid(last: u8) -> [u8; 16] {
    let mut value = [0; 16];
    value[6] = 0x70;
    value[8] = 0x80;
    value[15] = last;
    value
}
fn catalogue(delivery: DeliveryKind) -> Catalogue {
    Catalogue::admit(
        "r1",
        [AdmittedOffer {
            key: "offer".into(),
            category: "c".into(),
            kind: OfferKind::Item,
            price_coins: 1,
            currency: CurrencyKind::TibiaCoin,
            delivery,
            eligibility: Eligibility::default(),
        }],
    )
    .unwrap()
}
fn intent() -> DeliveryIntent {
    DeliveryIntent {
        operation_id: OperationId(uuid(1)),
        account_id: AccountId::from_bytes(uuid(2)).unwrap(),
        character_id: CharacterId::from_bytes(uuid(3)).unwrap(),
        world_id: WorldId::from_bytes(uuid(4)).unwrap(),
        fulfilment_unit_id: "unit".into(),
        offer_key: "offer".into(),
        catalog_revision: "r1".into(),
        quantity: 2,
    }
}

#[test]
fn apply_plan_is_deterministic_and_routes_items_to_store_inbox() {
    let item_catalogue = catalogue(DeliveryKind::Item {
        item_key: "parcel".into(),
        count: 3,
    });
    let first = plan_delivery(&item_catalogue, &intent(), &CharacterStoreView::default()).unwrap();
    let replay = plan_delivery(&item_catalogue, &intent(), &CharacterStoreView::default()).unwrap();
    assert_eq!(first, replay);
    assert_eq!(
        first,
        [DeliveryEffect::MintStoreInbox {
            item_key: "parcel".into(),
            count: 6
        }]
    );
}

#[test]
fn every_delivery_rejection_is_typed() {
    let item_catalogue = catalogue(DeliveryKind::Item {
        item_key: "item".into(),
        count: u32::MAX,
    });
    let mut value = intent();
    value.catalog_revision = "old".into();
    assert_eq!(
        plan_delivery(&item_catalogue, &value, &Default::default()),
        Err(DeliveryRejection::UnknownCatalogueRevision)
    );
    value = intent();
    value.offer_key = "missing".into();
    assert_eq!(
        plan_delivery(&item_catalogue, &value, &Default::default()),
        Err(DeliveryRejection::UnknownOffer)
    );
    value = intent();
    value.quantity = 0;
    assert_eq!(
        plan_delivery(&item_catalogue, &value, &Default::default()),
        Err(DeliveryRejection::ZeroQuantity)
    );
    assert_eq!(
        plan_delivery(&item_catalogue, &intent(), &Default::default()),
        Err(DeliveryRejection::QuantityOverflow)
    );
    let unlock_catalogue = catalogue(DeliveryKind::Unlock {
        unlock_key: "mount".into(),
    });
    let mut view = CharacterStoreView::default();
    view.owned_unlocks.insert("mount".into());
    assert_eq!(
        plan_delivery(&unlock_catalogue, &intent(), &view),
        Err(DeliveryRejection::AlreadyOwned)
    );
}

#[test]
fn service_and_coin_package_have_no_game_effect() {
    for delivery in [DeliveryKind::Service, DeliveryKind::CoinPackage] {
        assert_eq!(
            plan_delivery(&catalogue(delivery), &intent(), &Default::default()).unwrap(),
            [DeliveryEffect::NoGameEffect]
        );
    }
}
