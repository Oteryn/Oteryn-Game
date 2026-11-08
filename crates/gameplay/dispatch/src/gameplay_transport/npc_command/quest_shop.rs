//! Display snapshots are advisory; the ordinary trade writer rechecks every source predicate.
use super::*;
use crate::content::npc_catalogue::quest_shops;
use crate::content::{ProjectV2ServiceOffer, ProjectV2ServiceOfferDirection};
use crate::gameplay_transport::ComposedFreshAdmission;
use std::collections::BTreeSet;

type Visible = BTreeSet<(String, String, ProjectV2ServiceOfferDirection)>;

pub(super) fn eligible_items(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    native: Option<&crate::content::native_gameplay::NativeGameplayState>,
) -> Visible {
    let (Some(native), Ok(shops), Ok(sessions)) = (
        native,
        quest_shops::shops(),
        admission.states.quest_sessions.lock(),
    ) else {
        return Visible::new();
    };
    let Some(copy) = sessions.get(&session).and_then(|s| s.copy.as_ref()) else {
        return Visible::new();
    };
    let mut visible = Visible::new();
    for shop in shops {
        for offer in &shop.offers {
            if !quest_shops::eligible(offer, |r| copy.tracks().get(&r.track).copied()) {
                continue;
            }
            let item = crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: offer.item.key.clone(),
                revision_ref: offer.item.revision.clone(),
            };
            if super::super::item_ref_admission::generation_item_facts(native, &item).is_ok_and(
                |facts| {
                    facts.weight.is_some()
                        && !matches!(
                            facts.stack,
                            crate::durability::item_transfer::ItemStackClass::Unknown
                        )
                },
            ) {
                visible.insert((shop.npc.clone(), offer.item.key.clone(), offer.direction));
            }
        }
    }
    visible
}

pub(super) fn visible(npc: &str, offer: &ProjectV2ServiceOffer, cache: &Visible) -> bool {
    match quest_shops::shops().and_then(|shops| {
        quest_shops::required(
            shops,
            npc,
            &offer.item,
            offer.direction,
            offer.unit_price,
            offer.count.unwrap_or(1),
        )
    }) {
        Ok(None) => true,
        Ok(Some(_)) => cache.contains(&(npc.into(), offer.item.key.clone(), offer.direction)),
        Err(_) => false,
    }
}

impl NpcConversations {
    pub(super) fn refresh_quest_shop_context(&self, session: GameSessionId, visible: Visible) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.entry(session).or_default().quest_shop_offers = visible;
        }
    }
}
