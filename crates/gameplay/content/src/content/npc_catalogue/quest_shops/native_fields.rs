//! Preserve the existing native service offer's count and subtype through its quest gate.
use super::{Offer, ProjectV2ServiceOffer};

pub(super) fn valid(offer: &Offer) -> bool {
    offer
        .count
        .is_none_or(|count| (1..=u32::from(u16::MAX)).contains(&count))
}

pub(super) fn project(offer: &Offer) -> ProjectV2ServiceOffer {
    ProjectV2ServiceOffer {
        item: offer.item.clone(),
        direction: offer.direction,
        unit_price: offer.unit_price,
        currency: None,
        count: Some(offer.count.unwrap_or(1)),
        sub_type: offer.sub_type,
        parity_pending: false,
    }
}
