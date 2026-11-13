//! One bounded offer catalogue for both fact selection and dialogue routing.
use super::*;

pub(super) fn load(admission: &ComposedFreshAdmission<'_, '_, '_>) -> Option<Offers> {
    let mut offers = Offers {
        schema: 1,
        offers: Vec::new(),
    };
    for source in [
        include_str!("../../../../../../../content/quests/shared-reward-offers.json"),
        include_str!("../../../../../../../content/quests/chosen-reward-offers.json"),
        include_str!("../../../../../../../content/quests/npc-offers/l6-stories.json"),
        include_str!("../../../../../../../content/quests/npc-offers/m16-targuna-marker.json"),
        include_str!("../../../../../../../content/quests/npc-offers/qnpcfx-koshei.json"),
        include_str!("../../../../../../../content/quests/npc-offers/qnpcfx-m10.json"),
    ] {
        let document: Offers = serde_json::from_str(source).ok()?;
        if document.schema != 1 {
            return None;
        }
        offers.offers.extend(document.offers);
    }
    offers
        .offers
        .extend(super::offer_match::rafzan_offers()?.offers);
    #[cfg(test)]
    if let Some(rows) = admission.states.npc_conversations.fixture_offer_rows.get() {
        offers.offers.extend(rows.iter().cloned());
    }
    #[cfg(not(test))]
    let _ = admission;
    if offers.offers.len() > 512
        || offers.offers.iter().any(|offer| {
            offer.requires.len() > 8
                || offer.minimum_level == Some(0)
                || offer.failure_reply.len() > 8
                || offer.failure_reply.iter().any(|line| {
                    line.is_empty() || line.len() > 512 || line.chars().any(char::is_control)
                })
        })
    {
        return None;
    }
    Some(offers)
}
