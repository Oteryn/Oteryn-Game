#![allow(clippy::unwrap_used)]

use super::*;
use crate::store::CurrencyKind;

fn uuid(last: u8) -> [u8; 16] {
    let mut value = [0; 16];
    value[6] = 0x70;
    value[8] = 0x80;
    value[15] = last;
    value
}

fn request() -> PurchaseStoreOffer {
    PurchaseStoreOffer {
        purchase_request_id: PurchaseRequestId::from_bytes(uuid(1)).unwrap(),
        account_id: AccountId::from_bytes(uuid(2)).unwrap(),
        character_id: CharacterId::from_bytes(uuid(3)).unwrap(),
        offer_key: "offer".into(),
        catalog_revision: "r1".into(),
        price: 50,
        currency: CurrencyKind::TibiaCoin,
    }
}

#[test]
fn request_id_requires_uuid_v7_rfc_variant() {
    assert_eq!(
        PurchaseRequestId::from_bytes([0; 16]),
        Err(PurchaseRequestIdError::Nil)
    );
    let mut wrong_version = uuid(1);
    wrong_version[6] = 0x40;
    assert_eq!(
        PurchaseRequestId::from_bytes(wrong_version),
        Err(PurchaseRequestIdError::WrongVersion)
    );
    let mut wrong_variant = uuid(1);
    wrong_variant[8] = 0x40;
    assert_eq!(
        PurchaseRequestId::from_bytes(wrong_variant),
        Err(PurchaseRequestIdError::WrongVariant)
    );
}

#[test]
fn binding_digest_is_stable_and_covers_each_binding_member() {
    let original = request();
    assert_eq!(original.binding_digest(), request().binding_digest());
    let mut changed = request();
    changed.price += 1;
    assert_ne!(original.binding_digest(), changed.binding_digest());
    changed = request();
    changed.offer_key.push('x');
    assert_ne!(original.binding_digest(), changed.binding_digest());
    changed = request();
    changed.catalog_revision.push('x');
    assert_ne!(original.binding_digest(), changed.binding_digest());
    changed = request();
    changed.currency = CurrencyKind::TransferableTibiaCoin;
    assert_ne!(original.binding_digest(), changed.binding_digest());
}
