//! One bounded offer catalogue for both fact selection and dialogue routing.
use super::*;

const SHARED_OFFERS: &str =
    include_str!("../../../../../../../content/quests/shared-reward-offers.json");

pub(super) fn load(admission: &ComposedFreshAdmission<'_, '_, '_>) -> Option<Offers> {
    let mut offers = Offers {
        schema: 1,
        offers: Vec::new(),
    };
    for source in [
        SHARED_OFFERS,
        include_str!("../../../../../../../content/quests/chosen-reward-offers.json"),
        include_str!("../../../../../../../content/quests/npc-offers/l6-stories.json"),
        include_str!("../../../../../../../content/quests/npc-offers/m09-rookie.json"),
        include_str!("../../../../../../../content/quests/npc-offers/m16-targuna-marker.json"),
        include_str!("../../../../../../../content/quests/npc-offers/qd-steal-from-thieves.json"),
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

#[cfg(test)]
#[test]
fn pirate_and_oriental_offers_accept_the_named_source_materials()
-> Result<(), Box<dyn std::error::Error>> {
    let shared: Offers = serde_json::from_str(SHARED_OFFERS)?;
    for (npc, topic, keywords) in [
        ("duncan", 5, &["hooks", "pirate hooks"][..]),
        ("habdel", 3, &["comb", "mermaid comb", "mermaid's comb"]),
        ("ishina", 3, &["comb", "mermaid comb", "mermaid's comb"]),
    ] {
        let offer = shared
            .offers
            .iter()
            .find(|offer| {
                offer.npc.strip_prefix("oteryn:npc.") == Some(npc)
                    && offer.matches_topic(0)
                    && offer.next_topic == topic
            })
            .ok_or_else(|| format!("source offer for {npc}"))?;
        assert_eq!(offer.keywords, keywords, "{npc}");
    }
    Ok(())
}
