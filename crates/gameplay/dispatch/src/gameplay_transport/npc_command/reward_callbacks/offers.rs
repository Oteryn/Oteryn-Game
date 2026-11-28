//! One bounded offer catalogue for both fact selection and dialogue routing.
use super::*;

use std::borrow::Cow;
use std::sync::OnceLock;

const MAX_OFFERS: usize = 640;
const SHARED_OFFERS: &str =
    include_str!("../../../../../../../content/quests/shared-reward-offers.json");
const SOURCES: &[&str] = &[
    // The free-account price takes precedence over the shared Hunter addon offer.
    include_str!("../../../../../../../content/quests/npc-offers/hunter-glove-sales.json"),
    SHARED_OFFERS,
    include_str!("../../../../../../../content/quests/chosen-reward-offers.json"),
    include_str!("../../../../../../../content/quests/npc-offers/an-interest-in-botany.json"),
    include_str!("../../../../../../../content/quests/npc-offers/ape-city.json"),
    include_str!("../../../../../../../content/quests/npc-offers/classic-quests.json"),
    include_str!("../../../../../../../content/quests/npc-offers/explorer-society.json"),
    include_str!("../../../../../../../content/quests/npc-offers/l6-stories.json"),
    include_str!("../../../../../../../content/quests/npc-offers/m09-rookie.json"),
    include_str!("../../../../../../../content/quests/npc-offers/m16-targuna-marker.json"),
    include_str!("../../../../../../../content/quests/npc-offers/nomads-land.json"),
    include_str!("../../../../../../../content/quests/npc-offers/qd-steal-from-thieves.json"),
    include_str!("../../../../../../../content/quests/npc-offers/qnpcfx-koshei.json"),
    include_str!("../../../../../../../content/quests/npc-offers/qnpcfx-m10.json"),
    include_str!("../../../../../../../content/quests/npc-offers/rookgaard-exchanges.json"),
    include_str!("../../../../../../../content/quests/npc-offers/threatened-dreams.json"),
    include_str!("../../../../../../../content/quests/npc-offers/classic-missions.json"),
    include_str!("../../../../../../../content/quests/npc-offers/newhaven-gustavo.json"),
    include_str!("../../../../../../../content/quests/npc-offers/quest-contacts.json"),
    include_str!("../../../../../../../content/quests/npc-offers/tibia-tales.json"),
    include_str!("../../../../../../../content/quests/npc-offers/too-hot-to-handle.json"),
    include_str!("../../../../../../../content/quests/npc-offers/tower-defence.json"),
    include_str!("../../../../../../../content/quests/npc-offers/vampire-hunter.json"),
];

pub(super) fn load(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
) -> Option<Cow<'static, [Offer]>> {
    let offers = embedded()?;
    #[cfg(not(test))]
    let _ = admission;
    #[cfg(test)]
    if let Some(rows) = admission.states.npc_conversations.fixture_offer_rows.get() {
        let mut combined = offers.offers.clone();
        combined.extend(rows.iter().cloned());
        return valid(&combined).then_some(Cow::Owned(combined));
    }
    Some(Cow::Borrowed(&offers.offers))
}

pub(super) fn embedded() -> Option<&'static Offers> {
    static OFFERS: OnceLock<Option<Offers>> = OnceLock::new();
    OFFERS.get_or_init(|| parse(SOURCES)).as_ref()
}

fn parse(sources: &[&str]) -> Option<Offers> {
    let mut offers = Offers {
        schema: 1,
        offers: Vec::new(),
    };
    for source in sources {
        let document: Offers = serde_json::from_str(source).ok()?;
        if document.schema != 1 {
            return None;
        }
        offers.offers.extend(document.offers);
    }
    offers
        .offers
        .extend(super::offer_match::rafzan_offers()?.offers);
    valid(&offers.offers).then_some(offers)
}

fn valid(offers: &[Offer]) -> bool {
    offers.len() <= MAX_OFFERS
        && !offers.iter().any(|offer| {
            offer.requires.len() > 8
                || offer.inventory.len() > 8
                || offer.inventory.iter().any(|rule| {
                    !rule.item.starts_with("oteryn:item.") || rule.minimum > rule.maximum
                })
                || offer.minimum_level == Some(0)
                || offer.failure_reply.len() > 8
                || offer.failure_reply.iter().any(|line| {
                    line.is_empty() || line.len() > 512 || line.chars().any(char::is_control)
                })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quest_offers_accept_the_train_size_and_both_premium_spellings() -> Result<(), String> {
        let registered = embedded().ok_or("registered quest offers")?;
        let sources = std::collections::BTreeSet::<_>::from_iter(SOURCES.iter().copied());
        assert_eq!(
            sources.len(),
            SOURCES.len(),
            "each offer shard has one owner"
        );
        let expected: usize = SOURCES
            .iter()
            .map(|source| serde_json::from_str::<Offers>(source).map(|shard| shard.offers.len()))
            .sum::<Result<_, _>>()
            .map_err(|error| error.to_string())?;
        let rafzan = super::super::offer_match::rafzan_offers().ok_or("Rafzan offers")?;
        assert_eq!(registered.offers.len(), expected + rafzan.offers.len());
        let offers = serde_json::json!({
            "schema": 1,
            "offers": (0..529).map(|next_topic| serde_json::json!({
                "npc": "oteryn:npc.elane",
                "keywords": ["glove"],
                "topic": 0,
                "next_topic": next_topic,
                "storage": [],
                "reply": ["Would you sell it?"],
                (if next_topic % 2 == 0 { "premium" } else { "requires_premium" }): next_topic % 2 != 0
            })).collect::<Vec<_>>(),
        });
        let parsed = parse(&[&offers.to_string()]).ok_or("composed offer schema and extent")?;
        assert!(parsed.offers.len() > 512);
        assert!(registered.offers.len() <= MAX_OFFERS);
        for (index, offer) in parsed.offers.iter().take(529).enumerate() {
            assert_eq!(offer.requires_premium, Some(index % 2 != 0));
        }
        assert!(!valid(&vec![parsed.offers[0].clone(); MAX_OFFERS + 1]));
        Ok(())
    }
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
