#![allow(clippy::unwrap_used)]

use super::*;
use crate::store::{AdmittedOffer, DeliveryKind, Eligibility, OfferKind};

fn catalogue(eligibility: Eligibility) -> Catalogue {
    Catalogue::admit(
        "revision-7",
        [AdmittedOffer {
            key: "offer".into(),
            category: "c".into(),
            kind: OfferKind::Mount,
            price_coins: 99,
            currency: CurrencyKind::TransferableTibiaCoin,
            delivery: DeliveryKind::Unlock {
                unlock_key: "mount".into(),
            },
            eligibility,
        }],
    )
    .unwrap()
}

#[test]
fn quote_fixes_the_catalogue_binding() {
    let quote = build_quote(
        &catalogue(Eligibility::default()),
        "offer",
        &CharacterStoreView::default(),
    )
    .unwrap();
    assert_eq!(
        (
            quote.catalog_revision.as_str(),
            quote.offer_key.as_str(),
            quote.price,
            quote.currency
        ),
        (
            "revision-7",
            "offer",
            99,
            CurrencyKind::TransferableTibiaCoin
        )
    );
}

#[test]
fn every_local_refusal_is_typed() {
    let mut view = CharacterStoreView {
        premium: false,
        vocation: "knight".into(),
        level: 5,
        ..Default::default()
    };
    assert_eq!(
        build_quote(&catalogue(Eligibility::default()), "missing", &view),
        Err(QuoteRefusal::UnknownOffer)
    );
    assert_eq!(
        build_quote(
            &catalogue(Eligibility {
                premium_required: true,
                ..Default::default()
            }),
            "offer",
            &view
        ),
        Err(QuoteRefusal::PremiumRequired)
    );
    assert_eq!(
        build_quote(
            &catalogue(Eligibility {
                vocations: ["druid".into()].into(),
                ..Default::default()
            }),
            "offer",
            &view
        ),
        Err(QuoteRefusal::VocationNotEligible)
    );
    assert_eq!(
        build_quote(
            &catalogue(Eligibility {
                minimum_level: Some(10),
                ..Default::default()
            }),
            "offer",
            &view
        ),
        Err(QuoteRefusal::LevelTooLow { required: 10 })
    );
    view.owned_unlocks.insert("mount".into());
    assert_eq!(
        build_quote(
            &catalogue(Eligibility {
                reject_if_unlock_owned: Some("mount".into()),
                ..Default::default()
            }),
            "offer",
            &view
        ),
        Err(QuoteRefusal::UnlockAlreadyOwned)
    );
    view.purchases_by_offer.insert("offer".into(), 2);
    assert_eq!(
        build_quote(
            &catalogue(Eligibility {
                per_character_limit: Some(2),
                ..Default::default()
            }),
            "offer",
            &view
        ),
        Err(QuoteRefusal::PerCharacterLimitReached { limit: 2 })
    );
}
