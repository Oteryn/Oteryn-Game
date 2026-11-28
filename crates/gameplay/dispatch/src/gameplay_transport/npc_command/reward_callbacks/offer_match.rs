//! Read-only offer admission, shared by NPC prepare and its boundary qualification.
use super::{Offer, Offers};
use oteryn_gameplay_quests::quest::QuestStateCatalogue;
use std::collections::BTreeMap;

impl Offer {
    pub(super) fn matches_keyword(&self, text: &str) -> bool {
        if self.crystal_keyword {
            self.keywords
                .iter()
                .any(|word| super::crystal_keyword(text, word))
        } else {
            super::matches_menu_or_exact_keyword(&self.npc, &self.keywords, text)
        }
    }

    pub(super) fn admits(
        &self,
        npc: &str,
        topic: u16,
        text: &str,
        catalogue: &QuestStateCatalogue,
        tracks: &BTreeMap<String, i64>,
    ) -> bool {
        self.npc == npc
            && self.matches_topic(topic)
            && self.matches_keyword(text)
            && self.storage.iter().all(|s| {
                tracks
                    .get(&s.track)
                    .copied()
                    .or_else(|| catalogue.track(&s.track).map(|track| track.initial))
                    .is_some_and(|value| (s.minimum..=s.maximum).contains(&value))
            })
            && self.requires.iter().all(|r| r.holds(catalogue, tracks))
    }
}

pub(super) fn rafzan_offers() -> Option<Offers> {
    let rows: Offers = serde_json::from_str(include_str!(
        "../../../../../../../content/quests/npc-offers/m13-guardcatcher.json"
    ))
    .ok()?;
    (rows.schema == 1).then_some(rows)
}
